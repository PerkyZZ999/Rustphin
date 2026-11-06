//! What a right-click offers for the current selection.
//!
//! The list is decided here, away from drawing, so a missing operation fails a
//! test rather than only disappearing on screen.

use std::path::PathBuf;

/// One application that can open the selected file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppChoice {
    /// Desktop-file id.
    pub id: String,
    /// The name shown in the menu.
    pub name: String,
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
            }],
            types_ready: true,
        };
        change(&mut facts);
        facts
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
                command: MenuCommand::MoveToTrash | MenuCommand::Cut | MenuCommand::Rename,
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
            }))
        );
    }
}
