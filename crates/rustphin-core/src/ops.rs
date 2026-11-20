//! The operation engine: the one place where the core changes the filesystem.
//!
//! Rules it keeps (decision D-011):
//!
//! - A copy is written under a temporary name in the destination directory and
//!   renamed into place when complete, so a cancelled, failed or killed copy
//!   never leaves a partial entry under the final name.
//! - A move between filesystems removes a source file only after its copy is in
//!   place and the source is unchanged since it was read.
//! - Nothing is replaced unless the [`Controller`] chose
//!   [`ConflictChoice::Overwrite`] for that entry in this operation.
//! - A request is checked in full before the first change is made.
//!
//! [`run`] is synchronous and blocks until the operation is over; callers run
//! it on a worker thread.

use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    io,
    ops::ControlFlow,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{EntryKind, FileInfo, Fs, archive::ForeignReader};

/// What a file created from a template starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Template {
    /// These bytes: one of the built-in templates.
    Contents(Vec<u8>),
    /// A copy of this file: one of the user's or the system's templates.
    File(PathBuf),
}

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Create the empty folder `name` in `parent`.
    CreateDir {
        /// Existing directory to create in.
        parent: PathBuf,
        /// Name of the new folder.
        name: OsString,
    },
    /// Create the empty file `name` in `parent`.
    CreateFile {
        /// Existing directory to create in.
        parent: PathBuf,
        /// Name of the new file.
        name: OsString,
    },
    /// Create the file `name` in `parent` from `template`, as Dolphin's
    /// "Create New" menu does. Written under a temporary name, then given
    /// its name; nothing is ever replaced.
    CreateFromTemplate {
        /// Existing directory to create in.
        parent: PathBuf,
        /// Name of the new file.
        name: OsString,
        /// What the file starts with.
        template: Template,
    },
    /// Give `path` the name `new_name` within the same directory.
    Rename {
        /// Entry to rename.
        path: PathBuf,
        /// Its new name.
        new_name: OsString,
    },
    /// Rename every pair inside one directory, or change nothing.
    ///
    /// Refused before the first change when a new name is unusable, two files
    /// would receive the same name, or a file that is not part of the plan
    /// already has a new name. A swap or a chain is applied through temporary
    /// names and remembered as one change, so undo puts every file back.
    RenameBatch {
        /// The directory that holds every file.
        directory: PathBuf,
        /// Current name, then the name it should have.
        pairs: Vec<(OsString, OsString)>,
    },
    /// Explicitly restore original names from an interrupted batch in one directory.
    RecoverRenameBatch {
        /// Directory holding the batch recovery record.
        directory: PathBuf,
    },
    /// Copy each of `sources`, with everything inside it, into the directory `into`.
    Copy {
        /// Entries to copy. Symbolic links are copied as links.
        sources: Vec<PathBuf>,
        /// Existing directory to copy into.
        into: PathBuf,
    },
    /// Copy each of `sources` beside itself as "name copy.ext", or the
    /// first free "name copy (n).ext" (D-021, Dolphin's Duplicate).
    Duplicate {
        /// Entries to duplicate, each in its own folder.
        sources: Vec<PathBuf>,
    },
    /// Move each of `sources` into the directory `into`.
    Move {
        /// Entries to move.
        sources: Vec<PathBuf>,
        /// Existing directory to move into.
        into: PathBuf,
    },
    /// Put each of `paths` in the trash, from where it can be restored.
    Trash {
        /// Entries to trash.
        paths: Vec<PathBuf>,
    },
    /// Bring each of `items` back from the trash to where it was.
    Restore {
        /// Entries in the trash.
        items: Vec<TrashedItem>,
    },
    /// Remove each of `paths` for good, with everything inside it. The caller
    /// must have had the user confirm exactly this; nothing can bring the
    /// entries back.
    Delete {
        /// Entries to delete. A symbolic link is removed, not what it points to.
        paths: Vec<PathBuf>,
    },
    /// Unpack `archive` into the directory `into` (W-111, D-032), laid out
    /// as `layout` says; with `trash_archive`, put the archive in the trash
    /// once everything was extracted.
    Extract {
        /// The archive.
        archive: PathBuf,
        /// Existing directory to extract into.
        into: PathBuf,
        /// Where the contents go.
        layout: Layout,
        /// Trash the archive afterwards.
        trash_archive: bool,
    },
    /// Pack each of `sources`, with everything inside it, into the new
    /// archive `archive` (W-111). Nothing is ever replaced.
    Compress {
        /// Entries to pack, each under its own name.
        sources: Vec<PathBuf>,
        /// The archive to make; must not exist.
        archive: PathBuf,
        /// Its format.
        format: crate::archive::Format,
        /// How hard to compress.
        level: crate::archive::Level,
    },
    /// Give `path` these permission bits. Other attributes stay as they are.
    SetMode {
        /// The entry to change.
        path: PathBuf,
        /// Permission bits, including any special bits that should remain.
        mode: u32,
    },
    /// Give `path` to another group (W-170); the owner stays.
    SetGroup {
        /// The entry to change.
        path: PathBuf,
        /// The group's id.
        group: u32,
    },
}

/// An entry that is in the trash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedItem {
    /// Where the entry was before it was trashed.
    pub original: PathBuf,
    /// Where its content is now.
    pub content: PathBuf,
    /// The trash's record of it.
    pub record: PathBuf,
}

/// The desktop's trash. Implemented by the platform layer.
pub trait Trash: Send + Sync {
    /// Moves `path` into the trash. Must not copy across filesystems or
    /// delete anything: when the entry cannot be moved to a trash on its own
    /// filesystem, it fails and the entry stays.
    fn trash(&self, path: &Path) -> io::Result<TrashedItem>;

    /// Moves `item` back to where it was. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when something else is there now.
    fn restore(&self, item: &TrashedItem) -> io::Result<()>;
}

/// A system without a trash: trashing always fails.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTrash;

impl Trash for NoTrash {
    fn trash(&self, _: &Path) -> io::Result<TrashedItem> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no trash is available",
        ))
    }

    fn restore(&self, _: &TrashedItem) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no trash is available",
        ))
    }
}

