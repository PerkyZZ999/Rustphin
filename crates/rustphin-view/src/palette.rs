//! The command palette's results: what each row is, its icon, and which
//! row a query's answer selects (W-114b2). The palette's window stays with
//! each interface.

use std::path::PathBuf;

/// What choosing a result does.
#[derive(Debug, Clone)]
pub enum Target {
    /// Forget the recent folders.
    ClearHistory,
    /// Run the command at this index of the command list.
    Command(usize),
    /// Open this local folder.
    Folder(PathBuf),
    /// A remote folder, opened again through kio-fuse.
    Remote(rustphin_platform::remote::RemoteAddress),
    /// Switch to the tab at this index.
    Tab(usize),
}

/// One row the palette can show.
#[derive(Clone)]
pub struct Candidate {
    /// "Category: Name".
    pub label: String,
    /// Extra words shown after the name, such as a folder's path.
    pub detail: String,
    /// The key that runs it, if any.
    pub shortcut: String,
    /// What a query is matched against: the label and the detail.
    pub search: String,
    /// What choosing it does.
    pub target: Target,
}

impl Candidate {
    /// A row with these words, matched by its label and detail.
    pub fn new(label: String, detail: String, shortcut: String, target: Target) -> Self {
        let search = format!("{label} {detail}");
        Self {
            label,
            detail,
            shortcut,
            search,
            target,
        }
    }
}

/// The palette's commands, in the table's order, each with its main key
/// (`commands::COMMANDS`); `Target::Command` is the index in the table.
pub fn command_candidates() -> Vec<Candidate> {
    crate::commands::COMMANDS
        .iter()
        .enumerate()
        .map(|(index, command)| {
            Candidate::new(
                command.label.to_owned(),
                command.target().detail().to_owned(),
                command.shown_key().unwrap_or_default(),
                Target::Command(index),
            )
        })
        .collect()
}

/// A result's label split into its category and its name: "View: Show
/// hidden files" gives ("View", "Show hidden files").
pub fn split_label(label: &str) -> (&str, &str) {
    label.split_once(": ").unwrap_or(("", label))
}

/// The icon of a result's category.
pub fn category_icon(category: &str) -> crate::icons::Icon {
    use crate::icons::Icon as I;
    match category {
        "Files" => I::File,
        "Navigation" => I::Compass,
        "Places" => I::Bookmark,
        "Selection" => I::SquareCheck,
        "Trash" => I::Trash,
        "View" => I::Eye,
        "Workspace" => I::PanelsTopLeft,
        "Tab" => I::PanelTop,
        "Place" => I::Folder,
        "Recent" | "History" => I::Clock,
        _ => I::Command,
    }
}

