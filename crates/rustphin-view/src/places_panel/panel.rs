//! The places panel's state and what each key, menu entry, drop and rename
//! does to it, free of any toolkit (W-127).
//!
//! [`PlacesPanel`] owns the places read from KDE's shared file, Rustphin's
//! own groups (D-008), the volumes, which row has the keyboard, the name
//! being edited and the menu shown. Every change returns a list of
//! [`Effect`]s for the Qt adapter to carry out: save the groups, edit the
//! shared places file, open a place, mount, unmount or remove a volume,
//! copy its location, and redraw
//! the rows or lay them out again. The adapter never decides anything.

use std::{collections::HashMap, fmt, path::PathBuf};

use rustphin_core::display_name;
use rustphin_platform::{
    devices::Device,
    places::{Place, Places, PlacesError},
    remote::{RemoteAddress, network_folder_name},
};

use super::{
    DEVICES_UNAVAILABLE, Row, can_remove, can_unmount, device_card, device_name, icon_for,
    is_remote, is_shown, loose_slots, network_address, rows, shown_loose,
};
use crate::{
    commands::PlacesCommand,
    groups::{Groups, SeparatorAt},
    menu::OpenMenu,
    place_menu::{self, PlaceCommand, Target},
    settings::PlacesChoice,
};

/// A change to KDE's shared places file (`user-places.xbel`), made by the
/// platform's [`Places`] editor on another thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacesEdit {
    /// Add a bookmark for `folder`, then put it in `group`.
    Add {
        /// The folder bookmarked.
        folder: PathBuf,
        /// The name shown: the folder's own name.
        title: String,
        /// The group the new place goes into.
        group: Option<usize>,
    },
    /// Rename the place `id`.
    Rename {
        /// The place.
        id: String,
        /// Its new name.
        title: String,
    },
    /// Hide or show the place `id`; it stays in the file.
    Hide {
        /// The place.
        id: String,
        /// Whether it is hidden.
        hidden: bool,
    },
    /// Take the place `id` out of the file.
    Remove {
        /// The place.
        id: String,
    },
    /// Move the place `id` before the place `before`, or to the end.
    Move {
        /// The place.
        id: String,
        /// The place it goes before.
        before: Option<String>,
    },
}

impl PlacesEdit {
    /// The shared file's new text, and the identifier of an added place.
    /// `now` is the time in seconds since the Unix epoch, which KDE builds
    /// identifiers from.
    pub fn apply(
        &self,
        places: &Places,
        now: u64,
    ) -> Result<(String, Option<String>), PlacesError> {
        Ok(match self {
            Self::Add { folder, title, .. } => {
                let (text, id) = places.with_added(title, folder, "folder", now)?;
                (text, Some(id))
            }
            Self::Rename { id, title } => (places.with_title(id, title)?, None),
            Self::Hide { id, hidden } => (places.with_hidden(id, *hidden)?, None),
            Self::Remove { id } => (places.without(id)?, None),
            Self::Move { id, before } => (places.with_moved(id, before.as_deref())?, None),
        })
    }
}

/// What opening a row leads to.
#[derive(Clone, PartialEq, Eq)]
pub enum Opening {
    /// A local folder.
    Folder(PathBuf),
    /// The trash.
    Trash,
    /// A network folder by its address, mounted through kio-fuse.
    Remote(RemoteAddress),
    /// A network folder made in Dolphin (`remote:/NAME`), whose address is
    /// looked up in KDE's file of its own.
    NetworkFolder(String),
    /// A volume that is not mounted: mount it, then open it.
    Mount(Device),
}

impl fmt::Debug for Opening {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Folder(path) => formatter.debug_tuple("Folder").field(path).finish(),
            Self::Trash => formatter.write_str("Trash"),
            // An address may hold a password.
            Self::Remote(_) => formatter.write_str("Remote(..)"),
            Self::NetworkFolder(name) => {
                formatter.debug_tuple("NetworkFolder").field(name).finish()
            }
            Self::Mount(device) => formatter.debug_tuple("Mount").field(&device.id).finish(),
        }
    }
}

/// What the adapter does after a change, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// The rows may have changed: lay them out again when they did, redraw
    /// them in place when they did not (a reset would end a rename).
    Relayout,
    /// Another row has the keyboard; nothing moved.
    Focus,
    /// The name field opens on [`PlacesPanel::renaming`], holding this text.
    Rename(String),
    /// The right-click menu to show, as JSON; empty closes it.
    Menu(String),
    /// Write the groups to Rustphin's own file (D-008).
    SaveGroups,
    /// Change KDE's shared places file, then tell [`PlacesPanel::edited`].
    EditPlaces(PlacesEdit),
    /// Open a place or volume, in this tab or a new one.
    Open {
        /// Where it leads.
        opening: Opening,
        /// In a new tab.
        new_tab: bool,
    },
    /// Mount the volume without opening it.
    Mount(Device),
    /// Unmount the volume.
    Unmount(Device),
    /// Unmount the volume when mounted, then power its drive off when it
    /// can be, so it can be unplugged.
    SafelyRemove(Device),
    /// Put this location on the clipboard as text.
    CopyLocation(PathBuf),
    /// Keep in Settings whether hidden places are shown.
    ShowHidden(bool),
    /// Tell the user something (not an error).
    Say(String),
}

/// What kind of row a row is, as QML names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A place.
    Place,
    /// A group heading.
    Group,
    /// A volume.
    Device,
    /// The Devices or Remote heading.
    Heading,
    /// A separator (W-152).
    Separator,
}

impl RowKind {
    /// The name QML compares against.
    pub fn name(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Group => "group",
            Self::Device => "device",
            Self::Heading => "heading",
            Self::Separator => "separator",
        }
    }
}

/// Everything a row shows.
#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    /// What kind of row.
    pub kind: RowKind,
    /// Its name.
    pub title: String,
    /// The Lucide icon, or empty.
    pub icon: &'static str,
    /// It is the folder the active pane shows.
    pub current: bool,
    /// A hidden place, shown because hidden places are.
    pub hidden: bool,
    /// A folded heading.
    pub collapsed: bool,
    /// A group's count, or a drive card's last line: its filesystem and
    /// traits, "Not mounted" or "Working…" (W-148).
    pub detail: String,
    /// A drive card's figures right of its name: "used / total", or its size.
    pub figures: String,
    /// A drive card's "NN% used", or empty.
    pub percent: String,
    /// How full a volume is, 0 to 1; -1 when there is no bar.
    pub usage: f64,
    /// A volume at least 90 % full: its bar warns.
    pub nearly_full: bool,
    /// What a screen reader says after the name: a drive card's words.
    pub description: String,
    /// A volume being mounted, unmounted or removed.
    pub busy: bool,
    /// It has the keyboard.
    pub focused: bool,
    /// Its name is being edited.
    pub renaming: bool,
    /// A place or separator in a group.
    pub grouped: bool,
}

/// The places panel: its content, its rows and what is going on in it.
#[derive(Debug)]
pub struct PlacesPanel {
    places: Vec<Place>,
    groups: Groups,
    choice: PlacesChoice,
    home: Option<PathBuf>,
    devices: Vec<Device>,
    devices_unavailable: bool,
    usage: HashMap<String, (u64, u64)>,
    busy: Option<String>,
    current: Option<PathBuf>,
    /// The folder that holds the home trash's contents: the Trash place
    /// leads there, though its address is `trash:/`.
    trash_folder: Option<PathBuf>,
    rows: Vec<Row>,
    focused: Option<Row>,
    renaming: Option<Row>,
    /// A group just made, whose name field opens once it is drawn.
    rename_requested: Option<Row>,
    /// The open menu and the row it was opened on (none: empty space).
    menu: OpenMenu<PlaceCommand, Option<Row>>,
}

impl PlacesPanel {
    /// An empty panel showing the sections `choice` asks for; `home` is
    /// the user's home folder, whose volume is never offered for ejecting.
    pub fn new(choice: PlacesChoice, home: Option<PathBuf>) -> Self {
        Self {
            places: Vec::new(),
            groups: Groups::default(),
            choice,
            home,
            devices: Vec::new(),
            devices_unavailable: false,
            usage: HashMap::new(),
            busy: None,
            current: None,
            trash_folder: None,
            rows: Vec::new(),
            focused: None,
            renaming: None,
            rename_requested: None,
            menu: OpenMenu::default(),
        }
    }

    // ----- What the adapter reads -----

    /// The rows, top to bottom.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The places of the shared file, in its order.
    pub fn places(&self) -> &[Place] {
        &self.places
    }

    /// The groups, to save or list.
    pub fn groups(&self) -> &Groups {
        &self.groups
    }

    /// The volume named `id`.
    pub fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// The row with the keyboard; never a row that is not shown.
    pub fn focused(&self) -> Option<&Row> {
        self.focused.as_ref()
    }

    /// The row whose name is being edited.
    pub fn renaming(&self) -> Option<&Row> {
        self.renaming.as_ref()
    }

    /// Whether any place or group is listed, so the panel says how to add
    /// one only when there is none (volumes do not count).
    pub fn has_places(&self) -> bool {
        self.rows
            .iter()
            .any(|row| matches!(row, Row::Place(_) | Row::Group(_)))
    }

