//! The filesystem as the core sees it, behind a trait so tests can substitute it.

use std::{
    ffi::OsString,
    fs, io,
    os::unix::{ffi::OsStringExt, fs::MetadataExt},
    path::Path,
};

use crate::EntryKind;

/// One directory entry as returned by a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    /// File name as raw bytes.
    pub name: Vec<u8>,
    /// Kind, without following symbolic links.
    pub kind: EntryKind,
}

/// Details of an entry that cost a separate system call to learn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// Size in bytes. For a symbolic link, the size of what it points to.
    pub size: u64,
    /// Last modification, in seconds since the Unix epoch.
    pub modified: i64,
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

    /// Reads the details of `path`, following symbolic links. A link whose
    /// target is missing reports the link itself.
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;

    /// Calls `on_change` whenever something directly inside the directory
    /// `path` is created, removed, renamed or modified, or the directory itself
    /// goes away. Calls may come in bursts and from any thread. Watching stops
    /// when the returned guard is dropped.
    fn watch(&self, path: &Path, on_change: Box<dyn Fn() + Send + Sync>) -> io::Result<WatchGuard>;
}

/// Keeps a [`Fs::watch`] alive; dropping it ends the watch.
pub type WatchGuard = Box<dyn Send>;

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

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let metadata = fs::metadata(path).or_else(|_| fs::symlink_metadata(path))?;
        Ok(Metadata {
            size: metadata.len(),
            modified: metadata.mtime(),
        })
    }

    fn watch(&self, path: &Path, on_change: Box<dyn Fn() + Send + Sync>) -> io::Result<WatchGuard> {
        use notify::{EventKind, RecursiveMode, Watcher as _};

        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // Reads by this or any other program are not changes. An error (for
                // example a queue overflow) may hide changes, so it counts as one.
                if !matches!(
                    event,
                    Ok(notify::Event {
                        kind: EventKind::Access(_),
                        ..
                    })
                ) {
                    on_change();
                }
            })
            .map_err(io::Error::other)?;
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        Ok(Box::new(watcher))
    }
}
