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
    crate::kdeglobals::read(config_home)
        .and_then(|text| terminal_in(&text))
        .unwrap_or_else(|| vec!["konsole".to_owned()])
}

/// The `TerminalApplication` of the `[General]` group, split into words.
fn terminal_in(kdeglobals: &str) -> Option<Vec<String>> {
    crate::kdeglobals::value(kdeglobals, "General", "TerminalApplication").and_then(words)
}

/// A terminal command line split into words as a desktop file's `Exec` is;
/// `None` when it is empty or its quotes do not close.
pub fn words(command: &str) -> Option<Vec<String>> {
    split_exec(command.trim()).filter(|words| !words.is_empty())
}

/// How a terminal is named to the user: its program's file name with a
/// capital, "Ghostty" for `/usr/bin/ghostty --gtk-single-instance=true`.
pub fn display_name(words: &[String]) -> String {
    let program = words.first().map(String::as_str).unwrap_or_default();
    let name = Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
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

/// Opens the terminal in `folder`: `command` when Rustphin's settings name
/// one, else the one KDE is set to. Reads the settings: call from a
/// background thread.
pub fn open_in(
    folder: &Path,
    command: Option<&str>,
    config_home: Option<PathBuf>,
) -> Result<(), LaunchError> {
    let words = match command {
        Some(command) => {
            words(command).ok_or_else(|| LaunchError::NoCommand(command.to_owned()))?
        }
        None => configured(config_home.as_deref()),
    };
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
    fn a_terminal_is_named_by_its_program() {
        assert_eq!(
            display_name(&words("/usr/bin/ghostty --gtk-single-instance=true")),
            "Ghostty"
        );
        assert_eq!(display_name(&words("konsole")), "Konsole");
        assert_eq!(display_name(&[]), "");
    }

    #[test]
    fn a_chosen_command_is_split_and_an_unclosed_quote_is_refused() {
        assert_eq!(
            super::words("  kitty --single-instance "),
            Some(words("kitty --single-instance"))
        );
        assert_eq!(super::words("\"foot"), None);
        assert_eq!(super::words("   "), None);
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
