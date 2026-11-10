//! What a right-click offers in the places panel (W-102).
//!
//! Places, groups and the panel's empty space each get their own list,
//! decided here away from drawing so a missing choice fails a test. Every
//! row has a key that does the same while the panel has the focus, and the
//! row shows it.

use gpui_kit::assets::IconName;

use crate::menu::{Command, MenuEntry, RowIcon};

/// An operation the places menu can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceCommand {
    /// Show the place in the active pane.
    Open,
    /// Show the place in a new tab.
    OpenInNewTab,
    /// Put the place in the group at this position.
    MoveToGroup(usize),
    /// Make a group and put the place in it.
    NewGroupWith,
    /// Take the place out of its group.
    LeaveGroup,
    /// One position up among its neighbours.
    MoveUp,
    /// One position down among its neighbours.
    MoveDown,
    /// Edit the place's or group's name in place.
    Rename,
    /// Hide the place (it stays in the shared file).
    Hide,
    /// Show a hidden place again.
    Show,
    /// Take the place out of the shared file; the folder is untouched.
    Remove,
    /// Fold the group's places away.
    Collapse,
    /// Unfold the group.
    Expand,
    /// Delete the group; its places stay, outside groups.
    DeleteGroup,
    /// Make an empty group and name it.
    NewGroup,
    /// Add the folder the active pane shows.
    AddShownFolder,
    /// Show or hide the hidden places.
    ToggleHiddenPlaces,
}

impl Command for PlaceCommand {
    fn shortcut(&self) -> Option<&'static str> {
        Some(match self {
            Self::Open => "Enter",
            Self::Rename => "F2",
            Self::Hide | Self::Show => "H",
            Self::Remove | Self::DeleteGroup => "Del",
            Self::MoveUp => "Alt+Up",
            Self::MoveDown => "Alt+Down",
            Self::Collapse => "Left",
            Self::Expand => "Right",
            Self::NewGroup => "Ctrl+Shift+G",
            Self::AddShownFolder => "Ctrl+Shift+D",
            Self::OpenInNewTab
            | Self::MoveToGroup(_)
            | Self::NewGroupWith
            | Self::LeaveGroup
            | Self::ToggleHiddenPlaces => return None,
        })
    }

    fn row_icon(&self) -> RowIcon {
        RowIcon::Symbol(match self {
            Self::Open => IconName::SquareArrowOutUpRight,
            Self::OpenInNewTab => IconName::SquarePlus,
            Self::MoveToGroup(_) => IconName::FolderInput,
            Self::NewGroupWith | Self::NewGroup => IconName::ListPlus,
            Self::LeaveGroup => IconName::FolderOutput,
            Self::MoveUp => IconName::ArrowUp,
            Self::MoveDown => IconName::ArrowDown,
            Self::Rename => IconName::Pencil,
            Self::Hide => IconName::EyeOff,
            Self::Show | Self::ToggleHiddenPlaces => IconName::Eye,
            Self::Remove => IconName::BookmarkMinus,
            Self::Collapse => IconName::ChevronsDownUp,
            Self::Expand => IconName::ChevronsUpDown,
            Self::DeleteGroup => IconName::CircleX,
            Self::AddShownFolder => IconName::BookmarkPlus,
        })
    }
}

/// What was right-clicked.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A place.
    Place {
        /// One of KDE's own places (Home, Trash…): it can be hidden, not
        /// removed.
        system: bool,
        /// Hidden, and shown because hidden places are.
        hidden: bool,
        /// The group it is in.
        group: Option<usize>,
        /// Every group's name, in order.
        groups: &'a [String],
    },
    /// A group heading.
    Group {
        /// Its places are folded away.
        collapsed: bool,
    },
    /// The panel's empty space or its title.
    Panel {
        /// The active pane shows a local folder that could be added.
        can_add_shown: bool,
        /// Hidden places are being shown.
        showing_hidden: bool,
    },
}

fn item(label: &str, command: PlaceCommand, enabled: bool) -> MenuEntry<PlaceCommand> {
    MenuEntry::Item {
        label: label.to_owned(),
        command,
        enabled,
    }
}

/// The label of the submenu that sorts a place into a group.
pub const MOVE_TO_GROUP: &str = "Move to group";

