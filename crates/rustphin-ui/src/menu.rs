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
    Symbol(gpui_kit::assets::IconName),
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
}

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// An operation. Disabled rows are shown and skipped by the keyboard.
    Item {
        /// The words on the row.
        label: String,
        /// What choosing it does.
        command: MenuCommand,
        /// Whether it can be chosen.
        enabled: bool,
    },
    /// A row that opens `items` beside the menu.
    Submenu {
        /// The words on the row.
        label: String,
        /// The rows of the second menu.
        items: Vec<MenuEntry>,
    },
    /// A hairline between groups.
    Separator,
}

impl MenuCommand {
    /// The key that does the same outside the menu, as the row shows it.
    pub fn shortcut(&self) -> Option<&'static str> {
        Some(match self {
            Self::Open => "Enter",
            Self::Cut => "Ctrl+X",
            Self::Copy => "Ctrl+C",
            Self::Paste => "Ctrl+V",
            Self::CopyLocation => "Ctrl+Alt+C",
            Self::Duplicate => "Ctrl+D",
            Self::Rename => "F2",
            Self::MoveToTrash => "Del",
            Self::Delete => "Shift+Del",
            Self::Properties => "Alt+Enter",
            Self::NewFolder => "F10",
            Self::NewFile => "Ctrl+Alt+N",
            Self::OpenWith(_) | Self::Restore | Self::EmptyTrash | Self::OpenTrash(_) => {
                return None;
            }
        })
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
    pub fn icon(&self) -> gpui_kit::assets::IconName {
        use gpui_kit::assets::IconName as I;
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
            Self::Restore => I::ArchiveRestore,
            Self::NewFolder => I::FolderPlus,
            Self::NewFile => I::FilePlus,
        }
    }
}

impl MenuEntry {
    fn enabled_choice(&self) -> bool {
        match self {
            Self::Item { enabled, .. } => *enabled,
            Self::Submenu { items, .. } => items.iter().any(Self::enabled_choice),
            Self::Separator => false,
        }
    }

    fn command(&self) -> Option<&MenuCommand> {
        match self {
            Self::Item {
                command, enabled, ..
            } if *enabled => Some(command),
            _ => None,
        }
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
    /// The clipboard holds files that can be pasted here.
    pub can_paste: bool,
    /// Applications registered for the one selected file.
    pub applications: Vec<AppChoice>,
    /// The type database has been read.
    pub types_ready: bool,
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
pub fn column_height(entries: &[MenuEntry], row: f32, separator: f32, chrome: f32) -> f32 {
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
    } else {
        entries.push(item("Move to trash", MenuCommand::MoveToTrash, true));
    }
    entries.push(item("Delete…", MenuCommand::Delete, true));
    entries.push(MenuEntry::Separator);
    entries.push(item("Properties", MenuCommand::Properties, one));
    entries
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
pub enum MenuEffect {
    /// The cursor moved, or a key did nothing.
    Stay,
    /// Close the menu without running anything.
    Close,
    /// Close the menu and run this.
    Run(MenuCommand),
}

/// The first row the keyboard can land on.
pub fn initial(entries: &[MenuEntry]) -> Option<MenuState> {
    Some(MenuState {
        cursor: first_selectable(entries)?,
        submenu: false,
        sub_cursor: 0,
    })
}

/// Moves the cursor or runs the current row.
pub fn on_key(entries: &[MenuEntry], state: &mut MenuState, key: MenuKey) -> MenuEffect {
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

fn first_selectable(entries: &[MenuEntry]) -> Option<usize> {
    entries.iter().position(MenuEntry::enabled_choice)
}

fn step(entries: &[MenuEntry], from: usize, forward: bool) -> usize {
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
#[cfg(test)]
fn labels(entries: &[MenuEntry]) -> Vec<&str> {
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
        assert_eq!(keys(MenuCommand::Open), Some("Enter"));
        assert_eq!(keys(MenuCommand::Cut), Some("Ctrl+X"));
        assert_eq!(keys(MenuCommand::Copy), Some("Ctrl+C"));
        assert_eq!(keys(MenuCommand::Paste), Some("Ctrl+V"));
        assert_eq!(keys(MenuCommand::CopyLocation), Some("Ctrl+Alt+C"));
        assert_eq!(keys(MenuCommand::Duplicate), Some("Ctrl+D"));
        assert_eq!(keys(MenuCommand::Rename), Some("F2"));
        assert_eq!(keys(MenuCommand::MoveToTrash), Some("Del"));
        assert_eq!(keys(MenuCommand::Delete), Some("Shift+Del"));
        assert_eq!(keys(MenuCommand::Properties), Some("Alt+Enter"));
        assert_eq!(keys(MenuCommand::NewFolder), Some("F10"));
        assert_eq!(keys(MenuCommand::NewFile), Some("Ctrl+Alt+N"));
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
            can_paste: true,
            applications: vec![AppChoice {
                id: "org.kde.kate.desktop".into(),
                name: "Kate".into(),
                icon: None,
            }],
            types_ready: true,
        };
        change(&mut facts);
        facts
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
            RowIcon::Symbol(gpui_kit::assets::IconName::AppWindow)
        );
        assert_eq!(
            MenuCommand::Copy.row_icon(),
            RowIcon::Symbol(gpui_kit::assets::IconName::Copy)
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
            MenuEntry::Submenu { .. }
                | MenuEntry::Item {
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
        assert!(
            entries
                .iter()
                .all(|entry| !matches!(entry, MenuEntry::Submenu { .. }))
        );
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
        assert_eq!(on_key(&entries, &mut state, MenuKey::Up), MenuEffect::Stay);
        assert!(matches!(
            &entries[state.cursor],
            MenuEntry::Item {
                command: MenuCommand::Delete,
                ..
            }
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
}
