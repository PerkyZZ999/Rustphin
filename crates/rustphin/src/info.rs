//! The Properties window's object (W-114f, W-170): the entry's facts in
//! four tabs (`rustphin_view::properties_window`), its size (a folder
//! measured in the background), its permissions and group, changed through
//! the operation engine (D-011), its checksums, calculated on demand, and
//! what Rustphin reads from the file itself. QML receives one JSON view.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "info_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use jiff::tz::TimeZone;
use rustphin_core::{combine_mode, display_name, ops::Request, with_bit};
use rustphin_platform::{
    checksum,
    properties::{self, Facts, Group, Kind, Picture},
    uri::{file_uri, percent_encode},
};
use rustphin_view::{
    properties::{SizeLine, bit, first_size, permissions, size_text, time_text, type_label},
    properties_window::{
        self as window, ACCESS_CHOICES, ALGORITHMS, Pasted, access, algorithm_name, pending_note,
        progress_text, runs, size_rows, verdict, with_access, with_runs,
    },
};
use serde_json::{Value, json};

/// How often a measurement or a calculation shows its progress.
const TICK: Duration = Duration::from_millis(250);

/// Where one checksum stands.
#[derive(Default)]
enum Sum {
    /// Not asked for.
    #[default]
    Idle,
    /// Being calculated: bytes read so far, and its stop.
    Running(Arc<AtomicU64>, Arc<AtomicBool>),
    /// The result, in lowercase hexadecimal.
    Done(String),
    /// Why it failed.
    Failed(String),
}

/// The Rust side of `PropertiesModel`.
pub struct PropertiesModelRust {
    /// The number of the window that opened Properties: the entry is read
    /// and changed with that window's rights.
    window: i32,
    title: QString,
    kind: QString,
    icon: QString,
    theme_icon: QString,
    thumbnail: QString,
    view: QString,
    error: QString,
    loading: bool,
    dirty: bool,
    path: PathBuf,
    read: Option<Facts>,
    folder: bool,
    mime: String,
    picture: Option<Picture>,
    groups: Vec<Group>,
    size: SizeLine,
    mode: u32,
    original: u32,
    group: u32,
    original_group: u32,
    sums: [Sum; 4],
    pasted: String,
    /// The calculation the pasted text started, stopped if the text turns
    /// into another kind (typing passes through 32 and 40 digits).
    started_by_paste: Option<usize>,
    cancel: Arc<AtomicBool>,
    seen: Arc<AtomicU64>,
    zone: TimeZone,
}

impl Default for PropertiesModelRust {
    fn default() -> Self {
        Self {
            window: 0,
            title: QString::default(),
            kind: QString::default(),
            icon: QString::from("file"),
            theme_icon: QString::default(),
            thumbnail: QString::default(),
            view: QString::default(),
            error: QString::default(),
            loading: true,
            dirty: false,
            path: PathBuf::new(),
            read: None,
            folder: false,
            mime: String::new(),
            picture: None,
            groups: Vec::new(),
            size: SizeLine::Reading,
            mode: 0,
            original: 0,
            group: 0,
            original_group: 0,
            sums: Default::default(),
            pasted: String::new(),
            started_by_paste: None,
            cancel: Arc::new(AtomicBool::new(false)),
            seen: Arc::new(AtomicU64::new(0)),
            zone: TimeZone::system(),
        }
    }
}

/// What the background read finds.
struct Read {
    facts: std::io::Result<Facts>,
    target_is_folder: bool,
    picture: Option<Picture>,
    groups: Vec<Group>,
}

impl qobject::PropertiesModel {
    fn open(mut self: Pin<&mut Self>, uri: &QString) {
        let Some(path) = crate::qml_path::from_qml(uri) else {
            return;
        };
        let title = path
            .file_name()
            .map(|name| display_name(name.as_encoded_bytes()).into_owned())
            .unwrap_or_else(|| "Properties".to_owned());
        self.as_mut().set_title(QString::from(&title));
        self.as_mut().rust_mut().path = path.clone();
        let thread = self.qt_thread();
        let helper = crate::admin::helper(*self.window());
        std::thread::spawn(move || {
            let facts = match helper {
                Some(helper) => helper
                    .stat(&path)
                    .map(|entry| properties::described_as_administrator(&entry))
                    .map_err(std::io::Error::from),
                None => properties::read(&path),
            };
            // A link is drawn as what it points to; following it is a read.
            let target_is_folder = facts
                .as_ref()
                .is_ok_and(|facts| facts.kind == Kind::Symlink)
                && std::fs::metadata(&path).is_ok_and(|target| target.is_dir());
            let file = facts.as_ref().is_ok_and(|facts| facts.kind == Kind::File);
            let read = Read {
                picture: file.then(|| properties::picture(&path)).flatten(),
                groups: properties::user_groups(),
                facts,
                target_is_folder,
            };
            let _ = thread.queue(move |model| model.facts_read(read));
        });
    }

