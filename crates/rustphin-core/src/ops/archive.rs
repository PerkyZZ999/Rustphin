//! Compress and extract (W-111, D-032) as engine operations.
//!
//! Extraction unpacks into a hidden temporary folder inside the destination,
//! checking every entry ([`archive::entry_path`], [`archive::link_stays_inside`]);
//! a failure, a wrong password or Stop removes that folder, so nothing half
//! extracted ever shows. Only then is the result given its place, with the
//! usual conflict question, and recorded as copied, so Undo trashes it.
//! Compression writes a temporary file beside the archive and renames it
//! into place when complete.

use std::{
    cell::Cell,
    collections::HashSet,
    ffi::OsStr,
    io::{self, Read, Write as _},
    ops::ControlFlow,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};

use super::{
    Change, Conflict, ConflictChoice, Engine, OpError, Progress, Skipped, file_name, io_error,
    temp_name, validate_name,
};
use crate::{
    EntryKind, FileInfo,
    archive::{self, Format, Kind, Level, ReadError, Source},
};

/// How Extract lays out what it unpacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// As Ark's "Extract here": a single top-level folder or file lands as
    /// it is; anything else goes into a new folder named after the archive.
    Here,
    /// Always into a new folder named after the archive.
    NewFolder,
}

/// How many times a wrong password is asked for again.
const PASSWORD_TRIES: u32 = 3;
/// Bytes copied per step while extracting; progress and Stop are checked
/// once per step.
const CHUNK: usize = 64 * 1024;
/// Errors that stop an extraction rather than leave one entry out: the
/// disk or the quota is full.
const FULL: [i32; 2] = [28, 122];

/// How unpacking into the temporary folder ended.
enum Unpacked {
    /// Everything was read; the archive's own top-level names.
    Done(HashSet<Vec<u8>>),
    Stopped,
    Failed(ReadError),
    NoSpace {
        needed: u64,
        free: u64,
    },
}