    /// The note under the panel: why no volumes are listed, if so.
    pub fn devices_note(&self) -> &'static str {
        if self.devices_unavailable && self.choice.devices {
            DEVICES_UNAVAILABLE
        } else {
            ""
        }
    }

    /// What `row` shows; `None` for a row that no longer exists.
    pub fn view(&self, row: &Row) -> Option<RowView> {
        let blank = RowView {
            kind: RowKind::Heading,
            title: String::new(),
            icon: "",
            current: false,
            hidden: false,
            collapsed: false,
            detail: String::new(),
            figures: String::new(),
            percent: String::new(),
            usage: -1.,
            nearly_full: false,
            description: String::new(),
            busy: false,
            focused: self.focused.as_ref() == Some(row),
            renaming: self.renaming.as_ref() == Some(row),
            grouped: false,
        };
        Some(match row {
            Row::Place(id) => {
                let place = self.place(id)?;
                RowView {
                    kind: RowKind::Place,
                    title: place.title.clone(),
                    icon: icon_for(place),
                    current: self.current.is_some()
                        && if place.href.starts_with("trash:") {
                            self.trash_folder == self.current
                        } else {
                            place.path == self.current
                        },
                    hidden: place.hidden,
                    grouped: self.groups.group_of(id).is_some(),
                    ..blank
                }
            }
            Row::Group(index) => {
                let group = self.groups.groups.get(*index)?;
                RowView {
                    kind: RowKind::Group,
                    title: if group.name.is_empty() {
                        "Group".to_owned()
                    } else {
                        group.name.clone()
                    },
                    collapsed: group.collapsed,
                    detail: group.place_count().to_string(),
                    ..blank
                }
            }
            Row::Separator(id) => {
                let at = self.groups.find_separator(id)?;
                RowView {
                    kind: RowKind::Separator,
                    title: "Separator".to_owned(),
                    grouped: matches!(at, SeparatorAt::InGroup(..)),
                    ..blank
                }
            }
            Row::Devices => RowView {
                title: if self.groups.devices_folded {
                    format!("Devices ({})", self.devices.len())
                } else {
                    "Devices".to_owned()
                },
                collapsed: self.groups.devices_folded,
                ..blank
            },
            Row::Remote => RowView {
                title: if self.groups.remote_folded {
                    let count = self
                        .places
                        .iter()
                        .filter(|place| {
                            is_shown(place, self.choice.show_hidden)
                                && is_remote(place)
                                && self.groups.group_of(&place.id).is_none()
                        })
                        .count();
                    format!("Remote ({count})")
                } else {
                    "Remote".to_owned()
                },
                collapsed: self.groups.remote_folded,
                ..blank
            },
            Row::Device(id) => {
                let device = self.device(id)?;
                let busy = self.busy.as_deref() == Some(id);
                let card = device_card(device, self.usage.get(id).copied(), busy);
                RowView {
                    kind: RowKind::Device,
                    title: device_name(device),
                    icon: if device.removable {
                        "usb"
                    } else {
                        "hard-drive"
                    },
                    current: self.current.is_some() && device.mount_point == self.current,
                    detail: card.traits,
                    figures: card.figures,
                    percent: card.percent,
                    usage: card.usage,
                    nearly_full: card.nearly_full,
                    description: card.description,
                    busy,
                    ..blank
                }
            }
        })
    }

    // ----- What the panel is told -----

    /// The places just read from the shared file. Groups forget places
    /// that are gone.
    pub fn set_places(&mut self, places: Vec<Place>) -> Vec<Effect> {
        if places == self.places {
            return Vec::new();
        }
        self.places = places;
        let forgot = self.groups.forget_missing(&self.places);
        self.changed(forgot)
    }

    /// The groups just read from Rustphin's file.
    pub fn set_groups(&mut self, groups: Groups) -> Vec<Effect> {
        self.groups = groups;
        self.changed(false)
    }

    /// The volumes just listed, with the usage of those mounted.
    pub fn set_devices(
        &mut self,
        devices: Vec<Device>,
        usage: HashMap<String, (u64, u64)>,
    ) -> Vec<Effect> {
        self.devices = devices;
        self.usage = usage;
        self.devices_unavailable = false;
        self.changed(false)
    }

    /// The device service cannot be reached: no volumes, and a note.
    pub fn devices_failed(&mut self) -> Vec<Effect> {
        self.devices_unavailable = true;
        self.devices.clear();
        self.usage.clear();
        self.changed(false)
    }

    /// The volume being mounted, unmounted or removed, or none.
    pub fn set_busy(&mut self, id: Option<String>) -> Vec<Effect> {
        self.busy = id;
        self.changed(false)
    }

    /// Where the home trash keeps its contents, so the Trash place is
    /// marked while that folder is shown.
    pub fn set_trash_folder(&mut self, folder: Option<PathBuf>) {
        self.trash_folder = folder;
    }

    /// The folder the active pane shows. Returns whether it changed.
    pub fn set_current(&mut self, folder: Option<PathBuf>) -> bool {
        let changed = self.current != folder;
        self.current = folder;
        changed
    }

    /// Which sections show, from Settings.
    pub fn set_choice(&mut self, choice: PlacesChoice) -> Vec<Effect> {
        let sections = |choice: &PlacesChoice| (choice.devices, choice.remote, choice.show_hidden);
        let changed = sections(&self.choice) != sections(&choice);
        self.choice = choice;
        if changed {
            self.changed(false)
        } else {
            Vec::new()
        }
    }

    /// The rows were laid out again ([`Effect::Relayout`] carried out):
    /// a group made a moment ago opens its name field now, after its
    /// heading is drawn and has the keyboard. Opened earlier, the field
    /// would lose the keyboard to the row as it is drawn, ending the rename.
    pub fn laid_out(&mut self) -> Vec<Effect> {
        match self.rename_requested.take() {
            Some(row) => self.begin_rename(row),
            None => Vec::new(),
        }
    }

    // ----- What the user does -----

    /// Gives the keyboard to the row at `at`; none when there is no such row.
    pub fn focus_on(&mut self, at: Option<usize>) -> Vec<Effect> {
        self.focused = at.and_then(|at| self.rows.get(at)).cloned();
        vec![Effect::Focus]
    }

    /// Moves the keyboard `delta` rows, stopping at the ends; from no row,
    /// down starts at the top and up at the bottom.
    pub fn move_focus(&mut self, delta: isize) -> Vec<Effect> {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return Vec::new();
        };
        let at = self.focused.as_ref().and_then(|row| self.at(row));
        let target = match at {
            Some(at) => at.saturating_add_signed(delta).min(last),
            None if delta < 0 => last,
            None => 0,
        };
        self.focused = Some(self.rows[target].clone());
        vec![Effect::Focus]
    }

    /// Opens the row at `at` (a click): a place, a volume, or folds a heading.
    pub fn activate(&mut self, at: usize, new_tab: bool) -> Vec<Effect> {
        let Some(row) = self.rows.get(at).cloned() else {
            return Vec::new();
        };
        self.focused = Some(row.clone());
        let mut effects = vec![Effect::Focus];
        effects.extend(self.open_row(&row, new_tab));
        effects
    }

    /// Runs a command on the row with the keyboard (W-123); `None` when it
    /// does not apply here.
    pub fn run(&mut self, command: PlacesCommand) -> Option<Vec<Effect>> {
        let focused = self.focused.clone();
        Some(match command {
            PlacesCommand::FocusPrevious => self.move_focus(-1),
            PlacesCommand::FocusNext => self.move_focus(1),
            PlacesCommand::Open => self.open_row(&focused?, false),
            PlacesCommand::Rename => self.begin_rename(focused?),
            PlacesCommand::Remove => self.remove_focused(),
            PlacesCommand::ToggleHide => match &focused {
                Some(Row::Place(id)) => match self.place(id) {
                    Some(place) => vec![Effect::EditPlaces(PlacesEdit::Hide {
                        id: id.clone(),
                        hidden: !place.hidden,
                    })],
                    None => Vec::new(),
                },
                _ => Vec::new(),
            },
            PlacesCommand::MoveUp => self.shift_focused(false),
            PlacesCommand::MoveDown => self.shift_focused(true),
            PlacesCommand::Collapse => self.fold(&focused?, true)?,
            PlacesCommand::Expand => self.fold(&focused?, false)?,
            PlacesCommand::NextGroup => self.cycle_group(),
            PlacesCommand::NewGroup => self.new_group(None),
            PlacesCommand::Eject => match &focused {
                Some(Row::Device(id)) => self.eject_device(id),
                _ => Vec::new(),
            },
            PlacesCommand::ToggleHiddenPlaces => self.set_show_hidden(!self.choice.show_hidden),
        })
    }

    /// Builds the right-click menu for the row at `at`, or the panel's empty
    /// space when there is no such row; `None` when there is nothing to
    /// offer or a name is being edited.
    pub fn open_menu(&mut self, at: Option<usize>) -> Option<Vec<Effect>> {
        if self.renaming.is_some() {
            return None;
        }
        let row = at.and_then(|at| self.rows.get(at)).cloned();
        let names: Vec<(usize, String)> = self
            .groups
            .groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.is_group())
            .map(|(index, group)| (index, group.name.clone()))
            .collect();
        let entries = match &row {
            Some(Row::Place(id)) => {
                let place = self.place(id)?;
                let group = self.groups.group_of(id);
                place_menu::place_menu(Target::Place {
                    system: place.system,
                    hidden: place.hidden,
                    group,
                    groups: &names,
                    separable: group.is_some() || !is_remote(place),
                })
            }
            Some(Row::Separator(_)) => place_menu::place_menu(Target::Separator),
            Some(Row::Group(index)) => place_menu::place_menu(Target::Group {
                collapsed: self.groups.groups.get(*index)?.collapsed,
            }),
            Some(Row::Device(id)) => {
                let device = self.device(id)?;
                let home = self.home.as_deref();
                place_menu::place_menu(Target::Device {
                    mounted: device.mount_point.is_some(),
                    can_unmount: can_unmount(device, home),
                    can_remove: can_remove(device, home),
                })
            }
            Some(Row::Devices | Row::Remote) => return None,
            None => place_menu::place_menu(Target::Panel {
                can_add_shown: self.current.is_some(),
                showing_hidden: self.choice.show_hidden,
            }),
        };
        let json = self.menu.open(entries, row.clone())?;
        if row.is_some() {
            self.focused = row;
        }
        Some(vec![Effect::Menu(json), Effect::Focus])
    }

    /// Runs the menu entry at `path` (as the menu's JSON gives it); an
    /// empty path closes the menu. A path from a menu since replaced
    /// does nothing.
    pub fn run_menu(&mut self, path: &str) -> Vec<Effect> {
        let chosen = if path.is_empty() {
            self.menu.close();
            None
        } else {
            self.menu.choose(path)
        };
        let mut effects = Vec::new();
        if !self.menu.is_open() {
            effects.push(Effect::Menu(String::new()));
        }
        let Some((command, row)) = chosen else {
            return effects;
        };
        let place = match &row {
            Some(Row::Place(id)) => Some(id.clone()),
            _ => None,
        };
        let device = match &row {
            Some(Row::Device(id)) => self.device(id).cloned(),
            _ => None,
        };
        effects.extend(match command {
            PlaceCommand::Open | PlaceCommand::OpenInNewTab => match &row {
                Some(row) => self.open_row(row, command == PlaceCommand::OpenInNewTab),
                None => Vec::new(),
            },
            PlaceCommand::MoveToGroup(index) => match place {
                Some(id) => self.move_to_group(&id, Some(index), None),
                None => Vec::new(),
            },
            PlaceCommand::NewGroupWith => match place {
                Some(id) => self.new_group(Some(&id)),
                None => Vec::new(),
            },
            PlaceCommand::LeaveGroup => match place {
                Some(id) => self.move_to_group(&id, None, None),
                None => Vec::new(),
            },
            PlaceCommand::MoveUp => self.shift_focused(false),
            PlaceCommand::MoveDown => self.shift_focused(true),
            PlaceCommand::Rename => match row {
                Some(row) => self.begin_rename(row),
                None => Vec::new(),
            },
            PlaceCommand::Hide | PlaceCommand::Show => match place {
                Some(id) => vec![Effect::EditPlaces(PlacesEdit::Hide {
                    id,
                    hidden: command == PlaceCommand::Hide,
                })],
                None => Vec::new(),
            },
            PlaceCommand::Remove | PlaceCommand::DeleteGroup | PlaceCommand::RemoveSeparator => {
                self.remove_focused()
            }
            PlaceCommand::AddSeparator => self.add_separator(row.as_ref()),
            PlaceCommand::Collapse | PlaceCommand::Expand => match &row {
                Some(row @ Row::Group(_)) => self
                    .fold(row, command == PlaceCommand::Collapse)
                    .unwrap_or_default(),
                _ => Vec::new(),
            },
            PlaceCommand::NewGroup => self.new_group(None),
            PlaceCommand::AddShownFolder => match self.current.clone() {
                Some(folder) => self.add_folder(folder, None),
                None => Vec::new(),
            },
            PlaceCommand::ToggleHiddenPlaces => self.set_show_hidden(!self.choice.show_hidden),
            PlaceCommand::Mount => match device {
                Some(device) if device.mount_point.is_none() => {
                    self.busy_with(device, Effect::Mount)
                }
                _ => Vec::new(),
            },
            PlaceCommand::Unmount => match device {
                Some(device) if can_unmount(&device, self.home.as_deref()) => {
                    self.busy_with(device, Effect::Unmount)
                }
                _ => Vec::new(),
            },
            PlaceCommand::SafelyRemove => match device {
                Some(device) if can_remove(&device, self.home.as_deref()) => {
                    self.busy_with(device, Effect::SafelyRemove)
                }
                _ => Vec::new(),
            },
            PlaceCommand::CopyLocation => match device {
                Some(device) => match device.mount_point.clone() {
                    Some(path) => vec![
                        Effect::CopyLocation(path),
                        Effect::Say(format!(
                            "Copied the location of \u{201c}{}\u{201d}.",
                            device_name(&device)
                        )),
                    ],
                    None => Vec::new(),
                },
                None => Vec::new(),
            },
        });
        effects
    }

    /// Ends renaming; with `apply`, the row takes `typed`. An empty or
    /// unchanged name changes nothing.
    pub fn finish_rename(&mut self, typed: &str, apply: bool) -> Vec<Effect> {
        let Some(row) = self.renaming.take() else {
            return Vec::new();
        };
        let typed = typed.trim();
        let mut effects = Vec::new();
        if apply && !typed.is_empty() {
            match row {
                Row::Place(id) => {
                    if self.place(&id).is_some_and(|place| place.title != typed) {
                        effects.push(Effect::EditPlaces(PlacesEdit::Rename {
                            id,
                            title: typed.to_owned(),
                        }));
                    }
                }
                Row::Group(index) => {
                    if self
                        .groups
                        .groups
                        .get(index)
                        .is_some_and(|group| group.name != typed)
                    {
                        self.groups.rename(index, typed);
                        effects.push(Effect::SaveGroups);
                    }
                }
                Row::Device(_) | Row::Devices | Row::Remote | Row::Separator(_) => {}
            }
        }
        self.lay_out();
        effects.push(Effect::Relayout);
        effects
    }

    /// Adds a place for `folder` (Ctrl+Shift+D, a drop), into `group`.
    pub fn add_folder(&mut self, folder: PathBuf, group: Option<usize>) -> Vec<Effect> {
        if self
            .places
            .iter()
            .any(|place| place.path.as_deref() == Some(&folder))
        {
            return vec![Effect::Say(format!(
                "\u{201c}{}\u{201d} is already in the places.",
                folder.display()
            ))];
        }
        let title = match folder.file_name() {
            Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
            None => "/".to_owned(),
        };
        vec![Effect::EditPlaces(PlacesEdit::Add {
            folder,
            title,
            group,
        })]
    }

    /// The shared file was changed as asked; `added` is the identifier of
    /// a place just added, which goes in its group and takes the keyboard.
    pub fn edited(&mut self, edit: &PlacesEdit, added: Option<String>) -> Vec<Effect> {
        let (PlacesEdit::Add { group, .. }, Some(id)) = (edit, added) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        if group.is_some() {
            self.groups.assign(&id, *group, None);
            effects.push(Effect::SaveGroups);
        }
        self.focused = Some(Row::Place(id));
        effects
    }

    /// The row at `from` was dragged onto the row at `to`: a place goes
    /// into a group, or before another place; a group takes another's
    /// position.
    pub fn drop_row(&mut self, from: usize, to: usize) -> Vec<Effect> {
        let (Some(from), Some(target)) = (self.rows.get(from).cloned(), self.rows.get(to).cloned())
        else {
            return Vec::new();
        };
        match from {
            Row::Separator(id) => self.drop_separator(&id, &target),
            Row::Place(id) => match target {
                Row::Group(index) => self.move_to_group(&id, Some(index), None),
                Row::Separator(separator) => match self.groups.find_separator(&separator) {
                    Some(SeparatorAt::InGroup(group, _)) => {
                        self.move_to_group(&id, Some(group), Some(&separator))
                    }
                    _ => Vec::new(),
                },
                Row::Device(_) | Row::Devices | Row::Remote => Vec::new(),
                Row::Place(other) if other == id => Vec::new(),
                Row::Place(other) => match self.groups.group_of(&other) {
                    Some(index) => self.move_to_group(&id, Some(index), Some(&other)),
                    None => {
                        let mut effects = self.move_to_group(&id, None, None);
                        effects.push(Effect::EditPlaces(PlacesEdit::Move {
                            id,
                            before: Some(other),
                        }));
                        effects
                    }
                },
            },
            Row::Group(from) => {
                let to = match &target {
                    Row::Group(index) => Some(*index),
                    Row::Place(id) => self.groups.group_of(id),
                    Row::Separator(id) => match self.groups.find_separator(id) {
                        Some(SeparatorAt::Between(index) | SeparatorAt::InGroup(index, _)) => {
                            Some(index)
                        }
                        Some(SeparatorAt::Loose(_)) | None => None,
                    },
                    Row::Device(_) | Row::Devices | Row::Remote => None,
                };
                let Some(to) = to.filter(|to| *to != from) else {
                    return Vec::new();
                };
                let at = self.groups.move_group(from, to);
                self.focused = Some(Row::Group(at));
                self.changed(true)
            }
            Row::Device(_) | Row::Devices | Row::Remote => Vec::new(),
        }
    }

    /// Folders dragged in from elsewhere were dropped on the row at `to`
    /// (none: the end): each becomes a place, in the group dropped on or
    /// the group of the place dropped on.
    pub fn drop_folders(&mut self, folders: Vec<PathBuf>, to: Option<usize>) -> Vec<Effect> {
        let group = match to.and_then(|to| self.rows.get(to)) {
            Some(Row::Group(index)) => Some(*index),
            Some(Row::Place(id) | Row::Separator(id)) => self.groups.group_of(id),
            _ => None,
        };
        folders
            .into_iter()
            .flat_map(|folder| self.add_folder(folder, group))
            .collect()
    }

    /// Moves group `index` by `delta` (Settings, Places).
    pub fn shift_group(&mut self, index: usize, delta: isize) -> Vec<Effect> {
        self.groups.shift(index, delta);
        self.changed(true)
    }

    /// Deletes group `index`; its places stay (Settings, Places).
    pub fn delete_group(&mut self, index: usize) -> Vec<Effect> {
        self.groups.remove(index);
        self.changed(true)
    }

    /// Renames group `index` (Settings, Places).
    pub fn rename_group(&mut self, index: usize, name: &str) -> Vec<Effect> {
        self.groups.rename(index, name);
        self.changed(true)
    }

    // ----- Inside -----

    fn place(&self, id: &str) -> Option<&Place> {
        self.places.iter().find(|place| place.id == id)
    }

    fn at(&self, row: &Row) -> Option<usize> {
        self.rows.iter().position(|shown| shown == row)
    }

    /// Lays the rows out from what the panel holds; the keyboard leaves a
    /// row that is no longer shown.
    fn lay_out(&mut self) {
        let mut rows = rows(
            &self.places,
            &self.groups,
            self.choice.show_hidden,
            self.groups.remote_folded,
            self.choice.remote,
        );
        if self.choice.devices {
            if !self.devices.is_empty() || self.devices_unavailable {
                rows.push(Row::Devices);
            }
            if !self.groups.devices_folded {
                rows.extend(
                    self.devices
                        .iter()
                        .map(|device| Row::Device(device.id.clone())),
                );
            }
        }
        self.rows = rows;
        if self
            .focused
            .as_ref()
            .is_some_and(|focused| !self.rows.contains(focused))
        {
            self.focused = None;
        }
    }

    /// The rows after a change, and the groups saved when `save`.
    fn changed(&mut self, save: bool) -> Vec<Effect> {
        self.lay_out();
        if save {
            vec![Effect::SaveGroups, Effect::Relayout]
        } else {
            vec![Effect::Relayout]
        }
    }

    fn open_row(&mut self, row: &Row, new_tab: bool) -> Vec<Effect> {
        let opening = match row {
            Row::Place(id) => {
                let Some(place) = self.place(id) else {
                    return Vec::new();
                };
                if let Some(path) = &place.path {
                    Opening::Folder(path.clone())
                } else if place.href.starts_with("trash:") {
                    Opening::Trash
                } else if let Some(address) = network_address(place) {
                    Opening::Remote(address)
                } else if let Some(name) = network_folder_name(&place.href) {
                    Opening::NetworkFolder(name)
                } else {
                    return Vec::new();
                }
            }
            Row::Device(id) => {
                let Some(device) = self.device(id).cloned() else {
                    return Vec::new();
                };
                match &device.mount_point {
                    Some(path) => Opening::Folder(path.clone()),
                    None => {
                        let mut effects = self.set_busy(Some(device.id.clone()));
                        effects.push(Effect::Open {
                            opening: Opening::Mount(device),
                            new_tab,
                        });
                        return effects;
                    }
                }
            }
            Row::Group(index) => {
                let collapsed = self
                    .groups
                    .groups
                    .get(*index)
                    .is_some_and(|group| group.collapsed);
                return self.fold(row, !collapsed).unwrap_or_default();
            }
            Row::Devices => {
                let folded = self.groups.devices_folded;
                return self.fold(row, !folded).unwrap_or_default();
            }
            Row::Remote => {
                let folded = self.groups.remote_folded;
                return self.fold(row, !folded).unwrap_or_default();
            }
            Row::Separator(_) => return Vec::new(),
        };
        vec![Effect::Open { opening, new_tab }]
    }

    /// Folds or unfolds a heading; `None` for a row that is not one.
    fn fold(&mut self, row: &Row, collapse: bool) -> Option<Vec<Effect>> {
        match row {
            Row::Group(index) => self.groups.set_collapsed(*index, collapse),
            Row::Devices => self.groups.devices_folded = collapse,
            Row::Remote => self.groups.remote_folded = collapse,
            Row::Place(_) | Row::Device(_) | Row::Separator(_) => return None,
        }
        Some(self.changed(true))
    }

    fn begin_rename(&mut self, row: Row) -> Vec<Effect> {
        let name = match &row {
            Row::Place(id) => self.place(id).map(|place| place.title.clone()),
            Row::Group(index) => self.groups.groups.get(*index).map(|g| g.name.clone()),
            Row::Device(_) | Row::Devices | Row::Remote | Row::Separator(_) => None,
        };
        let Some(name) = name else {
            return Vec::new();
        };
        self.renaming = Some(row);
        vec![Effect::Rename(name)]
    }

    /// Delete: a place leaves the shared file, but one of KDE's own places
    /// (Home, Trash…) is hidden instead, as KDE would bring it back; a group
    /// goes and its places stay.
    fn remove_focused(&mut self) -> Vec<Effect> {
        match self.focused.clone() {
            Some(Row::Place(id)) => {
                let Some(place) = self.place(&id) else {
                    return Vec::new();
                };
                if place.system {
                    return vec![Effect::EditPlaces(PlacesEdit::Hide { id, hidden: true })];
                }
                let mut effects = self.move_focus(1);
                effects.push(Effect::EditPlaces(PlacesEdit::Remove { id }));
                effects
            }
            Some(Row::Group(index)) => {
                self.groups.remove(index);
                self.focused = None;
                self.changed(true)
            }
            Some(row @ Row::Separator(_)) => {
                let Row::Separator(id) = &row else {
                    return Vec::new();
                };
                let at = self.at(&row);
                if !self.groups.remove_separator(id) {
                    return Vec::new();
                }
                // The keyboard stays where the separator was.
                let effects = self.changed(true);
                self.focused = at
                    .and_then(|at| self.rows.get(at).or_else(|| self.rows.last()))
                    .cloned();
                effects
            }
            Some(Row::Device(_) | Row::Devices | Row::Remote) | None => Vec::new(),
        }
    }

    /// Alt+Up/Down: a group moves among the groups, a grouped place within
    /// its group, and a place outside groups among the local places outside
    /// groups (in the shared file). None leaves its section.
    fn shift_focused(&mut self, down: bool) -> Vec<Effect> {
        match self.focused.clone() {
            Some(Row::Group(index)) => {
                let target = self.groups.shift(index, if down { 1 } else { -1 });
                self.focused = Some(Row::Group(target));
                self.changed(true)
            }
            Some(Row::Place(id)) if self.groups.group_of(&id).is_some() => {
                if self.groups.shift_place(&id, down) {
                    self.changed(true)
                } else {
                    Vec::new()
                }
            }
            Some(Row::Separator(id)) => self.shift_separator(&id, down),
            Some(Row::Place(id)) => {
                let shown: Vec<&str> =
                    shown_loose(&self.places, &self.groups, self.choice.show_hidden)
                        .into_iter()
                        .map(|place| place.id.as_str())
                        .collect();
                let Some(at) = shown.iter().position(|shown| *shown == id) else {
                    return Vec::new();
                };
                let before = if down {
                    if at + 1 >= shown.len() {
                        return Vec::new();
                    }
                    shown.get(at + 2).map(|id| (*id).to_owned())
                } else {
                    match at.checked_sub(1) {
                        Some(previous) => Some(shown[previous].to_owned()),
                        None => return Vec::new(),
                    }
                };
                vec![Effect::EditPlaces(PlacesEdit::Move { id, before })]
            }
            Some(Row::Device(_) | Row::Devices | Row::Remote) | None => Vec::new(),
        }
    }

    fn move_to_group(
        &mut self,
        id: &str,
        group: Option<usize>,
        before: Option<&str>,
    ) -> Vec<Effect> {
        self.groups.assign(id, group, before);
        if let Some(index) = group {
            self.groups.set_collapsed(index, false);
        }
        self.changed(true)
    }

    /// G: the place with the keyboard goes into the first group, then the
    /// next, and out of groups after the last.
    fn cycle_group(&mut self) -> Vec<Effect> {
        let Some(Row::Place(id)) = self.focused.clone() else {
            return Vec::new();
        };
        let real: Vec<usize> = (0..self.groups.groups.len())
            .filter(|index| self.groups.groups[*index].is_group())
            .collect();
        let next = match self.groups.group_of(&id) {
            None => match real.first() {
                Some(first) => Some(*first),
                None => return Vec::new(),
            },
            Some(index) => real.iter().copied().find(|later| *later > index),
        };
        self.move_to_group(&id, next, None)
    }

    /// Makes a group, with the place `with` in it, and asks for its name
    /// field (see [`PlacesPanel::laid_out`]).
    fn new_group(&mut self, with: Option<&str>) -> Vec<Effect> {
        let index = self.groups.add("");
        if let Some(id) = with {
            self.groups.assign(id, Some(index), None);
        }
        let row = Row::Group(index);
        self.focused = Some(row.clone());
        self.rename_requested = Some(row);
        self.changed(true)
    }

    /// E: what the eject button did before W-148. Safely remove where the
    /// drive menu offers it, else Unmount; nothing for the volumes the
    /// system and the home folder run from.
    fn eject_device(&mut self, id: &str) -> Vec<Effect> {
        let Some(device) = self.device(id).cloned() else {
            return Vec::new();
        };
        let home = self.home.as_deref();
        if can_remove(&device, home) {
            self.busy_with(device, Effect::SafelyRemove)
        } else if can_unmount(&device, home) {
            self.busy_with(device, Effect::Unmount)
        } else {
            Vec::new()
        }
    }

    /// Marks `device` busy, then asks the adapter for `effect` on it.
    fn busy_with(&mut self, device: Device, effect: fn(Device) -> Effect) -> Vec<Effect> {
        let mut effects = self.set_busy(Some(device.id.clone()));
        effects.push(effect(device));
        effects
    }

    /// Adds a separator (W-152) below the place `row`, above the group
    /// `row`, or after the last group from the empty space; it takes the
    /// keyboard.
    fn add_separator(&mut self, row: Option<&Row>) -> Vec<Effect> {
        let id = match row {
            Some(Row::Place(id)) => match self.groups.group_of(id) {
                Some(group) => {
                    let at = self.groups.groups[group]
                        .places
                        .iter()
                        .position(|place| place == id)
                        .map_or(usize::MAX, |at| at + 1);
                    self.groups.add_separator_in(group, at)
                }
                None if self.place(id).is_some_and(|place| !is_remote(place)) => {
                    Some(self.groups.add_loose_separator(Some(id)))
                }
                None => None,
            },
            Some(Row::Group(index)) => Some(self.groups.add_separator_between(*index)),
            None => Some(self.groups.add_separator_between(usize::MAX)),
            Some(Row::Separator(_) | Row::Device(_) | Row::Devices | Row::Remote) => None,
        };
        let Some(id) = id else {
            return Vec::new();
        };
        self.focused = Some(Row::Separator(id));
        self.changed(true)
    }

    /// Alt+Up/Down on a separator: one row up or down within its section.
    /// Among the places outside groups it goes past one place; it does not
    /// leave the section it is in.
    fn shift_separator(&mut self, id: &str, down: bool) -> Vec<Effect> {
        let moved = match self.groups.find_separator(id) {
            Some(SeparatorAt::Between(index)) => {
                let target = self.groups.shift(index, if down { 1 } else { -1 });
                target != index
            }
            Some(SeparatorAt::InGroup(..)) => self.groups.shift_place(id, down),
            Some(SeparatorAt::Loose(at)) => {
                let shown = shown_loose(&self.places, &self.groups, self.choice.show_hidden);
                let slots = loose_slots(&self.places, &shown, &self.groups);
                let Some(slot) = slots.get(at).copied().flatten() else {
                    return Vec::new();
                };
                let ids: Vec<String> = shown.iter().map(|place| place.id.clone()).collect();
                if down {
                    // Below the next place, above the separators already there.
                    let Some(next) = ids.get(slot) else {
                        return Vec::new();
                    };
                    let first_there = self
                        .groups
                        .separators
                        .iter()
                        .zip(&slots)
                        .find(|(_, at)| **at == Some(slot + 1))
                        .map(|(separator, _)| separator.id.clone());
                    self.groups
                        .move_separator_loose(id, Some(next), first_there.as_deref());
                } else {
                    // Below the place before the one above it, under the
                    // separators already there.
                    let Some(above) = slot.checked_sub(1) else {
                        return Vec::new();
                    };
                    let after = above.checked_sub(1).map(|before| ids[before].clone());
                    self.groups.move_separator_loose(id, after.as_deref(), None);
                }
                true
            }
            None => false,
        };
        if moved {
            self.changed(true)
        } else {
            Vec::new()
        }
    }

    /// The separator `id` was dragged onto `target`: it goes above that row,
    /// in that row's section.
    fn drop_separator(&mut self, id: &str, target: &Row) -> Vec<Effect> {
        match target {
            Row::Separator(other) if other == id => return Vec::new(),
            Row::Separator(other) => match self.groups.find_separator(other) {
                Some(SeparatorAt::Loose(at)) => {
                    let after = self.groups.separators[at].after.clone();
                    self.groups
                        .move_separator_loose(id, after.as_deref(), Some(other));
                }
                Some(SeparatorAt::Between(index)) => {
                    self.groups.move_separator_between(id, index);
                }
                Some(SeparatorAt::InGroup(group, _)) => {
                    self.groups.move_separator_in(id, group, Some(other));
                }
                None => return Vec::new(),
            },
            Row::Group(index) => {
                self.groups.move_separator_between(id, *index);
            }
            Row::Place(other) => match self.groups.group_of(other) {
                Some(group) => self.groups.move_separator_in(id, group, Some(other)),
                None => {
                    let shown = shown_loose(&self.places, &self.groups, self.choice.show_hidden);
                    let Some(at) = shown.iter().position(|place| place.id == *other) else {
                        return Vec::new();
                    };
                    let after = at.checked_sub(1).map(|before| shown[before].id.clone());
                    self.groups.move_separator_loose(id, after.as_deref(), None);
                }
            },
            Row::Device(_) | Row::Devices | Row::Remote => return Vec::new(),
        }
        self.focused = Some(Row::Separator(id.to_owned()));
        self.changed(true)
    }

    fn set_show_hidden(&mut self, show: bool) -> Vec<Effect> {
        self.choice.show_hidden = show;
        self.lay_out();
        vec![Effect::ShowHidden(show), Effect::Relayout]
    }
}

