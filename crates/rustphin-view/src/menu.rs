//! What a right-click offers for the current selection.
//!
//! The list is decided here, away from drawing, so a missing operation fails a
//! test rather than only disappearing on screen.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// One application that can open the selected file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppChoice {
    /// Desktop-file id.
    pub id: String,
    /// The name shown in the menu.
    pub name: String,
    /// The application's own icon file, if it has one.
    pub icon: Option<Arc<Path>>,
}

/// What a menu row shows before its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowIcon {
    /// One of the interface's Lucide outlines.
    Symbol(crate::icons::Icon),
    /// An application's own icon, in colour.
    Image(Arc<Path>),
}

/// An operation the menu can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuCommand {
    /// Open the selection: enter a folder, or activate a file.
    Open,
    /// Open the file with this application.
    OpenWith(AppChoice),
    /// Copy the selection to the clipboard.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste the clipboard into the folder being shown.
    Paste,
    /// Put the path (or remote address) of the first selected entry on the
    /// clipboard as text, as Dolphin's copy-location action.
    CopyLocation,
    /// Copy the selection beside itself as "name copy.ext" (Dolphin's
    /// duplicate-here action, D-021).
    Duplicate,
    /// Rename the selection.
    Rename,
    /// Move the selection to the trash.
    MoveToTrash,
    /// Delete the selection for good, after the usual confirmation.
    Delete,
    /// Show properties for the one selected entry.
    Properties,
    /// Extract the selected archives (W-111).
    Extract(ExtractHow),
    /// Compress the selection into a new archive (W-111).
    Compress(CompressHow),
    /// Bring the selection back from the trash.
    Restore,
    /// Ask before emptying the trash being shown.
    EmptyTrash,
    /// Browse another known trash's files.
    OpenTrash(PathBuf),
    /// Create a folder.
    NewFolder,
    /// Create an empty file.
    NewFile,
    /// Create the New menu's entry at `index` (W-108).
    NewFromTemplate {
        /// Position in the browser's list of templates.
        index: usize,
        /// Which picture the row shows.
        kind: TemplateKind,
    },
}

/// How the Extract menu unpacks (W-111, D-032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractHow {
    /// As Ark's "Extract here".
    Here,
    /// Into a new folder named after the archive.
    NewFolder,
    /// As "Extract here", then the archive goes to the trash.
    AndTrash,
    /// Into a folder chosen in KDE's folder dialog.
    To,
}

/// What the Compress menu makes (W-111, D-032).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressHow {
    /// "name.zip" beside the selection.
    Zip,
    /// "name.tar.gz" beside the selection.
    TarGz,
    /// Name, format and level chosen in a dialog.
    Choose,
}

/// What kind of file a New menu entry creates, for its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    /// A text file.
    Text,
    /// An empty file.
    Empty,
    /// An HTML page.
    Html,
    /// One of the user's templates.
    Other,
}

/// What a menu row runs: the file view's operations, or the places
/// panel's (`sidebar::PlaceCommand`).
pub trait Command: Clone {
    /// The key that does the same outside the menu, as the row shows it.
    fn shortcut(&self) -> Option<String>;
    /// What the row shows before its words.
    fn row_icon(&self) -> RowIcon;
}

impl Command for MenuCommand {
    fn shortcut(&self) -> Option<String> {
        MenuCommand::shortcut(self)
    }

    fn row_icon(&self) -> RowIcon {
        MenuCommand::row_icon(self)
    }
}

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry<C = MenuCommand> {
    /// An operation. Disabled rows are shown and skipped by the keyboard.
    Item {
        /// The words on the row.
        label: String,
        /// What choosing it does.
        command: C,
        /// Whether it can be chosen.
        enabled: bool,
    },
    /// A row that opens `items` beside the menu.
    Submenu {
        /// The words on the row.
        label: String,
        /// The icon before the words, as on every other row.
        icon: crate::icons::Icon,
        /// The rows of the second menu.
        items: Vec<MenuEntry<C>>,
    },
    /// A hairline between groups.
    Separator,
}

impl MenuCommand {
    /// The pane command the row runs, when it is one the palette and keys
    /// run too (W-123); rows tied to one application, trash or template
    /// have none.
    pub fn action(&self) -> Option<crate::commands::PaneCommand> {
        use crate::commands::PaneCommand as P;
        Some(match self {
            Self::Open => P::Activate,
            Self::Cut => P::Cut,
            Self::Copy => P::Copy,
            Self::Paste => P::Paste,
            Self::CopyLocation => P::CopyLocation,
            Self::Duplicate => P::Duplicate,
            Self::Rename => P::Rename,
            Self::MoveToTrash => P::MoveToTrash,
            Self::Delete => P::DeletePermanently,
            Self::Properties => P::Properties,
            Self::NewFolder => P::NewFolder,
            Self::NewFile => P::NewFile,
            Self::Extract(how) => P::Extract(*how),
            Self::Compress(how) => P::Compress(*how),
            Self::Restore => P::RestoreTrash,
            Self::EmptyTrash => P::EmptyTrash,
            Self::OpenWith(_) | Self::OpenTrash(_) | Self::NewFromTemplate { .. } => return None,
        })
    }

    /// The key that does the same outside the menu, as the row shows it
    /// (`commands::COMMANDS`).
    pub fn shortcut(&self) -> Option<String> {
        self.action()
            .and_then(|action| crate::commands::key_of(crate::commands::Action::Pane(action)))
    }