impl Engine<'_> {
    /// The format of `path`, judged by its name and first bytes.
    fn archive_format(&self, path: &Path) -> Result<Format, OpError> {
        let mut head = vec![0; 512];
        let mut reader = self.fs.open_read(path).map_err(io_error(path))?;
        let mut filled = 0;
        while filled < head.len() {
            match reader.read(&mut head[filled..]) {
                Ok(0) => break,
                Ok(read) => filled += read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(io_error(path)(error)),
            }
        }
        head.truncate(filled);
        let name = file_name(path)?;
        archive::detect(name.as_bytes(), &head)
            .or_else(|| archive::foreign_reader().map(|_| Format::Foreign))
            .ok_or_else(|| OpError::NotAnArchive(path.to_owned()))
    }

    /// Removes the temporary folder `path` and everything in it. Failures
    /// are ignored: it is hidden, and its name marks it as Rustphin's.
    fn remove_temp(&self, path: &Path) {
        let Ok(info) = self.fs.info(path) else {
            return;
        };
        if info.kind == EntryKind::Directory {
            if let Ok(entries) = self.fs.read_dir(path) {
                let names: Vec<_> = entries.flatten().map(|entry| entry.name).collect();
                for name in names {
                    self.remove_temp(&path.join(OsStr::from_bytes(&name)));
                }
            }
            let _ = self.fs.remove_dir(path);
        } else {
            let _ = self.fs.remove_file(path);
        }
    }

    /// Unpacks `archive` (W-111).
    pub(super) fn extract(
        &mut self,
        archive: &Path,
        into: &Path,
        layout: Layout,
        trash_archive: bool,
    ) -> Result<bool, OpError> {
        let into_info = self.fs.info(into).map_err(io_error(into))?;
        if into_info.kind != EntryKind::Directory && !self.fs.is_dir(into) {
            return Err(OpError::NotADirectory(into.to_owned()));
        }
        let format = self.archive_format(archive)?;
        let free = self.fs.free_space(into).ok();
        let mut password: Option<String> = None;
        let mut tries = 0;
        let (temp, tops) = loop {
            let temp = into.join(temp_name());
            self.fs.create_dir(&temp, 0o700).map_err(io_error(&temp))?;
            let unpacked = self.unpack(archive, format, &temp, password.as_deref(), free);
            if let Unpacked::Done(tops) = unpacked {
                break (temp, tops);
            }
            self.remove_temp(&temp);
            self.skipped.clear();
            match unpacked {
                Unpacked::Done(_) | Unpacked::Stopped => return Ok(false),
                Unpacked::NoSpace { needed, free } => {
                    return Err(OpError::NotEnoughSpace {
                        path: into.to_owned(),
                        needed,
                        free,
                    });
                }
                Unpacked::Failed(ReadError::NeedsPassword | ReadError::WrongPassword)
                    if tries < PASSWORD_TRIES =>
                {
                    match self.controller.password(archive, password.is_some()) {
                        Some(given) => {
                            password = Some(given);
                            tries += 1;
                        }
                        None => return Ok(false),
                    }
                }
                Unpacked::Failed(ReadError::NeedsPassword | ReadError::WrongPassword) => {
                    return Err(OpError::WrongPassword(archive.to_owned()));
                }
                Unpacked::Failed(ReadError::Damaged(reason)) => {
                    return Err(OpError::Damaged {
                        path: archive.to_owned(),
                        reason,
                    });
                }
                Unpacked::Failed(ReadError::Io(error)) => return Err(io_error(into)(error)),
            }
        };
        let completed = self.publish(archive, &temp, into, layout, tops.len())?;
        if completed && trash_archive && self.skipped.is_empty() {
            match self.trash.trash(archive) {
                Ok(item) => self.changes.push(Change::Trashed { item }),
                Err(error) => self.skipped.push(Skipped {
                    path: archive.to_owned(),
                    reason: format!("it was extracted but could not go to the trash: {error}"),
                }),
            }
        }
        Ok(completed)
    }

    /// Unpacks every entry of `archive` into `temp`.
    fn unpack(
        &mut self,
        archive: &Path,
        format: Format,
        temp: &Path,
        password: Option<&str>,
        free: Option<u64>,
    ) -> Unpacked {
        let source = match self.fs.open_read(archive) {
            Ok(source) => source,
            Err(error) => return Unpacked::Failed(ReadError::Io(error)),
        };
        self.progress = Progress {
            current: archive.to_owned(),
            ..Progress::default()
        };
        self.controller.progress(&self.progress);
        let declared = Cell::new(None);
        let short = Cell::new(None);
        let stopped = Cell::new(false);
        let mut made: HashSet<PathBuf> = HashSet::new();
        let mut tops: HashSet<Vec<u8>> = HashSet::new();
        let mut folders: Vec<(PathBuf, Option<u32>, Option<i64>)> = Vec::new();
        let read = archive::read_entries(
            format,
            archive,
            source,
            password,
            &mut |items, bytes| {
                declared.set(Some((items, bytes)));
                match free {
                    Some(free) if bytes > free => {
                        short.set(Some((bytes, free)));
                        ControlFlow::Break(())
                    }
                    _ => ControlFlow::Continue(()),
                }
            },
            &mut |header, content| {
                if let Some((items, bytes)) = declared.take() {
                    self.progress.items_total = items;
                    self.progress.bytes_total = bytes;
                }
                if self.controller.cancelled() {
                    stopped.set(true);
                    return Ok(ControlFlow::Break(()));
                }
                // The layout follows the archive, not what could be written.
                if let Ok(relative) = archive::entry_path(&header.name)
                    && let Some(top) = relative.components().next()
                {
                    tops.insert(top.as_os_str().as_bytes().to_vec());
                }
                match self.unpack_entry(archive, temp, header, content, &mut made, &mut folders) {
                    Ok(()) => {}
                    Err(error) if archive::is_stopped(&error) => {
                        stopped.set(true);
                        return Ok(ControlFlow::Break(()));
                    }
                    Err(error) => return Err(error),
                }
                self.finished(1);
                Ok(ControlFlow::Continue(()))
            },
        );
        if let Some((needed, free)) = short.get() {
            return Unpacked::NoSpace { needed, free };
        }
        if stopped.get() {
            return Unpacked::Stopped;
        }
        if let Err(error) = read {
            return Unpacked::Failed(error);
        }
        // Folders get their own times and modes last: adding their
        // contents changed them. Deepest first.
        for (path, mode, modified) in folders.into_iter().rev() {
            let mode = (mode.unwrap_or(0o755) & 0o777) | 0o700;
            if let Ok(info) = self.fs.info(&path) {
                let _ = self.fs.set_attributes(
                    &path,
                    &FileInfo {
                        mode,
                        modified_ns: modified.map_or(info.modified_ns, |seconds| {
                            i128::from(seconds) * 1_000_000_000
                        }),
                        ..info
                    },
                );
            }
        }
        Unpacked::Done(tops)
    }

    /// Writes one entry under `temp`. Errors that concern only this entry
    /// leave it out with a reason; a full disk and Stop end the extraction.
    fn unpack_entry(
        &mut self,
        archive: &Path,
        temp: &Path,
        header: &archive::Header,
        content: &mut dyn Read,
        made: &mut HashSet<PathBuf>,
        folders: &mut Vec<(PathBuf, Option<u32>, Option<i64>)>,
    ) -> io::Result<()> {
        let shown = archive.join(OsStr::from_bytes(&header.name));
        let relative = match archive::entry_path(&header.name) {
            Ok(relative) => relative,
            Err(reason) => {
                self.skipped.push(Skipped {
                    path: shown,
                    reason: format!("it was left out because {reason}"),
                });
                return Ok(());
            }
        };
        self.progress.current = shown.clone();
        let path = temp.join(&relative);
        let written = match &header.kind {
            Kind::Directory => self.make_folders(temp, &relative, made).map(|()| {
                folders.push((path, header.mode, header.modified));
            }),
            Kind::File => self.unpack_file(temp, &relative, header, content, made),
            Kind::Link(target) if archive::link_stays_inside(&relative, target) => self
                .make_parents(temp, &relative, made)
                .and_then(|()| self.fs.symlink(Path::new(OsStr::from_bytes(target)), &path)),
            Kind::Link(_) => {
                self.skipped.push(Skipped {
                    path: shown,
                    reason: "it was left out because its link points outside the folder".into(),
                });
                return Ok(());
            }
            Kind::Special => {
                self.skipped.push(Skipped {
                    path: shown,
                    reason: "it was left out because it is a device, a pipe or a hard link".into(),
                });
                return Ok(());
            }
        };
        match written {
            Ok(()) => Ok(()),
            Err(error)
                if archive::is_stopped(&error)
                    || error
                        .raw_os_error()
                        .is_some_and(|code| FULL.contains(&code)) =>
            {
                Err(error)
            }
            Err(error) => {
                self.skipped.push(Skipped {
                    path: shown,
                    reason: format!("it could not be written: {error}"),
                });
                Ok(())
            }
        }
    }

    /// Makes the folders `relative` names under `temp`, those not made yet.
    fn make_folders(
        &self,
        temp: &Path,
        relative: &Path,
        made: &mut HashSet<PathBuf>,
    ) -> io::Result<()> {
        let mut path = temp.to_path_buf();
        for part in relative.components() {
            path.push(part);
            if made.contains(&path) {
                continue;
            }
            // The temporary folder above is private; inside it, folders get
            // the user's usual rights (0777 less the umask).
            match self.fs.create_dir(&path, 0o777) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if self.fs.info(&path)?.kind != EntryKind::Directory {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "a file of the same name is in the way",
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
            made.insert(path.clone());
        }
        Ok(())
    }

    /// Makes the folders that hold `relative`.
    fn make_parents(
        &self,
        temp: &Path,
        relative: &Path,
        made: &mut HashSet<PathBuf>,
    ) -> io::Result<()> {
        match relative
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            Some(parent) => self.make_folders(temp, parent, made),
            None => Ok(()),
        }
    }

    fn unpack_file(
        &mut self,
        temp: &Path,
        relative: &Path,
        header: &archive::Header,
        content: &mut dyn Read,
        made: &mut HashSet<PathBuf>,
    ) -> io::Result<()> {
        self.make_parents(temp, relative, made)?;
        let path = temp.join(relative);
        // An archive may hold a name twice; the later entry wins, as in tar.
        if let Ok(existing) = self.fs.info(&path) {
            if existing.kind == EntryKind::Directory {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "a folder of the same name is in the way",
                ));
            }
            self.fs.remove_file(&path)?;
        }
        // Owner can always read and write; set-ID and sticky bits are dropped.
        let mode = (header.mode.unwrap_or(0o644) & 0o777) | 0o600;
        let mut out = self.fs.create_new_file(&path, mode)?;
        let mut buffer = vec![0; CHUNK];
        loop {
            let read = match content.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            out.write_all(&buffer[..read])?;
            self.progress.bytes_done = self.progress.bytes_done.saturating_add(read as u64).min(
                self.progress
                    .bytes_total
                    .max(self.progress.bytes_done + read as u64),
            );
            self.controller.progress(&self.progress);
            if self.controller.cancelled() {
                return Err(io::Error::other(archive::Stopped));
            }
        }
        out.finish()?;
        if let Some(seconds) = header.modified {
            let info = self.fs.info(&path)?;
            self.fs.set_attributes(
                &path,
                &FileInfo {
                    modified_ns: i128::from(seconds) * 1_000_000_000,
                    ..info
                },
            )?;
        }
        Ok(())
    }

    /// Gives what was unpacked in `temp` its place in `into`; `tops` is how
    /// many top-level names the archive holds.
    fn publish(
        &mut self,
        archive: &Path,
        temp: &Path,
        into: &Path,
        layout: Layout,
        tops: usize,
    ) -> Result<bool, OpError> {
        let top: Vec<Vec<u8>> = self
            .fs
            .read_dir(temp)
            .map_err(io_error(temp))?
            .filter_map(Result::ok)
            .map(|entry| entry.name)
            .collect();
        if top.is_empty() {
            self.remove_temp(temp);
            self.skipped.push(Skipped {
                path: archive.to_owned(),
                reason: "the archive holds nothing that could be extracted".into(),
            });
            return Ok(true);
        }
        let name = file_name(archive)?;
        let (from, mut target) = match (layout, top.as_slice()) {
            (Layout::Here, [only]) if tops == 1 => (
                temp.join(OsStr::from_bytes(only)),
                into.join(OsStr::from_bytes(only)),
            ),
            _ => {
                // The folder is Rustphin's own until now; give it ordinary rights.
                let _ = self.fs.set_mode(temp, 0o755);
                (
                    temp.to_path_buf(),
                    into.join(OsStr::from_bytes(archive::folder_stem(name.as_bytes()))),
                )
            }
        };
        let mut replace = false;
        loop {
            let target_info = match self.fs.info(&target) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => return Err(io_error(&target)(error)),
                Ok(info) => info,
            };
            let source_info = self.fs.info(&from).map_err(io_error(&from))?;
            let can_overwrite = source_info.kind != EntryKind::Directory
                && target_info.kind != EntryKind::Directory;
            let remembered = match self.for_all {
                Some(ConflictChoice::Overwrite) if !can_overwrite => None,
                choice => choice,
            };
            let choice = match remembered {
                Some(choice) => choice,
                None => {
                    let resolution = self.controller.conflict(&Conflict {
                        source: from.clone(),
                        target: target.clone(),
                        source_info,
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
                ConflictChoice::Cancel => {
                    self.remove_temp(temp);
                    return Ok(false);
                }
                ConflictChoice::Skip => {
                    self.remove_temp(temp);
                    self.skipped.push(Skipped {
                        path: target,
                        reason: "an entry with this name already exists".into(),
                    });
                    return Ok(true);
                }
                ConflictChoice::KeepBoth => target = self.free_name(&target),
                ConflictChoice::Overwrite if can_overwrite => {
                    replace = true;
                    break;
                }
                ConflictChoice::Overwrite => self.for_all = None,
            }
        }
        let placed = if replace {
            self.fs.rename_over(&from, &target)
        } else {
            self.fs.rename_new(&from, &target)
        };
        if let Err(error) = placed {
            self.remove_temp(temp);
            return Err(io_error(&target)(error));
        }
        if replace {
            self.changes.push(Change::Replaced {
                path: target.clone(),
            });
        }
        let info = self.fs.info(&target).map_err(io_error(&target))?;
        self.changes.push(Change::Copied {
            to: target.clone(),
            info,
        });
        if from != temp {
            self.remove_temp(temp);
        }
        let _ = self.fs.sync_directory(into);
        Ok(true)
    }

    /// Packs `sources` into the new archive `archive` (W-111).
    pub(super) fn compress(
        &mut self,
        sources: &[PathBuf],
        archive: &Path,
        format: Format,
        level: Level,
    ) -> Result<bool, OpError> {
        let name = file_name(archive)?;
        validate_name(name)?;
        if !format.writable() {
            return Err(OpError::NotAnArchive(archive.to_owned()));
        }
        let parent = archive
            .parent()
            .ok_or_else(|| OpError::NotADirectory(archive.to_owned()))?;
        if !self.fs.is_dir(parent) {
            return Err(OpError::NotADirectory(parent.to_owned()));
        }
        if self.fs.info(archive).is_ok() {
            return Err(OpError::AlreadyExists(archive.to_owned()));
        }
        let mut names: Vec<&OsStr> = Vec::new();
        let mut entries = Vec::new();
        for source in sources {
            let base = file_name(source)?;
            if names.contains(&base) {
                return Err(OpError::DuplicateName(base.to_owned()));
            }
            names.push(base);
            let node = self.scan(source.clone()).map_err(io_error(source))?;
            self.collect(&node, base.as_bytes().to_vec(), &mut entries);
        }
        self.progress = Progress {
            items_total: entries.len() as u64,
            bytes_total: entries.iter().map(|entry| entry.size).sum(),
            current: archive.to_owned(),
            ..Progress::default()
        };
        self.controller.progress(&self.progress);

        let temp = parent.join(temp_name());
        let mut out = self
            .fs
            .create_new_file(&temp, 0o666)
            .map_err(io_error(&temp))?;
        let fs = self.fs;
        let open = |path: &Path| -> io::Result<Box<dyn Read>> { Ok(fs.open_read(path)?) };
        let mut left_out: Vec<(PathBuf, &'static str)> = Vec::new();
        let written = archive::write_archive(
            format,
            level,
            &mut out,
            &entries,
            &open,
            &mut |bytes| {
                self.progress.bytes_done = self.progress.bytes_done.saturating_add(bytes);
                self.controller.progress(&self.progress);
                if self.controller.cancelled() {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
            &mut |entry, reason| left_out.push((entry.path.clone(), reason)),
        )
        .and_then(|()| out.finish());
        if let Err(error) = written {
            let _ = self.fs.remove_file(&temp);
            if archive::is_stopped(&error) {
                return Ok(false);
            }
            return Err(io_error(archive)(error));
        }
        for (path, reason) in left_out {
            self.skipped.push(Skipped {
                path,
                reason: format!("it was left out: {reason}"),
            });
        }
        if let Err(error) = self.fs.rename_new(&temp, archive) {
            let _ = self.fs.remove_file(&temp);
            return Err(io_error(archive)(error));
        }
        let _ = self.fs.sync_directory(parent);
        let info = self.fs.info(archive).map_err(io_error(archive))?;
        self.changes.push(Change::Copied {
            to: archive.to_owned(),
            info,
        });
        self.progress.items_done = self.progress.items_total;
        self.controller.progress(&self.progress);
        Ok(true)
    }

    /// Adds `node` and everything inside it to `entries`, named from `name`.
    fn collect(&mut self, node: &super::Node, name: Vec<u8>, entries: &mut Vec<Source>) {
        let kind = match node.info.kind {
            EntryKind::Directory => Kind::Directory,
            EntryKind::Symlink => match self.fs.read_link(&node.path) {
                Ok(target) => Kind::Link(target.as_os_str().as_bytes().to_vec()),
                Err(error) => {
                    self.skipped.push(Skipped {
                        path: node.path.clone(),
                        reason: format!("its link could not be read: {error}"),
                    });
                    return;
                }
            },
            EntryKind::File => Kind::File,
            _ => {
                self.skipped.push(Skipped {
                    path: node.path.clone(),
                    reason: "it was left out because it is not a file, folder or link".into(),
                });
                return;
            }
        };
        entries.push(Source {
            path: node.path.clone(),
            name: name.clone(),
            kind,
            mode: node.info.mode,
            modified: i64::try_from(node.info.modified_ns.div_euclid(1_000_000_000)).unwrap_or(0),
            size: if node.info.kind == EntryKind::File {
                node.info.size
            } else {
                0
            },
        });
        if let Some(kind) = node.unreadable {
            self.skipped.push(Skipped {
                path: node.path.clone(),
                reason: format!("its contents could not be read: {}", io::Error::from(kind)),
            });
        }
        for child in &node.children {
            let Some(child_name) = child.path.file_name() else {
                continue;
            };
            let mut path = name.clone();
            path.push(b'/');
            path.extend_from_slice(child_name.as_bytes());
            self.collect(child, path, entries);
        }
    }
}