#[cfg(test)]
// Test helpers outside `#[test]` functions unwrap what a test cannot go on without.
#[allow(clippy::unwrap_used)]
mod tests {
    use rustphin_platform::places::EMPTY;

    use super::*;
    use crate::groups::Group;

    /// A shared places file with a bookmark for each `(id, folder, system)`.
    fn xbel(entries: &[(&str, &str, bool)]) -> String {
        let bookmarks: String = entries
            .iter()
            .map(|(id, folder, system)| {
                let title = folder.rsplit('/').next().unwrap_or(folder);
                format!(
                    " <bookmark href=\"file://{folder}\">\n  <title>{title}</title>\n  <info>\n   \
                     <metadata owner=\"http://www.kde.org\">\n    <ID>{id}</ID>\n{}   </metadata>\n  \
                     </info>\n </bookmark>\n",
                    if *system {
                        "    <isSystemItem>true</isSystemItem>\n"
                    } else {
                        ""
                    }
                )
            })
            .collect();
        EMPTY.replace("</xbel>", &format!("{bookmarks}</xbel>"))
    }

    fn group(name: &str, places: &[&str]) -> Group {
        Group {
            name: name.to_owned(),
            collapsed: false,
            places: places.iter().map(|id| (*id).to_owned()).collect(),
            separator: None,
        }
    }

