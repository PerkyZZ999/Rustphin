//! The open tabs and panes, remembered between sessions as TOML.
//!
//! Reading never fails: a missing or damaged file yields an empty session,
//! which the workspace replaces with a single tab.

use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

/// The format version this build writes.
const VERSION: u32 = 2;

/// The tabs that were open, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    version: u32,
    /// Index of the tab that was showing.
    pub active: usize,
    /// The tabs, left to right.
    pub tabs: Vec<SessionTab>,
}

/// One tab: the folder of each of its panes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTab {
    /// One folder, or two when the tab was split.
    #[serde(with = "pane_paths")]
    pub panes: Vec<PathBuf>,
    /// Index of the pane that had the keyboard focus.
    pub active_pane: usize,
}

impl Session {
    /// A session of the given tabs.
    pub fn new(tabs: Vec<SessionTab>, active: usize) -> Self {
        Self {
            version: VERSION,
            active,
            tabs,
        }
    }

    /// A session with one tab showing `folder`.
    pub fn single(folder: PathBuf) -> Self {
        Self::folders(vec![folder])
    }

    /// One tab per folder, in order, with the first one showing.
    pub fn folders(paths: Vec<PathBuf>) -> Self {
        Self::new(
            paths
                .into_iter()
                .map(|folder| SessionTab {
                    panes: vec![folder],
                    active_pane: 0,
                })
                .collect(),
            0,
        )
    }

    /// Reads a session from TOML text. Unreadable text gives an empty session.
    /// Tabs without a folder are dropped, panes beyond the second ignored and
    /// relative paths rejected, so what remains can be opened as it is.
    pub fn parse(text: &str) -> Self {
        let mut session: Self = toml::from_str(text).unwrap_or_else(|error| {
            tracing::warn!(%error, "ignoring unreadable session");
            Self::default()
        });
        for tab in &mut session.tabs {
            tab.panes.retain(|path| {
                path.is_absolute() && !path.as_os_str().as_encoded_bytes().contains(&0)
            });
            tab.panes.truncate(2);
            tab.active_pane = tab.active_pane.min(tab.panes.len().saturating_sub(1));
        }
        session.tabs.retain(|tab| !tab.panes.is_empty());
        session.active = session.active.min(session.tabs.len().saturating_sub(1));
        if session.version <= VERSION {
            session.version = VERSION;
        }
        session
    }

    /// The session as TOML text, with byte-preserving local file URIs.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Whether the file was written by a later version of Rustphin and must
    /// not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }
}

// Version 1 stored UTF-8 paths directly. New files use URIs so every valid
// Linux filename round-trips; a literal percent in an old path stays literal.
mod pane_paths {
    use std::path::PathBuf;

    use rustphin_platform::uri::{file_uri, path_from_uri};
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(paths.iter().map(|path| file_uri(path)))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<PathBuf>, D::Error> {
        let paths = Vec::<String>::deserialize(deserializer)?;
        Ok(paths
            .into_iter()
            .filter_map(|value| {
                if value.starts_with('/') {
                    Some(PathBuf::from(value))
                } else {
                    path_from_uri(value.as_bytes())
                }
            })
            .collect())
    }
}

/// Prepares session snapshots on the UI thread for background publication.
#[derive(Default)]
pub(crate) struct SessionSaves {
    issued: u64,
    latest_attempt: Arc<Mutex<u64>>,
}

impl SessionSaves {
    pub(crate) fn prepare(&mut self, session: Session) -> io::Result<SessionSave> {
        self.issued = self
            .issued
            .checked_add(1)
            .ok_or_else(|| io::Error::other("session-save sequence exhausted"))?;
        Ok(SessionSave {
            session,
            revision: self.issued,
            latest_attempt: Arc::clone(&self.latest_attempt),
        })
    }
}

/// One pending background save.
pub(crate) struct SessionSave {
    session: Session,
    revision: u64,
    latest_attempt: Arc<Mutex<u64>>,
}

