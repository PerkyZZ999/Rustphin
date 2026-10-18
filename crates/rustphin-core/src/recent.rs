//! Recently visited folders: bounded, distinct, newest first, with byte-safe paths.

use std::{collections::HashSet, os::unix::ffi::OsStrExt as _, path::PathBuf};

/// Maximum number of folders retained by the palette (R-015's benchmark size).
pub const RECENT_LOCATION_LIMIT: usize = 10_000;

/// Recent successful directory visits. This type does no filesystem I/O.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentLocations {
    paths: Vec<PathBuf>,
}

fn usable(path: &std::path::Path) -> bool {
    path.is_absolute() && !path.as_os_str().as_bytes().contains(&0)
}

impl RecentLocations {
    /// Restores newest-first paths, dropping duplicates, relative paths and
    /// embedded NUL bytes. Keeps the first occurrence and bounds the result.
    pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut seen = HashSet::new();
        let paths = paths
            .into_iter()
            .filter(|path| usable(path) && seen.insert(path.clone()))
            .take(RECENT_LOCATION_LIMIT)
            .collect();
        Self { paths }
    }

    /// Records a successful visit, moving an existing path to the front.
    /// Returns whether history changed. Callers must not record failed opens.
    pub fn visit(&mut self, path: PathBuf) -> bool {
        if !usable(&path) || self.paths.first() == Some(&path) {
            return false;
        }
        self.paths.retain(|old| old != &path);
        self.paths.insert(0, path);
        self.paths.truncate(RECENT_LOCATION_LIMIT);
        true
    }

    /// The recorded paths, newest first, without any name conversion.
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Forgets every recorded visit. Returns whether anything was forgotten.
    pub fn clear(&mut self) -> bool {
        let changed = !self.paths.is_empty();
        self.paths.clear();
        changed
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::ffi::OsStringExt as _;

    use super::*;

    #[test]
    fn revisiting_a_folder_moves_it_to_the_front_without_duplicates() {
        let mut history = RecentLocations::from_paths(["/a".into(), "/b".into()]);
        assert!(history.visit("/b".into()));
        assert_eq!(history.paths(), [PathBuf::from("/b"), PathBuf::from("/a")]);
        assert!(!history.visit("/b".into()));
    }

    #[test]
    fn restoring_keeps_the_newest_occurrence_and_rejects_unusable_paths() {
        let history = RecentLocations::from_paths([
            "/b".into(),
            "relative".into(),
            "/a".into(),
            "/b".into(),
            "/bad\0name".into(),
        ]);
        assert_eq!(history.paths(), [PathBuf::from("/b"), PathBuf::from("/a")]);
    }

    #[test]
    fn a_new_visit_evicts_the_oldest_when_history_is_full() {
        let mut history = RecentLocations::from_paths(
            (0..RECENT_LOCATION_LIMIT + 1).map(|index| PathBuf::from(format!("/{index}"))),
        );
        assert_eq!(history.paths().len(), RECENT_LOCATION_LIMIT);
        assert!(history.visit("/new".into()));
        assert_eq!(history.paths().first(), Some(&PathBuf::from("/new")));
        assert_eq!(history.paths().last(), Some(&PathBuf::from("/9998")));
        assert_eq!(history.paths().len(), RECENT_LOCATION_LIMIT);
    }

    #[test]
    fn preserves_non_utf8_names_and_control_characters_without_loss() {
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"/data/\xff\n".to_vec()));
        let mut history = RecentLocations::default();
        assert!(history.visit(path.clone()));
        assert_eq!(history.paths(), [path]);
    }

    #[test]
    fn clearing_forgets_every_visit_and_invalid_visits_do_not_change_history() {
        let mut history = RecentLocations::from_paths(["/a".into()]);
        assert!(!history.visit("relative".into()));
        assert!(!history.visit("/bad\0name".into()));
        assert!(history.clear());
        assert!(history.paths().is_empty());
        assert!(!history.clear());
    }
}
