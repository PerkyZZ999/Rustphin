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

use crate::settings::Settings;

/// The shared settings.
pub struct Preferences {
    settings: Settings,
    writer: Option<Writer>,
    /// The file exists: it was read at start-up or has been written since.
    saved: bool,
}

struct Shared(Entity<Preferences>);

impl Global for Shared {}

/// Makes the settings read at start-up the shared ones. `path` is where
/// they are saved; nothing is saved when it is `None` or the file came
/// from a newer version. `saved` says whether the file was there.
pub fn init(settings: Settings, path: Option<PathBuf>, saved: bool, cx: &mut App) {
    let writer = path
        .filter(|_| !settings.is_from_newer_version())
        .map(Writer::new);
    let entity = cx.new(|_| Preferences {
        settings,
        writer,
        saved,
    });
    cx.set_global(Shared(entity));
}

/// The shared settings.
pub fn preferences(cx: &App) -> Entity<Preferences> {
    cx.global::<Shared>().0.clone()
}

impl Preferences {
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
        if let Some(writer) = &self.writer {
            let job = writer.job(self.settings.to_toml());
            cx.background_executor()
                .spawn(async move { job.run() })
                .detach();
            self.saved = true;
        }
        cx.notify();
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