    /// What the row shows before its words: an application's own icon
    /// where there is one, else the operation's outline.
    pub fn row_icon(&self) -> RowIcon {
        match self {
            Self::OpenWith(AppChoice {
                icon: Some(icon), ..
            }) => RowIcon::Image(Arc::clone(icon)),
            _ => RowIcon::Symbol(self.icon()),
        }
    }

    /// The Lucide icon of the row.
    pub fn icon(&self) -> crate::icons::Icon {
        use crate::icons::Icon as I;
        match self {
            Self::Open => I::SquareArrowOutUpRight,
            Self::OpenWith(_) => I::AppWindow,
            Self::Cut => I::Scissors,
            Self::Copy => I::Copy,
            Self::Paste => I::ClipboardPaste,
            Self::CopyLocation => I::Link,
            Self::Duplicate => I::CopyPlus,
            Self::Rename => I::Pencil,
            Self::MoveToTrash | Self::EmptyTrash | Self::OpenTrash(_) => I::Trash,
            Self::Delete => I::CircleX,
            Self::Properties => I::Info,
            Self::Extract(ExtractHow::Here) => I::PackageOpen,
            Self::Extract(ExtractHow::NewFolder) => I::FolderArchive,
            Self::Extract(ExtractHow::AndTrash) => I::ArchiveX,
            Self::Extract(ExtractHow::To) => I::FolderInput,
            Self::Compress(CompressHow::Zip | CompressHow::TarGz) => I::FileArchive,
            Self::Compress(CompressHow::Choose) => I::Archive,
            Self::Restore => I::ArchiveRestore,
            Self::NewFolder => I::FolderPlus,
            Self::NewFile => I::FilePlus,
            Self::NewFromTemplate { kind, .. } => match kind {
                TemplateKind::Text => I::FileText,
                TemplateKind::Html => I::FileCode,
                TemplateKind::Empty | TemplateKind::Other => I::File,
            },
        }
    }
}

impl<C> MenuEntry<C> {
    fn enabled_choice(&self) -> bool {
        match self {
            Self::Item { enabled, .. } => *enabled,
            Self::Submenu { items, .. } => items.iter().any(Self::enabled_choice),
            Self::Separator => false,
        }
    }

    fn command(&self) -> Option<&C> {
        match self {
            Self::Item {
                command, enabled, ..
            } if *enabled => Some(command),
            _ => None,
        }
    }
}

/// A menu as QML draws it: each entry with its label, key, icon (a Lucide
/// name or a picture's `file://` address), whether it can be chosen, and
/// its path ("2" or "3.1") for [`command_at`]. [`OpenMenu`] numbers the paths.
pub fn to_json<C: Command>(entries: &[MenuEntry<C>]) -> serde_json::Value {
    json_under(entries, "")
}

/// [`to_json`] with every path starting `prefix.` (none when empty).
fn json_under<C: Command>(entries: &[MenuEntry<C>], prefix: &str) -> serde_json::Value {
    fn walk<C: Command>(entries: &[MenuEntry<C>], prefix: &str) -> Vec<serde_json::Value> {
        entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let path = if prefix.is_empty() {
                    index.to_string()
                } else {
                    format!("{prefix}.{index}")
                };
                match entry {
                    MenuEntry::Item {
                        label,
                        command,
                        enabled,
                    } => serde_json::json!({
                        "kind": "item",
                        "label": label,
                        "shortcut": command.shortcut().unwrap_or_default(),
                        "icon": icon_text(&command.row_icon()),
                        "enabled": enabled,
                        "path": path,
                    }),
                    MenuEntry::Submenu { label, icon, items } => serde_json::json!({
                        "kind": "submenu",
                        "label": label,
                        "icon": icon.lucide_name(),
                        "enabled": entry.enabled_choice(),
                        "path": path,
                        "items": walk(items, &path),
                    }),
                    MenuEntry::Separator => {
                        serde_json::json!({ "kind": "separator", "path": path })
                    }
                }
            })
            .collect()
    }
    serde_json::Value::Array(walk(entries, prefix))
}

/// How a row's icon is named for QML.
fn icon_text(icon: &RowIcon) -> String {
    match icon {
        RowIcon::Symbol(icon) => icon.lucide_name(),
        RowIcon::Image(path) => rustphin_platform::uri::file_uri(path),
    }
}

/// The command at `path` ("2", "3.1") of a menu, if it can be chosen.
pub fn command_at<'a, C>(entries: &'a [MenuEntry<C>], path: &str) -> Option<&'a C> {
    let mut entries = entries;
    let mut steps = path.split('.').peekable();
    while let Some(step) = steps.next() {
        let entry = entries.get(step.parse::<usize>().ok()?)?;
        match entry {
            MenuEntry::Submenu { items, .. } if steps.peek().is_some() => entries = items,
            _ if steps.peek().is_none() => return entry.command(),
            _ => return None,
        }
    }
    None
}

/// The menu a view has open: its entries and what it was opened for
/// (`Ctx`: the row right-clicked, or `()`), until an entry is chosen or
/// the menu closes (W-131).
///
/// Paths in its JSON carry the menu's number ("4.2.1"), so a choice made
/// in a menu that has since been replaced is not taken for the entry at
/// the same place in the new one.
#[derive(Debug, Clone)]
pub struct OpenMenu<C, Ctx = ()> {
    shown: Option<(u32, Vec<MenuEntry<C>>, Ctx)>,
    opened: u32,
}

impl<C, Ctx> Default for OpenMenu<C, Ctx> {
    fn default() -> Self {
        Self {
            shown: None,
            opened: 0,
        }
    }
}

