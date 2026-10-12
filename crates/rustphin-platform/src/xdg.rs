//! Locations defined by the XDG Base Directory specification.

use std::{env, path::PathBuf};

/// The user's home directory, if the environment names one.
pub fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

/// Directories searched for shared data, most important first: the user's own
/// data directory, then the system ones.
pub fn data_dirs() -> Vec<PathBuf> {
    let absolute = |path: PathBuf| path.is_absolute().then_some(path);
    let mut dirs = Vec::new();
    match env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .and_then(absolute)
    {
        Some(dir) => dirs.push(dir),
        None => dirs.extend(home().map(|home| home.join(".local/share"))),
    }
    let system = env::var_os("XDG_DATA_DIRS")
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(env::split_paths(&system).filter_map(absolute));
    dirs
}

/// The directory holding the user's configuration files.
pub fn config_home() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(".config")))
}
