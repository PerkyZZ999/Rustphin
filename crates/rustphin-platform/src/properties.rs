//! Facts shown for one entry, and the size of a folder.
//!
//! Reading does not follow a symbolic link in the last component. A folder's
//! size walks its contents without following links, so a link cycle cannot
//! make the walk endless. The walk stops when `cancel` is set.

use std::{
    fs, io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
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
    /// The owning user's name, or its number when it has none here.
    pub owner: String,
    /// The owning group's name, or its number when it has none here.
    pub group: String,
}

/// The size of everything inside a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderSize {
    /// Bytes in files that could be read. Symbolic links are not followed
    /// and are not counted.
    pub bytes: u64,
    /// Entries that could not be read.
    pub unreadable: u64,
    /// Entries found, folders included.
    pub items: u64,
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
        owner: account_name(&account_file("/etc/passwd"), metadata.uid()),
        group: account_name(&account_file("/etc/group"), metadata.gid()),
    })
}

/// The same facts from KDE's administrator helper's description (W-098):
/// for entries the user cannot read.
pub fn described_as_administrator(entry: &crate::admin::AdminEntry) -> Facts {
    use crate::admin::AdminKind;
    let kind = match entry.kind {
        AdminKind::File => Kind::File,
        AdminKind::Directory => Kind::Directory,
        AdminKind::Symlink => Kind::Symlink,
        AdminKind::Other => Kind::Other,
    };
    let seconds = |time: Option<(i64, u32)>| time.map(|(seconds, _)| seconds);
    let (uid, gid) = entry.owner.unwrap_or((u32::MAX, u32::MAX));
    let owner = entry.owner_names.0.clone().unwrap_or_else(|| {
        if uid == u32::MAX {
            "unknown".to_owned()
        } else {
            account_name(&account_file("/etc/passwd"), uid)
        }
    });
    let group = entry.owner_names.1.clone().unwrap_or_else(|| {
        if gid == u32::MAX {
            "unknown".to_owned()
        } else {
            account_name(&account_file("/etc/group"), gid)
        }
    });
    Facts {
        kind,
        target: entry.link_target.as_ref().map(PathBuf::from),
        size: (kind != Kind::Directory).then_some(entry.size),
        modified: seconds(entry.modified),
        accessed: seconds(entry.accessed),
        created: seconds(entry.created),
        mode: entry.permissions,
        owner,
        group,
    }
}

/// The contents of an account file, or nothing when it cannot be read.
fn account_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// The name an account file (`/etc/passwd` or `/etc/group`: `name:x:id:…`)
/// gives `id`, or the number itself. Accounts kept elsewhere (LDAP and the
/// like) show as numbers.
fn account_name(file: &str, id: u32) -> String {
    file.lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next().filter(|name| !name.is_empty())?;
            let number = fields.nth(1)?.parse::<u32>().ok()?;
            (number == id).then_some(name)
        })
        .next()
        .map_or_else(|| id.to_string(), str::to_owned)
}