impl<C: Command + Clone, Ctx> OpenMenu<C, Ctx> {
    /// Opens `entries` for `context`, replacing any menu open, and gives
    /// the JSON QML draws; `None`, leaving the open menu as it is, when
    /// there are no entries.
    pub fn open(&mut self, entries: Vec<MenuEntry<C>>, context: Ctx) -> Option<String> {
        if entries.is_empty() {
            return None;
        }
        self.opened = self.opened.wrapping_add(1);
        let json = json_under(&entries, &self.opened.to_string()).to_string();
        self.shown = Some((self.opened, entries, context));
        Some(json)
    }

    /// Whether a menu is open.
    pub fn is_open(&self) -> bool {
        self.shown.is_some()
    }

    /// The command at `path` of the open menu and what the menu was opened
    /// for, and the menu closes; `None` for a disabled entry, a submenu or
    /// a separator (the menu closes too). A path from a menu that is no
    /// longer open gives `None` and leaves the open one alone.
    pub fn choose(&mut self, path: &str) -> Option<(C, Ctx)> {
        let (number, rest) = path.split_once('.')?;
        let opened = self.shown.as_ref().map(|(opened, ..)| *opened)?;
        if number.parse::<u32>().ok()? != opened {
            return None;
        }
        let (_, entries, context) = self.shown.take()?;
        let command = command_at(&entries, rest)?.clone();
        Some((command, context))
    }

    /// Closes the menu without running anything.
    pub fn close(&mut self) {
        self.shown = None;
    }
}

/// The facts the menu is built from.
#[derive(Debug, Clone)]
pub struct MenuFacts {
    /// The menu is for the selection. Otherwise it is for empty space.
    pub on_selection: bool,
    /// How many entries are selected.
    pub count: usize,
    /// The only selected entry can be entered, when `count` is 1.
    pub single_container: bool,
    /// The folder being shown is a trash.
    pub in_trash: bool,
    /// The window acts as administrator: KDE's helper has no trash.
    pub as_administrator: bool,
    /// The clipboard holds files that can be pasted here.
    pub can_paste: bool,
    /// Applications registered for the one selected file.
    pub applications: Vec<AppChoice>,
    /// The type database has been read.
    pub types_ready: bool,
    /// Every selected entry is an archive something can open (W-111).
    pub all_archives: bool,
    /// The folder "Extract into" makes, when one archive is selected.
    pub extract_folder: Option<String>,
    /// The files Compress's two quick entries make, ZIP then TAR.GZ.
    pub compress_names: [String; 2],
}

/// The menu for these facts. Empty when nothing in it can be chosen.
pub fn context_menu(facts: &MenuFacts) -> Vec<MenuEntry> {
    let mut entries = if facts.on_selection && facts.count > 0 {
        selection_menu(facts)
    } else {
        background_menu(facts)
    };
    while matches!(entries.last(), Some(MenuEntry::Separator)) {
        entries.pop();
    }
    if entries.iter().any(MenuEntry::enabled_choice) {
        entries
    } else {
        Vec::new()
    }
}

/// Actions for the current trash and navigation to other known trashes.
pub fn trash_actions(
    has_selection: bool,
    others: impl IntoIterator<Item = (String, PathBuf)>,
) -> Vec<MenuEntry> {
    let mut entries = vec![
        item("Restore", MenuCommand::Restore, has_selection),
        item("Empty trash…", MenuCommand::EmptyTrash, true),
    ];
    let others: Vec<_> = others
        .into_iter()
        .map(|(label, files)| item(&label, MenuCommand::OpenTrash(files), true))
        .collect();
    if !others.is_empty() {
        entries.push(MenuEntry::Separator);
        entries.extend(others);
    }
    entries
}

/// The height of a menu column: `row` for each item or submenu row,
/// `separator` for each separator, plus `chrome` for padding and borders.
pub fn column_height<C>(entries: &[MenuEntry<C>], row: f32, separator: f32, chrome: f32) -> f32 {
    entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Separator => separator,
            _ => row,
        })
        .sum::<f32>()
        + chrome
}

/// Whether a menu opened at an entry goes above it: it does not fit below,
/// and there is more room above, as Qt places menus.
pub fn opens_upward(height: f32, room_below: f32, room_above: f32) -> bool {
    height > room_below && room_above > room_below
}

fn item(label: &str, command: MenuCommand, enabled: bool) -> MenuEntry {
    MenuEntry::Item {
        label: label.to_owned(),
        command,
        enabled,
    }
}

fn selection_menu(facts: &MenuFacts) -> Vec<MenuEntry> {
    let one = facts.count == 1;
    let mut entries = vec![item("Open", MenuCommand::Open, one)];
    if one && !facts.single_container {
        entries.push(open_with(facts));
    }
    entries.push(MenuEntry::Separator);
    entries.push(item("Cut", MenuCommand::Cut, !facts.in_trash));
    entries.push(item("Copy", MenuCommand::Copy, true));
    entries.push(item(
        "Paste",
        MenuCommand::Paste,
        facts.can_paste && !facts.in_trash,
    ));
    entries.push(item("Copy location", MenuCommand::CopyLocation, true));
    entries.push(item(
        "Duplicate here",
        MenuCommand::Duplicate,
        !facts.in_trash,
    ));
    entries.push(MenuEntry::Separator);
    entries.push(item("Rename", MenuCommand::Rename, one && !facts.in_trash));
    if facts.in_trash {
        entries.push(item("Restore", MenuCommand::Restore, true));
    } else if !facts.as_administrator {
        entries.push(item("Move to trash", MenuCommand::MoveToTrash, true));
    }
    entries.push(item("Delete…", MenuCommand::Delete, true));
    entries.push(MenuEntry::Separator);
    if !facts.in_trash && !facts.as_administrator {
        let archives = archive_menus(facts);
        if !archives.is_empty() {
            entries.extend(archives);
            entries.push(MenuEntry::Separator);
        }
    }
    entries.push(item("Properties", MenuCommand::Properties, one));
    entries
}

