//! What the "New" menu can create, as Dolphin's "Create New" menu (W-108).
//!
//! KDE's built-in entries (a text file, an empty file, an HTML file), then
//! the user's templates folder (`XDG_TEMPLATES_DIR`, `~/Templates`): each
//! file there is offered under its own name. Then template entries in the
//! `templates` folders of the data directories (`.desktop` files naming a
//! file beside them), as KIO reads them. Links (to a location, a file or an
//! application) are not offered yet.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use rustphin_core::ops::Template;

/// One thing the menu can create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    /// The words in the menu, without the dots ("Text File").
    pub label: String,
    /// The name the new file gets before the user renames it.
    pub file_name: String,
    /// What the file starts with.
    pub template: Template,
    /// Which built-in entry it is, for its icon; `None` for the user's.
    pub built_in: Option<BuiltIn>,
}

/// KDE's own entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn {
    /// A plain text file.
    Text,
    /// A file with nothing in it.
    Empty,
    /// An HTML page.
    Html,
}

/// What a new HTML file holds: a minimal valid page.
const HTML: &str = "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title></title>\n</head>\n<body>\n\n</body>\n</html>\n";

/// KDE's built-in entries, in its order.
pub fn built_in() -> Vec<NewFile> {
    vec![
        NewFile {
            label: "Text File".into(),
            file_name: "Text File.txt".into(),
            template: Template::Contents(Vec::new()),
            built_in: Some(BuiltIn::Text),
        },
        NewFile {
            label: "Empty File".into(),
            file_name: "Empty File".into(),
            template: Template::Contents(Vec::new()),
            built_in: Some(BuiltIn::Empty),
        },
        NewFile {
            label: "HTML File".into(),
            file_name: "HTML File.html".into(),
            template: Template::Contents(HTML.as_bytes().to_vec()),
            built_in: Some(BuiltIn::Html),
        },
    ]
}

/// Everything the menu offers: the built-in entries, the files in
/// `templates_dir` and the template entries under `data_dirs`. Reads the
/// disk: call from a background thread.
pub fn load(templates_dir: Option<&Path>, data_dirs: &[PathBuf]) -> Vec<NewFile> {
    let mut all = built_in();
    let mut seen: HashSet<String> = all.iter().map(|entry| entry.label.clone()).collect();
    let mut add = |entry: NewFile, all: &mut Vec<NewFile>| {
        if seen.insert(entry.label.clone()) {
            all.push(entry);
        }
    };
    if let Some(dir) = templates_dir {
        let mut files: Vec<NewFile> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter_map(|entry| {
                let file_name = entry.file_name().into_string().ok()?;
                if file_name.starts_with('.') {
                    return None;
                }
                Some(NewFile {
                    label: stem(&file_name).to_owned(),
                    file_name,
                    template: Template::File(entry.path()),
                    built_in: None,
                })
            })
            .collect();
        files.sort_by_key(|file| file.label.to_lowercase());
        for file in files {
            add(file, &mut all);
        }
    }
    for dir in data_dirs {
        let folder = dir.join("templates");
        let mut entries: Vec<NewFile> = fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "desktop"))
            .filter_map(|entry| {
                let text = fs::read_to_string(entry.path()).ok()?;
                desktop_template(&text, &folder)
            })
            .collect();
        entries.sort_by_key(|entry| entry.label.to_lowercase());
        for entry in entries {
            add(entry, &mut all);
        }
    }
    all
}

/// The user's templates folder from `user-dirs.dirs` under `config_home`,
/// or `~/Templates`. KIO ignores a templates folder set to the home folder
/// itself, and so does this.
pub fn templates_dir(home: &Path, config_home: Option<&Path>) -> Option<PathBuf> {
    let configured = config_home
        .and_then(|config| fs::read_to_string(config.join("user-dirs.dirs")).ok())
        .and_then(|text| {
            text.lines().find_map(|line| {
                let value = line.trim().strip_prefix("XDG_TEMPLATES_DIR=")?;
                let value = value.trim().trim_matches('"');
                Some(match value.strip_prefix("$HOME") {
                    Some(rest) => home.join(rest.trim_start_matches('/')),
                    None => PathBuf::from(value),
                })
            })
        });
    let dir = configured.unwrap_or_else(|| home.join("Templates"));
    (dir != home).then_some(dir)
}

