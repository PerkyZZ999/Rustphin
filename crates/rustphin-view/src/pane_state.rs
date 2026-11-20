//! What one browsing pane holds between snapshots, free of any toolkit
//! (W-121, W-122): the folder it is in, kept as the path itself; the
//! listing shown and the newest asked for; the selection; the name being
//! edited; the selections waiting for entries to appear; the failure
//! shown. [`PaneState::arrive`] takes each new snapshot and says what
//! changed, so the Qt list model only redraws.
//!
//! The Qt model shows the folder as text but never reads a path back from
//! that text: a folder whose name is not UTF-8 would come back as another
//! path.

use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustphin_core::{Selection, Snapshot};

use crate::pane::{Failure, select_names};

/// The state of one pane.
#[derive(Debug, Default)]
pub struct PaneState {
    /// The folder asked for or shown; `None` before the first and while
    /// the pane changes filesystem.
    folder: Option<PathBuf>,
    /// The listing shown; it may belong to another folder while the next
    /// one is asked for.
    snapshot: Option<Arc<Snapshot>>,
    /// The newest generation asked of the model.
    generation: u64,
    selection: Selection,
    /// The name being edited in place.
    renaming: Option<Vec<u8>>,
    /// Names to select once the folder now asked for is listed in full.
    select_on_arrival: Option<Vec<Vec<u8>>>,
    /// A name to select once it appears; `true` to rename it then.
    select_when_seen: Option<(Vec<u8>, bool)>,
    /// Names to select together once every one has appeared.
    select_all_when_seen: Vec<Vec<u8>>,
    /// Why the folder could not be shown, if it could not.
    failure: Option<Failure>,
    /// The folder last recorded in the history of visited folders.
    visited: Option<PathBuf>,
    /// The listing a reload the user asked for (F5, Reload) will bring.
    reloading: Option<u64>,
    /// The first entry is to show the focus once a listing has rows: a new
    /// folder was entered, or the keyboard came while there were none.
    focus_owed: bool,
}

/// What a new snapshot changed, for the view to act on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arrival {
    /// It lists another folder than the one shown before: its filter,
    /// trash state, free space and view are the new folder's.
    pub new_folder: bool,
    /// It is the folder shown, read again: what was learnt about its files
    /// (thumbnails) may be stale.
    pub reread: bool,
    /// The entry whose name field opens now.
    pub rename: Option<Vec<u8>>,
    /// The failure shown no longer applies and was cleared.
    pub failure_cleared: bool,
    /// The folder is listed in full for the first time since it was
    /// entered: it goes in the history of visited folders.
    pub visited: bool,
    /// The reload the user asked for is done: the folder is listed in full
    /// again, and the user is told so (W-168).
    pub reloaded: bool,
}

/// A re-read of the folder shown, whose selection is carried over to the
/// new listing by name, on the core's threads, before it arrives.
#[derive(Debug)]
pub struct Carry {
    /// The selection to carry.
    pub selection: Selection,
    /// The listing it was made on.
    pub old: Arc<Snapshot>,
    /// The selection's revision: the carried selection is taken only if
    /// nothing changed the selection meanwhile.
    pub revision: u64,
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

    /// The path of the entry called `name` in the pane's folder.
    pub fn entry_path(&self, name: &[u8]) -> Option<PathBuf> {
        Some(self.folder.as_deref()?.join(OsStr::from_bytes(name)))
    }

    /// The listing shown.
    pub fn snapshot(&self) -> Option<&Arc<Snapshot>> {
        self.snapshot.as_ref()
    }

    /// The selection in the listing shown.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// The selection, to change.
    pub fn selection_mut(&mut self) -> &mut Selection {
        &mut self.selection
    }

    /// The name being edited in place.
    pub fn renaming(&self) -> Option<&[u8]> {
        self.renaming.as_deref()
    }

    /// Starts editing the name `name` in place.
    pub fn start_renaming(&mut self, name: Vec<u8>) {
        self.renaming = Some(name);
    }

    /// Ends editing a name, giving the name that was edited.
    pub fn take_renaming(&mut self) -> Option<Vec<u8>> {
        self.renaming.take()
    }

    /// Why the folder could not be shown, if it could not.
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// Shows or clears a failure.
    pub fn set_failure(&mut self, failure: Option<Failure>) {
        self.failure = failure;
    }

