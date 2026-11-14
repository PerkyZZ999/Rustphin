//! Applying a batch rename as one change, or not at all.

use std::{
    collections::HashSet,
    ffi::OsString,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::{
    BatchRename, Change, Engine, OpError, RENAME_JOURNAL, Skipped, TEMP_PREFIX, io_error,
    validate_name,
};

mod journal;

enum Where {
    Origin,
    Temporary,
    Final,
}

struct Slot {
    origin: PathBuf,
    temporary: PathBuf,
    final_path: PathBuf,
    at: Where,
    info: crate::FileInfo,
}

impl Engine<'_> {
    /// Renames every pair, or leaves the directory untouched when a later
    /// rename fails and the earlier ones can be put back.
    pub(super) fn rename_batch(
        &mut self,
        directory: &Path,
        pairs: &[(OsString, OsString)],
    ) -> Result<bool, OpError> {
        let _lock = self
            .fs
            .lock_directory(directory)
            .map_err(io_error(directory))?;
        let record = directory.join(RENAME_JOURNAL);
        match self.fs.info(&record) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&record)(error)),
            Ok(_) => {
                return Err(io_error(&record)(io::Error::other(
                    "a recovery record exists; use Recover original names after interruption before renaming again",
                )));
            }
        }
        let active = prepare(self, directory, pairs)?;
        if active.is_empty() {
            return Ok(true);
        }
        self.progress.items_total = active.len() as u64;
        self.controller.progress(&self.progress);

        let mut slots = active;
        let record_info = journal::save(self.fs, directory, &slots).map_err(io_error(&record))?;
        if let Err(error) = stage(self.fs, &mut slots) {
            return finish(self, directory, &mut slots, record_info, error);
        }
        if let Err(error) = publish(self.fs, &mut slots) {
            return finish(self, directory, &mut slots, record_info, error);
        }

        let mut items = Vec::with_capacity(slots.len());
        for slot in &slots {
            let info = match self.fs.info(&slot.final_path) {
                Ok(info) => info,
                Err(error) => return finish(self, directory, &mut slots, record_info, error),
            };
            items.push(BatchRename {
                from: slot.origin.clone(),
                to: slot.final_path.clone(),
                info,
            });
            self.progress.items_done += 1;
        }
        let cleanup = match journal::remove(self.fs, directory, record_info) {
            Ok(cleanup) => cleanup,
            Err(error) => return finish(self, directory, &mut slots, record_info, error),
        };
        if let journal::Cleanup::Uncertain(error) = cleanup {
            self.skipped.push(Skipped {
                path: directory.to_owned(),
                reason: format!("renamed files are visible; {error}. Check this folder again with Recover original names after interruption to retry its directory sync"),
            });
        }
        self.controller.progress(&self.progress);
        self.changes.push(Change::BatchRenamed { items });
        Ok(true)
    }
}

fn prepare(
    engine: &Engine<'_>,
    directory: &Path,
    pairs: &[(OsString, OsString)],
) -> Result<Vec<Slot>, OpError> {
    let mut targets = HashSet::new();
    let mut sources = HashSet::new();
    for (from, to) in pairs {
        validate_name(from)?;
        validate_name(to)?;
        if from == RENAME_JOURNAL || to == RENAME_JOURNAL {
            return Err(OpError::AlreadyExists(directory.join(RENAME_JOURNAL)));
        }
        if !sources.insert(from.as_bytes().to_vec()) {
            return Err(OpError::DuplicateName(from.clone()));
        }
        if !targets.insert(to.as_bytes().to_vec()) {
            return Err(OpError::DuplicateName(to.clone()));
        }
    }

    let present = names_in(engine, directory)?;
    for (from, _) in pairs {
        if !present.contains(from.as_bytes()) {
            return Err(OpError::Io {
                path: directory.join(from),
                source: io::ErrorKind::NotFound.into(),
            });
        }
    }
    let staying: HashSet<&[u8]> = pairs
        .iter()
        .filter(|(from, to)| from == to)
        .map(|(from, _)| from.as_bytes())
        .collect();
    let leaving: HashSet<&[u8]> = pairs
        .iter()
        .filter(|(from, to)| from != to)
        .map(|(from, _)| from.as_bytes())
        .collect();
    for (from, to) in pairs {
        if from == to {
            continue;
        }
        // A name is free when nothing has it, or the file that has it is
        // itself moving somewhere else. A file that stays keeps its name.
        let blocked = (present.contains(to.as_bytes()) && !leaving.contains(to.as_bytes()))
            || staying.contains(to.as_bytes());
        if blocked {
            return Err(OpError::AlreadyExists(directory.join(to)));
        }
    }

    let moving: Vec<_> = pairs
        .iter()
        .filter(|(from, to)| from != to)
        .cloned()
        .collect();
    let forbidden: HashSet<Vec<u8>> = present
        .iter()
        .cloned()
        .chain(targets.iter().cloned())
        .collect();
    let mut used = HashSet::new();
    let mut slots = Vec::with_capacity(moving.len());
    for (index, (from, to)) in moving.into_iter().enumerate() {
        let temporary = directory.join(fresh_temporary(&forbidden, &mut used, index));
        slots.push(Slot {
            origin: directory.join(&from),
            temporary,
            final_path: directory.join(to),
            at: Where::Origin,
            info: engine
                .fs
                .info(&directory.join(&from))
                .map_err(io_error(directory))?,
        });
    }
    Ok(slots)
}