/// Commands with an icon of their own: the same as their toolbar button or
/// menu row. Other results show their category's icon.
pub const COMMAND_ICONS: &[(&str, crate::icons::Icon)] = {
    use crate::icons::Icon as I;
    &[
        ("Navigation: Back", I::ArrowLeft),
        ("Navigation: Forward", I::ArrowRight),
        ("Navigation: Up one folder", I::ArrowUp),
        ("Navigation: Home folder", I::House),
        ("Navigation: Enter a location", I::TextCursorInput),
        ("Navigation: Reload folder", I::RefreshCw),
        ("Navigation: Open terminal here", I::SquareTerminal),
        ("Navigation: Open selected folder", I::SquareArrowOutUpRight),
        ("View: Icons", I::LayoutGrid),
        ("View: Compact", I::Columns3),
        ("View: Details", I::List),
        ("View: Show or hide hidden files", I::Eye),
        ("View: Filter by name", I::Funnel),
        ("View: Zoom in", I::ZoomIn),
        ("View: Zoom out", I::ZoomOut),
        ("View: Reset zoom", I::Search),
        ("View: Sort by name (toggle direction)", I::ArrowDownAZ),
        (
            "View: Sort by size (toggle direction)",
            I::ArrowDownWideNarrow,
        ),
        (
            "View: Sort by modified date (toggle direction)",
            I::CalendarClock,
        ),
        ("View: Sort by type (toggle direction)", I::FileType),
        ("Files: New folder", I::FolderPlus),
        ("Files: New file", I::FilePlus),
        ("Files: Rename selection", I::Pencil),
        ("Files: Properties", I::Info),
        ("Files: Copy selection", I::Copy),
        ("Files: Cut selection", I::Scissors),
        ("Files: Paste into this folder", I::ClipboardPaste),
        ("Files: Duplicate here", I::CopyPlus),
        ("Files: Extract here", I::PackageOpen),
        ("Files: Extract into a new folder", I::FolderArchive),
        ("Files: Extract and trash archive", I::ArchiveX),
        ("Files: Extract to\u{2026}", I::FolderInput),
        ("Files: Compress to ZIP", I::FileArchive),
        ("Files: Compress to TAR.GZ", I::FileArchive),
        ("Files: Compress to\u{2026}", I::Archive),
        ("Files: Copy location", I::Link),
        ("Files: Undo last operation", I::Undo2),
        ("Files: Move selection to trash", I::Trash),
        ("Files: Delete selection permanently…", I::CircleX),
        ("Files: Stop running operations", I::CircleStop),
        ("Files: Copy selection to the other view", I::ArrowLeftRight),
        ("Files: Move selection to the other view", I::ArrowLeftRight),
        ("Trash: Restore selection", I::ArchiveRestore),
        ("Workspace: New tab", I::Plus),
        ("Workspace: Reopen closed tab", I::RotateCcw),
        ("Workspace: Close tab or window", I::X),
        ("Workspace: Toggle split view", I::SquareSplitHorizontal),
        ("Workspace: Show or hide places", I::PanelLeft),
        ("Workspace: Quit", I::Power),
        ("Workspace: Settings…", I::Settings),
        ("Places: New group", I::ListPlus),
        ("Places: Add current folder", I::BookmarkPlus),
        ("Places: Rename selected place or group", I::Pencil),
        ("Places: Remove selected place or group", I::BookmarkMinus),
        ("Places: Hide or show selected place", I::EyeOff),
        ("Places: Move selected place to next group", I::FolderInput),
        ("Places: Collapse selected group", I::ChevronsDownUp),
        ("Places: Expand selected group", I::ChevronsUpDown),
        (
            "Places: Unmount or safely remove selected device",
            I::ArrowUpFromLine,
        ),
    ]
};

/// The icon of a command, as its palette row shows it (also on the
/// toolbar, W-106f).
pub fn command_icon(label: &str) -> crate::icons::Icon {
    result_icon(label)
}

/// The icon of one result.
pub fn result_icon(label: &str) -> crate::icons::Icon {
    COMMAND_ICONS
        .iter()
        .find(|(command, _)| *command == label)
        .map_or_else(|| category_icon(split_label(label).0), |(_, icon)| *icon)
}

/// Pure selection state: a query invalidates every previous result immediately.
#[derive(Default)]
pub struct Results {
    /// The query the results belong to.
    pub generation: u64,
    /// The matching candidates, best first.
    pub indices: Vec<usize>,
    /// The highlighted row.
    pub selected: usize,
    /// Whether the newest query is still running.
    pub pending: bool,
}

impl Results {
    /// Starts a new query and forgets the results of older ones; returns its number.
    pub fn begin(&mut self) -> u64 {
        self.generation += 1;
        self.indices.clear();
        self.selected = 0;
        self.pending = true;
        self.generation
    }

    /// Shows the results of query `generation`; false when a newer query was started.
    pub fn complete(&mut self, generation: u64, indices: Vec<usize>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.indices = indices;
        self.pending = false;
        true
    }

