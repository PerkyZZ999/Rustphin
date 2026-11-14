//! Every command in one table (W-123): its stable id, its label, the keys
//! that run it and what it acts on, with its typed form. The palette, the
//! Settings shortcuts page, menu rows, toolbar tooltips and the window's
//! shortcuts all read this table, so a key is written once (KEYBOARD.md).
//!
//! Each key says which part of the interface catches it ([`Catcher`]):
//! window shortcuts work wherever the focus is, so they may not share a
//! sequence with any other key; keys that also edit text (arrows, Enter,
//! Backspace, Home, End, Tab, Escape) are caught by the pane's or the places
//! panel's own key handler instead (AGENTS.md, "Known gotchas").

use rustphin_core::SortKey;

use crate::{
    layout::{Step, ViewMode},
    menu::{CompressHow, ExtractHow},
};

/// What a command acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The active pane.
    Pane,
    /// The places panel's focused row.
    Places,
    /// The window: tabs, panels, dialogs, operations.
    Window,
}

impl Target {
    /// What the palette says a command acts on.
    pub fn detail(self) -> &'static str {
        match self {
            Self::Pane => "Action \u{b7} active pane",
            Self::Places => "Action \u{b7} selected place or group",
            Self::Window => "Action \u{b7} window",
        }
    }
}

/// A command the active pane runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneCommand {
    /// Back in the pane's history.
    Back,
    /// Forward in the pane's history.
    Forward,
    /// The parent folder.
    Up,
    /// The home folder.
    Home,
    /// Read the folder again.
    Reload,
    /// Open a terminal in the folder.
    OpenTerminal,
    /// Open the selection.
    Activate,
    /// Show the entries this way.
    Show(ViewMode),
    /// Show or hide hidden files.
    ToggleHidden,
    /// Folders before files, or mixed.
    ToggleFoldersFirst,
    /// One zoom level larger.
    ZoomIn,
    /// One zoom level smaller.
    ZoomOut,
    /// The default zoom level.
    ZoomReset,
    /// Sort by this key, or turn its direction round.
    SortBy(SortKey),
    /// Create a folder.
    NewFolder,
    /// Create an empty file.
    NewFile,
    /// Rename the selection (several files: batch rename).
    Rename,
    /// Show properties of the selection.
    Properties,
    /// Put back the names an interrupted batch rename left.
    RecoverRenames,
    /// Copy the selection to the clipboard.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste into the folder.
    Paste,
    /// Put the selection's location on the clipboard as text.
    CopyLocation,
    /// Copy the selection beside itself (D-021).
    Duplicate,
    /// Extract the selected archives.
    Extract(ExtractHow),
    /// Compress the selection.
    Compress(CompressHow),
    /// Move the selection to the trash.
    MoveToTrash,
    /// Delete the selection for good, after asking.
    DeletePermanently,
    /// Show the trash.
    OpenTrash,
    /// Bring the selection back from the trash.
    RestoreTrash,
    /// Ask before emptying the trash shown.
    EmptyTrash,
    /// Select every entry.
    SelectAll,
    /// Select exactly the entries that were not.
    InvertSelection,
    /// Select nothing.
    ClearSelection,
    /// Move the keyboard focus; with `extend`, select the range to it.
    Move {
        /// Where to.
        step: Step,
        /// Whether the selection grows to the new focus.
        extend: bool,
    },
}

/// A command the places panel runs on its focused row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacesCommand {
    /// Focus the row above.
    FocusPrevious,
    /// Focus the row below.
    FocusNext,
    /// Open the place or mount the device.
    Open,
    /// Edit the place's or group's name.
    Rename,
    /// Remove the place or group.
    Remove,
    /// Hide the place, or show it again.
    ToggleHide,
    /// One position up.
    MoveUp,
    /// One position down.
    MoveDown,
    /// Fold the group away.
    Collapse,
    /// Unfold the group.
    Expand,
    /// Put the place in the next group.
    NextGroup,
    /// Make a new group and name it.
    NewGroup,
    /// Unmount or safely remove the device.
    Eject,
    /// List or hide the hidden places.
    ToggleHiddenPlaces,
}

/// A command the window runs (in QML: tabs, panels, dialogs, operations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    /// Edit the location field, or leave it.
    EditLocation,
    /// Open or close the name filter.
    ShowFilter,
    /// Copy the selection to the other view.
    CopyToOtherView,
    /// Move the selection to the other view.
    MoveToOtherView,
    /// Undo the last operation.
    Undo,
    /// Stop the running operations.
    StopOperations,
    /// Dismiss the operation message.
    DismissNotice,
    /// Show the trash's menu.
    TrashActions,
    /// Open a tab.
    NewTab,
    /// Reopen the last closed tab.
    ReopenClosedTab,
    /// Focus the other view of a split.
    FocusOtherView,
    /// Focus the places panel.
    FocusPlaces,
    /// End Rustphin.
    Quit,
    /// Open Settings.
    OpenSettings,
    /// Close the tab, or the window with its last tab.
    CloseTab,
    /// The next tab.
    NextTab,
    /// The previous tab.
    PreviousTab,
    /// Move the tab one place left.
    MoveTabLeft,
    /// Move the tab one place right.
    MoveTabRight,
    /// The tab with this number from 1; 0 is the last tab.
    ActivateTab(u8),
    /// Split the view, or join it.
    ToggleSplit,
    /// The next view, then the places panel.
    OtherPane,
    /// Show or hide the places panel.
    ToggleSidebar,
    /// Add the shown folder to the places.
    AddPlace,
    /// Open the command palette.
    OpenPalette,
}

/// What running a command does, typed for the part that runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Run by the active pane.
    Pane(PaneCommand),
    /// Run by the places panel.
    Places(PlacesCommand),
    /// Run by the window.
    Window(WindowCommand),
}

impl Action {
    /// What the action acts on.
    pub fn target(self) -> Target {
        match self {
            Self::Pane(_) => Target::Pane,
            Self::Places(_) => Target::Places,
            Self::Window(_) => Target::Window,
        }
    }
}

/// Which part of the interface catches a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Catcher {
    /// A window shortcut, wherever the focus is.
    Window,
    /// A window shortcut only while the file list has the focus and no
    /// field is being edited.
    List,
    /// The pane's own key handler: keys that also edit text.
    Pane,
    /// The places panel's key handler, while it has the focus.
    Places,
    /// The places panel's key handler, ahead of a window shortcut on the
    /// same key: the panel claims it (Qt's shortcut override).
    PlacesFirst,
}

