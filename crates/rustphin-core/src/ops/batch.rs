//! Applying a batch rename as one change, or not at all.

use std::{
    collections::HashSet,
    ffi::OsString,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::{BatchRename, Change, Engine, OpError, Skipped, TEMP_PREFIX, io_error, validate_name};

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
}

impl Engine<'_> {
    /// Renames every pair, or leaves the directory untouched when a later
    /// rename fails and the earlier ones can be put back.
    pub(super) fn rename_batch(
        &mut self,
        directory: &Path,
        pairs: &[(OsString, OsString)],
    ) -> Result<bool, OpError> {
        let active = prepare(self, directory, pairs)?;
        if active.is_empty() {
            return Ok(true);
        }
        self.progress.items_total = active.len() as u64;
        self.controller.progress(&self.progress);

        let mut slots = active;
        if let Err(error) = stage(self.fs, &mut slots) {
            return finish(self, directory, &mut slots, error);
        }
        if let Err(error) = publish(self.fs, &mut slots) {
            return finish(self, directory, &mut slots, error);
        }

        let mut items = Vec::with_capacity(slots.len());
        for slot in &slots {
            let info = self
                .fs
                .info(&slot.final_path)
                .map_err(io_error(&slot.final_path))?;
            items.push(BatchRename {
                from: slot.origin.clone(),
                to: slot.final_path.clone(),
                info,
            });
            self.progress.items_done += 1;
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
        validate_name(to)?;
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
            origin: directory.join(from),
            temporary,
            final_path: directory.join(to),
            at: Where::Origin,
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
        fs.rename_new(&slot.origin, &slot.temporary)?;
        slot.at = Where::Temporary;
    }
    Ok(())
}

fn publish(fs: &dyn crate::Fs, slots: &mut [Slot]) -> Result<(), io::Error> {
    for slot in slots.iter_mut() {
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
    error: io::Error,
) -> Result<bool, OpError> {
    let restored = restore(engine.fs, slots);
    engine.skipped.push(Skipped {
        path: directory.to_owned(),
        reason: if restored {
            format!("the rename stopped ({error}). Nothing was changed.")
        } else {
            format!("the rename stopped ({error}) and could not be fully undone. Undo to put back the files that moved.")
        },
    });
    if restored {
        return Ok(false);
    }
    let mut items = Vec::new();
    for slot in slots.iter() {
        let current = match slot.at {
            Where::Origin => continue,
            Where::Temporary => &slot.temporary,
            Where::Final => &slot.final_path,
        };
        let Ok(info) = engine.fs.info(current) else {
            continue;
        };
        items.push(BatchRename {
            from: slot.origin.clone(),
            to: current.clone(),
            info,
        });
    }
    if !items.is_empty() {
        engine.changes.push(Change::BatchRenamed { items });
    }
    Ok(false)
}

fn restore(fs: &dyn crate::Fs, slots: &mut [Slot]) -> bool {
    let mut ok = true;
    for slot in slots.iter_mut().rev() {
        if matches!(slot.at, Where::Final)
            && fs.rename_new(&slot.final_path, &slot.temporary).is_err()
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
            && fs.rename_new(&slot.temporary, &slot.origin).is_err()
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
) -> Result<(), super::UndoError> {
    let Some(parent) = items.first().and_then(|item| item.to.parent()) else {
        return Ok(());
    };
    let mut temps = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let temp = parent.join(format!("{TEMP_PREFIX}undo-{index}"));
        fs.rename_new(&item.to, &temp).map_err(undo_io(&item.to))?;
        temps.push(temp);
    }
    for (item, temp) in items.iter().zip(&temps) {
        fs.rename_new(temp, &item.from).map_err(undo_io(temp))?;
    }
    Ok(())
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