/// Why a request was refused before anything was changed.
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    /// The name cannot be used for an entry.
    #[error("\"{}\" cannot be used as a name: {reason}", crate::display_name(name.as_bytes()))]
    InvalidName {
        /// The refused name.
        name: OsString,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// Something already has the requested name.
    #[error("\"{}\" already exists", .0.display())]
    AlreadyExists(PathBuf),
    /// A folder was to be copied or moved into itself or a folder inside it.
    #[error("\"{}\" cannot be put inside itself", .0.display())]
    IntoItself(PathBuf),
    /// An entry was to be moved into the folder it is already in.
    #[error("\"{}\" is already in that folder", .0.display())]
    AlreadyThere(PathBuf),
    /// Two of the entries to copy or move have the same name.
    #[error("two of the selected entries are named \"{}\"", crate::display_name(.0.as_bytes()))]
    DuplicateName(OsString),
    /// The destination is not a directory.
    #[error("\"{}\" is not a folder", .0.display())]
    NotADirectory(PathBuf),
    /// Rustphin cannot open or make this kind of archive itself.
    #[error("\"{}\" is not an archive Rustphin can open", .0.display())]
    NotAnArchive(PathBuf),
    /// The archive is damaged or not what it claims to be.
    #[error("\"{}\" is damaged: {reason}", path.display())]
    Damaged {
        /// The archive.
        path: PathBuf,
        /// What the reader found.
        reason: String,
    },
    /// The password was wrong every time it was asked for.
    #[error("the password for \"{}\" was not right", .0.display())]
    WrongPassword(PathBuf),
    /// The archive declares more than the destination has free.
    #[error("not enough free space in \"{}\": {needed} bytes are needed and {free} are free", path.display())]
    NotEnoughSpace {
        /// The destination.
        path: PathBuf,
        /// Bytes the archive holds.
        needed: u64,
        /// Bytes free.
        free: u64,
    },
    /// The filesystem refused.
    #[error("\"{}\": {source}", path.display())]
    Io {
        /// What was being accessed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

/// An entry to copy or move would land on an existing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The entry being copied or moved.
    pub source: PathBuf,
    /// The existing entry with the same name in the destination.
    pub target: PathBuf,
    /// The entry being copied or moved, as last read.
    pub source_info: FileInfo,
    /// The existing entry, as last read.
    pub target_info: FileInfo,
    /// Where [`ConflictChoice::KeepBoth`] would put the entry: the first
    /// free `name (1).ext`, `name (2).ext`… beside the existing one.
    pub keep_both: PathBuf,
    /// Whether [`ConflictChoice::Overwrite`] is possible: file over file
    /// replaces it, folder into folder merges the contents. A file cannot
    /// replace a folder nor a folder a file.
    pub can_overwrite: bool,
}

/// How to settle a [`Conflict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    /// Leave the existing entry; do not copy or move this one.
    Skip,
    /// Replace the existing file, or merge into the existing folder.
    Overwrite,
    /// Keep the existing entry and give the new one a free name.
    KeepBoth,
    /// Stop the whole operation. What was done so far stays.
    Cancel,
}

/// A [`Controller`]'s answer to a [`Conflict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// What to do with this entry.
    pub choice: ConflictChoice,
    /// Use the same choice for later conflicts in this operation without asking.
    pub apply_to_all: bool,
}

/// One entry could not be processed.
#[derive(Debug)]
pub struct Failure {
    /// The entry that failed.
    pub path: PathBuf,
    /// Why.
    pub error: io::Error,
}

/// A [`Controller`]'s answer to a [`Failure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// Leave this entry out and continue.
    Skip,
    /// Try the same entry again.
    Retry,
    /// Stop the whole operation. What was done so far stays.
    Cancel,
}

/// How far an operation is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// Entries finished, counting files, links and folders.
    pub items_done: u64,
    /// Entries known to be part of the operation.
    pub items_total: u64,
    /// Bytes of file content copied.
    pub bytes_done: u64,
    /// Bytes of file content known to need copying.
    pub bytes_total: u64,
    /// The entry being worked on.
    pub current: PathBuf,
}

/// The side of an operation that decides and observes: the user interface, or
/// a script in tests. Called on the thread that runs the operation.
pub trait Controller {
    /// Called after every finished entry and every copy step.
    fn progress(&mut self, progress: &Progress);
    /// Whether the operation should stop at the next safe point.
    fn cancelled(&self) -> bool;
    /// Decides what happens to an entry whose name is taken.
    fn conflict(&mut self, conflict: &Conflict) -> Resolution;
    /// Decides what happens after an entry failed.
    fn failure(&mut self, failure: &Failure) -> Recovery;
    /// Asks for the password of the encrypted `archive`; `wrong` when the
    /// one given before was not right. `None` stops the operation. Without
    /// an answer, encrypted archives are not opened.
    fn password(&mut self, _archive: &Path, _wrong: bool) -> Option<String> {
        None
    }
}

/// One file in a [`Change::BatchRenamed`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRename {
    /// Where the file was.
    pub from: PathBuf,
    /// Where the file is.
    pub to: PathBuf,
    /// The file at `to` right after the rename.
    pub info: FileInfo,
}

/// One change an operation made, in the order they happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A new empty folder or file.
    Created {
        /// The new entry.
        path: PathBuf,
        /// The new entry right after creation.
        info: FileInfo,
    },
    /// An entry got a new name or place.
    Moved {
        /// Where it was.
        from: PathBuf,
        /// Where it is now.
        to: PathBuf,
        /// The entry at `to` right after the move.
        info: FileInfo,
    },
    /// Several renames that were applied together, including swaps.
    BatchRenamed {
        /// Each file's old path, its path now, and the entry there.
        items: Vec<BatchRename>,
    },
    /// A new copy.
    Copied {
        /// The copy.
        to: PathBuf,
        /// The copy right after it was put in place.
        info: FileInfo,
    },
    /// An existing file was replaced by explicit choice. Its previous content
    /// is gone, so the operation cannot be undone past this point.
    Replaced {
        /// The replaced file.
        path: PathBuf,
    },
    /// A move between filesystems that left part of the source behind: some
    /// entries are at `to`, the rest still at `from`. It cannot be undone
    /// automatically.
    PartlyMoved {
        /// The source, which still holds what could not be moved.
        from: PathBuf,
        /// The destination, which holds what was moved.
        to: PathBuf,
    },
    /// An entry was put in the trash.
    Trashed {
        /// Where it went.
        item: TrashedItem,
    },
    /// An entry was brought back from the trash.
    Restored {
        /// Where it is again.
        original: PathBuf,
    },
    /// An entry was removed for good by explicit choice. The operation cannot
    /// be undone past this point.
    Deleted {
        /// The removed entry.
        path: PathBuf,
    },
    /// Permission bits were changed.
    ModeSet {
        /// The entry.
        path: PathBuf,
        /// The bits before the change, so it can be undone.
        previous: u32,
        /// The entry right after the change.
        info: FileInfo,
    },
    /// The owning group was changed.
    GroupSet {
        /// The entry.
        path: PathBuf,
        /// The group before the change, so it can be undone.
        previous: u32,
        /// The entry right after the change.
        info: FileInfo,
    },
    /// A folder that was empty after its contents were merged into another
    /// folder by a move, and was removed.
    RemovedEmptyDir {
        /// The removed folder.
        path: PathBuf,
        /// Its permission bits.
        mode: u32,
    },
}