/// Extract › for archives, and Compress › for anything but one archive,
/// as Dolphin shows Ark's menus (W-111, D-032).
fn archive_menus(facts: &MenuFacts) -> Vec<MenuEntry> {
    let mut menus = Vec::new();
    if facts.all_archives {
        let several = facts.count > 1;
        let into = match &facts.extract_folder {
            Some(folder) if !several => format!("Extract into \u{201c}{folder}/\u{201d}"),
            _ => "Extract each into its own folder".to_owned(),
        };
        menus.push(MenuEntry::Submenu {
            label: "Extract".to_owned(),
            icon: crate::icons::Icon::ArchiveRestore,
            items: vec![
                item("Extract here", MenuCommand::Extract(ExtractHow::Here), true),
                item(&into, MenuCommand::Extract(ExtractHow::NewFolder), true),
                item(
                    if several {
                        "Extract and trash archives"
                    } else {
                        "Extract and trash archive"
                    },
                    MenuCommand::Extract(ExtractHow::AndTrash),
                    true,
                ),
                item(
                    "Extract to\u{2026}",
                    MenuCommand::Extract(ExtractHow::To),
                    true,
                ),
            ],
        });
    }
    // One archive alone is not offered for compressing again, as in Dolphin.
    if !(facts.all_archives && facts.count == 1) {
        let [zip, tar_gz] = &facts.compress_names;
        menus.push(MenuEntry::Submenu {
            label: "Compress".to_owned(),
            icon: crate::icons::Icon::Archive,
            items: vec![
                item(
                    &format!("Compress to \u{201c}{zip}\u{201d}"),
                    MenuCommand::Compress(CompressHow::Zip),
                    true,
                ),
                item(
                    &format!("Compress to \u{201c}{tar_gz}\u{201d}"),
                    MenuCommand::Compress(CompressHow::TarGz),
                    true,
                ),
                item(
                    "Compress to\u{2026}",
                    MenuCommand::Compress(CompressHow::Choose),
                    true,
                ),
            ],
        });
    }
    menus
}

fn open_with(facts: &MenuFacts) -> MenuEntry {
    if !facts.types_ready {
        return item("Looking up applications…", MenuCommand::Open, false);
    }
    if facts.applications.is_empty() {
        return item("No application for this type", MenuCommand::Open, false);
    }
    MenuEntry::Submenu {
        label: "Open with".to_owned(),
        icon: crate::icons::Icon::AppWindow,
        items: facts
            .applications
            .iter()
            .cloned()
            .map(|app| {
                let name = app.name.clone();
                item(&name, MenuCommand::OpenWith(app), true)
            })
            .collect(),
    }
}

fn background_menu(facts: &MenuFacts) -> Vec<MenuEntry> {
    vec![
        item(
            "Paste",
            MenuCommand::Paste,
            facts.can_paste && !facts.in_trash,
        ),
        MenuEntry::Separator,
        item("New folder", MenuCommand::NewFolder, !facts.in_trash),
        item("New file", MenuCommand::NewFile, !facts.in_trash),
    ]
}

/// The New button's menu, as Dolphin's "Create New": a folder, then each
/// template by its label (W-108).
pub fn new_menu(templates: &[(String, TemplateKind)]) -> Vec<MenuEntry> {
    let mut entries = vec![item("Folder\u{2026}", MenuCommand::NewFolder, true)];
    if !templates.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.extend(templates.iter().enumerate().map(|(index, (label, kind))| {
        item(
            &format!("{label}\u{2026}"),
            MenuCommand::NewFromTemplate { index, kind: *kind },
            true,
        )
    }));
    entries
}

/// Which row the keyboard is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    /// Index into the top menu.
    pub cursor: usize,
    /// The application list is open.
    pub submenu: bool,
    /// Index into that list.
    pub sub_cursor: usize,
}

/// A key pressed while the menu has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    /// The previous enabled row.
    Up,
    /// The next enabled row.
    Down,
    /// Close the application list, if it is open.
    Left,
    /// Open the application list, if the row has one.
    Right,
    /// Run the row, or open its application list.
    Accept,
    /// Close the menu.
    Close,
}

/// What a key did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEffect<C = MenuCommand> {
    /// The cursor moved, or a key did nothing.
    Stay,
    /// Close the menu without running anything.
    Close,
    /// Close the menu and run this.
    Run(C),
}

/// The first row the keyboard can land on.
pub fn initial<C>(entries: &[MenuEntry<C>]) -> Option<MenuState> {
    Some(MenuState {
        cursor: first_selectable(entries)?,
        submenu: false,
        sub_cursor: 0,
    })
}

