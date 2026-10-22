//! A bounded, byte-preserving record written before any batch entry moves.

use super::{Slot, Where};
use crate::ops::{RENAME_JOURNAL, validate_name};
use crate::{EntryKind, FileInfo, Fs};
use std::{
    collections::HashSet,
    ffi::OsString,
    io,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::Path,
};

const MAGIC: &[u8] = b"RUSTPHIN-RENAME\x01";
const MAX_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn save(fs: &dyn Fs, directory: &Path, slots: &[Slot]) -> io::Result<FileInfo> {
    let root = fs.info(&directory.join("."))?;
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&root.device.to_le_bytes());
    bytes.extend_from_slice(&root.inode.to_le_bytes());
    bytes.extend_from_slice(&(slots.len() as u64).to_le_bytes());
    for slot in slots {
        for path in [&slot.origin, &slot.temporary, &slot.final_path] {
            let name = path.file_name().ok_or_else(invalid)?.as_bytes();
            bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
            bytes.extend_from_slice(name);
        }
        let info = slot.info;
        bytes.push(kind(info.kind));
        for n in [info.device, info.inode, info.size] {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        bytes.extend_from_slice(&info.modified_ns.to_le_bytes());
        bytes.extend_from_slice(&info.mode.to_le_bytes());
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
    }
    let record = directory.join(RENAME_JOURNAL);
    fs.write_new(&record, &bytes)?;
    fs.info(&record)
}

pub(super) fn load(fs: &dyn Fs, directory: &Path) -> io::Result<Option<(Vec<Slot>, FileInfo)>> {
    let record = directory.join(RENAME_JOURNAL);
    let stamp = match fs.info(&record) {
        Ok(info) => info,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if stamp.kind != EntryKind::File || stamp.size > MAX_BYTES as u64 {
        return Err(invalid());
    }
    let bytes = fs.read_bounded(&record, MAX_BYTES)?;
    if fs.info(&record)? != stamp {
        return Err(invalid());
    }
    let mut input = Input(&bytes);
    if input.take(MAGIC.len())? != MAGIC {
        return Err(invalid());
    }
    let root = fs.info(&directory.join("."))?;
    if input.u64()? != root.device || input.u64()? != root.inode {
        return Err(invalid());
    }
    let count = usize::try_from(input.u64()?).map_err(|_| invalid())?;
    // Each record has three nonempty names, six length bytes and 45 metadata bytes.
    if count == 0 || count > input.0.len() / 54 {
        return Err(invalid());
    }
    let mut slots = Vec::with_capacity(count);
    let (mut sources, mut targets, mut temps) = (HashSet::new(), HashSet::new(), HashSet::new());
    for _ in 0..count {
        let origin = input.name()?;
        let temporary = input.name()?;
        let final_name = input.name()?;
        if origin == final_name
            || !sources.insert(origin.clone())
            || !targets.insert(final_name.clone())
            || !temps.insert(temporary.clone())
        {
            return Err(invalid());
        }
        let temp = temporary.as_bytes();
        let Some(suffix) = temp.strip_prefix(b".rustphin-part-rename-") else {
            return Err(invalid());
        };
        if suffix.is_empty() || !suffix.iter().all(u8::is_ascii_digit) {
            return Err(invalid());
        }
        let kind = match input.take(1)?[0] {
            0 => EntryKind::File,
            1 => EntryKind::Directory,
            2 => EntryKind::Symlink,
            3 => EntryKind::Other,
            _ => return Err(invalid()),
        };
        let info = FileInfo {
            kind,
            device: input.u64()?,
            inode: input.u64()?,
            size: input.u64()?,
            modified_ns: i128::from_le_bytes(input.take(16)?.try_into().map_err(|_| invalid())?),
            mode: u32::from_le_bytes(input.take(4)?.try_into().map_err(|_| invalid())?),
        };
        slots.push(Slot {
            origin: directory.join(origin),
            temporary: directory.join(temporary),
            final_path: directory.join(final_name),
            info,
            at: Where::Origin,
        });
    }
    if !input.0.is_empty()
        || temps
            .iter()
            .any(|t| sources.contains(t) || targets.contains(t))
    {
        return Err(invalid());
    }
    Ok(Some((slots, stamp)))
}

fn kind(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::File => 0,
        EntryKind::Directory => 1,
        EntryKind::Symlink => 2,
        EntryKind::Other => 3,
    }
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid or changed batch-rename recovery record; no files were recovered",
    )
}

struct Input<'a>(&'a [u8]);
impl<'a> Input<'a> {
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let head = self.0.get(..n).ok_or_else(invalid)?;
        self.0 = &self.0[n..];
        Ok(head)
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| invalid())?,
        ))
    }
    fn name(&mut self) -> io::Result<OsString> {
        let len = u16::from_le_bytes(self.take(2)?.try_into().map_err(|_| invalid())?) as usize;
        let name = OsString::from_vec(self.take(len)?.to_vec());
        validate_name(&name).map_err(|_| invalid())?;
        if name.as_bytes() == RENAME_JOURNAL.as_bytes() {
            return Err(invalid());
        }
        Ok(name)
    }
}

pub(super) fn remove(fs: &dyn Fs, directory: &Path, stamp: FileInfo) -> io::Result<()> {
    let record = directory.join(RENAME_JOURNAL);
    if fs.info(&record)? != stamp {
        return Err(io::Error::other("the recovery record changed; it was kept"));
    }
    fs.remove_file(&record)
}
