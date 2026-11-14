//! What a right-click offers in the places panel (W-102).
//!
//! Places, groups, drives (W-148) and the panel's empty space each get their own list,
//! decided here away from drawing so a missing choice fails a test. Every
//! row has a key that does the same while the panel has the focus, and the
//! row shows it.

use crate::icons::Icon as IconName;

use crate::{
    commands::Action,
    menu::{Command, MenuEntry, RowIcon},
};

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
    /// Mount the drive, without opening it.
    Mount,
    /// Unmount the drive.
    Unmount,
    /// Unmount the drive and power it off when it can be, so it can be
    /// unplugged.
    SafelyRemove,
    /// Put the drive's mount point on the clipboard.
    CopyLocation,
}

impl PlaceCommand {
    /// The command the row runs from the keyboard and the palette too
    /// (W-123), whose key the row shows.
    pub fn action(&self) -> Option<Action> {
        use crate::commands::{PlacesCommand as P, WindowCommand};
        Some(match self {
            Self::Open => Action::Places(P::Open),
            Self::Rename => Action::Places(P::Rename),
            Self::Hide | Self::Show => Action::Places(P::ToggleHide),
            Self::Remove | Self::DeleteGroup => Action::Places(P::Remove),
            Self::MoveUp => Action::Places(P::MoveUp),
            Self::MoveDown => Action::Places(P::MoveDown),
            Self::Collapse => Action::Places(P::Collapse),
            Self::Expand => Action::Places(P::Expand),
            Self::NewGroup => Action::Places(P::NewGroup),
            Self::ToggleHiddenPlaces => Action::Places(P::ToggleHiddenPlaces),
            Self::AddShownFolder => Action::Window(WindowCommand::AddPlace),
            // E safely removes a drive the menu offers to remove (W-148).
            Self::SafelyRemove => Action::Places(P::Eject),
            Self::OpenInNewTab
            | Self::MoveToGroup(_)
            | Self::NewGroupWith
            | Self::LeaveGroup
            | Self::Mount
            | Self::Unmount
            | Self::CopyLocation => {
                return None;
            }
        })
    }
}

impl Command for PlaceCommand {
    fn shortcut(&self) -> Option<String> {
        self.action().and_then(crate::commands::key_of)
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
            Self::Mount => IconName::HardDrive,
            // Lucide has no eject glyph; DESIGN.md names this one for it.
            Self::Unmount => IconName::ArrowUpFromLine,
            Self::SafelyRemove => IconName::Power,
            Self::CopyLocation => IconName::Link,
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
    /// A drive (W-148).
    Device {
        /// It is mounted.
        mounted: bool,
        /// It can be unmounted: mounted, and not a volume the system or the
        /// home folder runs from.
        can_unmount: bool,
        /// It can be safely removed: removable or able to power off, and
        /// not a volume the system or the home folder runs from.
        can_remove: bool,
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
                    icon: IconName::FolderInput,
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
        Target::Device {
            mounted,
            can_unmount,
            can_remove,
        } => {
            let mut entries = vec![
                item("Open", PlaceCommand::Open, true),
                item("Open in new tab", PlaceCommand::OpenInNewTab, true),
            ];
            let mut removal = Vec::new();
            if !mounted {
                removal.push(item("Mount", PlaceCommand::Mount, true));
            } else if can_unmount {
                removal.push(item("Unmount", PlaceCommand::Unmount, true));
            }
            if can_remove {
                removal.push(item("Safely remove", PlaceCommand::SafelyRemove, true));
            }
            if !removal.is_empty() {
                entries.push(MenuEntry::Separator);
                entries.extend(removal);
            }
            if mounted {
                entries.push(MenuEntry::Separator);
                entries.push(item("Copy location", PlaceCommand::CopyLocation, true));
            }
            entries
        }
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
                MenuEntry::Submenu { label, items, .. } if label == MOVE_TO_GROUP => {
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

    fn drive(mounted: bool, can_unmount: bool, can_remove: bool) -> Vec<MenuEntry<PlaceCommand>> {
        place_menu(Target::Device {
            mounted,
            can_unmount,
            can_remove,
        })
    }

    #[test]
    fn a_mounted_removable_drive_opens_unmounts_removes_and_copies_its_location() {
        assert_eq!(
            labels(&drive(true, true, true)),
            [
                "Open",
                "Open in new tab",
                "",
                "Unmount",
                "Safely remove",
                "",
                "Copy location"
            ]
        );
    }

    #[test]
    fn an_unmounted_drive_offers_to_mount_and_has_no_location_to_copy() {
        assert_eq!(
            labels(&drive(false, false, true)),
            ["Open", "Open in new tab", "", "Mount", "Safely remove"]
        );
        assert_eq!(
            labels(&drive(false, false, false)),
            ["Open", "Open in new tab", "", "Mount"]
        );
    }

    #[test]
    fn the_systems_own_volume_is_only_opened_or_its_location_copied() {
        assert_eq!(
            labels(&drive(true, false, false)),
            ["Open", "Open in new tab", "", "Copy location"]
        );
        // A fixed data disk unmounts but is not removed.
        assert_eq!(
            labels(&drive(true, true, false)),
            [
                "Open",
                "Open in new tab",
                "",
                "Unmount",
                "",
                "Copy location"
            ]
        );
    }

    #[test]
    fn a_drive_menu_shows_enter_on_open_and_e_on_safe_removal() {
        assert_eq!(PlaceCommand::SafelyRemove.shortcut().as_deref(), Some("E"));
        assert_eq!(PlaceCommand::Open.shortcut().as_deref(), Some("Enter"));
        assert_eq!(PlaceCommand::Unmount.shortcut(), None);
    }

    #[test]
    fn rows_show_the_panel_key_that_does_the_same() {
        let key = |command: PlaceCommand| command.shortcut();
        assert_eq!(key(PlaceCommand::Rename).as_deref(), Some("F2"));
        assert_eq!(key(PlaceCommand::Hide).as_deref(), Some("H"));
        assert_eq!(key(PlaceCommand::Remove).as_deref(), Some("Delete"));
        assert_eq!(key(PlaceCommand::NewGroup).as_deref(), Some("Ctrl+Shift+G"));
        assert_eq!(key(PlaceCommand::MoveToGroup(0)), None);
    }
}
