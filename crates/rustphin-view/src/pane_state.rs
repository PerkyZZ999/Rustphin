//! What one browsing pane holds between snapshots, free of any toolkit
//! (W-121): the folder it is in, kept as the path itself. The Qt list
//! model shows this folder as text, but never reads a path back from that
//! text: a folder whose name is not UTF-8 would come back as another path.

use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};

/// The state of one pane.
#[derive(Debug, Default)]
pub struct PaneState {
    /// The folder asked for or shown; `None` before the first and while
    /// the pane changes filesystem.
    folder: Option<PathBuf>,
}

impl PaneState {
    /// The folder the pane is in: the one asked for, or shown.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Makes `path` the pane's folder; false when it already was.
    pub fn set_folder(&mut self, path: &Path) -> bool {
        if self.folder.as_deref() == Some(path) {
            return false;
        }
        self.folder = Some(path.to_owned());
        true
    }

    /// Forgets the folder, so the next one counts as a change.
    pub fn forget_folder(&mut self) {
        self.folder = None;
    }

    /// The path of the entry called `name` in the pane's folder.
    pub fn entry_path(&self, name: &[u8]) -> Option<PathBuf> {
        Some(self.folder.as_deref()?.join(OsStr::from_bytes(name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "café" in Latin-1: not UTF-8.
    fn latin1_folder() -> PathBuf {
        Path::new("/home/user").join(OsStr::from_bytes(b"caf\xe9"))
    }

    #[test]
    fn a_folder_whose_name_is_not_utf8_is_kept_byte_for_byte() {
        let mut pane = PaneState::default();
        pane.set_folder(&latin1_folder());
        assert_eq!(pane.folder(), Some(latin1_folder().as_path()));
        // Renaming "old" in it acts on the folder itself, not a look-alike.
        assert_eq!(
            pane.entry_path(b"old").map(PathBuf::into_os_string),
            Some(latin1_folder().join("old").into_os_string()),
        );
    }

    #[test]
    fn two_folders_that_display_alike_are_different_folders() {
        let mut pane = PaneState::default();
        assert!(pane.set_folder(&latin1_folder()));
        // Shown as text, both read "caf\u{fffd}"; they are still two.
        let other = Path::new("/home/user").join(OsStr::from_bytes(b"caf\xff"));
        assert!(pane.set_folder(&other));
        assert!(!pane.set_folder(&other));
    }

    #[test]
    fn a_forgotten_folder_is_no_folder() {
        let mut pane = PaneState::default();
        pane.set_folder(Path::new("/tmp"));
        pane.forget_folder();
        assert_eq!(pane.folder(), None);
        assert_eq!(pane.entry_path(b"x"), None);
        assert!(pane.set_folder(Path::new("/tmp")));
    }
}
