//! Command-line folders. Nothing here reads the filesystem: a path that
//! names a file is still returned as given, and the caller decides whether
//! to open its parent.

use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use rustphin_platform::uri::path_from_uri;

/// The folders named by `args`, skipping the program name.
///
/// A `file:` URL becomes its local path. Anything else is a path, and a
/// relative one is joined to `cwd`. A URL that does not name a local file
/// is left out.
pub fn launch_targets<I, S>(args: I, cwd: &Path) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .skip(1)
        .filter_map(|arg| target(arg.as_ref(), cwd))
        .collect()
}

fn target(arg: &OsStr, cwd: &Path) -> Option<PathBuf> {
    let bytes = arg.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() >= 5 && bytes[..5].eq_ignore_ascii_case(b"file:") {
        return match path_from_uri(bytes) {
            Some(path) => Some(path),
            None => {
                tracing::warn!("ignoring a file URL that does not name a local path");
                None
            }
        };
    }
    let path = PathBuf::from(arg);
    Some(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_opens_nothing_explicit() {
        assert!(launch_targets(["rustphin"], Path::new("/cwd")).is_empty());
    }

    #[test]
    fn paths_stay_in_order_and_relative_ones_join_the_working_directory() {
        let targets = launch_targets(
            ["rustphin", "/srv/one", "two", "/srv/three"],
            Path::new("/cwd"),
        );
        assert_eq!(
            targets,
            [
                PathBuf::from("/srv/one"),
                PathBuf::from("/cwd/two"),
                PathBuf::from("/srv/three")
            ]
        );
    }

    #[test]
    fn file_urls_become_local_paths_and_other_hosts_are_left_out() {
        let targets = launch_targets(
            [
                "rustphin",
                "file:///srv/docs",
                "FILE://localhost/srv/%C3%89t%C3%A9",
                "file://files.example/srv/remote",
            ],
            Path::new("/cwd"),
        );
        assert_eq!(
            targets,
            [
                PathBuf::from("/srv/docs"),
                PathBuf::from("/srv/\u{c9}t\u{e9}")
            ]
        );
    }

    #[test]
    fn the_desktop_entry_handles_folders_and_passes_urls() {
        let text = include_str!("../../../assets/rustphin.desktop");
        assert!(text.contains("Type=Application"));
        assert!(text.contains("MimeType=inode/directory;"));
        assert!(text.contains("Exec=rustphin %U"));
        assert!(text.contains("StartupWMClass=rustphin"));
    }
}