    fn facts_read(mut self: Pin<&mut Self>, read: Read) {
        self.as_mut().set_loading(false);
        let facts = match read.facts {
            Ok(facts) => facts,
            Err(error) => {
                let words = format!("Could not read this item: {error}. Close and try again.");
                return self.set_error(QString::from(&words));
            }
        };
        let path = self.rust().path.clone();
        let types = crate::types::when_ready_now();
        let (name, directory) =
            rustphin_view::properties::icon_subject(&path, &facts, read.target_is_folder);
        let (mime, description, picture, theme_icon) = types.as_ref().map_or(
            (String::new(), String::new(), false, String::new()),
            |types| {
                let mime = types.mimes.mime_type(name, directory);
                // The type's icon from the theme, as the pane draws it.
                let icon = crate::folder::icon_id(&types.mimes.icon_names(mime), directory);
                (
                    mime.to_owned(),
                    types.mimes.description(mime),
                    mime.starts_with("image/"),
                    icon,
                )
            },
        );
        self.as_mut().set_theme_icon(QString::from(&theme_icon));
        let icon = match facts.kind {
            _ if directory => "folder",
            _ if picture => "image",
            _ => "file",
        };
        self.as_mut().set_icon(QString::from(icon));
        if picture
            && let Some(thumb) =
                rustphin_platform::thumbnails::ThumbnailCache::for_user().and_then(|cache| {
                    cache.get(
                        &path,
                        rustphin_platform::thumbnails::ThumbSize::for_edge(96),
                    )
                })
        {
            self.as_mut()
                .set_thumbnail(QString::from(&file_uri(&thumb)));
        }
        self.as_mut()
            .set_kind(QString::from(&type_label(&facts, &description)));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.mode = facts.mode;
            rust.original = facts.mode;
            rust.group = facts.group_id;
            rust.original_group = facts.group_id;
            rust.size = first_size(&facts);
            rust.folder = directory;
            rust.mime = mime;
            rust.picture = read.picture;
            rust.groups = read.groups;
            rust.read = Some(facts);
        }
        self.as_mut().show();
        if directory
            && self
                .rust()
                .read
                .as_ref()
                .is_some_and(|f| f.kind == Kind::Directory)
        {
            self.measure();
        }
    }

    /// Measures a folder in the background, showing its progress.
    fn measure(self: Pin<&mut Self>) {
        let path = self.rust().path.clone();
        let cancel = Arc::clone(&self.rust().cancel);
        let seen = Arc::clone(&self.rust().seen);
        self.as_ref().get_ref().keep_ticking(Arc::clone(&cancel));
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let measured = properties::folder_size(&path, &cancel, &seen);
            cancel.store(true, Ordering::Relaxed);
            let _ = thread.queue(move |mut model| {
                model.as_mut().rust_mut().size = match measured {
                    Ok(size) if !size.cancelled => SizeLine::Folder {
                        bytes: size.bytes,
                        unreadable: size.unreadable,
                        items: size.items,
                    },
                    Ok(_) => return,
                    Err(error) => SizeLine::Failed(format!("Could not measure: {error}.")),
                };
                model.show();
            });
        });
    }

    /// Shows the progress every [`TICK`] until `stop` is set.
    fn keep_ticking(&self, stop: Arc<AtomicBool>) {
        let ticks = self.qt_thread();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(TICK);
                if ticks.queue(|model| model.tick()).is_err() {
                    break;
                }
            }
        });
    }

    fn tick(self: Pin<&mut Self>) {
        let busy = self.rust().size == SizeLine::Calculating
            || self
                .rust()
                .sums
                .iter()
                .any(|sum| matches!(sum, Sum::Running(..)));
        if busy {
            self.show();
        }
    }

    /// Publishes everything the window shows, as it is now.
    fn show(mut self: Pin<&mut Self>) {
        let Some(view) = self.rust().view() else {
            return;
        };
        let dirty = view["dirty"] == true;
        self.as_mut().set_view(QString::from(&view.to_string()));
        self.set_dirty(dirty);
    }

    fn choose_access(mut self: Pin<&mut Self>, who: i32, index: i32) {
        let (Ok(class), Some(&choice)) = (
            u8::try_from(who),
            usize::try_from(index)
                .ok()
                .and_then(|index| ACCESS_CHOICES.get(index)),
        ) else {
            return;
        };
        if class > 2 {
            return;
        }
        let rust = self.as_mut().rust_mut();
        let folder = rust.folder;
        let mode = with_access(rust.mode, class, choice, folder);
        self.as_mut().rust_mut().mode = mode;
        self.show();
    }

    fn set_runs(mut self: Pin<&mut Self>, on: bool) {
        let mode = with_runs(self.rust().mode, on);
        self.as_mut().rust_mut().mode = mode;
        self.show();
    }

    fn set_bit(mut self: Pin<&mut Self>, group: i32, which: i32, on: bool) {
        let (Ok(class), Ok(which)) = (u8::try_from(group), u8::try_from(which)) else {
            return;
        };
        if class > 2 || which > 2 {
            return;
        }
        let mode = with_bit(self.rust().mode, bit(class, which), on);
        self.as_mut().rust_mut().mode = mode;
        self.show();
    }

    fn choose_group(mut self: Pin<&mut Self>, index: i32) {
        let Some(id) = usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().groups.get(index))
            .map(|group| group.id)
        else {
            return;
        };
        self.as_mut().rust_mut().group = id;
        self.show();
    }

    fn apply(mut self: Pin<&mut Self>) {
        if !*self.dirty() {
            return;
        }
        let as_admin = crate::admin::active(*self.window());
        let path = self.rust().path.clone();
        if self.rust().mode != self.rust().original {
            let mode = combine_mode(self.rust().mode, self.rust().original);
            crate::hub::start(
                Request::SetMode {
                    path: path.clone(),
                    mode,
                },
                as_admin,
            );
            self.as_mut().rust_mut().original = mode;
            self.as_mut().rust_mut().mode = mode;
        }
        if self.rust().group != self.rust().original_group {
            let group = self.rust().group;
            crate::hub::start(Request::SetGroup { path, group }, as_admin);
            self.as_mut().rust_mut().original_group = group;
        }
        self.show();
    }

    fn calculate(mut self: Pin<&mut Self>, index: i32) {
        let Some(at) = usize::try_from(index)
            .ok()
            .filter(|at| *at < ALGORITHMS.len())
        else {
            return;
        };
        if matches!(self.rust().sums[at], Sum::Running(..)) {
            return;
        }
        let done = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().sums[at] = Sum::Running(Arc::clone(&done), Arc::clone(&stop));
        let path = self.rust().path.clone();
        let finished = Arc::new(AtomicBool::new(false));
        self.as_ref().get_ref().keep_ticking(Arc::clone(&finished));
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = checksum::calculate(&path, ALGORITHMS[at], &stop, &done);
            finished.store(true, Ordering::Relaxed);
            let _ = thread.queue(move |mut model| {
                // A stop already put the slot back.
                if !matches!(model.rust().sums[at], Sum::Running(..)) {
                    return;
                }
                model.as_mut().rust_mut().sums[at] = match result {
                    Ok(Some(sum)) => Sum::Done(sum),
                    Ok(None) => Sum::Idle,
                    Err(error) => Sum::Failed(format!("Could not read the file: {error}.")),
                };
                model.show();
            });
        });
        self.show();
    }

    fn stop_sum(mut self: Pin<&mut Self>, index: i32) {
        let Some(at) = usize::try_from(index)
            .ok()
            .filter(|at| *at < ALGORITHMS.len())
        else {
            return;
        };
        if let Sum::Running(_, stop) = &self.rust().sums[at] {
            stop.store(true, Ordering::Relaxed);
        }
        self.as_mut().rust_mut().sums[at] = Sum::Idle;
        self.show();
    }

    /// The text of "Check a download": a checksum Rustphin knows starts
    /// its calculation, unless it is known already.
    fn paste(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        let wanted = match window::pasted(&text) {
            Pasted::Sum(algorithm, _) => ALGORITHMS.iter().position(|a| *a == algorithm),
            _ => None,
        };
        self.as_mut().rust_mut().pasted = text;
        let earlier = self.rust().started_by_paste;
        if let Some(earlier) = earlier.filter(|earlier| Some(*earlier) != wanted) {
            self.as_mut().rust_mut().started_by_paste = None;
            if matches!(self.rust().sums[earlier], Sum::Running(..)) {
                self.as_mut().stop_sum(i32::try_from(earlier).unwrap_or(0));
            }
        }
        match wanted {
            Some(at) if matches!(self.rust().sums[at], Sum::Idle | Sum::Failed(_)) => {
                self.as_mut().rust_mut().started_by_paste = Some(at);
                self.calculate(i32::try_from(at).unwrap_or(0));
            }
            _ => self.show(),
        }
    }

    /// The default application may have changed (Change…).
    fn refresh_app(self: Pin<&mut Self>) {
        self.show();
    }

    fn close(mut self: Pin<&mut Self>) {
        self.rust().cancel.store(true, Ordering::Relaxed);
        for sum in &self.rust().sums {
            if let Sum::Running(_, stop) = sum {
                stop.store(true, Ordering::Relaxed);
            }
        }
        self.as_mut().rust_mut().sums = Default::default();
    }
}

