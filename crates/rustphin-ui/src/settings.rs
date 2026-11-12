//! Rustphin's settings, stored as TOML (D-029): how it starts and works
//! with the desktop, and the view folders open in.
//!
//! Reading never fails: a missing, damaged or partly unknown file yields the
//! defaults for whatever could not be understood.

use rustphin_core::{SortKey, ViewOptions};
use serde::{Deserialize, Serialize};

use crate::layout::{self, ViewMode};

/// The format version this build writes. Version 2 has seven zoom levels
/// where version 1 had five (W-107); version 3 adds `[general]` (W-106c).
const VERSION: u32 = 3;

/// Where each of version 1's five zoom levels (16, 20, 24, 32 and 48-pixel
/// icons) is among today's seven.
const ZOOM_FROM_VERSION_1: [usize; 5] = [0, 1, 2, 4, 6];

/// Everything Rustphin remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    version: u32,
    general: General,
    view: View,
}

/// How Rustphin starts and works with the rest of the desktop.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
struct General {
    #[serde(deserialize_with = "lenient")]
    start: Start,
    /// The folder for `Start::Folder`, saved as a session location.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    start_folder: Option<String>,
    #[serde(deserialize_with = "lenient")]
    shown_folders: ShownIn,
    /// A terminal command chosen here instead of KDE's.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    terminal: Option<String>,
    /// How files open; KDE's setting when absent.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    click: Option<Click>,
}

/// What Rustphin shows when it starts without folders to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Start {
    /// The tabs open when it last closed.
    #[default]
    Restore,
    /// The home folder.
    Home,
    /// The chosen folder.
    Folder,
}

/// Where folders other applications ask for are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShownIn {
    /// A new tab of the open window.
    #[default]
    Tab,
    /// A window of their own.
    Window,
}

/// How many clicks open a file or folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Click {
    /// A double click; one click selects.
    Double,
    /// A single click; Ctrl or Shift and a click select.
    Single,
}

