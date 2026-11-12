//! The settings every window shares, kept in one file (D-029).
//!
//! Panes report their view to the store and the Settings dialog changes
//! the rest; the store writes the whole file, atomically and off the UI
//! thread, the latest version always last. A file from a newer Rustphin is
//! read as far as it is understood and never written.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use gpui_kit::{App, AppContext as _, Context, Entity, Global};

use crate::{
    folder_views::{FolderView, FolderViews},
    settings::{Click, Settings},
};

/// The shared settings.
pub struct Preferences {
    settings: Settings,
    writer: Option<Writer>,
    /// The file exists: it was read at start-up or has been written since.
    saved: bool,
    /// KDE is set to open files with one click (`kdeglobals`, read at start).
    kde_single_click: bool,
    /// Each folder's own view, when the settings say to remember them.
    folder_views: FolderViews,
    folder_writer: Option<Writer>,
    /// Counts the default view's changes made in Settings; open panes take
    /// on the default when it moves (W-106c2).
    view_epoch: u64,
}

/// What the store starts from, read before the first window.
pub struct Loaded {
    /// The settings file's contents, or the defaults.
    pub settings: Settings,
    /// Where the settings are saved; `None` saves nothing.
    pub path: Option<PathBuf>,
    /// The settings file was there.
    pub saved: bool,
    /// KDE's choice of how files open.
    pub kde_single_click: bool,
    /// The folders' own views.
    pub folder_views: FolderViews,
    /// Where those are saved.
    pub folder_path: Option<PathBuf>,
}

struct Shared(Entity<Preferences>);

impl Global for Shared {}

/// Makes the settings read at start-up the shared ones. Nothing is saved
/// where a path is `None`, nor when the settings came from a newer version.
pub fn init(loaded: Loaded, cx: &mut App) {
    let newer = loaded.settings.is_from_newer_version();
    let writer = loaded.path.filter(|_| !newer).map(Writer::new);
    let folder_writer = loaded.folder_path.filter(|_| !newer).map(Writer::new);
    let (dates, units) = loaded.settings.formats();
    crate::format::set_style(units, dates);
    let entity = cx.new(|_| Preferences {
        settings: loaded.settings,
        writer,
        saved: loaded.saved,
        kde_single_click: loaded.kde_single_click,
        folder_views: loaded.folder_views,
        folder_writer,
        view_epoch: 0,
    });
    cx.set_global(Shared(entity));
}

/// The shared settings.
pub fn preferences(cx: &App) -> Entity<Preferences> {
    cx.global::<Shared>().0.clone()
}

impl Preferences {
    /// The settings as they are now.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Whether one click opens a file or folder: as chosen in Settings, or
    /// as KDE is set.
    pub fn opens_with_one_click(&self) -> bool {
        self.settings
            .click()
            .map_or(self.kde_single_click, |click| click == Click::Single)
    }

    /// KDE's own choice of how files open.
    pub fn kde_single_click(&self) -> bool {
        self.kde_single_click
    }

    /// Where they are saved, if anywhere.
    pub fn path(&self) -> Option<&Path> {
        self.writer.as_ref().map(|writer| writer.path.as_path())
    }

    /// Whether the settings file exists yet: a fresh home has none until
    /// the first change.
    pub fn saved(&self) -> bool {
        self.saved
    }

    /// Changes the settings, saves them if they changed and tells every
    /// window.
    pub fn update(&mut self, change: impl FnOnce(&mut Settings), cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        change(&mut next);
        if next == self.settings {
            return;
        }
        self.settings = next;
        let (dates, units) = self.settings.formats();
        crate::format::set_style(units, dates);
        if let Some(writer) = &self.writer {
            let job = writer.job(self.settings.to_toml());
            cx.background_executor()
                .spawn(async move { job.run() })
                .detach();
            self.saved = true;
        }
        cx.notify();
    }

    /// Changes the default view from Settings: every open pane takes it on,
    /// except a folder with a view of its own while those are remembered.
    pub fn update_default_view(
        &mut self,
        change: impl FnOnce(&mut Settings),
        cx: &mut Context<Self>,
    ) {
        self.view_epoch += 1;
        self.update(change, cx);
        cx.notify();
    }

    /// How many times Settings changed the default view.
    pub fn view_epoch(&self) -> u64 {
        self.view_epoch
    }

    /// The view remembered for the folder at `location`, if any.
    pub fn folder_view(&self, location: &str) -> Option<FolderView> {
        self.folder_views.get(location)
    }

    /// Remembers the folder's view and saves the list off the UI thread.
    pub fn set_folder_view(&mut self, location: String, view: FolderView, cx: &mut Context<Self>) {
        if !self.folder_views.set(location, view) {
            return;
        }
        if let Some(writer) = &self.folder_writer {
            let job = writer.job(self.folder_views.to_toml());
            cx.background_executor()
                .spawn(async move { job.run() })
                .detach();
        }
    }
}

/// Writes the settings file so that the newest settings always win, even
/// when two writes overlap.
#[derive(Clone)]
struct Writer {
    path: PathBuf,
    latest: Arc<AtomicU64>,
    lock: Arc<Mutex<()>>,
}

/// One write, numbered.
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
    /// Writes the file unless a newer job exists: writes happen one at a
    /// time, and an older one that comes second is dropped.
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
    fn the_newest_settings_win_whatever_order_the_writes_run_in() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let writer = Writer::new(path.clone());
        let older = writer.job("older".into());
        let newer = writer.job("newer".into());
        // The newer write runs first; the older one, coming late, is dropped.
        newer.run();
        older.run();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "newer");
        // In order, the last one is what stays.
        writer.job("first".into()).run();
        writer.job("second".into()).run();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }
}
