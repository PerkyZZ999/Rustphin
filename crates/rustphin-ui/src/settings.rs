//! View settings remembered between sessions, stored as TOML.
//!
//! Reading never fails: a missing, damaged or partly unknown file yields the
//! defaults for whatever could not be understood.

use rustphin_core::{SortKey, ViewOptions};
use serde::{Deserialize, Serialize};

use crate::layout::{self, ViewMode};

/// The format version this build writes. Version 2 has seven zoom levels
/// where version 1 had five (W-107).
const VERSION: u32 = 2;

/// Where each of version 1's five zoom levels (16, 20, 24, 32 and 48-pixel
/// icons) is among today's seven.
const ZOOM_FROM_VERSION_1: [usize; 5] = [0, 1, 2, 4, 6];

/// Everything Rustphin remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    version: u32,
    view: View,
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
            "version = 2",
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
        assert!(settings.to_toml().contains("version = 2"));
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
        assert!(Settings::parse("version = 3\n").is_from_newer_version());
        assert!(!sample().is_from_newer_version());
        assert!(!Settings::parse("").is_from_newer_version());
    }
}