/// A template entry: `Name` and a `URL` naming a regular file beside it.
fn desktop_template(text: &str, folder: &Path) -> Option<NewFile> {
    let mut in_entry = false;
    let (mut name, mut url, mut hidden) = (None, None, false);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry {
            if let Some(value) = line.strip_prefix("Name=") {
                name = Some(
                    value
                        .trim()
                        .trim_end_matches('…')
                        .trim_end_matches("...")
                        .trim()
                        .to_owned(),
                );
            } else if let Some(value) = line.strip_prefix("URL=") {
                url = Some(value.trim().to_owned());
            } else if line == "Hidden=true" || line == "NoDisplay=true" {
                hidden = true;
            }
        }
    }
    let (name, url) = (name.filter(|name| !name.is_empty())?, url?);
    if hidden || url.ends_with(".desktop") {
        // Links are made by asking for a target, which is not offered yet.
        return None;
    }
    let source = folder.join(&url);
    if !fs::metadata(&source).is_ok_and(|metadata| metadata.is_file()) {
        return None;
    }
    let file_name = source.file_name()?.to_str()?.to_owned();
    Some(NewFile {
        label: name,
        file_name,
        template: Template::File(source),
        built_in: None,
    })
}

/// A file name without its extension; a leading dot is part of the name.
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

/// A free name for a new file: `name` itself, else "Text File 2.txt",
/// "Text File 3.txt", … keeping the extension.
pub fn free_file_name(name: &str, is_taken: impl Fn(&str) -> bool) -> String {
    if !is_taken(name) {
        return name.to_owned();
    }
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    };
    (2..)
        .map(|n| format!("{stem} {n}{extension}"))
        .find(|candidate| !is_taken(candidate))
        .unwrap_or_else(|| name.to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn labels(entries: &[NewFile]) -> Vec<&str> {
        entries.iter().map(|entry| entry.label.as_str()).collect()
    }

    #[test]
    fn kdes_own_entries_come_first_and_the_html_one_is_a_page() {
        let entries = built_in();
        assert_eq!(labels(&entries), ["Text File", "Empty File", "HTML File"]);
        assert_eq!(entries[0].file_name, "Text File.txt");
        let Template::Contents(html) = &entries[2].template else {
            panic!("built in");
        };
        assert!(String::from_utf8_lossy(html).starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn the_users_template_files_and_desktop_templates_follow() {
        let root = tempfile::tempdir().unwrap();
        let templates = root.path().join("Templates");
        fs::create_dir_all(&templates).unwrap();
        fs::write(templates.join("Letter.odt"), b"letter").unwrap();
        fs::write(templates.join(".hidden"), b"").unwrap();
        fs::create_dir(templates.join("a folder")).unwrap();
        let data = root.path().join("share");
        let system = data.join("templates");
        fs::create_dir_all(system.join(".source")).unwrap();
        fs::write(system.join(".source/Sheet.ods"), b"sheet").unwrap();
        fs::write(
            system.join("Sheet.desktop"),
            "[Desktop Entry]\nName=Spreadsheet…\nURL=.source/Sheet.ods\nType=Link\n",
        )
        .unwrap();
        fs::write(
            system.join("linkURL.desktop"),
            "[Desktop Entry]\nName=Link to Location…\nURL=.source/URL.desktop\nType=Link\n",
        )
        .unwrap();
        fs::write(
            system.join("Gone.desktop"),
            "[Desktop Entry]\nName=Gone\nURL=.source/missing.txt\n",
        )
        .unwrap();
        let entries = load(Some(&templates), &[data]);
        assert_eq!(
            labels(&entries),
            [
                "Text File",
                "Empty File",
                "HTML File",
                "Letter",
                "Spreadsheet"
            ]
        );
        assert_eq!(entries[3].file_name, "Letter.odt");
        assert_eq!(
            entries[3].template,
            Template::File(templates.join("Letter.odt"))
        );
        assert_eq!(entries[4].file_name, "Sheet.ods");
    }

    #[test]
    fn the_templates_folder_comes_from_user_dirs_and_never_is_home() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let config = root.path().join("config");
        fs::create_dir_all(&config).unwrap();
        assert_eq!(
            templates_dir(&home, Some(&config)),
            Some(home.join("Templates"))
        );
        fs::write(
            config.join("user-dirs.dirs"),
            "XDG_TEMPLATES_DIR=\"$HOME/Modèles\"\n",
        )
        .unwrap();
        assert_eq!(
            templates_dir(&home, Some(&config)),
            Some(home.join("Modèles"))
        );
        fs::write(
            config.join("user-dirs.dirs"),
            "XDG_TEMPLATES_DIR=\"$HOME/\"\n",
        )
        .unwrap();
        assert_eq!(templates_dir(&home, Some(&config)), None);
    }

    #[test]
    fn a_taken_name_gets_a_number_before_its_extension() {
        let taken = ["Text File.txt", "Text File 2.txt", "Empty File"];
        let free = |name: &str| free_file_name(name, |candidate| taken.contains(&candidate));
        assert_eq!(free("Text File.txt"), "Text File 3.txt");
        assert_eq!(free("Empty File"), "Empty File 2");
        assert_eq!(free("HTML File.html"), "HTML File.html");
        assert_eq!(free(".bashrc"), ".bashrc");
    }
}
