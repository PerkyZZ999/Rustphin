//! Views remembered for each folder (Settings, Views, "Remember the view
//! for each folder", W-106c2), as Dolphin remembers them: kept in
//! Rustphin's state folder, `views.toml`, apart from the settings.
//!
//! Reading never fails: an unreadable file or entry is skipped. The list
//! is bounded; the folders set longest ago leave first.

use rustphin_core::{SortKey, ViewOptions};
use serde::{Deserialize, Serialize};

use crate::layout::{self, ViewMode};

/// How many folders are remembered.
const MAX_FOLDERS: usize = 2_000;

/// The format version this build writes.
const VERSION: u32 = 1;

/// One folder's view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderView {
    /// Icons, compact or details.
    pub mode: ViewMode,
    /// The zoom level.
    pub zoom: usize,
    /// Order and filtering.
    pub options: ViewOptions,
}

/// The remembered folders, oldest first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FolderViews {
    folders: Vec<(String, FolderView)>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct File {
    version: u32,
    folders: Vec<toml::Value>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    location: String,
    mode: String,
    zoom: usize,
    sort: String,
    descending: bool,
    show_hidden: bool,
    folders_first: bool,
}

impl FolderViews {
    /// Reads `views.toml` text; whatever cannot be understood is left out.
    pub fn parse(text: &str) -> Self {
        let file: File = toml::from_str(text).unwrap_or_default();
        let mut views = Self::default();
        for value in file.folders {
            if let Ok(saved) = Saved::deserialize(value)
                && let Some(view) = saved.view()
            {
                views.set(saved.location, view);
            }
        }
        views
    }

    /// The file's text.
    pub fn to_toml(&self) -> String {
        let folders = self
            .folders
            .iter()
            .filter_map(|(location, view)| {
                toml::Value::try_from(Saved::from(location.clone(), *view)).ok()
            })
            .collect();
        toml::to_string_pretty(&File {
            version: VERSION,
            folders,
        })
        .unwrap_or_default()
    }

    /// The view remembered for the folder at `location` (a saved session
    /// location), if any.
    pub fn get(&self, location: &str) -> Option<FolderView> {
        self.folders
            .iter()
            .find(|(saved, _)| saved == location)
            .map(|(_, view)| *view)
    }

    /// Remembers `view` for `location`, as the newest. Returns whether
    /// anything changed.
    pub fn set(&mut self, location: String, view: FolderView) -> bool {
        if let Some(at) = self
            .folders
            .iter()
            .position(|(saved, _)| *saved == location)
        {
            if self.folders[at].1 == view && at + 1 == self.folders.len() {
                return false;
            }
            self.folders.remove(at);
        }
        self.folders.push((location, view));
        if self.folders.len() > MAX_FOLDERS {
            self.folders.remove(0);
        }
        true
    }

    /// How many folders are remembered.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.folders.len()
    }
}

impl Saved {
    fn from(location: String, view: FolderView) -> Self {
        Self {
            location,
            mode: match view.mode {
                ViewMode::Icons => "icons",
                ViewMode::Compact => "compact",
                ViewMode::Details => "details",
            }
            .to_owned(),
            zoom: view.zoom,
            sort: match view.options.sort {
                SortKey::Name => "name",
                SortKey::Size => "size",
                SortKey::Modified => "modified",
                SortKey::Type => "type",
            }
            .to_owned(),
            descending: view.options.descending,
            show_hidden: view.options.show_hidden,
            folders_first: view.options.folders_first,
        }
    }

    fn view(&self) -> Option<FolderView> {
        let mode = match self.mode.as_str() {
            "icons" => ViewMode::Icons,
            "compact" => ViewMode::Compact,
            "details" => ViewMode::Details,
            _ => return None,
        };
        let sort = match self.sort.as_str() {
            "name" => SortKey::Name,
            "size" => SortKey::Size,
            "modified" => SortKey::Modified,
            "type" => SortKey::Type,
            _ => return None,
        };
        Some(FolderView {
            mode,
            zoom: self.zoom.min(layout::ZOOM_LEVELS - 1),
            options: ViewOptions {
                sort,
                descending: self.descending,
                show_hidden: self.show_hidden,
                folders_first: self.folders_first,
                details: mode == ViewMode::Details,
            },
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn view(mode: ViewMode, zoom: usize) -> FolderView {
        FolderView {
            mode,
            zoom,
            options: ViewOptions {
                sort: SortKey::Modified,
                descending: true,
                show_hidden: true,
                folders_first: false,
                details: mode == ViewMode::Details,
            },
        }
    }

    #[test]
    fn each_folder_keeps_its_view_through_the_file() {
        let mut views = FolderViews::default();
        views.set(
            "file:///home/u/Pictures".to_owned(),
            view(ViewMode::Icons, 5),
        );
        views.set(
            "file:///home/u/Music".to_owned(),
            view(ViewMode::Details, 1),
        );
        let loaded = FolderViews::parse(&views.to_toml());
        assert_eq!(loaded, views);
        assert_eq!(
            loaded.get("file:///home/u/Pictures"),
            Some(view(ViewMode::Icons, 5))
        );
        assert_eq!(loaded.get("file:///home/u/Other"), None);
    }

    #[test]
    fn setting_again_replaces_and_the_oldest_leave_first() {
        let mut views = FolderViews::default();
        assert!(views.set("a".to_owned(), view(ViewMode::Icons, 1)));
        assert!(!views.set("a".to_owned(), view(ViewMode::Icons, 1)));
        assert!(views.set("a".to_owned(), view(ViewMode::Compact, 2)));
        assert_eq!(views.len(), 1);
        for index in 0..MAX_FOLDERS {
            views.set(format!("f{index}"), view(ViewMode::Icons, 0));
        }
        assert_eq!(views.len(), MAX_FOLDERS);
        assert_eq!(views.get("a"), None, "the oldest left");
        assert!(views.get(&format!("f{}", MAX_FOLDERS - 1)).is_some());
    }

    #[test]
    fn unreadable_entries_are_skipped_and_the_rest_kept() {
        let text = "version = 1\n\
            [[folders]]\nlocation = \"good\"\nmode = \"compact\"\nzoom = 99\nsort = \"size\"\ndescending = false\nshow_hidden = false\nfolders_first = true\n\
            [[folders]]\nlocation = \"bad\"\nmode = \"sideways\"\nzoom = 1\nsort = \"name\"\ndescending = false\nshow_hidden = false\nfolders_first = true\n\
            [[folders]]\nlocation = \"partial\"\n";
        let views = FolderViews::parse(text);
        let good = views.get("good").unwrap();
        assert_eq!(good.mode, ViewMode::Compact);
        assert_eq!(good.zoom, layout::ZOOM_LEVELS - 1);
        assert_eq!(views.get("bad"), None);
        assert_eq!(views.get("partial"), None);
        assert_eq!(FolderViews::parse("not toml ["), FolderViews::default());
    }
}
