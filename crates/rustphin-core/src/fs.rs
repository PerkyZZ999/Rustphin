//! The filesystem as the core sees it, behind a trait so tests can substitute it.

use std::{
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    ops::ControlFlow,
    os::unix::{
        ffi::OsStringExt,
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
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

/// What an entry is right now, without following symbolic links. Two readings
/// that compare equal mean the entry has not been replaced or rewritten in
/// between, as far as the filesystem can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileInfo {
    /// Kind of the entry itself; a symbolic link reports [`EntryKind::Symlink`].
    pub kind: EntryKind,
    /// Size in bytes.
    pub size: u64,
    /// Last modification, in nanoseconds since the Unix epoch.
    pub modified_ns: i128,
    /// Device the entry lives on.
    pub device: u64,
    /// Inode number.
    pub inode: u64,
    /// Permission bits.
    pub mode: u32,
}

/// Bytes copied per step of [`Fs::copy_file`]; progress is reported, and
/// cancellation checked, once per step.
pub const COPY_STEP: usize = 4 * 1024 * 1024;

/// A file open for reading with seeking: an archive being read.
pub trait ReadSeek: io::Read + io::Seek + Send {}
impl<T: io::Read + io::Seek + Send> ReadSeek for T {}

/// A new file being written: an archive being made, or an entry being
/// extracted.
pub trait NewFile: io::Write + io::Seek + Send {
    /// Syncs the content to the disk; the file is complete after this.
    fn finish(self: Box<Self>) -> io::Result<()>;
}

impl NewFile for fs::File {
    fn finish(self: Box<Self>) -> io::Result<()> {
        self.sync_all()
    }
}

/// Filesystem access used by the core.
///
/// The methods that change the filesystem are deliberately small steps, so a
/// test can make any one of them fail and inspect what is left behind. None of
/// them follows a symbolic link in the last path component, and none replaces
/// an existing entry except [`Fs::rename_over`].
///
/// A filesystem that cannot be changed keeps the default implementations of
/// the changing methods, which fail with [`io::ErrorKind::Unsupported`].
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

    /// Describes the entry at `path` itself.
    fn info(&self, _path: &Path) -> io::Result<FileInfo> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates a directory with the given permission bits. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when something is there.
    fn create_dir(&self, _path: &Path, _mode: u32) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates an empty file. Fails with [`io::ErrorKind::AlreadyExists`]
    /// when something is there.
    fn create_file(&self, _path: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Copies the regular file `from` to the new file `to`, with its
    /// permission bits and modification time, syncing its content and attributes
    /// before returning success. This does not sync the containing directory.
    /// Fails with
    /// [`io::ErrorKind::AlreadyExists`] when `to` exists.
    ///
    /// `progress` is called with the number of bytes copied since its last
    /// call. When it breaks, copying stops with [`io::ErrorKind::Interrupted`].
    /// After any error `to` may exist with partial content; the caller removes it.
    fn copy_file(
        &self,
        _from: &Path,
        _to: &Path,
        _progress: &mut dyn FnMut(u64) -> ControlFlow<()>,
    ) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Reads where the symbolic link `path` points.
    fn read_link(&self, _path: &Path) -> io::Result<PathBuf> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates the symbolic link `link` pointing to `target`. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when something is there.
    fn symlink(&self, _target: &Path, _link: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Renames `from` to `to` in one step. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when `to` exists and with
    /// [`io::ErrorKind::CrossesDevices`] when they are on different filesystems.
    fn rename_new(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Renames `from` to `to` in one step, replacing `to` if it exists.
    fn rename_over(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Removes a file, symbolic link or other non-directory entry.
    fn remove_file(&self, _path: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Removes an empty directory.
    fn remove_dir(&self, _path: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Syncs a directory's metadata and entries, refusing a final-component link.
    /// Unsupported directory syncing is an error, not a successful durability claim.
    fn sync_directory(&self, _path: &Path) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Gives `path` the permission bits and modification time in `info`.
    /// For a directory, syncs it through a handle opened before changing modes.
    fn set_attributes(&self, _path: &Path, _info: &FileInfo) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Opens the regular file `path` for reading, with seeking.
    fn open_read(&self, _path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates the new file `path` with permission bits `mode` (less the
    /// umask) for writing. Fails with [`io::ErrorKind::AlreadyExists`] when
    /// something is there. Its content is on the disk only after
    /// [`NewFile::finish`]; after an error the caller removes it.
    fn create_new_file(&self, _path: &Path, _mode: u32) -> io::Result<Box<dyn NewFile>> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Bytes free for this user on the filesystem holding `path`.
    fn free_space(&self, _path: &Path) -> io::Result<u64> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Reads a whole small file, such as a trash record.
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Reads at most `limit` bytes of a regular file, refusing links and larger files.
    fn read_bounded(&self, _path: &Path, _limit: usize) -> io::Result<Vec<u8>> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Holds an exclusive advisory lock on a directory until the guard is dropped.
    /// Fails immediately if another batch operation holds the lock.
    fn lock_directory(&self, _path: &Path) -> io::Result<Box<dyn Send>> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates a private file `path` with `content` and syncs it. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when something is there.
    fn write_new(&self, _path: &Path, _content: &[u8]) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Creates the file `path` with `content`, with the permissions any new
    /// file of the user gets, and syncs it. Fails with
    /// [`io::ErrorKind::AlreadyExists`] when something is there.
    fn create_file_with(&self, _path: &Path, _content: &[u8]) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    /// Sets the permission bits of `path`. Other attributes are left as they are.
    fn set_mode(&self, _path: &Path, _mode: u32) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
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
        if let Ok(metadata) = fs::metadata(path) {
            return Ok(Metadata {
                size: metadata.len(),
                modified: metadata.mtime(),
            });
        }
        // A link whose target is missing has no size of its own to show;
        // Dolphin shows 0 B rather than the length of the link text.
        let link = fs::symlink_metadata(path)?;
        Ok(Metadata {
            size: if link.file_type().is_symlink() {
                0
            } else {
                link.len()
            },
            modified: link.mtime(),
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

    fn info(&self, path: &Path) -> io::Result<FileInfo> {
        let metadata = fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();
        Ok(FileInfo {
            kind: if file_type.is_dir() {
                EntryKind::Directory
            } else if file_type.is_file() {
                EntryKind::File
            } else if file_type.is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::Other
            },
            size: metadata.len(),
            modified_ns: i128::from(metadata.mtime()) * 1_000_000_000
                + i128::from(metadata.mtime_nsec()),
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode() & 0o7777,
        })
    }

    fn create_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::DirBuilder::new().mode(mode).create(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<()> {
        fs::File::create_new(path).map(drop)
    }

    fn copy_file(
        &self,
        from: &Path,
        to: &Path,
        progress: &mut dyn FnMut(u64) -> ControlFlow<()>,
    ) -> io::Result<()> {
        copy_file_with_sync(from, to, progress, || Ok(()))
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        fs::read_link(path)
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    fn rename_new(&self, from: &Path, to: &Path) -> io::Result<()> {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        // Without kernel no-replace support, fail closed. A stat followed by
        // ordinary rename could overwrite an entry created between the calls.
        Ok(renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE)?)
    }

    fn rename_over(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }

    fn set_mode(&self, path: &Path, mode: u32) -> io::Result<()> {
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o7777))
    }

    fn sync_directory(&self, path: &Path) -> io::Result<()> {
        open_directory(path)?.sync_all()
    }

    fn set_attributes(&self, path: &Path, info: &FileInfo) -> io::Result<()> {
        set_attributes_with_sync(path, info, || Ok(()))
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        let file = fs::File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        Ok(Box::new(file))
    }

    fn create_new_file(&self, path: &Path, mode: u32) -> io::Result<Box<dyn NewFile>> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(path)?;
        Ok(Box::new(file))
    }

    fn free_space(&self, path: &Path) -> io::Result<u64> {
        let stats = rustix::fs::statvfs(path)?;
        Ok(stats.f_bavail.saturating_mul(stats.f_frsize))
    }

    fn read_bounded(&self, path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        let flags = rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(flags.bits() as i32)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut bytes = Vec::new();
        file.take(limit.saturating_add(1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "recovery record is too large",
            ));
        }
        Ok(bytes)
    }

    fn lock_directory(&self, path: &Path) -> io::Result<Box<dyn Send>> {
        let directory = fs::File::open(path.join("."))?;
        rustix::fs::flock(
            &directory,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )?;
        Ok(Box::new(directory))
    }

    fn write_new(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content)?;
        file.sync_all()
    }

    fn create_file_with(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        // 0o666 less the umask, as for any file the user creates.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(content)?;
        file.sync_all()
    }
}

fn open_directory(path: &Path) -> io::Result<fs::File> {
    let flags = rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::NOFOLLOW;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags.bits() as i32)
        .open(path)
}

pub(crate) fn set_attributes_with_sync(
    path: &Path,
    info: &FileInfo,
    before_sync: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, UTIME_OMIT, futimens, utimensat};
    let times = Timestamps {
        last_access: Timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT,
        },
        last_modification: Timespec {
            tv_sec: info.modified_ns.div_euclid(1_000_000_000) as i64,
            tv_nsec: info.modified_ns.rem_euclid(1_000_000_000) as _,
        },
    };
    if info.kind == EntryKind::Directory {
        let directory = open_directory(path)?;
        directory.set_permissions(fs::Permissions::from_mode(info.mode))?;
        futimens(&directory, &times)?;
        before_sync()?;
        directory.sync_all()
    } else {
        fs::set_permissions(path, fs::Permissions::from_mode(info.mode))?;
        Ok(utimensat(CWD, path, &times, AtFlags::empty())?)
    }
}

/// Copies through the same open destination, with an injectable sync boundary.
pub(crate) fn copy_file_with_sync(
    from: &Path,
    to: &Path,
    progress: &mut dyn FnMut(u64) -> ControlFlow<()>,
    before_sync: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut source = fs::File::open(from)?;
    let metadata = source.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(metadata.mode() & 0o7777)
        .open(to)?;
    copy_contents(&mut source, &mut target, metadata.len(), progress)?;
    // The mode given at creation is reduced by the umask; set it exactly.
    target.set_permissions(fs::Permissions::from_mode(metadata.mode() & 0o7777))?;
    target.set_modified(metadata.modified()?)?;
    before_sync()
        .and_then(|()| target.sync_all())
        .map_err(|error| {
            io::Error::new(error.kind(), format!("could not sync copied file: {error}"))
        })
}

/// Copies `len` bytes of file content: by sharing extents where the filesystem
/// can (reflink), else inside the kernel (`copy_file_range`), else by reading
/// and writing.
fn copy_contents(
    source: &mut fs::File,
    target: &mut fs::File,
    len: u64,
    progress: &mut dyn FnMut(u64) -> ControlFlow<()>,
) -> io::Result<()> {
    let mut report = |bytes: u64| match progress(bytes) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(()) => Err(io::Error::from(io::ErrorKind::Interrupted)),
    };
    if len > 0 && rustix::fs::ioctl_ficlone(&*target, &*source).is_ok() {
        return report(len);
    }
    let mut copied = 0;
    while copied < len {
        let step = COPY_STEP.min(usize::try_from(len - copied).unwrap_or(COPY_STEP));
        match rustix::fs::copy_file_range(&*source, None, &*target, None, step) {
            // The source ended early: it was truncated while being copied.
            Ok(0) => break,
            Ok(bytes) => {
                copied += bytes as u64;
                report(bytes as u64)?;
            }
            // Not supported between these two files. Nothing was copied by
            // this call, so reading and writing continues from the same place.
            Err(
                rustix::io::Errno::XDEV
                | rustix::io::Errno::INVAL
                | rustix::io::Errno::NOSYS
                | rustix::io::Errno::OPNOTSUPP,
            ) => break,
            Err(error) => return Err(error.into()),
        }
    }
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let bytes = match source.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        target.write_all(&buffer[..bytes])?;
        report(bytes as u64)?;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_link_shows_its_target_size_and_a_broken_link_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plain.txt"), b"twelve bytes").unwrap();
        std::os::unix::fs::symlink("plain.txt", dir.path().join("link")).unwrap();
        std::os::unix::fs::symlink("missing-target", dir.path().join("broken")).unwrap();

        assert_eq!(RealFs.metadata(&dir.path().join("link")).unwrap().size, 12);
        assert_eq!(RealFs.metadata(&dir.path().join("broken")).unwrap().size, 0);
        assert!(RealFs.metadata(&dir.path().join("absent")).is_err());
    }

    fn kind_of(result: io::Result<Vec<u8>>) -> io::ErrorKind {
        result.unwrap_err().kind()
    }

    #[test]
    fn a_bounded_read_takes_a_file_up_to_its_limit() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("record");
        std::fs::write(&record, b"12345").unwrap();

        assert_eq!(RealFs.read_bounded(&record, 5).unwrap(), b"12345");
        assert_eq!(
            kind_of(RealFs.read_bounded(&record, 4)),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn a_bounded_read_never_follows_a_link() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("record"), b"12345").unwrap();
        std::os::unix::fs::symlink("record", dir.path().join("link")).unwrap();

        let error = RealFs
            .read_bounded(&dir.path().join("link"), 5)
            .unwrap_err();

        assert_eq!(
            error.raw_os_error(),
            Some(rustix::io::Errno::LOOP.raw_os_error()),
            "{error}"
        );
    }

    #[test]
    fn a_bounded_read_refuses_what_is_not_a_file_without_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let pipe = dir.path().join("pipe");
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &pipe,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::from_raw_mode(0o600),
            0,
        )
        .unwrap();

        // Opening a pipe for reading would wait for a writer.
        assert_eq!(
            kind_of(RealFs.read_bounded(&pipe, 5)),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            kind_of(RealFs.read_bounded(dir.path(), 5)),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn a_locked_directory_refuses_a_second_lock_until_released() {
        let dir = tempfile::tempdir().unwrap();
        let held = RealFs.lock_directory(dir.path()).unwrap();

        let second = RealFs.lock_directory(dir.path()).map(drop);
        drop(held);
        let after = RealFs.lock_directory(dir.path()).map(drop);

        assert_eq!(second.unwrap_err().kind(), io::ErrorKind::WouldBlock);
        assert!(after.is_ok(), "{after:?}");
    }
}
