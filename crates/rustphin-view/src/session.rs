//! The open tabs and panes, remembered between sessions as TOML.
//!
//! Missing files allow an initial save. Read, format and version failures keep
//! the saved file protected while the workspace uses fallback or explicit tabs.

use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rustphin_core::RecentKey as _;
use serde::{Deserialize, Serialize};

use crate::location::Location;

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
    pub panes: Vec<Location>,
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
                    panes: vec![Location::Folder(folder)],
                    active_pane: 0,
                })
                .collect(),
            0,
        )
    }

    /// Reads supported TOML formats, rejecting future schemas before decoding tabs.
    /// Invalid paths are dropped and pane/tab indices are clamped.
    pub fn parse(text: &str) -> Result<Self, SessionError> {
        #[derive(Deserialize)]
        struct Header {
            #[serde(default)]
            version: u32,
        }
        let header: Header = toml::from_str(text).map_err(SessionError::Damaged)?;
        if header.version > VERSION {
            return Err(SessionError::NewerVersion(header.version));
        }
        let mut session: Self = toml::from_str(text).map_err(SessionError::Damaged)?;
        for tab in &mut session.tabs {
            tab.panes.retain(Location::is_usable);
            tab.panes.truncate(2);
            tab.active_pane = tab.active_pane.min(tab.panes.len().saturating_sub(1));
        }
        session.tabs.retain(|tab| !tab.panes.is_empty());
        session.active = session.active.min(session.tabs.len().saturating_sub(1));
        session.version = VERSION;
        Ok(session)
    }

    /// The session as TOML text, with byte-preserving local file URIs.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

/// Startup tabs and permission to replace the saved session.
pub struct RestoredSession {
    /// The tabs to open.
    pub session: Session,
    /// Why the saved session could not be used as it was, if so.
    pub issue: Option<SessionError>,
}

impl RestoredSession {
    /// Whether the saved session may be replaced: not when it could not be
    /// read, so a damaged or newer file is kept for the user.
    pub fn can_save(&self) -> bool {
        self.issue.is_none()
    }
}

/// Why a saved session must not be replaced during this run.
#[derive(Debug)]
pub enum SessionError {
    /// The file could not be read.
    Read(io::Error),
    /// The TOML document or supported schema is invalid.
    Damaged(toml::de::Error),
    /// The file needs a later session-format reader.
    NewerVersion(u32),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "could not read saved tabs: {error}"),
            Self::Damaged(error) => write!(f, "saved tabs are damaged: {error}"),
            Self::NewerVersion(version) => write!(
                f,
                "session format {version} is newer than supported format {VERSION}"
            ),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Damaged(error) => Some(error),
            Self::NewerVersion(_) => None,
        }
    }
}

impl SessionError {
    /// The notice shown to the user.
    pub fn notice(&self) -> &'static str {
        match self {
            Self::Read(_) => {
                "Saved tabs could not be read. Tab saving is off; the file was kept. Check session.toml and its permissions, then restart."
            }
            Self::Damaged(_) => {
                "Saved tabs are damaged. Tab saving is off; the file was kept. Quit, repair or move session.toml aside, then restart."
            }
            Self::NewerVersion(_) => {
                "Saved tabs use a newer file format. Tab saving is off; the file was kept. Use a newer Rustphin or quit and move session.toml aside."
            }
        }
    }
}

/// Chooses startup tabs without accessing the filesystem. Explicit folders
/// change what is shown, not permission to replace unreadable/unsupported data.
pub fn restore(saved: io::Result<Option<String>>, starts: &[PathBuf]) -> RestoredSession {
    let decoded = match saved {
        Ok(Some(text)) => Session::parse(&text),
        Ok(None) => Ok(Session::default()),
        Err(error) => Err(SessionError::Read(error)),
    };
    let (session, issue) = match decoded {
        Ok(session) => (session, None),
        Err(issue) => (Session::default(), Some(issue)),
    };
    let session = if starts.is_empty() {
        session
    } else {
        Session::folders(starts.to_vec())
    };
    RestoredSession { session, issue }
}

// Version 1 stored UTF-8 paths directly. New files use URIs so every valid
// Linux filename round-trips; a literal percent in an old path stays literal.
// Remote panes are stored as their password-free address; readers before
// W-080b drop them as unknown entries.
mod pane_paths {
    use serde::{Deserialize as _, Deserializer, Serializer};

    use super::Location;