impl Change {
    /// Whether this change can be reverted. Replacing a file, deleting for
    /// good and a move that stopped half-way cannot.
    pub fn is_reversible(&self) -> bool {
        !matches!(
            self,
            Self::Replaced { .. } | Self::Deleted { .. } | Self::PartlyMoved { .. }
        )
    }
}

/// An entry that was left out, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The entry.
    pub path: PathBuf,
    /// A short reason for the user.
    pub reason: String,
}

/// How an operation ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// False when the operation was cancelled before it finished.
    pub completed: bool,
    /// Every change made, in order. Present even when cancelled.
    pub changes: Vec<Change>,
    /// Entries left out by choice or after a failure.
    pub skipped: Vec<Skipped>,
}

/// Runs `request` to its end. Returns `Err` when the request is refused
/// before any change; afterwards problems with single entries go to the
/// controller and the result is an [`Outcome`].
pub fn run(
    fs: &dyn Fs,
    trash: &dyn Trash,
    foreign: &dyn ForeignReader,
    request: &Request,
    controller: &mut dyn Controller,
) -> Result<Outcome, OpError> {
    let mut engine = Engine {
        fs,
        trash,
        foreign,
        controller,
        changes: Vec::new(),
        skipped: Vec::new(),
        progress: Progress::default(),
        for_all: None,
        left_out: HashSet::new(),
    };
    let completed = match request {
        Request::CreateDir { parent, name } => {
            engine.create(parent, name, EntryKind::Directory)?;
            true
        }
        Request::CreateFile { parent, name } => {
            engine.create(parent, name, EntryKind::File)?;
            true
        }
        Request::CreateFromTemplate {
            parent,
            name,
            template,
        } => {
            engine.create_from_template(parent, name, template)?;
            true
        }
        Request::Rename { path, new_name } => {
            engine.rename(path, new_name)?;
            true
        }
        Request::RenameBatch { directory, pairs } => engine.rename_batch(directory, pairs)?,
        Request::RecoverRenameBatch { directory } => match recover_rename_batch(fs, directory) {
            Ok(count) => {
                engine.progress.items_total = count as u64;
                engine.progress.items_done = count as u64;
                engine.controller.progress(&engine.progress);
                true
            }
            Err(error) => {
                engine.skipped.push(Skipped {
                    path: directory.clone(),
                    reason: format!("Recovery could not finish: {error}. Nothing was replaced. Keep any recovery record and temporary entries, resolve the problem, then retry recovery."),
                });
                false
            }
        },
        Request::Copy { sources, into } => {
            let plan = engine.plan_transfer(sources, into, false)?;
            engine.controller.progress(&engine.progress);
            engine.transfer(plan, into, false).is_ok()
        }
        Request::Duplicate { sources } => {
            let mut plans = Vec::new();
            for source in sources {
                let parent = source
                    .parent()
                    .ok_or_else(|| OpError::NotADirectory(source.clone()))?;
                plans.push(engine.plan_transfer(std::slice::from_ref(source), parent, false)?);
            }
            engine.controller.progress(&engine.progress);
            engine.duplicate(plans.into_iter().flatten()).is_ok()
        }
        Request::Move { sources, into } => {
            let plan = engine.plan_transfer(sources, into, true)?;
            engine.controller.progress(&engine.progress);
            engine.transfer(plan, into, true).is_ok()
        }
        Request::Trash { paths } => {
            let plan = engine.plan_removal(paths, false)?;
            engine.controller.progress(&engine.progress);
            engine.trash_all(plan).is_ok()
        }
        Request::Restore { items } => {
            engine.progress.items_total = items.len() as u64;
            engine.controller.progress(&engine.progress);
            engine.restore_all(items).is_ok()
        }
        Request::Delete { paths } => {
            let plan = engine.plan_removal(paths, true)?;
            engine.controller.progress(&engine.progress);
            engine.delete_all(plan).is_ok()
        }
        Request::SetMode { path, mode } => {
            engine.set_mode(path, *mode)?;
            true
        }
        Request::SetGroup { path, group } => {
            engine.set_group(path, *group)?;
            true
        }
        Request::Extract {
            archive,
            into,
            layout,
            trash_archive,
        } => engine.extract(archive, into, *layout, *trash_archive)?,
        Request::Compress {
            sources,
            archive,
            format,
            level,
        } => engine.compress(sources, archive, *format, *level)?,
    };
    Ok(Outcome {
        completed,
        changes: engine.changes,
        skipped: engine.skipped,
    })
}

/// Checks that `name` can name a single entry.
pub fn validate_name(name: &OsStr) -> Result<(), OpError> {
    let bytes = name.as_bytes();
    let reason = if bytes.is_empty() {
        "a name cannot be empty"
    } else if bytes == b"." || bytes == b".." {
        "this name is reserved"
    } else if bytes.contains(&b'/') {
        "a name cannot contain \"/\""
    } else if bytes.contains(&0) {
        "a name cannot contain a null character"
    } else if bytes.len() > 255 {
        "a name cannot be longer than 255 bytes"
    } else {
        return Ok(());
    };
    Err(OpError::InvalidName {
        name: name.to_owned(),
        reason,
    })
}

/// Prefix of temporary operation entries. A prefix alone never authorises
/// cleanup: batch-rename entries can hold the only copy of an original file.
pub const TEMP_PREFIX: &str = ".rustphin-part-";

