//! Starting Rustphin at login (W-110c, D-031): one entry in the user's
//! autostart folder, `$XDG_CONFIG_HOME/autostart/rustphin.desktop`, that
//! runs Rustphin with `--background`. Nothing else on the system changes.
//! Only an entry Rustphin wrote is ever removed.

use std::{
    io,
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};

/// The entry's file name.
const FILE: &str = "rustphin.desktop";
/// The line that marks an entry as Rustphin's own.
const MARK: &str = "X-Rustphin-Autostart=true";

/// Where the entry lives under `config_home`.
pub fn entry_path(config_home: &Path) -> PathBuf {
    config_home.join("autostart").join(FILE)
}

/// `program` quoted for a desktop entry's `Exec` key: in double quotes,
/// with `"`, `` ` ``, `$` and `\` escaped, as the Desktop Entry
/// Specification asks; `%` doubled, since it starts a field code.
fn quoted(program: &Path) -> String {
    let text = String::from_utf8_lossy(program.as_os_str().as_bytes());
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' | '`' | '$' | '\\' => {
                // The value is itself unescaped once as a string first.
                quoted.push_str("\\\\");
                quoted.push(character);
            }
            '%' => quoted.push_str("%%"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// The entry that starts `program` in the background at login.
pub fn entry(program: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Rustphin\n\
         Comment=Keeps Rustphin ready so its windows open at once\n\
         Exec={} --background\n\
         Icon=rustphin\n\
         Terminal=false\n\
         NoDisplay=true\n\
         {MARK}\n",
        quoted(program)
    )
}

/// Whether the file at `path` is an entry Rustphin wrote.
fn is_ours(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|text| text.lines().any(|line| line == MARK))
}

/// Adds the entry for `program` when `enabled`, or removes Rustphin's own
/// entry when not. An entry of the same name that someone else wrote is
/// left alone and reported as [`io::ErrorKind::AlreadyExists`].
pub fn set(config_home: &Path, program: &Path, enabled: bool) -> io::Result<()> {
    let path = entry_path(config_home);
    let exists = path.exists();
    if exists && !is_ours(&path) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} was not written by Rustphin", path.display()),
        ));
    }
    if enabled {
        crate::config::write_atomic(&path, entry(program).as_bytes())
    } else if exists {
        std::fs::remove_file(&path)
    } else {
        Ok(())
    }
}

/// [`set`] for the program now running, in the user's configuration
/// folder.
pub fn set_for_this_program(enabled: bool) -> io::Result<()> {
    let config_home = crate::xdg::config_home()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no configuration folder"))?;
    let program = std::env::current_exe()?;
    set(&config_home, &program, enabled)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_starts_the_program_in_the_background_with_a_safe_path() {
        let text = entry(Path::new("/home/some one/bin/rust\"phin$%"));
        assert!(text.contains("Exec=\"/home/some one/bin/rust\\\\\"phin\\\\$%%\" --background\n"));
        assert!(text.contains("NoDisplay=true\n"));
        assert!(text.lines().any(|line| line == MARK));
    }

    #[test]
    fn turning_it_on_and_off_adds_and_removes_one_entry() {
        let home = tempfile::tempdir().unwrap();
        let program = Path::new("/usr/local/bin/rustphin");
        set(home.path(), program, true).unwrap();
        let path = entry_path(home.path());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), entry(program));
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
        set(home.path(), program, false).unwrap();
        assert!(!path.exists());
        // Off again when already off is not an error.
        set(home.path(), program, false).unwrap();
    }

    #[test]
    fn an_entry_someone_else_wrote_is_never_touched() {
        let home = tempfile::tempdir().unwrap();
        let path = entry_path(home.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[Desktop Entry]\nExec=my-own-script\n").unwrap();
        for enabled in [true, false] {
            let error = set(home.path(), Path::new("/bin/rustphin"), enabled).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[Desktop Entry]\nExec=my-own-script\n"
        );
    }
}