    pub fn serialize<S: Serializer>(panes: &[Location], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(panes.iter().map(Location::to_saved))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Location>, D::Error> {
        let values = Vec::<String>::deserialize(deserializer)?;
        Ok(values
            .iter()
            .filter_map(|value| Location::from_saved(value))
            .collect())
    }
}

/// Prepares session snapshots on the UI thread for background publication.
#[derive(Default)]
pub struct SessionSaves {
    issued: u64,
    latest_attempt: Arc<Mutex<u64>>,
}

impl SessionSaves {
    /// Encodes `session` as the next save, numbered after earlier ones.
    pub fn prepare(&mut self, session: Session) -> io::Result<SessionSave> {
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
pub struct SessionSave {
    session: Session,
    revision: u64,
    latest_attempt: Arc<Mutex<u64>>,
}

impl SessionSave {
    /// Writes this save to `path`, unless a newer save already reached it.
    pub fn write(self, path: &Path) -> io::Result<()> {
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
    use rustphin_platform::remote::RemoteAddress;

    use super::*;

    #[allow(clippy::unwrap_used)]
    fn parse(text: &str) -> Session {
        Session::parse(text).unwrap()
    }

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
        assert_eq!(parse(&saved), latest);
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
        assert_eq!(parse(&saved), latest);
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
                    panes: vec![PathBuf::from("/Été").into(), raw.into()],
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
        assert_eq!(parse(&saved), session);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_legacy_session_keeps_literal_percent_paths_and_upgrades_when_saved() {
        let restored = parse(
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
        assert_eq!(parse(&saved), restored);
    }

    #[test]
    fn uri_panes_ignore_remote_relative_and_nul_paths_before_clamping() {
        let restored = parse(
            "version = 2\n[[tabs]]\npanes = ['file://remote/data', 'file:///bad%00name', 'relative', 'file:///saved%20folder']\n",
        );
        assert_eq!(restored, Session::single("/saved folder".into()));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn remote_panes_are_kept_by_address_without_their_password() {
        let remote = RemoteAddress::parse("sftp://me:hunter2@host/srv/sub dir").unwrap();
        let session = Session::new(
            vec![SessionTab {
                panes: vec![
                    Location::Remote(remote.clone()),
                    PathBuf::from("/home/me").into(),
                ],
                active_pane: 0,
            }],
            0,
        );

        let text = session.to_toml().unwrap();

        assert!(!text.contains("hunter2"), "{text}");
        let restored = parse(&text);
        assert_eq!(
            restored.tabs[0].panes,
            [
                Location::Remote(RemoteAddress::parse("sftp://me@host/srv/sub dir").unwrap()),
                PathBuf::from("/home/me").into(),
            ]
        );
    }

    #[test]
    fn an_unknown_newer_schema_is_read_only() {
        let restored = restore(
            Ok(Some(format!("version = {}\ntabs = 7\n", VERSION + 1))),
            &[],
        );
        assert!(!restored.can_save());
        assert!(
            matches!(restored.issue, Some(SessionError::NewerVersion(version)) if version == VERSION + 1)
        );
    }

    #[test]
    fn explicit_folders_do_not_authorize_replacing_a_newer_session() {
        let restored = restore(
            Ok(Some(format!("version = {}\n", VERSION + 1))),
            &["/requested".into()],
        );
        assert_eq!(restored.session, Session::single("/requested".into()));
        assert!(!restored.can_save());
    }

    #[test]
    fn a_read_failure_disables_saving_with_explicit_folders() {
        let restored = restore(
            Err(io::ErrorKind::PermissionDenied.into()),
            &["/requested".into()],
        );
        assert_eq!(restored.session, Session::single("/requested".into()));
        assert!(!restored.can_save());
        assert!(
            matches!(restored.issue, Some(SessionError::Read(error)) if error.kind() == io::ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn damaged_current_data_is_not_replaced_by_fallback_tabs() {
        let restored = restore(Ok(Some(format!("version = {VERSION}\ntabs = 7\n"))), &[]);
        assert!(restored.session.tabs.is_empty());
        assert!(!restored.can_save());
        assert!(matches!(restored.issue, Some(SessionError::Damaged(_))));
    }

    #[test]
    fn a_missing_file_allows_the_first_session_save() {
        let restored = restore(Ok(None), &["/requested".into()]);
        assert_eq!(restored.session, Session::single("/requested".into()));
        assert!(restored.can_save());
    }

    fn tab(panes: &[&str], active_pane: usize) -> SessionTab {
        SessionTab {
            panes: panes
                .iter()
                .map(|path| Location::Folder(PathBuf::from(path)))
                .collect(),
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
        assert_eq!(parse(&saved.to_toml().unwrap()), saved);
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
    fn empty_documents_load_but_damaged_sessions_report_errors() {
        assert!(parse("").tabs.is_empty());
        assert!(matches!(
            Session::parse("not [ toml"),
            Err(SessionError::Damaged(_))
        ));
        assert!(matches!(
            Session::parse("tabs = \"oops\""),
            Err(SessionError::Damaged(_))
        ));
    }

    #[test]
    fn unusable_entries_are_dropped_and_indices_clamped() {
        let text = "active = 7\n\
            [[tabs]]\npanes = []\n\
            [[tabs]]\npanes = [\"relative/path\"]\n\
            [[tabs]]\npanes = [\"/a\", \"/b\", \"/c\"]\nactive_pane = 9\n";
        let session = parse(text);
        assert_eq!(session.tabs, [tab(&["/a", "/b"], 1)]);
        assert_eq!(session.active, 0);
    }

    #[test]
    fn a_newer_version_is_rejected_before_decoding_its_schema() {
        assert!(
            matches!(Session::parse(&format!("version = {}\ntabs = 7\n", VERSION + 1)), Err(SessionError::NewerVersion(version)) if version == VERSION + 1)
        );
    }
}
