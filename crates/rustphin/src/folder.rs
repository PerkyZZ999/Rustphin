//! One pane's folder as a QML list model (W-114c, W-114d): rows are made on
//! demand from the core's newest snapshot, which arrives from the core's
//! own threads and is swapped in on the Qt thread (D-009). The pane's
//! decisions (selection, keyboard movement, rubber band, type-ahead, view
//! mode and zoom, renaming, the status line) are made here and in
//! `rustphin_view::pane`; QML only draws them. No file is read here.

// cxx-qt generates the unsafe FFI glue between Rust and the C++ QObject
// for the list model; nothing in the bridge file is written by hand but
// declarations, and the code here stays under the lint (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "folder_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::OsStr,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Instant,
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QStringList, QVariant,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    DirectoryModel, EntryKind, LoadError, ModelEvent, NameFilter, RealFs, Selection, Snapshot,
    SortKey, ViewOptions, display_name,
};
use rustphin_platform::{
    thumbnails::{ThumbSize, ThumbnailCache, Thumbnailer},
    trash::FreedesktopTrash,
    xdg,
};
use rustphin_view::{
    folder_views::FolderView,
    format,
    layout::{Flow, Grid, Rect, Step, ViewMode, ZOOM_LEVELS, step_zoom},
    location::Location,
    operations::{Listed, duplicate_names, free_name, stem_len},
    pane::{self, Failure, StatusFacts, TypeAhead, Way},
    pane_state::PaneState,
    toolbar_items::Item,
};

use crate::prefs;

/// The roles QML reads, after Qt's own (`Qt::UserRole` is 0x100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Name = 0x100,
    Size,
    Modified,
    Kind,
    Folder,
    Hidden,
    Icon,
    Thumbnail,
    Selected,
    Current,
    Opacity,
    Link,
    Renaming,
}

impl Role {
    const ALL: [Role; 13] = [
        Role::Name,
        Role::Size,
        Role::Modified,
        Role::Kind,
        Role::Folder,
        Role::Hidden,
        Role::Icon,
        Role::Thumbnail,
        Role::Selected,
        Role::Current,
        Role::Opacity,
        Role::Link,
        Role::Renaming,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Name => "name",
            Role::Size => "size",
            Role::Modified => "modified",
            Role::Kind => "kind",
            Role::Folder => "isFolder",
            Role::Hidden => "hidden",
            Role::Icon => "icon",
            Role::Thumbnail => "thumbnail",
            Role::Selected => "selected",
            Role::Current => "current",
            Role::Opacity => "fade",
            Role::Link => "link",
            Role::Renaming => "renaming",
        }
    }
}

/// Threads making thumbnails for one pane.
const THUMB_WORKERS: usize = 2;
/// Thumbnail answers remembered before the oldest are thrown away.
const MAX_KNOWN: usize = 8192;

/// Which pictures may be shown small instead of their type's icon.
fn has_thumbnail(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp" | "image/x-bmp"
    )
}

/// The pane's thumbnails: which are ready, which are being made.
#[derive(Default)]
struct Thumbs {
    thumbnailer: Option<Thumbnailer>,
    known: HashMap<(PathBuf, ThumbSize), Option<PathBuf>>,
    waiting: HashSet<(PathBuf, ThumbSize)>,
    limit: u64,
}

impl Thumbs {
    /// The thumbnail if made; asks for it otherwise.
    fn get(&mut self, file: PathBuf, size: ThumbSize) -> Option<PathBuf> {
        let key = (file, size);
        if let Some(known) = self.known.get(&key) {
            return known.clone();
        }
        if let Some(thumbnailer) = &self.thumbnailer
            && self.waiting.insert(key.clone())
            && let Some(dropped) = thumbnailer.request(key.0, key.1)
        {
            self.waiting.remove(&dropped);
        }
        None
    }

    fn remember(&mut self, key: (PathBuf, ThumbSize), thumb: Option<PathBuf>) {
        self.waiting.remove(&key);
        if self.known.len() >= MAX_KNOWN {
            self.known.clear();
        }
        self.known.insert(key, thumb);
    }

    fn drop_waiting(&mut self) {
        if let Some(thumbnailer) = &self.thumbnailer {
            for dropped in thumbnailer.clear() {
                self.waiting.remove(&dropped);
            }
        }
    }
}

/// The Rust side of `FolderModel`.
pub struct FolderModelRust {
    count: i32,
    loading: bool,
    location: QString,
    title: QString,
    total: QString,
    selection_text: QString,
    free_space: QString,
    problem: QString,
    problem_text: QString,
    problem_status: QString,
    way_label: QString,
    banner: QString,
    can_go_back: bool,
    can_go_forward: bool,
    can_go_up: bool,
    show_hidden: bool,
    notice: QString,
    mode: i32,
    zoom: i32,
    icon_size: f64,
    row_height: f64,
    cell_width: f64,
    name_text: f64,
    name_line: f64,
    data_text: f64,
    focus_row: i32,
    selected_count: i32,
    sort_column: i32,
    descending: bool,
    folders_first: bool,
    show_size: bool,
    show_modified: bool,
    show_type: bool,
    shows_free_space: bool,
    shows_totals: bool,
    renaming_row: i32,
    rename_text: QString,
    rename_stem: i32,
    filter_text: QString,
    trash: bool,
    empty: bool,
    empty_title: QString,
    empty_text: QString,
    crumb_labels: QStringList,
    crumb_paths: QStringList,
    one_click: bool,

    /// The pane's folder, kept as a path: `location` only shows it.
    pane: PaneState,
    model: Option<DirectoryModel>,
    /// The listing shown; it may belong to another folder while the next
    /// one is requested.
    snapshot: Option<Arc<Snapshot>>,
    /// The newest generation asked of the model.
    generation: u64,
    selection: Selection,
    /// The selection a rubber band adds to.
    band_kept: Selection,
    /// The selection's revision when a carry-over started for a re-sort.
    carrying: Option<u64>,
    failure: Option<Failure>,
    /// Names to select once the folder now requested is listed.
    select_on_arrival: Option<Vec<Vec<u8>>>,
    /// A name to select once it appears; `true` to rename it then.
    select_when_seen: Option<(Vec<u8>, bool)>,
    /// Names to select together once every one has appeared.
    select_all_when_seen: Vec<Vec<u8>>,
    /// The name being edited in place.
    renaming: Option<Vec<u8>>,
    type_ahead: TypeAhead,
    /// The file types, once loaded in the background.
    types: Option<Arc<crate::types::Types>>,
    /// Type descriptions already looked up, by MIME type.
    descriptions: RefCell<HashMap<String, String>>,
    thumbs: RefCell<Thumbs>,
    trash_store: Option<Arc<FreedesktopTrash>>,
    /// The trash directory shown, if the folder is one.
    trash_dir: Option<PathBuf>,
    options: ViewOptions,
    view: ViewMode,
    zoom_level: usize,
    view_epoch: u64,
    zone: TimeZone,
    width: f32,
    /// The folder last reported as visited.
    visited: Option<PathBuf>,
    /// The folder, total and free bytes of its volume.
    usage: Option<(PathBuf, u64, u64)>,
    frame_log: bool,
    menu: QString,
    scroll_bench: bool,
    /// When the folder or order now awaited was asked for, what it was, and
    /// its generation, until a frame shows it (W-073g).
    asked: Option<(Instant, &'static str, u64)>,
    /// The asked-for listing has rows on screen as of the next frame.
    shown: bool,
    /// Frame times of the scroll benchmark.
    frames: Vec<Instant>,
    /// The model lists through the administrator's helper.
    as_admin: bool,
    /// Counts models, so a replaced one's events are dropped.
    model_epoch: u64,
    /// The New menu's files.
    new_files: Vec<rustphin_platform::templates::NewFile>,
    /// The open menu and what it acts on.
    menu_entries: Vec<rustphin_view::menu::MenuEntry>,
    /// The hub's counters last taken up.
    clip_serial: u64,
    created_serial: u64,
    admin_changes: u64,
}

impl Default for FolderModelRust {
    fn default() -> Self {
        let settings = prefs::settings();
        let view = settings.mode();
        let zoom_level = settings.zoom();
        let options = ViewOptions {
            details: view == ViewMode::Details,
            ..settings.options()
        };
        let mut rust = Self {
            count: 0,
            loading: false,
            location: QString::default(),
            title: QString::default(),
            total: QString::default(),
            selection_text: QString::default(),
            free_space: QString::default(),
            problem: QString::default(),
            problem_text: QString::default(),
            problem_status: QString::default(),
            way_label: QString::default(),
            banner: QString::default(),
            can_go_back: false,
            can_go_forward: false,
            can_go_up: false,
            show_hidden: options.show_hidden,
            notice: QString::default(),
            mode: mode_number(view),
            zoom: i32::try_from(zoom_level).unwrap_or(0),
            icon_size: 0.,
            row_height: 0.,
            cell_width: 0.,
            name_text: 0.,
            name_line: 0.,
            data_text: 0.,
            focus_row: -1,
            selected_count: 0,
            sort_column: column_of(options.sort),
            descending: options.descending,
            folders_first: options.folders_first,
            show_size: true,
            show_modified: true,
            show_type: false,
            shows_free_space: true,
            shows_totals: true,
            renaming_row: -1,
            rename_text: QString::default(),
            rename_stem: 0,
            filter_text: QString::default(),
            trash: false,
            empty: false,
            empty_title: QString::default(),
            empty_text: QString::default(),
            crumb_labels: QStringList::default(),
            crumb_paths: QStringList::default(),
            one_click: prefs::with(|store| store.opens_with_one_click()),
            pane: PaneState::default(),
            model: None,
            snapshot: None,
            generation: 0,
            selection: Selection::default(),
            band_kept: Selection::default(),
            carrying: None,
            failure: None,
            select_on_arrival: None,
            select_when_seen: None,
            select_all_when_seen: Vec::new(),
            renaming: None,
            type_ahead: TypeAhead::default(),
            types: None,
            descriptions: RefCell::new(HashMap::new()),
            thumbs: RefCell::new(Thumbs {
                limit: settings.previews().limit_bytes(),
                ..Thumbs::default()
            }),
            trash_store: crate::hub::with(|hub| hub.trash()),
            trash_dir: None,
            options,
            view,
            zoom_level,
            view_epoch: prefs::with(|store| store.view_epoch()),
            zone: TimeZone::system(),
            width: 1000.,
            visited: None,
            usage: None,
            menu: QString::default(),
            frame_log: std::env::var_os("RUSTPHIN_FRAME_LOG").is_some()
                || std::env::var_os("RUSTPHIN_SCROLL_BENCH").is_some(),
            scroll_bench: std::env::var_os("RUSTPHIN_SCROLL_BENCH").is_some(),
            asked: None,
            shown: false,
            frames: Vec::new(),
            as_admin: false,
            model_epoch: 0,
            new_files: rustphin_platform::templates::built_in(),
            menu_entries: Vec::new(),
            clip_serial: 0,
            created_serial: crate::hub::with(|hub| hub.created.0),
            admin_changes: 0,
        };
        rust.apply_layout();
        rust
    }
}

impl FolderModelRust {
    /// Takes the sizes of the view mode and zoom level.
    fn apply_layout(&mut self) {
        let layout = self.view.layout(self.zoom_level);
        self.icon_size = f64::from(layout.icon);
        self.row_height = f64::from(layout.row_height);
        self.cell_width = f64::from(layout.cell_width.unwrap_or(0.));
        self.name_text = f64::from(layout.name_text);
        self.name_line = f64::from(layout.name_line);
        self.data_text = f64::from(layout.data_text);
        self.mode = mode_number(self.view);
        self.zoom = i32::try_from(self.zoom_level).unwrap_or(0);
    }

