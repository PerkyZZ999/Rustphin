//! Reading and writing Rustphin's own configuration files.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Rustphin's folder inside the user's configuration directory.
pub fn config_dir() -> Option<PathBuf> {
    crate::xdg::config_home().map(|home| home.join("rustphin"))
}

/// Rustphin's folder inside the user's state directory.
pub fn state_dir() -> Option<PathBuf> {
    crate::xdg::state_home().map(|home| home.join("rustphin"))
}

/// Reads a configuration file. A missing file is `Ok(None)`.
pub fn read(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Atomically replaces `path` with a private file containing `contents`.
/// Missing parent folders are created. The file is synced before publication,
/// then its directory is synced. A sync error after publication is reported,
/// although the new whole file is already visible; callers must not assume an
/// error means the old contents remain. This is not a hardware power-loss proof.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_atomic_with_sync(path, contents, &File::sync_all)
}

fn write_atomic_with_sync(
    path: &Path,
    contents: &[u8],
    sync_directory: &impl Fn(&File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let name = path.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    let folder = ensure_directory(parent, sync_directory)?;
    let (temporary, mut file) = create_temporary(parent, name)?;
    let written = (|| {
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if let Err(error) = written {
        if let Err(cleanup) = fs::remove_file(&temporary) {
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "configuration write failed: {error}; temporary file cleanup failed: {cleanup}"
                ),
            ));
        }
        return Err(error);
    }
    sync_directory(&folder).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("configuration was replaced, but its directory could not be synced: {error}"),
        )
    })
}

fn ensure_directory(
    path: &Path,
    sync_directory: &impl Fn(&File) -> io::Result<()>,
) -> io::Result<File> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    match File::open(path) {
        Ok(folder) => {
            if !folder.metadata()?.is_dir() {
                return Err(io::ErrorKind::NotADirectory.into());
            }
            Ok(folder)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
            let parent_folder = ensure_directory(parent, sync_directory)?;
            match fs::create_dir(path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
            let folder = File::open(path)?;
            if !folder.metadata()?.is_dir() {
                return Err(io::ErrorKind::NotADirectory.into());
            }
            sync_directory(&folder)?;
            sync_directory(&parent_folder)?;
            Ok(folder)
        }
        Err(error) => Err(error),
    }
}

fn create_temporary(parent: &Path, name: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..128 {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = name.to_os_string();
        temporary_name.push(format!(".tmp-{}-{id}", std::process::id()));
        let temporary = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free configuration temporary name",
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_missing_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("absent.toml")).unwrap(), None);
    }

    #[test]
    fn writing_creates_missing_folders_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rustphin/nested/settings.toml");
        write_atomic(&path, b"zoom = 2\n").unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some("zoom = 2\n"));
    }

    #[test]
    fn writing_replaces_the_file_and_leaves_nothing_else_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some("new"));
        assert_eq!(names(dir.path()), ["settings.toml"]);
    }

    #[test]
    fn a_failed_write_keeps_the_old_file_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        write_atomic(&path, b"old").unwrap();
        // A folder in the way of the temporary file's final name makes the rename fail.
        let blocked = dir.path().join("blocked.toml");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("occupant"), b"").unwrap();

        assert!(write_atomic(&blocked, b"new").is_err());
        assert_eq!(read(&path).unwrap().as_deref(), Some("old"));
        assert_eq!(names(dir.path()), ["blocked.toml", "settings.toml"]);
    }

    #[test]
    fn a_path_without_a_folder_or_name_is_rejected() {
        assert!(write_atomic(Path::new("/"), b"x").is_err());
    }

    #[test]
    fn writing_does_not_follow_or_remove_an_existing_temporary_link() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let other = dir.path().join("other");
        fs::write(&other, b"keep me").unwrap();
        let link = dir
            .path()
            .join(format!("settings.toml.tmp-{}", std::process::id()));
        std::os::unix::fs::symlink(&other, &link).unwrap();

        write_atomic(&path, b"new settings").unwrap();

        assert_eq!(fs::read(&other).unwrap(), b"keep me");
        assert_eq!(fs::read_link(&link).unwrap(), other);
        assert_eq!(fs::read(&path).unwrap(), b"new settings");
    }

    #[test]
    fn written_configuration_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        write_atomic(&path, b"private settings").unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn a_directory_sync_failure_reports_that_the_whole_file_was_published() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        fs::write(&path, b"old").unwrap();

        let error = write_atomic_with_sync(&path, b"new", &|_| {
            Err(io::Error::other("injected directory sync failure"))
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(error.to_string().contains("was replaced"));
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(names(dir.path()), ["settings.toml"]);
    }

    #[test]
    fn a_new_directory_sync_failure_stops_before_publishing_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new/settings.toml");

        let error = write_atomic_with_sync(&path, b"new", &|_| {
            Err(io::Error::other("injected new-directory sync failure"))
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(!path.exists());
        assert!(names(&dir.path().join("new")).is_empty());
    }

    #[test]
    fn concurrent_configuration_writes_publish_one_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for value in 0..8_u8 {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    write_atomic(path, &vec![value; 65536]).unwrap();
                });
            }
        });
        let contents = fs::read(&path).unwrap();
        assert_eq!(contents.len(), 65536);
        assert!(contents[0] < 8);
        assert!(contents.iter().all(|byte| *byte == contents[0]));
        assert_eq!(names(dir.path()), ["settings.toml"]);
    }
}
