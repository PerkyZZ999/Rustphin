//! The directory model: navigation, background listing and published snapshots.

use std::{
    ffi::OsStr,
    io, mem,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use crate::{CHUNK_LEN, Chunk, EntryKind, EntryStore, Fs, sort};

/// How often, in entries, a running listing checks whether it was superseded.
const CANCEL_CHECK_INTERVAL: usize = 512;

/// Options that decide which entries are shown and in what order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewOptions {
    /// Show entries whose name starts with a dot.
    pub show_hidden: bool,
    /// List directories, and links to directories, before everything else.
    pub folders_first: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            folders_first: true,
        }
    }
}

/// Why a directory could not be opened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// The path does not exist.
    #[error("this location does not exist")]
    NotFound,
    /// The user may not read the directory.
    #[error("you do not have permission to open this location")]
    PermissionDenied,
    /// The path exists but is not a directory.
    #[error("this location is not a folder")]
    NotADirectory,
    /// Any other failure, with the system's description.
    #[error("this location could not be opened: {0}")]
    Io(String),
}

impl From<io::Error> for LoadError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::NotADirectory => Self::NotADirectory,
            _ => Self::Io(error.to_string()),
        }
    }
}

/// An immutable view of one directory: its visible entries in display order.
#[derive(Debug)]
pub struct Snapshot {
    path: PathBuf,
    generation: u64,
    loading: bool,
    options: ViewOptions,
    store: EntryStore,
    order: Vec<u32>,
}

impl Snapshot {
    /// The directory this snapshot shows.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The request this snapshot answers. Later requests have larger numbers.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether the listing is still running and more entries will follow.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// The options this snapshot was built with.
    pub fn options(&self) -> ViewOptions {
        self.options
    }

    /// Number of visible rows.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether there are no visible rows.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Number of entries listed so far, including ones hidden by the options.
    pub fn total(&self) -> usize {
        self.store.len()
    }

    /// The raw file name shown in `row`.
    pub fn name(&self, row: usize) -> &[u8] {
        self.store.name(self.order[row])
    }

    /// The kind of the entry shown in `row`.
    pub fn kind(&self, row: usize) -> EntryKind {
        self.store.kind(self.order[row])
    }

    /// Whether the entry shown in `row` can be entered.
    pub fn is_container(&self, row: usize) -> bool {
        self.store.is_container(self.order[row])
    }

    /// The full path of the entry shown in `row`.
    pub fn entry_path(&self, row: usize) -> PathBuf {
        self.path.join(OsStr::from_bytes(self.name(row)))
    }

    /// The row showing the entry called `name`, if it is visible.
    pub fn row_of(&self, name: &[u8]) -> Option<usize> {
        self.order
            .iter()
            .position(|&index| self.store.name(index) == name)
    }
}

/// What the model publishes to its consumer.
#[derive(Debug, Clone)]
pub enum ModelEvent {
    /// A new view of the directory being shown.
    Snapshot(Arc<Snapshot>),
    /// A request to open `path` failed. The model's location is back where it was.
    Failed {
        /// The failed request.
        generation: u64,
        /// The directory that could not be opened.
        path: PathBuf,
        /// Why.
        error: LoadError,
    },
}

#[derive(Debug, Clone, Default)]
struct History {
    entries: Vec<PathBuf>,
    position: usize,
}

impl History {
    fn current(&self) -> Option<&PathBuf> {
        self.entries.get(self.position)
    }
}

struct Complete {
    generation: u64,
    path: PathBuf,
    store: EntryStore,
}

struct Shared {
    fs: Arc<dyn Fs>,
    events: flume::Sender<ModelEvent>,
    pool: rayon::ThreadPool,
    generation: AtomicU64,
    history: Mutex<History>,
    complete: Mutex<Option<Complete>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // No guarded section can leave these values half-updated, so a poisoned lock is still usable.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }

    fn publish(&self, request: &Request, store: &EntryStore, loading: bool) {
        let order = self
            .pool
            .install(|| sort::build_order(store, request.options));
        if !self.is_current(request.generation) {
            return;
        }
        let snapshot = Snapshot {
            path: request.path.clone(),
            generation: request.generation,
            loading,
            options: request.options,
            store: store.clone(),
            order,
        };
        // The consumer having gone away is not an error for a worker.
        let _ = self.events.send(ModelEvent::Snapshot(Arc::new(snapshot)));
    }

