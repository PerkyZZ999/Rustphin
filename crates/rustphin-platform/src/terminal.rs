//! Opening the user's terminal in a folder ("Open Terminal Here", W-108).
//!
//! The terminal is the one chosen in KDE's settings (`TerminalApplication`
//! in `kdeglobals`), Konsole when none is. As Dolphin does, it starts with the
//! folder as its working directory. Terminals that run as one instance open
//! their new window where the running instance is, so the folder is also
//! passed with the option each known terminal understands.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use crate::apps::{LaunchError, spawn, split_exec};

/// The terminal command chosen in the KDE settings under `config_home`, as
/// words, or Konsole.
pub fn configured(config_home: Option<&Path>) -> Vec<String> {
    config_home
        .and_then(|home| std::fs::read_to_string(home.join("kdeglobals")).ok())
        .and_then(|text| terminal_in(&text))
        .unwrap_or_else(|| vec!["konsole".to_owned()])
}

/// The `TerminalApplication` of the `[General]` group, split into words.
fn terminal_in(kdeglobals: &str) -> Option<Vec<String>> {
    let mut in_general = false;
    for line in kdeglobals.lines().map(str::trim) {
        if line.starts_with('[') {
            in_general = line == "[General]";
        } else if in_general && let Some(value) = line.strip_prefix("TerminalApplication=") {
            return split_exec(value.trim()).filter(|words| !words.is_empty());
        }
    }
    None
}

/// The program and arguments that open `words`'s terminal in `folder`.
pub fn command_in(words: &[String], folder: &Path) -> Option<(OsString, Vec<OsString>)> {
    let (program, args) = words.split_first()?;
    let name = Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let with_value = |flag: &str| {
        let mut joined = OsString::from(flag);
        joined.push(folder);
        joined
    };
    match name.as_str() {
        "konsole" => args.extend([OsString::from("--workdir"), folder.into()]),
        "ghostty" => args.push(with_value("--working-directory=")),
        "kitty" => args.extend([OsString::from("--directory"), folder.into()]),
        "alacritty" => args.extend([OsString::from("--working-directory"), folder.into()]),
        "wezterm" => args.extend([
            OsString::from("start"),
            OsString::from("--cwd"),
            folder.into(),
        ]),
        "gnome-terminal" | "kgx" | "ptyxis" => args.push(with_value("--working-directory=")),
        "foot" | "footclient" => args.push(with_value("--working-directory=")),
        // Any other terminal starts in its working directory.
        _ => {}
    }
    Some((program.into(), args))
}

/// Opens the configured terminal in `folder`. Reads the settings: call
/// from a background thread.
pub fn open_in(folder: &Path, config_home: Option<PathBuf>) -> Result<(), LaunchError> {
    let words = configured(config_home.as_deref());
    let Some((program, args)) = command_in(&words, folder) else {
        return Err(LaunchError::NoCommand(String::new()));
    };
    let mut command = Command::new(&program);
    command.args(args).current_dir(folder);
    spawn(command, &program.to_string_lossy())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split(' ').map(str::to_owned).collect()
    }

    fn args(words: &[String], folder: &str) -> Vec<String> {
        let (_, args) = command_in(words, Path::new(folder)).unwrap();
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_terminal_is_read_from_kdes_general_settings_or_is_konsole() {
        let kdeglobals = "[Icons]\nTheme=breeze\n\n[General]\nTerminalApplication=/usr/bin/ghostty --gtk-single-instance=true\nTerminalService=com.mitchellh.ghostty.desktop\n";
        assert_eq!(
            terminal_in(kdeglobals),
            Some(words("/usr/bin/ghostty --gtk-single-instance=true"))
        );
        // The same key elsewhere does not count.
        assert_eq!(terminal_in("[Other]\nTerminalApplication=xterm\n"), None);
        assert_eq!(terminal_in("[General]\nTerminalApplication=\n"), None);
        let none = tempfile::tempdir().unwrap();
        assert_eq!(configured(Some(none.path())), words("konsole"));
        assert_eq!(configured(None), words("konsole"));
    }

    #[test]
    fn known_terminals_are_told_the_folder_and_others_start_in_it() {
        let folder = "/home/u/my projects";
        assert_eq!(args(&words("konsole"), folder), ["--workdir", folder]);
        assert_eq!(
            args(
                &words("/usr/bin/ghostty --gtk-single-instance=true"),
                folder
            ),
            [
                "--gtk-single-instance=true".to_owned(),
                format!("--working-directory={folder}")
            ]
        );
        assert_eq!(args(&words("kitty"), folder), ["--directory", folder]);
        assert_eq!(args(&words("wezterm"), folder), ["start", "--cwd", folder]);
        assert!(args(&words("xterm"), folder).is_empty());
        assert_eq!(command_in(&[], Path::new(folder)), None);
    }
}
