//! Named entry points into existing actions. The palette never reimplements them.

use crate::{browser as b, sidebar as s, workspace as w};
use gpui_kit::Action;

#[derive(Clone, Copy)]
pub enum Scope {
    Browser,
    Sidebar,
}

pub struct Command {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub scope: Scope,
    pub action: fn() -> Box<dyn Action>,
}

fn action<A: Action + Default>() -> Box<dyn Action> {
    Box::<A>::default()
}

pub fn commands() -> Vec<Command> {
    macro_rules! command {
        ($label:literal, $key:literal, $scope:ident, $action:ty) => {
            Command {
                label: $label,
                shortcut: $key,
                scope: Scope::$scope,
                action: action::<$action>,
            }
        };
    }
    vec![
        command!("Navigation: Back", "Alt+Left", Browser, b::GoBack),
        command!("Navigation: Forward", "Alt+Right", Browser, b::GoForward),
        command!("Navigation: Up one folder", "Alt+Up", Browser, b::GoUp),
        command!(
            "Navigation: Enter a location",
            "Ctrl+L",
            Browser,
            b::EditLocation
        ),
        command!("Navigation: Reload folder", "F5", Browser, b::Reload),
        command!(
            "Navigation: Open selected folder",
            "Enter",
            Browser,
            b::Activate
        ),
        command!("View: Icons", "Ctrl+1", Browser, b::ShowIcons),
        command!("View: Compact", "Ctrl+2", Browser, b::ShowCompact),
        command!("View: Details", "Ctrl+3", Browser, b::ShowDetails),
        command!(
            "View: Show or hide hidden files",
            "Ctrl+H",
            Browser,
            b::ToggleHidden
        ),
        command!("View: Zoom in", "Ctrl+=", Browser, b::ZoomIn),
        command!("View: Zoom out", "Ctrl+-", Browser, b::ZoomOut),
        command!("View: Reset zoom", "Ctrl+0", Browser, b::ZoomReset),
        command!(
            "View: Sort by name (toggle direction)",
            "",
            Browser,
            b::SortName
        ),
        command!(
            "View: Sort by size (toggle direction)",
            "",
            Browser,
            b::SortSize
        ),
        command!(
            "View: Sort by modified date (toggle direction)",
            "",
            Browser,
            b::SortModified
        ),
        command!(
            "View: Sort by type (toggle direction)",
            "",
            Browser,
            b::SortType
        ),
        command!("Files: New folder", "Ctrl+Shift+N", Browser, b::NewFolder),
        command!("Files: New file", "Ctrl+Alt+N", Browser, b::NewFile),
        command!("Files: Rename selection", "F2", Browser, b::Rename),
        command!("Files: Copy selection", "Ctrl+C", Browser, b::Copy),
        command!("Files: Cut selection", "Ctrl+X", Browser, b::Cut),
        command!("Files: Paste into this folder", "Ctrl+V", Browser, b::Paste),
        command!(
            "Files: Move selection to trash",
            "Delete",
            Browser,
            b::MoveToTrash
        ),
        command!(
            "Files: Delete selection permanently…",
            "Shift+Delete",
            Browser,
            b::DeletePermanently
        ),
        command!("Files: Undo last operation", "Ctrl+Z", Browser, w::Undo),
        command!(
            "Files: Stop running operations",
            "",
            Browser,
            w::StopOperations
        ),
        command!(
            "Files: Dismiss operation message",
            "",
            Browser,
            w::DismissNotice
        ),
        command!("Trash: Open trash", "", Browser, b::OpenTrash),
        command!("Trash: Restore selection", "", Browser, b::RestoreTrash),
        command!("Trash: Empty this trash…", "", Browser, b::EmptyTrash),
        command!("Selection: Select all", "Ctrl+A", Browser, b::SelectAll),
        command!(
            "Selection: Invert",
            "Ctrl+Shift+A",
            Browser,
            b::InvertSelection
        ),
        command!("Selection: Clear", "Escape", Browser, b::CancelEdit),
        command!("Selection: Previous row", "Up", Browser, b::SelectPrevious),
        command!("Selection: Next row", "Down", Browser, b::SelectNext),
        command!("Selection: First entry", "Home", Browser, b::SelectFirst),
        command!("Selection: Last entry", "End", Browser, b::SelectLast),
        command!("Selection: Previous page", "PageUp", Browser, b::PageUp),
        command!("Selection: Next page", "PageDown", Browser, b::PageDown),
        command!(
            "Selection: Extend to previous row",
            "Shift+Up",
            Browser,
            b::ExtendPrevious
        ),
        command!(
            "Selection: Extend to next row",
            "Shift+Down",
            Browser,
            b::ExtendNext
        ),
        command!(
            "Selection: Extend to first entry",
            "Shift+Home",
            Browser,
            b::ExtendFirst
        ),
        command!(
            "Selection: Extend to last entry",
            "Shift+End",
            Browser,
            b::ExtendLast
        ),
        command!(
            "Selection: Extend to previous page",
            "Shift+PageUp",
            Browser,
            b::ExtendPageUp
        ),
        command!(
            "Selection: Extend to next page",
            "Shift+PageDown",
            Browser,
            b::ExtendPageDown
        ),
        command!("Selection: Move left", "Left", Browser, b::SelectLeft),
        command!("Selection: Move right", "Right", Browser, b::SelectRight),
        command!(
            "Selection: Extend left",
            "Shift+Left",
            Browser,
            b::ExtendLeft
        ),
        command!(
            "Selection: Extend right",
            "Shift+Right",
            Browser,
            b::ExtendRight
        ),
        command!("Workspace: New tab", "Ctrl+T", Browser, w::NewTab),
        command!(
            "Workspace: Close tab or window",
            "Ctrl+W",
            Browser,
            w::CloseTab
        ),
        command!("Workspace: Next tab", "Ctrl+Tab", Browser, w::NextTab),
        command!(
            "Workspace: Previous tab",
            "Ctrl+Shift+Tab",
            Browser,
            w::PreviousTab
        ),
        command!(
            "Workspace: Move tab left",
            "Ctrl+Shift+PageUp",
            Browser,
            w::MoveTabLeft
        ),
        command!(
            "Workspace: Move tab right",
            "Ctrl+Shift+PageDown",
            Browser,
            w::MoveTabRight
        ),
        command!(
            "Workspace: Toggle split view",
            "F3",
            Browser,
            w::ToggleSplit
        ),
        command!(
            "Workspace: Focus next pane or places",
            "Tab",
            Browser,
            w::OtherPane
        ),
        command!(
            "Workspace: Show or hide places",
            "F9",
            Browser,
            w::ToggleSidebar
        ),
        command!("Places: Add current folder", "Ctrl+D", Browser, w::AddPlace),
        command!("Places: New group", "Ctrl+Shift+G", Sidebar, s::NewGroup),
        command!(
            "Places: Rename selected place or group",
            "F2",
            Sidebar,
            s::RenameRow
        ),
        command!(
            "Places: Remove selected place or group",
            "Delete",
            Sidebar,
            s::RemoveRow
        ),
        command!(
            "Places: Hide or show selected place",
            "H",
            Sidebar,
            s::HideRow
        ),
        command!(
            "Places: List or hide hidden places",
            "",
            Sidebar,
            s::ToggleHiddenPlaces
        ),
        command!(
            "Places: Move selected row up",
            "Alt+Up",
            Sidebar,
            s::MoveRowUp
        ),
        command!(
            "Places: Move selected row down",
            "Alt+Down",
            Sidebar,
            s::MoveRowDown
        ),
        command!(
            "Places: Collapse selected group",
            "Left",
            Sidebar,
            s::Collapse
        ),
        command!("Places: Expand selected group", "Right", Sidebar, s::Expand),
        command!(
            "Places: Move selected place to next group",
            "G",
            Sidebar,
            s::NextGroup
        ),
        command!(
            "Places: Focus previous row",
            "Up",
            Sidebar,
            s::FocusPrevious
        ),
        command!("Places: Focus next row", "Down", Sidebar, s::FocusNext),
        command!(
            "Places: Open selected place or mount selected device",
            "Enter",
            Sidebar,
            s::Open
        ),
        command!(
            "Places: Unmount or safely remove selected device",
            "E",
            Sidebar,
            s::Eject
        ),
        command!(
            "Workspace: Command palette",
            "Ctrl+Shift+P",
            Browser,
            w::OpenPalette
        ),
    ]
}
