//! One folder's entries as a QML list model (W-114c): rows are made on
//! demand from the core's newest snapshot, which arrives from the core's
//! own threads and is swapped in on the Qt thread (D-009). No file is read
//! here.

// cxx-qt generates the unsafe FFI glue between Rust and the C++ QObject
// for the list model; nothing in the bridge file is written by hand but
// declarations, and the code here stays under the lint (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "folder_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use jiff::tz::TimeZone;
use rustphin_core::{DirectoryModel, ModelEvent, RealFs, Snapshot, SortKey, ViewOptions};
use rustphin_platform::{mime::MimeDb, xdg};
use rustphin_view::format;

/// The roles QML reads, after Qt's own (`Qt::UserRole` is 0x100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Name = 0x100,
    Size,
    Modified,
    Folder,
    Hidden,
    Icon,
}

impl Role {
    const ALL: [Role; 6] = [
        Role::Name,
        Role::Size,
        Role::Modified,
        Role::Folder,
        Role::Hidden,
        Role::Icon,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Name => "name",
            Role::Size => "size",
            Role::Modified => "modified",
            Role::Folder => "folder",
            Role::Hidden => "hidden",
            Role::Icon => "icon",
        }
    }
}

/// The Rust side of `FolderModel`.
pub struct FolderModelRust {
    count: i32,
    loading: bool,
    location: QString,
    summary: QString,
    problem: QString,
    can_go_back: bool,
    can_go_forward: bool,
    can_go_up: bool,
    model: Option<DirectoryModel>,
    snapshot: Option<Arc<Snapshot>>,
    /// The file types, once loaded in the background.
    types: Option<Arc<MimeDb>>,
    options: ViewOptions,
    zone: TimeZone,
}

impl Default for FolderModelRust {
    fn default() -> Self {
        Self {
            count: 0,
            loading: false,
            location: QString::default(),
            summary: QString::default(),
            problem: QString::default(),
            can_go_back: false,
            can_go_forward: false,
            can_go_up: false,
            model: None,
            snapshot: None,
            types: None,
            options: ViewOptions {
                details: true,
                ..ViewOptions::default()
            },
            zone: TimeZone::system(),
        }
    }
}

/// The status line under the list: "3 items", with "loading" while the
/// listing is not complete.
fn summary_text(len: usize, loading: bool) -> String {
    let items = format!(
        "{} {}",
        format::count(len),
        format::plural(len, "item", "items")
    );
    if loading {
        format!("{items}, loading…")
    } else {
        items
    }
}