/// Reads a value, or its default when it is not one this build knows, so
/// one strange value does not cost the rest of the file.
fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let value = toml::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct View {
    mode: Mode,
    zoom: usize,
    sort: Sort,
    descending: bool,
    show_hidden: bool,
    folders_first: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Icons,
    Compact,
    #[default]
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Sort {
    #[default]
    Name,
    Size,
    Modified,
    Type,
}

impl Default for View {
    fn default() -> Self {
        let options = ViewOptions::default();
        Self {
            mode: Mode::default(),
            zoom: layout::DEFAULT_ZOOM,
            sort: Sort::default(),
            descending: options.descending,
            show_hidden: options.show_hidden,
            folders_first: options.folders_first,
        }
    }
}

impl Settings {
    /// Captures the current view state.
    pub fn new(mode: ViewMode, zoom: usize, options: ViewOptions) -> Self {
        Self {
            version: VERSION,
            general: General::default(),
            view: View {
                mode: match mode {
                    ViewMode::Icons => Mode::Icons,
                    ViewMode::Compact => Mode::Compact,
                    ViewMode::Details => Mode::Details,
                },
                zoom,
                sort: match options.sort {
                    SortKey::Name => Sort::Name,
                    SortKey::Size => Sort::Size,
                    SortKey::Modified => Sort::Modified,
                    SortKey::Type => Sort::Type,
                },
                descending: options.descending,
                show_hidden: options.show_hidden,
                folders_first: options.folders_first,
            },
        }
    }

    /// Replaces the view part with the current view, keeping the rest.
    pub fn set_view(&mut self, mode: ViewMode, zoom: usize, options: ViewOptions) {
        let current = Self::new(mode, zoom, options);
        self.view = current.view;
        self.version = VERSION;
    }

    /// What Rustphin shows when it starts without folders to open, and the
    /// chosen folder as a session location.
    pub fn start(&self) -> (Start, Option<&str>) {
        (self.general.start, self.general.start_folder.as_deref())
    }

    /// Sets what Rustphin shows at start; `folder` is kept when `None`.
    pub fn set_start(&mut self, start: Start, folder: Option<String>) {
        self.general.start = start;
        if folder.is_some() {
            self.general.start_folder = folder;
        }
        self.version = VERSION;
    }

    /// Where folders other applications ask for are shown.
    pub fn shown_folders(&self) -> ShownIn {
        self.general.shown_folders
    }

    /// Sets where folders other applications ask for are shown.
    pub fn set_shown_folders(&mut self, shown: ShownIn) {
        self.general.shown_folders = shown;
        self.version = VERSION;
    }

    /// The terminal command chosen here, if any.
    pub fn terminal(&self) -> Option<&str> {
        self.general.terminal.as_deref()
    }

    /// Chooses a terminal command; empty words go back to KDE's terminal.
    pub fn set_terminal(&mut self, command: &str) {
        let command = command.trim();
        self.general.terminal = (!command.is_empty()).then(|| command.to_owned());
        self.version = VERSION;
    }

    /// How files open, when chosen here; `None` follows KDE.
    pub fn click(&self) -> Option<Click> {
        self.general.click
    }

    /// Chooses how files open; `None` follows KDE.
    pub fn set_click(&mut self, click: Option<Click>) {
        self.general.click = click;
        self.version = VERSION;
    }

    /// Puts the General page back as it was before anything was chosen.
    pub fn reset_general(&mut self) {
        self.general = General::default();
        self.version = VERSION;
    }

    /// Reads settings from TOML text, falling back to the defaults for the
    /// whole file if it cannot be parsed.
    pub fn parse(text: &str) -> Self {
        toml::from_str(text).unwrap_or_else(|error| {
            tracing::warn!(%error, "ignoring unreadable settings");
            Self::default()
        })
    }

    /// The settings as TOML text.
    pub fn to_toml(&self) -> String {
        // Serialising plain booleans, integers and unit enums cannot fail; an
        // empty file would simply read back as the defaults.
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Whether the file was written by a later version of Rustphin. Such a
    /// file is read as far as it is understood but must not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }

    /// The remembered view mode.
    pub fn mode(&self) -> ViewMode {
        match self.view.mode {
            Mode::Icons => ViewMode::Icons,
            Mode::Compact => ViewMode::Compact,
            Mode::Details => ViewMode::Details,
        }
    }

    /// The remembered zoom level, kept within the valid levels. A file
    /// from before the seven levels keeps its icon size.
    pub fn zoom(&self) -> usize {
        if self.version < 2 {
            ZOOM_FROM_VERSION_1[self.view.zoom.min(ZOOM_FROM_VERSION_1.len() - 1)]
        } else {
            self.view.zoom.min(layout::ZOOM_LEVELS - 1)
        }
    }

    /// The remembered ordering and filtering, with `details` set for the mode.
    pub fn options(&self) -> ViewOptions {
        ViewOptions {
            show_hidden: self.view.show_hidden,
            folders_first: self.view.folders_first,
            sort: match self.view.sort {
                Sort::Name => SortKey::Name,
                Sort::Size => SortKey::Size,
                Sort::Modified => SortKey::Modified,
                Sort::Type => SortKey::Type,
            },
            descending: self.view.descending,
            details: self.mode() == ViewMode::Details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Settings {
        Settings::new(
            ViewMode::Icons,
            3,
            ViewOptions {
                show_hidden: true,
                folders_first: false,
                sort: SortKey::Modified,
                descending: true,
                details: false,
            },
        )
    }

    #[test]
    fn settings_survive_being_written_and_read_back() {
        let saved = sample();
        let loaded = Settings::parse(&saved.to_toml());
        assert_eq!(loaded, saved);
        assert_eq!(loaded.mode(), ViewMode::Icons);
        assert_eq!(loaded.zoom(), 3);
        assert_eq!(loaded.options().sort, SortKey::Modified);
        assert!(loaded.options().descending && loaded.options().show_hidden);
        assert!(!loaded.options().folders_first);
    }

    #[test]
    fn the_file_uses_plain_readable_names() {
        let text = sample().to_toml();
        for line in [
            "version = 3",
            "[view]",
            "mode = \"icons\"",
            "sort = \"modified\"",
            "zoom = 3",
        ] {
            assert!(text.contains(line), "missing {line:?} in:\n{text}");
        }
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let settings = Settings::parse("");
        assert_eq!(settings.mode(), ViewMode::Details);
        assert_eq!(settings.zoom(), layout::DEFAULT_ZOOM);
        assert_eq!(
            settings.options(),
            ViewOptions {
                details: true,
                ..ViewOptions::default()
            }
        );
    }

    #[test]
    fn missing_keys_keep_their_defaults() {
        let settings = Settings::parse("[view]\nmode = \"compact\"\n");
        assert_eq!(settings.mode(), ViewMode::Compact);
        assert_eq!(settings.zoom(), layout::DEFAULT_ZOOM);
        assert!(settings.options().folders_first);
        assert!(!settings.options().details);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let settings = Settings::parse("future = true\n[view]\nzoom = 2\ncolour = \"teal\"\n");
        assert_eq!(settings.zoom(), 2);
    }

    #[test]
    fn damaged_files_and_wrong_values_fall_back_to_the_defaults() {
        for text in [
            "this is not toml [",
            "[view]\nmode = \"sideways\"\n",
            "[view]\nzoom = \"big\"\n",
            "[view]\nzoom = -1\n",
        ] {
            assert_eq!(Settings::parse(text), Settings::default(), "for {text:?}");
        }
    }

    #[test]
    fn setting_the_view_changes_only_the_view_and_writes_todays_version() {
        let mut settings = Settings::parse("version = 1\n[view]\nzoom = 3\n");
        settings.set_view(
            ViewMode::Compact,
            5,
            ViewOptions {
                show_hidden: true,
                ..ViewOptions::default()
            },
        );
        assert_eq!(settings.mode(), ViewMode::Compact);
        assert_eq!(settings.zoom(), 5);
        assert!(settings.options().show_hidden);
        assert!(settings.to_toml().contains("version = 3"));
    }

    #[test]
    fn a_zoom_saved_with_five_levels_keeps_its_icon_size() {
        let icon = |text: &str| ViewMode::Details.layout(Settings::parse(text).zoom()).icon;
        // Version 1 had 16, 20, 24, 32 and 48-pixel levels.
        let old: Vec<f32> = (0..5)
            .map(|zoom| icon(&format!("version = 1\n[view]\nzoom = {zoom}\n")))
            .collect();
        assert_eq!(old, [16., 20., 24., 32., 48.]);
        // Today's files are taken as they are.
        assert_eq!(icon("version = 2\n[view]\nzoom = 3\n"), 28.);
    }

    #[test]
    fn an_out_of_range_zoom_is_clamped() {
        assert_eq!(
            Settings::parse("[view]\nzoom = 99\n").zoom(),
            layout::ZOOM_LEVELS - 1
        );
    }

    #[test]
    fn a_file_from_a_newer_version_is_recognised() {
        assert!(Settings::parse("version = 4\n").is_from_newer_version());
        assert!(!sample().is_from_newer_version());
        assert!(!Settings::parse("").is_from_newer_version());
    }

    #[test]
    fn the_general_settings_survive_being_written_and_read_back() {
        let mut settings = sample();
        settings.set_start(Start::Folder, Some("/home/u/Work".to_owned()));
        settings.set_shown_folders(ShownIn::Window);
        settings.set_terminal("  kitty --single-instance ");
        settings.set_click(Some(Click::Single));
        let text = settings.to_toml();
        assert!(text.contains("[general]") && text.contains("start = \"folder\""));
        let loaded = Settings::parse(&text);
        assert_eq!(loaded, settings);
        assert_eq!(loaded.start(), (Start::Folder, Some("/home/u/Work")));
        assert_eq!(loaded.shown_folders(), ShownIn::Window);
        assert_eq!(loaded.terminal(), Some("kitty --single-instance"));
        assert_eq!(loaded.click(), Some(Click::Single));
    }

    #[test]
    fn a_value_this_build_does_not_know_falls_back_alone() {
        let settings = Settings::parse(
            "version = 3\n[general]\nstart = \"sideways\"\nshown_folders = \"window\"\nclick = 2\n[view]\nzoom = 4\n",
        );
        assert_eq!(settings.start().0, Start::Restore);
        assert_eq!(settings.shown_folders(), ShownIn::Window);
        assert_eq!(settings.click(), None);
        assert_eq!(settings.zoom(), 4);
    }

    #[test]
    fn choosing_no_terminal_or_resetting_the_page_goes_back_to_kdes() {
        let mut settings = sample();
        settings.set_terminal("foot");
        settings.set_terminal("   ");
        assert_eq!(settings.terminal(), None);
        settings.set_start(Start::Home, None);
        settings.set_click(Some(Click::Double));
        settings.reset_general();
        assert_eq!(settings.start(), (Start::Restore, None));
        assert_eq!(settings.click(), None);
        // The view is another page's.
        assert_eq!(settings.mode(), ViewMode::Icons);
        assert!(!settings.to_toml().contains("terminal"));
    }
}