impl PropertiesModelRust {
    /// Everything the window shows, as JSON; `None` before the entry is read.
    fn view(&self) -> Option<Value> {
        let facts = self.read.as_ref()?;
        let tabs: Vec<Value> = window::tabs(facts, self.folder)
            .into_iter()
            .map(
                |tab| json!({ "id": format!("{tab:?}"), "label": tab.label(), "icon": tab.icon() }),
            )
            .collect();
        let size = size_text(&self.size, self.seen.load(Ordering::Relaxed));
        let mut dates =
            vec![json!({ "label": "Created", "value": time_text(facts.created, &self.zone) })];
        dates.push(json!({ "label": "Modified", "value": time_text(facts.modified, &self.zone) }));
        dates.push(json!({ "label": "Accessed", "value": time_text(facts.accessed, &self.zone) }));
        let types = crate::types::when_ready_now();
        let opens_with = types.as_ref().filter(|_| !self.folder).and_then(|types| {
            let app = types.applications(&self.mime).into_iter().next()?;
            let icon = types.app_icon(&app.id).map(|icon| file_uri(&icon));
            Some(json!({ "name": app.name, "icon": icon.unwrap_or_default() }))
        });
        let parent = self.path.parent().map(PathBuf::from).unwrap_or_default();
        let shown = permissions(facts, self.mode);
        let classes: Vec<Value> = (0..3u8)
            .map(|class| {
                let chosen = access(self.mode, class, self.folder);
                let who = match class {
                    0 => facts.owner.clone(),
                    1 => self
                        .groups
                        .iter()
                        .find(|group| group.id == self.group)
                        .map_or_else(|| facts.group.clone(), |group| group.name.clone()),
                    _ => String::new(),
                };
                json!({
                    "name": rustphin_view::properties::PERMISSION_CLASSES[usize::from(class)],
                    "who": who,
                    "choice": ACCESS_CHOICES.iter().position(|c| *c == chosen).map_or(-1, |at| at as i64),
                    "custom": chosen.label(self.folder),
                    "changed": access(self.original, class, self.folder) != chosen,
                    "bits": shown.classes[usize::from(class)].bits,
                })
            })
            .collect();
        let choices: Vec<&str> = ACCESS_CHOICES
            .iter()
            .map(|choice| choice.label(self.folder))
            .collect();
        // The group list: the user's groups, with the file's own first when
        // the user is not in it.
        let mut group_names: Vec<String> = self.groups.iter().map(|g| g.name.clone()).collect();
        let group_index = self.groups.iter().position(|g| g.id == self.group);
        let group_editable = facts.owned_by_user && group_index.is_some() && shown.editable;
        if group_index.is_none() {
            group_names.insert(0, facts.group.clone());
        }
        let pasted = window::pasted(&self.pasted);
        let sums: Vec<Value> = ALGORITHMS
            .iter()
            .zip(&self.sums)
            .map(|(algorithm, sum)| {
                let (state, value, fraction) = match sum {
                    Sum::Idle => ("idle", String::new(), 0.),
                    Sum::Running(done, _) => {
                        let done = done.load(Ordering::Relaxed);
                        let total = facts.size.unwrap_or(0);
                        let fraction = if total == 0 {
                            0.
                        } else {
                            done as f64 / total as f64
                        };
                        ("running", progress_text(done, total), fraction.min(1.))
                    }
                    Sum::Done(sum) => ("done", sum.clone(), 1.),
                    Sum::Failed(why) => ("failed", why.clone(), 0.),
                };
                json!({ "name": algorithm_name(*algorithm), "state": state, "value": value, "fraction": fraction })
            })
            .collect();
        let calculated =
            match &pasted {
                Pasted::Sum(algorithm, _) => ALGORITHMS
                    .iter()
                    .position(|a| a == algorithm)
                    .and_then(|at| match &self.sums[at] {
                        Sum::Done(sum) => Some(sum.as_str()),
                        _ => None,
                    }),
                _ => None,
            };
        let target = facts
            .target
            .as_ref()
            .map(|target| display_name(target.as_os_str().as_encoded_bytes()).into_owned());
        let description = self.title_kind();
        let details = window::details(
            self.picture.as_ref(),
            &description,
            &self.mime,
            facts.size,
            &time_text(facts.modified, &self.zone),
            target.as_deref(),
        );
        let mode_changed = shown.editable && self.mode != self.original;
        let group_changed = self.group != self.original_group;
        Some(json!({
            "tabs": tabs,
            "folder": self.folder,
            "opensWith": opens_with,
            "location": display_name(parent.as_os_str().as_encoded_bytes()).into_owned(),
            "locationUri": file_uri(&parent),
            "nameKey": self.path.file_name().map(|name| percent_encode(name.as_encoded_bytes())).unwrap_or_default(),
            "size": size_rows(&size, facts),
            "dates": dates,
            "pointsTo": target.unwrap_or_default(),
            "access": {
                "classes": classes,
                "choices": choices,
                "runs": runs(self.mode),
                "runsShown": facts.kind == Kind::File,
                "runsChanged": runs(self.mode) != runs(self.original),
                "words": shown.words,
                "editable": shown.editable,
                "note": shown.note,
            },
            "owner": facts.owner,
            "ownerNote": if facts.owned_by_user {
                "Only an administrator can give it to another user."
            } else {
                "Only its owner or an administrator can change its group."
            },
            "groups": group_names,
            "groupIndex": group_index.map_or(0, |at| at as i64),
            "groupEditable": group_editable,
            "groupChanged": group_changed,
            "pending": pending_note(mode_changed, group_changed),
            "dirty": mode_changed || group_changed,
            "sums": sums,
            "verdict": verdict(&pasted, calculated),
            "details": details,
        }))
    }