    /// The pane's folder, or an empty path before the first.
    fn folder(&self) -> PathBuf {
        self.pane.folder().map(Path::to_owned).unwrap_or_default()
    }

    /// The snapshot of the folder now shown, if it is the one requested.
    fn current(&self) -> Option<&Arc<Snapshot>> {
        self.snapshot.as_ref()
    }

    /// The type's description, looked up once.
    fn description(&self, mime: &str) -> String {
        let Some(types) = &self.types else {
            return String::new();
        };
        self.descriptions
            .borrow_mut()
            .entry(mime.to_owned())
            .or_insert_with(|| types.mimes.description(mime))
            .clone()
    }

    /// The thumbnail's URL for the entry at `row`, if it may have one and
    /// it is ready; asking gets it made.
    fn thumbnail(&self, snapshot: &Snapshot, row: usize) -> Option<String> {
        let edge = self.view.layout(self.zoom_level).icon;
        // No previews as administrator: they would read every picture as
        // root (docs/project/ADMIN_MODE.md).
        if snapshot.is_container(row)
            || edge < pane::MIN_THUMBNAIL_EDGE
            || self.trash_dir.is_some()
            || self.as_admin
        {
            return None;
        }
        let name = snapshot.link_target_name(row).unwrap_or(snapshot.name(row));
        if !has_thumbnail(self.types.as_ref()?.mimes.mime_type(name, false)) {
            return None;
        }
        let size = ThumbSize::for_edge(edge as u32);
        let thumb = self
            .thumbs
            .borrow_mut()
            .get(snapshot.entry_path(row), size)?;
        Some(rustphin_platform::uri::file_uri(&thumb))
    }

    /// The opacity an entry is drawn with.
    fn fade(&self, snapshot: &Snapshot, row: usize) -> f64 {
        let opacity = prefs::with(|store| store.settings().hidden_opacity()) as f32 / 100.;
        f64::from(pane::entry_opacity(
            snapshot.is_hidden(row),
            crate::hub::is_cut(&snapshot.entry_path(row)),
            self.selection.contains(snapshot, row),
            opacity,
        ))
    }
}

/// The QML number of a view mode.
fn mode_number(mode: ViewMode) -> i32 {
    match mode {
        ViewMode::Icons => 0,
        ViewMode::Compact => 1,
        ViewMode::Details => 2,
    }
}

/// The view mode a QML number names.
fn mode_of(number: i32) -> ViewMode {
    match number {
        0 => ViewMode::Icons,
        1 => ViewMode::Compact,
        _ => ViewMode::Details,
    }
}

/// The sort a column asks for.
fn sort_key(column: i32) -> SortKey {
    match column {
        1 => SortKey::Size,
        2 => SortKey::Modified,
        3 => SortKey::Type,
        _ => SortKey::Name,
    }
}

/// The column a sort is shown on.
fn column_of(key: SortKey) -> i32 {
    match key {
        SortKey::Size => 1,
        SortKey::Modified => 2,
        SortKey::Type => 3,
        _ => 0,
    }
}

/// The keyboard step a QML number names.
fn step_of(number: i32) -> Option<Step> {
    Some(match number {
        0 => Step::Up,
        1 => Step::Down,
        2 => Step::Left,
        3 => Step::Right,
        4 => Step::PageUp,
        5 => Step::PageDown,
        6 => Step::First,
        7 => Step::Last,
        _ => return None,
    })
}

/// The icon-theme name an entry is drawn with: its type's icon; a link
/// looks like what it points to, as in Dolphin.
fn entry_icon(types: Option<&crate::types::Types>, snapshot: &Snapshot, row: usize) -> String {
    let is_folder = snapshot.is_container(row);
    let name = snapshot.link_target_name(row).unwrap_or(snapshot.name(row));
    let names = types.map_or_else(Vec::new, |types| {
        types
            .mimes
            .icon_names(types.mimes.mime_type(name, is_folder))
    });
    icon_id(&names, is_folder)
}

/// The `image://theme/` id of an entry's icon: the theme's names for its
/// type, most specific first, then the generic folder or file icon.
pub(crate) fn icon_id(type_names: &[String], is_folder: bool) -> String {
    let generic = if is_folder {
        "folder"
    } else {
        "text-x-generic"
    };
    type_names
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(generic))
        .collect::<Vec<_>>()
        .join("|")
}

fn text(words: &str) -> QString {
    QString::from(words)
}

fn path_text(path: &Path) -> QString {
    QString::from(path.to_string_lossy().as_ref())
}

fn string_list<'a>(items: impl IntoIterator<Item = &'a str>) -> QStringList {
    let list: QList<QString> = items.into_iter().map(QString::from).collect();
    QStringList::from(&list)
}

