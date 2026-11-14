//! The Properties dialog's object (W-114f): the entry's facts, its size
//! (a folder measured in the background), its permissions, changed
//! through the operation engine (D-011).

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
use rustphin_platform::properties::{self, Facts, Kind};
use rustphin_view::properties::{
    SizeLine, bit, facts_rows, first_size, permissions, size_text, type_label,
};

/// How often a folder's measuring is shown.
const TICK: Duration = Duration::from_millis(250);

/// The Rust side of `PropertiesModel`.
pub struct PropertiesModelRust {
    title: QString,
    kind: QString,
    icon: QString,
    theme_icon: QString,
    thumbnail: QString,
    facts: QString,
    permissions: QString,
    error: QString,
    loading: bool,
    dirty: bool,
    path: PathBuf,
    read: Option<Facts>,
    size: SizeLine,
    mode: u32,
    original: u32,
    cancel: Arc<AtomicBool>,
    seen: Arc<AtomicU64>,
    zone: TimeZone,
}

impl Default for PropertiesModelRust {
    fn default() -> Self {
        Self {
            title: QString::default(),
            kind: QString::default(),
            icon: QString::from("file"),
            theme_icon: QString::default(),
            thumbnail: QString::default(),
            facts: QString::default(),
            permissions: QString::default(),
            error: QString::default(),
            loading: true,
            dirty: false,
            path: PathBuf::new(),
            read: None,
            size: SizeLine::Reading,
            mode: 0,
            original: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            seen: Arc::new(AtomicU64::new(0)),
            zone: TimeZone::system(),
        }
    }
}

impl qobject::PropertiesModel {
    fn open(mut self: Pin<&mut Self>, path: &QString) {
        let path = PathBuf::from(path.to_string());
        let title = path
            .file_name()
            .map(|name| display_name(name.as_encoded_bytes()).into_owned())
            .unwrap_or_else(|| "Properties".to_owned());
        self.as_mut().set_title(QString::from(&title));
        self.as_mut().rust_mut().path = path.clone();
        let thread = self.qt_thread();
        let helper = crate::admin::helper();
        std::thread::spawn(move || {
            let read = match helper {
                Some(helper) => helper
                    .stat(&path)
                    .map(|entry| properties::described_as_administrator(&entry))
                    .map_err(std::io::Error::from),
                None => properties::read(&path),
            };
            let _ = thread.queue(move |model| model.facts_read(read));
        });
    }

    fn facts_read(mut self: Pin<&mut Self>, read: std::io::Result<Facts>) {
        self.as_mut().set_loading(false);
        let facts = match read {
            Ok(facts) => facts,
            Err(error) => {
                let words = format!("Could not read this item: {error}. Close and try again.");
                return self.set_error(QString::from(&words));
            }
        };
        let path = self.rust().path.clone();
        let types = crate::types::when_ready_now();
        let directory = facts.kind == Kind::Directory;
        let (description, picture, theme_icon) =
            types.map_or((String::new(), false, String::new()), |types| {
                let name = path
                    .file_name()
                    .map(|name| name.as_encoded_bytes())
                    .unwrap_or_default();
                let mime = types.mimes.mime_type(name, directory);
                // The type's icon from the theme, as the pane draws it.
                let icon = crate::folder::icon_id(&types.mimes.icon_names(mime), directory);
                (
                    types.mimes.description(mime),
                    mime.starts_with("image/"),
                    icon,
                )
            });
        self.as_mut().set_theme_icon(QString::from(&theme_icon));
        let icon = match facts.kind {
            Kind::Directory => "folder",
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
                .set_thumbnail(QString::from(&rustphin_platform::uri::file_uri(&thumb)));
        }
        self.as_mut()
            .set_kind(QString::from(&type_label(&facts, &description)));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.mode = facts.mode;
            rust.original = facts.mode;
            rust.size = first_size(&facts);
            rust.read = Some(facts);
        }
        self.as_mut().show();
        if directory {
            self.measure();
        }
    }

    /// Measures a folder in the background, showing its progress.
    fn measure(self: Pin<&mut Self>) {
        let path = self.rust().path.clone();
        let cancel = Arc::clone(&self.rust().cancel);
        let seen = Arc::clone(&self.rust().seen);
        let ticks = self.qt_thread();
        let stop = Arc::clone(&cancel);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(TICK);
                if ticks.queue(|model| model.tick()).is_err() {
                    break;
                }
            }
        });
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

    fn tick(self: Pin<&mut Self>) {
        if self.rust().size == SizeLine::Calculating {
            self.show();
        }
    }

    /// Publishes the facts and permissions as they are.
    fn show(mut self: Pin<&mut Self>) {
        let Some(facts) = self.rust().read.clone() else {
            return;
        };
        let size = size_text(&self.rust().size, self.rust().seen.load(Ordering::Relaxed));
        let rows = facts_rows(&self.rust().path, &facts, &size, &self.rust().zone);
        self.as_mut().set_facts(QString::from(
            &serde_json::to_string(&rows).unwrap_or_default(),
        ));
        let shown = permissions(&facts, self.rust().mode);
        self.as_mut().set_permissions(QString::from(
            &serde_json::to_string(&shown).unwrap_or_default(),
        ));
        let dirty = shown.editable && self.rust().mode != self.rust().original;
        self.as_mut().set_dirty(dirty);
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

    fn apply(mut self: Pin<&mut Self>) {
        if !*self.dirty() {
            return;
        }
        let mode = combine_mode(self.rust().mode, self.rust().original);
        crate::hub::start(Request::SetMode {
            path: self.rust().path.clone(),
            mode,
        });
        self.as_mut().rust_mut().original = mode;
        self.show();
    }

    fn close(self: Pin<&mut Self>) {
        self.rust().cancel.store(true, Ordering::Relaxed);
    }
}