    /// The panel with what the adapter does around it: the shared file
    /// edited through the platform's editor and read back, and the groups
    /// written to their own file.
    struct World {
        panel: PlacesPanel,
        file: String,
        saved: Option<Groups>,
        effects: Vec<Effect>,
    }

    impl World {
        fn new(entries: &[(&str, &str, bool)], groups: Vec<Group>) -> Self {
            let choice = PlacesChoice {
                devices: true,
                remote: true,
                show_hidden: false,
                width: 240,
            };
            let mut world = Self {
                panel: PlacesPanel::new(choice, Some(PathBuf::from("/home/u"))),
                file: xbel(entries),
                saved: None,
                effects: Vec::new(),
            };
            let mut stored = Groups::default();
            stored.groups = groups;
            let effects = world.panel.set_groups(stored);
            world.carry_out(effects);
            world.read_file();
            world.effects.clear();
            world
        }

        fn read_file(&mut self) {
            let places = Places::parse(&self.file)
                .unwrap()
                .places()
                .cloned()
                .collect();
            let effects = self.panel.set_places(places);
            self.carry_out(effects);
        }

        fn carry_out(&mut self, effects: Vec<Effect>) {
            for effect in effects {
                self.effects.push(effect.clone());
                match effect {
                    Effect::SaveGroups => {
                        self.saved = Some(Groups::parse(&self.panel.groups().to_toml()));
                    }
                    Effect::EditPlaces(edit) => {
                        let places = Places::parse(&self.file).unwrap();
                        let (text, added) = edit.apply(&places, 1_000).unwrap();
                        self.file = text;
                        let effects = self.panel.edited(&edit, added);
                        self.carry_out(effects);
                        self.read_file();
                    }
                    Effect::Relayout => {
                        let effects = self.panel.laid_out();
                        self.carry_out(effects);
                    }
                    _ => {}
                }
            }
        }

        fn command(&mut self, command: PlacesCommand) {
            let effects = self.panel.run(command).unwrap_or_default();
            self.carry_out(effects);
        }