impl qobject::FolderModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let (Some(snapshot), Ok(row), Some(role)) = (
            rust.snapshot.as_ref(),
            usize::try_from(index.row()),
            Role::from_number(role),
        ) else {
            return QVariant::default();
        };
        if row >= snapshot.len() {
            return QVariant::default();
        }
        let words = |words: &str| QVariant::from(&QString::from(words));
        let renaming = rust.renaming.as_deref() == Some(snapshot.name(row));
        match role {
            Role::Name => words(&display_name(snapshot.name(row))),
            Role::Size => words(&format::size_column(
                snapshot.is_container(row),
                snapshot.metadata(row).map(|meta| meta.size),
            )),
            Role::Modified => words(
                &snapshot
                    .metadata(row)
                    .map(|meta| format::timestamp(meta.modified, &rust.zone))
                    .unwrap_or_default(),
            ),
            Role::Kind => {
                let Some(types) = &rust.types else {
                    return words("");
                };
                let name = snapshot.link_target_name(row).unwrap_or(snapshot.name(row));
                words(&rust.description(types.mimes.mime_type(name, snapshot.is_container(row))))
            }
            Role::Folder => QVariant::from(&snapshot.is_container(row)),
            Role::Hidden => QVariant::from(&snapshot.is_hidden(row)),
            Role::Icon => words(&entry_icon(rust.types.as_deref(), snapshot, row)),
            Role::Thumbnail => words(&rust.thumbnail(snapshot, row).unwrap_or_default()),
            Role::Selected => {
                QVariant::from(&(rust.selection.contains(snapshot, row) && !renaming))
            }
            Role::Current => {
                QVariant::from(&(rust.selection.focus_row(snapshot) == Some(row) || renaming))
            }
            Role::Opacity => QVariant::from(&rust.fade(snapshot, row)),
            Role::Link => QVariant::from(&(snapshot.kind(row) == EntryKind::Symlink)),
            Role::Renaming => QVariant::from(&renaming),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Tells QML that `roles` of every row may have changed.
    fn rows_changed(mut self: Pin<&mut Self>, roles: &[Role]) {
        let last = *self.count() - 1;
        if last < 0 {
            return;
        }
        let first = self.index(0, 0, &QModelIndex::default());
        let last = self.index(last, 0, &QModelIndex::default());
        let roles: QList<i32> = roles.iter().map(|role| *role as i32).collect();
        self.as_mut().data_changed(&first, &last, &roles);
    }

    /// Tells QML that `roles` of `row` changed.
    fn row_changed(mut self: Pin<&mut Self>, row: usize, roles: &[Role]) {
        let Ok(row) = i32::try_from(row) else {
            return;
        };
        let index = self.index(row, 0, &QModelIndex::default());
        let roles: QList<i32> = roles.iter().map(|role| *role as i32).collect();
        self.as_mut().data_changed(&index, &index, &roles);
    }

    /// Starts the core's model, the file types and the thumbnails the
    /// first time; false when the model could not start.
    fn start(mut self: Pin<&mut Self>) -> bool {
        if self.rust().model.is_some() {
            return true;
        }
        let fs: Arc<dyn rustphin_core::Fs> =
            crate::admin::filesystem().unwrap_or_else(|| Arc::new(RealFs));
        self.as_mut().rust_mut().as_admin = crate::admin::active();
        let (mut model, events) = match DirectoryModel::new(fs) {
            Ok(started) => started,
            Err(error) => {
                let words = format!("Folders cannot be listed: {error}.");
                self.as_mut().set_notice(text(&words));
                return false;
            }
        };
        model.set_options(self.rust().options);
        self.as_mut().rust_mut().model = Some(model);
        crate::hub::watch_pane(self.qt_thread());
        self.as_mut().listen(events);
        let thread = self.qt_thread();
        crate::types::when_ready(move |types| {
            let _ = thread.queue(move |model| model.set_types(types));
        });
        // The New menu's templates, from the user's and the system's folders.
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            use rustphin_platform::templates;
            let folder = xdg::home()
                .and_then(|home| templates::templates_dir(&home, xdg::config_home().as_deref()));
            let files = templates::load(folder.as_deref(), &xdg::data_dirs());
            let _ = thread.queue(move |mut model| model.as_mut().rust_mut().new_files = files);
        });
        if let Some(cache) = ThumbnailCache::for_user() {
            let (thumbnailer, results) = Thumbnailer::start(cache, THUMB_WORKERS);
            thumbnailer.set_limit(self.rust().thumbs.borrow().limit);
            self.rust().thumbs.borrow_mut().thumbnailer = Some(thumbnailer);
            let thread = self.qt_thread();
            std::thread::spawn(move || {
                while let Ok(first) = results.recv() {
                    let mut batch = vec![first];
                    batch.extend(results.try_iter());
                    if thread
                        .queue(move |model| model.thumbs_ready(batch))
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        true
    }

    /// Hands the model's events to the Qt thread; ends with the model.
    fn listen(self: Pin<&mut Self>, events: flume::Receiver<ModelEvent>) {
        let thread = self.qt_thread();
        let epoch = self.rust().model_epoch;
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let queued = thread.queue(move |model| {
                    if model.rust().model_epoch == epoch {
                        model.on_event(event);
                    }
                });
                if queued.is_err() {
                    break;
                }
            }
        });
    }

    /// Lists through the administrator's helper, or the user's own
    /// filesystem again, from the folder shown.
    fn switch_filesystem(mut self: Pin<&mut Self>) {
        let admin = crate::admin::active();
        if self.rust().model.is_none() || self.rust().as_admin == admin {
            return;
        }
        let fs: Arc<dyn rustphin_core::Fs> =
            crate::admin::filesystem().unwrap_or_else(|| Arc::new(RealFs));
        let location = self.rust().folder();
        let options = self.rust().options;
        let (mut model, events) = match DirectoryModel::new(fs) {
            Ok(started) => started,
            Err(error) => {
                tracing::error!(%error, "could not start the directory model");
                return;
            }
        };
        model.set_options(options);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.as_admin = admin;
            rust.model = Some(model);
            rust.model_epoch += 1;
            rust.generation = 0;
            rust.snapshot = None;
            rust.visited = None;
            rust.selection = Selection::default();
            rust.pane.forget_folder();
        }
        self.as_mut().listen(events);
        self.as_mut().set_location(QString::default());
        self.go_to(location);
    }

    /// One event from the core's model.
    fn on_event(mut self: Pin<&mut Self>, event: ModelEvent) {
        match event {
            ModelEvent::Snapshot(snapshot) if snapshot.generation() >= self.rust().generation => {
                // A re-sort or re-read of the folder shown: its selection is
                // carried over by name on the core's threads first.
                let carry = self.rust().snapshot.as_ref().filter(|old| {
                    old.path() == snapshot.path() && old.listing() != snapshot.listing()
                });
                if let (Some(old), Some(model)) = (carry.cloned(), self.rust().model.as_ref()) {
                    let receive =
                        model.carry_selection(self.rust().selection.clone(), old, snapshot.clone());
                    let revision = self.rust().selection.revision();
                    self.as_mut().rust_mut().carrying = Some(revision);
                    let thread = self.qt_thread();
                    std::thread::spawn(move || {
                        if let Ok(selection) = receive.recv() {
                            let _ = thread.queue(move |model| {
                                model.show(snapshot, Some((revision, selection)))
                            });
                        }
                    });
                } else {
                    self.show(snapshot, None);
                }
            }
            ModelEvent::Moved { generation, from } if generation >= self.rust().generation => {
                self.as_mut().rust_mut().generation = generation;
                self.as_mut().rust_mut().select_on_arrival = None;
                self.as_mut().set_failure(Some(Failure {
                    path: from,
                    error: LoadError::NotFound,
                    moved: true,
                }));
            }
            ModelEvent::Failed {
                generation,
                path,
                error,
            } if generation == self.rust().generation => {
                self.as_mut().rust_mut().select_on_arrival = None;
                self.as_mut().measure_free_space(path.clone());
                self.as_mut().set_location_text(&path);
                self.as_mut().set_failure(Some(Failure {
                    path,
                    error,
                    moved: false,
                }));
            }
            _ => {}
        }
    }

    /// Swaps in the newest snapshot, with the selection carried over to it
    /// when one was (and nothing has changed it since).
    fn show(mut self: Pin<&mut Self>, snapshot: Arc<Snapshot>, carried: Option<(u64, Selection)>) {
        if snapshot.generation() < self.rust().generation {
            return;
        }
        let arrived = self
            .rust()
            .snapshot
            .as_ref()
            .is_none_or(|shown| shown.path() != snapshot.path());
        let reread = !arrived
            && self
                .rust()
                .snapshot
                .as_ref()
                .is_some_and(|shown| shown.listing() != snapshot.listing());
        {
            let mut rust = self.as_mut().rust_mut();
            rust.generation = snapshot.generation();
            rust.carrying = None;
            match carried {
                Some((revision, selection)) if rust.selection.revision() == revision => {
                    rust.selection = selection;
                }
                _ => rust.selection.sync(&snapshot),
            }
            if arrived {
                rust.renaming = None;
                rust.select_when_seen = None;
                rust.select_all_when_seen.clear();
                rust.thumbs.borrow_mut().drop_waiting();
            } else if reread {
                let folder = snapshot.path().to_owned();
                rust.thumbs
                    .borrow_mut()
                    .known
                    .retain(|(file, _), _| file.parent() != Some(folder.as_path()));
            }
            let arrival = rust.select_on_arrival.take_if(|_| !snapshot.is_loading());
            if let Some(names) = arrival {
                pane::select_names(&mut rust.selection, &snapshot, &names);
            }
            let awaited = rust
                .select_when_seen
                .as_ref()
                .and_then(|(name, _)| snapshot.row_of(name));
            if let Some(row) = awaited {
                rust.selection.select_only(&snapshot, row);
            }
            if !rust.select_all_when_seen.is_empty()
                && snapshot
                    .rows_of(&rust.select_all_when_seen)
                    .iter()
                    .all(Option::is_some)
            {
                let names = std::mem::take(&mut rust.select_all_when_seen);
                pane::select_names(&mut rust.selection, &snapshot, &names);
            }
            if let Some(name) = &rust.renaming
                && snapshot.row_of(name).is_none()
            {
                rust.renaming = None;
            }
        }
        let rename_now = self
            .rust()
            .select_when_seen
            .as_ref()
            .filter(|(name, _)| snapshot.row_of(name).is_some())
            .map(|(name, rename)| (name.clone(), *rename));
        if rename_now.is_some() {
            self.as_mut().rust_mut().select_when_seen = None;
        }
        let rows = i32::try_from(snapshot.len()).unwrap_or(i32::MAX);
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.snapshot = Some(Arc::clone(&snapshot));
            rust.count = rows;
        }
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        if !snapshot.is_empty()
            && self
                .rust()
                .asked
                .is_some_and(|(_, _, generation)| snapshot.generation() >= generation)
        {
            self.as_mut().rust_mut().shown = true;
        }
        self.as_mut().set_loading(snapshot.is_loading());
        self.as_mut().set_location_text(snapshot.path());
        if self
            .rust()
            .failure
            .as_ref()
            .is_some_and(|failure| !failure.moved || failure.path.parent() != Some(snapshot.path()))
        {
            self.as_mut().set_failure(None);
        }
        if arrived {
            self.as_mut().set_filter_text(QString::default());
            self.as_mut().detect_trash(snapshot.path().to_owned());
            self.as_mut().measure_free_space(snapshot.path().to_owned());
            self.as_mut().adopt_folder_view();
        }
        if !snapshot.is_loading() && self.rust().visited.as_deref() != Some(snapshot.path()) {
            self.as_mut().rust_mut().visited = Some(snapshot.path().to_owned());
            crate::history::visit(snapshot.path());
            self.as_mut().visited(path_text(snapshot.path()));
        }
        self.as_mut().update_navigation();
        self.as_mut().selection_changed();
        if let Some((name, true)) = rename_now {
            self.as_mut().rename_name(name);
        }
    }

    /// Updates what follows from the selection: the focus, the counts, the
    /// status line, and the rows' selected state.
    fn selection_changed(mut self: Pin<&mut Self>) {
        let (focus, selected) = match self.rust().current() {
            Some(snapshot) => (
                self.rust().selection.focus_row(snapshot),
                self.rust().selection.len(),
            ),
            None => (None, 0),
        };
        let focus = focus.and_then(|row| i32::try_from(row).ok()).unwrap_or(-1);
        let moved = *self.focus_row() != focus;
        self.as_mut().set_focus_row(focus);
        self.as_mut()
            .set_selected_count(i32::try_from(selected).unwrap_or(i32::MAX));
        self.as_mut().update_status();
        self.as_mut()
            .rows_changed(&[Role::Selected, Role::Current, Role::Opacity]);
        if moved && focus >= 0 {
            self.as_mut().reveal_row(focus);
        }
    }

    /// Runs `change` on the selection and shows the result.
    fn change_selection(mut self: Pin<&mut Self>, change: impl FnOnce(&mut Selection, &Snapshot)) {
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        change(&mut self.as_mut().rust_mut().selection, &snapshot);
        self.selection_changed();
    }

    /// The status line, the empty state and the in-place error.
    fn update_status(mut self: Pin<&mut Self>) {
        let rust = self.rust();
        let Some(snapshot) = rust.snapshot.clone() else {
            return;
        };
        let selected_rows = rust.selection.rows(&snapshot);
        let filter = snapshot.filter().map(|filter| filter.text().to_owned());
        let renaming = rust
            .renaming
            .as_deref()
            .map(|name| display_name(name).into_owned());
        let facts = StatusFacts {
            shown: snapshot.len(),
            unfiltered: snapshot.unfiltered_len(),
            total: snapshot.total(),
            selected: selected_rows.len(),
            filter: filter.as_deref(),
            loading: snapshot.is_loading(),
            shown_size: pane::total_size(&snapshot, 0..snapshot.len()),
            selected_size: pane::total_size(&snapshot, selected_rows.iter().copied()),
            renaming: renaming.as_deref(),
        };
        let parts = pane::status_parts(&facts);
        let zones = pane::StatusZones::for_width(rust.width, parts.selection.is_some());
        let free = rust
            .usage
            .as_ref()
            .filter(|(path, ..)| path == snapshot.path())
            .map(|(_, total, free)| pane::free_text(*total, *free, None))
            .unwrap_or_default();
        let empty = snapshot.is_empty();
        let state = pane::empty_state(
            snapshot.is_loading(),
            snapshot.total(),
            snapshot.unfiltered_len(),
            filter.as_deref(),
            snapshot.path(),
            rust.trash_dir.is_some(),
        );
        self.as_mut().set_total(text(&parts.total));
        self.as_mut()
            .set_selection_text(text(parts.selection.as_deref().unwrap_or("")));
        self.as_mut().set_free_space(text(&free));
        self.as_mut().set_shows_free_space(zones.free_space);
        self.as_mut().set_shows_totals(zones.totals);
        self.as_mut().set_empty(empty);
        self.as_mut().set_empty_title(text(&state.heading));
        self.as_mut().set_empty_text(text(&state.text));
    }

    /// Shows or clears a failure: in place when nothing is listed for the
    /// folder that failed, else as a banner over the listing.
    fn set_failure(mut self: Pin<&mut Self>, failure: Option<Failure>) {
        self.as_mut().rust_mut().failure = failure.clone();
        let location = self.rust().folder();
        let listed = self
            .rust()
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.path() == location);
        let (title, explanation, status, way, banner) = match &failure {
            Some(failure) if failure.in_place(Some(&location), listed) => {
                let way = Way::out(*self.can_go_back(), location.parent().is_some());
                (
                    failure.title().to_owned(),
                    failure.explanation(way),
                    failure.status().to_owned(),
                    way.label().unwrap_or("").to_owned(),
                    String::new(),
                )
            }
            Some(failure) => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                failure.message(),
            ),
            None => Default::default(),
        };
        self.as_mut().set_problem(text(&title));
        self.as_mut().set_problem_text(text(&explanation));
        self.as_mut().set_problem_status(text(&status));
        self.as_mut().set_way_label(text(&way));
        self.as_mut().set_banner(text(&banner));
    }

    /// Takes the loaded file types and redraws the icons.
    fn set_types(mut self: Pin<&mut Self>, types: Arc<crate::types::Types>) {
        self.as_mut().rust_mut().types = Some(types);
        self.rows_changed(&[Role::Icon, Role::Kind, Role::Thumbnail]);
    }

    /// Takes finished thumbnails and redraws their rows.
    fn thumbs_ready(
        mut self: Pin<&mut Self>,
        batch: Vec<rustphin_platform::thumbnails::Thumbnail>,
    ) {
        let mut rows = Vec::new();
        {
            let rust = self.rust();
            let mut thumbs = rust.thumbs.borrow_mut();
            for done in batch {
                if let Some(snapshot) = &rust.snapshot
                    && done.file.parent() == Some(snapshot.path())
                    && let Some(row) = done
                        .file
                        .file_name()
                        .and_then(|name| snapshot.row_of(name.as_bytes()))
                {
                    rows.push(row);
                }
                thumbs.remember((done.file, done.size), done.thumb);
            }
        }
        for row in rows {
            self.as_mut().row_changed(row, &[Role::Thumbnail]);
        }
    }

    fn open(self: Pin<&mut Self>, path: &QString) {
        self.go_to(PathBuf::from(path.to_string()));
    }

    fn reveal(mut self: Pin<&mut Self>, path: &QString, names: &QStringList) {
        let folder = PathBuf::from(path.to_string());
        let names: Vec<Vec<u8>> = QList::<QString>::from(names)
            .iter()
            .map(|name| name.to_string().into_bytes())
            .collect();
        let ready = self
            .rust()
            .current()
            .is_some_and(|snapshot| snapshot.path() == folder.as_path() && !snapshot.is_loading());
        if ready {
            self.change_selection(|selection, snapshot| {
                pane::select_names(selection, snapshot, &names);
            });
        } else {
            self.as_mut().rust_mut().select_on_arrival = Some(names);
            self.go_to(folder);
        }
    }

    /// Opens `path` as a new step in the history.
    pub fn go_to(mut self: Pin<&mut Self>, path: PathBuf) {
        if !self.as_mut().start() {
            return;
        }
        self.as_mut().set_location_text(&path);
        let generation = self
            .as_mut()
            .rust_mut()
            .model
            .as_mut()
            .map(|model| model.open(&path));
        if let Some(generation) = generation {
            self.as_mut().rust_mut().generation = generation;
            self.as_mut().note_asked("open", generation);
        }
        self.as_mut().set_failure(None);
        self.update_navigation();
    }

    /// Remembers when a listing was asked for, to log its first frame.
    fn note_asked(mut self: Pin<&mut Self>, what: &'static str, generation: u64) {
        if self.rust().frame_log {
            let mut rust = self.as_mut().rust_mut();
            rust.asked = Some((Instant::now(), what, generation));
            rust.shown = false;
        }
    }

    fn frame_swapped(mut self: Pin<&mut Self>) {
        let mut rust = self.as_mut().rust_mut();
        let Some((asked, what, _)) = rust.asked else {
            return;
        };
        if rust.shown {
            let rows = rust.snapshot.as_ref().map_or(0, |snapshot| snapshot.len());
            let loading = rust
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.is_loading());
            rust.asked = None;
            rust.shown = false;
            eprintln!(
                "rustphin: first entries ({what}): {:.1} ms to the frame showing {rows} rows{}, own memory {} kB",
                asked.elapsed().as_secs_f64() * 1e3,
                if loading { " (still loading)" } else { "" },
                rustphin_platform::process::own_memory().unwrap_or(0) / 1024,
            );
        }
    }

    fn bench_tick(mut self: Pin<&mut Self>) {
        const BENCH_FRAMES: usize = 1_200;
        let mut rust = self.as_mut().rust_mut();
        rust.frames.push(Instant::now());
        if rust.frames.len() == BENCH_FRAMES + 1 {
            let intervals: Vec<f64> = rust
                .frames
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).as_secs_f64() * 1e3)
                .collect();
            rust.frames.clear();
            let report = rustphin_view::pane::frame_report(&intervals);
            eprintln!("rustphin: scroll benchmark result: {report}");
        }
    }

    fn open_typed(mut self: Pin<&mut Self>, typed: &QString) -> bool {
        let typed = typed.to_string();
        let typed = typed.trim();
        if pane::names_trash(typed) {
            self.open_trash();
            return true;
        }
        if let Some(path) = rustphin_platform::uri::path_from_uri(typed.as_bytes()) {
            self.go_to(path);
            return true;
        }
        // A remote address is mounted through kio-fuse first (W-080).
        if typed.contains("://")
            && let Some(address) = rustphin_platform::remote::RemoteAddress::parse(typed)
        {
            let thread = self.qt_thread();
            crate::remote::open(address, move |mounted| {
                if let Some(path) = mounted {
                    let _ = thread.queue(move |model| model.go_to(path));
                }
            });
            return true;
        }
        let base = self.rust().folder();
        match pane::expand_path(typed, Some(&base)) {
            Some(path) => {
                self.as_mut().go_to(path);
                true
            }
            None => false,
        }
    }

    /// Runs one of the model's navigation steps.
    fn navigate(mut self: Pin<&mut Self>, step: fn(&mut DirectoryModel) -> Option<u64>) {
        let generation = self.as_mut().rust_mut().model.as_mut().and_then(step);
        if let Some(generation) = generation {
            self.as_mut().rust_mut().generation = generation;
            self.as_mut().note_asked("navigate", generation);
            if let Some(path) = self
                .rust()
                .model
                .as_ref()
                .and_then(DirectoryModel::location)
            {
                self.as_mut().set_location_text(&path);
            }
            self.as_mut().set_failure(None);
        }
        self.update_navigation();
    }

    fn back(self: Pin<&mut Self>) {
        self.navigate(DirectoryModel::back);
    }

    fn forward(self: Pin<&mut Self>) {
        self.navigate(DirectoryModel::forward);
    }

    fn up(mut self: Pin<&mut Self>) {
        // The folder left is selected in its parent, as in Dolphin.
        let came_from = self
            .rust()
            .model
            .as_ref()
            .and_then(DirectoryModel::location)
            .and_then(|path| path.file_name().map(|name| name.as_bytes().to_vec()));
        let moved = self
            .rust()
            .model
            .as_ref()
            .and_then(DirectoryModel::location)
            .is_some_and(|path| path.parent().is_some());
        if moved {
            self.as_mut().rust_mut().select_on_arrival = came_from.map(|name| vec![name]);
        }
        self.navigate(DirectoryModel::up);
    }

    fn reload(self: Pin<&mut Self>) {
        self.navigate(DirectoryModel::reload);
    }

    fn home(self: Pin<&mut Self>) {
        if let Some(home) = std::env::var_os("HOME") {
            self.go_to(PathBuf::from(home));
        }
    }

    fn open_trash(mut self: Pin<&mut Self>) {
        let folder = self
            .rust()
            .trash_store
            .as_ref()
            .map(|trash| trash.home_files_dir());
        match folder {
            Some(Ok(folder)) => self.go_to(folder),
            Some(Err(error)) => {
                self.as_mut()
                    .say(&text(&format!("The trash could not be opened: {error}.")));
            }
            None => self.as_mut().say(&text("There is no trash for this user.")),
        }
    }

    fn activate(mut self: Pin<&mut Self>, row: i32) {
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let Some(row) = usize::try_from(row)
            .ok()
            .filter(|row| *row < snapshot.len())
        else {
            return;
        };
        let path = snapshot.entry_path(row);
        if snapshot.is_container(row) {
            self.go_to(path);
        } else {
            self.as_mut().file_activated(path_text(&path));
        }
    }

    fn activate_selected(mut self: Pin<&mut Self>) {
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let rows = self.rust().selection.rows(&snapshot);
        // One folder opens here; files open through the window.
        if let [row] = rows[..] {
            return self.activate(i32::try_from(row).unwrap_or(-1));
        }
        for row in rows {
            if !snapshot.is_container(row) {
                self.as_mut()
                    .file_activated(path_text(&snapshot.entry_path(row)));
            }
        }
    }

    fn middle_click(mut self: Pin<&mut Self>, row: i32) {
        let target = self.rust().current().and_then(|snapshot| {
            let row = usize::try_from(row)
                .ok()
                .filter(|row| *row < snapshot.len())?;
            snapshot.is_container(row).then(|| snapshot.entry_path(row))
        });
        if let Some(path) = target {
            self.as_mut().open_in_new_tab(path_text(&path));
        }
    }

    fn run(self: Pin<&mut Self>, id: &QString) {
        match Item::from_id(&id.to_string()) {
            Some(Item::Back) => self.back(),
            Some(Item::Forward) => self.forward(),
            Some(Item::Up) => self.up(),
            Some(Item::Reload) => self.reload(),
            Some(Item::Hidden) => {
                let show = !*self.show_hidden();
                self.set_hidden(show);
            }
            Some(Item::Terminal) => {
                let folder = self.rust().folder();
                let command = prefs::settings().terminal().map(str::to_owned);
                let thread = self.qt_thread();
                std::thread::spawn(move || {
                    let result = rustphin_platform::terminal::open_in(
                        &folder,
                        command.as_deref(),
                        xdg::config_home(),
                    );
                    if let Err(error) = result {
                        let words = format!("The terminal could not be opened: {error}.");
                        let _ = thread.queue(move |model| model.say(&text(&words)));
                    }
                });
            }
            _ => {}
        }
    }

    fn set_hidden(mut self: Pin<&mut Self>, show: bool) {
        let options = ViewOptions {
            show_hidden: show,
            ..self.rust().options
        };
        self.as_mut().set_show_hidden(show);
        self.as_mut().apply_options(options);
        self.save_view();
    }

    /// Shows a message on the status line until the folder changes.
    fn say(mut self: Pin<&mut Self>, words: &QString) {
        self.as_mut().set_notice(words.clone());
    }

    /// Shows `path` as the location, with its crumbs and title.
    fn set_location_text(mut self: Pin<&mut Self>, path: &Path) {
        if !self.as_mut().rust_mut().pane.set_folder(path) {
            return;
        }
        self.as_mut().set_location(path_text(path));
        if !self.notice().is_empty() {
            self.as_mut().set_notice(QString::default());
        }
        let remote = crate::remote::root_of(path);
        let trail = pane::crumb_trail(
            path,
            remote
                .as_ref()
                .map(|(root, address)| (root.as_path(), address.as_str())),
        );
        self.as_mut()
            .set_crumb_labels(string_list(trail.iter().map(|(_, label)| label.as_str())));
        let paths: Vec<String> = trail
            .iter()
            .map(|(path, _)| path.to_string_lossy().into_owned())
            .collect();
        self.as_mut()
            .set_crumb_paths(string_list(paths.iter().map(String::as_str)));
        let title = path.file_name().map_or_else(
            || "/".to_owned(),
            |name| display_name(name.as_bytes()).into_owned(),
        );
        self.as_mut().set_title(text(&title));
    }

    /// Updates which navigation steps are possible.
    fn update_navigation(mut self: Pin<&mut Self>) {
        let (back, forward) = self.rust().model.as_ref().map_or((false, false), |model| {
            (model.can_go_back(), model.can_go_forward())
        });
        let up = self.rust().pane.folder().and_then(Path::parent).is_some();
        self.as_mut().set_can_go_back(back);
        self.as_mut().set_can_go_forward(forward);
        self.as_mut().set_can_go_up(up);
    }

    /// Finds out in the background whether the folder is a trash directory.
    fn detect_trash(mut self: Pin<&mut Self>, path: PathBuf) {
        self.as_mut().rust_mut().trash_dir = None;
        self.as_mut().set_trash(false);
        let Some(trash) = self.rust().trash_store.clone() else {
            return;
        };
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let dir = trash.dir_showing(&path);
            let _ = thread.queue(move |mut model| {
                if model.rust().pane.folder() == Some(path.as_path()) {
                    let label: Option<String> = dir.as_ref().map(|dir| pane::trash_label(dir));
                    model.as_mut().rust_mut().trash_dir = dir;
                    model.as_mut().set_trash(label.is_some());
                    if let Some(label) = label {
                        model.as_mut().set_title(text(&label));
                    }
                    model.as_mut().update_status();
                    model.rows_changed(&[Role::Thumbnail]);
                }
            });
        });
    }

    /// Reads how much space the folder's volume has left, in the background.
    fn measure_free_space(mut self: Pin<&mut Self>, path: PathBuf) {
        self.as_mut().rust_mut().usage = None;
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let usage = rustphin_platform::devices::usage(&path);
            let _ = thread.queue(move |mut model| {
                if let Some((total, free)) = usage {
                    model.as_mut().rust_mut().usage = Some((path, total, free));
                    model.update_status();
                }
            });
        });
    }

    /// Sets the model's options, if they changed.
    fn apply_options(mut self: Pin<&mut Self>, options: ViewOptions) {
        self.as_mut().rust_mut().options = options;
        let generation = self
            .as_mut()
            .rust_mut()
            .model
            .as_mut()
            .and_then(|model| model.set_options(options));
        if let Some(generation) = generation {
            self.as_mut().rust_mut().generation = generation;
            self.as_mut().note_asked("re-sort", generation);
        }
        self.as_mut().set_sort_column(column_of(options.sort));
        self.as_mut().set_descending(options.descending);
        self.as_mut().set_folders_first(options.folders_first);
        self.as_mut().set_show_hidden(options.show_hidden);
    }

    /// Tells the shared settings this pane's view: as the folder's own view
    /// while those are remembered, else as the default (D-029).
    fn save_view(self: Pin<&mut Self>) {
        let (mode, zoom, options) = (
            self.rust().view,
            self.rust().zoom_level,
            self.rust().options,
        );
        let location = self.rust().folder();
        prefs::with(|store| {
            if store.settings().per_folder() && !location.as_os_str().is_empty() {
                store.set_folder_view(
                    Location::Folder(location).to_saved(),
                    FolderView {
                        mode,
                        zoom,
                        options,
                    },
                );
            } else {
                store.update(|settings| settings.set_view(mode, zoom, options));
            }
        });
    }

    /// Shows `view` without saving it.
    fn apply_view(mut self: Pin<&mut Self>, view: FolderView) {
        if self.rust().view != view.mode || self.rust().zoom_level != view.zoom {
            self.as_mut().rust_mut().view = view.mode;
            self.as_mut().rust_mut().zoom_level = view.zoom.min(ZOOM_LEVELS - 1);
            self.as_mut().layout_changed();
        }
        let options = ViewOptions {
            details: view.mode == ViewMode::Details,
            ..view.options
        };
        self.apply_options(options);
    }

    /// On arriving in a folder while each folder keeps its view: its own
    /// view, or the default for one never set.
    fn adopt_folder_view(self: Pin<&mut Self>) {
        let location = self.rust().folder();
        let view = prefs::with(|store| {
            store.settings().per_folder().then(|| {
                store
                    .folder_view(&Location::Folder(location).to_saved())
                    .unwrap_or_else(|| default_view(store.settings()))
            })
        });
        if let Some(view) = view {
            self.apply_view(view);
        }
    }

    fn adopt_settings(mut self: Pin<&mut Self>) {
        let location = self.rust().folder();
        let (epoch, adopt, one_click, limit) = prefs::with(|store| {
            let own = store.settings().per_folder()
                && store
                    .folder_view(&Location::Folder(location).to_saved())
                    .is_some();
            (
                store.view_epoch(),
                (!own).then(|| default_view(store.settings())),
                store.opens_with_one_click(),
                store.settings().previews().limit_bytes(),
            )
        });
        if epoch != self.rust().view_epoch {
            self.as_mut().rust_mut().view_epoch = epoch;
            if let Some(view) = adopt {
                self.as_mut().apply_view(view);
            }
        }
        self.as_mut().set_one_click(one_click);
        {
            let rust = self.rust();
            let mut thumbs = rust.thumbs.borrow_mut();
            if thumbs.limit != limit {
                thumbs.limit = limit;
                if let Some(thumbnailer) = &thumbs.thumbnailer {
                    thumbnailer.set_limit(limit);
                }
                thumbs.known.clear();
                thumbs.waiting.clear();
            }
        }
        let width = self.rust().width;
        self.as_mut().set_width(f64::from(width));
        self.as_mut().update_status();
        self.rows_changed(&Role::ALL);
    }

    /// Publishes the sizes of the view mode and zoom level, and asks for
    /// thumbnails and icons at the new size.
    fn layout_changed(mut self: Pin<&mut Self>) {
        let (mode, zoom, icon, row, cell, name, line, data) = {
            let mut rust = self.as_mut().rust_mut();
            rust.apply_layout();
            (
                rust.mode,
                rust.zoom,
                rust.icon_size,
                rust.row_height,
                rust.cell_width,
                rust.name_text,
                rust.name_line,
                rust.data_text,
            )
        };
        self.as_mut().mode_changed();
        self.as_mut().zoom_changed();
        self.as_mut().icon_size_changed();
        self.as_mut().row_height_changed();
        self.as_mut().cell_width_changed();
        self.as_mut().name_text_changed();
        self.as_mut().name_line_changed();
        self.as_mut().data_text_changed();
        let _ = (mode, zoom, icon, row, cell, name, line, data);
        self.rows_changed(&[Role::Thumbnail, Role::Icon]);
    }

    fn sort_by(mut self: Pin<&mut Self>, column: i32) {
        let options = self.rust().options;
        let key = sort_key(column);
        let options = ViewOptions {
            sort: key,
            descending: options.sort == key && !options.descending,
            ..options
        };
        self.as_mut().apply_options(options);
        self.save_view();
    }

    fn set_sort(mut self: Pin<&mut Self>, column: i32, descending: bool) {
        let options = ViewOptions {
            sort: sort_key(column),
            descending,
            ..self.rust().options
        };
        self.as_mut().apply_options(options);
        self.save_view();
    }

    fn toggle_folders_first(mut self: Pin<&mut Self>) {
        let options = self.rust().options;
        let folders_first = !options.folders_first;
        self.as_mut().apply_options(ViewOptions {
            folders_first,
            ..options
        });
        self.as_mut().save_view();
        let words = if folders_first {
            "Folders are listed before files."
        } else {
            "Folders are sorted together with files."
        };
        self.say(&text(words));
    }

    fn choose_mode(mut self: Pin<&mut Self>, mode: i32) {
        let mode = mode_of(mode);
        if self.rust().view == mode {
            return;
        }
        self.as_mut().rust_mut().view = mode;
        self.as_mut().layout_changed();
        // Only the details view shows sizes and dates.
        let options = ViewOptions {
            details: mode == ViewMode::Details,
            ..self.rust().options
        };
        self.as_mut().apply_options(options);
        self.save_view();
    }

    fn zoom_by(self: Pin<&mut Self>, delta: i32) {
        let zoom = step_zoom(self.rust().zoom_level, delta as isize);
        self.choose_zoom(i32::try_from(zoom).unwrap_or(0));
    }

    fn choose_zoom(mut self: Pin<&mut Self>, zoom: i32) {
        let zoom = usize::try_from(zoom).unwrap_or(0).min(ZOOM_LEVELS - 1);
        if self.rust().zoom_level == zoom {
            return;
        }
        self.as_mut().rust_mut().zoom_level = zoom;
        self.as_mut().layout_changed();
        self.save_view();
    }

    fn set_width(mut self: Pin<&mut Self>, width: f64) {
        let width = width as f32;
        self.as_mut().rust_mut().width = width;
        let columns = pane::Columns::for_width(width).within(prefs::settings().detail_columns());
        self.as_mut().set_show_size(columns.size);
        self.as_mut().set_show_modified(columns.modified);
        self.as_mut().set_show_type(columns.kind);
        self.update_status();
    }

    /// Where the entries sit, `lanes` to a line or column.
    fn grid(&self, lanes: i32) -> Option<Grid> {
        let snapshot = self.rust().current()?;
        Some(Grid::new(
            self.rust().view.layout(self.rust().zoom_level),
            self.rust().view.flow(),
            usize::try_from(lanes).unwrap_or(1),
            snapshot.len(),
        ))
    }

    fn click(self: Pin<&mut Self>, row: i32, modifiers: i32) {
        let Ok(row) = usize::try_from(row) else {
            return self.clear_selection();
        };
        self.change_selection(|selection, snapshot| {
            if row >= snapshot.len() {
                return;
            }
            if modifiers & 2 != 0 {
                selection.extend_to(snapshot, row);
            } else if modifiers & 1 != 0 {
                selection.toggle(snapshot, row);
            } else {
                selection.select_only(snapshot, row);
            }
        });
    }

    fn press(self: Pin<&mut Self>, row: i32, modifiers: i32) {
        // A press on a selected entry keeps the selection, so it can be
        // dragged; the click that follows without a drag narrows it.
        if modifiers == 0 && self.is_selected(row) {
            return;
        }
        self.click(row, modifiers);
    }

    fn move_focus(self: Pin<&mut Self>, step: i32, extend: bool, lanes: i32) {
        let (Some(step), Some(grid)) = (step_of(step), self.grid(lanes)) else {
            return;
        };
        let from = self
            .rust()
            .current()
            .and_then(|snapshot| self.rust().selection.focus_row(snapshot));
        let Some(row) = grid.step(from, step) else {
            return;
        };
        self.change_selection(|selection, snapshot| {
            if extend {
                selection.extend_to(snapshot, row);
            } else {
                selection.select_only(snapshot, row);
            }
        });
    }

    fn begin_band(mut self: Pin<&mut Self>) {
        let kept = self.rust().selection.clone();
        self.as_mut().rust_mut().band_kept = kept;
    }

    #[allow(clippy::too_many_arguments)] // The band's corners, as QML has them.
    fn band(self: Pin<&mut Self>, x0: f64, y0: f64, x1: f64, y1: f64, lanes: i32, add: bool) {
        let Some(grid) = self.grid(lanes) else {
            return;
        };
        let band = Rect::between((x0 as f32, y0 as f32), (x1 as f32, y1 as f32));
        let rows = grid.rows_in(band);
        let kept = self.rust().band_kept.clone();
        self.change_selection(|selection, snapshot| {
            if add {
                *selection = kept;
                selection.select_rows(snapshot, rows, true);
            } else {
                selection.select_rows(snapshot, rows, false);
            }
        });
    }

    fn select_all(self: Pin<&mut Self>) {
        self.change_selection(Selection::select_all);
    }

    fn invert_selection(self: Pin<&mut Self>) {
        self.change_selection(Selection::invert);
    }

    fn clear_selection(self: Pin<&mut Self>) {
        self.change_selection(|selection, _| selection.clear());
    }

    fn type_ahead(mut self: Pin<&mut Self>, typed: &QString) {
        let typed = typed.to_string();
        if typed.is_empty() || typed.chars().any(char::is_control) {
            return;
        }
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let focus = self.rust().selection.focus_row(&snapshot);
        let found =
            self.as_mut()
                .rust_mut()
                .type_ahead
                .find(&typed, Instant::now(), &snapshot, focus);
        if let Some(row) = found {
            self.change_selection(|selection, snapshot| selection.select_only(snapshot, row));
        }
    }

    fn begin_rename(mut self: Pin<&mut Self>) {
        if self.rust().trash_dir.is_some() || self.as_mut().offer_batch_rename() {
            return;
        }
        let name = self.rust().current().and_then(|snapshot| {
            let row = self.rust().selection.focus_row(snapshot)?;
            self.rust()
                .selection
                .contains(snapshot, row)
                .then(|| snapshot.name(row).to_vec())
        });
        if let Some(name) = name {
            self.as_mut().rename_name(name);
        }
    }

    /// Shows the name field over the entry called `name`, with the part
    /// before the extension to be selected.
    fn rename_name(mut self: Pin<&mut Self>, name: Vec<u8>) {
        let Some((row, folder)) = self.rust().current().and_then(|snapshot| {
            let row = snapshot.row_of(&name)?;
            Some((row, snapshot.is_container(row)))
        }) else {
            return;
        };
        let shown = display_name(&name).into_owned();
        let stem = stem_len(&shown, folder);
        self.as_mut().rust_mut().renaming = Some(name);
        self.as_mut().set_rename_text(text(&shown));
        self.as_mut()
            .set_rename_stem(i32::try_from(shown[..stem].chars().count()).unwrap_or(0));
        self.as_mut()
            .set_renaming_row(i32::try_from(row).unwrap_or(-1));
        self.as_mut()
            .row_changed(row, &[Role::Renaming, Role::Selected, Role::Current]);
        self.update_status();
    }

    fn finish_rename(mut self: Pin<&mut Self>, typed: &QString, apply: bool) {
        let Some(old) = self.as_mut().rust_mut().renaming.take() else {
            return;
        };
        let typed = typed.to_string();
        let unchanged = typed.is_empty() || typed == display_name(&old);
        let path = self.rust().pane.entry_path(&old);
        if let Some(path) = path.filter(|_| apply && !unchanged) {
            self.as_mut().rust_mut().select_when_seen = Some((typed.clone().into_bytes(), false));
            crate::hub::start(rustphin_core::ops::Request::Rename {
                path,
                new_name: typed.into(),
            });
        }
        let row = *self.renaming_row();
        self.as_mut().set_renaming_row(-1);
        if let Ok(row) = usize::try_from(row) {
            self.as_mut()
                .row_changed(row, &[Role::Renaming, Role::Selected, Role::Current]);
        }
        self.update_status();
    }

    fn set_filter(mut self: Pin<&mut Self>, typed: &QString) {
        let words = typed.to_string();
        self.as_mut().set_filter_text(typed.clone());
        let generation = self
            .as_mut()
            .rust_mut()
            .model
            .as_mut()
            .and_then(|model| model.set_filter(NameFilter::new(&words)));
        if let Some(generation) = generation {
            self.as_mut().rust_mut().generation = generation;
        }
    }

    fn selected_paths(&self) -> QStringList {
        let paths: Vec<String> = self.rust().current().map_or_else(Vec::new, |snapshot| {
            self.rust()
                .selection
                .rows(snapshot)
                .into_iter()
                .map(|row| snapshot.entry_path(row).to_string_lossy().into_owned())
                .collect()
        });
        string_list(paths.iter().map(String::as_str))
    }

    fn path_at(&self, row: i32) -> QString {
        self.rust()
            .current()
            .and_then(|snapshot| {
                let row = usize::try_from(row)
                    .ok()
                    .filter(|row| *row < snapshot.len())?;
                Some(path_text(&snapshot.entry_path(row)))
            })
            .unwrap_or_default()
    }

    fn icon_at(&self, row: i32) -> QString {
        let rust = self.rust();
        let Some(snapshot) = rust.current() else {
            return QString::default();
        };
        let Some(row) = usize::try_from(row)
            .ok()
            .filter(|row| *row < snapshot.len())
        else {
            return QString::default();
        };
        QString::from(&entry_icon(rust.types.as_deref(), snapshot, row))
    }

    fn selected_uris(&self) -> QString {
        let paths: Vec<PathBuf> = self.rust().current().map_or_else(Vec::new, |snapshot| {
            self.rust()
                .selection
                .rows(snapshot)
                .into_iter()
                .map(|row| snapshot.entry_path(row))
                .collect()
        });
        QString::from(&rustphin_platform::uri::uri_list(&paths))
    }

    fn drop_row(&self, row: i32) -> i32 {
        let rust = self.rust();
        let folder = rust.trash_dir.is_none()
            && rust.current().is_some_and(|snapshot| {
                usize::try_from(row)
                    .ok()
                    .is_some_and(|row| row < snapshot.len() && snapshot.is_container(row))
            });
        if folder { row } else { -1 }
    }

    fn takes_drops(&self) -> bool {
        self.rust().trash_dir.is_none() && self.rust().current().is_some()
    }

    fn drop_on(self: Pin<&mut Self>, row: i32, uris: &QString, action: i32) {
        if !self.takes_drops() {
            return;
        }
        let into = match usize::try_from(self.drop_row(row)) {
            Ok(row) => self
                .rust()
                .current()
                .map(|snapshot| snapshot.entry_path(row)),
            Err(_) => Some(self.rust().folder()),
        };
        let Some(into) = into else { return };
        let sources = rustphin_platform::uri::paths_from_uri_list(uris.to_string().as_bytes());
        crate::hub::drop_files(sources, into, drop_action(action));
    }

    fn is_selected(&self, row: i32) -> bool {
        self.rust().current().is_some_and(|snapshot| {
            usize::try_from(row)
                .ok()
                .filter(|row| *row < snapshot.len())
                .is_some_and(|row| self.rust().selection.contains(snapshot, row))
        })
    }
}

