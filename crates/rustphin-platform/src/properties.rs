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
    /// The owning group's id; `u32::MAX` when unknown.
    pub group_id: u32,
    /// The user running Rustphin owns the entry: only then may they give it
    /// to another of their groups.
    pub owned_by_user: bool,
    /// Bytes the entry takes on its disk (allocated blocks); `None` for a
    /// directory or when unknown.
    pub on_disk: Option<u64>,
}

/// What a picture's header says (Properties › Details, W-170).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The format's name: "PNG", "JPEG"…
    pub format: &'static str,
    /// Colour and channel depth, in words: "colour, 8 bits per channel".
    pub colour: String,
}

/// Reads the header of the picture at `path`, never decoding its pixels;
/// `None` for anything that is not a picture Rustphin reads (PNG, JPEG,
/// GIF, WebP, BMP). Reads the disk: call from a background thread.
pub fn picture(path: &Path) -> Option<Picture> {
    use image::ImageDecoder;
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let format = match reader.format()? {
        image::ImageFormat::Png => "PNG",
        image::ImageFormat::Jpeg => "JPEG",
        image::ImageFormat::Gif => "GIF",
        image::ImageFormat::WebP => "WebP",
        image::ImageFormat::Bmp => "BMP",
        _ => return None,
    };
    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    let colour = decoder.color_type();
    let channels = u16::from(colour.channel_count());
    let depth = colour.bits_per_pixel() / channels.max(1);
    let kind = match (colour.has_color(), colour.has_alpha()) {
        (true, true) => "colour with transparency",
        (true, false) => "colour",
        (false, true) => "greyscale with transparency",
        (false, false) => "greyscale",
    };
    Some(Picture {
        width,
        height,
        format,
        colour: format!("{kind}, {depth} bits per channel"),
    })
}

/// A group the user belongs to: its id and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The group's id.
    pub id: u32,
    /// Its name, or its number when it has none here.
    pub name: String,
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
        group_id: metadata.gid(),
        owned_by_user: own_uid(&account_file("/proc/self/status")) == Some(metadata.uid()),
        on_disk: (kind != Kind::Directory).then(|| metadata.blocks().saturating_mul(512)),
    })
}

/// The groups the user belongs to, by name, the primary group first: those
/// a file the user owns can be given to (W-170).
pub fn user_groups() -> Vec<Group> {
    groups_of(
        &account_file("/proc/self/status"),
        &account_file("/etc/group"),
    )
}

/// The groups `status` (the text of `/proc/<pid>/status`) lists, the
/// effective group first, named from `groups` (the text of `/etc/group`).
fn groups_of(status: &str, groups: &str) -> Vec<Group> {
    let field = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .map(|rest| {
                rest.split_whitespace()
                    .filter_map(|id| id.parse::<u32>().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    // `Gid:` is real, effective, saved, filesystem.
    let mut ids: Vec<u32> = field("Gid:").into_iter().nth(1).into_iter().collect();
    for id in field("Groups:") {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids.into_iter()
        .map(|id| Group {
            id,
            name: account_name(groups, id),
        })
        .collect()
}

/// The effective user id `status` (the text of `/proc/<pid>/status`) gives.
fn own_uid(status: &str) -> Option<u32> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|ids| ids.split_whitespace().nth(1))
        .and_then(|id| id.parse().ok())
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
        group_id: gid,
        // As administrator the group is changed by KDE's helper, not here.
        owned_by_user: false,
        on_disk: None,
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

    const STATUS: &str = "Name:\trustphin\nUid:\t1000\t1001\t1000\t1000\nGid:\t100\t1000\t1002\t1003\nGroups:\t4 1000 27 \n";
    const GROUPS: &str = "adm:x:4:alex\nsudo:x:27:alex\nalex:x:1000:\n";

    /// W-170: the groups offered, the effective group first, then the
    /// others once each, by name; a group with no name shows its number.
    #[test]
    fn the_users_groups_come_from_proc_and_are_named_from_etc_group() {
        let groups = groups_of(STATUS, GROUPS);
        let named: Vec<(u32, &str)> = groups.iter().map(|g| (g.id, g.name.as_str())).collect();
        assert_eq!(named, [(1000, "alex"), (4, "adm"), (27, "sudo")]);
        assert_eq!(groups_of("Groups:\t5\n", "")[0].name, "5");
        assert!(groups_of("", GROUPS).is_empty());
    }

    #[test]
    fn a_pictures_header_gives_its_size_format_and_colour() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.png");
        image::RgbaImage::new(30, 20).save(&path).unwrap();
        assert_eq!(
            picture(&path),
            Some(Picture {
                width: 30,
                height: 20,
                format: "PNG",
                colour: "colour with transparency, 8 bits per channel".into(),
            })
        );
        // Named like a picture, but not one: nothing.
        let fake = dir.path().join("fake.png");
        fs::write(&fake, b"not a picture").unwrap();
        assert_eq!(picture(&fake), None);
    }

    #[test]
    fn the_effective_user_is_the_second_uid() {
        assert_eq!(own_uid(STATUS), Some(1001));
        assert_eq!(own_uid("Name: x"), None);
    }

    #[test]
    fn a_file_read_knows_its_group_its_owner_and_its_place_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        fs::write(&path, vec![1u8; 10_000]).unwrap();
        let facts = read(&path).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(facts.group_id, metadata.gid());
        assert!(facts.owned_by_user, "the test made it");
        assert_eq!(facts.on_disk, Some(metadata.blocks() * 512));
        assert_eq!(read(dir.path()).unwrap().on_disk, None);
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