        fn at(&self, row: &Row) -> usize {
            self.panel
                .rows()
                .iter()
                .position(|shown| shown == row)
                .unwrap_or_else(|| panic!("{row:?} is not shown in {:?}", self.panel.rows()))
        }

        fn focus(&mut self, row: Row) {
            let at = self.at(&row);
            let effects = self.panel.focus_on(Some(at));
            self.carry_out(effects);
        }

        fn ids_in_file(&self) -> Vec<String> {
            Places::parse(&self.file)
                .unwrap()
                .places()
                .map(|place| place.id.clone())
                .collect()
        }

        fn saved_group(&self, index: usize) -> Vec<String> {
            self.saved.as_ref().unwrap().groups[index].places.clone()
        }
    }

    fn p(id: &str) -> Row {
        Row::Place(id.to_owned())
    }

    /// Home (one of KDE's own), three local places, two grouped ones.
    fn world() -> World {
        World::new(
            &[
                ("home", "/home/u", true),
                ("a", "/a", false),
                ("b", "/b", false),
                ("c", "/c", false),
                ("x", "/x", false),
                ("y", "/y", false),
            ],
            vec![group("Work", &["x", "y"]), group("Media", &[])],
        )
    }

    #[test]
    fn alt_up_and_down_reorder_within_a_section_and_never_across_it() {
        let mut world = world();
        // Outside groups the order is the shared file's.
        world.focus(p("a"));
        world.command(PlacesCommand::MoveDown);
        assert_eq!(world.ids_in_file(), ["home", "b", "a", "c", "x", "y"]);
        assert_eq!(world.panel.focused(), Some(&p("a")));
        // The last local place does not jump into the group below.
        world.focus(p("c"));
        world.command(PlacesCommand::MoveDown);
        world.focus(p("home"));
        world.command(PlacesCommand::MoveUp);
        assert_eq!(world.ids_in_file(), ["home", "b", "a", "c", "x", "y"]);
        // In a group the order is the group's, saved in Rustphin's file.
        world.focus(p("x"));
        world.command(PlacesCommand::MoveUp);
        assert_eq!(world.saved, None, "the first in its group stays");
        world.command(PlacesCommand::MoveDown);
        assert_eq!(world.saved_group(0), ["y", "x"]);
        world.command(PlacesCommand::MoveDown);
        assert_eq!(
            world.saved_group(0),
            ["y", "x"],
            "the last in its group stays"
        );
        // A heading moves its group past the next one.
        world.focus(Row::Group(0));
        world.command(PlacesCommand::MoveDown);
        assert_eq!(
            world.panel.rows(),
            [
                p("home"),
                p("b"),
                p("a"),
                p("c"),
                Row::Group(0),
                Row::Group(1),
                p("y"),
                p("x"),
            ]
        );
        assert_eq!(world.panel.focused(), Some(&Row::Group(1)));
        assert_eq!(world.saved.as_ref().unwrap().groups[1].name, "Work");
    }

    #[test]
    fn delete_hides_one_of_kdes_own_places_and_removes_a_users_place() {
        let mut world = world();
        world.focus(p("home"));
        world.command(PlacesCommand::Remove);
        // KDE would bring Home back if it were removed: it is hidden.
        assert_eq!(world.ids_in_file(), ["home", "a", "b", "c", "x", "y"]);
        assert!(world.panel.places()[0].hidden);
        assert!(!world.panel.rows().contains(&p("home")));

        world.focus(p("a"));
        world.command(PlacesCommand::Remove);
        assert_eq!(world.ids_in_file(), ["home", "b", "c", "x", "y"]);
        assert_eq!(
            world.panel.focused(),
            Some(&p("b")),
            "the next row takes the keyboard"
        );

        // Showing hidden places brings Home back, faded.
        world.command(PlacesCommand::ToggleHiddenPlaces);
        assert!(world.effects.contains(&Effect::ShowHidden(true)));
        assert_eq!(world.panel.rows()[0], p("home"));
        assert!(world.panel.view(&p("home")).unwrap().hidden);
    }

    /// The path of the row labelled `label` in a menu as QML gets it,
    /// submenus included.
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
    fn a_new_group_takes_the_place_it_was_made_with_and_is_named_at_once() {
        let mut world = world();
        let at = world.at(&p("a"));
        let opened = world.panel.open_menu(Some(at)).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu in {opened:?}");
        };
        let effects = world.panel.run_menu(&path_of(json, "New group\u{2026}"));
        world.carry_out(effects);
        assert_eq!(world.saved_group(2), ["a"]);
        assert_eq!(world.panel.renaming(), Some(&Row::Group(2)));
        assert_eq!(world.panel.focused(), Some(&Row::Group(2)));
        // The field opens once the new heading is drawn, not before: drawn
        // with the field open, the row would take the keyboard from it.
        assert_eq!(
            &world.effects[world.effects.len() - 2..],
            [Effect::Relayout, Effect::Rename("New Group".into())]
        );
        assert_eq!(
            &world.panel.rows()[world.panel.rows().len() - 2..],
            [Row::Group(2), p("a")]
        );

        let effects = world.panel.finish_rename("Trips", true);
        world.carry_out(effects);
        assert_eq!(world.saved.as_ref().unwrap().groups[2].name, "Trips");