impl qobject::FolderModel {
    pub(crate) fn hub_changed(mut self: Pin<&mut Self>) {
        let (clip, created, admin) =
            crate::hub::with(|hub| (hub.clip_serial, hub.created.clone(), hub.admin_changes));
        if self.rust().clip_serial != clip {
            self.as_mut().rust_mut().clip_serial = clip;
            self.as_mut().rows_changed(&[Role::Opacity]);
        }
        if self.rust().created_serial != created.0 {
            self.as_mut().rust_mut().created_serial = created.0;
            let location = self.rust().folder();
            let names: Vec<Vec<u8>> = created
                .1
                .iter()
                .filter(|path| path.parent() == Some(location.as_path()))
                .filter_map(|path| path.file_name().map(|name| name.as_bytes().to_vec()))
                .collect();
            if !names.is_empty() {
                self.as_mut().rust_mut().select_all_when_seen = names;
            }
        }
        if self.rust().admin_changes != admin {
            self.as_mut().rust_mut().admin_changes = admin;
            self.as_mut().reload();
        }
        self.switch_filesystem();
    }

    /// What a question lists for the selected entries.
    fn listed_selection(&self) -> (Vec<PathBuf>, Vec<Listed>) {
        let Some(snapshot) = self.rust().current() else {
            return Default::default();
        };
        self.rust()
            .selection
            .rows(snapshot)
            .into_iter()
            .map(|row| {
                let folder = snapshot.is_container(row);
                (
                    snapshot.entry_path(row),
                    Listed {
                        folder,
                        size: (!folder)
                            .then(|| snapshot.metadata(row).map(|meta| meta.size))
                            .flatten(),
                    },
                )
            })
            .unzip()
    }

