//! The desktop trash, following the freedesktop.org Trash specification 1.0,
//! so entries trashed here show up in other file managers and theirs here.
//!
//! An entry goes to a trash directory on its own filesystem: the home trash
//! (`$XDG_DATA_HOME/Trash`) when it shares a filesystem with it, otherwise
//! `.Trash/$uid` or `.Trash-$uid` at the top of its volume. Trashing is always
//! a rename; nothing is copied between filesystems and nothing is deleted.

use std::{
    ffi::{OsStr, OsString},
    io,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    sync::Arc,
};

use rustphin_core::{
    EntryKind, Fs,
    ops::{Trash, TrashedItem},
};

use crate::uri::{percent_decode, percent_encode};

/// One entry found in a trash directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashEntry {
    /// Where to find and restore it.
    pub item: TrashedItem,
    /// When it was trashed, as recorded: local time, `YYYY-MM-DDThh:mm:ss`.
    pub deleted: String,
}

/// The freedesktop.org trash.
pub struct FreedesktopTrash {
    fs: Arc<dyn Fs>,
    home_trash: PathBuf,
    uid: u32,
    now: fn() -> String,
}

fn local_time() -> String {
    jiff::Zoned::now().strftime("%Y-%m-%dT%H:%M:%S").to_string()
}