/// One key of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// As Qt reads it (`QKeySequence`'s portable text): "Alt+Left".
    pub sequence: &'static str,
    /// What catches it.
    pub catcher: Catcher,
}

const fn window(sequence: &'static str) -> Key {
    Key {
        sequence,
        catcher: Catcher::Window,
    }
}

const fn list(sequence: &'static str) -> Key {
    Key {
        sequence,
        catcher: Catcher::List,
    }
}

const fn pane(sequence: &'static str) -> Key {
    Key {
        sequence,
        catcher: Catcher::Pane,
    }
}

const fn places(sequence: &'static str) -> Key {
    Key {
        sequence,
        catcher: Catcher::Places,
    }
}

const fn places_first(sequence: &'static str) -> Key {
    Key {
        sequence,
        catcher: Catcher::PlacesFirst,
    }
}

/// One command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// What the interface runs it by: `go-back`, `new-tab`… Saved in
    /// settings (toolbar buttons), so it never changes.
    pub id: &'static str,
    /// "Category: what it does".
    pub label: &'static str,
    /// What it does.
    pub action: Action,
    /// Its keys; the first is the one menus and tooltips show.
    pub keys: &'static [Key],
}

impl Command {
    /// What it acts on.
    pub fn target(&self) -> Target {
        self.action.target()
    }

    /// Its keys as people read them, each once, the main one first.
    pub fn shown_keys(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        let mut shown: Vec<String> = Vec::new();
        for key in self.keys {
            let same = canonical(key.sequence);
            let words = display(key.sequence);
            if seen.contains(&same) || shown.contains(&words) {
                continue;
            }
            seen.push(same);
            shown.push(words);
        }
        shown
    }

    /// The key menus, tooltips and the palette show.
    pub fn shown_key(&self) -> Option<String> {
        self.keys.first().map(|key| display(key.sequence))
    }
}

/// The command `id` names.
pub fn find(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.id == id)
}

/// What the command `id` does, typed for the part that runs it.
pub fn parse(id: &str) -> Option<Action> {
    find(id).map(|command| command.action)
}

/// The command that does `action`.
pub fn of(action: Action) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.action == action)
}

/// The key shown for `action`, if it has one.
pub fn key_of(action: Action) -> Option<String> {
    of(action).and_then(Command::shown_key)
}

/// One window shortcut: a command's keys that one `Shortcut` item listens
/// to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WindowShortcut {
    /// The command it runs.
    pub id: &'static str,
    /// Its sequences, as Qt reads them.
    pub keys: Vec<&'static str>,
    /// Only while the file list has the focus and no field is edited.
    pub list: bool,
}

/// The shortcuts the window listens to: per command, its window keys and
/// its list keys.
pub fn window_shortcuts() -> Vec<WindowShortcut> {
    let mut shortcuts = Vec::new();
    for command in COMMANDS {
        for (catcher, list) in [(Catcher::Window, false), (Catcher::List, true)] {
            let keys: Vec<&'static str> = command
                .keys
                .iter()
                .filter(|key| key.catcher == catcher)
                .map(|key| key.sequence)
                .collect();
            if !keys.is_empty() {
                shortcuts.push(WindowShortcut {
                    id: command.id,
                    keys,
                    list,
                });
            }
        }
    }
    shortcuts
}

/// Splits a sequence into its modifiers and its key ("Ctrl++" is Ctrl and
/// "+").
fn split(sequence: &str) -> (Vec<&str>, &str) {
    if let Some(rest) = sequence.strip_suffix("++") {
        let modifiers = rest.split('+').filter(|part| !part.is_empty()).collect();
        return (modifiers, "+");
    }
    if sequence == "+" {
        return (Vec::new(), "+");
    }
    let mut parts: Vec<&str> = sequence.split('+').collect();
    let key = parts.pop().unwrap_or_default();
    (parts, key)
}

/// A sequence as people read it: "Page Up", not Qt's "PgUp".
pub fn display(sequence: &str) -> String {
    let (modifiers, key) = split(sequence);
    let key = match key {
        "Return" => "Enter",
        "Del" => "Delete",
        "Esc" => "Escape",
        "PgUp" | "PageUp" => "Page Up",
        "PgDown" | "PageDown" => "Page Down",
        "Back" => "Back key",
        "Forward" => "Forward key",
        "Refresh" => "Refresh key",
        "Menu" => "Menu key",
        other => other,
    };
    modifiers
        .into_iter()
        .chain(std::iter::once(key))
        .collect::<Vec<_>>()
        .join("+")
}

/// The keys Shift makes on a US layout, which Qt matches either way: a
/// press of Shift+, is also "<", so "Ctrl+Shift+," and "Ctrl+<" are one
/// key to Qt.
const SHIFTED: [(char, char); 21] = [
    (',', '<'),
    ('.', '>'),
    ('/', '?'),
    (';', ':'),
    ('\'', '"'),
    ('[', '{'),
    (']', '}'),
    ('\\', '|'),
    ('=', '+'),
    ('-', '_'),
    ('`', '~'),
    ('1', '!'),
    ('2', '@'),
    ('3', '#'),
    ('4', '$'),
    ('5', '%'),
    ('6', '^'),
    ('7', '&'),
    ('8', '*'),
    ('9', '('),
    ('0', ')'),
];