/// Recovery record in the directory whose entries a batch rename changes.
pub const RENAME_JOURNAL: &str = ".rustphin-rename-journal";

/// Restores an interrupted batch's original names after validating its record.
/// With no journal, syncs the directory without moving entries.
/// The caller must explicitly request this operation.
/// Changed or missing entries and occupied original names refuse recovery.
/// An I/O error during recovery can leave some entries staged; the record is
/// kept so recovery can be retried. If only the final cleanup sync fails, the
/// original names are restored and the record is already removed; retrying
/// syncs the directory. No existing entry is replaced.
pub fn recover_rename_batch(fs: &dyn Fs, directory: &Path) -> Result<usize, OpError> {
    batch::recover(fs, directory)
}

/// Whether `directory` holds the record of an unfinished batch rename, which
/// [`recover_rename_batch`] can restore (D-020: checked at startup).
pub fn has_rename_journal(fs: &dyn Fs, directory: &Path) -> bool {
    fs.info(&directory.join(RENAME_JOURNAL)).is_ok()
}

/// The operation was cancelled; unwind to the top.
struct Stop;

/// An entry to transfer, read before the first change.
struct Node {
    path: PathBuf,
    info: FileInfo,
    /// Contents of a directory.
    children: Vec<Node>,
    /// Why the contents of a directory could not be listed.
    unreadable: Option<io::ErrorKind>,
}

impl Node {
    fn count(&self) -> (u64, u64) {
        let own_bytes = if self.info.kind == EntryKind::File {
            self.info.size
        } else {
            0
        };
        self.children
            .iter()
            .map(Node::count)
            .fold((1, own_bytes), |(items, bytes), (i, b)| {
                (items + i, bytes + b)
            })
    }
}

struct Engine<'a> {
    fs: &'a dyn Fs,
    trash: &'a dyn Trash,
    /// Reads the archive formats Rustphin does not read itself (D-036).
    foreign: &'a dyn ForeignReader,
    controller: &'a mut dyn Controller,
    changes: Vec<Change>,
    skipped: Vec<Skipped>,
    progress: Progress,
    /// The conflict choice to reuse without asking.
    for_all: Option<ConflictChoice>,
    /// Source entries that were not transferred and so must stay.
    left_out: HashSet<PathBuf>,
}

fn io_error(path: &Path) -> impl FnOnce(io::Error) -> OpError + '_ {
    move |source| OpError::Io {
        path: path.to_owned(),
        source,
    }
}

fn file_name(path: &Path) -> Result<&OsStr, OpError> {
    path.file_name().ok_or_else(|| OpError::InvalidName {
        name: path.as_os_str().to_owned(),
        reason: "it has no name of its own",
    })
}

