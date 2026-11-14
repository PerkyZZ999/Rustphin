//! The palette's commands (W-114b3, W-114e): each with its label, its key
//! and what it acts on, named by an id the interface runs. The palette
//! never reimplements a command; it runs the same one the key does.

/// What a command acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The active pane.
    Browser,
    /// The selected place or group.
    Sidebar,
}

/// One command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// What the interface runs: `go-back`, `new-tab`…
    pub id: &'static str,
    /// "Category: what it does".
    pub label: &'static str,
    /// Its key, as shown; empty for none.
    pub shortcut: &'static str,
    /// What it acts on.
    pub scope: Scope,
}

impl Scope {
    /// What the palette says a command acts on.
    pub fn detail(self) -> &'static str {
        match self {
            Self::Browser => "Action \u{b7} active pane",
            Self::Sidebar => "Action \u{b7} selected place or group",
        }
    }
}

/// Every command, in the palette's order.
pub const COMMANDS: &[Command] = &[
    Command {
        id: "go-back",
        label: "Navigation: Back",
        shortcut: "Backspace",
        scope: Scope::Browser,
    },
    Command {
        id: "go-forward",
        label: "Navigation: Forward",
        shortcut: "Alt+Right",
        scope: Scope::Browser,
    },
    Command {
        id: "go-up",
        label: "Navigation: Up one folder",
        shortcut: "Alt+Up",
        scope: Scope::Browser,
    },
    Command {
        id: "go-home",
        label: "Navigation: Home folder",
        shortcut: "Alt+Home",
        scope: Scope::Browser,
    },
    Command {
        id: "edit-location",
        label: "Navigation: Enter a location",
        shortcut: "Ctrl+L",
        scope: Scope::Browser,
    },
    Command {
        id: "reload",
        label: "Navigation: Reload folder",
        shortcut: "F5",
        scope: Scope::Browser,
    },
    Command {
        id: "open-terminal",
        label: "Navigation: Open terminal here",
        shortcut: "Shift+F4",
        scope: Scope::Browser,
    },
    Command {
        id: "activate",
        label: "Navigation: Open selected folder",
        shortcut: "Enter",
        scope: Scope::Browser,
    },
    Command {
        id: "show-icons",
        label: "View: Icons",
        shortcut: "Ctrl+1",
        scope: Scope::Browser,
    },
    Command {
        id: "show-compact",
        label: "View: Compact",
        shortcut: "Ctrl+2",
        scope: Scope::Browser,
    },
    Command {
        id: "show-details",
        label: "View: Details",
        shortcut: "Ctrl+3",
        scope: Scope::Browser,
    },
    Command {
        id: "toggle-hidden",
        label: "View: Show or hide hidden files",
        shortcut: "Ctrl+H",
        scope: Scope::Browser,
    },
    Command {
        id: "show-filter",
        label: "View: Filter by name",
        shortcut: "Ctrl+I",
        scope: Scope::Browser,
    },
    Command {
        id: "toggle-folders-first",
        label: "View: Folders first",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "zoom-in",
        label: "View: Zoom in",
        shortcut: "Ctrl+=",
        scope: Scope::Browser,
    },
    Command {
        id: "zoom-out",
        label: "View: Zoom out",
        shortcut: "Ctrl+-",
        scope: Scope::Browser,
    },
    Command {
        id: "zoom-reset",
        label: "View: Reset zoom",
        shortcut: "Ctrl+0",
        scope: Scope::Browser,
    },
    Command {
        id: "sort-name",
        label: "View: Sort by name (toggle direction)",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "sort-size",
        label: "View: Sort by size (toggle direction)",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "sort-modified",
        label: "View: Sort by modified date (toggle direction)",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "sort-type",
        label: "View: Sort by type (toggle direction)",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "new-folder",
        label: "Files: New folder",
        shortcut: "Ctrl+Shift+N",
        scope: Scope::Browser,
    },
    Command {
        id: "new-file",
        label: "Files: New file",
        shortcut: "Ctrl+Alt+N",
        scope: Scope::Browser,
    },
    Command {
        id: "rename",
        label: "Files: Rename selection",
        shortcut: "F2",
        scope: Scope::Browser,
    },
    Command {
        id: "show-properties",
        label: "Files: Properties",
        shortcut: "Alt+Enter",
        scope: Scope::Browser,
    },
    Command {
        id: "recover-renames",
        label: "Files: Recover original names after interruption",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "copy",
        label: "Files: Copy selection",
        shortcut: "Ctrl+C",
        scope: Scope::Browser,
    },
    Command {
        id: "cut",
        label: "Files: Cut selection",
        shortcut: "Ctrl+X",
        scope: Scope::Browser,
    },
    Command {
        id: "paste",
        label: "Files: Paste into this folder",
        shortcut: "Ctrl+V",
        scope: Scope::Browser,
    },
    Command {
        id: "copy-location",
        label: "Files: Copy location",
        shortcut: "Ctrl+Alt+C",
        scope: Scope::Browser,
    },
    Command {
        id: "duplicate",
        label: "Files: Duplicate here",
        shortcut: "Ctrl+D",
        scope: Scope::Browser,
    },
    Command {
        id: "extract-here",
        label: "Files: Extract here",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "extract-into-folder",
        label: "Files: Extract into a new folder",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "extract-and-trash",
        label: "Files: Extract and trash archive",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "extract-to",
        label: "Files: Extract to\u{2026}",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "compress-zip",
        label: "Files: Compress to ZIP",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "compress-tar-gz",
        label: "Files: Compress to TAR.GZ",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "compress-choose",
        label: "Files: Compress to\u{2026}",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "copy-to-other-view",
        label: "Files: Copy selection to the other view",
        shortcut: "Shift+F5",
        scope: Scope::Browser,
    },
    Command {
        id: "move-to-other-view",
        label: "Files: Move selection to the other view",
        shortcut: "Shift+F6",
        scope: Scope::Browser,
    },
    Command {
        id: "move-to-trash",
        label: "Files: Move selection to trash",
        shortcut: "Delete",
        scope: Scope::Browser,
    },
    Command {
        id: "delete-permanently",
        label: "Files: Delete selection permanently…",
        shortcut: "Shift+Delete",
        scope: Scope::Browser,
    },
    Command {
        id: "undo",
        label: "Files: Undo last operation",
        shortcut: "Ctrl+Z",
        scope: Scope::Browser,
    },
    Command {
        id: "stop-operations",
        label: "Files: Stop running operations",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "dismiss-notice",
        label: "Files: Dismiss operation message",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "trash-actions",
        label: "Trash: Show trash actions",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "open-trash",
        label: "Trash: Open trash",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "restore-trash",
        label: "Trash: Restore selection",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "empty-trash",
        label: "Trash: Empty this trash…",
        shortcut: "",
        scope: Scope::Browser,
    },
    Command {
        id: "select-all",
        label: "Selection: Select all",
        shortcut: "Ctrl+A",
        scope: Scope::Browser,
    },
    Command {
        id: "invert-selection",
        label: "Selection: Invert",
        shortcut: "Ctrl+Shift+A",
        scope: Scope::Browser,
    },
    Command {
        id: "cancel-edit",
        label: "Selection: Clear",
        shortcut: "Escape",
        scope: Scope::Browser,
    },
    Command {
        id: "select-previous",
        label: "Selection: Previous row",
        shortcut: "Up",
        scope: Scope::Browser,
    },
    Command {
        id: "select-next",
        label: "Selection: Next row",
        shortcut: "Down",
        scope: Scope::Browser,
    },
    Command {
        id: "select-first",
        label: "Selection: First entry",
        shortcut: "Home",
        scope: Scope::Browser,
    },
    Command {
        id: "select-last",
        label: "Selection: Last entry",
        shortcut: "End",
        scope: Scope::Browser,
    },
    Command {
        id: "page-up",
        label: "Selection: Previous page",
        shortcut: "PageUp",
        scope: Scope::Browser,
    },
    Command {
        id: "page-down",
        label: "Selection: Next page",
        shortcut: "PageDown",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-previous",
        label: "Selection: Extend to previous row",
        shortcut: "Shift+Up",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-next",
        label: "Selection: Extend to next row",
        shortcut: "Shift+Down",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-first",
        label: "Selection: Extend to first entry",
        shortcut: "Shift+Home",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-last",
        label: "Selection: Extend to last entry",
        shortcut: "Shift+End",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-page-up",
        label: "Selection: Extend to previous page",
        shortcut: "Shift+PageUp",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-page-down",
        label: "Selection: Extend to next page",
        shortcut: "Shift+PageDown",
        scope: Scope::Browser,
    },
    Command {
        id: "select-left",
        label: "Selection: Move left",
        shortcut: "Left",
        scope: Scope::Browser,
    },
    Command {
        id: "select-right",
        label: "Selection: Move right",
        shortcut: "Right",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-left",
        label: "Selection: Extend left",
        shortcut: "Shift+Left",
        scope: Scope::Browser,
    },
    Command {
        id: "extend-right",
        label: "Selection: Extend right",
        shortcut: "Shift+Right",
        scope: Scope::Browser,
    },
    Command {
        id: "new-tab",
        label: "Workspace: New tab",
        shortcut: "Ctrl+T",
        scope: Scope::Browser,
    },
    Command {
        id: "reopen-closed-tab",
        label: "Workspace: Reopen closed tab",
        shortcut: "Ctrl+Shift+T",
        scope: Scope::Browser,
    },
    Command {
        id: "focus-other-view",
        label: "Workspace: Focus the other view",
        shortcut: "Ctrl+F3",
        scope: Scope::Browser,
    },
    Command {
        id: "focus-places",
        label: "Workspace: Focus the places panel",
        shortcut: "Ctrl+P",
        scope: Scope::Browser,
    },
    Command {
        id: "quit",
        label: "Workspace: Quit",
        shortcut: "Ctrl+Q",
        scope: Scope::Browser,
    },
    Command {
        id: "open-settings",
        label: "Workspace: Settings…",
        shortcut: "Ctrl+Shift+,",
        scope: Scope::Browser,
    },
    Command {
        id: "close-tab",
        label: "Workspace: Close tab or window",
        shortcut: "Ctrl+W",
        scope: Scope::Browser,
    },
    Command {
        id: "next-tab",
        label: "Workspace: Next tab",
        shortcut: "Ctrl+Tab",
        scope: Scope::Browser,
    },
    Command {
        id: "previous-tab",
        label: "Workspace: Previous tab",
        shortcut: "Ctrl+Shift+Tab",
        scope: Scope::Browser,
    },
    Command {
        id: "move-tab-left",
        label: "Workspace: Move tab left",
        shortcut: "Ctrl+Shift+PageUp",
        scope: Scope::Browser,
    },
    Command {
        id: "move-tab-right",
        label: "Workspace: Move tab right",
        shortcut: "Ctrl+Shift+PageDown",
        scope: Scope::Browser,
    },
    Command {
        id: "toggle-split",
        label: "Workspace: Toggle split view",
        shortcut: "F3",
        scope: Scope::Browser,
    },
    Command {
        id: "other-pane",
        label: "Workspace: Focus next pane or places",
        shortcut: "Tab",
        scope: Scope::Browser,
    },
    Command {
        id: "toggle-sidebar",
        label: "Workspace: Show or hide places",
        shortcut: "F9",
        scope: Scope::Browser,
    },
    Command {
        id: "add-place",
        label: "Places: Add current folder",
        shortcut: "Ctrl+Shift+D",
        scope: Scope::Browser,
    },
    Command {
        id: "new-group",
        label: "Places: New group",
        shortcut: "Ctrl+Shift+G",
        scope: Scope::Sidebar,
    },
    Command {
        id: "rename-row",
        label: "Places: Rename selected place or group",
        shortcut: "F2",
        scope: Scope::Sidebar,
    },
    Command {
        id: "remove-row",
        label: "Places: Remove selected place or group",
        shortcut: "Delete",
        scope: Scope::Sidebar,
    },
    Command {
        id: "hide-row",
        label: "Places: Hide or show selected place",
        shortcut: "H",
        scope: Scope::Sidebar,
    },
    Command {
        id: "toggle-hidden-places",
        label: "Places: List or hide hidden places",
        shortcut: "",
        scope: Scope::Sidebar,
    },
    Command {
        id: "move-row-up",
        label: "Places: Move selected row up",
        shortcut: "Alt+Up",
        scope: Scope::Sidebar,
    },
    Command {
        id: "move-row-down",
        label: "Places: Move selected row down",
        shortcut: "Alt+Down",
        scope: Scope::Sidebar,
    },
    Command {
        id: "collapse",
        label: "Places: Collapse selected group",
        shortcut: "Left",
        scope: Scope::Sidebar,
    },
    Command {
        id: "expand",
        label: "Places: Expand selected group",
        shortcut: "Right",
        scope: Scope::Sidebar,
    },
    Command {
        id: "next-group",
        label: "Places: Move selected place to next group",
        shortcut: "G",
        scope: Scope::Sidebar,
    },
    Command {
        id: "focus-previous",
        label: "Places: Focus previous row",
        shortcut: "Up",
        scope: Scope::Sidebar,
    },
    Command {
        id: "focus-next",
        label: "Places: Focus next row",
        shortcut: "Down",
        scope: Scope::Sidebar,
    },
    Command {
        id: "open",
        label: "Places: Open selected place or mount selected device",
        shortcut: "Enter",
        scope: Scope::Sidebar,
    },
    Command {
        id: "eject",
        label: "Places: Unmount or safely remove selected device",
        shortcut: "E",
        scope: Scope::Sidebar,
    },
    Command {
        id: "open-palette",
        label: "Workspace: Command palette",
        shortcut: "Ctrl+Shift+P",
        scope: Scope::Browser,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_its_own_id_and_a_category() {
        let mut ids: Vec<&str> = COMMANDS.iter().map(|command| command.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), COMMANDS.len());
        assert!(COMMANDS.iter().all(|command| command.label.contains(": ")));
        assert!(COMMANDS.len() > 80);
    }
}