    /// Selects `names` once the folder now asked for is listed in full.
    pub fn select_on_arrival(&mut self, names: Vec<Vec<u8>>) {
        self.select_on_arrival = Some(names);
    }

    /// Selects `name` alone once it appears in this folder, and renames it
    /// then if `rename`.
    pub fn select_when_seen(&mut self, name: Vec<u8>, rename: bool) {
        self.select_when_seen = Some((name, rename));
    }

    /// Selects `names` together once every one has appeared in this folder.
    pub fn select_all_when_seen(&mut self, names: Vec<Vec<u8>>) {
        self.select_all_when_seen = names;
    }

    /// Notes that the model was asked for listing `generation`; older
    /// snapshots are ignored from now on.
    pub fn asked(&mut self, generation: u64) {
        self.generation = generation;
    }

    /// Notes that the user asked to reload the folder (F5, Reload) and the
    /// model will bring it as listing `generation`; its arrival in full
    /// reports [`Arrival::reloaded`].
    pub fn reload_asked(&mut self, generation: u64) {
        self.asked(generation);
        self.reloading = Some(generation);
    }

    /// Whether a snapshot or event of `generation` is still wanted.
    pub fn is_current(&self, generation: u64) -> bool {
        generation >= self.generation
    }

    /// The folder shown went away and the model moved to an ancestor with
    /// listing `generation`; false when that is already superseded.
    pub fn moved(&mut self, generation: u64) -> bool {
        if !self.is_current(generation) {
            return false;
        }
        self.generation = generation;
        self.select_on_arrival = None;
        true
    }

    /// Asking for listing `generation` failed; false when that is not the
    /// newest asked for.
    pub fn failed(&mut self, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.select_on_arrival = None;
        true
    }

    /// When `snapshot` re-reads or re-sorts the folder shown, the selection
    /// to carry over to it before it arrives.
    pub fn carry_to(&self, snapshot: &Snapshot) -> Option<Carry> {
        let old = self
            .snapshot
            .as_ref()
            .filter(|old| old.path() == snapshot.path() && old.listing() != snapshot.listing())?;
        Some(Carry {
            selection: self.selection.clone(),
            old: Arc::clone(old),
            revision: self.selection.revision(),
        })
    }

    /// Takes `snapshot` as the listing shown, with the selection `carried`
    /// over to it (from [`Self::carry_to`]) when one was; `None` when a
    /// newer listing has been asked for since, and nothing changes.
    pub fn arrive(
        &mut self,
        snapshot: Arc<Snapshot>,
        carried: Option<(u64, Selection)>,
    ) -> Option<Arrival> {
        if !self.is_current(snapshot.generation()) {
            return None;
        }
        let new_folder = self
            .snapshot
            .as_ref()
            .is_none_or(|shown| shown.path() != snapshot.path());
        let reread = !new_folder
            && self
                .snapshot
                .as_ref()
                .is_some_and(|shown| shown.listing() != snapshot.listing());
        self.generation = snapshot.generation();
        match carried {
            Some((revision, selection)) if self.selection.revision() == revision => {
                self.selection = selection;
            }
            _ => self.selection.sync(&snapshot),
        }
        let reloaded = !new_folder
            && !snapshot.is_loading()
            && self
                .reloading
                .take_if(|asked| snapshot.generation() >= *asked)
                .is_some();
        if new_folder {
            self.focus_owed = true;
            self.reloading = None;
            self.renaming = None;
            self.select_when_seen = None;
            self.select_all_when_seen.clear();
        }
        if let Some(names) = self.select_on_arrival.take_if(|_| !snapshot.is_loading()) {
            select_names(&mut self.selection, &snapshot, &names);
        }
        let mut rename = None;
        if let Some((name, rename_it)) = self.select_when_seen.take() {
            match snapshot.row_of(&name) {
                Some(row) => {
                    self.selection.select_only(&snapshot, row);
                    rename = rename_it.then_some(name);
                }
                None => self.select_when_seen = Some((name, rename_it)),
            }
        }
        if !self.select_all_when_seen.is_empty()
            && snapshot
                .rows_of(&self.select_all_when_seen)
                .iter()
                .all(Option::is_some)
        {
            let names = std::mem::take(&mut self.select_all_when_seen);
            select_names(&mut self.selection, &snapshot, &names);
        }
        if self
            .renaming
            .as_ref()
            .is_some_and(|name| snapshot.row_of(name).is_none())
        {
            self.renaming = None;
        }
        // A failure stays only while it explains this listing: the folder
        // that went away, over the parent shown in its place.
        let failure_cleared = self
            .failure
            .take_if(|failure| !failure.moved || failure.path.parent() != Some(snapshot.path()));
        let visited = !snapshot.is_loading() && self.visited.as_deref() != Some(snapshot.path());
        if visited {
            self.visited = Some(snapshot.path().to_owned());
        }
        self.snapshot = Some(snapshot);
        Some(Arrival {
            new_folder,
            reread,
            rename,
            failure_cleared: failure_cleared.is_some(),
            visited,
            reloaded,
        })
    }