    /// The type's description, as the General tab says it.
    fn title_kind(&self) -> String {
        self.kind.to_string()
    }
}

#[cfg(test)]
mod tests {
    /// W-170: a Properties window made from JavaScript with no parent can
    /// be taken by the collector while open (seen: the window vanished when
    /// Change… opened the chooser). It is parented to the main window.
    #[test]
    fn a_properties_window_is_owned_by_the_window_that_opened_it() {
        let main = include_str!("../qml/Main.qml");
        assert!(
            main.contains("propertiesWindow.createObject(root,"),
            "Main.qml must parent each Properties window to the main window"
        );
        assert!(!main.contains("propertiesWindow.createObject(null"));
    }

    /// W-170: a drop-down whose model is assigned again resets its index
    /// and drops a binding (seen: every class read "Can view & modify").
    /// The window sets the index whenever the model or the choice changes.
    #[test]
    fn the_access_and_group_drop_downs_set_their_index_after_their_model() {
        let window = include_str!("../qml/PropertiesWindow.qml");
        assert!(
            !window.contains("currentIndex: accessLine")
                && !window.contains("currentIndex: props.groupIndex"),
            "a bound currentIndex is lost when the model is assigned"
        );
        assert_eq!(window.matches("onModelChanged:").count(), 2);
    }
}
