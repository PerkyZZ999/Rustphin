//! KDE's shared settings file, `kdeglobals`, read as Dolphin reads it: the
//! terminal to open and whether one click opens a file (W-106c).

use std::path::Path;

/// The text of `kdeglobals` under `config_home`, if there is one.
pub fn read(config_home: Option<&Path>) -> Option<String> {
    config_home.and_then(|home| std::fs::read_to_string(home.join("kdeglobals")).ok())
}

/// The value of `key` in `[group]`, trimmed. Later groups of the same name
/// count too; the first value found wins, as in the file KDE writes.
pub fn value<'a>(text: &'a str, group: &str, key: &str) -> Option<&'a str> {
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[') {
            inside = name.strip_suffix(']') == Some(group);
        } else if inside
            && let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            return Some(value.trim());
        }
    }
    None
}

/// Whether KDE is set to open files and folders with a single click
/// (`SingleClick` in `[KDE]`). Plasma's default is a double click.
pub fn single_click(text: &str) -> bool {
    value(text, "KDE", "SingleClick") == Some("true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_found_only_in_its_own_group() {
        let text = "[General]\nTerminalApplication = ghostty \n[KDE]\nSingleClick=true\n";
        assert_eq!(
            value(text, "General", "TerminalApplication"),
            Some("ghostty")
        );
        assert_eq!(value(text, "KDE", "TerminalApplication"), None);
        assert_eq!(value(text, "Missing", "SingleClick"), None);
    }

    #[test]
    fn one_click_opens_only_when_kde_says_so() {
        assert!(single_click("[KDE]\nSingleClick=true\n"));
        assert!(!single_click("[KDE]\nSingleClick=false\n"));
        assert!(!single_click("[General]\nSingleClick=true\n"));
        assert!(!single_click(""));
    }
}