    fn fail(&self, request: Request, error: LoadError) {
        {
            let mut history = lock(&self.history);
            if !self.is_current(request.generation) {
                return;
            }
            *history = request.restore;
        }
        let _ = self.events.send(ModelEvent::Failed {
            generation: request.generation,
            path: request.path,
            error,
        });
    }

    fn list(&self, request: Request) {
        let entries = match self.fs.read_dir(&request.path) {
            Ok(entries) => entries,
            Err(error) => return self.fail(request, error.into()),
        };
        let mut store = EntryStore::default();
        let mut chunk = Chunk::default();
        let mut publish_at = CHUNK_LEN;
        let mut skipped = 0usize;
        for (count, entry) in entries.enumerate() {
            if count % CANCEL_CHECK_INTERVAL == 0 && !self.is_current(request.generation) {
                return;
            }
            let Ok(entry) = entry else {
                skipped += 1;
                continue;
            };
            let container = match entry.kind {
                EntryKind::Directory => true,
                EntryKind::Symlink => self
                    .fs
                    .is_dir(&request.path.join(OsStr::from_bytes(&entry.name))),
                EntryKind::File | EntryKind::Other => false,
            };
            chunk.push(&entry.name, entry.kind, container);
            if chunk.len() == CHUNK_LEN {
                store.push_chunk(mem::take(&mut chunk));
                // Publish at doubling sizes so the total sorting work stays linear.
                if store.len() >= publish_at {
                    self.publish(&request, &store, true);
                    publish_at *= 2;
                }
            }
        }
        if !chunk.is_empty() {
            store.push_chunk(chunk);
        }
        if skipped > 0 {
            tracing::warn!(path = %request.path.display(), skipped, "unreadable directory entries");
        }
        self.finish(request, store);
    }

    /// Records a completed listing and publishes its final snapshot.
    ///
    /// The record is stored first: a consumer reacting to the snapshot must
    /// already find the listing reusable.
    fn finish(&self, request: Request, store: EntryStore) {
        if !self.is_current(request.generation) {
            return;
        }
        *lock(&self.complete) = Some(Complete {
            generation: request.generation,
            path: request.path.clone(),
            store: store.clone(),
        });
        self.publish(&request, &store, false);
    }
}

struct Request {
    path: PathBuf,
    generation: u64,
    options: ViewOptions,
    /// History to put back if the request fails.
    restore: History,
}

/// Navigation and listing for one view of the filesystem.
///
/// Every method returns immediately. Listing and sorting run on worker threads
/// and their results arrive as [`ModelEvent`]s on the channel returned by
/// [`DirectoryModel::new`]. Each request has a generation number; consumers
/// should ignore snapshots older than the latest generation they asked for.
pub struct DirectoryModel {
    shared: Arc<Shared>,
    options: ViewOptions,
}

impl DirectoryModel {
    /// Creates a model reading through `fs`, and the channel its events arrive on.
    pub fn new(fs: Arc<dyn Fs>) -> io::Result<(Self, flume::Receiver<ModelEvent>)> {
        let (events, receiver) = flume::unbounded();
        let pool = rayon::ThreadPoolBuilder::new()
            .thread_name(|index| format!("rustphin-cpu-{index}"))
            .build()
            .map_err(io::Error::other)?;
        let shared = Arc::new(Shared {
            fs,
            events,
            pool,
            generation: AtomicU64::new(0),
            history: Mutex::default(),
            complete: Mutex::default(),
        });
        Ok((
            Self {
                shared,
                options: ViewOptions::default(),
            },
            receiver,
        ))
    }

    /// The directory currently shown, if any has been opened.
    pub fn location(&self) -> Option<PathBuf> {
        lock(&self.shared.history).current().cloned()
    }

    /// The options in effect.
    pub fn options(&self) -> ViewOptions {
        self.options
    }

    /// Whether [`Self::back`] would navigate.
    pub fn can_go_back(&self) -> bool {
        lock(&self.shared.history).position > 0
    }

    /// Whether [`Self::forward`] would navigate.
    pub fn can_go_forward(&self) -> bool {
        let history = lock(&self.shared.history);
        history.position + 1 < history.entries.len()
    }