    /// The keyboard reached the pane (Tab, Shift+Tab, a click): with no
    /// focused entry, the first one on screen, `first_shown`, takes the
    /// focus without being selected, so the focus can be seen (R-107).
    /// `true` when the focus moved.
    pub fn keyboard_arrived(&mut self, first_shown: usize) -> bool {
        let Some(snapshot) = self
            .snapshot
            .clone()
            .filter(|snapshot| !snapshot.is_empty())
        else {
            self.focus_owed = true;
            return false;
        };
        self.focus_owed = false;
        if self.selection.focus_row(&snapshot).is_some() {
            return false;
        }
        self.selection
            .focus_only(&snapshot, first_shown.min(snapshot.len() - 1));
        true
    }

    /// A listing arrived while the pane has the keyboard: as
    /// [`Self::keyboard_arrived`], but only when the focus is owed (a new
    /// folder, or rows the keyboard was waiting for). A re-read of the
    /// folder shown leaves the focus as the user left it, none after a
    /// click on empty space (W-167).
    pub fn listing_arrived(&mut self, first_shown: usize) -> bool {
        self.focus_owed && self.keyboard_arrived(first_shown)
    }

    /// Forgets the folder and its listing, as when the pane starts listing
    /// through another filesystem; what comes next is a new folder.
    pub fn leave_filesystem(&mut self) {
        self.folder = None;
        self.generation = 0;
        self.snapshot = None;
        self.visited = None;
        self.selection = Selection::default();
        self.focus_owed = true;
    }
}

