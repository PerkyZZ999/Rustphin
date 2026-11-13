//! Rustphin, a file manager for KDE Plasma on Wayland.

use std::{
    env,
    path::{Path, PathBuf},
};

use tracing_subscriber::EnvFilter;

fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("RUSTPHIN_LOG").unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();

    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    // Each path is a tab. With none, the previous session's tabs come back.
    // A path that names a file opens the folder that contains it.
    let launch = rustphin_ui::parse_launch(env::args_os(), &cwd);
    rustphin_ui::start(rustphin_ui::Start {
        command: launch.command,
        starts: launch.targets.into_iter().map(folder_tab).collect(),
        home,
        force_opaque: env::var_os("RUSTPHIN_OPAQUE").is_some_and(|value| value != "0"),
    })
}

/// A directory stays as it is. An existing file becomes the folder around it,
/// so a tab always has a directory to list.
fn folder_tab(path: PathBuf) -> PathBuf {
    let is_file = std::fs::metadata(&path).is_ok_and(|meta| meta.is_file());
    if is_file {
        path.parent()
            .map(Path::to_path_buf)
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(path)
    } else {
        path
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_file_opens_the_folder_that_contains_it() {
        let dir = std::env::temp_dir().join(format!("rustphin-launch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");
        std::fs::write(&file, b"note").unwrap();
        let linked = dir.join("linked-dir");
        std::os::unix::fs::symlink(&dir, &linked).unwrap();

        assert_eq!(folder_tab(file), dir);
        assert_eq!(folder_tab(dir.clone()), dir);
        assert_eq!(folder_tab(linked), dir.join("linked-dir"));
        assert_eq!(
            folder_tab(PathBuf::from("/no/such/rustphin-folder")),
            PathBuf::from("/no/such/rustphin-folder")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