    /// Moves the highlight by `delta` rows, wrapping around.
    pub fn move_by(&mut self, delta: isize) {
        if !self.indices.is_empty() {
            self.selected =
                (self.selected as isize + delta).rem_euclid(self.indices.len() as isize) as usize;
        }
    }

    /// The highlighted candidate, unless the newest query is still running.
    pub fn chosen(&self) -> Option<usize> {
        if self.pending {
            None
        } else {
            self.indices.get(self.selected).copied()
        }
    }

    /// Highlights `row` of query `generation`; false when that query is old, still running, or has no such row.
    pub fn select(&mut self, generation: u64, row: usize) -> bool {
        if generation != self.generation || self.pending || row >= self.indices.len() {
            return false;
        }
        self.selected = row;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Results, result_icon, split_label};
    use crate::icons::Icon as I;

    #[test]
    fn a_command_shows_its_toolbar_or_menu_icon_and_others_their_category() {
        assert_eq!(result_icon("Navigation: Back"), I::ArrowLeft);
        assert_eq!(result_icon("Files: New folder"), I::FolderPlus);
        assert_eq!(result_icon("View: Show or hide hidden files"), I::Eye);
        assert_eq!(result_icon("Places: New group"), I::ListPlus);
        // No icon of its own: the category's.
        assert_eq!(result_icon("Selection: Invert"), I::SquareCheck);
        assert_eq!(result_icon("Recent: projects"), I::Clock);
    }

    #[test]
    fn a_result_shows_its_name_and_names_its_category_in_the_trail() {
        assert_eq!(
            split_label("View: Show hidden files"),
            ("View", "Show hidden files")
        );
        // Only the first colon separates: a folder name may hold more.
        assert_eq!(split_label("Recent: a: b"), ("Recent", "a: b"));
        assert_eq!(split_label("No category"), ("", "No category"));
    }

    #[test]
    fn a_selection_from_an_old_query_cannot_choose_a_new_result() {
        let mut results = Results::default();
        let old = results.begin();
        assert!(results.complete(old, vec![2, 4]));
        let latest = results.begin();
        assert!(results.complete(latest, vec![7, 9]));
        assert!(!results.select(old, 1));
        assert_eq!(results.chosen(), Some(7));
        assert!(results.select(latest, 1));
        assert_eq!(results.chosen(), Some(9));
    }

    #[test]
    fn pending_or_missing_results_cannot_be_selected() {
        let mut results = Results::default();
        let query = results.begin();
        assert!(!results.select(query, 0));
        assert_eq!(results.chosen(), None);
        assert!(results.complete(query, vec![7, 9]));
        assert!(!results.select(query, 2));
        assert_eq!(results.chosen(), Some(7));
    }

    #[test]
    fn a_late_query_cannot_replace_newer_results() {
        let mut results = Results::default();
        let old = results.begin();
        let latest = results.begin();
        assert!(results.complete(latest, vec![7]));
        assert!(!results.complete(old, vec![3]));
        assert_eq!(results.chosen(), Some(7));
    }

    #[test]
    fn typing_invalidates_the_previous_choice_until_the_new_query_finishes() {
        let mut results = Results::default();
        let first = results.begin();
        assert!(results.complete(first, vec![2, 4]));
        results.move_by(1);
        assert_eq!(results.chosen(), Some(4));
        let next = results.begin();
        assert_eq!(results.chosen(), None);
        assert!(results.complete(next, vec![8, 9]));
        assert_eq!(results.chosen(), Some(8));
    }

    #[test]
    fn navigation_wraps_and_empty_results_never_activate() {
        let mut results = Results::default();
        results.move_by(-1);
        assert_eq!(results.chosen(), None);
        let query = results.begin();
        assert!(results.complete(query, vec![2, 4]));
        results.move_by(-1);
        assert_eq!(results.chosen(), Some(4));
        results.move_by(1);
        assert_eq!(results.chosen(), Some(2));
    }
}
