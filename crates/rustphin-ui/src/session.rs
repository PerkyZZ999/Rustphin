//! The open tabs and panes, remembered between sessions as TOML.
//!
//! Reading never fails: a missing or damaged file yields an empty session,
//! which the workspace replaces with a single tab.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The format version this build writes.
const VERSION: u32 = 1;

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
        Self::new(
            vec![SessionTab {
                panes: vec![folder],
                active_pane: 0,
            }],
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
            tab.panes.retain(|path| path.is_absolute());
            tab.panes.truncate(2);
            tab.active_pane = tab.active_pane.min(tab.panes.len().saturating_sub(1));
        }
        session.tabs.retain(|tab| !tab.panes.is_empty());
        session.active = session.active.min(session.tabs.len().saturating_sub(1));
        session
    }

    /// The session as TOML text.
    pub fn to_toml(&self) -> String {
        // Serialising paths, integers and lists of them cannot fail for valid
        // UTF-8 paths; anything else reads back as an empty session.
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Whether the file was written by a later version of Rustphin and must
    /// not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(panes: &[&str], active_pane: usize) -> SessionTab {
        SessionTab {
            panes: panes.iter().map(PathBuf::from).collect(),
            active_pane,
        }
    }

    #[test]
    fn a_session_survives_being_written_and_read_back() {
        let saved = Session::new(
            vec![tab(&["/home/me"], 0), tab(&["/srv/a", "/srv/b"], 1)],
            1,
        );
        assert_eq!(Session::parse(&saved.to_toml()), saved);
    }

    #[test]
    fn a_single_folder_makes_one_unsplit_tab() {
        let session = Session::single(PathBuf::from("/tmp"));
        assert_eq!(session.tabs, [tab(&["/tmp"], 0)]);
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
        assert!(Session::parse("version = 2").is_from_newer_version());
        assert!(!Session::single(PathBuf::from("/")).is_from_newer_version());
    }
}
