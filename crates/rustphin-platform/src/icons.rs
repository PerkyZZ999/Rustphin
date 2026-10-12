//! Icon lookup in freedesktop icon themes.
//!
//! Follows the lookup of the Icon Theme specification: a theme, then the
//! themes it inherits, then `hicolor`; within each, a directory whose size
//! fits is preferred and otherwise the closest one is taken. Only scale-1
//! directories and SVG or PNG files are considered.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// The theme every lookup ends in.
const FALLBACK_THEME: &str = "hicolor";
const EXTENSIONS: [&str; 2] = ["svg", "png"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sizing {
    Fixed,
    Scalable { min: u32, max: u32 },
    Threshold(u32),
}

#[derive(Debug)]
struct ThemeDir {
    path: PathBuf,
    size: u32,
    sizing: Sizing,
    /// Position of the directory's theme in the inheritance chain.
    theme: usize,
}

impl ThemeDir {
    /// How far this directory's icons are from `size`; zero means they fit.
    fn distance(&self, size: u32) -> u32 {
        let (min, max) = match self.sizing {
            Sizing::Fixed => (self.size, self.size),
            Sizing::Scalable { min, max } => (min, max),
            Sizing::Threshold(threshold) => (
                self.size.saturating_sub(threshold),
                self.size.saturating_add(threshold),
            ),
        };
        if size < min {
            min - size
        } else {
            size.saturating_sub(max)
        }
    }
}

/// An icon theme with everything it inherits from.
#[derive(Debug, Default)]
pub struct IconTheme {
    dirs: Vec<ThemeDir>,
    themes: usize,
    cache: Mutex<HashMap<(String, u32), Option<PathBuf>>>,
}

impl IconTheme {
    /// Loads the theme called `name` from the `icons` folder of each data
    /// directory. An unknown theme still yields the `hicolor` fallback.
    pub fn load(name: &str, data_dirs: &[PathBuf]) -> Self {
        let bases: Vec<PathBuf> = data_dirs.iter().map(|dir| dir.join("icons")).collect();
        let mut chain: Vec<String> = Vec::new();
        let mut pending = vec![name.to_owned()];
        let mut dirs = Vec::new();
        while let Some(theme) = pending.pop() {
            // A theme name comes from configuration: it must be a single path component.
            let valid = !theme.is_empty() && !theme.contains(['/', '\\']) && theme != "..";
            if !valid || chain.contains(&theme) {
                continue;
            }
            let Some(index) = bases
                .iter()
                .find_map(|base| fs::read_to_string(base.join(&theme).join("index.theme")).ok())
            else {
                continue;
            };
            let rank = chain.len();
            chain.push(theme.clone());
            let (inherits, sections) = parse_index(&index);
            for (subdir, size, sizing) in sections {
                dirs.extend(
                    bases
                        .iter()
                        .map(|base| base.join(&theme).join(&subdir))
                        .filter(|p| p.is_dir())
                        .map(|path| ThemeDir {
                            path,
                            size,
                            sizing,
                            theme: rank,
                        }),
                );
            }
            // Depth-first, in the order the theme lists its parents.
            pending.extend(inherits.into_iter().rev());
            if pending.is_empty() && !chain.iter().any(|theme| theme == FALLBACK_THEME) {
                pending.push(FALLBACK_THEME.to_owned());
            }
        }
        Self {
            dirs,
            themes: chain.len(),
            cache: Mutex::default(),
        }
    }

    /// Finds the file for the first of `names` the theme has, drawn at `size`
    /// pixels. Results are remembered.
    pub fn find(&self, names: &[String], size: u32) -> Option<PathBuf> {
        names.iter().find_map(|name| self.find_one(name, size))
    }

    fn find_one(&self, name: &str, size: u32) -> Option<PathBuf> {
        let key = (name.to_owned(), size);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(found) = cache.get(&key) {
            return found.clone();
        }
        let found = (0..self.themes).find_map(|theme| {
            let mut candidates: Vec<&ThemeDir> =
                self.dirs.iter().filter(|dir| dir.theme == theme).collect();
            // Closest size first; among equals, the larger directory scales down better.
            candidates.sort_by_key(|dir| (dir.distance(size), u32::MAX - dir.size));
            candidates
                .into_iter()
                .find_map(|dir| icon_file(&dir.path, name))
        });
        cache.insert(key, found.clone());
        found
    }
}

fn icon_file(dir: &Path, name: &str) -> Option<PathBuf> {
    EXTENSIONS
        .iter()
        .map(|extension| dir.join(format!("{name}.{extension}")))
        .find(|path| path.is_file())
}

/// Reads an `index.theme`: the inherited themes and, for each scale-1 icon
/// directory, its path, nominal size and sizing rule.
fn parse_index(index: &str) -> (Vec<String>, Vec<(String, u32, Sizing)>) {
    let mut sections: Vec<(&str, HashMap<&str, &str>)> = Vec::new();
    for line in index.lines().map(str::trim) {
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            sections.push((name, HashMap::new()));
        } else if let (Some((key, value)), Some((_, keys))) =
            (line.split_once('='), sections.last_mut())
        {
            keys.insert(key.trim(), value.trim());
        }
    }
    let list = |value: Option<&&str>| -> Vec<String> {
        value
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let header = sections
        .iter()
        .find(|(name, _)| *name == "Icon Theme")
        .map(|(_, keys)| keys);
    let inherits = list(header.and_then(|keys| keys.get("Inherits")));
    let listed = list(header.and_then(|keys| keys.get("Directories")));

    let mut dirs = Vec::new();
    for (name, keys) in &sections {
        let number = |key: &str| keys.get(key).and_then(|value| value.parse::<u32>().ok());
        // A directory is a relative path inside the theme; never let it climb out.
        let safe = !name.starts_with('/') && !name.split('/').any(|part| part == "..");
        let (Some(size), true, true) = (number("Size"), safe, listed.iter().any(|dir| dir == name))
        else {
            continue;
        };
        if number("Scale").unwrap_or(1) != 1 {
            continue;
        }
        let sizing = match keys.get("Type").copied() {
            Some("Fixed") => Sizing::Fixed,
            Some("Scalable") => Sizing::Scalable {
                min: number("MinSize").unwrap_or(size),
                max: number("MaxSize").unwrap_or(size),
            },
            _ => Sizing::Threshold(number("Threshold").unwrap_or(2)),
        };
        dirs.push(((*name).to_owned(), size, sizing));
    }
    (inherits, dirs)
}

/// The icon theme chosen in the KDE settings under `config_home`, if any.
pub fn system_theme_name(config_home: &Path) -> Option<String> {
    let config = fs::read_to_string(config_home.join("kdeglobals")).ok()?;
    let mut in_icons = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_icons = line == "[Icons]";
        } else if in_icons && let Some(theme) = line.strip_prefix("Theme=") {
            return Some(theme.trim().to_owned()).filter(|theme| !theme.is_empty());
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Writes a theme with the given `index.theme` body and icon files.
    fn theme(root: &Path, name: &str, index: &str, files: &[&str]) {
        let dir = root.join("icons").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.theme"), index).unwrap();
        for file in files {
            let path = dir.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"").unwrap();
        }
    }

    const BASE: &str = "[Icon Theme]\nName=Base\nDirectories=mime/16,mime/32,mime/scalable,mime/16@2x\n\
        [mime/16]\nSize=16\nType=Fixed\n\
        [mime/32]\nSize=32\nType=Fixed\n\
        [mime/scalable]\nSize=64\nType=Scalable\nMinSize=48\nMaxSize=256\n\
        [mime/16@2x]\nSize=16\nScale=2\nType=Fixed\n";

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    fn relative(root: &Path, found: Option<PathBuf>) -> Option<String> {
        found.map(|path| {
            path.strip_prefix(root.join("icons"))
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
    }

    #[test]
    fn prefers_the_directory_whose_size_fits() {
        let root = tempfile::tempdir().unwrap();
        theme(
            root.path(),
            "base",
            BASE,
            &[
                "mime/16/text.svg",
                "mime/32/text.svg",
                "mime/scalable/text.svg",
            ],
        );
        let icons = IconTheme::load("base", &[root.path().to_path_buf()]);

        let find = |size| relative(root.path(), icons.find(&names(&["text"]), size));
        assert_eq!(find(16).as_deref(), Some("base/mime/16/text.svg"));
        assert_eq!(find(32).as_deref(), Some("base/mime/32/text.svg"));
        assert_eq!(find(128).as_deref(), Some("base/mime/scalable/text.svg"));
    }

    #[test]
    fn takes_the_closest_size_when_none_fits() {
        let root = tempfile::tempdir().unwrap();
        theme(
            root.path(),
            "base",
            BASE,
            &["mime/16/text.svg", "mime/32/text.png"],
        );
        let icons = IconTheme::load("base", &[root.path().to_path_buf()]);

        let find = |size| relative(root.path(), icons.find(&names(&["text"]), size));
        assert_eq!(find(22).as_deref(), Some("base/mime/16/text.svg"));
        assert_eq!(find(28).as_deref(), Some("base/mime/32/text.png"));
    }

    #[test]
    fn ignores_directories_for_other_scales() {
        let root = tempfile::tempdir().unwrap();
        theme(root.path(), "base", BASE, &["mime/16@2x/only-hidpi.svg"]);
        let icons = IconTheme::load("base", &[root.path().to_path_buf()]);
        assert_eq!(icons.find(&names(&["only-hidpi"]), 16), None);
    }

    #[test]
    fn falls_back_through_inherited_themes_then_hicolor() {
        let root = tempfile::tempdir().unwrap();
        let child =
            "[Icon Theme]\nInherits=base\nDirectories=mime/16\n[mime/16]\nSize=16\nType=Fixed\n";
        theme(root.path(), "child", child, &["mime/16/own.svg"]);
        theme(
            root.path(),
            "base",
            BASE,
            &["mime/16/parent.svg", "mime/16/own.svg"],
        );
        theme(root.path(), "hicolor", BASE, &["mime/16/last-resort.png"]);
        let icons = IconTheme::load("child", &[root.path().to_path_buf()]);

        let find = |name| relative(root.path(), icons.find(&names(&[name]), 16));
        assert_eq!(find("own").as_deref(), Some("child/mime/16/own.svg"));
        assert_eq!(find("parent").as_deref(), Some("base/mime/16/parent.svg"));
        assert_eq!(
            find("last-resort").as_deref(),
            Some("hicolor/mime/16/last-resort.png")
        );
        assert_eq!(find("nowhere"), None);
    }

    #[test]
    fn tries_names_in_order_across_the_whole_chain() {
        let root = tempfile::tempdir().unwrap();
        let child =
            "[Icon Theme]\nInherits=base\nDirectories=mime/16\n[mime/16]\nSize=16\nType=Fixed\n";
        theme(root.path(), "child", child, &["mime/16/generic.svg"]);
        theme(root.path(), "base", BASE, &["mime/16/specific.svg"]);
        let icons = IconTheme::load("child", &[root.path().to_path_buf()]);

        // The specific icon in a parent beats the generic one in the theme itself.
        let found = icons.find(&names(&["missing", "specific", "generic"]), 16);
        assert_eq!(
            relative(root.path(), found).as_deref(),
            Some("base/mime/16/specific.svg")
        );
    }

    #[test]
    fn survives_inheritance_cycles_and_unknown_or_unsafe_theme_names() {
        let root = tempfile::tempdir().unwrap();
        let looped = |parent: &str| {
            format!(
                "[Icon Theme]\nInherits={parent}\nDirectories=mime/16\n[mime/16]\nSize=16\nType=Fixed\n"
            )
        };
        theme(root.path(), "a", &looped("b"), &["mime/16/in-a.svg"]);
        theme(
            root.path(),
            "b",
            &looped("a,../a,missing"),
            &["mime/16/in-b.svg"],
        );
        let dirs = [root.path().to_path_buf()];

        let icons = IconTheme::load("a", &dirs);
        assert!(icons.find(&names(&["in-b"]), 16).is_some());
        assert_eq!(
            IconTheme::load("no-such-theme", &dirs).find(&names(&["in-a"]), 16),
            None
        );
        assert_eq!(
            IconTheme::load("../icons/a", &dirs).find(&names(&["in-a"]), 16),
            None
        );
    }

    #[test]
    fn a_directory_section_cannot_point_outside_the_theme() {
        let root = tempfile::tempdir().unwrap();
        let index = "[Icon Theme]\nDirectories=../other/16\n[../other/16]\nSize=16\nType=Fixed\n";
        theme(root.path(), "sneaky", index, &[]);
        theme(root.path(), "other", "", &["16/secret.svg"]);
        let icons = IconTheme::load("sneaky", &[root.path().to_path_buf()]);
        assert_eq!(icons.find(&names(&["secret"]), 16), None);
    }

    #[test]
    fn reads_the_theme_name_from_the_kde_settings() {
        let config = tempfile::tempdir().unwrap();
        assert_eq!(system_theme_name(config.path()), None);
        fs::write(
            config.path().join("kdeglobals"),
            "[General]\nTheme=not-this\n\n[Icons]\nTheme=breeze-dark\n\n[KDE]\nTheme=nor-this\n",
        )
        .unwrap();
        assert_eq!(
            system_theme_name(config.path()).as_deref(),
            Some("breeze-dark")
        );
    }
}