/// Moves the cursor or runs the current row.
pub fn on_key<C: Clone>(
    entries: &[MenuEntry<C>],
    state: &mut MenuState,
    key: MenuKey,
) -> MenuEffect<C> {
    let Some(entry) = entries.get(state.cursor) else {
        return MenuEffect::Close;
    };
    if state.submenu {
        let MenuEntry::Submenu { items, .. } = entry else {
            state.submenu = false;
            return MenuEffect::Stay;
        };
        return match key {
            MenuKey::Up => {
                state.sub_cursor = step(items, state.sub_cursor, false);
                MenuEffect::Stay
            }
            MenuKey::Down => {
                state.sub_cursor = step(items, state.sub_cursor, true);
                MenuEffect::Stay
            }
            MenuKey::Left | MenuKey::Close => {
                state.submenu = false;
                if key == MenuKey::Close {
                    MenuEffect::Close
                } else {
                    MenuEffect::Stay
                }
            }
            MenuKey::Right => MenuEffect::Stay,
            MenuKey::Accept => items
                .get(state.sub_cursor)
                .and_then(MenuEntry::command)
                .cloned()
                .map_or(MenuEffect::Stay, MenuEffect::Run),
        };
    }
    match key {
        MenuKey::Up => {
            state.cursor = step(entries, state.cursor, false);
            MenuEffect::Stay
        }
        MenuKey::Down => {
            state.cursor = step(entries, state.cursor, true);
            MenuEffect::Stay
        }
        MenuKey::Left => MenuEffect::Stay,
        MenuKey::Right => {
            if let MenuEntry::Submenu { items, .. } = entry {
                state.submenu = true;
                state.sub_cursor = first_selectable(items).unwrap_or(0);
            }
            MenuEffect::Stay
        }
        MenuKey::Accept => match entry {
            MenuEntry::Submenu { items, .. } => {
                state.submenu = true;
                state.sub_cursor = first_selectable(items).unwrap_or(0);
                MenuEffect::Stay
            }
            _ => entry
                .command()
                .cloned()
                .map_or(MenuEffect::Stay, MenuEffect::Run),
        },
        MenuKey::Close => MenuEffect::Close,
    }
}

fn first_selectable<C>(entries: &[MenuEntry<C>]) -> Option<usize> {
    entries.iter().position(MenuEntry::enabled_choice)
}

fn step<C>(entries: &[MenuEntry<C>], from: usize, forward: bool) -> usize {
    let selectable: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.enabled_choice())
        .map(|(index, _)| index)
        .collect();
    if selectable.is_empty() {
        return from;
    }
    let position = selectable
        .iter()
        .position(|index| *index == from)
        .unwrap_or(0);
    let next = if forward {
        (position + 1) % selectable.len()
    } else {
        (position + selectable.len() - 1) % selectable.len()
    };
    selectable[next]
}

