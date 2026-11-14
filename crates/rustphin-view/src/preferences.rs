//! One store of settings for every window (D-029), free of any toolkit
//! (W-114d): read once at start, changed through [`Store::update`], and
//! written in the background, newest change last, so a slow disk never
//! holds the interface. Each folder's own view (W-106c2) lives beside it.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    folder_views::{FolderView, FolderViews},
    settings::{Click, Settings},
};

/// What [`Store::load`] found.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The settings, or the defaults.
    pub settings: Settings,
    /// Where they are saved; `None` when there is no config directory.
    pub path: Option<PathBuf>,
    /// The file existed.
    pub saved: bool,
    /// KDE's own choice of single-click opening.
    pub kde_single_click: bool,
    /// Each folder's own view.
    pub folder_views: FolderViews,
    /// Where they are saved.
    pub folder_path: Option<PathBuf>,
}

impl Loaded {
    /// Reads the settings, KDE's click setting and the folders' views.
    pub fn read() -> Self {
        let path = rustphin_platform::config::config_dir().map(|dir| dir.join("settings.toml"));
        let text = path
            .as_deref()
            .and_then(|path| match rustphin_platform::config::read(path) {
                Ok(text) => text,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "could not read the settings");
                    None
                }
            });
        let saved = text.is_some();
        let kde_single_click =
            rustphin_platform::kdeglobals::read(rustphin_platform::xdg::config_home().as_deref())
                .is_some_and(|text| rustphin_platform::kdeglobals::single_click(&text));
        let settings = text.map(|text| Settings::parse(&text)).unwrap_or_default();
        let folder_path = rustphin_platform::config::state_dir().map(|dir| dir.join("views.toml"));
        let folder_views = folder_path
            .as_deref()
            .and_then(|path| rustphin_platform::config::read(path).ok().flatten())
            .map(|text| FolderViews::parse(&text))
            .unwrap_or_default();
        Self {
            settings,
            path,
            saved,
            kde_single_click,
            folder_views,
            folder_path,
        }
    }
}

/// What a change to the settings asks of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changed {
    /// Something changed.
    pub any: bool,
    /// Text size, hidden-file fading or formats changed: redraw everything.
    pub redraw: bool,
}

/// The settings every window shares.
#[derive(Debug)]
pub struct Store {
    settings: Settings,
    writer: Option<Writer>,
    saved: bool,
    kde_single_click: bool,
    folder_views: FolderViews,
    folder_writer: Option<Writer>,
    view_epoch: u64,
}

impl Store {
    /// The store over what was loaded. Settings written by a newer Rustphin
    /// are used but never overwritten.
    pub fn new(loaded: Loaded) -> Self {
        let newer = loaded.settings.is_from_newer_version();
        let store = Self {
            writer: loaded.path.filter(|_| !newer).map(Writer::new),
            folder_writer: loaded.folder_path.filter(|_| !newer).map(Writer::new),
            settings: loaded.settings,
            saved: loaded.saved,
            kde_single_click: loaded.kde_single_click,
            folder_views: loaded.folder_views,
            view_epoch: 0,
        };
        store.apply_globals();
        store
    }

    fn apply_globals(&self) {
        let (dates, units) = self.settings.formats();
        crate::format::set_style(units, dates);
    }

    /// The settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Whether one click opens entries: the choice in Settings, or KDE's.
    pub fn opens_with_one_click(&self) -> bool {
        self.settings
            .click()
            .map_or(self.kde_single_click, |click| click == Click::Single)
    }

    /// KDE's own single-click choice.
    pub fn kde_single_click(&self) -> bool {
        self.kde_single_click
    }

    /// Where the settings are saved.
    pub fn path(&self) -> Option<&Path> {
        self.writer.as_ref().map(|writer| writer.path.as_path())
    }

    /// Whether a settings file exists.
    pub fn saved(&self) -> bool {
        self.saved
    }