        // Ctrl+Shift+G: an empty group, focused, its name field open.
        world.command(PlacesCommand::NewGroup);
        assert_eq!(world.saved.as_ref().unwrap().groups.len(), 4);
        assert_eq!(world.panel.focused(), Some(&Row::Group(3)));
        assert_eq!(world.panel.renaming(), Some(&Row::Group(3)));
    }

    #[test]
    fn g_moves_a_place_through_each_group_and_out() {
        let mut world = world();
        world.focus(p("a"));
        world.command(PlacesCommand::NextGroup);
        assert_eq!(world.saved_group(0), ["x", "y", "a"]);
        world.command(PlacesCommand::NextGroup);
        assert_eq!(world.saved_group(0), ["x", "y"]);
        assert_eq!(world.saved_group(1), ["a"]);
        world.command(PlacesCommand::NextGroup);
        assert!(world.saved.as_ref().unwrap().group_of("a").is_none());
        assert_eq!(
            world.panel.focused(),
            Some(&p("a")),
            "the keyboard follows it"
        );
        assert_eq!(
            world.panel.rows()[1],
            p("a"),
            "back among the places outside groups"
        );
    }

    #[test]
    fn folding_a_heading_hides_its_rows_and_shows_its_count() {
        let mut world = World::new(
            &[("x", "/x", false), ("y", "/y", false)],
            vec![group("Work", &["x", "y"])],
        );
        world.focus(Row::Group(0));
        world.command(PlacesCommand::Collapse);
        assert_eq!(world.panel.rows(), [Row::Group(0)]);
        let heading = world.panel.view(&Row::Group(0)).unwrap();
        assert!(heading.collapsed);
        assert_eq!(
            (heading.title.as_str(), heading.detail.as_str()),
            ("Work", "2")
        );
        assert!(world.saved.as_ref().unwrap().groups[0].collapsed);
        world.command(PlacesCommand::Expand);
        assert_eq!(world.panel.rows(), [Row::Group(0), p("x"), p("y")]);
        // Left on a place is not a fold: the key goes elsewhere.
        world.focus(p("x"));
        assert_eq!(world.panel.run(PlacesCommand::Collapse), None);

        // The Remote heading counts what it folds away.
        let mut remote = Place {
            id: "nas".into(),
            title: "NAS".into(),
            href: "smb://nas/share".into(),
            path: None,
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        };
        let mut places = world.panel.places().to_vec();
        places.push(remote.clone());
        let effects = world.panel.set_places(places.clone());
        world.carry_out(effects);
        let at = world.at(&Row::Remote);
        let effects = world.panel.activate(at, false);
        world.carry_out(effects);
        assert_eq!(world.panel.view(&Row::Remote).unwrap().title, "Remote (1)");
        assert!(!world.panel.rows().contains(&p("nas")));
        remote.id = "nas2".into();
        places.push(remote);
        let effects = world.panel.set_places(places);
        world.carry_out(effects);
        assert_eq!(world.panel.view(&Row::Remote).unwrap().title, "Remote (2)");
        // One put in a group is the group's, not counted under Remote.
        let effects = world.panel.set_groups(Groups::parse(
            "remote_folded = true\n[[groups]]\nname = \"Work\"\nplaces = [\"x\", \"y\", \"nas2\"]\n",
        ));
        world.carry_out(effects);
        assert_eq!(world.panel.view(&Row::Remote).unwrap().title, "Remote (1)");
    }

    #[test]
    fn a_place_dropped_on_a_heading_joins_that_group_and_unfolds_it() {
        let mut media = group("Media", &[]);
        media.collapsed = true;
        let mut world = World::new(
            &[("a", "/a", false), ("x", "/x", false), ("y", "/y", false)],
            vec![group("Work", &["x", "y"]), media],
        );
        let (from, to) = (world.at(&p("a")), world.at(&Row::Group(1)));
        let effects = world.panel.drop_row(from, to);
        world.carry_out(effects);
        assert_eq!(world.saved_group(1), ["a"]);
        assert!(!world.saved.as_ref().unwrap().groups[1].collapsed);
        assert_eq!(world.panel.rows().last(), Some(&p("a")));
        // Dropped on a grouped place, it goes before that place.
        let (from, to) = (world.at(&p("a")), world.at(&p("y")));
        let effects = world.panel.drop_row(from, to);
        world.carry_out(effects);
        assert_eq!(world.saved_group(0), ["x", "a", "y"]);
    }

    #[test]
    fn folders_dropped_from_outside_become_places_where_they_were_dropped() {
        let mut world = world();
        let to = world.at(&p("y"));
        let effects = world.panel.drop_folders(
            vec![PathBuf::from("/new/one"), PathBuf::from("/new/two")],
            Some(to),
        );
        world.carry_out(effects);
        let titles: Vec<&str> = world
            .panel
            .places()
            .iter()
            .map(|p| p.title.as_str())
            .collect();
        assert_eq!(&titles[6..], ["one", "two"]);
        let added: Vec<String> = world.panel.places()[6..]
            .iter()
            .map(|p| p.id.clone())
            .collect();
        // In the group of the place they were dropped on.
        assert_eq!(world.saved_group(0)[2..], added[..]);
        assert_eq!(world.panel.focused(), Some(&p(&added[1])));
        // Dropped on empty space: outside groups. A folder already in the
        // places is not added twice.
        let effects = world
            .panel
            .drop_folders(vec![PathBuf::from("/new/three"), PathBuf::from("/a")], None);
        world.carry_out(effects);
        assert_eq!(world.panel.places().len(), 9);
        assert!(
            world
                .panel
                .groups()
                .group_of(&world.panel.places()[8].id)
                .is_none()
        );
        assert!(world.effects.contains(&Effect::Say(
            "\u{201c}/a\u{201d} is already in the places.".into()
        )));
    }

    #[test]
    fn finishing_a_rename_with_an_empty_or_unchanged_name_changes_nothing() {
        let mut world = world();
        let file = world.file.clone();
        for (typed, apply) in [("   ", true), ("a", true), (" a ", true), ("Alpha", false)] {
            world.focus(p("a"));
            world.command(PlacesCommand::Rename);
            assert_eq!(world.effects.last(), Some(&Effect::Rename("a".into())));
            world.effects.clear();
            let effects = world.panel.finish_rename(typed, apply);
            assert_eq!(effects, [Effect::Relayout], "{typed:?}");
            assert_eq!(world.panel.renaming(), None);
        }
        for (typed, apply) in [("", true), ("Work", true), ("Other", false)] {
            world.focus(Row::Group(0));
            world.command(PlacesCommand::Rename);
            let effects = world.panel.finish_rename(typed, apply);
            assert_eq!(effects, [Effect::Relayout], "{typed:?}");
        }
        assert_eq!(world.file, file);
        assert_eq!(world.saved, None);
        // A real name is kept.
        world.focus(p("a"));
        world.command(PlacesCommand::Rename);
        let effects = world.panel.finish_rename(" Alpha ", true);
        world.carry_out(effects);
        assert_eq!(world.panel.view(&p("a")).unwrap().title, "Alpha");
    }

    #[test]
    fn the_keyboard_never_lands_on_a_row_that_is_not_shown() {
        let mut media = group("Media", &["m"]);
        media.collapsed = true;
        let mut world = World::new(
            &[("a", "/a", false), ("h", "/h", false), ("m", "/m", false)],
            vec![media],
        );
        world.focus(p("h"));
        world.command(PlacesCommand::ToggleHide);
        assert_eq!(
            world.panel.focused(),
            None,
            "a place hidden loses the keyboard"
        );
        let mut visited = Vec::new();
        for _ in 0..5 {
            world.command(PlacesCommand::FocusNext);
            visited.push(world.panel.focused().cloned().unwrap());
        }
        for _ in 0..5 {
            world.command(PlacesCommand::FocusPrevious);
            visited.push(world.panel.focused().cloned().unwrap());
        }
        assert!(
            visited
                .iter()
                .all(|row| *row == p("a") || *row == Row::Group(0))
        );
        // Folding a group away takes the keyboard off its places.
        world.focus(Row::Group(0));
        world.command(PlacesCommand::Expand);
        world.focus(p("m"));
        let effects = world.panel.set_groups(Groups::parse(
            "[[groups]]\nname = \"Media\"\ncollapsed = true\nplaces = [\"m\"]\n",
        ));
        world.carry_out(effects);
        assert_eq!(world.panel.focused(), None);
    }

    #[test]
    fn a_reread_of_the_same_places_leaves_a_rename_going() {
        let mut world = world();
        world.focus(p("a"));
        world.command(PlacesCommand::Rename);
        let same = world.panel.places().to_vec();
        assert_eq!(world.panel.set_places(same), []);
        // Another place changed: the rows are redrawn, the rename goes on.
        world.file = world.file.replace("<title>b</title>", "<title>Bee</title>");
        world.read_file();
        assert_eq!(world.panel.renaming(), Some(&p("a")));
    }

    #[test]
    fn the_hint_shows_until_a_place_or_group_is_listed_whatever_the_devices() {
        let mut world = World::new(&[], Vec::new());
        let stick = Device {
            id: "sdb1".into(),
            label: Some("STICK".into()),
            mount_point: None,
            size: 1,
            removable: true,
            power_off: Some("/drive".into()),
            filesystem: None,
            encrypted: false,
        };
        let effects = world.panel.set_devices(vec![stick.clone()], HashMap::new());
        world.carry_out(effects);
        assert_eq!(
            world.panel.rows(),
            [Row::Devices, Row::Device("sdb1".into())]
        );
        assert!(!world.panel.has_places());
        world.command(PlacesCommand::NewGroup);
        assert!(world.panel.has_places());

        // A volume that is not mounted is marked busy and mounted first.
        let at = world.at(&Row::Device("sdb1".into()));
        let effects = world.panel.activate(at, true);
        assert!(world.panel.view(&Row::Device("sdb1".into())).unwrap().busy);
        assert_eq!(
            effects.last(),
            Some(&Effect::Open {
                opening: Opening::Mount(stick),
                new_tab: true
            })
        );
    }

    fn device(id: &str, mount: Option<&str>, removable: bool) -> Device {
        Device {
            id: id.into(),
            label: Some(id.to_uppercase()),
            mount_point: mount.map(PathBuf::from),
            size: 1,
            removable,
            power_off: removable.then(|| "/drive".to_owned()),
            filesystem: Some("ext4".to_owned()),
            encrypted: false,
        }
    }

    /// The world with four volumes: a data disk a quarter free, a stick
    /// not mounted, the system's and home's volumes.
    fn with_devices() -> World {
        let mut world = world();
        let usage = HashMap::from([
            ("data".to_owned(), (1000, 250)),
            ("empty".to_owned(), (0, 0)),
        ]);
        let effects = world.panel.set_devices(
            vec![
                device("data", Some("/data"), false),
                device("stick", None, true),
                device("root", Some("/"), false),
                device("home", Some("/home"), false),
                device("empty", Some("/empty"), false),
                device("bare", Some("/bare"), false),
            ],
            usage,
        );
        world.carry_out(effects);
        world
    }

    fn d(id: &str) -> Row {
        Row::Device(id.to_owned())
    }

    #[test]
    fn a_volume_row_shows_its_card() {
        let mut world = with_devices();
        assert!(world.panel.set_current(Some(PathBuf::from("/data"))));
        let data = world.panel.view(&d("data")).unwrap();
        assert_eq!(data.kind, RowKind::Device);
        assert_eq!(data.title, "DATA");
        assert_eq!(data.icon, "hard-drive");
        assert!(data.current);
        let card = super::super::device_card(
            &device("data", Some("/data"), false),
            Some((1000, 250)),
            false,
        );
        assert_eq!(
            (data.detail, data.figures, data.percent, data.description),
            (card.traits, card.figures, card.percent, card.description)
        );
        assert!((data.usage - 0.75).abs() < 1e-9, "{}", data.usage);
        assert!(!data.nearly_full && !data.busy);

        let stick = world.panel.view(&d("stick")).unwrap();
        assert_eq!(stick.icon, "usb");
        assert!(!stick.current, "not mounted, so not the folder shown");
        assert_eq!(stick.detail, "Not mounted");
        assert!((stick.usage + 1.).abs() < 1e-9, "unknown: {}", stick.usage);

        assert!(!world.panel.view(&d("bare")).unwrap().current);
        // A volume of no size is not full.
        assert!(world.panel.view(&d("empty")).unwrap().usage.abs() < 1e-9);

        let effects = world.panel.set_busy(Some("data".into()));
        world.carry_out(effects);
        let data = world.panel.view(&d("data")).unwrap();
        assert!(data.busy);
        assert_eq!(data.detail, "Working\u{2026}");
        assert!(!world.panel.view(&d("stick")).unwrap().busy);
    }

    #[test]
    fn a_nearly_full_volume_row_warns() {
        let mut world = world();
        let effects = world.panel.set_devices(
            vec![device("full", Some("/full"), false)],
            HashMap::from([("full".to_owned(), (1000, 40))]),
        );
        world.carry_out(effects);
        let full = world.panel.view(&d("full")).unwrap();
        assert!(full.nearly_full);
        assert_eq!(full.percent, "96% used");
    }

    #[test]
    fn with_nothing_shown_no_volume_is_the_current_one() {
        let world = with_devices();
        // Neither the stick (no mount) nor any other is current.
        assert!(!world.panel.view(&d("stick")).unwrap().current);
        assert!(!world.panel.view(&d("data")).unwrap().current);
    }

    #[test]
    fn the_trash_place_is_marked_while_the_trash_is_shown() {
        let mut world = world();
        let mut places = world.panel.places().to_vec();
        places.push(Place {
            id: "trash".into(),
            title: "Trash".into(),
            href: "trash:/".into(),
            path: None,
            icon: None,
            hidden: false,
            system: true,
            only_in_app: None,
        });
        let effects = world.panel.set_places(places);
        world.carry_out(effects);
        let files = PathBuf::from("/home/u/.local/share/Trash/files");
        world.panel.set_trash_folder(Some(files.clone()));

        world.panel.set_current(Some(files));
        assert!(world.panel.view(&p("trash")).unwrap().current);
        assert!(!world.panel.view(&p("a")).unwrap().current);

        world.panel.set_current(Some(PathBuf::from("/a")));
        assert!(!world.panel.view(&p("trash")).unwrap().current);
        // No folder shown is not the trash, even before its folder is known.
        world.panel.set_trash_folder(None);
        world.panel.set_current(None);
        assert!(!world.panel.view(&p("trash")).unwrap().current);
    }

    #[test]
    fn a_place_row_shows_its_kind_icon_and_whether_it_is_shown_or_grouped() {
        let mut world = world();
        let mut places = world.panel.places().to_vec();
        places.push(Place {
            id: "nas".into(),
            title: "NAS".into(),
            href: "smb://nas/share".into(),
            path: None,
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        });
        let effects = world.panel.set_places(places);
        world.carry_out(effects);
        world.panel.set_current(Some(PathBuf::from("/a")));
        let a = world.panel.view(&p("a")).unwrap();
        assert_eq!(a.kind, RowKind::Place);
        assert_eq!(a.icon, "folder");
        assert!(a.current && !a.grouped);
        assert!(!world.panel.view(&p("b")).unwrap().current);
        assert!(world.panel.view(&p("x")).unwrap().grouped);
        // A place without a folder is never the folder shown.
        assert!(!world.panel.view(&p("nas")).unwrap().current);
        world.panel.set_current(None);
        assert!(!world.panel.view(&p("nas")).unwrap().current);
    }

    #[test]
    fn heading_rows_show_their_kind_title_and_fold() {
        let mut world = with_devices();
        let mut unnamed = Groups::parse(&world.panel.groups().to_toml());
        unnamed.groups[1].name = String::new();
        unnamed.groups[1].collapsed = true;
        unnamed.devices_folded = true;
        unnamed.remote_folded = true;
        let effects = world.panel.set_groups(unnamed);
        world.carry_out(effects);
        let group = world.panel.view(&Row::Group(1)).unwrap();
        assert_eq!(group.kind, RowKind::Group);
        assert_eq!(group.title, "Group");
        assert!(group.collapsed);
        let devices = world.panel.view(&Row::Devices).unwrap();
        assert_eq!(devices.kind, RowKind::Heading);
        assert_eq!(devices.title, "Devices (6)");
        assert!(devices.collapsed);
        assert!(world.panel.view(&Row::Remote).unwrap().collapsed);
        assert_eq!(RowKind::Place.name(), "place");
        assert_eq!(RowKind::Group.name(), "group");
        assert_eq!(RowKind::Device.name(), "device");
        assert_eq!(RowKind::Heading.name(), "heading");
    }

    #[test]
    fn when_volumes_cannot_be_listed_the_panel_says_so_under_devices() {
        let mut world = with_devices();
        assert_eq!(world.panel.devices_note(), "");
        let effects = world.panel.devices_failed();
        assert_eq!(effects, [Effect::Relayout]);
        world.carry_out(effects);
        assert_eq!(world.panel.devices_note(), DEVICES_UNAVAILABLE);
        assert_eq!(world.panel.rows().last(), Some(&Row::Devices));
        assert!(!world.panel.rows().contains(&d("data")));
        // With Devices turned off, nothing is said.
        let effects = world.panel.set_choice(PlacesChoice {
            devices: false,
            remote: true,
            show_hidden: false,
            width: 240,
        });
        assert_eq!(effects, [Effect::Relayout]);
        assert_eq!(world.panel.devices_note(), "");
        assert!(!world.panel.rows().contains(&Row::Devices));
    }

    #[test]
    fn only_a_change_of_sections_lays_the_rows_out_again() {
        let mut world = world();
        let choice = |width| PlacesChoice {
            devices: true,
            remote: true,
            show_hidden: false,
            width,
        };
        assert_eq!(world.panel.set_choice(choice(300)), []);
        assert_eq!(
            world.panel.set_choice(PlacesChoice {
                show_hidden: true,
                ..choice(300)
            }),
            [Effect::Relayout]
        );
    }

    #[test]
    fn the_folder_shown_is_told_once_per_change() {
        let mut world = world();
        assert!(world.panel.set_current(Some(PathBuf::from("/a"))));
        assert!(!world.panel.set_current(Some(PathBuf::from("/a"))));
        assert!(world.panel.set_current(None));
    }

    #[test]
    fn the_keyboard_enters_the_panel_at_the_top_going_down_and_the_bottom_going_up() {
        let mut world = world();
        world.command(PlacesCommand::FocusPrevious);
        assert_eq!(world.panel.focused(), world.panel.rows().last());
        let effects = world.panel.focus_on(None);
        world.carry_out(effects);
        world.command(PlacesCommand::FocusNext);
        assert_eq!(world.panel.focused(), Some(&p("home")));
        // From a row, it moves one row.
        world.focus(p("b"));
        world.command(PlacesCommand::FocusNext);
        assert_eq!(world.panel.focused(), Some(&p("c")));
        world.command(PlacesCommand::FocusPrevious);
        assert_eq!(world.panel.focused(), Some(&p("b")));
    }

    #[test]
    fn e_safely_removes_a_removable_drive_unmounts_a_fixed_one_and_spares_the_system() {
        let mut world = with_devices();
        world.focus(d("stick"));
        let effects = world.panel.run(PlacesCommand::Eject).unwrap();
        assert_eq!(
            effects.last(),
            Some(&Effect::SafelyRemove(device("stick", None, true)))
        );
        assert!(world.panel.view(&d("stick")).unwrap().busy);

        world.focus(d("data"));
        let effects = world.panel.run(PlacesCommand::Eject).unwrap();
        assert_eq!(
            effects.last(),
            Some(&Effect::Unmount(device("data", Some("/data"), false)))
        );

        // A mounted stick is removed, not only unmounted.
        let mounted = device("usb", Some("/run/media/u/USB"), true);
        let effects = world
            .panel
            .set_devices(vec![mounted.clone()], HashMap::new());
        world.carry_out(effects);
        world.focus(d("usb"));
        let effects = world.panel.run(PlacesCommand::Eject).unwrap();
        assert_eq!(effects.last(), Some(&Effect::SafelyRemove(mounted)));
        let effects = world.panel.set_devices(
            vec![
                device("root", Some("/"), false),
                device("home", Some("/home"), false),
            ],
            HashMap::new(),
        );
        world.carry_out(effects);

        for spared in ["root", "home"] {
            world.focus(d(spared));
            assert_eq!(world.panel.run(PlacesCommand::Eject), Some(Vec::new()));
        }
        world.focus(p("a"));
        assert_eq!(world.panel.run(PlacesCommand::Eject), Some(Vec::new()));
    }

    /// Opens the menu on `row` and chooses `label` in it.
    fn choose(world: &mut World, row: &Row, label: &str) -> Vec<Effect> {
        let at = world.at(row);
        let opened = world.panel.open_menu(Some(at)).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu in {opened:?}");
        };
        world.panel.run_menu(&path_of(json, label))
    }

    fn menu_labels(world: &mut World, row: &Row) -> Vec<String> {
        let at = world.at(row);
        let opened = world.panel.open_menu(Some(at)).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu in {opened:?}");
        };
        let rows: serde_json::Value = serde_json::from_str(json).unwrap();
        world.panel.run_menu("");
        rows.as_array()
            .unwrap()
            .iter()
            .map(|row| row["label"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn a_drive_has_its_own_menu_and_the_systems_offers_no_removal() {
        let mut world = with_devices();
        assert_eq!(
            menu_labels(&mut world, &d("stick")),
            ["Open", "Open in new tab", "", "Mount", "Safely remove"]
        );
        assert_eq!(
            menu_labels(&mut world, &d("root")),
            ["Open", "Open in new tab", "", "Copy location"]
        );
        assert_eq!(
            menu_labels(&mut world, &d("data")),
            [
                "Open",
                "Open in new tab",
                "",
                "Unmount",
                "",
                "Copy location"
            ]
        );
        // The menu takes the keyboard to the drive it was opened on.
        assert_eq!(world.panel.focused(), Some(&d("data")));
    }

    #[test]
    fn the_drive_menu_mounts_unmounts_and_removes_the_drive_it_was_opened_on() {
        let mut world = with_devices();
        let effects = choose(&mut world, &d("stick"), "Mount");
        assert_eq!(
            effects.last(),
            Some(&Effect::Mount(device("stick", None, true)))
        );
        assert!(world.panel.view(&d("stick")).unwrap().busy);
        let effects = choose(&mut world, &d("stick"), "Safely remove");
        assert_eq!(
            effects.last(),
            Some(&Effect::SafelyRemove(device("stick", None, true)))
        );
        let effects = choose(&mut world, &d("data"), "Unmount");
        assert_eq!(effects[0], Effect::Menu(String::new()));
        assert_eq!(
            effects.last(),
            Some(&Effect::Unmount(device("data", Some("/data"), false)))
        );
    }

    #[test]
    fn copy_location_puts_the_mount_point_on_the_clipboard_and_says_so() {
        let mut world = with_devices();
        let effects = choose(&mut world, &d("data"), "Copy location");
        assert_eq!(
            effects,
            [
                Effect::Menu(String::new()),
                Effect::CopyLocation(PathBuf::from("/data")),
                Effect::Say("Copied the location of \u{201c}DATA\u{201d}.".to_owned()),
            ]
        );
    }

    #[test]
    fn a_drive_opens_from_its_menu_mounting_it_first_when_needed() {
        let mut world = with_devices();
        let effects = choose(&mut world, &d("data"), "Open in new tab");
        assert_eq!(
            effects.last(),
            Some(&Effect::Open {
                opening: Opening::Folder("/data".into()),
                new_tab: true
            })
        );
        let effects = choose(&mut world, &d("stick"), "Open");
        assert_eq!(
            effects.last(),
            Some(&Effect::Open {
                opening: Opening::Mount(device("stick", None, true)),
                new_tab: false
            })
        );
    }

    #[test]
    fn the_menu_opens_in_this_tab_or_a_new_one_and_closes_once_chosen() {
        let mut world = world();
        for (label, new_tab) in [("Open", false), ("Open in new tab", true)] {
            let at = world.at(&p("a"));
            let opened = world.panel.open_menu(Some(at)).unwrap();
            let Some(Effect::Menu(json)) = opened.first() else {
                panic!("no menu");
            };
            let effects = world.panel.run_menu(&path_of(json, label));
            assert_eq!(
                effects,
                [
                    Effect::Menu(String::new()),
                    Effect::Open {
                        opening: Opening::Folder("/a".into()),
                        new_tab
                    }
                ],
                "{label}"
            );
        }
    }

    #[test]
    fn a_group_heading_folds_from_its_menu() {
        let mut world = world();
        let at = world.at(&Row::Group(0));
        let opened = world.panel.open_menu(Some(at)).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu");
        };
        let effects = world.panel.run_menu(&path_of(json, "Collapse"));
        world.carry_out(effects);
        assert!(world.saved.as_ref().unwrap().groups[0].collapsed);
        let at = world.at(&Row::Group(0));
        let opened = world.panel.open_menu(Some(at)).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu");
        };
        let effects = world.panel.run_menu(&path_of(json, "Expand"));
        world.carry_out(effects);
        assert!(!world.saved.as_ref().unwrap().groups[0].collapsed);
    }

    #[test]
    fn the_panel_menu_shows_hidden_places() {
        let mut world = world();
        let opened = world.panel.open_menu(None).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu");
        };
        let effects = world.panel.run_menu(&path_of(json, "Show hidden places"));
        assert!(effects.contains(&Effect::ShowHidden(true)), "{effects:?}");
    }

    #[test]
    fn a_place_dropped_on_itself_or_a_group_on_itself_changes_nothing() {
        let mut world = world();
        let at = world.at(&p("a"));
        assert_eq!(world.panel.drop_row(at, at), []);
        let at = world.at(&Row::Group(0));
        assert_eq!(world.panel.drop_row(at, at), []);
        // A grouped place dropped on its own group's heading: the heading
        // is its group.
        let (from, to) = (world.at(&p("x")), world.at(&Row::Group(0)));
        let effects = world.panel.drop_row(from, to);
        world.carry_out(effects);
        assert_eq!(world.saved_group(0), ["y", "x"]);
    }

    #[test]
    fn a_heading_dropped_on_another_takes_its_place() {
        let mut world = world();
        let (from, to) = (world.at(&Row::Group(0)), world.at(&Row::Group(1)));
        let effects = world.panel.drop_row(from, to);
        world.carry_out(effects);
        let names: Vec<String> = world
            .saved
            .as_ref()
            .unwrap()
            .groups
            .iter()
            .map(|group| group.name.clone())
            .collect();
        assert_eq!(names, ["Media", "Work"]);
        assert_eq!(world.panel.focused(), Some(&Row::Group(1)));
    }

    #[test]
    fn folders_dropped_on_a_heading_go_into_its_group() {
        let mut world = world();
        let to = world.at(&Row::Group(1));
        let effects = world
            .panel
            .drop_folders(vec![PathBuf::from("/new/one")], Some(to));
        world.carry_out(effects);
        let added = world.panel.places().last().unwrap().id.clone();
        assert_eq!(world.saved_group(1), [added]);
    }

    #[test]
    fn settings_moves_renames_and_deletes_groups_and_saves_them() {
        let mut world = world();
        let effects = world.panel.shift_group(0, 1);
        assert_eq!(effects, [Effect::SaveGroups, Effect::Relayout]);
        world.carry_out(effects);
        assert_eq!(world.saved.as_ref().unwrap().groups[1].name, "Work");
        let effects = world.panel.rename_group(1, "Jobs");
        assert_eq!(effects, [Effect::SaveGroups, Effect::Relayout]);
        world.carry_out(effects);
        assert_eq!(world.saved.as_ref().unwrap().groups[1].name, "Jobs");
        let effects = world.panel.delete_group(0);
        assert_eq!(effects, [Effect::SaveGroups, Effect::Relayout]);
        world.carry_out(effects);
        assert_eq!(world.saved.as_ref().unwrap().groups.len(), 1);
    }

    #[test]
    fn the_devices_heading_folds_and_unfolds_when_clicked() {
        let mut world = with_devices();
        let at = world.at(&Row::Devices);
        let effects = world.panel.activate(at, false);
        world.carry_out(effects);
        assert!(world.saved.as_ref().unwrap().devices_folded);
        assert!(!world.panel.rows().contains(&d("data")));
        let at = world.at(&Row::Devices);
        let effects = world.panel.activate(at, false);
        world.carry_out(effects);
        assert!(!world.saved.as_ref().unwrap().devices_folded);
    }

    #[test]
    fn alt_up_moves_a_heading_up() {
        let mut world = world();
        world.focus(Row::Group(1));
        world.command(PlacesCommand::MoveUp);
        assert_eq!(world.saved.as_ref().unwrap().groups[0].name, "Media");
        assert_eq!(world.panel.focused(), Some(&Row::Group(0)));
    }

    #[test]
    fn alt_up_passes_over_hidden_places() {
        let mut world = World::new(
            &[("a", "/a", false), ("h", "/h", false), ("b", "/b", false)],
            Vec::new(),
        );
        world.focus(p("h"));
        world.command(PlacesCommand::ToggleHide);
        world.focus(p("b"));
        world.command(PlacesCommand::MoveUp);
        assert_eq!(world.ids_in_file(), ["b", "a", "h"]);
    }

    #[test]
    fn g_does_nothing_without_groups() {
        let mut world = World::new(&[("a", "/a", false)], Vec::new());
        world.focus(p("a"));
        assert_eq!(world.panel.run(PlacesCommand::NextGroup), Some(Vec::new()));
    }

    #[test]
    fn an_address_hides_no_password_in_the_logs() {
        let address = RemoteAddress::parse("sftp://me:hunter2@host/srv").unwrap();
        let shown = format!(
            "{:?} {:?} {:?} {:?}",
            Opening::Remote(address),
            Opening::Trash,
            Opening::Folder("/a".into()),
            Opening::NetworkFolder("nas".into())
        );
        assert_eq!(
            shown,
            "Remote(..) Trash Folder(\"/a\") NetworkFolder(\"nas\")"
        );
    }

    #[test]
    fn a_row_shows_whether_it_has_the_keyboard_or_its_name_is_edited() {
        let mut world = world();
        world.focus(p("a"));
        world.command(PlacesCommand::Rename);
        let a = world.panel.view(&p("a")).unwrap();
        assert!(a.focused && a.renaming);
        let b = world.panel.view(&p("b")).unwrap();
        assert!(!b.focused && !b.renaming);
        // Only volumes have a usage; every other row's is unknown.
        assert!((a.usage + 1.).abs() < 1e-9, "{}", a.usage);
        let heading = world.panel.view(&Row::Group(0)).unwrap();
        assert!((heading.usage + 1.).abs() < 1e-9, "{}", heading.usage);
    }

    #[test]
    fn a_click_on_a_group_heading_folds_and_unfolds_it() {
        let mut world = world();
        let at = world.at(&Row::Group(0));
        let effects = world.panel.activate(at, false);
        world.carry_out(effects);
        assert!(world.saved.as_ref().unwrap().groups[0].collapsed);
        assert!(!world.panel.rows().contains(&p("x")));
        let effects = world.panel.activate(at, false);
        world.carry_out(effects);
        assert!(!world.saved.as_ref().unwrap().groups[0].collapsed);
        assert!(world.panel.rows().contains(&p("x")));
    }

    // ----- Separators (W-152) -----

    fn sep(number: u32) -> Row {
        Row::Separator(format!("separator-{number}"))
    }

    fn choose_on_empty_space(world: &mut World, label: &str) {
        let opened = world.panel.open_menu(None).unwrap();
        let Some(Effect::Menu(json)) = opened.first() else {
            panic!("no menu in {opened:?}");
        };
        let effects = world.panel.run_menu(&path_of(json, label));
        world.carry_out(effects);
    }

    fn choose_and_carry_out(world: &mut World, row: &Row, label: &str) {
        let effects = choose(world, row, label);
        world.carry_out(effects);
    }

    /// The world with a separator below "a", one between the groups and
    /// one between the places of Work.
    fn separated() -> World {
        let mut world = world();
        choose_and_carry_out(&mut world, &p("a"), "Add separator below");
        choose_and_carry_out(&mut world, &Row::Group(1), "Add separator above");
        choose_and_carry_out(&mut world, &p("x"), "Add separator below");
        world.effects.clear();
        world
    }

    #[test]
    fn a_separator_is_added_below_a_place_above_a_group_or_after_the_last_group() {
        let mut world = separated();
        assert_eq!(world.panel.focused(), Some(&sep(3)));
        choose_on_empty_space(&mut world, "Add separator");
        assert_eq!(
            world.panel.rows(),
            [
                p("home"),
                p("a"),
                sep(1),
                p("b"),
                p("c"),
                Row::Group(0),
                p("x"),
                sep(3),
                p("y"),
                sep(2),
                Row::Group(2),
                sep(4),
            ]
        );
        assert_eq!(world.panel.focused(), Some(&sep(4)));
        // Kept in Rustphin's own file, never in the shared one.
        let saved = world.saved.clone().unwrap();
        assert_eq!(saved.separators, world.panel.groups().separators);
        assert_eq!(saved.groups, world.panel.groups().groups);
        assert_eq!(world.ids_in_file(), ["home", "a", "b", "c", "x", "y"]);
        // A separator is drawn as one, and inside a group indented with it.
        let view = |row: &Row| world.panel.view(row).unwrap();
        assert_eq!(view(&sep(1)).kind, RowKind::Separator);
        assert!(view(&sep(3)).grouped && !view(&sep(1)).grouped && !view(&sep(2)).grouped);
        // A group counts its places, not its separators.
        assert_eq!(view(&Row::Group(0)).detail, "2");
    }

    #[test]
    fn a_separator_has_its_own_menu_and_a_network_place_offers_none_below_it() {
        let mut world = separated();
        assert_eq!(
            menu_labels(&mut world, &sep(1)),
            ["Move up", "Move down", "", "Remove separator"]
        );
        let mut remote = World::new(&[("home", "/home/u", true)], Vec::new());
        remote.file = remote.file.replace(
            "</xbel>",
            " <bookmark href=\"sftp://me@host/\">\n  <title>vps</title>\n  <info>\n   \
             <metadata owner=\"http://www.kde.org\">\n    <ID>vps</ID>\n   </metadata>\n  \
             </info>\n </bookmark>\n</xbel>",
        );
        remote.read_file();
        assert!(!menu_labels(&mut remote, &p("vps")).contains(&"Add separator below".to_owned()));
        assert!(menu_labels(&mut remote, &p("home")).contains(&"Add separator below".to_owned()));
    }

    #[test]
    fn alt_up_and_down_move_a_separator_within_its_section() {
        let mut world = separated();
        let rows = |world: &World| world.panel.rows().to_vec();
        // Outside groups it goes past one place at a time, and stops at
        // the ends of the section.
        world.focus(sep(1));
        world.command(PlacesCommand::MoveDown);
        assert_eq!(
            rows(&world)[..5],
            [p("home"), p("a"), p("b"), sep(1), p("c")]
        );
        world.command(PlacesCommand::MoveDown);
        world.command(PlacesCommand::MoveDown);
        assert_eq!(
            rows(&world)[..5],
            [p("home"), p("a"), p("b"), p("c"), sep(1)]
        );
        for _ in 0..5 {
            world.command(PlacesCommand::MoveUp);
        }
        assert_eq!(rows(&world)[..2], [sep(1), p("home")]);
        assert_eq!(world.panel.focused(), Some(&sep(1)));
        // In a group, among its places.
        world.focus(sep(3));
        world.command(PlacesCommand::MoveDown);
        assert_eq!(world.saved_group(0), ["x", "y", "separator-3"]);
        // Between groups, past a group.
        world.focus(sep(2));
        world.command(PlacesCommand::MoveUp);
        assert_eq!(
            rows(&world)[5..],
            [sep(2), Row::Group(1), p("x"), p("y"), sep(3), Row::Group(2)]
        );
        // The shared file never moved.
        assert_eq!(world.ids_in_file(), ["home", "a", "b", "c", "x", "y"]);
    }

    #[test]
    fn delete_takes_a_separator_away_and_the_keyboard_stays_there() {
        let mut world = separated();
        world.focus(sep(1));
        world.command(PlacesCommand::Remove);
        assert_eq!(world.panel.rows()[..3], [p("home"), p("a"), p("b")]);
        assert_eq!(world.panel.focused(), Some(&p("b")));
        assert!(world.saved.as_ref().unwrap().separators.is_empty());
        choose_and_carry_out(&mut world, &sep(3), "Remove separator");
        assert_eq!(world.saved_group(0), ["x", "y"]);
        // No place left the shared file or its group.
        assert_eq!(world.ids_in_file(), ["home", "a", "b", "c", "x", "y"]);
        // Renaming, folding and opening do nothing to a separator.
        world.focus(sep(2));
        for command in [
            PlacesCommand::Rename,
            PlacesCommand::Collapse,
            PlacesCommand::Open,
        ] {
            let effects = world.panel.run(command).unwrap_or_default();
            assert!(
                effects.iter().all(|effect| *effect == Effect::Focus),
                "{command:?}: {effects:?}"
            );
        }
    }

    #[test]
    fn a_dragged_separator_goes_above_the_row_it_is_dropped_on() {
        let mut world = separated();
        let drop = |world: &mut World, from: Row, to: Row| {
            let (from, to) = (world.at(&from), world.at(&to));
            let effects = world.panel.drop_row(from, to);
            world.carry_out(effects);
        };
        // Onto a group heading: between the groups, above it.
        drop(&mut world, sep(1), Row::Group(0));
        assert_eq!(
            world.panel.rows()[..6],
            [p("home"), p("a"), p("b"), p("c"), sep(1), Row::Group(1)]
        );
        // Onto a place in a group: into the group, above it.
        drop(&mut world, sep(1), p("y"));
        assert_eq!(
            world.saved_group(0),
            ["x", "separator-3", "separator-1", "y"]
        );
        // Onto a place outside groups: above it.
        drop(&mut world, sep(1), p("b"));
        assert_eq!(world.panel.rows()[..4], [p("home"), p("a"), sep(1), p("b")]);
        // Onto another separator: above that one.
        drop(&mut world, sep(2), sep(3));
        assert_eq!(
            world.saved_group(0),
            ["x", "separator-2", "separator-3", "y"]
        );
        assert_eq!(world.panel.focused(), Some(&sep(2)));
        // A place dropped on a separator in a group goes above it.
        drop(&mut world, p("c"), sep(3));
        assert_eq!(
            world.saved_group(0),
            ["x", "separator-2", "c", "separator-3", "y"]
        );
    }

    #[test]
    fn a_separator_below_a_place_goes_with_it_and_follows_it_out_of_sight() {
        let mut world = separated();
        // Hidden: the separator stays below the place shown before it.
        world.focus(p("a"));
        world.command(PlacesCommand::ToggleHide);
        assert_eq!(world.panel.rows()[..3], [p("home"), sep(1), p("b")]);
        // Removed from the places: the separator goes too.
        world.focus(p("b"));
        choose_and_carry_out(&mut world, &p("b"), "Add separator below");
        assert_eq!(world.panel.rows()[..4], [p("home"), sep(1), p("b"), sep(4)]);
        world.focus(p("b"));
        world.command(PlacesCommand::Remove);
        assert_eq!(world.panel.rows()[..3], [p("home"), sep(1), p("c")]);
        assert!(!world.panel.rows().contains(&sep(4)));
    }

    #[test]
    fn separators_between_groups_are_no_groups_to_sort_places_into() {
        let mut world = separated();
        let effects = choose(&mut world, &p("a"), "Media");
        world.carry_out(effects);
        assert_eq!(world.saved_group(2), ["a"]);
        // G goes from Work straight to Media, past the separator.
        world.focus(p("b"));
        world.command(PlacesCommand::NextGroup);
        assert_eq!(world.saved_group(0), ["x", "separator-3", "y", "b"]);
        world.command(PlacesCommand::NextGroup);
        assert_eq!(world.saved_group(2), ["a", "b"]);
        assert!(
            world.saved_group(1).is_empty(),
            "the separator holds nothing"
        );
        // A folder dropped on a separator in a group joins that group.
        let at = world.at(&sep(3));
        let effects = world
            .panel
            .drop_folders(vec![PathBuf::from("/new")], Some(at));
        assert!(
            effects.contains(&Effect::EditPlaces(PlacesEdit::Add {
                folder: PathBuf::from("/new"),
                title: "new".to_owned(),
                group: Some(0),
            })),
            "{effects:?}"
        );
    }
}