    fn copy_selected(self: Pin<&mut Self>, cut: bool) {
        let (paths, _) = self.listed_selection();
        crate::hub::pick_up(paths, cut);
    }

    fn paste(self: Pin<&mut Self>) {
        if self.rust().trash_dir.is_none() && self.rust().current().is_some() {
            crate::hub::paste(self.rust().folder());
        }
    }

    fn trash_selected(self: Pin<&mut Self>) {
        let (paths, listed) = self.listed_selection();
        // What is already in the trash can only be deleted for good; so can
        // anything as administrator: KDE's helper has no trash.
        if self.rust().trash_dir.is_some() || self.rust().as_admin {
            crate::hub::delete(paths, listed);
        } else {
            crate::hub::trash(paths, listed);
        }
    }

    fn delete_selected(self: Pin<&mut Self>) {
        let (paths, listed) = self.listed_selection();
        crate::hub::delete(paths, listed);
    }

    fn duplicate_selected(mut self: Pin<&mut Self>) {
        if self.rust().trash_dir.is_some() {
            return;
        }
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let rows = self.rust().selection.rows(&snapshot);
        if rows.is_empty() {
            return;
        }
        let names: Vec<Vec<u8>> = rows
            .iter()
            .map(|&row| snapshot.name(row).to_vec())
            .collect();
        let sources = rows.iter().map(|&row| snapshot.entry_path(row)).collect();
        // Hidden or filtered entries still take a name.
        let listed = snapshot.listed_names();
        self.as_mut().rust_mut().select_all_when_seen =
            duplicate_names(&names, |name| listed.contains(name));
        crate::hub::start(rustphin_core::ops::Request::Duplicate { sources });
    }