/// The words on the top-level rows, separators included as an empty string.
pub fn labels<C>(entries: &[MenuEntry<C>]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Item { label, .. } | MenuEntry::Submenu { label, .. } => label.as_str(),
            MenuEntry::Separator => "",
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn menu_rows_show_the_keys_that_do_the_same() {
        let keys = |command: MenuCommand| command.shortcut();
        assert_eq!(keys(MenuCommand::Open).as_deref(), Some("Enter"));
        assert_eq!(keys(MenuCommand::Cut).as_deref(), Some("Ctrl+X"));
        assert_eq!(keys(MenuCommand::Copy).as_deref(), Some("Ctrl+C"));
        assert_eq!(keys(MenuCommand::Paste).as_deref(), Some("Ctrl+V"));
        assert_eq!(
            keys(MenuCommand::CopyLocation).as_deref(),
            Some("Ctrl+Alt+C")
        );
        assert_eq!(keys(MenuCommand::Duplicate).as_deref(), Some("Ctrl+D"));
        assert_eq!(keys(MenuCommand::Rename).as_deref(), Some("F2"));
        assert_eq!(keys(MenuCommand::MoveToTrash).as_deref(), Some("Delete"));
        assert_eq!(keys(MenuCommand::Delete).as_deref(), Some("Shift+Delete"));
        assert_eq!(keys(MenuCommand::Properties).as_deref(), Some("Alt+Enter"));
        assert_eq!(keys(MenuCommand::NewFolder).as_deref(), Some("F10"));
        assert_eq!(keys(MenuCommand::NewFile).as_deref(), Some("Ctrl+Alt+N"));
        // Rows without a key of their own show none.
        assert_eq!(keys(MenuCommand::Restore), None);
        assert_eq!(keys(MenuCommand::EmptyTrash), None);
    }

    fn facts(mut change: impl FnMut(&mut MenuFacts)) -> MenuFacts {
        let mut facts = MenuFacts {
            on_selection: true,
            count: 1,
            single_container: false,
            in_trash: false,
            as_administrator: false,
            can_paste: true,
            applications: vec![AppChoice {
                id: "org.kde.kate.desktop".into(),
                name: "Kate".into(),
                icon: None,
            }],
            types_ready: true,
            all_archives: false,
            extract_folder: None,
            compress_names: ["notes.zip".into(), "notes.tar.gz".into()],
        };
        change(&mut facts);
        facts
    }

    /// The labels inside the submenu called `name`.
    fn submenu<'a>(entries: &'a [MenuEntry], name: &str) -> Option<Vec<&'a str>> {
        entries.iter().find_map(|entry| match entry {
            MenuEntry::Submenu { label, items, .. } if label == name => Some(
                items
                    .iter()
                    .map(|item| match item {
                        MenuEntry::Item { label, .. } => label.as_str(),
                        _ => "",
                    })
                    .collect(),
            ),
            _ => None,
        })
    }

    /// The icon each submenu row is sent to QML with, by its label.
    fn submenu_icons(entries: &[MenuEntry]) -> Vec<(String, String)> {
        let json = to_json(entries);
        json.as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "submenu")
            .map(|row| {
                (
                    row["label"].as_str().unwrap().to_owned(),
                    row["icon"].as_str().unwrap_or("").to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn submenu_rows_carry_an_icon_like_every_other_row() {
        let archives = context_menu(&facts(|f| f.all_archives = true));
        let files = context_menu(&facts(|f| {
            f.applications.push(AppChoice {
                id: "org.kde.okular.desktop".into(),
                name: "Okular".into(),
                icon: None,
            });
        }));
        assert_eq!(
            submenu_icons(&archives),
            [
                ("Open with".to_owned(), "app-window".to_owned()),
                ("Extract".to_owned(), "archive-restore".to_owned())
            ]
        );
        assert_eq!(
            submenu_icons(&files),
            [
                ("Open with".to_owned(), "app-window".to_owned()),
                ("Compress".to_owned(), "archive".to_owned())
            ]
        );
    }

    #[test]
    fn one_archive_offers_extract_and_not_compress_as_in_dolphin() {
        let entries = context_menu(&facts(|f| {
            f.all_archives = true;
            f.extract_folder = Some("Holiday".into());
        }));
        assert_eq!(
            submenu(&entries, "Extract").unwrap(),
            [
                "Extract here",
                "Extract into \u{201c}Holiday/\u{201d}",
                "Extract and trash archive",
                "Extract to\u{2026}"
            ]
        );
        assert!(submenu(&entries, "Compress").is_none());
        // Before Properties, in a group of its own.
        let at = labels(&entries)
            .iter()
            .position(|label| *label == "Extract")
            .unwrap();
        assert_eq!(labels(&entries)[at + 1], "");
        assert_eq!(labels(&entries)[at + 2], "Properties");
    }

    #[test]
    fn other_selections_offer_compress_named_after_them_and_never_in_the_trash() {
        let entries = context_menu(&facts(|f| {
            f.count = 3;
            f.compress_names = ["invoice-2026 (1).zip".into(), "invoice-2026.tar.gz".into()];
        }));
        assert!(submenu(&entries, "Extract").is_none());
        assert_eq!(
            submenu(&entries, "Compress").unwrap(),
            [
                "Compress to \u{201c}invoice-2026 (1).zip\u{201d}",
                "Compress to \u{201c}invoice-2026.tar.gz\u{201d}",
                "Compress to\u{2026}"
            ]
        );
        let several_archives = context_menu(&facts(|f| {
            f.count = 2;
            f.all_archives = true;
        }));
        assert_eq!(
            submenu(&several_archives, "Extract").unwrap()[1],
            "Extract each into its own folder"
        );
        assert!(submenu(&several_archives, "Compress").is_some());
        for hidden in [
            facts(|f| f.in_trash = true),
            facts(|f| f.as_administrator = true),
        ] {
            let entries = context_menu(&hidden);
            assert!(
                submenu(&entries, "Compress").is_none() && submenu(&entries, "Extract").is_none()
            );
        }
    }

    #[test]
    fn a_menu_height_counts_rows_separators_and_chrome() {
        let entries = trash_actions(true, [("Other".into(), PathBuf::from("/t"))]);
        assert_eq!(labels(&entries), ["Restore", "Empty trash…", "", "Other"]);
        assert_eq!(column_height(&entries, 28., 9., 10.), 3. * 28. + 9. + 10.);
    }

    #[test]
    fn a_menu_opens_upward_only_when_below_is_too_small_and_above_is_larger() {
        assert!(!opens_upward(300., 400., 100.));
        assert!(opens_upward(300., 120., 500.));
        assert!(!opens_upward(300., 120., 100.));
    }

    #[test]
    fn trash_without_a_selection_skips_restore_but_keeps_empty_reachable() {
        let entries = trash_actions(false, []);
        assert_eq!(labels(&entries), ["Restore", "Empty trash…"]);
        let mut state = initial(&entries).unwrap();
        assert_eq!(
            on_key(&entries, &mut state, MenuKey::Accept),
            MenuEffect::Run(MenuCommand::EmptyTrash)
        );
    }

    #[test]
    fn trash_selection_can_be_restored_without_offering_permanent_delete() {
        let entries = trash_actions(true, []);
        let mut state = initial(&entries).unwrap();
        assert_eq!(
            on_key(&entries, &mut state, MenuKey::Accept),
            MenuEffect::Run(MenuCommand::Restore)
        );
        assert_eq!(labels(&entries), ["Restore", "Empty trash…"]);
    }

    #[test]
    fn other_trash_choices_keep_their_exact_paths_and_skip_the_separator() {
        use std::os::unix::ffi::OsStringExt as _;
        let first = PathBuf::from("/generated/volume A/.Trash-1000/files");
        let second = PathBuf::from(std::ffi::OsString::from_vec(
            b"/generated/volume-\xff/.Trash/1000/files".to_vec(),
        ));
        let entries = trash_actions(
            false,
            [
                ("Volume A".into(), first.clone()),
                ("Byte name".into(), second.clone()),
            ],
        );
        let mut state = initial(&entries).unwrap();
        on_key(&entries, &mut state, MenuKey::Down);
        assert_eq!(
            on_key(&entries, &mut state, MenuKey::Accept),
            MenuEffect::Run(MenuCommand::OpenTrash(first))
        );
        on_key(&entries, &mut state, MenuKey::Down);
        assert_eq!(
            on_key(&entries, &mut state, MenuKey::Accept),
            MenuEffect::Run(MenuCommand::OpenTrash(second))
        );
        assert_eq!(
            labels(&entries),
            ["Restore", "Empty trash…", "", "Volume A", "Byte name"]
        );
    }

    #[test]
    fn an_application_shows_its_own_icon_and_falls_back_to_the_outline() {
        let kate = |icon: Option<Arc<Path>>| {
            MenuCommand::OpenWith(AppChoice {
                id: "org.kde.kate.desktop".into(),
                name: "Kate".into(),
                icon,
            })
        };
        let file: Arc<Path> = Arc::from(Path::new("/icons/kate.svg"));
        assert_eq!(
            kate(Some(Arc::clone(&file))).row_icon(),
            RowIcon::Image(file)
        );
        assert_eq!(
            kate(None).row_icon(),
            RowIcon::Symbol(crate::icons::Icon::AppWindow)
        );
        assert_eq!(
            MenuCommand::Copy.row_icon(),
            RowIcon::Symbol(crate::icons::Icon::Copy)
        );
    }

    #[test]
    fn a_file_offers_open_with_and_the_file_operations() {
        let entries = context_menu(&facts(|_| {}));
        assert_eq!(
            labels(&entries),
            [
                "Open",
                "Open with",
                "",
                "Cut",
                "Copy",
                "Paste",
                "Copy location",
                "Duplicate here",
                "",
                "Rename",
                "Move to trash",
                "Delete…",
                "",
                "Compress",
                "",
                "Properties"
            ]
        );
        match &entries[1] {
            MenuEntry::Submenu { items, .. } => match &items[0] {
                MenuEntry::Item {
                    command: MenuCommand::OpenWith(app),
                    ..
                } => assert_eq!(app.name, "Kate"),
                other => panic!("expected Kate, got {other:?}"),
            },
            other => panic!("expected the application list, got {other:?}"),
        }
    }

    #[test]
    fn several_files_cannot_be_opened_or_renamed_as_one() {
        let entries = context_menu(&facts(|facts| facts.count = 3));
        let open = entries.first().expect("open");
        assert!(matches!(
            open,
            MenuEntry::Item {
                command: MenuCommand::Open,
                enabled: false,
                ..
            }
        ));
        assert!(entries.iter().all(|entry| !matches!(
            entry,
            MenuEntry::Submenu { label, .. } if label == "Open with"
        ) && !matches!(
            entry,
            MenuEntry::Item {
                command: MenuCommand::Rename,
                enabled: true,
                ..
            }
        )));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            MenuEntry::Item {
                command: MenuCommand::MoveToTrash,
                enabled: true,
                ..
            }
        )));
    }

    #[test]
    fn a_folder_is_opened_and_has_no_application_list() {
        let entries = context_menu(&facts(|facts| facts.single_container = true));
        assert!(matches!(
            &entries[0],
            MenuEntry::Item {
                command: MenuCommand::Open,
                enabled: true,
                ..
            }
        ));
        // No application list for a folder (Compress is its own submenu).
        assert!(entries.iter().all(
            |entry| !matches!(entry, MenuEntry::Submenu { label, .. } if label == "Open with")
        ));
    }

    #[test]
    fn trash_offers_restore_and_not_a_second_trash() {
        let entries = context_menu(&facts(|facts| facts.in_trash = true));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            MenuEntry::Item {
                command: MenuCommand::Restore,
                enabled: true,
                ..
            }
        )));
        assert!(entries.iter().all(|entry| !matches!(
            entry,
            MenuEntry::Item {
                command: MenuCommand::MoveToTrash
                    | MenuCommand::Cut
                    | MenuCommand::Rename
                    | MenuCommand::Duplicate,
                enabled: true,
                ..
            }
        )));
    }

    #[test]
    fn empty_space_offers_paste_and_new_entries() {
        let entries = context_menu(&facts(|facts| facts.on_selection = false));
        assert_eq!(labels(&entries), ["Paste", "", "New folder", "New file"]);
    }

    #[test]
    fn empty_trash_space_offers_nothing() {
        let entries = context_menu(&facts(|facts| {
            facts.on_selection = false;
            facts.in_trash = true;
        }));
        assert!(entries.is_empty());
    }

    #[test]
    fn the_new_menu_offers_a_folder_then_each_template_with_its_picture() {
        let entries = new_menu(&[
            ("Text File".into(), TemplateKind::Text),
            ("HTML File".into(), TemplateKind::Html),
            ("Letter".into(), TemplateKind::Other),
        ]);
        assert_eq!(
            labels(&entries),
            [
                "Folder\u{2026}",
                "",
                "Text File\u{2026}",
                "HTML File\u{2026}",
                "Letter\u{2026}"
            ]
        );
        let MenuEntry::Item { command, .. } = &entries[3] else {
            panic!("an item");
        };
        assert_eq!(
            *command,
            MenuCommand::NewFromTemplate {
                index: 1,
                kind: TemplateKind::Html
            }
        );
        assert_eq!(command.icon(), crate::icons::Icon::FileCode);
        assert_eq!(labels(&new_menu(&[])), ["Folder\u{2026}"]);
    }

    #[test]
    fn acting_as_administrator_offers_no_trash() {
        let entries = context_menu(&facts(|facts| facts.as_administrator = true));
        let labels = labels(&entries);
        assert!(!labels.contains(&"Move to trash"), "{labels:?}");
        assert!(labels.contains(&"Delete…"), "{labels:?}");
    }

    #[test]
    fn no_registered_application_is_said_in_words() {
        let entries = context_menu(&facts(|facts| facts.applications.clear()));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            MenuEntry::Item {
                label,
                enabled: false,
                ..
            } if label == "No application for this type"
        )));
    }

    #[test]
    fn the_keyboard_skips_disabled_rows_and_opens_the_application_list() {
        let entries = context_menu(&facts(|facts| facts.count = 2));
        let mut state = initial(&entries).expect("an enabled row");
        // Open is disabled, so the cursor starts on Cut.
        assert!(matches!(
            &entries[state.cursor],
            MenuEntry::Item {
                command: MenuCommand::Cut,
                ..
            }
        ));
        // Up wraps to the last row that can be chosen: Compress
        // (Properties is for one entry only).
        assert_eq!(on_key(&entries, &mut state, MenuKey::Up), MenuEffect::Stay);
        assert!(matches!(
            &entries[state.cursor],
            MenuEntry::Submenu { label, .. } if label == "Compress"
        ));
        let single = context_menu(&facts(|_| {}));
        let mut state = initial(&single).unwrap();
        assert_eq!(on_key(&single, &mut state, MenuKey::Down), MenuEffect::Stay);
        assert_eq!(
            on_key(&single, &mut state, MenuKey::Right),
            MenuEffect::Stay
        );
        assert!(state.submenu);
        assert_eq!(
            on_key(&single, &mut state, MenuKey::Accept),
            MenuEffect::Run(MenuCommand::OpenWith(AppChoice {
                id: "org.kde.kate.desktop".into(),
                name: "Kate".into(),
                icon: None,
            }))
        );
    }

    #[test]
    fn a_menu_reaches_qml_with_paths_that_find_its_commands() {
        let entries = vec![
            MenuEntry::Item {
                label: "Open".into(),
                command: MenuCommand::Open,
                enabled: true,
            },
            MenuEntry::Separator,
            MenuEntry::Submenu {
                label: "More".into(),
                icon: crate::icons::Icon::Menu,
                items: vec![
                    MenuEntry::Item {
                        label: "Copy".into(),
                        command: MenuCommand::Copy,
                        enabled: true,
                    },
                    MenuEntry::Item {
                        label: "Paste".into(),
                        command: MenuCommand::Paste,
                        enabled: false,
                    },
                ],
            },
        ];
        let json = to_json(&entries);
        assert_eq!(json[0]["shortcut"], "Enter");
        assert_eq!(json[2]["items"][1]["path"], "2.1");
        assert_eq!(command_at(&entries, "0"), Some(&MenuCommand::Open));
        assert_eq!(command_at(&entries, "2.0"), Some(&MenuCommand::Copy));
        // Disabled entries, separators and submenus themselves run nothing.
        assert_eq!(command_at(&entries, "2.1"), None);
        assert_eq!(command_at(&entries, "1"), None);
        assert_eq!(command_at(&entries, "2"), None);
        assert_eq!(command_at(&entries, "9"), None);
    }

    fn item(label: &str, command: MenuCommand, enabled: bool) -> MenuEntry {
        MenuEntry::Item {
            label: label.into(),
            command,
            enabled,
        }
    }

    /// The path QML reports for the row labelled `label` of `json`.
    fn path_of(json: &str, label: &str) -> String {
        fn find(rows: &serde_json::Value, label: &str) -> Option<String> {
            rows.as_array()?.iter().find_map(|row| {
                if row["label"] == label {
                    row["path"].as_str().map(str::to_owned)
                } else {
                    find(&row["items"], label)
                }
            })
        }
        find(&serde_json::from_str(json).unwrap(), label).unwrap()
    }

    #[test]
    fn an_open_menu_gives_its_choice_and_context_once() {
        let mut menu = OpenMenu::default();
        let json = menu
            .open(
                vec![
                    item("Open", MenuCommand::Open, true),
                    MenuEntry::Submenu {
                        label: "Edit".into(),
                        icon: crate::icons::Icon::Pencil,
                        items: vec![item("Copy", MenuCommand::Copy, true)],
                    },
                ],
                "row 3",
            )
            .unwrap();
        let copy = path_of(&json, "Copy");
        assert_eq!(menu.choose(&copy), Some((MenuCommand::Copy, "row 3")));
        assert!(!menu.is_open());
        assert_eq!(menu.choose(&copy), None);
    }

    #[test]
    fn a_choice_from_a_replaced_menu_is_not_taken_from_the_new_one() {
        let mut menu = OpenMenu::default();
        let first = menu
            .open(vec![item("Open", MenuCommand::Open, true)], 1)
            .unwrap();
        let second = menu
            .open(vec![item("Cut", MenuCommand::Cut, true)], 2)
            .unwrap();
        assert_eq!(menu.choose(&path_of(&first, "Open")), None);
        assert!(menu.is_open());
        assert_eq!(
            menu.choose(&path_of(&second, "Cut")),
            Some((MenuCommand::Cut, 2))
        );
    }

    #[test]
    fn closing_without_choosing_forgets_the_menu() {
        let mut menu = OpenMenu::default();
        let json = menu
            .open(vec![item("Open", MenuCommand::Open, true)], ())
            .unwrap();
        menu.close();
        assert!(!menu.is_open());
        assert_eq!(menu.choose(&path_of(&json, "Open")), None);
        // Nothing to offer opens nothing.
        assert_eq!(menu.open(Vec::new(), ()), None);
        assert!(!menu.is_open());
    }

    #[test]
    fn disabled_entries_and_submenus_are_never_chosen() {
        let mut menu = OpenMenu::default();
        let entries = vec![
            item("Paste", MenuCommand::Paste, false),
            MenuEntry::Submenu {
                label: "Edit".into(),
                icon: crate::icons::Icon::Pencil,
                items: vec![item("Copy", MenuCommand::Copy, true)],
            },
        ];
        let json = menu.open(entries.clone(), ()).unwrap();
        assert_eq!(menu.choose(&path_of(&json, "Paste")), None);
        let json = menu.open(entries, ()).unwrap();
        assert_eq!(menu.choose(&path_of(&json, "Edit")), None);
    }
}
