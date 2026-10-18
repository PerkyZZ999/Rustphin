//! Undoing what an operation did, from its record of changes.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::{Change, Controller, Engine, Node, Placed, Progress, Request, Trash};
use crate::{EntryKind, Fs};

/// Why an operation cannot be undone, or why undoing it stopped.
#[derive(Debug, thiserror::Error)]
pub enum UndoError {
    /// The operation replaced a file; its previous content no longer exists.
    #[error("it replaced \"{}\", and the replaced file's content is gone", .0.display())]
    Replaced(PathBuf),
    /// The operation deleted something for good.
    #[error("\"{}\" was deleted for good", .0.display())]
    Deleted(PathBuf),
    /// A move left part of the source behind.
    #[error("\"{}\" was only partly moved; move the rest by hand", .0.display())]
    PartlyMoved(PathBuf),
    /// An entry the operation made or moved was modified afterwards.
    #[error("\"{}\" has changed since", .0.display())]
    Changed(PathBuf),
    /// An entry the operation made or moved is gone.
    #[error("\"{}\" is no longer there", .0.display())]
    Missing(PathBuf),
    /// Something now occupies the place an entry has to go back to.
    #[error("something else is now at \"{}\"", .0.display())]
    Occupied(PathBuf),
    /// The filesystem or the trash refused.
    #[error("\"{}\": {source}", path.display())]
    Io {
        /// What was being accessed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

/// An undo that did not go through.
#[derive(Debug)]
pub struct UndoFailure {
    /// What stood in the way.
    pub error: UndoError,
    /// The changes still in effect, in their original order. Equal to the
    /// whole record when nothing was undone.
    pub remaining: Vec<Change>,
}

/// Reverts `changes`, newest first.
///
/// Every change is checked before the first one is reverted: if an affected
/// entry was modified, removed or its old place taken, nothing is touched and
/// the error says which. Copies are undone by trashing them, never by deleting.
pub fn undo(
    fs: &dyn Fs,
    trash: &dyn Trash,
    changes: &[Change],
    controller: &mut dyn Controller,
) -> Result<(), UndoFailure> {
    let unchanged = |error| UndoFailure {
        error,
        remaining: changes.to_vec(),
    };
    changes
        .iter()
        .try_for_each(|change| check(fs, change))
        .map_err(unchanged)?;

    let mut engine = Engine {
        fs,
        trash,
        controller,
        changes: Vec::new(),
        skipped: Vec::new(),
        progress: Progress::default(),
        for_all: None,
        left_out: Default::default(),
    };
    for (index, change) in changes.iter().enumerate().rev() {
        if let Err(error) = revert(&mut engine, change) {
            return Err(UndoFailure {
                error,
                remaining: changes[..=index].to_vec(),
            });
        }
    }
    Ok(())
}

fn io_error(path: &Path) -> impl FnOnce(io::Error) -> UndoError + '_ {
    move |source| UndoError::Io {
        path: path.to_owned(),
        source,
    }
}

/// Whether `now` can be taken for the entry `then` recorded, unmodified.
///
/// A file must have the same size, modification time and permissions. Its
/// inode is not compared: an entry that an earlier undo brought back from
/// another filesystem is a new inode with the same content and times.
///
/// A folder only has to still be a folder. Its modification time moves
/// whenever something is created or removed directly inside it, which later
/// operations and their undo do legitimately. That is safe because reverting
/// a folder never deletes: a created folder is removed only if empty, a moved
/// one goes back with all it holds, a copied one goes to the trash.
fn same_entry(then: &crate::FileInfo, now: &crate::FileInfo) -> bool {
    match then.kind {
        EntryKind::Directory => now.kind == EntryKind::Directory,
        // A link's own times are those of its creation, not of its target.
        EntryKind::Symlink => (now.kind, now.size) == (EntryKind::Symlink, then.size),
        _ => {
            (then.kind, then.size, then.modified_ns, then.mode)
                == (now.kind, now.size, now.modified_ns, now.mode)
        }
    }
}

/// Fails unless `path` is as `expected` recorded it (see [`same_entry`]).
fn expect_unchanged(fs: &dyn Fs, path: &Path, expected: &crate::FileInfo) -> Result<(), UndoError> {
    match fs.info(path) {
        Ok(info) if same_entry(expected, &info) => Ok(()),
        Ok(_) => Err(UndoError::Changed(path.to_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(UndoError::Missing(path.to_owned()))
        }
        Err(error) => Err(io_error(path)(error)),
    }
}

/// Fails unless nothing is at `path`.
fn expect_free(fs: &dyn Fs, path: &Path) -> Result<(), UndoError> {
    match fs.info(path) {
        Ok(_) => Err(UndoError::Occupied(path.to_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path)(error)),
    }
}

fn check(fs: &dyn Fs, change: &Change) -> Result<(), UndoError> {
    match change {
        Change::Created { path, info } | Change::Copied { to: path, info } => {
            expect_unchanged(fs, path, info)
        }
        Change::Moved { from, to, info } => {
            expect_unchanged(fs, to, info)?;
            expect_free(fs, from)
        }
        Change::BatchRenamed { items } => {
            for item in items {
                expect_unchanged(fs, &item.to, &item.info)?;
            }
            for item in items {
                super::batch::source_is_available(fs, &item.from, items)?;
            }
            Ok(())
        }
        Change::Trashed { item } => {
            fs.info(&item.content).map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    UndoError::Missing(item.content.clone())
                } else {
                    io_error(&item.content)(error)
                }
            })?;
            expect_free(fs, &item.original)
        }
        Change::ModeSet { path, info, .. } => expect_unchanged(fs, path, info),
        Change::RemovedEmptyDir { path, .. } => expect_free(fs, path),
        Change::Restored { original } => match fs.info(original) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Err(UndoError::Missing(original.clone()))
            }
            Err(error) => Err(io_error(original)(error)),
        },
        Change::Replaced { path } => Err(UndoError::Replaced(path.clone())),
        Change::Deleted { path } => Err(UndoError::Deleted(path.clone())),
        Change::PartlyMoved { from, .. } => Err(UndoError::PartlyMoved(from.clone())),
    }
}