    fn restore_selected(self: Pin<&mut Self>) {
        let (contents, _) = self.listed_selection();
        if let Some(dir) = self.rust().trash_dir.clone()
            && !contents.is_empty()
        {
            crate::hub::restore(dir, contents);
        }
    }

    fn empty_trash(self: Pin<&mut Self>) {
        if let Some(dir) = self.rust().trash_dir.clone() {
            crate::hub::empty_trash(dir);
        }
    }

    fn create_entry(mut self: Pin<&mut Self>, folder: bool) {
        if self.rust().trash_dir.is_some() {
            return;
        }
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let parent = snapshot.path().to_owned();
        let base = if folder { "New Folder" } else { "New File" };
        let name = free_name(base, |name| snapshot.row_of(name.as_bytes()).is_some());
        self.as_mut().rust_mut().select_when_seen = Some((name.clone().into_bytes(), true));
        let name = name.into();
        crate::hub::start(if folder {
            rustphin_core::ops::Request::CreateDir { parent, name }
        } else {
            rustphin_core::ops::Request::CreateFile { parent, name }
        });
    }
}

/// Entries handed to the window's dialogs (Compress, batch rename,
/// Extract to): QML gets a ticket, the paths stay here as bytes.
/// What a drop does, from QML's number: 1 copy, 2 move, else ask.
pub(crate) fn drop_action(action: i32) -> Option<rustphin_view::operations::DropAction> {
    use rustphin_view::operations::DropAction;
    match action {
        1 => Some(DropAction::Copy),
        2 => Some(DropAction::Move),
        _ => None,
    }
}