#[cfg(test)]
// Test helpers outside `#[test]` functions unwrap what a test cannot go on without.
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        collections::HashMap,
        io,
        sync::{Arc, Mutex},
    };

    use rustphin_core::{
        DirectoryModel, EntryKind, Fs, LoadError, Metadata, ModelEvent, RawEntry, WatchGuard,
    };

    use super::*;

    /// "café" in Latin-1: not UTF-8.
    fn latin1_folder() -> PathBuf {
        Path::new("/home/user").join(OsStr::from_bytes(b"caf\xe9"))
    }

    /// Folders held in memory, each a list of file names.
    #[derive(Default)]
    struct MemFs {
        folders: Mutex<HashMap<PathBuf, Vec<Vec<u8>>>>,
    }

    impl MemFs {
        fn put(&self, folder: &Path, names: &[&[u8]]) {
            if let Ok(mut folders) = self.folders.lock() {
                folders.insert(
                    folder.to_owned(),
                    names.iter().map(|name| name.to_vec()).collect(),
                );
            }
        }
    }

    impl Fs for MemFs {
        fn read_dir(
            &self,
            path: &Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<RawEntry>> + '_>> {
            let names = self
                .folders
                .lock()
                .map_err(|_| io::Error::other("poisoned"))?
                .get(path)
                .cloned()
                .ok_or(io::ErrorKind::NotFound)?;
            Ok(Box::new(names.into_iter().map(|name| {
                Ok(RawEntry {
                    name,
                    kind: EntryKind::File,
                })
            })))
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.folders
                .lock()
                .is_ok_and(|folders| folders.contains_key(path))
        }

        fn metadata(&self, _: &Path) -> io::Result<Metadata> {
            Ok(Metadata {
                size: 1,
                modified: 0,
            })
        }

        fn watch(&self, _: &Path, _: Box<dyn Fn() + Send + Sync>) -> io::Result<WatchGuard> {
            Ok(Box::new(()))
        }
    }

    /// A pane over a model of a [`MemFs`], fed as the Qt model feeds it.
    struct Harness {
        fs: Arc<MemFs>,
        model: DirectoryModel,
        events: flume::Receiver<ModelEvent>,
        pane: PaneState,
    }

    impl Harness {
        fn new() -> Self {
            let fs = Arc::new(MemFs::default());
            let (model, events) = DirectoryModel::new(fs.clone()).unwrap();
            Self {
                fs,
                model,
                events,
                pane: PaneState::default(),
            }
        }

        /// The complete snapshot of `generation`.
        fn listed(&self, generation: u64) -> Arc<Snapshot> {
            loop {
                match self.events.recv().unwrap() {
                    ModelEvent::Snapshot(snapshot)
                        if snapshot.generation() == generation && !snapshot.is_loading() =>
                    {
                        return snapshot;
                    }
                    _ => {}
                }
            }
        }

        /// Opens `folder` and lets its listing arrive.
        fn open(&mut self, folder: &Path) -> Arrival {
            let generation = self.model.open(folder);
            self.pane.set_folder(folder);
            self.pane.asked(generation);
            let snapshot = self.listed(generation);
            self.pane.arrive(snapshot, None).unwrap()
        }

        /// Reads the folder again (the watcher's or F5's re-read) and lets
        /// the listing arrive with the selection carried, as the Qt model
        /// does.
        fn reread(&mut self) -> Arrival {
            let generation = self.model.reload().unwrap();
            self.pane.asked(generation);
            let snapshot = self.listed(generation);
            let carried = self.pane.carry_to(&snapshot).map(|carry| {
                let revision = carry.revision;
                let selection = self
                    .model
                    .carry_selection(carry.selection, carry.old, snapshot.clone())
                    .recv()
                    .unwrap();
                (revision, selection)
            });
            self.pane.arrive(snapshot, carried).unwrap()
        }

        fn selected(&self) -> Vec<Vec<u8>> {
            let snapshot = self.pane.snapshot().unwrap();
            self.pane
                .selection()
                .rows(snapshot)
                .into_iter()
                .map(|row| snapshot.name(row).to_vec())
                .collect()
        }

        fn select(&mut self, names: &[&[u8]]) {
            let snapshot = self.pane.snapshot().unwrap().clone();
            let names: Vec<Vec<u8>> = names.iter().map(|name| name.to_vec()).collect();
            select_names(self.pane.selection_mut(), &snapshot, &names);
        }
    }

    fn names(names: &[&[u8]]) -> Vec<Vec<u8>> {
        names.iter().map(|name| name.to_vec()).collect()
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
    fn the_keyboard_reaching_a_pane_shows_its_focus_on_the_first_entry_on_screen() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"a", b"b", b"c", b"d"]);
        harness.open(Path::new("/a"));
        assert!(harness.pane.keyboard_arrived(2));
        let snapshot = harness.pane.snapshot().unwrap().clone();
        let selection = harness.pane.selection();
        assert_eq!(selection.focus_row(&snapshot), Some(2));
        // Focused, not selected: nothing acts on it until the user chooses.
        assert!(selection.is_empty());
        // An entry already focused keeps the focus.
        assert!(!harness.pane.keyboard_arrived(0));
        assert_eq!(harness.pane.selection().focus_row(&snapshot), Some(2));
    }

    /// W-167: a click on empty space leaves no entry focused, and a re-read
    /// of the folder (F5, the watcher) must not bring the focus back; a
    /// new folder, or rows arriving after the keyboard found none, do.
    #[test]
    fn a_listing_shows_the_focus_only_when_one_is_owed() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"a", b"b"]);
        harness.fs.put(Path::new("/b"), &[b"c"]);
        harness.open(Path::new("/a"));
        // A new folder with the keyboard in the pane focuses its first entry.
        assert!(harness.pane.listing_arrived(0));
        // The user clicks empty space: nothing is focused.
        harness.pane.selection_mut().clear();
        harness.reread();
        assert!(!harness.pane.listing_arrived(0), "a re-read refocused");
        let snapshot = harness.pane.snapshot().unwrap().clone();
        assert_eq!(harness.pane.selection().focus_row(&snapshot), None);
        // The keyboard arriving again (Tab) still shows it (R-107).
        assert!(harness.pane.keyboard_arrived(0));
        // Another folder owes the focus again.
        harness.open(Path::new("/b"));
        assert!(harness.pane.listing_arrived(0));
    }

    #[test]
    fn rows_arriving_after_the_keyboard_found_none_take_the_focus() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[]);
        harness.open(Path::new("/a"));
        // Nothing to focus yet: the focus stays owed.
        assert!(!harness.pane.listing_arrived(0));
        assert!(!harness.pane.keyboard_arrived(0));
        harness.fs.put(Path::new("/a"), &[b"new"]);
        harness.reread();
        assert!(harness.pane.listing_arrived(0));
    }

    #[test]
    fn a_selection_without_focus_is_kept_when_the_keyboard_arrives() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"a", b"b", b"c"]);
        harness.open(Path::new("/a"));
        let snapshot = harness.pane.snapshot().unwrap().clone();
        harness.pane.selection_mut().select_all(&snapshot);
        // Past the end of the list: the last entry takes the focus.
        assert!(harness.pane.keyboard_arrived(9));
        assert_eq!(harness.pane.selection().focus_row(&snapshot), Some(2));
        assert_eq!(harness.pane.selection().len(), 3);
    }

    #[test]
    fn an_empty_or_unlisted_pane_has_no_entry_to_focus() {
        let mut harness = Harness::new();
        assert!(!harness.pane.keyboard_arrived(0));
        harness.fs.put(Path::new("/empty"), &[]);
        harness.open(Path::new("/empty"));
        assert!(!harness.pane.keyboard_arrived(0));
    }

    #[test]
    fn leaving_the_filesystem_forgets_the_folder_and_its_listing() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"x"]);
        harness.open(Path::new("/a"));
        harness.pane.leave_filesystem();
        assert_eq!(harness.pane.folder(), None);
        assert_eq!(harness.pane.entry_path(b"x"), None);
        assert!(harness.pane.snapshot().is_none());
        // Coming back to the same folder is arriving in it, and visiting it.
        let arrival = harness.open(Path::new("/a"));
        assert!(arrival.new_folder && arrival.visited);
    }

    #[test]
    fn a_folder_whose_name_is_not_utf8_is_entered_and_listed() {
        let mut harness = Harness::new();
        harness.fs.put(&latin1_folder(), &[b"note.txt"]);
        let arrival = harness.open(&latin1_folder());
        assert!(arrival.new_folder);
        let snapshot = harness.pane.snapshot().unwrap();
        assert_eq!(snapshot.path().as_os_str(), latin1_folder().as_os_str());
        assert_eq!(snapshot.name(0), b"note.txt");
        assert_eq!(
            harness
                .pane
                .entry_path(b"note.txt")
                .map(PathBuf::into_os_string),
            Some(latin1_folder().join("note.txt").into_os_string()),
        );
    }

    #[test]
    fn duplicated_entries_are_selected_together_once_all_have_appeared() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"two"]);
        harness.open(Path::new("/a"));
        harness.select(&[b"one", b"two"]);
        harness
            .pane
            .select_all_when_seen(names(&[b"one copy", b"two copy"]));
        // One copy so far: the originals stay selected.
        harness
            .fs
            .put(Path::new("/a"), &[b"one", b"one copy", b"two"]);
        harness.reread();
        assert_eq!(harness.selected(), names(&[b"one", b"two"]));
        harness
            .fs
            .put(Path::new("/a"), &[b"one", b"one copy", b"two", b"two copy"]);
        harness.reread();
        assert_eq!(harness.selected(), names(&[b"one copy", b"two copy"]));
    }

    #[test]
    fn a_new_entry_is_selected_and_its_name_field_opens_once_it_appears() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one"]);
        harness.open(Path::new("/a"));
        harness.select(&[b"one"]);
        harness.pane.select_when_seen(b"New Folder".to_vec(), true);
        let arrival = harness.reread();
        assert_eq!(arrival.rename, None, "not there yet");
        harness.fs.put(Path::new("/a"), &[b"New Folder", b"one"]);
        let arrival = harness.reread();
        assert_eq!(arrival.rename, Some(b"New Folder".to_vec()));
        assert_eq!(harness.selected(), names(&[b"New Folder"]));
        // Once only.
        assert_eq!(harness.reread().rename, None);
    }

    #[test]
    fn a_renamed_entry_is_selected_without_opening_its_name_field() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"old", b"other"]);
        harness.open(Path::new("/a"));
        harness.pane.select_when_seen(b"new".to_vec(), false);
        harness.fs.put(Path::new("/a"), &[b"new", b"other"]);
        assert_eq!(harness.reread().rename, None);
        assert_eq!(harness.selected(), names(&[b"new"]));
    }

    #[test]
    fn a_snapshot_older_than_the_newest_asked_for_is_ignored() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"x"]);
        harness.fs.put(Path::new("/b"), &[b"y"]);
        let generation = harness.model.open("/a");
        harness.pane.asked(generation);
        let stale = harness.listed(generation);
        let newer = harness.model.open("/b");
        harness.pane.asked(newer);
        assert_eq!(harness.pane.arrive(stale, None), None);
        assert!(harness.pane.snapshot().is_none());
        assert!(!harness.pane.failed(generation));
        assert!(!harness.pane.moved(generation));
    }

    #[test]
    fn a_reread_keeps_the_selection_and_reports_the_reread() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"two"]);
        let first = harness.open(Path::new("/a"));
        assert!(first.new_folder && !first.reread && first.visited);
        harness.select(&[b"two"]);
        harness.fs.put(Path::new("/a"), &[b"one", b"three", b"two"]);
        let arrival = harness.reread();
        assert!(arrival.reread && !arrival.new_folder && !arrival.visited);
        assert_eq!(harness.selected(), names(&[b"two"]));
    }

    #[test]
    fn a_reload_the_user_asked_for_is_reported_once_listed_in_full() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"two"]);
        harness.open(Path::new("/a"));
        // The watcher's re-read is not the user's reload.
        assert!(!harness.reread().reloaded);
        let generation = harness.model.reload().unwrap();
        harness.pane.reload_asked(generation);
        let arrival = harness
            .pane
            .arrive(harness.listed(generation), None)
            .unwrap();
        assert!(arrival.reloaded && arrival.reread);
        // Said once: the next re-read is the watcher's again.
        assert!(!harness.reread().reloaded);
    }

    #[test]
    fn a_reload_overtaken_by_another_folder_is_not_reported() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one"]);
        harness.fs.put(Path::new("/b"), &[b"two"]);
        harness.open(Path::new("/a"));
        let generation = harness.model.reload().unwrap();
        harness.pane.reload_asked(generation);
        assert!(!harness.open(Path::new("/b")).reloaded);
        assert!(!harness.reread().reloaded, "the reload of /a is forgotten");
    }

    #[test]
    fn a_carried_selection_is_dropped_when_the_selection_changed_meanwhile() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"two"]);
        harness.open(Path::new("/a"));
        harness.select(&[b"two"]);
        harness.fs.put(Path::new("/a"), &[b"one", b"two", b"zz"]);
        let generation = harness.model.reload().unwrap();
        harness.pane.asked(generation);
        let snapshot = harness.listed(generation);
        let carry = harness.pane.carry_to(&snapshot).unwrap();
        let carried = harness
            .model
            .carry_selection(carry.selection, carry.old, snapshot.clone())
            .recv()
            .unwrap();
        // The user selects "one" before the carried selection is back.
        harness.select(&[b"one"]);
        harness
            .pane
            .arrive(snapshot, Some((carry.revision, carried)))
            .unwrap();
        // The carried "two" is stale; the user's change was made on the
        // old listing, which the new one replaces.
        assert!(harness.selected().is_empty());
    }

    #[test]
    fn the_rename_ends_when_its_entry_disappears() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"two"]);
        harness.open(Path::new("/a"));
        harness.pane.start_renaming(b"one".to_vec());
        harness.fs.put(Path::new("/a"), &[b"one", b"two", b"three"]);
        harness.reread();
        assert_eq!(harness.pane.renaming(), Some(&b"one"[..]));
        harness.fs.put(Path::new("/a"), &[b"two", b"three"]);
        harness.reread();
        assert_eq!(harness.pane.renaming(), None);
    }

    #[test]
    fn entering_another_folder_drops_what_waited_in_the_last() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"x"]);
        harness.fs.put(Path::new("/b"), &[b"x", b"y"]);
        harness.open(Path::new("/a"));
        harness.pane.start_renaming(b"x".to_vec());
        harness.pane.select_when_seen(b"y".to_vec(), true);
        harness.pane.select_all_when_seen(names(&[b"x", b"y"]));
        let arrival = harness.open(Path::new("/b"));
        assert!(arrival.new_folder && arrival.visited);
        assert!(!arrival.reread, "another folder is not a re-read");
        assert_eq!(arrival.rename, None);
        assert_eq!(harness.pane.renaming(), None);
        assert!(harness.selected().is_empty());
    }

    #[test]
    fn going_up_selects_the_folder_left_once_the_parent_is_listed() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"one", b"sub", b"two"]);
        harness.pane.select_on_arrival(names(&[b"sub"]));
        harness.open(Path::new("/a"));
        assert_eq!(harness.selected(), names(&[b"sub"]));
    }

    #[test]
    fn a_failure_lasts_only_while_it_explains_the_listing() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"x"]);
        harness.open(Path::new("/a"));
        // The folder shown went away and its parent is shown instead.
        harness.pane.set_failure(Some(Failure {
            path: PathBuf::from("/a/gone"),
            error: LoadError::NotFound,
            moved: true,
        }));
        assert!(!harness.reread().failure_cleared);
        assert!(harness.pane.failure().is_some());
        // Any other failure goes with the next listing.
        harness.pane.set_failure(Some(Failure {
            path: PathBuf::from("/a/gone"),
            error: LoadError::NotFound,
            moved: false,
        }));
        assert!(harness.reread().failure_cleared);
        assert!(harness.pane.failure().is_none());
    }

    #[test]
    fn a_failed_or_moved_request_drops_the_selection_waiting_for_it() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"sub"]);
        harness.pane.select_on_arrival(names(&[b"sub"]));
        let generation = harness.model.open("/missing");
        harness.pane.asked(generation);
        assert!(harness.pane.failed(generation));
        harness.open(Path::new("/a"));
        assert!(harness.selected().is_empty());

        harness.pane.select_on_arrival(names(&[b"sub"]));
        let generation = harness.model.reload().unwrap();
        assert!(harness.pane.moved(generation));
        let snapshot = harness.listed(generation);
        harness.pane.arrive(snapshot, None).unwrap();
        assert!(harness.selected().is_empty());
    }

    #[test]
    fn ending_a_rename_gives_the_name_that_was_edited_once() {
        let mut pane = PaneState::default();
        pane.start_renaming(b"caf\xe9".to_vec());
        assert_eq!(pane.take_renaming(), Some(b"caf\xe9".to_vec()));
        assert_eq!(pane.renaming(), None);
        assert_eq!(pane.take_renaming(), None);
    }

    #[test]
    fn a_listing_of_another_folder_carries_no_selection() {
        let mut harness = Harness::new();
        harness.fs.put(Path::new("/a"), &[b"x"]);
        harness.fs.put(Path::new("/b"), &[b"x", b"y"]);
        harness.open(Path::new("/a"));
        harness.select(&[b"x"]);
        let generation = harness.model.open("/b");
        let other = harness.listed(generation);
        assert!(harness.pane.carry_to(&other).is_none());
    }

    #[test]
    fn a_new_folder_takes_a_name_the_listing_does_not_hold() {
        use crate::selection_actions::{Facts, Plan, SelectionAction, plan};

        let mut harness = Harness::new();
        harness
            .fs
            .put(Path::new("/a"), &[b"New Folder", b"New Folder 2"]);
        harness.open(Path::new("/a"));
        let snapshot = harness.pane.snapshot().unwrap().clone();
        let facts = Facts {
            folder: Path::new("/a"),
            in_trash: false,
            as_administrator: false,
            read_only: false,
            selected: Vec::new(),
            listing: &*snapshot,
        };
        let Plan::Start { requests, .. } = plan(SelectionAction::NewFolder, &facts) else {
            panic!("not started");
        };
        assert_eq!(
            requests,
            vec![rustphin_core::ops::Request::CreateDir {
                parent: "/a".into(),
                name: "New Folder 3".into(),
            }]
        );
    }
}