fn revert(engine: &mut Engine<'_>, change: &Change) -> Result<(), UndoError> {
    let fs = engine.fs;
    match change {
        Change::Created { path, info } => {
            // Checked again: an earlier step of this undo may have taken time.
            expect_unchanged(fs, path, info)?;
            // Removing a folder fails if anything is in it, so nothing made
            // since can be lost.
            if info.kind == EntryKind::Directory {
                fs.remove_dir(path)
            } else {
                fs.remove_file(path)
            }
            .map_err(io_error(path))
        }
        Change::Copied { to, info } => {
            expect_unchanged(fs, to, info)?;
            engine.trash.trash(to).map(drop).map_err(io_error(to))
        }
        Change::BatchRenamed { items } => super::batch::revert_batch(fs, items),
        Change::Moved { from, to, info } => {
            expect_unchanged(fs, to, info)?;
            let node = Node {
                path: to.clone(),
                info: *info,
                children: Vec::new(),
                unreadable: None,
            };
            match engine.move_entry(&node, from, false) {
                Ok(Ok(Placed::Done)) => Ok(()),
                Ok(Ok(Placed::NameTaken)) => Err(UndoError::Occupied(from.clone())),
                Ok(Ok(Placed::LeftOut)) => Err(UndoError::PartlyMoved(to.clone())),
                Ok(Err(error)) => Err(io_error(to)(error)),
                Err(_) => Err(io_error(to)(io::ErrorKind::Interrupted.into())),
            }
        }
        Change::Trashed { item } => engine.trash.restore(item).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                UndoError::Occupied(item.original.clone())
            } else {
                io_error(&item.original)(error)
            }
        }),
        Change::ModeSet {
            path,
            previous,
            info,
        } => {
            expect_unchanged(fs, path, info)?;
            fs.set_mode(path, *previous).map_err(io_error(path))
        }
        Change::RemovedEmptyDir { path, mode } => {
            fs.create_dir(path, *mode).map_err(io_error(path))
        }
        // Back into the trash, under a new record.
        Change::Restored { original } => engine
            .trash
            .trash(original)
            .map(drop)
            .map_err(io_error(original)),
        Change::Replaced { path } => Err(UndoError::Replaced(path.clone())),
        Change::Deleted { path } => Err(UndoError::Deleted(path.clone())),
        Change::PartlyMoved { from, .. } => Err(UndoError::PartlyMoved(from.clone())),
    }
}

/// One finished operation that can be offered for undo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// What was asked for, to describe the undo to the user.
    pub request: Request,
    /// What it changed.
    pub changes: Vec<Change>,
}

/// The operations of this session that can still be undone, oldest first.
#[derive(Debug, Default)]
pub struct History {
    records: Vec<Record>,
}

impl History {
    /// Operations remembered; older ones are forgotten.
    pub const LIMIT: usize = 100;

    /// Remembers an operation. One that changed nothing is not remembered,
    /// and neither is one that did something irreversible: it could never be
    /// undone and would only stand in the way of undoing what came before.
    pub fn push(&mut self, request: Request, changes: Vec<Change>) {
        if changes.is_empty() || !changes.iter().all(Change::is_reversible) {
            return;
        }
        if self.records.len() == Self::LIMIT {
            self.records.remove(0);
        }
        self.records.push(Record { request, changes });
    }

    /// How many operations can be undone.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether there is nothing to undo.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The operation the next undo would revert.
    pub fn last(&self) -> Option<&Record> {
        self.records.last()
    }

    /// Reverts the most recent operation. Returns `None` when there is none.
    ///
    /// On success the operation is forgotten and returned. When the undo is
    /// refused the operation stays; when it stops half-way, what is still in
    /// effect stays, so a later undo can finish the job.
    pub fn undo_last(
        &mut self,
        fs: &dyn Fs,
        trash: &dyn Trash,
        controller: &mut dyn Controller,
    ) -> Option<Result<Record, UndoError>> {
        let record = self.records.pop()?;
        Some(match undo(fs, trash, &record.changes, controller) {
            Ok(()) => Ok(record),
            Err(failure) => {
                if !failure.remaining.is_empty() {
                    self.records.push(Record {
                        request: record.request,
                        changes: failure.remaining,
                    });
                }
                Err(failure.error)
            }
        })
    }
}