    /// Opens `path` as a new step in the history. Returns the request's generation.
    pub fn open(&mut self, path: impl Into<PathBuf>) -> u64 {
        let path = path.into();
        self.navigate(|history| {
            if history.current() == Some(&path) {
                return;
            }
            if !history.entries.is_empty() {
                history.entries.truncate(history.position + 1);
            }
            history.entries.push(path);
            history.position = history.entries.len() - 1;
        })
    }

    /// Opens the parent of the current directory. Returns `None` at the root.
    pub fn up(&mut self) -> Option<u64> {
        let parent = self.location()?.parent()?.to_path_buf();
        Some(self.open(parent))
    }

    /// Goes one step back in the history, if possible.
    pub fn back(&mut self) -> Option<u64> {
        self.can_go_back()
            .then(|| self.navigate(|history| history.position -= 1))
    }

    /// Goes one step forward in the history, if possible.
    pub fn forward(&mut self) -> Option<u64> {
        self.can_go_forward()
            .then(|| self.navigate(|history| history.position += 1))
    }

    /// Lists the current directory again.
    pub fn reload(&mut self) -> Option<u64> {
        self.location()?;
        Some(self.navigate(|_| {}))
    }

    /// Changes the view options and republishes the current directory.
    pub fn set_options(&mut self, options: ViewOptions) -> Option<u64> {
        if options == self.options {
            return None;
        }
        self.options = options;
        let current = self.shared.generation.load(Ordering::Acquire);
        let listed = lock(&self.shared.complete)
            .as_ref()
            .filter(|complete| complete.generation == current)
            .map(|complete| (complete.path.clone(), complete.store.clone()));
        let Some((path, store)) = listed else {
            return self.reload();
        };
        // The listing is complete: re-sort it without touching the filesystem.
        let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let restore = lock(&self.shared.history).clone();
        let request = Request {
            path,
            generation,
            options,
            restore,
        };
        let shared = Arc::clone(&self.shared);
        self.spawn("rustphin-sort", request, move |request| {
            shared.finish(request, store)
        });
        Some(generation)
    }

    /// Applies `change` to the history and starts listing the resulting location.
    fn navigate(&mut self, change: impl FnOnce(&mut History)) -> u64 {
        let (request, generation) = {
            let mut history = lock(&self.shared.history);
            let restore = history.clone();
            change(&mut history);
            let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
            let path = history.current().cloned().unwrap_or_default();
            (
                Request {
                    path,
                    generation,
                    options: self.options,
                    restore,
                },
                generation,
            )
        };
        let shared = Arc::clone(&self.shared);
        self.spawn("rustphin-list", request, move |request| {
            shared.list(request)
        });
        generation
    }