/// One form for sequences Qt treats as the same press: modifiers in one
/// order, Qt's other key names, and Shift folded into the character it
/// makes ("Ctrl+Shift+," is "Ctrl+<"; "Ctrl+Shift+Tab" is "Ctrl+Backtab").
pub fn canonical(sequence: &str) -> String {
    let (modifiers, key) = split(sequence);
    let mut modifiers: Vec<String> = modifiers
        .into_iter()
        .map(|modifier| match modifier.to_lowercase().as_str() {
            "control" => "ctrl".to_owned(),
            "win" | "super" => "meta".to_owned(),
            other => other.to_owned(),
        })
        .collect();
    let mut key = match key.to_lowercase().as_str() {
        "del" => "delete".to_owned(),
        "esc" => "escape".to_owned(),
        "ins" => "insert".to_owned(),
        "pgup" | "page up" => "pageup".to_owned(),
        "pgdown" | "page down" => "pagedown".to_owned(),
        other => other.to_owned(),
    };
    let shift = modifiers.iter().any(|modifier| modifier == "shift");
    let mut chars = key.chars();
    if let (Some(only), None) = (chars.next(), chars.next()) {
        if let Some((_, shifted)) = SHIFTED.iter().find(|(plain, _)| *plain == only)
            && shift
        {
            key = shifted.to_string();
            modifiers.retain(|modifier| modifier != "shift");
        } else if SHIFTED.iter().any(|(_, shifted)| *shifted == only) {
            modifiers.retain(|modifier| modifier != "shift");
        }
    }
    if key == "tab" && shift {
        key = "backtab".to_owned();
        modifiers.retain(|modifier| modifier != "shift");
    }
    modifiers.sort_unstable();
    modifiers.dedup();
    modifiers.push(key);
    modifiers.join("+")
}

/// Whether one press could reach both keys: a window shortcut reaches
/// every part but the keys a panel claims first; a list shortcut, while
/// the list has the focus, also takes the pane's keys; the pane's and the
/// panel's handlers never have the focus together.
fn can_meet(one: Catcher, other: Catcher) -> bool {
    use Catcher as C;
    match (one, other) {
        (C::Window, C::PlacesFirst) | (C::PlacesFirst, C::Window) => false,
        (C::Window, _) | (_, C::Window) => true,
        (C::List | C::Pane, C::List | C::Pane) => true,
        (C::Places | C::PlacesFirst, C::Places | C::PlacesFirst) => true,
        _ => false,
    }
}

/// Pairs of commands whose keys one press could reach both of, which Qt
/// treats as ambiguous and runs neither: each as (id, key, id, key).
pub fn clashes() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    clashes_in(COMMANDS)
}

fn clashes_in(
    commands: &[Command],
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    let keys: Vec<(&Command, &Key, String)> = commands
        .iter()
        .flat_map(|command| {
            command
                .keys
                .iter()
                .map(move |key| (command, key, canonical(key.sequence)))
        })
        .collect();
    let mut found = Vec::new();
    for (at, (one, one_key, one_form)) in keys.iter().enumerate() {
        for (other, other_key, other_form) in &keys[at + 1..] {
            let same_command = one.id == other.id;
            let clash = if same_command {
                // A command may list Qt's other names for one press (the
                // window's Key acts on an ambiguous match), never one key twice.
                one_key == other_key
            } else {
                one_form == other_form && can_meet(one_key.catcher, other_key.catcher)
            };
            if clash {
                found.push((one.id, one_key.sequence, other.id, other_key.sequence));
            }
        }
    }
    found
}