fn names_in(engine: &Engine<'_>, directory: &Path) -> Result<HashSet<Vec<u8>>, OpError> {
    let listed = engine.fs.read_dir(directory).map_err(io_error(directory))?;
    let mut names = HashSet::new();
    for entry in listed {
        let entry = entry.map_err(io_error(directory))?;
        names.insert(entry.name);
    }
    Ok(names)
}

fn fresh_temporary(
    forbidden: &HashSet<Vec<u8>>,
    used: &mut HashSet<Vec<u8>>,
    index: usize,
) -> OsString {
    let mut n = index;
    loop {
        let name = format!("{TEMP_PREFIX}rename-{n}");
        let bytes = name.as_bytes().to_vec();
        if !forbidden.contains(&bytes) && used.insert(bytes) {
            return OsString::from(name);
        }
        n += 1;
    }
}

fn stage(fs: &dyn crate::Fs, slots: &mut [Slot]) -> Result<(), io::Error> {
    for slot in slots.iter_mut() {
        verify(fs, &slot.origin, slot.info)?;
        fs.rename_new(&slot.origin, &slot.temporary)?;
        slot.at = Where::Temporary;
    }
    Ok(())
}

fn publish(fs: &dyn crate::Fs, slots: &mut [Slot]) -> Result<(), io::Error> {
    for slot in slots.iter_mut() {
        verify(fs, &slot.temporary, slot.info)?;
        fs.rename_new(&slot.temporary, &slot.final_path)?;
        slot.at = Where::Final;
    }
    Ok(())
}

/// Puts every file back at its original name when the filesystem still
/// accepts the repairs. A repair that itself fails is recorded, so a later
/// undo can finish it.
fn finish(
    engine: &mut Engine<'_>,
    directory: &Path,
    slots: &mut [Slot],
    record_info: crate::FileInfo,
    error: io::Error,
) -> Result<bool, OpError> {
    let restored = restore(engine.fs, slots);
    let cleanup = restored.then(|| journal::remove(engine.fs, directory, record_info));
    let reason = match cleanup {
        Some(Ok(journal::Cleanup::Durable)) => {
            format!("the rename stopped ({error}). Nothing was changed.")
        }
        Some(Ok(journal::Cleanup::Uncertain(sync_error))) => format!(
            "original names were restored after the rename stopped ({error}); {sync_error}. Check recovery again to retry the directory sync"
        ),
        _ => format!(
            "the rename stopped ({error}). Keep any recovery record and temporary entries. Use Recover original names after interruption in this folder to restore original names"
        ),
    };
    engine.skipped.push(Skipped {
        path: directory.to_owned(),
        reason,
    });
    Ok(false)
}

fn restore(fs: &dyn crate::Fs, slots: &mut [Slot]) -> bool {
    let mut ok = true;
    for slot in slots.iter_mut().rev() {
        if matches!(slot.at, Where::Final)
            && verify(fs, &slot.final_path, slot.info)
                .and_then(|()| fs.rename_new(&slot.final_path, &slot.temporary))
                .is_err()
        {
            ok = false;
            break;
        }
        if matches!(slot.at, Where::Final) {
            slot.at = Where::Temporary;
        }
    }
    for slot in slots.iter_mut().rev() {
        if matches!(slot.at, Where::Temporary)
            && verify(fs, &slot.temporary, slot.info)
                .and_then(|()| fs.rename_new(&slot.temporary, &slot.origin))
                .is_err()
        {
            ok = false;
            break;
        }
        if matches!(slot.at, Where::Temporary) {
            slot.at = Where::Origin;
        }
    }
    ok
}