/// A name for a temporary entry, different on every call in this process.
fn temp_name() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{TEMP_PREFIX}{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// The name of a duplicate, as Dolphin gives it: " copy" before the
/// extension, keeping a compound `.tar.*` extension whole. `a.txt` becomes
/// `a copy.txt`, `b.tar.gz` `b copy.tar.gz`, `.bashrc` and `notes` gain
/// " copy" at the end.
pub fn duplicate_name(name: &OsStr) -> OsString {
    with_suffix(name.as_bytes(), b" copy")
}

/// The name a duplicate of `name` is given beside it: [`duplicate_name`],
/// or when `is_taken` says that exists, the first free `a copy (1).txt`,
/// `a copy (2).txt`… The operation engine names duplicates with this, so a
/// view can tell which entries a duplicate will create.
pub fn free_duplicate_name(name: &OsStr, is_taken: impl Fn(&OsStr) -> bool) -> OsString {
    let first = duplicate_name(name);
    if !is_taken(&first) {
        return first;
    }
    (1..=u32::MAX)
        .map(|n| with_suffix(name.as_bytes(), format!(" copy ({n})").as_bytes()))
        .find(|candidate| !is_taken(candidate))
        .unwrap_or(first)
}

/// `name` with `suffix` inserted before its extension; a compound `.tar.*`
/// extension stays whole and a leading dot is not an extension.
fn with_suffix(bytes: &[u8], suffix: &[u8]) -> OsString {
    let split = crate::rename::extension_start(bytes);
    let mut out = bytes[..split].to_vec();
    out.extend_from_slice(suffix);
    out.extend_from_slice(&bytes[split..]);
    OsString::from_vec(out)
}

/// `name` with ` (n)` before its extension: `a.txt` becomes `a (1).txt`.
fn numbered(name: &OsStr, n: u32) -> OsString {
    let bytes = name.as_bytes();
    let split = match bytes.iter().rposition(|&byte| byte == b'.') {
        Some(dot) if dot > 0 => dot,
        _ => bytes.len(),
    };
    let mut out = bytes[..split].to_vec();
    out.extend_from_slice(format!(" ({n})").as_bytes());
    out.extend_from_slice(&bytes[split..]);
    OsString::from_vec(out)
}

impl Engine<'_> {
    fn create(&mut self, parent: &Path, name: &OsStr, kind: EntryKind) -> Result<(), OpError> {
        validate_name(name)?;
        let path = parent.join(name);
        let created = if kind == EntryKind::Directory {
            self.fs.create_dir(&path, 0o777)
        } else {
            self.fs.create_file(&path)
        };
        match created {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(OpError::AlreadyExists(path));
            }
            Err(error) => return Err(io_error(&path)(error)),
        }
        let info = self.fs.info(&path).map_err(io_error(&path))?;
        self.changes.push(Change::Created { path, info });
        Ok(())
    }

    /// Writes the template under a temporary name, then gives it `name`
    /// without replacing anything; a failure leaves nothing behind.
    fn create_from_template(
        &mut self,
        parent: &Path,
        name: &OsStr,
        template: &Template,
    ) -> Result<(), OpError> {
        validate_name(name)?;
        let path = parent.join(name);
        if self.fs.info(&path).is_ok() {
            return Err(OpError::AlreadyExists(path));
        }
        let temp = parent.join(temp_name());
        let written = match template {
            Template::Contents(content) => self.fs.create_file_with(&temp, content),
            // A copy is published only when it is whole, as other copies.
            Template::File(source) => self
                .fs
                .copy_file(source, &temp, &mut |_| ControlFlow::Continue(()))
                .and_then(|()| {
                    let (from, to) = (self.fs.info(source)?, self.fs.info(&temp)?);
                    if from.size == to.size {
                        Ok(())
                    } else {
                        Err(io::Error::other("the template changed while it was copied"))
                    }
                }),
        };
        if let Err(error) = written {
            let _ = self.fs.remove_file(&temp);
            return Err(match template {
                Template::File(source) if error.kind() == io::ErrorKind::NotFound => {
                    io_error(source)(error)
                }
                _ => io_error(&path)(error),
            });
        }
        match self.fs.rename_new(&temp, &path) {
            Ok(()) => {}
            Err(error) => {
                let _ = self.fs.remove_file(&temp);
                return Err(if error.kind() == io::ErrorKind::AlreadyExists {
                    OpError::AlreadyExists(path)
                } else {
                    io_error(&path)(error)
                });
            }
        }
        let info = self.fs.info(&path).map_err(io_error(&path))?;
        self.changes.push(Change::Created { path, info });
        Ok(())
    }

    /// Sets permission bits. A mode that is already in place changes nothing.
    fn set_mode(&mut self, path: &Path, mode: u32) -> Result<(), OpError> {
        let mode = mode & 0o7777;
        let before = self.fs.info(path).map_err(io_error(path))?;
        if before.mode == mode {
            return Ok(());
        }
        self.fs.set_mode(path, mode).map_err(io_error(path))?;
        let info = self.fs.info(path).unwrap_or(FileInfo { mode, ..before });
        self.changes.push(Change::ModeSet {
            path: path.to_owned(),
            previous: before.mode,
            info,
        });
        Ok(())
    }

    /// Gives `path` to group `group`. The group it already has changes nothing.
    fn set_group(&mut self, path: &Path, group: u32) -> Result<(), OpError> {
        let previous = self.fs.group(path).map_err(io_error(path))?;
        if previous == group {
            return Ok(());
        }
        let before = self.fs.info(path).map_err(io_error(path))?;
        self.fs.set_group(path, group).map_err(io_error(path))?;
        let info = self.fs.info(path).unwrap_or(before);
        self.changes.push(Change::GroupSet {
            path: path.to_owned(),
            previous,
            info,
        });
        Ok(())
    }

    fn rename(&mut self, path: &Path, new_name: &OsStr) -> Result<(), OpError> {
        validate_name(new_name)?;
        let to = path.with_file_name(new_name);
        if to == path {
            return Ok(());
        }
        match self.fs.rename_new(path, &to) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(OpError::AlreadyExists(to));
            }
            Err(error) => return Err(io_error(path)(error)),
        }
        let info = self.fs.info(&to).map_err(io_error(&to))?;
        self.changes.push(Change::Moved {
            from: path.to_owned(),
            to,
            info,
        });
        Ok(())
    }

    /// Reads `path` and, for a directory, everything inside it.
    fn scan(&self, path: PathBuf) -> io::Result<Node> {
        let info = self.fs.info(&path)?;
        let mut node = Node {
            path,
            info,
            children: Vec::new(),
            unreadable: None,
        };
        if info.kind != EntryKind::Directory {
            return Ok(node);
        }
        let listed = self.fs.read_dir(&node.path).and_then(|entries| {
            entries
                .map(|entry| self.scan(node.path.join(OsStr::from_bytes(&entry?.name))))
                .collect::<io::Result<Vec<_>>>()
        });
        match listed {
            Ok(children) => node.children = children,
            Err(error) => node.unreadable = Some(error.kind()),
        }
        Ok(node)
    }

    /// Checks a copy or move in full and reads what a copy has to transfer.
    fn plan_transfer(
        &mut self,
        sources: &[PathBuf],
        into: &Path,
        moving: bool,
    ) -> Result<Vec<Node>, OpError> {
        let into_info = self.fs.info(into).map_err(io_error(into))?;
        // A link to a directory is a fine destination; what matters is where it leads.
        if into_info.kind != EntryKind::Directory && !self.fs.is_dir(into) {
            return Err(OpError::NotADirectory(into.to_owned()));
        }
        let mut names: Vec<&OsStr> = Vec::new();
        let mut plan = Vec::new();
        for source in sources {
            let name = file_name(source)?;
            if names.contains(&name) {
                return Err(OpError::DuplicateName(name.to_owned()));
            }
            names.push(name);
            if into.starts_with(source) {
                return Err(OpError::IntoItself(source.clone()));
            }
            if moving && source.parent() == Some(into) {
                return Err(OpError::AlreadyThere(source.clone()));
            }
            let node = if moving {
                // A move within a filesystem is one rename; the contents are
                // only read if it turns out to cross filesystems.
                Node {
                    info: self.fs.info(source).map_err(io_error(source))?,
                    path: source.clone(),
                    children: Vec::new(),
                    unreadable: None,
                }
            } else {
                self.scan(source.clone()).map_err(io_error(source))?
            };
            // A rename moves no bytes; they are counted if the move turns out
            // to need a copy.
            let (items, bytes) = if moving { (1, 0) } else { node.count() };
            self.progress.items_total += items;
            self.progress.bytes_total += bytes;
            plan.push(node);
        }
        Ok(plan)
    }

    /// Checks a trash or delete request and reads what a delete has to remove.
    fn plan_removal(&mut self, paths: &[PathBuf], deep: bool) -> Result<Vec<Node>, OpError> {
        let mut plan = Vec::new();
        for path in paths {
            file_name(path)?;
            if paths
                .iter()
                .any(|other| other != path && path.starts_with(other))
            {
                // Already covered by its folder, which is also in the request.
                continue;
            }
            let node = if deep {
                self.scan(path.clone()).map_err(io_error(path))?
            } else {
                Node {
                    info: self.fs.info(path).map_err(io_error(path))?,
                    path: path.clone(),
                    children: Vec::new(),
                    unreadable: None,
                }
            };
            self.progress.items_total += node.count().0;
            plan.push(node);
        }
        Ok(plan)
    }

    fn trash_all(&mut self, plan: Vec<Node>) -> Result<(), Stop> {
        let trash = self.trash;
        for node in plan {
            self.checkpoint(&node.path)?;
            match self.attempt(&node.path, |_| Ok(trash.trash(&node.path)))? {
                Ok(item) => {
                    self.changes.push(Change::Trashed { item });
                    self.finished(1);
                }
                Err(reason) => self.skip(&node, reason),
            }
        }
        Ok(())
    }

    fn restore_all(&mut self, items: &[TrashedItem]) -> Result<(), Stop> {
        let trash = self.trash;
        for item in items {
            self.checkpoint(&item.original)?;
            match self.attempt(&item.original, |_| Ok(trash.restore(item)))? {
                Ok(()) => self.changes.push(Change::Restored {
                    original: item.original.clone(),
                }),
                Err(reason) => self.skipped.push(Skipped {
                    path: item.original.clone(),
                    reason,
                }),
            }
            self.finished(1);
        }
        Ok(())
    }

    fn delete_all(&mut self, plan: Vec<Node>) -> Result<(), Stop> {
        for node in plan {
            if self.delete_tree(&node)? {
                self.changes.push(Change::Deleted {
                    path: node.path.clone(),
                });
            }
        }
        Ok(())
    }

    /// Removes `node` and everything inside it; says whether all of it went.
    fn delete_tree(&mut self, node: &Node) -> Result<bool, Stop> {
        self.checkpoint(&node.path)?;
        let fs = self.fs;
        let is_dir = node.info.kind == EntryKind::Directory;
        let mut emptied = true;
        for child in &node.children {
            emptied &= self.delete_tree(child)?;
        }
        if !emptied {
            // It still holds something that was kept, so it stays too; that
            // is not a second problem to report.
            self.finished(1);
            return Ok(false);
        }
        let removed = self.attempt(&node.path, |_| {
            Ok(match node.unreadable {
                Some(kind) => Err(kind.into()),
                None if is_dir => fs.remove_dir(&node.path),
                None => fs.remove_file(&node.path),
            })
        })?;
        if let Err(reason) = &removed {
            self.skipped.push(Skipped {
                path: node.path.clone(),
                reason: reason.clone(),
            });
        }
        self.finished(1);
        Ok(removed.is_ok())
    }

    /// Copies each planned entry beside itself under its duplicate name,
    /// or the first free numbered form of it.
    fn duplicate(&mut self, plan: impl Iterator<Item = Node>) -> Result<(), Stop> {
        for node in plan {
            let (Some(parent), Some(name)) = (node.path.parent(), node.path.file_name()) else {
                continue;
            };
            // Anything but "not found" counts as taken: never risk a clash.
            let target = parent.join(free_duplicate_name(name, |candidate| {
                !matches!(
                    self.fs.info(&parent.join(candidate)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound
                )
            }));
            self.place(node, target, false)?;
        }
        Ok(())
    }

    fn transfer(&mut self, plan: Vec<Node>, into: &Path, moving: bool) -> Result<(), Stop> {
        for node in plan {
            let Some(name) = node.path.file_name().map(OsStr::to_owned) else {
                continue;
            };
            self.place(node, into.join(name), moving)?;
        }
        Ok(())
    }

    fn checkpoint(&mut self, current: &Path) -> Result<(), Stop> {
        if self.controller.cancelled() {
            return Err(Stop);
        }
        self.progress.current = current.to_owned();
        Ok(())
    }

    fn finished(&mut self, items: u64) {
        self.progress.items_done += items;
        self.controller.progress(&self.progress);
    }

    fn skip(&mut self, node: &Node, reason: impl Into<String>) {
        self.skipped.push(Skipped {
            path: node.path.clone(),
            reason: reason.into(),
        });
        let (items, bytes) = node.count();
        // Never past the total: a moved entry's bytes are only counted once
        // the move turns out to need a copy.
        self.progress.bytes_done =
            (self.progress.bytes_done + bytes).min(self.progress.bytes_total);
        self.finished(items);
    }

    /// Asks what to do about `error` on `path`. `None` means try again;
    /// otherwise the entry is to be left out, for the returned reason.
    fn ask(&mut self, path: &Path, error: io::Error) -> Result<Option<String>, Stop> {
        let failure = Failure {
            path: path.to_owned(),
            error,
        };
        match self.controller.failure(&failure) {
            Recovery::Retry => Ok(None),
            Recovery::Skip => Ok(Some(failure.error.to_string())),
            Recovery::Cancel => Err(Stop),
        }
    }

    /// Runs `action` until it succeeds or the controller gives up on it;
    /// `Err` carries the reason the entry is left out.
    fn attempt<T>(
        &mut self,
        path: &Path,
        mut action: impl FnMut(&mut Self) -> Result<io::Result<T>, Stop>,
    ) -> Result<Result<T, String>, Stop> {
        loop {
            match action(self)? {
                Ok(value) => return Ok(Ok(value)),
                Err(error) => {
                    if let Some(reason) = self.ask(path, error)? {
                        return Ok(Err(reason));
                    }
                }
            }
        }
    }

    /// Puts `node` at `target` in an existing directory, settling a name
    /// conflict first.
    fn place(&mut self, node: Node, mut target: PathBuf, moving: bool) -> Result<(), Stop> {
        self.checkpoint(&node.path)?;
        let mut replace = false;
        loop {
            let target_info = match self.fs.info(&target) {
                Ok(info) => info,
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => {
                    self.skip(&node, error.to_string());
                    return Ok(());
                }
            };
            // A copy next to its source needs no question: there is nothing to
            // replace but the source itself.
            if target == node.path {
                target = self.free_name(&target);
                continue;
            }
            let both_dirs =
                node.info.kind == EntryKind::Directory && target_info.kind == EntryKind::Directory;
            let can_overwrite = both_dirs
                || (node.info.kind != EntryKind::Directory
                    && target_info.kind != EntryKind::Directory);
            let remembered = match self.for_all {
                Some(ConflictChoice::Overwrite) if !can_overwrite => None,
                choice => choice,
            };
            let choice = match remembered {
                Some(choice) => choice,
                None => {
                    let resolution = self.controller.conflict(&Conflict {
                        source: node.path.clone(),
                        target: target.clone(),
                        source_info: node.info,
                        target_info,
                        keep_both: self.free_name(&target),
                        can_overwrite,
                    });
                    if resolution.apply_to_all {
                        self.for_all = Some(resolution.choice);
                    }
                    resolution.choice
                }
            };
            match choice {
                ConflictChoice::Cancel => return Err(Stop),
                ConflictChoice::Skip => {
                    self.skip(&node, "an entry with this name already exists");
                    return Ok(());
                }
                ConflictChoice::KeepBoth => target = self.free_name(&target),
                ConflictChoice::Overwrite if both_dirs => {
                    return self.merge(node, &target, moving);
                }
                ConflictChoice::Overwrite if can_overwrite => {
                    replace = true;
                    break;
                }
                // Not possible for these two entries: forget it and ask again.
                ConflictChoice::Overwrite => self.for_all = None,
            }
        }

        loop {
            let placed = if moving {
                self.move_entry(&node, &target, replace)?
            } else {
                self.copy_entry(&node, &target, replace)?
            };
            match placed {
                Ok(Placed::Done | Placed::LeftOut) => return Ok(()),
                // Something took the name in the meantime: settle it like any conflict.
                Ok(Placed::NameTaken) => return self.place(node, target, moving),
                Err(error) => {
                    if let Some(reason) = self.ask(&node.path, error)? {
                        self.skip(&node, reason);
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Moves or copies the contents of the directory `node` into the existing
    /// directory `target`.
    fn merge(&mut self, mut node: Node, target: &Path, moving: bool) -> Result<(), Stop> {
        if moving {
            // The contents were not read at planning time.
            match self.scan(node.path.clone()) {
                Ok(scanned) => {
                    let (items, _) = scanned.count();
                    self.progress.items_total += items - 1;
                    node = scanned;
                }
                Err(error) => {
                    self.skip(&node, error.to_string());
                    return Ok(());
                }
            }
        }
        if let Some(kind) = node.unreadable {
            self.skip(&node, io::Error::from(kind).to_string());
            return Ok(());
        }
        let children = std::mem::take(&mut node.children);
        for child in children {
            let Some(name) = child.path.file_name().map(OsStr::to_owned) else {
                continue;
            };
            self.place(child, target.join(name), moving)?;
        }
        // The source folder goes only if everything inside it was moved;
        // removing a folder that still has contents fails and it stays.
        if moving && self.fs.remove_dir(&node.path).is_ok() {
            self.changes.push(Change::RemovedEmptyDir {
                path: node.path.clone(),
                mode: node.info.mode,
            });
        }
        self.finished(1);
        Ok(())
    }

    /// The first of `name (1)`, `name (2)`, … that is free next to `taken`.
    fn free_name(&self, taken: &Path) -> PathBuf {
        let name = taken.file_name().unwrap_or_default();
        (1..)
            .map(|n| taken.with_file_name(numbered(name, n)))
            .find(|candidate| {
                matches!(self.fs.info(candidate), Err(error) if error.kind() == io::ErrorKind::NotFound)
            })
            .unwrap_or_else(|| taken.to_owned())
    }

    /// Copies `node` to `target` through a temporary name beside it.
    fn copy_entry(
        &mut self,
        node: &Node,
        target: &Path,
        replace: bool,
    ) -> Result<io::Result<Placed>, Stop> {
        let temp = target.with_file_name(temp_name());
        let mut created = Vec::new();
        let before = (self.progress.items_done, self.progress.bytes_done);
        if let Err(Stop) = self.copy_tree(node, &temp, &mut created) {
            self.discard(created);
            return Err(Stop);
        }
        if created.is_empty() {
            // The entry itself was left out; that is already recorded.
            return Ok(Ok(Placed::LeftOut));
        }
        let published = if replace {
            self.fs.rename_over(&temp, target).map(|()| Placed::Done)
        } else {
            match self.fs.rename_new(&temp, target) {
                Ok(()) => Ok(Placed::Done),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(Placed::NameTaken),
                Err(error) => Err(error),
            }
        };
        match published {
            Ok(Placed::Done) => {
                if replace {
                    self.changes.push(Change::Replaced {
                        path: target.to_owned(),
                    });
                } else if let Ok(info) = self.fs.info(target) {
                    self.changes.push(Change::Copied {
                        to: target.to_owned(),
                        info,
                    });
                }
            }
            _ => {
                // Not in place: remove the temporary copy and forget its progress.
                self.discard(created);
                (self.progress.items_done, self.progress.bytes_done) = before;
            }
        }
        Ok(published)
    }

    /// Removes temporary entries this operation created, newest first.
    fn discard(&mut self, created: Vec<(PathBuf, bool)>) {
        for (path, is_dir) in created.into_iter().rev() {
            let removed = if is_dir {
                self.fs.remove_dir(&path)
            } else {
                self.fs.remove_file(&path)
            };
            if let Err(error) = removed {
                tracing::warn!(path = %path.display(), %error, "temporary entry left behind");
            }
        }
    }

    /// Leaves `node` out of a copy after a failure.
    fn leave_out(&mut self, node: &Node, reason: String) {
        self.left_out.insert(node.path.clone());
        self.skip(node, reason);
    }

    /// Copies the content of the file `node` to the new file `to`. After a
    /// failure nothing is left at `to`.
    fn copy_file(&mut self, node: &Node, to: &Path) -> Result<io::Result<()>, Stop> {
        let fs = self.fs;
        let bytes_before = self.progress.bytes_done;
        let mut stopped = false;
        let copied = fs
            .copy_file(&node.path, to, &mut |bytes| {
                self.progress.bytes_done += bytes;
                self.controller.progress(&self.progress);
                if self.controller.cancelled() {
                    stopped = true;
                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            })
            .and_then(|()| {
                // The copy must be as long as the source was when it was read.
                let size = fs.info(to)?.size;
                if size == node.info.size {
                    Ok(())
                } else {
                    Err(io::Error::other(format!(
                        "the copy has {size} bytes but the original had {}; it changed while being copied",
                        node.info.size
                    )))
                }
            });
        if copied.is_err() || stopped {
            // Our own partial file. When nothing was created, nothing is removed:
            // the name is a fresh temporary one.
            if let Err(error) = fs.remove_file(to)
                && error.kind() != io::ErrorKind::NotFound
            {
                tracing::warn!(path = %to.display(), %error, "partial copy left behind");
            }
            self.progress.bytes_done = bytes_before;
        }
        if stopped {
            return Err(Stop);
        }
        Ok(copied)
    }

    /// Copies `node` and everything inside it to the new path `to`, recording
    /// each entry it creates. Entries that fail are offered to the controller
    /// and may be left out; the rest is still copied.
    fn copy_tree(
        &mut self,
        node: &Node,
        to: &Path,
        created: &mut Vec<(PathBuf, bool)>,
    ) -> Result<(), Stop> {
        self.checkpoint(&node.path)?;
        let fs = self.fs;
        let made = match node.info.kind {
            EntryKind::File => self.attempt(&node.path, |engine| engine.copy_file(node, to))?,
            EntryKind::Symlink => self.attempt(&node.path, |_| {
                Ok(fs
                    .read_link(&node.path)
                    .and_then(|link| fs.symlink(&link, to)))
            })?,
            EntryKind::Directory => {
                // Writable by us while it is filled; its own mode comes last.
                self.attempt(&node.path, |_| {
                    Ok(match node.unreadable {
                        Some(kind) => Err(kind.into()),
                        None => fs.create_dir(to, 0o700),
                    })
                })?
            }
            EntryKind::Other => Err("devices, pipes and sockets cannot be copied".to_owned()),
        };
        if let Err(reason) = made {
            self.leave_out(node, reason);
            return Ok(());
        }
        let is_dir = node.info.kind == EntryKind::Directory;
        created.push((to.to_owned(), is_dir));
        if is_dir {
            for child in &node.children {
                if let Some(name) = child.path.file_name() {
                    self.copy_tree(child, &to.join(name), created)?;
                }
            }
            if let Err(reason) = self.attempt(to, |_| Ok(fs.set_attributes(to, &node.info)))? {
                // A directory whose entries/attributes could not be synced must
                // never authorise deletion of any part of its source tree.
                self.left_out.insert(node.path.clone());
                self.skipped.push(Skipped {
                    path: node.path.clone(),
                    reason: format!(
                        "folder copy could not be synced; its original tree was kept: {reason}"
                    ),
                });
            }
        }
        self.finished(1);
        Ok(())
    }

    /// Moves `node` to `target`: by renaming, or across filesystems by copying
    /// and then removing the source.
    fn move_entry(
        &mut self,
        node: &Node,
        target: &Path,
        replace: bool,
    ) -> Result<io::Result<Placed>, Stop> {
        let renamed = if replace {
            self.fs.rename_over(&node.path, target)
        } else {
            self.fs.rename_new(&node.path, target)
        };
        match renamed {
            Ok(()) => {
                if replace {
                    self.changes.push(Change::Replaced {
                        path: target.to_owned(),
                    });
                }
                if let Ok(info) = self.fs.info(target) {
                    self.changes.push(Change::Moved {
                        from: node.path.clone(),
                        to: target.to_owned(),
                        info,
                    });
                }
                self.finished(1);
                return Ok(Ok(Placed::Done));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && !replace => {
                return Ok(Ok(Placed::NameTaken));
            }
            Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {}
            Err(error) => return Ok(Err(error)),
        }

        // Another filesystem: copy everything, then remove the source.
        let scanned = match self.scan(node.path.clone()) {
            Ok(scanned) => scanned,
            Err(error) => return Ok(Err(error)),
        };
        let (items, bytes) = scanned.count();
        self.progress.items_total += items - 1;
        self.progress.bytes_total += bytes;
        match self.copy_entry(&scanned, target, replace)? {
            Ok(Placed::Done) => {}
            other => {
                self.progress.items_total -= items - 1;
                self.progress.bytes_total -= bytes;
                return Ok(other);
            }
        }
        // Publication has happened. Retry only its sync, never the copy or
        // replacement: both visible copies must survive a late sync failure.
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let fs = self.fs;
        if let Err(reason) = self.attempt(parent, |_| Ok(fs.sync_directory(parent)))? {
            self.skipped.push(Skipped {
                path: node.path.clone(),
                reason: format!("copy is visible, but its directory could not be synced; the original was kept: {reason}"),
            });
            return Ok(Ok(Placed::Done));
        }
        // In the record of changes it is a move, not a copy.
        self.changes
            .retain(|change| !matches!(change, Change::Copied { to, .. } if to == target));
        if self.remove_source(&scanned) {
            if let Ok(info) = self.fs.info(target) {
                self.changes.push(Change::Moved {
                    from: node.path.clone(),
                    to: target.to_owned(),
                    info,
                });
            }
            let parent = node.path.parent().unwrap_or_else(|| Path::new("."));
            if let Err(reason) = self.attempt(parent, |_| Ok(fs.sync_directory(parent)))? {
                self.skipped.push(Skipped {
                    path: node.path.clone(),
                    reason: format!(
                        "move is visible, but source removal could not be synced: {reason}"
                    ),
                });
            }
        } else {
            self.changes.push(Change::PartlyMoved {
                from: node.path.clone(),
                to: target.to_owned(),
            });
        }
        Ok(Ok(Placed::Done))
    }

    /// Removes a source tree whose copy is in place and says whether all of it
    /// went. An entry that was not copied, or that changed since it was read,
    /// is kept, and so are the folders above it.
    fn remove_source(&mut self, node: &Node) -> bool {
        if self.left_out.contains(&node.path) {
            return false;
        }
        let is_dir = node.info.kind == EntryKind::Directory;
        let mut children_gone = true;
        for child in &node.children {
            children_gone &= self.remove_source(child);
        }
        if !children_gone {
            return false;
        }
        let removed = match self.fs.info(&node.path) {
            Ok(info) if is_dir || info == node.info => {
                if is_dir {
                    self.fs.remove_dir(&node.path)
                } else {
                    self.fs.remove_file(&node.path)
                }
            }
            Ok(_) => Err(io::Error::other("it changed while it was being moved")),
            Err(error) => Err(error),
        };
        match removed {
            Ok(()) => true,
            Err(error) => {
                self.skipped.push(Skipped {
                    path: node.path.clone(),
                    reason: format!("copied, but the original was kept: {error}"),
                });
                false
            }
        }
    }
}

/// Result of trying to put an entry under its final name.
enum Placed {
    Done,
    /// The name was taken between the check and the rename.
    NameTaken,
    /// The entry was left out after a failure, which is already recorded.
    LeftOut,
}

mod archive;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod archive_tests;
mod batch;
#[cfg(any(test, feature = "test-support"))]
pub mod faults;
mod undo;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod undo_tests;
mod worker;

pub use archive::Layout;
pub use undo::{History, Record, UndoError, UndoFailure, undo};
pub use worker::{Finished, OpEvent, OpId, Operations, PROGRESS_INTERVAL};
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