fn refuse(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

impl FreedesktopTrash {
    /// The current user's trash, or `None` when the environment names no data
    /// directory to keep it in.
    pub fn for_user(fs: Arc<dyn Fs>) -> Option<Self> {
        let home_trash = crate::xdg::data_dirs().into_iter().next()?.join("Trash");
        Some(Self::new(
            fs,
            home_trash,
            rustix::process::getuid().as_raw(),
        ))
    }

    /// A trash whose home trash directory is `home_trash`, acting as user `uid`.
    pub fn new(fs: Arc<dyn Fs>, home_trash: PathBuf, uid: u32) -> Self {
        Self {
            fs,
            home_trash,
            uid,
            now: local_time,
        }
    }

    /// The home trash directory.
    pub fn home_trash(&self) -> &Path {
        &self.home_trash
    }

    /// The folder holding the home trash's contents, created if missing.
    pub fn home_files_dir(&self) -> io::Result<PathBuf> {
        self.prepare(&self.home_trash)?;
        Ok(self.home_trash.join("files"))
    }

    /// Creates `dir` and its `files` and `info` subdirectories if missing.
    fn prepare(&self, dir: &Path) -> io::Result<()> {
        for path in [dir.to_owned(), dir.join("files"), dir.join("info")] {
            match self.fs.create_dir(&path, 0o700) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Picks the trash directory for an entry on `device` located in the
    /// directory `parent`, and the directory its recorded path is relative to
    /// (none for the home trash, which records absolute paths).
    fn trash_dir_for(&self, parent: &Path, device: u64) -> io::Result<(PathBuf, Option<PathBuf>)> {
        if let Some(data_home) = self.home_trash.parent()
            && self
                .fs
                .info(data_home)
                .is_ok_and(|info| info.device == device)
        {
            self.prepare(&self.home_trash)?;
            return Ok((self.home_trash.clone(), None));
        }
        // The top of the volume: the highest folder still on the same device.
        let top = parent
            .ancestors()
            .take_while(|dir| self.fs.info(dir).is_ok_and(|info| info.device == device))
            .last()
            .ok_or_else(|| refuse("the top of its volume could not be found"))?;
        // An administrator-provided `.Trash` is used only if it is a real
        // folder with the sticky bit, as the specification demands.
        let shared = top.join(".Trash");
        let shared_ok = self
            .fs
            .info(&shared)
            .is_ok_and(|info| info.kind == EntryKind::Directory && info.mode & 0o1000 != 0);
        if shared_ok {
            let dir = shared.join(self.uid.to_string());
            if self.prepare(&dir).is_ok() {
                return Ok((dir, Some(top.to_owned())));
            }
        }
        let dir = top.join(format!(".Trash-{}", self.uid));
        self.prepare(&dir).map_err(|error| {
            refuse(format!(
                "this volume has no trash and one could not be created ({error})"
            ))
        })?;
        Ok((dir, Some(top.to_owned())))
    }

    /// The entries in the trash directory `dir`. Records that cannot be read
    /// or whose content is gone are left out.
    pub fn list_dir(&self, dir: &Path) -> io::Result<Vec<TrashEntry>> {
        // `$top/.Trash-$uid` or `$top/.Trash/$uid`: recorded paths are relative to `$top`.
        let top = dir.parent().map(|parent| {
            if parent.file_name() == Some(OsStr::new(".Trash")) {
                parent.parent().unwrap_or(parent)
            } else {
                parent
            }
        });
        let mut entries = Vec::new();
        let listing = match self.fs.read_dir(&dir.join("info")) {
            Ok(listing) => listing,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(entries),
            Err(error) => return Err(error),
        };
        for raw in listing {
            let Ok(raw) = raw else { continue };
            let Some(stem) = raw.name.strip_suffix(b".trashinfo") else {
                continue;
            };
            let record = dir.join("info").join(OsStr::from_bytes(&raw.name));
            let content = dir.join("files").join(OsStr::from_bytes(stem));
            let Ok(text) = self.fs.read(&record) else {
                continue;
            };
            let Some((path, deleted)) = parse_record(&text) else {
                continue;
            };
            if self.fs.info(&content).is_err() {
                continue;
            }
            let original = match (path.is_absolute(), top) {
                (true, _) => path,
                (false, Some(top)) => top.join(path),
                (false, None) => continue,
            };
            entries.push(TrashEntry {
                item: TrashedItem {
                    original,
                    content,
                    record,
                },
                deleted,
            });
        }
        Ok(entries)
    }

    /// The entries in the home trash.
    pub fn list_home(&self) -> io::Result<Vec<TrashEntry>> {
        self.list_dir(&self.home_trash)
    }

    /// Every trash directory of this user that exists: the home trash first,
    /// then those at the top of mounted volumes.
    pub fn dirs(&self) -> Vec<PathBuf> {
        let is_dir = |path: &Path| {
            self.fs
                .info(path)
                .is_ok_and(|info| info.kind == EntryKind::Directory)
        };
        let mut dirs = vec![self.home_trash.clone()];
        let mounts = self
            .fs
            .read(Path::new("/proc/self/mountinfo"))
            .map(|text| mount_points(&text))
            .unwrap_or_default();
        for top in mounts {
            let shared = top.join(".Trash");
            let shared_ok = self
                .fs
                .info(&shared)
                .is_ok_and(|info| info.kind == EntryKind::Directory && info.mode & 0o1000 != 0);
            let candidates = [
                shared_ok.then(|| shared.join(self.uid.to_string())),
                Some(top.join(format!(".Trash-{}", self.uid))),
            ];
            for dir in candidates.into_iter().flatten() {
                if is_dir(&dir.join("files")) && !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
        dirs
    }

    /// The trash directory whose contents are shown when `path` is listed:
    /// `path` is the `files` folder of one of [`Self::dirs`].
    pub fn dir_showing(&self, path: &Path) -> Option<PathBuf> {
        let dir = path.parent()?;
        (path.file_name() == Some(OsStr::new("files"))
            && self.dirs().iter().any(|known| known == dir))
        .then(|| dir.to_owned())
    }

    /// The trashed entries of `dir` whose content is one of `contents`, and
    /// how many of `contents` have no record and so no place to go back to.
    pub fn items_for(
        &self,
        dir: &Path,
        contents: &[PathBuf],
    ) -> io::Result<(Vec<TrashedItem>, usize)> {
        let items: Vec<TrashedItem> = self
            .list_dir(dir)?
            .into_iter()
            .map(|entry| entry.item)
            .filter(|item| contents.contains(&item.content))
            .collect();
        let without_record = contents.len() - items.len().min(contents.len());
        Ok((items, without_record))
    }

    /// Everything that emptying the trash directory `dir` removes: the
    /// trashed contents and their records.
    pub fn everything_in(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for part in ["files", "info"] {
            let folder = dir.join(part);
            let listing = match self.fs.read_dir(&folder) {
                Ok(listing) => listing,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            for entry in listing {
                paths.push(folder.join(OsStr::from_bytes(&entry?.name)));
            }
        }
        Ok(paths)
    }
}

impl Trash for FreedesktopTrash {
    fn trash(&self, path: &Path) -> io::Result<TrashedItem> {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return Err(refuse("it has no name of its own"));
        };
        if !path.is_absolute() {
            return Err(refuse("its location is not a full path"));
        }
        if self.home_trash.starts_with(path) {
            return Err(refuse("it contains the trash itself"));
        }
        let info = self.fs.info(path)?;
        // Resolve links in the folder's path so the volume's top is found by
        // walking real folders. The entry itself is never followed.
        let parent = std::fs::canonicalize(parent)?;
        let (dir, relative_to) = self.trash_dir_for(&parent, info.device)?;
        if parent.starts_with(&dir) {
            return Err(refuse("it is already in the trash"));
        }
        let recorded = match &relative_to {
            Some(top) => parent
                .strip_prefix(top)
                .map_err(|_| refuse("it is outside its volume"))?
                .join(name),
            None => parent.join(name),
        };
        let record_text = format!(
            "[Trash Info]\nPath={}\nDeletionDate={}\n",
            percent_encode(recorded.as_os_str().as_bytes()),
            (self.now)()
        );

        for attempt in 0..10_000 {
            let stored = stored_name(name, attempt);
            let mut record_name = stored.clone();
            record_name.push(".trashinfo");
            let record = dir.join("info").join(record_name);
            let content = dir.join("files").join(&stored);
            // The record is written first and reserves the name.
            match self.fs.write_new(&record, record_text.as_bytes()) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            match self.fs.rename_new(path, &content) {
                Ok(()) => {
                    return Ok(TrashedItem {
                        original: path.to_owned(),
                        content,
                        record,
                    });
                }
                Err(error) => {
                    // Our own record of something that did not happen.
                    let _ = self.fs.remove_file(&record);
                    if error.kind() != io::ErrorKind::AlreadyExists {
                        return Err(error);
                    }
                }
            }
        }
        Err(refuse("no free name could be found in the trash"))
    }

    fn restore(&self, item: &TrashedItem) -> io::Result<()> {
        self.fs.rename_new(&item.content, &item.original)?;
        // The entry is back; a record left behind is only clutter.
        let _ = self.fs.remove_file(&item.record);
        Ok(())
    }
}

/// The mount points listed in the text of `/proc/self/mountinfo`.
fn mount_points(mountinfo: &[u8]) -> Vec<PathBuf> {
    mountinfo
        .split(|&byte| byte == b'\n')
        // Fields: id, parent id, device, root, mount point, …
        .filter_map(|line| line.split(|&byte| byte == b' ').nth(4))
        .map(|field| PathBuf::from(OsString::from_vec(unescape_octal(field))))
        .filter(|path| path.is_absolute())
        .collect()
}

/// Undoes the kernel's `\040`-style escapes for spaces, tabs, newlines and
/// backslashes in mount points.
fn unescape_octal(field: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(field.len());
    let mut rest = field;
    while let [byte, tail @ ..] = rest {
        let digit = |b: u8| (b'0'..=b'7').contains(&b).then(|| b - b'0');
        match (byte, tail) {
            (b'\\', [a, b, c, after @ ..]) => match (digit(*a), digit(*b), digit(*c)) {
                (Some(a), Some(b), Some(c)) if a <= 3 => {
                    out.push(a << 6 | b << 3 | c);
                    rest = after;
                }
                _ => {
                    out.push(*byte);
                    rest = tail;
                }
            },
            _ => {
                out.push(*byte);
                rest = tail;
            }
        }
    }
    out
}

/// The name an entry gets inside the trash on the n-th try: its own name,
/// then with a number, kept short enough for the `.trashinfo` suffix.
fn stored_name(name: &OsStr, attempt: u32) -> OsString {
    const LIMIT: usize = 255 - ".trashinfo".len() - " (10000)".len();
    let bytes = name.as_bytes();
    let mut stored = bytes[..bytes.len().min(LIMIT)].to_vec();
    if attempt > 0 {
        stored.extend_from_slice(format!(" ({attempt})").as_bytes());
    }
    OsString::from_vec(stored)
}

/// Reads the original path and the deletion date out of a `.trashinfo` file.
fn parse_record(text: &[u8]) -> Option<(PathBuf, String)> {
    let mut lines = text
        .split(|&byte| byte == b'\n')
        .map(|line| line.trim_ascii());
    if lines.find(|line| !line.is_empty())? != b"[Trash Info]" {
        return None;
    }
    let (mut path, mut deleted) = (None, String::new());
    for line in lines {
        if line.starts_with(b"[") {
            break;
        }
        if let Some(value) = line.strip_prefix(b"Path=") {
            // The first occurrence counts.
            path.get_or_insert_with(|| PathBuf::from(OsString::from_vec(percent_decode(value))));
        } else if let Some(value) = line.strip_prefix(b"DeletionDate=")
            && deleted.is_empty()
        {
            deleted = String::from_utf8_lossy(value).into_owned();
        }
    }
    let path = path.filter(|path| !path.as_os_str().is_empty())?;
    Some((path, deleted))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use rustphin_core::{
        RealFs,
        ops::faults::{Fault, FaultFs},
    };
    use tempfile::TempDir;

    use super::*;

    /// A home with a data directory, and a folder of files to trash.
    struct Home {
        _temp: TempDir,
        root: PathBuf,
        work: PathBuf,
        trash_dir: PathBuf,
    }

    fn home() -> Home {
        let temp = tempfile::tempdir().unwrap();
        // Canonical, as the trash resolves links in the paths it is given.
        let root = temp.path().canonicalize().unwrap();
        let work = root.join("work");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        Home {
            trash_dir: root.join("data/Trash"),
            _temp: temp,
            root,
            work,
        }
    }

    fn trash_on(home: &Home, fs: impl Fs) -> FreedesktopTrash {
        let mut trash = FreedesktopTrash::new(Arc::new(fs), home.trash_dir.clone(), 1000);
        trash.now = || "2026-10-12T14:30:00".to_owned();
        trash
    }

    fn file(path: &Path, content: &str) -> PathBuf {
        fs::write(path, content).unwrap();
        path.to_owned()
    }

    #[test]
    fn trashing_moves_the_entry_and_writes_the_record_the_specification_asks_for() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let path = file(&h.work.join("notes.txt"), "notes");

        let item = trash.trash(&path).unwrap();

        assert!(!path.exists());
        assert_eq!(item.content, h.trash_dir.join("files/notes.txt"));
        assert_eq!(fs::read_to_string(&item.content).unwrap(), "notes");
        assert_eq!(
            fs::read_to_string(h.trash_dir.join("info/notes.txt.trashinfo")).unwrap(),
            format!(
                "[Trash Info]\nPath={}\nDeletionDate=2026-10-12T14:30:00\n",
                path.display()
            )
        );
        assert_eq!(
            fs::metadata(&h.trash_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn names_with_spaces_and_bytes_outside_utf8_are_percent_encoded_and_read_back() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let name = OsString::from_vec(b"caf\xe9 & 100%.txt".to_vec());
        let path = file(&h.work.join(&name), "x");

        trash.trash(&path).unwrap();

        let record = fs::read_to_string(
            h.trash_dir
                .join("info")
                .join(OsString::from_vec(b"caf\xe9 & 100%.txt.trashinfo".to_vec())),
        )
        .unwrap();
        assert!(
            record.contains("/work/caf%E9%20%26%20100%25.txt\n"),
            "{record}"
        );
        let listed = trash.list_home().unwrap();
        assert_eq!(listed[0].item.original, path);
        assert_eq!(listed[0].deleted, "2026-10-12T14:30:00");
    }

    #[test]
    fn two_entries_with_the_same_name_both_fit_and_restore_to_their_own_places() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        fs::create_dir(h.work.join("other")).unwrap();
        let first = file(&h.work.join("a.txt"), "first");
        let second = file(&h.work.join("other/a.txt"), "second");

        let first_item = trash.trash(&first).unwrap();
        let second_item = trash.trash(&second).unwrap();

        assert_eq!(second_item.content, h.trash_dir.join("files/a.txt (1)"));
        trash.restore(&second_item).unwrap();
        trash.restore(&first_item).unwrap();
        assert_eq!(fs::read_to_string(&first).unwrap(), "first");
        assert_eq!(fs::read_to_string(&second).unwrap(), "second");
        assert!(trash.list_home().unwrap().is_empty());
        assert_eq!(fs::read_dir(h.trash_dir.join("info")).unwrap().count(), 0);
    }

    #[test]
    fn content_left_in_the_trash_without_a_record_is_not_replaced() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        fs::create_dir_all(h.trash_dir.join("files")).unwrap();
        file(&h.trash_dir.join("files/a.txt"), "someone else's");
        let path = file(&h.work.join("a.txt"), "mine");

        let item = trash.trash(&path).unwrap();

        assert_eq!(item.content, h.trash_dir.join("files/a.txt (1)"));
        assert_eq!(
            fs::read_to_string(h.trash_dir.join("files/a.txt")).unwrap(),
            "someone else's"
        );
        // The record written for the name that turned out to be taken is gone.
        assert!(!h.trash_dir.join("info/a.txt.trashinfo").exists());
    }

    #[test]
    fn restoring_does_not_replace_what_took_the_place() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let path = file(&h.work.join("a.txt"), "trashed");
        let item = trash.trash(&path).unwrap();
        file(&path, "newer");

        let error = trash.restore(&item).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "newer");
        assert_eq!(fs::read_to_string(&item.content).unwrap(), "trashed");
        assert_eq!(trash.list_home().unwrap().len(), 1);
    }

    #[test]
    fn a_link_is_trashed_as_a_link_and_a_folder_with_its_contents() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        fs::create_dir(h.work.join("folder")).unwrap();
        file(&h.work.join("folder/inner"), "inner");
        std::os::unix::fs::symlink(h.work.join("folder"), h.work.join("link")).unwrap();

        let link = trash.trash(&h.work.join("link")).unwrap();
        let folder = trash.trash(&h.work.join("folder")).unwrap();

        assert!(fs::symlink_metadata(&link.content).unwrap().is_symlink());
        assert_eq!(
            fs::read_to_string(folder.content.join("inner")).unwrap(),
            "inner"
        );
    }

    #[test]
    fn the_trash_and_what_contains_it_cannot_be_trashed() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let item = trash.trash(&file(&h.work.join("a"), "a")).unwrap();

        let error = trash.trash(&h.root.join("data")).unwrap_err();
        assert!(error.to_string().contains("contains the trash"), "{error}");
        assert!(trash.trash(&h.trash_dir).is_err());
        assert!(trash.trash(&item.content).is_err());
        assert!(item.content.exists() && h.trash_dir.exists());
    }

    /// A folder that [`FaultFs`] presents as a filesystem of its own.
    fn volume(home: &Home) -> (PathBuf, FaultFs) {
        let volume = home.root.join("volume");
        fs::create_dir_all(volume.join("docs")).unwrap();
        let fs = FaultFs::new().with_device(&volume);
        (volume, fs)
    }

    #[test]
    fn an_entry_on_another_volume_goes_to_that_volumes_own_trash() {
        let h = home();
        let (volume, fs) = volume(&h);
        let trash = trash_on(&h, fs);
        let path = file(&volume.join("docs/report.pdf"), "pdf");

        let item = trash.trash(&path).unwrap();

        assert_eq!(item.content, volume.join(".Trash-1000/files/report.pdf"));
        assert_eq!(
            fs::read_to_string(volume.join(".Trash-1000/info/report.pdf.trashinfo")).unwrap(),
            "[Trash Info]\nPath=docs/report.pdf\nDeletionDate=2026-10-12T14:30:00\n"
        );
        assert!(!h.trash_dir.exists(), "the home trash was not needed");
        let listed = trash.list_dir(&volume.join(".Trash-1000")).unwrap();
        assert_eq!(listed[0].item.original, path);
        trash.restore(&item).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "pdf");
    }

    #[test]
    fn a_shared_trash_folder_is_used_only_when_it_has_the_sticky_bit() {
        let h = home();
        let (volume, fs) = volume(&h);
        let trash = trash_on(&h, fs);
        fs::create_dir(volume.join(".Trash")).unwrap();
        let first = file(&volume.join("first"), "1");
        let second = file(&volume.join("second"), "2");

        let without_sticky = trash.trash(&first).unwrap();
        fs::set_permissions(volume.join(".Trash"), fs::Permissions::from_mode(0o1777)).unwrap();
        let with_sticky = trash.trash(&second).unwrap();

        assert_eq!(
            without_sticky.content,
            volume.join(".Trash-1000/files/first")
        );
        assert_eq!(with_sticky.content, volume.join(".Trash/1000/files/second"));
        let listed = trash.list_dir(&volume.join(".Trash/1000")).unwrap();
        assert_eq!(listed[0].item.original, second);
    }

    #[test]
    fn an_entry_stays_when_its_volume_cannot_hold_a_trash() {
        let h = home();
        let (volume, fs) = volume(&h);
        let trash = trash_on(&h, fs);
        let path = file(&volume.join("docs/a.txt"), "a");
        // Something that is not a folder sits where the trash would go.
        file(&volume.join(".Trash-1000"), "in the way");

        let error = trash.trash(&path).unwrap_err();

        assert!(error.to_string().contains("no trash"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "a");
        assert!(!h.trash_dir.exists());
    }

    #[test]
    fn a_failure_at_any_step_leaves_the_entry_where_it_was_or_whole_in_the_trash() {
        for step in 0.. {
            let h = home();
            let fs = FaultFs::failing_at(step, Fault::Crash);
            let path = file(&h.work.join("a.txt"), "content");
            let trash = FreedesktopTrash::new(Arc::new(fs), h.trash_dir.clone(), 1000);

            let result = trash.trash(&path);

            let in_trash = h.trash_dir.join("files/a.txt");
            let here = fs::read_to_string(&path).ok();
            let there = fs::read_to_string(&in_trash).ok();
            assert!(
                (here.as_deref(), there.as_deref()) == (Some("content"), None)
                    || (here.as_deref(), there.as_deref()) == (None, Some("content")),
                "step {step}: here {here:?}, in the trash {there:?}"
            );
            if let Ok(item) = result {
                assert_eq!(item.content, in_trash);
                assert!(step >= 4, "only {step} steps were exercised");
                return;
            }
        }
    }

    #[test]
    fn mount_points_are_read_with_their_escapes_undone() {
        let text = b"36 25 0:31 / / rw,relatime shared:1 - btrfs /dev/sda2 rw\n\
            98 36 8:17 / /mnt/My\\040Disk rw - ext4 /dev/sdb1 rw\n\
            99 36 8:18 / /mnt/back\\134slash rw - ext4 /dev/sdb2 rw\n\
            garbage\n";
        assert_eq!(
            mount_points(text),
            [
                PathBuf::from("/"),
                PathBuf::from("/mnt/My Disk"),
                PathBuf::from("/mnt/back\\slash")
            ]
        );
    }

    #[test]
    fn selected_contents_map_to_their_records_and_emptying_lists_everything() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let a = trash.trash(&file(&h.work.join("a"), "a")).unwrap();
        let b = trash.trash(&file(&h.work.join("b"), "b")).unwrap();
        let orphan = file(&h.trash_dir.join("files/orphan"), "no record");

        let (items, without_record) = trash
            .items_for(&h.trash_dir, &[b.content.clone(), orphan.clone()])
            .unwrap();
        assert_eq!(items, std::slice::from_ref(&b));
        assert_eq!(without_record, 1);

        let mut everything = trash.everything_in(&h.trash_dir).unwrap();
        everything.sort();
        let mut expected = vec![a.content, b.content, orphan, a.record, b.record];
        expected.sort();
        assert_eq!(everything, expected);

        assert_eq!(
            trash.dir_showing(&h.trash_dir.join("files")),
            Some(h.trash_dir.clone())
        );
        assert_eq!(trash.dir_showing(&h.trash_dir), None);
        assert_eq!(trash.dir_showing(&h.work), None);
        assert_eq!(trash.dirs()[0], h.trash_dir);
    }

    #[test]
    fn records_from_other_file_managers_are_read() {
        // As written by Dolphin (KIO) and by GLib: key order and extra
        // whitespace vary, and paths in a volume trash are relative.
        assert_eq!(
            parse_record(
                b"[Trash Info]\nPath=/home/u/My%20File.txt\nDeletionDate=2026-01-02T03:04:05\n"
            ),
            Some(("/home/u/My File.txt".into(), "2026-01-02T03:04:05".into()))
        );
        assert_eq!(
            parse_record(
                b"\n[Trash Info]\r\nDeletionDate=2026-01-02T03:04:05\r\nPath=docs/a%2Fb\r\n"
            ),
            Some(("docs/a/b".into(), "2026-01-02T03:04:05".into()))
        );
        assert_eq!(parse_record(b"Path=/x\n"), None);
        assert_eq!(parse_record(b"[Trash Info]\nDeletionDate=2026\n"), None);
        assert_eq!(
            parse_record(b"[Trash Info]\nPath=bad%zzescape\n"),
            Some(("bad%zzescape".into(), String::new()))
        );
    }

    #[test]
    fn a_volume_trash_records_paths_relative_to_the_top_of_the_volume() {
        let h = home();
        let trash = trash_on(&h, RealFs);
        let volume_trash = h.root.join("volume/.Trash-1000");
        fs::create_dir_all(volume_trash.join("files")).unwrap();
        fs::create_dir_all(volume_trash.join("info")).unwrap();
        file(&volume_trash.join("files/report.pdf"), "pdf");
        file(
            &volume_trash.join("info/report.pdf.trashinfo"),
            "[Trash Info]\nPath=docs/report.pdf\nDeletionDate=2026-01-02T03:04:05\n",
        );
        // An orphan record, whose content is gone, is not listed.
        file(
            &volume_trash.join("info/gone.trashinfo"),
            "[Trash Info]\nPath=gone\nDeletionDate=2026-01-02T03:04:05\n",
        );

        let listed = trash.list_dir(&volume_trash).unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].item.original,
            h.root.join("volume/docs/report.pdf")
        );
    }
}
