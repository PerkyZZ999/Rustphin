//! The filesystem as the core sees it, behind a trait so tests can substitute it.

use std::{ffi::OsString, fs, io, os::unix::ffi::OsStringExt, path::Path};

use crate::EntryKind;

/// One directory entry as returned by a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    /// File name as raw bytes.
    pub name: Vec<u8>,
    /// Kind, without following symbolic links.
    pub kind: EntryKind,
}

/// Filesystem access used by the core.
pub trait Fs: Send + Sync + 'static {
    /// Lists a directory. Entries arrive in filesystem order; "." and ".." are omitted.
    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<RawEntry>> + '_>>;

    /// Whether `path`, following symbolic links, is a directory.
    fn is_dir(&self, path: &Path) -> bool;
}

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFs;

impl Fs for RealFs {
    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<RawEntry>> + '_>> {
        Ok(Box::new(fs::read_dir(path)?.map(|entry| {
            let entry = entry?;
            let kind = match entry.file_type() {
                Ok(t) if t.is_dir() => EntryKind::Directory,
                Ok(t) if t.is_file() => EntryKind::File,
                Ok(t) if t.is_symlink() => EntryKind::Symlink,
                _ => EntryKind::Other,
            };
            Ok(RawEntry {
                name: OsString::into_vec(entry.file_name()),
                kind,
            })
        })))
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}