impl SessionSave {
    pub(crate) fn write(self, path: &Path) -> io::Result<()> {
        let mut latest = self
            .latest_attempt
            .lock()
            .map_err(|_| io::Error::other("session-save ordering lock was poisoned"))?;
        if self.revision <= *latest {
            return Ok(());
        }
        // Keep this guard through publication. A failed newer attempt also
        // retires older requests; it must not silently restore their tabs.
        *latest = self.revision;
        let text = self
            .session
            .to_toml()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        rustphin_platform::config::write_atomic(path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_late_older_save_cannot_restore_a_closed_tab() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.toml");
        let mut saves = SessionSaves::default();
        let older = saves
            .prepare(Session::folders(vec!["/open".into(), "/closed".into()]))
            .unwrap();
        let latest = Session::single("/open".into());
        let newer = saves.prepare(latest.clone()).unwrap();

        newer.write(&path).unwrap();
        older.write(&path).unwrap();

        let saved = rustphin_platform::config::read(&path).unwrap().unwrap();
        assert_eq!(Session::parse(&saved), latest);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_failed_newer_save_does_not_publish_an_older_pending_session() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("blocked");
        std::fs::write(&parent, b"not a directory").unwrap();
        let path = parent.join("session.toml");
        let mut saves = SessionSaves::default();
        let older = saves.prepare(Session::single("/closed".into())).unwrap();
        let latest = Session::single("/open".into());
        let newer = saves.prepare(latest.clone()).unwrap();

        let error = newer.write(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
        std::fs::remove_file(&parent).unwrap();
        std::fs::create_dir(&parent).unwrap();
        older.write(&path).unwrap();
        assert_eq!(rustphin_platform::config::read(&path).unwrap(), None);

        saves.prepare(latest.clone()).unwrap().write(&path).unwrap();
        let saved = rustphin_platform::config::read(&path).unwrap().unwrap();
        assert_eq!(Session::parse(&saved), latest);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn saved_split_tabs_preserve_non_utf8_folder_names() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt as _};

        let raw = PathBuf::from(OsString::from_vec(b"/data/\xff%20#?\n".to_vec()));
        let session = Session::new(
            vec![
                tab(&["/normal"], 0),
                SessionTab {
                    panes: vec!["/Été".into(), raw],
                    active_pane: 1,
                },
            ],
            1,
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.toml");
        SessionSaves::default()
            .prepare(session.clone())
            .unwrap()
            .write(&path)
            .unwrap();

        let saved = rustphin_platform::config::read(&path).unwrap().unwrap();
        assert_eq!(Session::parse(&saved), session);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_legacy_session_keeps_literal_percent_paths_and_upgrades_when_saved() {
        let restored = Session::parse(
            "version = 1\nactive = 0\n[[tabs]]\npanes = ['/data/literal%20', '/data/Été']\nactive_pane = 1\n",
        );
        assert_eq!(restored.tabs, [tab(&["/data/literal%20", "/data/Été"], 1)]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.toml");
        SessionSaves::default()
            .prepare(restored.clone())
            .unwrap()
            .write(&path)
            .unwrap();

        let saved = rustphin_platform::config::read(&path).unwrap().unwrap();
        let document: toml::Value = toml::from_str(&saved).unwrap();
        assert_eq!(document["version"].as_integer(), Some(2));
        assert_eq!(Session::parse(&saved), restored);
    }

    #[test]
    fn uri_panes_ignore_remote_relative_and_nul_paths_before_clamping() {
        let restored = Session::parse(
            "version = 2\n[[tabs]]\npanes = ['file://remote/data', 'file:///bad%00name', 'relative', 'file:///saved%20folder']\n",
        );
        assert_eq!(restored, Session::single("/saved folder".into()));
    }

    fn tab(panes: &[&str], active_pane: usize) -> SessionTab {
        SessionTab {
            panes: panes.iter().map(PathBuf::from).collect(),
            active_pane,
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_session_survives_being_written_and_read_back() {
        let saved = Session::new(
            vec![tab(&["/home/me"], 0), tab(&["/srv/a", "/srv/b"], 1)],
            1,
        );
        assert_eq!(Session::parse(&saved.to_toml().unwrap()), saved);
    }

    #[test]
    fn a_single_folder_makes_one_unsplit_tab() {
        let session = Session::single(PathBuf::from("/tmp"));
        assert_eq!(session.tabs, [tab(&["/tmp"], 0)]);
        assert_eq!(session.active, 0);
    }

    #[test]
    fn several_folders_make_one_tab_each_with_the_first_showing() {
        let session = Session::folders(vec![PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(session.tabs, [tab(&["/a"], 0), tab(&["/b"], 0)]);
        assert_eq!(session.active, 0);
    }

    #[test]
    fn missing_and_damaged_files_give_an_empty_session() {
        assert!(Session::parse("").tabs.is_empty());
        assert!(Session::parse("not [ toml").tabs.is_empty());
        assert!(Session::parse("tabs = \"oops\"").tabs.is_empty());
    }

    #[test]
    fn unusable_entries_are_dropped_and_indices_clamped() {
        let text = "active = 7\n\
            [[tabs]]\npanes = []\n\
            [[tabs]]\npanes = [\"relative/path\"]\n\
            [[tabs]]\npanes = [\"/a\", \"/b\", \"/c\"]\nactive_pane = 9\n";
        let session = Session::parse(text);
        assert_eq!(session.tabs, [tab(&["/a", "/b"], 1)]);
        assert_eq!(session.active, 0);
    }

    #[test]
    fn a_file_from_a_newer_version_is_recognised() {
        assert!(Session::parse(&format!("version = {}", VERSION + 1)).is_from_newer_version());
        assert!(!Session::single(PathBuf::from("/")).is_from_newer_version());
    }
}
