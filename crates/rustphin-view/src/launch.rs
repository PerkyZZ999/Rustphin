//! Command-line folders. Nothing here reads the filesystem: a path that
//! names a file is still returned as given, and the caller decides whether
//! to open its parent.

use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use rustphin_platform::uri::path_from_uri;

/// What a launch asks for (W-110, D-031).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open a window, or the folders named.
    Open,
    /// `--background`: start without a window, to stay running.
    Background,
    /// `--quit`: end the running Rustphin.
    Quit,
}

/// A launch: its command and the folders it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// What to do.
    pub command: Command,
    /// The folders named, as [`launch_targets`] reads them.
    pub targets: Vec<PathBuf>,
}

/// Reads the command line: `--quit` and `--background` before the paths,
/// `--` ending the options so a folder may start with dashes. Another
/// argument starting with `--` is ignored with a warning.
pub fn parse_launch<I, S>(args: I, cwd: &Path) -> Launch
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::Open;
    let mut paths = Vec::new();
    let mut options_ended = false;
    for arg in args.into_iter().skip(1) {
        let arg = arg.as_ref();
        if !options_ended && arg.as_bytes().starts_with(b"--") {
            match arg.as_bytes() {
                b"--" => options_ended = true,
                b"--quit" => command = Command::Quit,
                b"--background" => command = Command::Background,
                _ => tracing::warn!(option = %arg.to_string_lossy(), "ignoring an unknown option"),
            }
            continue;
        }
        paths.push(arg.to_os_string());
    }
    let targets = paths.iter().filter_map(|arg| target(arg, cwd)).collect();
    Launch { command, targets }
}

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
    fn options_are_read_and_dashes_after_a_double_dash_are_a_path() {
        let cwd = Path::new("/home/someone");
        let quit = parse_launch(["rustphin", "--quit"], cwd);
        assert_eq!(quit.command, Command::Quit);
        assert!(quit.targets.is_empty());
        let background = parse_launch(["rustphin", "--background", "/srv"], cwd);
        assert_eq!(background.command, Command::Background);
        assert_eq!(background.targets, [PathBuf::from("/srv")]);
        let dashes = parse_launch(["rustphin", "--", "--quit"], cwd);
        assert_eq!(dashes.command, Command::Open);
        assert_eq!(dashes.targets, [PathBuf::from("/home/someone/--quit")]);
        let unknown = parse_launch(["rustphin", "--fast", "docs"], cwd);
        assert_eq!(unknown.command, Command::Open);
        assert_eq!(unknown.targets, [PathBuf::from("/home/someone/docs")]);
    }

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

    #[test]
    fn the_desktop_entry_names_the_bundled_icon() {
        // KWin finds the window's icon through the entry matching the app id `rustphin`;
        // the theme icon of that name is installed from assets/icons and assets/rustphin.svg.
        let text = include_str!("../../../assets/rustphin.desktop");
        assert!(text.lines().any(|line| line == "Icon=rustphin"));
    }
}