/// Sums the sizes of files under `path`. Does not follow symbolic links.
///
/// The first directory must be readable. A later directory or file that cannot
/// be read counts in [`FolderSize::unreadable`] and the walk continues. `seen`
/// counts the entries found so far, for a view to show while it waits.
pub fn folder_size(path: &Path, cancel: &AtomicBool, seen: &AtomicU64) -> io::Result<FolderSize> {
    let mut stack = vec![path.to_owned()];
    let mut bytes = 0;
    let mut unreadable = 0;
    let mut items = 0;
    let mut started = false;
    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(FolderSize {
                bytes,
                unreadable,
                items,
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
            items += 1;
            seen.store(items, Ordering::Relaxed);
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
        items,
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
        sync::atomic::{AtomicBool, AtomicU64},
    };

    use super::*;

    fn helper_entry() -> crate::admin::AdminEntry {
        crate::admin::AdminEntry {
            name: "shadow".into(),
            kind: crate::admin::AdminKind::File,
            size: 1234,
            permissions: 0o640,
            modified: Some((1_700_000_000, 5)),
            accessed: None,
            created: Some((1_600_000_000, 0)),
            owner: Some((0, 0)),
            owner_names: (Some("root".into()), Some("shadow".into())),
            identity: None,
            link_target: None,
            leads_to_directory: false,
        }
    }

    #[test]
    fn the_helpers_description_becomes_the_same_facts() {
        let facts = described_as_administrator(&helper_entry());
        assert_eq!(facts.kind, Kind::File);
        assert_eq!(facts.size, Some(1234));
        assert_eq!(facts.mode, 0o640);
        assert_eq!(facts.modified, Some(1_700_000_000));
        assert_eq!(facts.accessed, None);
        assert_eq!(facts.created, Some(1_600_000_000));
        assert_eq!(
            (facts.owner.as_str(), facts.group.as_str()),
            ("root", "shadow")
        );
    }

    #[test]
    fn a_folder_from_the_helper_has_no_size_and_a_link_keeps_its_target() {
        let mut entry = helper_entry();
        entry.kind = crate::admin::AdminKind::Directory;
        assert_eq!(described_as_administrator(&entry).size, None);
        entry.kind = crate::admin::AdminKind::Symlink;
        entry.link_target = Some("/etc/passwd".into());
        let facts = described_as_administrator(&entry);
        assert_eq!(facts.kind, Kind::Symlink);
        assert_eq!(facts.target, Some(PathBuf::from("/etc/passwd")));
    }

    #[test]
    fn owners_without_names_are_looked_up_or_called_unknown() {
        let mut entry = helper_entry();
        entry.owner_names = (None, None);
        entry.owner = None;
        let facts = described_as_administrator(&entry);
        assert_eq!(
            (facts.owner.as_str(), facts.group.as_str()),
            ("unknown", "unknown")
        );
    }

    #[test]
    fn owners_are_named_from_the_account_files_or_by_number() {
        let passwd =
            "root:x:0:0:root:/root:/bin/sh\n# comment\nalex:x:1000:1000:Alex:/home/alex:/bin/zsh\n";
        assert_eq!(account_name(passwd, 1000), "alex");
        assert_eq!(account_name(passwd, 0), "root");
        // Unknown ids and broken lines fall back to the number.
        assert_eq!(account_name(passwd, 4242), "4242");
        assert_eq!(account_name("broken\n:x:7:7\n", 7), "7");
        let group = "wheel:x:10:alex\nalex:x:1000:\n";
        assert_eq!(account_name(group, 10), "wheel");
    }

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

    /// Gives a folder its permissions back when the test ends, passed or
    /// not, so the temporary folder can be removed.
    struct Reopen<'a>(&'a Path);

    impl Drop for Reopen<'_> {
        fn drop(&mut self) {
            let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
        }
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
        fs::write(blocked.join("hidden"), b"never counted").unwrap();
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
        let _reopen = Reopen(&blocked);

        let seen = AtomicU64::new(0);
        let size = folder_size(&dir, &AtomicBool::new(false), &seen).unwrap();

        assert!(!size.cancelled);
        // Root reads the closed folder anyway.
        let as_root = rustix::process::geteuid().is_root();
        let hidden = if as_root { 13 } else { 0 };
        assert_eq!(size.bytes, 3 + 2 + 100 + hidden);
        assert_eq!(size.unreadable, u64::from(!as_root));
        // Every entry listed counts, as the dialog says while it waits:
        // a, nested, nested/b, big, link and blocked (and blocked/hidden).
        assert_eq!(size.items, 6 + u64::from(as_root));
        assert_eq!(seen.load(Ordering::Relaxed), size.items);
    }

    #[test]
    fn a_cancelled_folder_size_says_so() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), b"abc").unwrap();

        let stopped = folder_size(root.path(), &AtomicBool::new(true), &AtomicU64::new(0)).unwrap();

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
            folder_size(root.path(), &AtomicBool::new(false), &AtomicU64::new(0))
                .unwrap()
                .bytes,
            5
        );
    }
}