/// The menu for `target`.
pub fn place_menu(target: Target<'_>) -> Vec<MenuEntry<PlaceCommand>> {
    match target {
        Target::Place {
            system,
            hidden,
            group,
            groups,
        } => {
            let mut sort: Vec<MenuEntry<PlaceCommand>> = groups
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    item(name, PlaceCommand::MoveToGroup(index), group != Some(index))
                })
                .collect();
            if !sort.is_empty() {
                sort.push(MenuEntry::Separator);
            }
            sort.push(item("New group…", PlaceCommand::NewGroupWith, true));
            if group.is_some() {
                sort.push(item("Out of groups", PlaceCommand::LeaveGroup, true));
            }
            let mut entries = vec![
                item("Open", PlaceCommand::Open, true),
                item("Open in new tab", PlaceCommand::OpenInNewTab, true),
                MenuEntry::Separator,
                MenuEntry::Submenu {
                    label: MOVE_TO_GROUP.to_owned(),
                    items: sort,
                },
                item("Move up", PlaceCommand::MoveUp, true),
                item("Move down", PlaceCommand::MoveDown, true),
                MenuEntry::Separator,
                item("Rename", PlaceCommand::Rename, true),
            ];
            entries.push(if hidden {
                item("Show", PlaceCommand::Show, true)
            } else {
                item("Hide", PlaceCommand::Hide, true)
            });
            if !system {
                entries.push(item("Remove from places", PlaceCommand::Remove, true));
            }
            entries
        }
        Target::Group { collapsed } => vec![
            if collapsed {
                item("Expand", PlaceCommand::Expand, true)
            } else {
                item("Collapse", PlaceCommand::Collapse, true)
            },
            MenuEntry::Separator,
            item("Move up", PlaceCommand::MoveUp, true),
            item("Move down", PlaceCommand::MoveDown, true),
            MenuEntry::Separator,
            item("Rename group", PlaceCommand::Rename, true),
            item("Delete group", PlaceCommand::DeleteGroup, true),
        ],
        Target::Panel {
            can_add_shown,
            showing_hidden,
        } => vec![
            item("New group", PlaceCommand::NewGroup, true),
            item("Add to places", PlaceCommand::AddShownFolder, can_add_shown),
            MenuEntry::Separator,
            item(
                if showing_hidden {
                    "Hide hidden places"
                } else {
                    "Show hidden places"
                },
                PlaceCommand::ToggleHiddenPlaces,
                true,
            ),
        ],
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::menu::labels;

    fn place(group: Option<usize>, groups: &[String]) -> Target<'_> {
        Target::Place {
            system: false,
            hidden: false,
            group,
            groups,
        }
    }

    fn submenu(entries: &[MenuEntry<PlaceCommand>]) -> &[MenuEntry<PlaceCommand>] {
        entries
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Submenu { label, items } if label == MOVE_TO_GROUP => {
                    Some(items.as_slice())
                }
                _ => None,
            })
            .expect("a Move to group submenu")
    }

    #[test]
    fn a_place_can_be_sorted_into_any_group_or_a_new_one() {
        let groups = ["Work".to_owned(), "Photos".to_owned()];
        let entries = place_menu(place(None, &groups));
        let sort = submenu(&entries);
        assert_eq!(labels(sort), ["Work", "Photos", "", "New group…"]);
        assert!(sort.contains(&item("Photos", PlaceCommand::MoveToGroup(1), true)));
    }

    #[test]
    fn a_grouped_place_can_leave_its_group_and_its_own_group_is_not_offered() {
        let groups = ["Work".to_owned(), "Photos".to_owned()];
        let entries = place_menu(place(Some(0), &groups));
        let sort = submenu(&entries);
        assert_eq!(
            labels(sort),
            ["Work", "Photos", "", "New group…", "Out of groups"]
        );
        assert!(sort.contains(&item("Work", PlaceCommand::MoveToGroup(0), false)));
    }

    #[test]
    fn without_groups_a_place_can_still_start_one() {
        let entries = place_menu(place(None, &[]));
        assert_eq!(labels(submenu(&entries)), ["New group…"]);
    }

    #[test]
    fn kdes_own_places_are_hidden_not_removed_and_hidden_ones_can_be_shown() {
        let system = place_menu(Target::Place {
            system: true,
            hidden: false,
            group: None,
            groups: &[],
        });
        assert!(labels(&system).contains(&"Hide"));
        assert!(!labels(&system).contains(&"Remove from places"));
        let hidden = place_menu(Target::Place {
            system: false,
            hidden: true,
            group: None,
            groups: &[],
        });
        assert!(labels(&hidden).contains(&"Show"));
        assert!(labels(&hidden).contains(&"Remove from places"));
    }

    #[test]
    fn a_group_heading_folds_moves_renames_and_deletes() {
        let open = place_menu(Target::Group { collapsed: false });
        assert_eq!(
            labels(&open),
            [
                "Collapse",
                "",
                "Move up",
                "Move down",
                "",
                "Rename group",
                "Delete group"
            ]
        );
        let folded = place_menu(Target::Group { collapsed: true });
        assert_eq!(labels(&folded)[0], "Expand");
    }

    #[test]
    fn empty_space_makes_groups_adds_the_folder_shown_and_toggles_hidden_places() {
        let entries = place_menu(Target::Panel {
            can_add_shown: false,
            showing_hidden: true,
        });
        assert_eq!(
            labels(&entries),
            ["New group", "Add to places", "", "Hide hidden places"]
        );
        assert!(entries.contains(&item("Add to places", PlaceCommand::AddShownFolder, false)));
    }

    #[test]
    fn rows_show_the_panel_key_that_does_the_same() {
        assert_eq!(PlaceCommand::Rename.shortcut(), Some("F2"));
        assert_eq!(PlaceCommand::Hide.shortcut(), Some("H"));
        assert_eq!(PlaceCommand::Remove.shortcut(), Some("Del"));
        assert_eq!(PlaceCommand::NewGroup.shortcut(), Some("Ctrl+Shift+G"));
        assert_eq!(PlaceCommand::MoveToGroup(0).shortcut(), None);
    }
}
