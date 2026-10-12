//! Reading and writing Rustphin's own configuration files.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
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

/// Replaces `path` with `contents` so that a crash at any point leaves either
/// the old file or the new one, never a partial one: the data is written to a
/// temporary file in the same folder, flushed to disk, then renamed over
/// `path`. Missing parent folders are created.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let name = path.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(parent)?;

    let mut temporary_name = name.to_os_string();
    temporary_name.push(format!(".tmp-{}", std::process::id()));
    let temporary = parent.join(temporary_name);
    let written = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if written.is_err() {
        // Best effort: the error being returned matters more than the leftover.
        let _ = fs::remove_file(&temporary);
        return written;
    }
    // Make the rename itself durable. Not every filesystem supports syncing a
    // folder, and the file is already in place, so a failure here is ignored.
    if let Ok(folder) = File::open(parent) {
        let _ = folder.sync_all();
    }
    Ok(())
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
}