/// Moves a finished batch back, temporary names included, so a swap can return.
pub(super) fn revert_batch(
    fs: &dyn crate::Fs,
    items: &[BatchRename],
    controller: &mut dyn super::Controller,
) -> Result<(), super::UndoError> {
    let Some(parent) = items.first().and_then(|item| item.to.parent()) else {
        return Ok(());
    };
    let pairs = items
        .iter()
        .map(|item| {
            Ok((
                item.to
                    .file_name()
                    .ok_or_else(|| undo_io(&item.to)(io::ErrorKind::InvalidInput.into()))?
                    .to_owned(),
                item.from
                    .file_name()
                    .ok_or_else(|| undo_io(&item.from)(io::ErrorKind::InvalidInput.into()))?
                    .to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, super::UndoError>>()?;
    let request = super::Request::RenameBatch {
        directory: parent.to_owned(),
        pairs,
    };
    let outcome = super::run(
        fs,
        &super::NoTrash,
        &crate::archive::NoForeignReader,
        &request,
        controller,
    )
    .map_err(|e| undo_io(parent)(io::Error::other(e.to_string())))?;
    if outcome.completed {
        Ok(())
    } else {
        Err(undo_io(parent)(io::Error::other(
            outcome
                .skipped
                .first()
                .map_or("batch undo stopped", |s| s.reason.as_str()),
        )))
    }
}

fn undo_io(path: &Path) -> impl FnOnce(io::Error) -> super::UndoError + '_ {
    move |source| super::UndoError::Io {
        path: path.to_owned(),
        source,
    }
}

/// Whether `path` is free, or is the current place of another file in `items`.
pub(super) fn source_is_available(
    fs: &dyn crate::Fs,
    path: &Path,
    items: &[BatchRename],
) -> Result<(), super::UndoError> {
    if items.iter().any(|item| item.to == path) {
        return Ok(());
    }
    match fs.info(path) {
        Ok(_) => Err(super::UndoError::Occupied(path.to_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(undo_io(path)(error)),
    }
}

fn verify(fs: &dyn crate::Fs, path: &Path, expected: crate::FileInfo) -> io::Result<()> {
    if fs.info(path)? == expected {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} changed; recovery stopped without replacing it",
            path.display()
        )))
    }
}

pub(super) fn recover(fs: &dyn crate::Fs, directory: &Path) -> Result<usize, OpError> {
    let _lock = fs.lock_directory(directory).map_err(io_error(directory))?;
    let record = directory.join(RENAME_JOURNAL);
    let Some((mut slots, record_info)) = journal::load(fs, directory).map_err(io_error(&record))?
    else {
        // Also retries an earlier late cleanup sync after the record vanished.
        fs.sync_directory(directory).map_err(io_error(directory))?;
        return Ok(0);
    };
    let mut current = HashSet::new();
    for slot in &mut slots {
        let found = [&slot.temporary, &slot.origin, &slot.final_path]
            .into_iter()
            .find(|path| {
                !current.contains(*path) && fs.info(path).is_ok_and(|info| info == slot.info)
            })
            .ok_or_else(|| {
                io_error(&record)(io::Error::other(
                    "an entry is missing or changed; no recovery changes were made",
                ))
            })?;
        current.insert(found.clone());
        slot.at = if found == &slot.temporary {
            Where::Temporary
        } else if found == &slot.origin {
            Where::Origin
        } else {
            Where::Final
        };
    }
    // All occupied destinations must belong to this exact recorded batch.
    // This preflight completes before any recovery mutation.
    for slot in &slots {
        for path in [&slot.origin, &slot.temporary] {
            match fs.info(path) {
                Ok(_) if current.contains(path) => {}
                Ok(_) => return Err(OpError::AlreadyExists(path.clone())),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(path)(e)),
            }
        }
    }
    if !slots.iter().all(|s| matches!(s.at, Where::Origin)) {
        // Stage even already-restored entries so swaps and interrupted recovery
        // can use the same no-replace path on every retry.
        for slot in &mut slots {
            let path = match slot.at {
                Where::Origin => &slot.origin,
                Where::Final => &slot.final_path,
                Where::Temporary => continue,
            };
            verify(fs, path, slot.info)
                .and_then(|()| fs.rename_new(path, &slot.temporary))
                .map_err(io_error(path))?;
            slot.at = Where::Temporary;
        }
        if !restore(fs, &mut slots) {
            return Err(io_error(&record)(io::Error::other(
                "recovery paused; the record was kept and can be retried",
            )));
        }
    }
    match journal::remove(fs, directory, record_info).map_err(io_error(&record))? {
        journal::Cleanup::Durable => Ok(slots.len()),
        journal::Cleanup::Uncertain(error) => Err(io_error(directory)(error)),
    }
}