    /// Changes the settings and saves them in the background.
    pub fn update(&mut self, change: impl FnOnce(&mut Settings)) -> Changed {
        let mut next = self.settings.clone();
        change(&mut next);
        if next == self.settings {
            return Changed::default();
        }
        let redraw = (next.text_size(), next.hidden_opacity(), next.formats())
            != (
                self.settings.text_size(),
                self.settings.hidden_opacity(),
                self.settings.formats(),
            );
        self.settings = next;
        self.apply_globals();
        if let Some(writer) = &self.writer {
            writer.job(self.settings.to_toml()).spawn();
            self.saved = true;
        }
        Changed { any: true, redraw }
    }

    /// [`Self::update`] of the default view: open panes take it up.
    pub fn update_default_view(&mut self, change: impl FnOnce(&mut Settings)) -> Changed {
        self.view_epoch += 1;
        let mut changed = self.update(change);
        changed.any = true;
        changed
    }

    /// Counts changes of the default view.
    pub fn view_epoch(&self) -> u64 {
        self.view_epoch
    }

    /// The view remembered for the folder at `location`.
    pub fn folder_view(&self, location: &str) -> Option<FolderView> {
        self.folder_views.get(location)
    }

    /// Remembers a folder's view and saves the list in the background.
    pub fn set_folder_view(&mut self, location: String, view: FolderView) {
        if !self.folder_views.set(location, view) {
            return;
        }
        if let Some(writer) = &self.folder_writer {
            writer.job(self.folder_views.to_toml()).spawn();
        }
    }
}

/// Writes one file, the newest text winning.
#[derive(Debug, Clone)]
struct Writer {
    path: PathBuf,
    latest: Arc<AtomicU64>,
    lock: Arc<Mutex<()>>,
}

struct Job {
    writer: Writer,
    number: u64,
    text: String,
}

impl Writer {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            latest: Arc::new(AtomicU64::new(0)),
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn job(&self, text: String) -> Job {
        Job {
            writer: self.clone(),
            number: self.latest.fetch_add(1, Ordering::SeqCst) + 1,
            text,
        }
    }
}

impl Job {
    fn spawn(self) {
        let spawned = std::thread::Builder::new()
            .name("save-settings".into())
            .spawn(move || self.run());
        if let Err(error) = spawned {
            tracing::warn!(%error, "could not start saving the settings");
        }
    }

    /// Writes the text unless a newer job has been made since.
    fn run(self) {
        let _one_at_a_time = self
            .writer
            .lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if self.writer.latest.load(Ordering::SeqCst) != self.number {
            return;
        }
        if let Err(error) =
            rustphin_platform::config::write_atomic(&self.writer.path, self.text.as_bytes())
        {
            tracing::warn!(%error, path = %self.writer.path.display(), "could not save the settings");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn only_the_newest_text_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let writer = Writer::new(dir.path().join("settings.toml"));
        let old = writer.job("old".into());
        let new = writer.job("new".into());
        old.run();
        assert!(!dir.path().join("settings.toml").exists());
        new.run();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("settings.toml")).unwrap(),
            "new"
        );
    }

    #[test]
    fn a_change_says_whether_everything_must_be_redrawn() {
        let mut store = Store::new(Loaded::default());
        assert_eq!(store.update(|_| {}), Changed::default());
        let changed = store.update(|settings| settings.set_keep_running(false));
        assert!(changed.any && !changed.redraw);
        let opacity = store.settings().hidden_opacity();
        let changed = store.update(|settings| {
            settings.set_appearance(
                settings.look(),
                settings.title_bar(),
                settings.text_size(),
                opacity + 10,
            );
        });
        assert!(changed.redraw);
    }

    #[test]
    fn one_click_follows_settings_then_kde() {
        let kde = Store::new(Loaded {
            kde_single_click: true,
            ..Loaded::default()
        });
        assert!(kde.opens_with_one_click());
        let mut chosen = kde;
        chosen.update(|settings| settings.set_click(Some(Click::Double)));
        assert!(!chosen.opens_with_one_click());
    }
}