pub mod handed {
    use std::{cell::RefCell, ffi::OsString, path::PathBuf};

    /// What a dialog acts on.
    #[derive(Debug, Clone, Default)]
    pub struct Handed {
        /// The folder.
        pub folder: PathBuf,
        /// The entries' paths.
        pub sources: Vec<PathBuf>,
        /// Names in the folder that are not among them (batch rename).
        pub staying: Vec<OsString>,
        /// Every name in the folder, as bytes (Compress avoids them).
        pub taken: Vec<Vec<u8>>,
        /// For Extract to: each archive and whether Rustphin opens it.
        pub archives: Vec<PathBuf>,
        /// Extract to: trash the archives afterwards.
        pub trash: bool,
    }

    thread_local! {
        static HANDED: RefCell<Option<Handed>> = const { RefCell::new(None) };
    }

    /// Keeps `handed` for the dialog about to open.
    pub fn keep(handed: Handed) {
        HANDED.with(|slot| *slot.borrow_mut() = Some(handed));
    }

    /// What the open dialog acts on; it is handed over once.
    pub fn take() -> Option<Handed> {
        HANDED.with(|slot| slot.borrow_mut().take())
    }
}

impl qobject::FolderModel {
    /// The facts the context menu is built from.
    fn menu_facts(&self, on_selection: bool) -> rustphin_view::menu::MenuFacts {
        let rust = self.rust();
        let snapshot = rust.current().filter(|_| on_selection);
        let rows = snapshot
            .map(|snapshot| rust.selection.rows(snapshot))
            .unwrap_or_default();
        let single_container =
            rows.len() == 1 && snapshot.is_some_and(|snapshot| snapshot.is_container(rows[0]));
        let applications = match (rows.as_slice(), snapshot, rust.types.as_ref()) {
            ([row], Some(snapshot), Some(types)) if !single_container => types
                .applications(types.mimes.mime_type(snapshot.name(*row), false))
                .into_iter()
                .map(|app| rustphin_view::menu::AppChoice {
                    icon: types.app_icon(&app.id),
                    id: app.id,
                    name: app.name,
                })
                .collect(),
            _ => Vec::new(),
        };
        let can_paste = crate::hub::with(|hub| hub.has_clip());
        let (all_archives, extract_folder, compress_names) = match snapshot {
            Some(snapshot) if !rows.is_empty() => {
                let all_archives = rows.iter().all(|&row| {
                    !snapshot.is_container(row) && self.archive_opener(snapshot, row).is_some()
                });
                let extract_folder = (rows.len() == 1)
                    .then(|| rustphin_view::archives::folder_for(snapshot.name(rows[0])));
                let entries: Vec<(&[u8], bool)> = rows
                    .iter()
                    .map(|&row| (snapshot.name(row), snapshot.is_container(row)))
                    .collect();
                let listed = snapshot.listed_names();
                (
                    all_archives,
                    extract_folder,
                    rustphin_view::archives::quick_names(&entries, |name| listed.contains(name)),
                )
            }
            _ => (false, None, Default::default()),
        };
        rustphin_view::menu::MenuFacts {
            on_selection,
            count: rows.len(),
            single_container,
            in_trash: rust.trash_dir.is_some(),
            as_administrator: crate::admin::active(),
            can_paste,
            applications,
            types_ready: rust.types.is_some(),
            all_archives,
            extract_folder,
            compress_names,
        }
    }

    fn command(mut self: Pin<&mut Self>, id: &QString, lanes: i32) -> bool {
        use rustphin_view::menu::{CompressHow, ExtractHow};
        let id = id.to_string();
        let step = |id: &str| -> Option<(i32, bool)> {
            Some(match id {
                "select-previous" => (0, false),
                "select-next" => (1, false),
                "select-left" => (2, false),
                "select-right" => (3, false),
                "page-up" => (4, false),
                "page-down" => (5, false),
                "select-first" => (6, false),
                "select-last" => (7, false),
                "extend-previous" => (0, true),
                "extend-next" => (1, true),
                "extend-left" => (2, true),
                "extend-right" => (3, true),
                "extend-page-up" => (4, true),
                "extend-page-down" => (5, true),
                "extend-first" => (6, true),
                "extend-last" => (7, true),
                _ => return None,
            })
        };
        if let Some((step, extend)) = step(&id) {
            self.move_focus(step, extend, lanes);
            return true;
        }
        match id.as_str() {
            "go-back" => self.back(),
            "go-forward" => self.forward(),
            "go-up" => self.up(),
            "go-home" => self.home(),
            "reload" => self.reload(),
            "open-terminal" => self.run(&QString::from("terminal")),
            "activate" => self.activate_selected(),
            "show-icons" => self.choose_mode(0),
            "show-compact" => self.choose_mode(1),
            "show-details" => self.choose_mode(2),
            "toggle-hidden" => self.run(&QString::from("hidden")),
            "toggle-folders-first" => self.toggle_folders_first(),
            "zoom-in" => self.zoom_by(1),
            "zoom-out" => self.zoom_by(-1),
            "zoom-reset" => {
                self.choose_zoom(i32::try_from(rustphin_view::layout::DEFAULT_ZOOM).unwrap_or(2))
            }
            "sort-name" => self.sort_by(0),
            "sort-size" => self.sort_by(1),
            "sort-modified" => self.sort_by(2),
            "sort-type" => self.sort_by(3),
            "new-folder" => self.create_entry(true),
            "new-file" => self.create_entry(false),
            "rename" => self.begin_rename(),
            "recover-renames" => {
                if self.rust().trash_dir.is_none() && self.rust().current().is_some() {
                    crate::hub::start(rustphin_core::ops::Request::RecoverRenameBatch {
                        directory: self.rust().folder(),
                    });
                }
            }
            "copy" => self.copy_selected(false),
            "cut" => self.copy_selected(true),
            "paste" => self.paste(),
            "copy-location" => self.copy_location(),
            "duplicate" => self.duplicate_selected(),
            "extract-here" => self.extract_selected(ExtractHow::Here),
            "extract-into-folder" => self.extract_selected(ExtractHow::NewFolder),
            "extract-and-trash" => self.extract_selected(ExtractHow::AndTrash),
            "extract-to" => self.extract_selected(ExtractHow::To),
            "compress-zip" => self.compress_selected(CompressHow::Zip),
            "compress-tar-gz" => self.compress_selected(CompressHow::TarGz),
            "compress-choose" => self.compress_selected(CompressHow::Choose),
            "move-to-trash" => self.trash_selected(),
            "delete-permanently" => self.delete_selected(),
            "open-trash" => self.open_trash(),
            "restore-trash" => self.restore_selected(),
            "empty-trash" => self.empty_trash(),
            "select-all" => self.select_all(),
            "invert-selection" => self.invert_selection(),
            "cancel-edit" => self.clear_selection(),
            "show-properties" => {
                let (paths, _) = self.listed_selection();
                let list: QList<QString> = paths.iter().map(|path| path_text(path)).collect();
                self.as_mut().properties_requested(QStringList::from(&list));
            }
            _ => return false,
        }
        true
    }

    /// Which tool opens the archive at `row`, if any.
    fn archive_opener(
        &self,
        snapshot: &Snapshot,
        row: usize,
    ) -> Option<rustphin_view::archives::Opener> {
        let name = snapshot.name(row);
        let mime = self
            .rust()
            .types
            .as_ref()
            .map(|types| types.mimes.mime_type(name, false));
        rustphin_view::archives::opener(name, mime)
    }

    fn open_menu(mut self: Pin<&mut Self>, which: &QString) -> bool {
        use rustphin_view::menu::{self, TemplateKind};
        let entries = match which.to_string().as_str() {
            "new" => {
                if self.rust().trash_dir.is_some() || self.rust().current().is_none() {
                    return false;
                }
                let templates: Vec<(String, TemplateKind)> = self
                    .rust()
                    .new_files
                    .iter()
                    .map(|file| {
                        use rustphin_platform::templates::BuiltIn;
                        let kind = match file.built_in {
                            Some(BuiltIn::Text) => TemplateKind::Text,
                            Some(BuiltIn::Empty) => TemplateKind::Empty,
                            Some(BuiltIn::Html) => TemplateKind::Html,
                            None => TemplateKind::Other,
                        };
                        (file.label.clone(), kind)
                    })
                    .collect();
                menu::new_menu(&templates)
            }
            "trash" => {
                let Some(dir) = self.rust().trash_dir.clone() else {
                    return false;
                };
                let others: Vec<(String, PathBuf)> = self
                    .rust()
                    .trash_store
                    .as_ref()
                    .map(|trash| {
                        trash
                            .dirs()
                            .into_iter()
                            .filter(|other| *other != dir)
                            .map(|other| (pane::trash_label(&other), other.join("files")))
                            .collect()
                    })
                    .unwrap_or_default();
                menu::trash_actions(!self.rust().selection.is_empty(), others)
            }
            which => menu::context_menu(&self.menu_facts(which == "selection")),
        };
        if entries.is_empty() {
            return false;
        }
        let json = menu::to_json(&entries).to_string();
        self.as_mut().rust_mut().menu_entries = entries;
        self.as_mut().set_menu(QString::from(&json));
        true
    }