/// Every command, in the palette's order.
pub const COMMANDS: &[Command] = &[
    Command {
        id: "go-back",
        label: "Navigation: Back",
        action: Action::Pane(PaneCommand::Back),
        keys: &[window("Alt+Left"), pane("Backspace"), window("Back")],
    },
    Command {
        id: "go-forward",
        label: "Navigation: Forward",
        action: Action::Pane(PaneCommand::Forward),
        keys: &[window("Alt+Right"), window("Forward")],
    },
    Command {
        id: "go-up",
        label: "Navigation: Up one folder",
        action: Action::Pane(PaneCommand::Up),
        keys: &[window("Alt+Up")],
    },
    Command {
        id: "go-home",
        label: "Navigation: Home folder",
        action: Action::Pane(PaneCommand::Home),
        keys: &[window("Alt+Home")],
    },
    Command {
        id: "edit-location",
        label: "Navigation: Enter a location",
        action: Action::Window(WindowCommand::EditLocation),
        keys: &[window("Ctrl+L"), window("Alt+D"), window("F6")],
    },
    Command {
        id: "reload",
        label: "Navigation: Reload folder",
        action: Action::Pane(PaneCommand::Reload),
        keys: &[window("F5"), window("Refresh")],
    },
    Command {
        id: "open-terminal",
        label: "Navigation: Open terminal here",
        action: Action::Pane(PaneCommand::OpenTerminal),
        keys: &[window("Shift+F4")],
    },
    Command {
        id: "activate",
        label: "Navigation: Open selected folder",
        action: Action::Pane(PaneCommand::Activate),
        keys: &[pane("Return"), pane("Enter")],
    },
    Command {
        id: "show-icons",
        label: "View: Icons",
        action: Action::Pane(PaneCommand::Show(ViewMode::Icons)),
        keys: &[window("Ctrl+1")],
    },
    Command {
        id: "show-compact",
        label: "View: Compact",
        action: Action::Pane(PaneCommand::Show(ViewMode::Compact)),
        keys: &[window("Ctrl+2")],
    },
    Command {
        id: "show-details",
        label: "View: Details",
        action: Action::Pane(PaneCommand::Show(ViewMode::Details)),
        keys: &[window("Ctrl+3")],
    },
    Command {
        id: "toggle-hidden",
        label: "View: Show or hide hidden files",
        action: Action::Pane(PaneCommand::ToggleHidden),
        keys: &[window("Ctrl+H"), window("Alt+.")],
    },
    Command {
        id: "show-filter",
        label: "View: Filter by name",
        action: Action::Window(WindowCommand::ShowFilter),
        keys: &[window("Ctrl+I"), pane("/")],
    },
    Command {
        id: "toggle-folders-first",
        label: "View: Folders first",
        action: Action::Pane(PaneCommand::ToggleFoldersFirst),
        keys: &[],
    },
    Command {
        id: "zoom-in",
        label: "View: Zoom in",
        action: Action::Pane(PaneCommand::ZoomIn),
        keys: &[window("Ctrl++"), window("Ctrl+=")],
    },
    Command {
        id: "zoom-out",
        label: "View: Zoom out",
        action: Action::Pane(PaneCommand::ZoomOut),
        keys: &[window("Ctrl+-")],
    },
    Command {
        id: "zoom-reset",
        label: "View: Reset zoom",
        action: Action::Pane(PaneCommand::ZoomReset),
        keys: &[window("Ctrl+0")],
    },
    Command {
        id: "sort-name",
        label: "View: Sort by name (toggle direction)",
        action: Action::Pane(PaneCommand::SortBy(SortKey::Name)),
        keys: &[],
    },
    Command {
        id: "sort-size",
        label: "View: Sort by size (toggle direction)",
        action: Action::Pane(PaneCommand::SortBy(SortKey::Size)),
        keys: &[],
    },
    Command {
        id: "sort-modified",
        label: "View: Sort by modified date (toggle direction)",
        action: Action::Pane(PaneCommand::SortBy(SortKey::Modified)),
        keys: &[],
    },
    Command {
        id: "sort-type",
        label: "View: Sort by type (toggle direction)",
        action: Action::Pane(PaneCommand::SortBy(SortKey::Type)),
        keys: &[],
    },
    Command {
        id: "new-folder",
        label: "Files: New folder",
        action: Action::Pane(PaneCommand::NewFolder),
        keys: &[pane("F10"), pane("Ctrl+Shift+N")],
    },
    Command {
        id: "new-file",
        label: "Files: New file",
        action: Action::Pane(PaneCommand::NewFile),
        keys: &[window("Ctrl+Alt+N")],
    },
    Command {
        id: "rename",
        label: "Files: Rename selection",
        action: Action::Pane(PaneCommand::Rename),
        keys: &[pane("F2")],
    },
    Command {
        id: "show-properties",
        label: "Files: Properties",
        action: Action::Pane(PaneCommand::Properties),
        keys: &[list("Alt+Return"), list("Alt+Enter")],
    },
    Command {
        id: "recover-renames",
        label: "Files: Recover original names after interruption",
        action: Action::Pane(PaneCommand::RecoverRenames),
        keys: &[],
    },
    Command {
        id: "copy",
        label: "Files: Copy selection",
        action: Action::Pane(PaneCommand::Copy),
        keys: &[list("Ctrl+C")],
    },
    Command {
        id: "cut",
        label: "Files: Cut selection",
        action: Action::Pane(PaneCommand::Cut),
        keys: &[list("Ctrl+X")],
    },
    Command {
        id: "paste",
        label: "Files: Paste into this folder",
        action: Action::Pane(PaneCommand::Paste),
        keys: &[list("Ctrl+V")],
    },
    Command {
        id: "copy-location",
        label: "Files: Copy location",
        action: Action::Pane(PaneCommand::CopyLocation),
        keys: &[list("Ctrl+Alt+C")],
    },
    Command {
        id: "duplicate",
        label: "Files: Duplicate here",
        action: Action::Pane(PaneCommand::Duplicate),
        keys: &[list("Ctrl+D")],
    },
    Command {
        id: "extract-here",
        label: "Files: Extract here",
        action: Action::Pane(PaneCommand::Extract(ExtractHow::Here)),
        keys: &[],
    },
    Command {
        id: "extract-into-folder",
        label: "Files: Extract into a new folder",
        action: Action::Pane(PaneCommand::Extract(ExtractHow::NewFolder)),
        keys: &[],
    },
    Command {
        id: "extract-and-trash",
        label: "Files: Extract and trash archive",
        action: Action::Pane(PaneCommand::Extract(ExtractHow::AndTrash)),
        keys: &[],
    },
    Command {
        id: "extract-to",
        label: "Files: Extract to\u{2026}",
        action: Action::Pane(PaneCommand::Extract(ExtractHow::To)),
        keys: &[],
    },
    Command {
        id: "compress-zip",
        label: "Files: Compress to ZIP",
        action: Action::Pane(PaneCommand::Compress(CompressHow::Zip)),
        keys: &[],
    },
    Command {
        id: "compress-tar-gz",
        label: "Files: Compress to TAR.GZ",
        action: Action::Pane(PaneCommand::Compress(CompressHow::TarGz)),
        keys: &[],
    },
    Command {
        id: "compress-choose",
        label: "Files: Compress to\u{2026}",
        action: Action::Pane(PaneCommand::Compress(CompressHow::Choose)),
        keys: &[],
    },
    Command {
        id: "copy-to-other-view",
        label: "Files: Copy selection to the other view",
        action: Action::Window(WindowCommand::CopyToOtherView),
        keys: &[list("Shift+F5")],
    },
    Command {
        id: "move-to-other-view",
        label: "Files: Move selection to the other view",
        action: Action::Window(WindowCommand::MoveToOtherView),
        keys: &[list("Shift+F6")],
    },
    Command {
        id: "move-to-trash",
        label: "Files: Move selection to trash",
        action: Action::Pane(PaneCommand::MoveToTrash),
        keys: &[list("Delete")],
    },
    Command {
        id: "delete-permanently",
        label: "Files: Delete selection permanently…",
        action: Action::Pane(PaneCommand::DeletePermanently),
        keys: &[list("Shift+Delete")],
    },
    Command {
        id: "undo",
        label: "Files: Undo last operation",
        action: Action::Window(WindowCommand::Undo),
        keys: &[window("Ctrl+Z")],
    },
    Command {
        id: "stop-operations",
        label: "Files: Stop running operations",
        action: Action::Window(WindowCommand::StopOperations),
        keys: &[],
    },
    Command {
        id: "dismiss-notice",
        label: "Files: Dismiss operation message",
        action: Action::Window(WindowCommand::DismissNotice),
        keys: &[],
    },
    Command {
        id: "trash-actions",
        label: "Trash: Show trash actions",
        action: Action::Window(WindowCommand::TrashActions),
        keys: &[],
    },
    Command {
        id: "open-trash",
        label: "Trash: Open trash",
        action: Action::Pane(PaneCommand::OpenTrash),
        keys: &[],
    },
    Command {
        id: "restore-trash",
        label: "Trash: Restore selection",
        action: Action::Pane(PaneCommand::RestoreTrash),
        keys: &[],
    },
    Command {
        id: "empty-trash",
        label: "Trash: Empty this trash…",
        action: Action::Pane(PaneCommand::EmptyTrash),
        keys: &[],
    },
    Command {
        id: "select-all",
        label: "Selection: Select all",
        action: Action::Pane(PaneCommand::SelectAll),
        keys: &[pane("Ctrl+A")],
    },
    Command {
        id: "invert-selection",
        label: "Selection: Invert",
        action: Action::Pane(PaneCommand::InvertSelection),
        keys: &[pane("Ctrl+Shift+A")],
    },
    Command {
        id: "cancel-edit",
        label: "Selection: Clear",
        action: Action::Pane(PaneCommand::ClearSelection),
        keys: &[pane("Escape")],
    },
    Command {
        id: "select-previous",
        label: "Selection: Previous row",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Up,
            extend: false,
        }),
        keys: &[pane("Up")],
    },
    Command {
        id: "select-next",
        label: "Selection: Next row",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Down,
            extend: false,
        }),
        keys: &[pane("Down")],
    },
    Command {
        id: "select-first",
        label: "Selection: First entry",
        action: Action::Pane(PaneCommand::Move {
            step: Step::First,
            extend: false,
        }),
        keys: &[pane("Home")],
    },
    Command {
        id: "select-last",
        label: "Selection: Last entry",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Last,
            extend: false,
        }),
        keys: &[pane("End")],
    },
    Command {
        id: "page-up",
        label: "Selection: Previous page",
        action: Action::Pane(PaneCommand::Move {
            step: Step::PageUp,
            extend: false,
        }),
        keys: &[pane("PgUp")],
    },
    Command {
        id: "page-down",
        label: "Selection: Next page",
        action: Action::Pane(PaneCommand::Move {
            step: Step::PageDown,
            extend: false,
        }),
        keys: &[pane("PgDown")],
    },
    Command {
        id: "extend-previous",
        label: "Selection: Extend to previous row",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Up,
            extend: true,
        }),
        keys: &[pane("Shift+Up")],
    },
    Command {
        id: "extend-next",
        label: "Selection: Extend to next row",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Down,
            extend: true,
        }),
        keys: &[pane("Shift+Down")],
    },
    Command {
        id: "extend-first",
        label: "Selection: Extend to first entry",
        action: Action::Pane(PaneCommand::Move {
            step: Step::First,
            extend: true,
        }),
        keys: &[pane("Shift+Home")],
    },
    Command {
        id: "extend-last",
        label: "Selection: Extend to last entry",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Last,
            extend: true,
        }),
        keys: &[pane("Shift+End")],
    },
    Command {
        id: "extend-page-up",
        label: "Selection: Extend to previous page",
        action: Action::Pane(PaneCommand::Move {
            step: Step::PageUp,
            extend: true,
        }),
        keys: &[pane("Shift+PgUp")],
    },
    Command {
        id: "extend-page-down",
        label: "Selection: Extend to next page",
        action: Action::Pane(PaneCommand::Move {
            step: Step::PageDown,
            extend: true,
        }),
        keys: &[pane("Shift+PgDown")],
    },
    Command {
        id: "select-left",
        label: "Selection: Move left",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Left,
            extend: false,
        }),
        keys: &[pane("Left")],
    },
    Command {
        id: "select-right",
        label: "Selection: Move right",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Right,
            extend: false,
        }),
        keys: &[pane("Right")],
    },
    Command {
        id: "extend-left",
        label: "Selection: Extend left",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Left,
            extend: true,
        }),
        keys: &[pane("Shift+Left")],
    },
    Command {
        id: "extend-right",
        label: "Selection: Extend right",
        action: Action::Pane(PaneCommand::Move {
            step: Step::Right,
            extend: true,
        }),
        keys: &[pane("Shift+Right")],
    },
    Command {
        id: "new-tab",
        label: "Workspace: New tab",
        action: Action::Window(WindowCommand::NewTab),
        keys: &[window("Ctrl+T")],
    },
    Command {
        id: "reopen-closed-tab",
        label: "Workspace: Reopen closed tab",
        action: Action::Window(WindowCommand::ReopenClosedTab),
        keys: &[window("Ctrl+Shift+T")],
    },
    Command {
        id: "focus-other-view",
        label: "Workspace: Focus the other view",
        action: Action::Window(WindowCommand::FocusOtherView),
        keys: &[window("Ctrl+F3")],
    },
    Command {
        id: "focus-places",
        label: "Workspace: Focus the places panel",
        action: Action::Window(WindowCommand::FocusPlaces),
        keys: &[window("Ctrl+P")],
    },
    Command {
        id: "quit",
        label: "Workspace: Quit",
        action: Action::Window(WindowCommand::Quit),
        keys: &[window("Ctrl+Q")],
    },
    Command {
        id: "open-settings",
        label: "Workspace: Settings…",
        action: Action::Window(WindowCommand::OpenSettings),
        keys: &[
            window("Ctrl+Shift+,"),
            window("Ctrl+Shift+<"),
            window("Ctrl+<"),
        ],
    },
    Command {
        id: "close-tab",
        label: "Workspace: Close tab or window",
        action: Action::Window(WindowCommand::CloseTab),
        keys: &[window("Ctrl+W")],
    },
    Command {
        id: "next-tab",
        label: "Workspace: Next tab",
        action: Action::Window(WindowCommand::NextTab),
        keys: &[window("Ctrl+Tab"), window("Ctrl+PgDown")],
    },
    Command {
        id: "previous-tab",
        label: "Workspace: Previous tab",
        action: Action::Window(WindowCommand::PreviousTab),
        keys: &[window("Ctrl+Shift+Tab"), window("Ctrl+PgUp")],
    },
    Command {
        id: "move-tab-left",
        label: "Workspace: Move tab left",
        action: Action::Window(WindowCommand::MoveTabLeft),
        keys: &[window("Ctrl+Shift+PgUp")],
    },
    Command {
        id: "move-tab-right",
        label: "Workspace: Move tab right",
        action: Action::Window(WindowCommand::MoveTabRight),
        keys: &[window("Ctrl+Shift+PgDown")],
    },
    Command {
        id: "activate-tab-1",
        label: "Workspace: Go to tab 1",
        action: Action::Window(WindowCommand::ActivateTab(1)),
        keys: &[window("Alt+1")],
    },
    Command {
        id: "activate-tab-2",
        label: "Workspace: Go to tab 2",
        action: Action::Window(WindowCommand::ActivateTab(2)),
        keys: &[window("Alt+2")],
    },
    Command {
        id: "activate-tab-3",
        label: "Workspace: Go to tab 3",
        action: Action::Window(WindowCommand::ActivateTab(3)),
        keys: &[window("Alt+3")],
    },
    Command {
        id: "activate-tab-4",
        label: "Workspace: Go to tab 4",
        action: Action::Window(WindowCommand::ActivateTab(4)),
        keys: &[window("Alt+4")],
    },
    Command {
        id: "activate-tab-5",
        label: "Workspace: Go to tab 5",
        action: Action::Window(WindowCommand::ActivateTab(5)),
        keys: &[window("Alt+5")],
    },
    Command {
        id: "activate-tab-6",
        label: "Workspace: Go to tab 6",
        action: Action::Window(WindowCommand::ActivateTab(6)),
        keys: &[window("Alt+6")],
    },
    Command {
        id: "activate-tab-7",
        label: "Workspace: Go to tab 7",
        action: Action::Window(WindowCommand::ActivateTab(7)),
        keys: &[window("Alt+7")],
    },
    Command {
        id: "activate-tab-8",
        label: "Workspace: Go to tab 8",
        action: Action::Window(WindowCommand::ActivateTab(8)),
        keys: &[window("Alt+8")],
    },
    Command {
        id: "activate-tab-9",
        label: "Workspace: Go to tab 9",
        action: Action::Window(WindowCommand::ActivateTab(9)),
        keys: &[window("Alt+9")],
    },
    Command {
        id: "activate-last-tab",
        label: "Workspace: Go to last tab",
        action: Action::Window(WindowCommand::ActivateTab(0)),
        keys: &[window("Alt+0")],
    },
    Command {
        id: "toggle-split",
        label: "Workspace: Toggle split view",
        action: Action::Window(WindowCommand::ToggleSplit),
        keys: &[window("F3")],
    },
    Command {
        id: "other-pane",
        label: "Workspace: Focus next pane or places",
        action: Action::Window(WindowCommand::OtherPane),
        keys: &[pane("Tab")],
    },
    Command {
        id: "toggle-sidebar",
        label: "Workspace: Show or hide places",
        action: Action::Window(WindowCommand::ToggleSidebar),
        keys: &[window("F9")],
    },
    Command {
        id: "add-place",
        label: "Places: Add current folder",
        action: Action::Window(WindowCommand::AddPlace),
        keys: &[window("Ctrl+Shift+D")],
    },
    Command {
        id: "new-group",
        label: "Places: New group",
        action: Action::Places(PlacesCommand::NewGroup),
        keys: &[window("Ctrl+Shift+G")],
    },
    Command {
        id: "rename-row",
        label: "Places: Rename selected place or group",
        action: Action::Places(PlacesCommand::Rename),
        keys: &[places("F2")],
    },
    Command {
        id: "remove-row",
        label: "Places: Remove selected place or group",
        action: Action::Places(PlacesCommand::Remove),
        keys: &[places("Delete")],
    },
    Command {
        id: "hide-row",
        label: "Places: Hide or show selected place",
        action: Action::Places(PlacesCommand::ToggleHide),
        keys: &[places("H")],
    },
    Command {
        id: "toggle-hidden-places",
        label: "Places: List or hide hidden places",
        action: Action::Places(PlacesCommand::ToggleHiddenPlaces),
        keys: &[],
    },
    Command {
        id: "move-row-up",
        label: "Places: Move selected row up",
        action: Action::Places(PlacesCommand::MoveUp),
        keys: &[places_first("Alt+Up")],
    },
    Command {
        id: "move-row-down",
        label: "Places: Move selected row down",
        action: Action::Places(PlacesCommand::MoveDown),
        keys: &[places_first("Alt+Down")],
    },
    Command {
        id: "collapse",
        label: "Places: Collapse selected group",
        action: Action::Places(PlacesCommand::Collapse),
        keys: &[places("Left")],
    },
    Command {
        id: "expand",
        label: "Places: Expand selected group",
        action: Action::Places(PlacesCommand::Expand),
        keys: &[places("Right")],
    },
    Command {
        id: "next-group",
        label: "Places: Move selected place to next group",
        action: Action::Places(PlacesCommand::NextGroup),
        keys: &[places("G")],
    },
    Command {
        id: "focus-previous",
        label: "Places: Focus previous row",
        action: Action::Places(PlacesCommand::FocusPrevious),
        keys: &[places("Up")],
    },
    Command {
        id: "focus-next",
        label: "Places: Focus next row",
        action: Action::Places(PlacesCommand::FocusNext),
        keys: &[places("Down")],
    },
    Command {
        id: "open",
        label: "Places: Open selected place or mount selected device",
        action: Action::Places(PlacesCommand::Open),
        keys: &[places("Return"), places("Enter")],
    },
    Command {
        id: "eject",
        label: "Places: Unmount or safely remove selected device",
        action: Action::Places(PlacesCommand::Eject),
        keys: &[places("E")],
    },
    Command {
        id: "open-palette",
        label: "Workspace: Command palette",
        action: Action::Window(WindowCommand::OpenPalette),
        keys: &[window("Ctrl+Shift+P")],
    },
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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

    #[test]
    fn every_id_parses_to_its_own_typed_command_and_target() {
        for command in COMMANDS {
            let action = parse(command.id).unwrap();
            assert_eq!(action, command.action);
            // One command per action, so a menu row or toolbar button finds
            // its key from what it does.
            assert_eq!(of(action).map(|found| found.id), Some(command.id));
        }
        assert_eq!(parse("go-back"), Some(Action::Pane(PaneCommand::Back)));
        assert_eq!(
            parse("select-next"),
            Some(Action::Pane(PaneCommand::Move {
                step: Step::Down,
                extend: false
            }))
        );
        assert_eq!(
            parse("rename-row"),
            Some(Action::Places(PlacesCommand::Rename))
        );
        assert_eq!(
            parse("activate-last-tab"),
            Some(Action::Window(WindowCommand::ActivateTab(0)))
        );
        assert_eq!(find("new-group").map(Command::target), Some(Target::Places));
        assert_eq!(find("new-tab").map(Command::target), Some(Target::Window));
        assert_eq!(parse("no-such-command"), None);
    }

    #[test]
    fn no_two_commands_share_a_key() {
        assert_eq!(clashes(), []);
    }

    fn command(id: &'static str, keys: Vec<Key>) -> Command {
        Command {
            id,
            label: "Test: command",
            action: Action::Window(WindowCommand::Quit),
            keys: Vec::leak(keys),
        }
    }

    #[test]
    fn keys_clash_where_one_press_reaches_both() {
        let pairs = |table: &[Command]| clashes_in(table).len();
        // Qt's other names for one press (AGENTS.md: Shift+, is also "<").
        assert_eq!(
            pairs(&[
                command("a", vec![window("Ctrl+Shift+,")]),
                command("b", vec![window("Ctrl+<")]),
            ]),
            1
        );
        // A window shortcut takes the key from the places panel…
        assert_eq!(
            pairs(&[
                command("a", vec![window("Alt+Up")]),
                command("b", vec![places("Alt+Up")]),
            ]),
            1
        );
        // …unless the panel claims it first.
        assert_eq!(
            pairs(&[
                command("a", vec![window("Alt+Up")]),
                command("b", vec![places_first("Alt+Up")]),
            ]),
            0
        );
        // The pane and the panel never have the focus together.
        assert_eq!(
            pairs(&[
                command("a", vec![pane("F2")]),
                command("b", vec![places("F2")]),
            ]),
            0
        );
        // A list shortcut takes the pane's key.
        assert_eq!(
            pairs(&[
                command("a", vec![list("Ctrl+A")]),
                command("b", vec![pane("Ctrl+A")]),
            ]),
            1
        );
        // One command lists Qt's other names, never one key twice.
        assert_eq!(
            pairs(&[command("a", vec![window("Ctrl+Shift+,"), window("Ctrl+<")])]),
            0
        );
        assert_eq!(pairs(&[command("a", vec![window("F5"), window("F5")])]), 1);
    }

    #[test]
    fn sequences_qt_treats_as_one_press_have_one_form() {
        assert_eq!(canonical("Ctrl+Shift+,"), canonical("Ctrl+<"));
        assert_eq!(canonical("Ctrl+Shift+<"), canonical("Ctrl+<"));
        assert_eq!(canonical("Ctrl+Shift+="), canonical("Ctrl++"));
        assert_eq!(canonical("Shift+Ctrl+a"), canonical("Ctrl+Shift+A"));
        assert_eq!(canonical("Del"), canonical("Delete"));
        assert_eq!(canonical("Ctrl+PgUp"), canonical("Ctrl+PageUp"));
        assert_eq!(canonical("Ctrl+Shift+Tab"), canonical("Ctrl+Backtab"));
        assert_ne!(canonical("Ctrl+="), canonical("Ctrl++"));
        assert_ne!(canonical("Return"), canonical("Enter"));
        assert_ne!(canonical("Alt+Left"), canonical("Back"));
        assert_ne!(canonical("Shift+F10"), canonical("F10"));
    }

    #[test]
    fn keys_are_shown_as_people_read_them_once_each() {
        assert_eq!(display("Ctrl+Shift+PgDown"), "Ctrl+Shift+Page Down");
        assert_eq!(display("Ctrl++"), "Ctrl++");
        let shown = |id: &str| find(id).unwrap().shown_keys();
        assert_eq!(shown("go-back"), ["Alt+Left", "Backspace", "Back key"]);
        assert_eq!(shown("show-properties"), ["Alt+Enter"]);
        assert_eq!(shown("open-settings"), ["Ctrl+Shift+,"]);
        assert_eq!(shown("zoom-in"), ["Ctrl++", "Ctrl+="]);
        assert_eq!(
            find("move-to-trash").unwrap().shown_key().unwrap(),
            "Delete"
        );
    }

    #[test]
    fn the_window_listens_to_its_shortcuts_and_leaves_text_keys_to_the_handlers() {
        let shortcuts = window_shortcuts();
        let of = |id: &str, list: bool| {
            shortcuts
                .iter()
                .find(|shortcut| shortcut.id == id && shortcut.list == list)
                .map(|shortcut| shortcut.keys.clone())
        };
        assert_eq!(of("go-back", false), Some(vec!["Alt+Left", "Back"]));
        assert_eq!(of("move-to-trash", true), Some(vec!["Delete"]));
        assert_eq!(of("move-to-trash", false), None);
        // Keys that also edit text never become window shortcuts.
        for text_key in [
            "Backspace",
            "Return",
            "Enter",
            "Up",
            "Home",
            "End",
            "Tab",
            "Escape",
        ] {
            assert!(
                shortcuts
                    .iter()
                    .all(|shortcut| !shortcut.keys.contains(&text_key)),
                "{text_key} is a window shortcut"
            );
        }
    }

    /// The key a menu row, the palette, the toolbar and the Settings page
    /// show for one command are the table's: W-123 found Back shown as
    /// Backspace in the palette and Alt+Left on the toolbar.
    #[test]
    fn every_surface_shows_the_table_s_key() {
        use crate::{
            menu::{Command as _, MenuCommand},
            place_menu::PlaceCommand,
            toolbar_items::{BUILT_IN, Item},
        };
        let palette = |id: &str| {
            crate::palette::command_candidates()
                .into_iter()
                .find(|candidate| find(id).is_some_and(|command| command.label == candidate.label))
                .map(|candidate| candidate.shortcut)
                .filter(|shortcut| !shortcut.is_empty())
        };
        let settings = |id: &str| {
            let command = find(id).unwrap();
            let name = command.label.split_once(": ").unwrap().1;
            crate::settings_form::shortcut_groups("")
                .into_iter()
                .flat_map(|(_, list)| list)
                .find(|shortcut| shortcut.name == name)
                .and_then(|shortcut| shortcut.keys.first().cloned())
        };
        let mut checked = 0;
        for item in BUILT_IN.iter().chain([&Item::Actions]) {
            if let Some(id) = item.command_id() {
                assert_eq!(item.keys(), palette(id), "toolbar {item:?}");
                assert_eq!(item.keys(), settings(id), "toolbar {item:?}");
                checked += 1;
            }
        }
        assert_eq!(Item::Back.keys().as_deref(), Some("Alt+Left"));
        for row in [
            MenuCommand::Open,
            MenuCommand::Copy,
            MenuCommand::Cut,
            MenuCommand::Paste,
            MenuCommand::CopyLocation,
            MenuCommand::Duplicate,
            MenuCommand::Rename,
            MenuCommand::MoveToTrash,
            MenuCommand::Delete,
            MenuCommand::Properties,
            MenuCommand::NewFolder,
            MenuCommand::NewFile,
            MenuCommand::Restore,
        ] {
            let id = row
                .action()
                .and_then(|action| of(Action::Pane(action)))
                .unwrap()
                .id;
            assert_eq!(row.shortcut(), palette(id), "menu {row:?}");
            checked += 1;
        }
        for row in [
            PlaceCommand::Open,
            PlaceCommand::Rename,
            PlaceCommand::Hide,
            PlaceCommand::Remove,
            PlaceCommand::MoveUp,
            PlaceCommand::Collapse,
            PlaceCommand::NewGroup,
            PlaceCommand::AddShownFolder,
        ] {
            let id = row.action().and_then(of).unwrap().id;
            assert_eq!(row.shortcut(), palette(id), "places menu {row:?}");
            checked += 1;
        }
        assert!(checked > 25);
    }

    /// KEYBOARD.md's rows (action, then the commands it covers). Rows left
    /// out name no command (columns view, redo, new window, type-ahead,
    /// context menu) or describe keys in words (moving the selection, the
    /// places panel's own keys).
    const KEYBOARD_ROWS: &[(&str, &[&str])] = &[
        ("Back", &["go-back"]),
        ("Forward", &["go-forward"]),
        ("Up one folder", &["go-up"]),
        ("Home folder", &["go-home"]),
        ("Reload", &["reload"]),
        ("Edit location", &["edit-location"]),
        ("Open selected", &["activate"]),
        ("Select all", &["select-all"]),
        ("Invert selection", &["invert-selection"]),
        ("Clear selection", &["cancel-edit"]),
        (
            "Icons / Compact / Details",
            &["show-icons", "show-compact", "show-details"],
        ),
        (
            "Zoom in / out / reset",
            &["zoom-in", "zoom-out", "zoom-reset"],
        ),
        ("Show hidden files", &["toggle-hidden"]),
        ("Filter by name", &["show-filter"]),
        ("New folder", &["new-folder"]),
        ("Open terminal here", &["open-terminal"]),
        ("New file", &["new-file"]),
        ("Rename", &["rename"]),
        ("Move to trash", &["move-to-trash"]),
        ("Delete permanently", &["delete-permanently"]),
        ("Cut / Copy / Paste", &["cut", "copy", "paste"]),
        ("Undo", &["undo"]),
        ("Properties", &["show-properties"]),
        ("Duplicate here", &["duplicate"]),
        ("Copy location", &["copy-location"]),
        ("New tab", &["new-tab"]),
        ("Close tab", &["close-tab"]),
        ("Next / previous tab", &["next-tab", "previous-tab"]),
        (
            "Activate tab 1–9 / last",
            &[
                "activate-tab-1",
                "activate-tab-2",
                "activate-tab-3",
                "activate-tab-4",
                "activate-tab-5",
                "activate-tab-6",
                "activate-tab-7",
                "activate-tab-8",
                "activate-tab-9",
                "activate-last-tab",
            ],
        ),
        (
            "Move tab left / right",
            &["move-tab-left", "move-tab-right"],
        ),
        ("Reopen closed tab", &["reopen-closed-tab"]),
        ("Split view", &["toggle-split"]),
        ("Focus other view", &["focus-other-view", "other-pane"]),
        (
            "Copy / move to other view",
            &["copy-to-other-view", "move-to-other-view"],
        ),
        ("Places panel shown", &["toggle-sidebar"]),
        ("Add folder to Places", &["add-place"]),
        ("Focus places panel", &["focus-places"]),
        ("Quit", &["quit"]),
        ("Command palette", &["open-palette"]),
        ("Settings", &["open-settings"]),
    ];

    /// Whether a piece of a KEYBOARD.md cell names a key.
    fn names_a_key(piece: &str) -> bool {
        const NAMED: [&str; 17] = [
            "Enter",
            "Delete",
            "Backspace",
            "Escape",
            "Tab",
            "Home",
            "End",
            "Up",
            "Down",
            "Left",
            "Right",
            "Page Up",
            "Page Down",
            "Back key",
            "Forward key",
            "Refresh key",
            "Menu key",
        ];
        ["Ctrl+", "Alt+", "Shift+", "Meta+"]
            .iter()
            .any(|modifier| piece.starts_with(modifier))
            || NAMED.contains(&piece)
            || piece.chars().count() == 1
            || (piece.starts_with('F') && piece[1..].parse::<u8>().is_ok())
    }

    /// A cell's text without its asides in brackets.
    fn without_asides(cell: &str) -> String {
        let mut depth = 0;
        cell.chars()
            .filter(|c| {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        return false;
                    }
                    _ => {}
                }
                depth == 0
            })
            .collect()
    }

    #[test]
    fn keyboard_md_lists_the_table_s_keys() {
        let doc = include_str!("../../../docs/project/KEYBOARD.md");
        let mut covered: Vec<&str> = Vec::new();
        for (action, ids) in KEYBOARD_ROWS {
            let row = doc
                .lines()
                .find(|line| line.starts_with(&format!("| {action} |")))
                .unwrap_or_else(|| panic!("KEYBOARD.md has no row {action}"));
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            let (dolphin, rustphin) = (cells[2], cells[3]);
            // "same" stands for Dolphin's keys.
            let text = match rustphin.strip_prefix("same") {
                Some(rest) => format!("{dolphin}{rest}"),
                None => rustphin.to_owned(),
            };
            let keys: Vec<String> = ids
                .iter()
                .flat_map(|id| find(id).unwrap().shown_keys())
                .collect();
            for key in &keys {
                assert!(
                    text.contains(key.as_str()),
                    "{action}: {key} missing from \"{text}\""
                );
            }
            for piece in without_asides(&text)
                .split(", ")
                .flat_map(|piece| piece.split(" / "))
                .map(|piece| piece.split(':').next().unwrap_or_default().trim())
                .filter(|piece| names_a_key(piece))
            {
                assert!(
                    keys.iter().any(|key| key == piece),
                    "{action}: KEYBOARD.md has {piece}, the table does not"
                );
            }
            covered.extend(ids.iter());
        }
        for command in COMMANDS {
            let moves = matches!(command.action, Action::Pane(PaneCommand::Move { .. }));
            if !command.keys.is_empty() && command.target() != Target::Places && !moves {
                assert!(
                    covered.contains(&command.id),
                    "{} is not in KEYBOARD.md",
                    command.id
                );
            }
        }
    }
}
