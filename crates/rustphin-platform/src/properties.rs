//! Facts shown for one entry, and the size of a folder.
//!
//! Reading does not follow a symbolic link in the last component. A folder's
//! size walks its contents without following links, so a link cycle cannot
//! make the walk endless. The walk stops when `cancel` is set.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

/// What an entry is, without following a symbolic link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link.
    Symlink,
    /// A socket, FIFO, device node, or an entry whose type could not be told.
    Other,
}

/// What the properties view needs from one entry. A directory has no size
/// here; that is [`folder_size`], because it can take a long time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The entry's own kind.
    pub kind: Kind,
    /// Where a symbolic link points.
    pub target: Option<PathBuf>,
    /// Size in bytes. `None` for a directory.
    pub size: Option<u64>,
    /// Last modification, in seconds since the Unix epoch.
    pub modified: Option<i64>,
    /// Last access, in seconds since the Unix epoch.
    pub accessed: Option<i64>,
    /// Creation time, when the filesystem has one.
    pub created: Option<i64>,
    /// Permission bits.
    pub mode: u32,
}

/// The size of everything inside a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderSize {
    /// Bytes in files that could be read. Symbolic links are not followed
    /// and are not counted.
    pub bytes: u64,
    /// Entries that could not be read.
    pub unreadable: u64,
    /// The walk stopped because `cancel` was set.
    pub cancelled: bool,
}

/// Reads `path` itself. Does not walk a directory.
pub fn read(path: &Path) -> io::Result<Facts> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    let kind = if file_type.is_dir() {
        Kind::Directory
    } else if file_type.is_file() {
        Kind::File
    } else if file_type.is_symlink() {
        Kind::Symlink
    } else {
        Kind::Other
    };
    let target = if kind == Kind::Symlink {
        Some(fs::read_link(path)?)
    } else {
        None
    };
    Ok(Facts {
        kind,
        target,
        size: (kind != Kind::Directory).then_some(metadata.len()),
        modified: seconds(metadata.modified().ok()),
        accessed: seconds(metadata.accessed().ok()),
        created: seconds(metadata.created().ok()),
        mode: metadata.permissions().mode() & 0o7777,
    })
}

/// Sums the sizes of files under `path`. Does not follow symbolic links.
///
/// The first directory must be readable. A later directory or file that cannot
/// be read counts in [`FolderSize::unreadable`] and the walk continues.
pub fn folder_size(path: &Path, cancel: &AtomicBool) -> io::Result<FolderSize> {
    let mut stack = vec![path.to_owned()];
    let mut bytes = 0;
    let mut unreadable = 0;
    let mut started = false;
    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(FolderSize {
                bytes,
                unreadable,
                cancelled: true,
            });
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if !started => return Err(error),
            Err(_) => {
                unreadable += 1;
                continue;
            }
        };
        started = true;
        for entry in entries {
            let Ok(entry) = entry else {
                unreadable += 1;
                continue;
            };
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                unreadable += 1;
                continue;
            };
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
            } else {
                bytes += metadata.len();
            }
        }
    }
    Ok(FolderSize {
        bytes,
        unreadable,
        cancelled: false,
    })
}

fn seconds(time: Option<SystemTime>) -> Option<i64> {
    let time = time?;
    let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) else {
        return None;
    };
    i64::try_from(duration.as_secs()).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        fs,
        os::unix::{
            ffi::OsStringExt,
            fs::{PermissionsExt, symlink},
        },
        sync::atomic::AtomicBool,
    };

    use super::*;

    #[test]
    fn a_file_reports_its_size_and_permissions_and_a_bad_name_does_not_panic() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("note.txt");
        fs::write(&file, b"hello").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
        let facts = read(&file).unwrap();
        assert_eq!(facts.kind, Kind::File);
        assert_eq!(facts.size, Some(5));
        assert_eq!(facts.mode & 0o777, 0o640);
        assert!(facts.modified.is_some());

        let odd = root.path().join(std::ffi::OsString::from_vec(vec![0xff]));
        fs::write(&odd, b"x").unwrap();
        assert_eq!(read(&odd).unwrap().size, Some(1));
        assert!(read(&root.path().join("missing")).is_err());
    }

    #[test]
    fn a_folder_size_adds_files_skips_links_and_counts_what_it_cannot_read() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("docs");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("a"), b"abc").unwrap();
        let nested = dir.join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("b"), b"de").unwrap();
        fs::write(dir.join("big"), vec![0; 100]).unwrap();
        symlink(dir.join("big"), dir.join("link")).unwrap();
        let blocked = dir.join("blocked");
        fs::create_dir(&blocked).unwrap();
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

        let size = folder_size(&dir, &AtomicBool::new(false)).unwrap();
        assert!(!size.cancelled);
        assert_eq!(size.bytes, 3 + 2 + 100);
        assert!(size.unreadable >= 1);

        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).unwrap();
        let stopped = folder_size(&dir, &AtomicBool::new(true)).unwrap();
        assert!(stopped.cancelled);
    }

    #[test]
    fn a_symbolic_link_names_its_target_and_is_not_walked() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("note");
        fs::write(&file, b"hello").unwrap();
        let link = root.path().join("alias");
        symlink(&file, &link).unwrap();
        let facts = read(&link).unwrap();
        assert_eq!(facts.kind, Kind::Symlink);
        assert_eq!(facts.target.unwrap(), file);
        assert_eq!(
            folder_size(root.path(), &AtomicBool::new(false))
                .unwrap()
                .bytes,
            5
        );
    }
}