    fn run_menu(mut self: Pin<&mut Self>, path: &QString) {
        use rustphin_view::menu::{self, MenuCommand};
        let command = menu::command_at(&self.rust().menu_entries, &path.to_string()).cloned();
        self.as_mut().rust_mut().menu_entries.clear();
        self.as_mut().set_menu(QString::default());
        let Some(command) = command else {
            return;
        };
        match command {
            MenuCommand::Open => self.activate_selected(),
            MenuCommand::OpenWith(app) => {
                let (paths, _) = self.listed_selection();
                if let [path] = paths.as_slice() {
                    let (id, path) = (app.id, path.clone());
                    std::thread::spawn(move || {
                        if let Err(error) = rustphin_platform::apps::open_with(&id, &path) {
                            crate::hub::say(error.to_string(), true);
                        }
                    });
                }
            }
            MenuCommand::Cut => self.copy_selected(true),
            MenuCommand::Copy => self.copy_selected(false),
            MenuCommand::Paste => self.paste(),
            MenuCommand::CopyLocation => self.copy_location(),
            MenuCommand::Duplicate => self.duplicate_selected(),
            MenuCommand::Rename => self.begin_rename(),
            MenuCommand::MoveToTrash => self.trash_selected(),
            MenuCommand::Delete => self.delete_selected(),
            MenuCommand::Restore => self.restore_selected(),
            MenuCommand::EmptyTrash => self.empty_trash(),
            MenuCommand::OpenTrash(files) => self.go_to(files),
            MenuCommand::NewFolder => self.create_entry(true),
            MenuCommand::NewFile => self.create_entry(false),
            MenuCommand::NewFromTemplate { index, .. } => self.create_from_template(index),
            MenuCommand::Properties => {
                let (paths, _) = self.listed_selection();
                let list: QList<QString> = paths.iter().map(|path| path_text(path)).collect();
                self.as_mut().properties_requested(QStringList::from(&list));
            }
            MenuCommand::Extract(how) => self.extract_selected(how),
            MenuCommand::Compress(how) => self.compress_selected(how),
        }
    }

    fn copy_location(mut self: Pin<&mut Self>) {
        let (paths, _) = self.listed_selection();
        let Some(path) = paths.into_iter().next() else {
            return crate::hub::say("Select an item to copy its location.".to_owned(), false);
        };
        let text =
            crate::remote::address_of(&path).unwrap_or_else(|| path.to_string_lossy().into_owned());
        self.as_mut().copy_text(QString::from(&text));
        let name = path
            .file_name()
            .map(|name| display_name(name.as_bytes()).into_owned())
            .unwrap_or_default();
        crate::hub::say(
            format!("Copied the location of \u{201c}{name}\u{201d}."),
            false,
        );
    }

    fn create_from_template(mut self: Pin<&mut Self>, index: usize) {
        if self.rust().trash_dir.is_some() {
            return;
        }
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let Some(file) = self.rust().new_files.get(index).cloned() else {
            return;
        };
        let name = rustphin_platform::templates::free_file_name(&file.file_name, |name| {
            snapshot.row_of(name.as_bytes()).is_some()
        });
        self.as_mut().rust_mut().select_when_seen = Some((name.clone().into_bytes(), true));
        crate::hub::start(rustphin_core::ops::Request::CreateFromTemplate {
            parent: snapshot.path().to_owned(),
            name: name.into(),
            template: file.template,
        });
    }

    fn extract_selected(mut self: Pin<&mut Self>, how: rustphin_view::menu::ExtractHow) {
        use rustphin_core::ops::Layout;
        use rustphin_view::menu::ExtractHow;
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let rows = self.rust().selection.rows(&snapshot);
        let archives: Vec<PathBuf> = rows
            .iter()
            .filter(|&&row| {
                !snapshot.is_container(row) && self.archive_opener(&snapshot, row).is_some()
            })
            .map(|&row| snapshot.entry_path(row))
            .collect();
        if archives.is_empty() {
            return crate::hub::say("Select an archive to extract.".to_owned(), false);
        }
        let here = snapshot.path().to_owned();
        let (layout, trash) = match how {
            ExtractHow::Here | ExtractHow::To => (Layout::Here, false),
            ExtractHow::NewFolder => (Layout::NewFolder, false),
            ExtractHow::AndTrash => (Layout::Here, true),
        };
        if how == ExtractHow::To {
            handed::keep(handed::Handed {
                folder: here.clone(),
                archives,
                trash,
                ..handed::Handed::default()
            });
            let json = serde_json::json!({
                "folder": here.to_string_lossy(),
                "uri": rustphin_platform::uri::file_uri(&here),
            })
            .to_string();
            return self.as_mut().extract_to_requested(QString::from(&json));
        }
        start_extracting(archives, here, layout, trash);
    }

    fn compress_selected(mut self: Pin<&mut Self>, how: rustphin_view::menu::CompressHow) {
        use rustphin_core::archive::{Format, Level};
        use rustphin_view::menu::CompressHow;
        let Some(snapshot) = self.rust().current().cloned() else {
            return;
        };
        let rows = self.rust().selection.rows(&snapshot);
        if rows.is_empty() {
            return crate::hub::say("Select what to compress.".to_owned(), false);
        }
        let folder = snapshot.path().to_owned();
        let sources: Vec<PathBuf> = rows.iter().map(|&row| snapshot.entry_path(row)).collect();
        let entries: Vec<(&[u8], bool)> = rows
            .iter()
            .map(|&row| (snapshot.name(row), snapshot.is_container(row)))
            .collect();
        let listed = snapshot.listed_names();
        let [zip, tar_gz] =
            rustphin_view::archives::quick_names(&entries, |name| listed.contains(name));
        let (format, name) = match how {
            CompressHow::Zip => (Format::Zip, zip),
            CompressHow::TarGz => (Format::TarGz, tar_gz),
            CompressHow::Choose => {
                let base = rustphin_view::archives::compress_base(&entries);
                handed::keep(handed::Handed {
                    folder: folder.clone(),
                    sources,
                    taken: listed.iter().map(|name| name.to_vec()).collect(),
                    ..handed::Handed::default()
                });
                let json = serde_json::json!({
                    "folder": folder.to_string_lossy(),
                    "base": base,
                    "count": rows.len(),
                })
                .to_string();
                return self.as_mut().compress_requested(QString::from(&json));
            }
        };
        crate::hub::start(rustphin_core::ops::Request::Compress {
            sources,
            archive: folder.join(name),
            format,
            level: Level::Normal,
        });
    }

    /// Several selected: F2 renames them together in the batch dialog.
    fn offer_batch_rename(mut self: Pin<&mut Self>) -> bool {
        let Some(snapshot) = self.rust().current().cloned() else {
            return false;
        };
        let rows = self.rust().selection.rows(&snapshot);
        if rows.len() < 2 {
            return false;
        }
        let chosen: std::collections::HashSet<&[u8]> =
            rows.iter().map(|&row| snapshot.name(row)).collect();
        let staying = (0..snapshot.len())
            .filter(|row| !chosen.contains(snapshot.name(*row)))
            .map(|row| OsStr::from_bytes(snapshot.name(row)).to_owned())
            .collect();
        handed::keep(handed::Handed {
            folder: snapshot.path().to_owned(),
            sources: rows.iter().map(|&row| snapshot.entry_path(row)).collect(),
            staying,
            ..handed::Handed::default()
        });
        let json = serde_json::json!({
            "folder": snapshot.path().to_string_lossy(),
            "count": rows.len(),
        })
        .to_string();
        self.as_mut().batch_rename_requested(QString::from(&json));
        true
    }
}

/// Extracts each archive into `into` through the operation engine: its
/// own formats, and the others through libarchive or 7-Zip (D-036).
pub fn start_extracting(
    archives: Vec<PathBuf>,
    into: PathBuf,
    layout: rustphin_core::ops::Layout,
    trash: bool,
) {
    for archive in archives {
        crate::hub::start(rustphin_core::ops::Request::Extract {
            archive,
            into: into.clone(),
            layout,
            trash_archive: trash,
        });
    }
}

/// The default view as the settings hold it.
fn default_view(settings: &rustphin_view::settings::Settings) -> FolderView {
    FolderView {
        mode: settings.mode(),
        zoom: settings.zoom(),
        options: settings.options(),
    }
}

/// Whether a flow scrolls sideways (the compact view).
#[allow(dead_code)]
fn sideways(flow: Flow) -> bool {
    flow == Flow::Down
}

#[cfg(test)]
mod tests {
    use rustphin_view::operations::DropAction;

    #[test]
    fn a_drop_copies_with_ctrl_moves_with_shift_and_otherwise_asks() {
        assert_eq!(super::drop_action(1), Some(DropAction::Copy));
        assert_eq!(super::drop_action(2), Some(DropAction::Move));
        assert_eq!(super::drop_action(0), None);
        assert_eq!(super::drop_action(7), None);
    }

    use super::*;

    #[test]
    fn roles_are_found_by_number_and_named_once() {
        for role in Role::ALL {
            assert_eq!(Role::from_number(role as i32), Some(role));
        }
        assert_eq!(Role::from_number(0), None);
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Role::ALL.len());
    }

    #[test]
    fn an_icon_falls_back_from_the_types_names_to_the_generic_one() {
        let names = vec!["text-plain".to_owned(), "text-x-generic".to_owned()];
        assert_eq!(
            icon_id(&names, false),
            "text-plain|text-x-generic|text-x-generic"
        );
        assert_eq!(icon_id(&[], true), "folder");
    }

    #[test]
    fn every_interface_icon_is_bundled() {
        let icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
        let missing: Vec<String> = rustphin_view::icons::Icon::ALL
            .iter()
            .map(|icon| icon.lucide_name())
            .filter(|name| !icons.join(format!("{name}.svg")).is_file())
            .collect();
        assert!(missing.is_empty(), "not in icons/: {missing:?}");
    }

    #[test]
    fn columns_and_sorts_name_each_other() {
        for column in 0..4 {
            assert_eq!(column_of(sort_key(column)), column);
        }
        assert_eq!(sort_key(9), SortKey::Name);
    }

    #[test]
    fn modes_and_steps_are_numbered_as_qml_sends_them() {
        for mode in [ViewMode::Icons, ViewMode::Compact, ViewMode::Details] {
            assert_eq!(mode_of(mode_number(mode)), mode);
        }
        assert_eq!(step_of(0), Some(Step::Up));
        assert_eq!(step_of(7), Some(Step::Last));
        assert_eq!(step_of(8), None);
    }

    #[test]
    fn only_decodable_pictures_get_thumbnails() {
        assert!(has_thumbnail("image/png"));
        assert!(!has_thumbnail("image/svg+xml"));
        assert!(!has_thumbnail("video/mp4"));
    }
}