/// The `image://theme/` id of an entry's icon: the theme's names for its
/// type, most specific first, then the generic folder or file icon.
fn icon_id(type_names: &[String], is_folder: bool) -> String {
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

/// The sort a column asks for.
fn sort_key(column: i32) -> SortKey {
    match column {
        1 => SortKey::Size,
        2 => SortKey::Modified,
        3 => SortKey::Type,
        _ => SortKey::Name,
    }
}

impl qobject::FolderModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let (Some(snapshot), Ok(row), Some(role)) = (
            self.rust().snapshot.as_ref(),
            usize::try_from(index.row()),
            Role::from_number(role),
        ) else {
            return QVariant::default();
        };
        if row >= snapshot.len() {
            return QVariant::default();
        }
        let text = |words: &str| QVariant::from(&QString::from(words));
        match role {
            Role::Name => text(&String::from_utf8_lossy(snapshot.name(row))),
            Role::Size => text(&format::size_column(
                snapshot.is_container(row),
                snapshot.metadata(row).map(|meta| meta.size),
            )),
            Role::Modified => text(
                &snapshot
                    .metadata(row)
                    .map(|meta| format::timestamp(meta.modified, &self.rust().zone))
                    .unwrap_or_default(),
            ),
            Role::Folder => QVariant::from(&snapshot.is_container(row)),
            Role::Hidden => QVariant::from(&snapshot.is_hidden(row)),
            Role::Icon => {
                let is_folder = snapshot.is_container(row);
                let names = self.rust().types.as_ref().map_or_else(Vec::new, |types| {
                    types.icon_names(types.mime_type(snapshot.name(row), is_folder))
                });
                text(&icon_id(&names, is_folder))
            }
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Starts the core's model and the file types the first time; false
    /// when the model could not start, which [`Self::fail`] has said.
    fn start(mut self: Pin<&mut Self>) -> bool {
        if self.rust().model.is_some() {
            return true;
        }
        let (mut model, events) = match DirectoryModel::new(Arc::new(RealFs)) {
            Ok(started) => started,
            Err(error) => {
                self.fail(None, &error.to_string());
                return false;
            }
        };
        // Sizes and dates are read for the details view.
        model.set_options(self.rust().options);
        self.as_mut().rust_mut().model = Some(model);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            // Ends when the model, and with it the sender, is gone.
            while let Ok(event) = events.recv() {
                let sent = match event {
                    ModelEvent::Snapshot(snapshot) => {
                        thread.queue(move |model| model.show(snapshot))
                    }
                    ModelEvent::Failed { path, error, .. } => {
                        let words = error.to_string();
                        thread.queue(move |model| model.fail(Some(&path), &words))
                    }
                    ModelEvent::Moved { .. } => continue,
                };
                if sent.is_err() {
                    break;
                }
            }
        });
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let types = Arc::new(MimeDb::load(&xdg::data_dirs()));
            let _ = thread.queue(move |model| model.set_types(types));
        });
        true
    }

    fn open(self: Pin<&mut Self>, path: &QString) {
        self.go_to(PathBuf::from(path.to_string()));
    }

    /// Opens `path` as a new step in the history.
    fn go_to(mut self: Pin<&mut Self>, path: PathBuf) {
        if !self.as_mut().start() {
            return;
        }
        self.as_mut().set_location_text(&path);
        if let Some(model) = self.as_mut().rust_mut().model.as_mut() {
            model.open(&path);
        }
        self.update_navigation();
    }

    /// Runs one of the model's navigation steps.
    fn navigate(mut self: Pin<&mut Self>, step: fn(&mut DirectoryModel) -> Option<u64>) {
        if let Some(model) = self.as_mut().rust_mut().model.as_mut() {
            step(model);
        }
        self.update_navigation();
    }

    fn back(self: Pin<&mut Self>) {
        self.navigate(DirectoryModel::back);
    }

    fn forward(self: Pin<&mut Self>) {
        self.navigate(DirectoryModel::forward);
    }

    fn up(self: Pin<&mut Self>) {
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

    fn activate(self: Pin<&mut Self>, row: i32) {
        let target = self.rust().snapshot.as_ref().and_then(|snapshot| {
            let row = usize::try_from(row)
                .ok()
                .filter(|row| *row < snapshot.len())?;
            snapshot.is_container(row).then(|| snapshot.entry_path(row))
        });
        if let Some(path) = target {
            self.go_to(path);
        }
    }

    /// Shows `path` as the location.
    fn set_location_text(mut self: Pin<&mut Self>, path: &Path) {
        let text = QString::from(path.to_string_lossy().as_ref());
        if *self.location() != text {
            self.as_mut().rust_mut().location = text;
            self.as_mut().location_changed();
        }
    }

    /// Updates which navigation steps are possible.
    fn update_navigation(mut self: Pin<&mut Self>) {
        let (back, forward) = self.rust().model.as_ref().map_or((false, false), |model| {
            (model.can_go_back(), model.can_go_forward())
        });
        let up = Path::new(&self.location().to_string()).parent().is_some();
        if *self.can_go_back() != back {
            self.as_mut().rust_mut().can_go_back = back;
            self.as_mut().can_go_back_changed();
        }
        if *self.can_go_forward() != forward {
            self.as_mut().rust_mut().can_go_forward = forward;
            self.as_mut().can_go_forward_changed();
        }
        if *self.can_go_up() != up {
            self.as_mut().rust_mut().can_go_up = up;
            self.as_mut().can_go_up_changed();
        }
    }

    /// Takes the loaded file types and redraws the icons.
    fn set_types(mut self: Pin<&mut Self>, types: Arc<MimeDb>) {
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().types = Some(types);
        self.as_mut().end_reset_model();
    }

    fn sort_by(mut self: Pin<&mut Self>, column: i32, descending: bool) {
        let options = ViewOptions {
            sort: sort_key(column),
            descending,
            ..self.rust().options
        };
        let mut rust = self.as_mut().rust_mut();
        rust.options = options;
        if let Some(model) = rust.model.as_mut() {
            model.set_options(options);
        }
    }

    /// Swaps in the newest snapshot.
    fn show(mut self: Pin<&mut Self>, snapshot: Arc<Snapshot>) {
        let rows = i32::try_from(snapshot.len()).unwrap_or(i32::MAX);
        let loading = snapshot.is_loading();
        let summary = summary_text(snapshot.len(), loading);
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.snapshot = Some(snapshot);
            rust.count = rows;
            rust.loading = loading;
        }
        self.as_mut().end_reset_model();
        if let Some(path) = self
            .rust()
            .snapshot
            .as_ref()
            .map(|s| s.path().to_path_buf())
        {
            self.as_mut().set_location_text(&path);
        }
        self.as_mut().update_navigation();
        self.as_mut().count_changed();
        self.as_mut().loading_changed();
        self.as_mut().rust_mut().summary = QString::from(summary.as_str());
        self.as_mut().summary_changed();
        if !self.problem().is_empty() {
            self.as_mut().rust_mut().problem = QString::default();
            self.as_mut().problem_changed();
        }
    }

    /// Says why the folder (at `path`, when known) could not be shown.
    fn fail(mut self: Pin<&mut Self>, path: Option<&Path>, words: &str) {
        if let Some(path) = path {
            self.as_mut().set_location_text(path);
            self.as_mut().update_navigation();
        }
        self.as_mut().rust_mut().problem =
            QString::from(format!("This folder could not be opened: {words}.").as_str());
        self.as_mut().problem_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_found_by_number_and_named_once() {
        for role in Role::ALL {
            assert_eq!(Role::from_number(role as i32), Some(role));
        }
        assert_eq!(Role::from_number(0), None);
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.dedup();
        assert_eq!(names.len(), Role::ALL.len());
    }

    #[test]
    fn the_summary_counts_items_and_says_when_loading() {
        assert_eq!(summary_text(1, false), "1 item");
        assert_eq!(summary_text(3, true), "3 items, loading…");
    }

    #[test]
    fn an_icon_falls_back_from_the_types_names_to_the_generic_one() {
        let names = vec!["text-plain".to_owned(), "text-x-generic".to_owned()];
        assert_eq!(
            icon_id(&names, false),
            "text-plain|text-x-generic|text-x-generic"
        );
        assert_eq!(icon_id(&[], true), "folder");
        assert_eq!(icon_id(&[], false), "text-x-generic");
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
    fn columns_sort_by_name_size_date_and_type() {
        assert_eq!(sort_key(0), SortKey::Name);
        assert_eq!(sort_key(1), SortKey::Size);
        assert_eq!(sort_key(2), SortKey::Modified);
        assert_eq!(sort_key(3), SortKey::Type);
        assert_eq!(sort_key(9), SortKey::Name);
    }
}
