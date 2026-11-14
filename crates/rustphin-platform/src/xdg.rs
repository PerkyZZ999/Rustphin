//! Locations defined by the XDG Base Directory specification.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// The user's home directory, if the environment names one.
pub fn home() -> Option<PathBuf> {
    from_env().home()
}

/// The user's own data directory (`$XDG_DATA_HOME`, else `~/.local/share`).
pub fn data_home() -> Option<PathBuf> {
    from_env().data_home()
}

/// Directories searched for shared data, most important first: the user's own
/// data directory, then the system ones.
pub fn data_dirs() -> Vec<PathBuf> {
    from_env().data_dirs()
}

/// Directories searched for configuration, most important first: the user's
/// own config directory, then the system ones.
pub fn config_dirs() -> Vec<PathBuf> {
    from_env().config_dirs()
}

/// The directory holding the user's configuration files.
pub fn config_home() -> Option<PathBuf> {
    from_env().config_home()
}

/// The directory holding the user's state files (session data, history).
pub fn state_home() -> Option<PathBuf> {
    from_env().state_home()
}

/// The directory holding the user's caches.
pub fn cache_home() -> Option<PathBuf> {
    from_env().cache_home()
}

/// The locations this process's environment gives.
fn from_env() -> Locations<impl Fn(&str) -> Option<OsString>> {
    Locations(|name: &str| env::var_os(name))
}

/// The XDG locations an environment gives, read through `var`. Only
/// absolute paths count, as the specification asks: a relative or empty
/// value is ignored.
struct Locations<F>(F);

impl<F: Fn(&str) -> Option<OsString>> Locations<F> {
    fn absolute(&self, name: &str) -> Option<PathBuf> {
        (self.0)(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    }

    fn home(&self) -> Option<PathBuf> {
        self.absolute("HOME")
    }

    /// `$name`, else `fallback` inside the home directory.
    fn user_dir(&self, name: &str, fallback: &str) -> Option<PathBuf> {
        self.absolute(name)
            .or_else(|| self.home().map(|home| home.join(fallback)))
    }

    fn data_home(&self) -> Option<PathBuf> {
        self.user_dir("XDG_DATA_HOME", ".local/share")
    }

    fn config_home(&self) -> Option<PathBuf> {
        self.user_dir("XDG_CONFIG_HOME", ".config")
    }

    fn state_home(&self) -> Option<PathBuf> {
        self.user_dir("XDG_STATE_HOME", ".local/state")
    }

    fn cache_home(&self) -> Option<PathBuf> {
        self.user_dir("XDG_CACHE_HOME", ".cache")
    }

    /// The absolute folders of the list `$name`, else of `fallback`.
    fn system_dirs(&self, name: &str, fallback: &str) -> Vec<PathBuf> {
        let list = (self.0)(name)
            .filter(|list| !list.is_empty())
            .unwrap_or_else(|| fallback.into());
        env::split_paths(&list)
            .filter(|path| Path::is_absolute(path))
            .collect()
    }

    fn data_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self.data_home().into_iter().collect();
        dirs.extend(self.system_dirs("XDG_DATA_DIRS", "/usr/local/share:/usr/share"));
        dirs
    }

    fn config_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self.config_home().into_iter().collect();
        for dir in self.system_dirs("XDG_CONFIG_DIRS", "/etc/xdg") {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The locations an environment of `vars` gives.
    fn with(vars: &[(&str, &str)]) -> Locations<impl Fn(&str) -> Option<OsString>> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        Locations(move |name: &str| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.clone())
        })
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_users_folders_default_to_the_home_directory() {
        let xdg = with(&[("HOME", "/home/u")]);

        assert_eq!(xdg.home(), Some("/home/u".into()));
        assert_eq!(xdg.data_home(), Some("/home/u/.local/share".into()));
        assert_eq!(xdg.config_home(), Some("/home/u/.config".into()));
        assert_eq!(xdg.state_home(), Some("/home/u/.local/state".into()));
        assert_eq!(xdg.cache_home(), Some("/home/u/.cache".into()));
    }

    #[test]
    fn an_absolute_xdg_variable_is_used_and_a_relative_one_ignored() {
        let xdg = with(&[
            ("HOME", "/home/u"),
            ("XDG_DATA_HOME", "/data"),
            ("XDG_CONFIG_HOME", "/config"),
            ("XDG_STATE_HOME", "relative/state"),
            ("XDG_CACHE_HOME", ""),
        ]);

        assert_eq!(xdg.data_home(), Some("/data".into()));
        assert_eq!(xdg.config_home(), Some("/config".into()));
        assert_eq!(xdg.state_home(), Some("/home/u/.local/state".into()));
        assert_eq!(xdg.cache_home(), Some("/home/u/.cache".into()));
    }

    #[test]
    fn without_a_usable_home_there_are_no_users_folders() {
        for home in [None, Some(""), Some("relative/home")] {
            let vars: Vec<(&str, &str)> = home.map(|home| ("HOME", home)).into_iter().collect();
            let xdg = with(&vars);

            assert_eq!(xdg.home(), None, "{home:?}");
            assert_eq!(xdg.data_home(), None, "{home:?}");
            assert_eq!(xdg.cache_home(), None, "{home:?}");
            assert_eq!(xdg.data_dirs(), paths(&["/usr/local/share", "/usr/share"]));
        }
    }

    #[test]
    fn system_folders_follow_the_users_and_only_absolute_ones_count() {
        let xdg = with(&[
            ("HOME", "/home/u"),
            ("XDG_DATA_DIRS", "/opt/share:relative:/usr/share"),
            ("XDG_CONFIG_DIRS", ""),
        ]);

        assert_eq!(
            xdg.data_dirs(),
            paths(&["/home/u/.local/share", "/opt/share", "/usr/share"])
        );
        assert_eq!(xdg.config_dirs(), paths(&["/home/u/.config", "/etc/xdg"]));
    }

    #[test]
    fn a_config_folder_named_twice_is_searched_once() {
        let xdg = with(&[
            ("XDG_CONFIG_HOME", "/etc/xdg"),
            ("XDG_CONFIG_DIRS", "/etc/xdg:/opt/xdg"),
        ]);

        assert_eq!(xdg.config_dirs(), paths(&["/etc/xdg", "/opt/xdg"]));
    }
}