    fn spawn(&self, name: &str, request: Request, work: impl FnOnce(Request) + Send + 'static) {
        let (generation, path, restore) = (
            request.generation,
            request.path.clone(),
            request.restore.clone(),
        );
        let spawned = thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || work(request));
        if let Err(error) = spawned {
            let options = self.options;
            self.shared.fail(
                Request {
                    path,
                    generation,
                    options,
                    restore,
                },
                error.into(),
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{fs, os::unix::ffi::OsStringExt, time::Duration};

    use super::*;
    use crate::{RawEntry, RealFs};

    const TIMEOUT: Duration = Duration::from_secs(10);

    /// Waits for the final outcome of `generation`.
    fn settle(events: &flume::Receiver<ModelEvent>, generation: u64) -> ModelEvent {
        loop {
            let event = events.recv_timeout(TIMEOUT).expect("model event");
            match &event {
                ModelEvent::Snapshot(s) if s.generation() == generation && !s.is_loading() => {
                    return event;
                }
                ModelEvent::Failed {
                    generation: failed, ..
                } if *failed == generation => {
                    return event;
                }
                _ => {}
            }
        }
    }

    fn snapshot(events: &flume::Receiver<ModelEvent>, generation: u64) -> Arc<Snapshot> {
        match settle(events, generation) {
            ModelEvent::Snapshot(snapshot) => snapshot,
            other => panic!("expected a snapshot, got {other:?}"),
        }
    }

    fn names(snapshot: &Snapshot) -> Vec<String> {
        (0..snapshot.len())
            .map(|row| String::from_utf8_lossy(snapshot.name(row)).into_owned())
            .collect()
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in ["file10.txt", "file2.txt", ".hidden", "Zeta.md"] {
            fs::write(dir.path().join(name), b"").unwrap();
        }
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/inner.txt"), b"").unwrap();
        std::os::unix::fs::symlink(dir.path().join("sub"), dir.path().join("alias")).unwrap();
        dir
    }

    #[test]
    fn lists_sorted_with_folders_first() {
        let dir = tree();
        let (mut model, events) = DirectoryModel::new(Arc::new(RealFs)).unwrap();
        let shown = snapshot(&events, model.open(dir.path()));

        assert_eq!(
            names(&shown),
            ["alias", "sub", "file2.txt", "file10.txt", "Zeta.md"]
        );
        assert_eq!(shown.total(), 6);
        assert!(shown.is_container(0) && shown.kind(0) == EntryKind::Symlink);
        assert!(!shown.is_container(2));
        assert_eq!(shown.entry_path(1), dir.path().join("sub"));
        assert_eq!(shown.row_of(b"Zeta.md"), Some(4));
    }

    #[test]
    fn options_resort_without_relisting() {
        let dir = tree();
        let (mut model, events) = DirectoryModel::new(Arc::new(RealFs)).unwrap();
        snapshot(&events, model.open(dir.path()));
        // Entries created after the listing must not appear: the store is reused.
        fs::write(dir.path().join("later.txt"), b"").unwrap();

        let options = ViewOptions {
            show_hidden: true,
            folders_first: false,
        };
        let shown = snapshot(&events, model.set_options(options).unwrap());
        assert_eq!(
            names(&shown),
            [
                ".hidden",
                "alias",
                "file2.txt",
                "file10.txt",
                "sub",
                "Zeta.md"
            ]
        );
        assert_eq!(model.set_options(options), None);

        let reloaded = snapshot(&events, model.reload().unwrap());
        assert!(names(&reloaded).contains(&"later.txt".to_owned()));
    }

    #[test]
    fn navigates_through_history() {
        let dir = tree();
        let (mut model, events) = DirectoryModel::new(Arc::new(RealFs)).unwrap();
        assert_eq!(model.location(), None);
        assert_eq!(model.up(), None);

        snapshot(&events, model.open(dir.path()));
        let inner = snapshot(&events, model.open(dir.path().join("sub")));
        assert_eq!(names(&inner), ["inner.txt"]);
        assert!(model.can_go_back() && !model.can_go_forward());

        let back = snapshot(&events, model.back().unwrap());
        assert_eq!(back.path(), dir.path());
        assert!(model.can_go_forward());
        assert_eq!(
            snapshot(&events, model.forward().unwrap()).path(),
            dir.path().join("sub")
        );
        assert_eq!(model.forward(), None);

        assert_eq!(snapshot(&events, model.up().unwrap()).path(), dir.path());
        assert_eq!(model.location().as_deref(), Some(dir.path()));
        // Opening a new location drops the forward history.
        snapshot(&events, model.back().unwrap());
        snapshot(&events, model.open(dir.path().join("alias")));
        assert!(!model.can_go_forward());
    }

    #[test]
    fn failed_open_keeps_the_previous_location() {
        let dir = tree();
        let (mut model, events) = DirectoryModel::new(Arc::new(RealFs)).unwrap();
        snapshot(&events, model.open(dir.path()));

        let missing = dir.path().join("nope");
        let ModelEvent::Failed { path, error, .. } = settle(&events, model.open(&missing)) else {
            panic!("expected a failure");
        };
        assert_eq!((path, error), (missing, LoadError::NotFound));
        assert_eq!(model.location().as_deref(), Some(dir.path()));
        assert!(!model.can_go_forward());

        let file = dir.path().join("Zeta.md");
        let ModelEvent::Failed { error, .. } = settle(&events, model.open(file)) else {
            panic!("expected a failure");
        };
        assert_eq!(error, LoadError::NotADirectory);
    }

    #[test]
    fn keeps_names_that_are_not_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let raw = b"bad\xff\nname".to_vec();
        fs::write(
            dir.path().join(std::ffi::OsString::from_vec(raw.clone())),
            b"",
        )
        .unwrap();
        let (mut model, events) = DirectoryModel::new(Arc::new(RealFs)).unwrap();
        let shown = snapshot(&events, model.open(dir.path()));

        assert_eq!(shown.name(0), raw);
        assert!(shown.entry_path(0).exists());
        assert_eq!(
            crate::display_name(shown.name(0)),
            "bad\u{fffd}\u{240a}name"
        );
    }

    /// A filesystem with one huge synthetic directory and controllable failures.
    struct FakeFs {
        entries: usize,
    }

    impl Fs for FakeFs {
        fn read_dir(
            &self,
            path: &Path,
        ) -> io::Result<Box<dyn Iterator<Item = io::Result<RawEntry>> + '_>> {
            if path == Path::new("/denied") {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(Box::new((0..self.entries).map(|n| {
                if n % 1000 == 999 {
                    return Err(io::Error::other("unreadable entry"));
                }
                Ok(RawEntry {
                    name: format!("item{n}").into_bytes(),
                    kind: EntryKind::File,
                })
            })))
        }

        fn is_dir(&self, _: &Path) -> bool {
            false
        }
    }

    #[test]
    fn streams_large_listings_and_skips_bad_entries() {
        let entries = CHUNK_LEN * 5 + 123;
        let (mut model, events) = DirectoryModel::new(Arc::new(FakeFs { entries })).unwrap();
        let generation = model.open("/big");

        let mut partial = Vec::new();
        let last = loop {
            match events.recv_timeout(TIMEOUT).unwrap() {
                ModelEvent::Snapshot(s) if s.is_loading() => partial.push(s),
                ModelEvent::Snapshot(s) => break s,
                other => panic!("unexpected {other:?}"),
            }
        };
        assert_eq!(last.generation(), generation);
        assert_eq!(last.len(), entries - entries / 1000);
        assert_eq!(
            partial.iter().map(|s| s.len()).collect::<Vec<_>>(),
            [CHUNK_LEN, CHUNK_LEN * 2, CHUNK_LEN * 4]
        );
        for shown in partial.iter().chain([&last]) {
            let sorted = (1..shown.len())
                .all(|row| crate::natural_cmp(shown.name(row - 1), shown.name(row)).is_lt());
            assert!(sorted, "snapshot of {} rows is sorted", shown.len());
        }
        // An earlier snapshot is unaffected by the listing having grown.
        assert_eq!(partial[0].name(0), b"item0");
    }

    #[test]
    fn permission_errors_are_reported() {
        let (mut model, events) = DirectoryModel::new(Arc::new(FakeFs { entries: 0 })).unwrap();
        let ModelEvent::Failed { error, .. } = settle(&events, model.open("/denied")) else {
            panic!("expected a failure");
        };
        assert_eq!(error, LoadError::PermissionDenied);
        assert_eq!(model.location(), None);
    }

    #[test]
    fn a_newer_request_supersedes_an_older_one() {
        let (mut model, events) = DirectoryModel::new(Arc::new(FakeFs {
            entries: CHUNK_LEN * 8,
        }))
        .unwrap();
        let first = model.open("/first");
        let second = model.open("/second");
        let shown = snapshot(&events, second);
        assert_eq!(shown.path(), Path::new("/second"));
        assert!(second > first);
        // Nothing from the superseded request may arrive after the final snapshot.
        assert!(events.recv_timeout(Duration::from_millis(200)).is_err());
    }
    #[test]
    fn results_of_a_superseded_request_are_not_published() {
        let (mut model, events) = DirectoryModel::new(Arc::new(FakeFs { entries: 3 })).unwrap();
        let stale = model.open("/first");
        snapshot(&events, model.open("/second"));
        while events.try_recv().is_ok() {}

        // A listing that finishes after a newer request was made reaches `publish`
        // with its old generation. Called directly so the check has no race.
        let request = Request {
            path: PathBuf::from("/first"),
            generation: stale,
            options: ViewOptions::default(),
            restore: History::default(),
        };
        model
            .shared
            .publish(&request, &EntryStore::default(), false);

        assert!(matches!(events.try_recv(), Err(flume::TryRecvError::Empty)));
    }
}
