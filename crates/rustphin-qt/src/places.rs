//! The places panel as a QML list model (W-114e): KDE's shared places
//! (`user-places.xbel`) with Rustphin's own groups (D-008, W-105), the
//! Remote section, and the storage volumes from UDisks2 with their usage.
//! Files are read and written on other threads; the rows and what they
//! mean come from `rustphin_view::places_panel`.

// cxx-qt generates the unsafe FFI glue for the model; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "places_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use rustphin_core::{Fs as _, RealFs, WatchGuard, display_name};
use rustphin_platform::{
    config,
    devices::{self, Device, Devices, NoDevices, UDisks2},
    places::{EMPTY, Place, Places, PlacesError},
    remote::{network_folder_address, network_folder_name},
    xdg,
};
use rustphin_view::{
    groups::Groups,
    menu::{self, MenuEntry},
    place_menu::{self, PlaceCommand, Target},
    places_panel::{
        DEVICES_UNAVAILABLE, Row, can_eject, device_name, icon_for, is_remote, is_shown,
        network_address, rows, usage_text,
    },
    settings::PlacesChoice,
};

use crate::{hub, prefs};

/// How long a burst of changes to the places file or the volumes settles.
const RELOAD_DELAY: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Kind = 0x100,
    Title,
    Icon,
    Current,
    Hidden,
    Collapsed,
    Detail,
    Usage,
    Busy,
    Ejectable,
    Focused,
    Renaming,
    Grouped,
}

impl Role {
    const ALL: [Role; 13] = [
        Role::Kind,
        Role::Title,
        Role::Icon,
        Role::Current,
        Role::Hidden,
        Role::Collapsed,
        Role::Detail,
        Role::Usage,
        Role::Busy,
        Role::Ejectable,
        Role::Focused,
        Role::Renaming,
        Role::Grouped,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Kind => "kind",
            Role::Title => "title",
            Role::Icon => "icon",
            Role::Current => "current",
            Role::Hidden => "placeHidden",
            Role::Collapsed => "collapsed",
            Role::Detail => "detail",
            Role::Usage => "usage",
            Role::Busy => "busy",
            Role::Ejectable => "ejectable",
            Role::Focused => "focused",
            Role::Renaming => "renaming",
            Role::Grouped => "grouped",
        }
    }
}

/// The time now, in seconds since the Unix epoch; KDE builds identifiers from it.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The Rust side of `PlacesModel`.
pub struct PlacesModelRust {
    count: i32,
    focus_row: i32,
    renaming_row: i32,
    rename_text: QString,
    devices_note: QString,
    menu: QString,
    width: i32,

    started: bool,
    places_path: Option<PathBuf>,
    groups_path: Option<PathBuf>,
    places: Vec<Place>,
    places_load: u64,
    places_notice: Option<String>,
    groups: Groups,
    choice: PlacesChoice,
    rows: Vec<Row>,
    focused: Option<Row>,
    renaming: Option<Row>,
    rename_requested: Option<Row>,
    current: Option<PathBuf>,
    devices: Vec<Device>,
    devices_unavailable: bool,
    usage: HashMap<String, (u64, u64)>,
    device_service: Arc<dyn Devices>,
    device_busy: Option<String>,
    menu_entries: Vec<MenuEntry<PlaceCommand>>,
    menu_row: Option<Row>,
    _watch: Option<WatchGuard>,
}

impl Default for PlacesModelRust {
    fn default() -> Self {
        Self {
            count: 0,
            focus_row: -1,
            renaming_row: -1,
            rename_text: QString::default(),
            devices_note: QString::default(),
            menu: QString::default(),
            width: i32::try_from(prefs::settings().places().width).unwrap_or(240),
            started: false,
            places_path: xdg::data_dirs()
                .into_iter()
                .next()
                .map(|dir| dir.join("user-places.xbel")),
            groups_path: config::config_dir().map(|dir| dir.join("place-groups.toml")),
            places: Vec::new(),
            places_load: 0,
            places_notice: None,
            groups: Groups::default(),
            choice: prefs::settings().places(),
            rows: Vec::new(),
            focused: None,
            renaming: None,
            rename_requested: None,
            current: None,
            devices: Vec::new(),
            devices_unavailable: false,
            usage: HashMap::new(),
            device_service: Arc::new(NoDevices),
            device_busy: None,
            menu_entries: Vec::new(),
            menu_row: None,
            _watch: None,
        }
    }
}

impl PlacesModelRust {
    fn place(&self, id: &str) -> Option<&Place> {
        self.places.iter().find(|place| place.id == id)
    }

    fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// The panel's rows, top to bottom.
    fn compute_rows(&self) -> Vec<Row> {
        let mut rows = rows(
            &self.places,
            &self.groups,
            self.choice.show_hidden,
            self.groups.remote_folded,
            self.choice.remote,
        );
        if !self.choice.devices {
            return rows;
        }
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
        rows
    }

    fn row(&self, index: i32) -> Option<&Row> {
        self.rows.get(usize::try_from(index).ok()?)
    }
}

fn text(words: &str) -> QString {
    QString::from(words)
}

impl qobject::PlacesModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let (Some(row), Some(role)) = (rust.row(index.row()), Role::from_number(role)) else {
            return QVariant::default();
        };
        let words = |words: &str| QVariant::from(&QString::from(words));
        let focused = rust.focused.as_ref() == Some(row);
        let renaming = rust.renaming.as_ref() == Some(row);
        match (row, role) {
            (_, Role::Focused) => QVariant::from(&focused),
            (_, Role::Renaming) => QVariant::from(&renaming),
            (Row::Place(_), Role::Kind) => words("place"),
            (Row::Group(_), Role::Kind) => words("group"),
            (Row::Device(_), Role::Kind) => words("device"),
            (Row::Devices | Row::Remote, Role::Kind) => words("heading"),
            (Row::Place(id), _) => {
                let Some(place) = rust.place(id) else {
                    return QVariant::default();
                };
                match role {
                    Role::Title => words(&place.title),
                    Role::Icon => words(icon_for(place)),
                    Role::Current => QVariant::from(
                        &(place.path.is_some() && place.path.as_deref() == rust.current.as_deref()),
                    ),
                    Role::Hidden => QVariant::from(&place.hidden),
                    Role::Grouped => QVariant::from(&rust.groups.group_of(id).is_some()),
                    _ => QVariant::default(),
                }
            }
            (Row::Group(index), _) => {
                let Some(group) = rust.groups.groups.get(*index) else {
                    return QVariant::default();
                };
                match role {
                    Role::Title => words(if group.name.is_empty() {
                        "Group"
                    } else {
                        &group.name
                    }),
                    Role::Collapsed => QVariant::from(&group.collapsed),
                    Role::Detail => words(&group.places.len().to_string()),
                    _ => QVariant::default(),
                }
            }
            (Row::Devices, _) => match role {
                Role::Title => words(&if rust.groups.devices_folded {
                    format!("Devices ({})", rust.devices.len())
                } else {
                    "Devices".to_owned()
                }),
                Role::Collapsed => QVariant::from(&rust.groups.devices_folded),
                _ => QVariant::default(),
            },
            (Row::Remote, _) => match role {
                Role::Title => words(&if rust.groups.remote_folded {
                    let count = rust
                        .places
                        .iter()
                        .filter(|place| {
                            is_shown(place, rust.choice.show_hidden)
                                && is_remote(place)
                                && rust.groups.group_of(&place.id).is_none()
                        })
                        .count();
                    format!("Remote ({count})")
                } else {
                    "Remote".to_owned()
                }),
                Role::Collapsed => QVariant::from(&rust.groups.remote_folded),
                _ => QVariant::default(),
            },
            (Row::Device(id), _) => {
                let Some(device) = rust.device(id) else {
                    return QVariant::default();
                };
                let usage = rust.usage.get(id).copied();
                match role {
                    Role::Title => words(&device_name(device)),
                    Role::Icon => words(if device.removable {
                        "usb"
                    } else {
                        "hard-drive"
                    }),
                    Role::Current => {
                        QVariant::from(&rust.current.as_deref().is_some_and(|folder| {
                            device
                                .mount_point
                                .as_deref()
                                .is_some_and(|mount| mount == folder)
                        }))
                    }
                    Role::Detail => words(&usage.map_or_else(
                        || {
                            if device.mount_point.is_some() {
                                String::new()
                            } else {
                                "Not mounted".to_owned()
                            }
                        },
                        |(total, free)| usage_text(total, free),
                    )),
                    Role::Usage => QVariant::from(&usage.map_or(-1., |(total, free)| {
                        if total == 0 {
                            0.
                        } else {
                            (total - free.min(total)) as f64 / total as f64
                        }
                    })),
                    Role::Busy => QVariant::from(&(rust.device_busy.as_deref() == Some(id))),
                    Role::Ejectable => QVariant::from(&can_eject(device, xdg::home().as_deref())),
                    _ => QVariant::default(),
                }
            }
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Lays the rows out again and redraws them.
    fn refresh(mut self: Pin<&mut Self>) {
        let rows = self.rust().compute_rows();
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.count = i32::try_from(rows.len()).unwrap_or(i32::MAX);
            rust.rows = rows;
            if rust
                .focused
                .as_ref()
                .is_some_and(|focused| !rust.rows.contains(focused))
            {
                rust.focused = None;
            }
        }
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        let position = |row: &Option<Row>, rows: &[Row]| {
            row.as_ref()
                .and_then(|row| rows.iter().position(|shown| shown == row))
                .and_then(|at| i32::try_from(at).ok())
                .unwrap_or(-1)
        };
        let focus = position(&self.rust().focused, &self.rust().rows);
        let renaming = position(&self.rust().renaming, &self.rust().rows);
        self.as_mut().set_focus_row(focus);
        self.as_mut().set_renaming_row(renaming);
        let note = if self.rust().devices_unavailable && self.rust().choice.devices {
            DEVICES_UNAVAILABLE
        } else {
            ""
        };
        self.as_mut().set_devices_note(text(note));
        // A group made a moment ago is named straight away.
        if let Some(row) = self.as_mut().rust_mut().rename_requested.take() {
            self.begin_rename(row);
        }
    }

    fn start(mut self: Pin<&mut Self>) {
        if self.rust().started {
            return;
        }
        self.as_mut().rust_mut().started = true;
        // The places file's folder is watched: KDE replaces the file.
        let (changed, changes) = flume::unbounded::<()>();
        let watch = self
            .rust()
            .places_path
            .as_deref()
            .and_then(Path::parent)
            .and_then(|folder| {
                RealFs
                    .watch(
                        folder,
                        Box::new(move || {
                            let _ = changed.send(());
                        }),
                    )
                    .ok()
            });
        self.as_mut().rust_mut()._watch = watch;
        let watched = self.rust().places_path.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let signature = || {
                watched
                    .as_deref()
                    .and_then(|path| RealFs.info(path).ok())
                    .map(|info| (info.size, info.modified_ns, info.inode))
            };
            let mut seen = None;
            while changes.recv().is_ok() {
                std::thread::sleep(RELOAD_DELAY);
                while changes.try_recv().is_ok() {}
                let now = Some(signature());
                if now == seen {
                    continue;
                }
                seen = now;
                if thread.queue(|model| model.reload()).is_err() {
                    break;
                }
            }
        });
        self.as_mut().load_groups();
        self.as_mut().reload();
        self.connect_devices();
    }

    fn load_groups(self: Pin<&mut Self>) {
        let Some(path) = self.rust().groups_path.clone() else {
            return;
        };
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            if let Ok(Some(text)) = config::read(&path) {
                let _ = thread.queue(move |mut model| {
                    let groups = Groups::parse(&text);
                    if groups.is_from_newer_version() {
                        model.as_mut().rust_mut().groups_path = None;
                    }
                    model.as_mut().rust_mut().groups = groups;
                    model.refresh();
                });
            }
        });
    }

    fn save_groups(&self) {
        let Some(path) = self.rust().groups_path.clone() else {
            return;
        };
        let text = self.rust().groups.to_toml();
        std::thread::spawn(move || {
            if let Err(error) = config::write_atomic(&path, text.as_bytes()) {
                tracing::warn!(%error, path = %path.display(), "could not save the place groups");
            }
        });
    }

    fn reload(mut self: Pin<&mut Self>) {
        let Some(path) = self.rust().places_path.clone() else {
            return;
        };
        let load = self.rust().places_load.wrapping_add(1);
        self.as_mut().rust_mut().places_load = load;
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let read = config::read(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    Places::parse(&text.unwrap_or_else(|| EMPTY.to_owned()))
                        .map_err(|error| error.to_string())
                });
            let _ = thread.queue(move |mut model| {
                if model.rust().places_load != load {
                    return;
                }
                match read {
                    Ok(places) => {
                        model.as_mut().rust_mut().places_notice = None;
                        let places: Vec<Place> = places.places().cloned().collect();
                        if places != model.rust().places {
                            let forgot = {
                                let mut rust = model.as_mut().rust_mut();
                                rust.places = places;
                                let existing = rust.places.clone();
                                rust.groups.forget_missing(&existing)
                            };
                            if forgot {
                                model.save_groups();
                            }
                            model.refresh();
                        }
                    }
                    Err(error) => {
                        let message = format!(
                            "Could not update places: {error}. Your bookmarks have not been changed. Repair or restore user-places.xbel to reload them."
                        );
                        if model.rust().places_notice.as_ref() != Some(&message) {
                            tracing::warn!(%error, "the places file could not be read");
                            model.as_mut().rust_mut().places_notice = Some(message.clone());
                            hub::say(message, true);
                        }
                    }
                }
            });
        });
    }

    /// Changes the shared places file on another thread, then reads it again.
    fn edit_places<T: Send + 'static>(
        self: Pin<&mut Self>,
        edit: impl FnOnce(&Places) -> Result<(String, T), PlacesError> + Send + 'static,
        then: impl FnOnce(Pin<&mut Self>, T) + Send + 'static,
    ) {
        let Some(path) = self.rust().places_path.clone() else {
            return;
        };
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let written = (|| {
                let text = config::read(&path)
                    .map_err(|error| error.to_string())?
                    .unwrap_or_else(|| EMPTY.to_owned());
                let places = Places::parse(&text).map_err(|error| error.to_string())?;
                let (new_text, value) = edit(&places).map_err(|error| error.to_string())?;
                if new_text != text {
                    config::write_atomic(&path, new_text.as_bytes())
                        .map_err(|error| error.to_string())?;
                }
                Ok::<_, String>(value)
            })();
            let _ = thread.queue(move |mut model| match written {
                Ok(value) => {
                    then(model.as_mut(), value);
                    model.reload();
                }
                Err(error) => hub::say(format!("The places could not be changed: {error}."), true),
            });
        });
    }

    fn connect_devices(self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let (changed, changes) = flume::unbounded::<()>();
            let service = UDisks2::connect().and_then(|service| {
                service.watch(Box::new(move || {
                    let _ = changed.send(());
                }))?;
                Ok(service)
            });
            match service {
                Ok(service) => {
                    let service: Arc<dyn Devices> = Arc::new(service);
                    if thread
                        .queue(move |mut model| {
                            model.as_mut().rust_mut().device_service = service;
                            model.reload_devices();
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "no device service; volumes are not listed");
                    let _ = thread.queue(|model| model.devices_failed());
                    return;
                }
            }
            while changes.recv().is_ok() {
                std::thread::sleep(RELOAD_DELAY);
                while changes.try_recv().is_ok() {}
                if thread.queue(|model| model.reload_devices()).is_err() {
                    break;
                }
            }
        });
    }

    fn reload_devices(self: Pin<&mut Self>) {
        let service = self.rust().device_service.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let listed = service.list().map(|devices| {
                let usage: HashMap<String, (u64, u64)> = devices
                    .iter()
                    .filter_map(|device| {
                        let usage = devices::usage(device.mount_point.as_deref()?)?;
                        Some((device.id.clone(), usage))
                    })
                    .collect();
                (devices, usage)
            });
            let _ = thread.queue(move |mut model| match listed {
                Ok((devices, usage)) => {
                    let mut rust = model.as_mut().rust_mut();
                    rust.devices = devices;
                    rust.usage = usage;
                    rust.devices_unavailable = false;
                    model.refresh();
                }
                Err(error) => {
                    tracing::warn!(%error, "the volumes could not be listed");
                    model.devices_failed();
                }
            });
        });
    }

    fn devices_failed(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.devices_unavailable = true;
            rust.devices.clear();
            rust.usage.clear();
        }
        self.refresh();
    }

    fn set_current(mut self: Pin<&mut Self>, folder: &QString) {
        let folder =
            Some(PathBuf::from(folder.to_string())).filter(|path| !path.as_os_str().is_empty());
        if self.rust().current != folder {
            self.as_mut().rust_mut().current = folder;
            let last = *self.count() - 1;
            if last >= 0 {
                let first = self.index(0, 0, &QModelIndex::default());
                let last = self.index(last, 0, &QModelIndex::default());
                let roles: cxx_qt_lib::QList<i32> = std::iter::once(Role::Current as i32).collect();
                self.as_mut().data_changed(&first, &last, &roles);
            }
        }
    }

    fn resize(mut self: Pin<&mut Self>, width: f64) {
        let width = width.max(0.) as u32;
        prefs::update(|settings| {
            let choice = settings.places();
            settings.set_places(PlacesChoice { width, ..choice });
        });
        let choice = prefs::settings().places();
        self.as_mut().rust_mut().choice = choice;
        self.as_mut()
            .set_width(i32::try_from(choice.width).unwrap_or(240));
    }

    fn adopt_settings(mut self: Pin<&mut Self>) {
        let choice = prefs::settings().places();
        if self.rust().choice != choice {
            self.as_mut().rust_mut().choice = choice;
            self.as_mut()
                .set_width(i32::try_from(choice.width).unwrap_or(240));
            self.refresh();
        }
    }

    fn focus_on(mut self: Pin<&mut Self>, row: i32) {
        let target = self.rust().row(row).cloned();
        self.as_mut().rust_mut().focused = target;
        self.focus_changed();
    }

    /// Redraws which row has the keyboard, without laying rows out again
    /// (a reset would drop a press in progress).
    fn focus_changed(mut self: Pin<&mut Self>) {
        let at = self
            .rust()
            .focused
            .as_ref()
            .and_then(|focused| self.rust().rows.iter().position(|row| row == focused))
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        self.as_mut().set_focus_row(at);
        let last = *self.count() - 1;
        if last >= 0 {
            let first = self.index(0, 0, &QModelIndex::default());
            let last = self.index(last, 0, &QModelIndex::default());
            let roles: cxx_qt_lib::QList<i32> = std::iter::once(Role::Focused as i32).collect();
            self.as_mut().data_changed(&first, &last, &roles);
        }
    }

    fn move_focus(mut self: Pin<&mut Self>, delta: i32) {
        let rows = &self.rust().rows;
        if rows.is_empty() {
            return;
        }
        let at = self
            .rust()
            .focused
            .as_ref()
            .and_then(|focused| rows.iter().position(|row| row == focused));
        let target = match at {
            Some(at) => at.saturating_add_signed(delta as isize).min(rows.len() - 1),
            None if delta < 0 => rows.len() - 1,
            None => 0,
        };
        let row = rows[target].clone();
        self.as_mut().rust_mut().focused = Some(row);
        self.focus_changed();
    }

    fn activate(mut self: Pin<&mut Self>, row: i32, new_tab: bool) {
        let Some(row) = self.rust().row(row).cloned() else {
            return;
        };
        self.as_mut().rust_mut().focused = Some(row.clone());
        self.as_mut().focus_changed();
        self.open_row(&row, new_tab);
    }

    fn open_row(mut self: Pin<&mut Self>, row: &Row, new_tab: bool) {
        match row {
            Row::Place(id) => {
                let Some(place) = self.rust().place(id).cloned() else {
                    return;
                };
                if let Some(path) = place.path.clone() {
                    self.as_mut()
                        .open_folder(QString::from(path.to_string_lossy().as_ref()), new_tab);
                } else if place.href.starts_with("trash:") {
                    self.as_mut().open_trash();
                } else if let Some(address) = network_address(&place) {
                    self.open_remote(address, new_tab);
                } else if let Some(name) = network_folder_name(&place.href) {
                    let thread = self.qt_thread();
                    std::thread::spawn(move || {
                        let address =
                            xdg::data_home().and_then(|home| network_folder_address(&home, &name));
                        let _ = thread.queue(move |model| match address {
                            Some(address) => model.open_remote(address, new_tab),
                            None => hub::say(
                                format!(
                                    "The network folder \u{201c}{name}\u{201d} has no address Rustphin can open. Check it in Dolphin's Network view, or add its sftp:// or smb:// address to Places."
                                ),
                                true,
                            ),
                        });
                    });
                }
            }
            Row::Group(index) => {
                let collapsed = self
                    .rust()
                    .groups
                    .groups
                    .get(*index)
                    .is_some_and(|group| group.collapsed);
                self.as_mut()
                    .rust_mut()
                    .groups
                    .set_collapsed(*index, !collapsed);
                self.save_groups();
                self.refresh();
            }
            Row::Device(id) => self.open_device(id.clone(), new_tab),
            Row::Devices => {
                let folded = self.rust().groups.devices_folded;
                self.as_mut().rust_mut().groups.devices_folded = !folded;
                self.save_groups();
                self.refresh();
            }
            Row::Remote => {
                let folded = self.rust().groups.remote_folded;
                self.as_mut().rust_mut().groups.remote_folded = !folded;
                self.save_groups();
                self.refresh();
            }
        }
    }

    fn open_remote(
        self: Pin<&mut Self>,
        address: rustphin_platform::remote::RemoteAddress,
        new_tab: bool,
    ) {
        let thread = self.qt_thread();
        crate::remote::open(address, move |mounted| {
            if let Some(path) = mounted {
                let _ = thread.queue(move |model| {
                    model.open_folder(QString::from(path.to_string_lossy().as_ref()), new_tab);
                });
            }
        });
    }

    fn open_device(mut self: Pin<&mut Self>, id: String, new_tab: bool) {
        let Some(device) = self.rust().device(&id).cloned() else {
            return;
        };
        if let Some(path) = device.mount_point.clone() {
            return self.open_folder(QString::from(path.to_string_lossy().as_ref()), new_tab);
        }
        let service = self.rust().device_service.clone();
        self.as_mut().rust_mut().device_busy = Some(device.id.clone());
        self.as_mut().refresh();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let name = device_name(&device);
            let mounted = service.mount(&device);
            let _ = thread.queue(move |mut model| {
                model.as_mut().rust_mut().device_busy = None;
                match mounted {
                    Ok(path) => model
                        .as_mut()
                        .open_folder(QString::from(path.to_string_lossy().as_ref()), new_tab),
                    Err(error) => hub::say(
                        format!("\u{201c}{name}\u{201d} could not be mounted: {error}."),
                        true,
                    ),
                }
                model.reload_devices();
            });
        });
    }

    fn eject(mut self: Pin<&mut Self>, row: i32) {
        let Some(Row::Device(id)) = self.rust().row(row).cloned() else {
            return;
        };
        let Some(device) = self.rust().device(&id).cloned() else {
            return;
        };
        if !can_eject(&device, xdg::home().as_deref()) {
            return;
        }
        let service = self.rust().device_service.clone();
        self.as_mut().rust_mut().device_busy = Some(device.id.clone());
        self.as_mut().refresh();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let name = device_name(&device);
            let done = (|| {
                if device.mount_point.is_some() {
                    service.unmount(&device)?;
                }
                if device.removable && device.can_power_off() {
                    service.power_off(&device)?;
                    return Ok(true);
                }
                Ok::<_, devices::DeviceError>(false)
            })();
            let (words, is_error) = match done {
                Ok(true) => (
                    format!("\u{201c}{name}\u{201d} can be unplugged now."),
                    false,
                ),
                Ok(false) => (format!("\u{201c}{name}\u{201d} was unmounted."), false),
                Err(error) => (
                    format!("\u{201c}{name}\u{201d} could not be removed: {error}."),
                    true,
                ),
            };
            hub::say(words, is_error);
            let _ = thread.queue(|mut model| {
                model.as_mut().rust_mut().device_busy = None;
                model.reload_devices();
            });
        });
    }

    fn begin_rename(mut self: Pin<&mut Self>, row: Row) {
        let name = match &row {
            Row::Place(id) => self.rust().place(id).map(|place| place.title.clone()),
            Row::Group(index) => self
                .rust()
                .groups
                .groups
                .get(*index)
                .map(|g| g.name.clone()),
            Row::Device(_) | Row::Devices | Row::Remote => None,
        };
        let Some(name) = name else {
            return;
        };
        let at = self
            .rust()
            .rows
            .iter()
            .position(|shown| *shown == row)
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        self.as_mut().rust_mut().renaming = Some(row);
        self.as_mut().set_rename_text(text(&name));
        self.as_mut().set_renaming_row(at);
    }

    fn finish_rename(mut self: Pin<&mut Self>, typed: &QString, apply: bool) {
        let Some(row) = self.as_mut().rust_mut().renaming.take() else {
            return;
        };
        self.as_mut().set_renaming_row(-1);
        let typed = typed.to_string().trim().to_owned();
        if apply && !typed.is_empty() {
            match row {
                Row::Place(id) => {
                    if self
                        .rust()
                        .place(&id)
                        .is_some_and(|place| place.title != typed)
                    {
                        self.as_mut().edit_places(
                            move |places| Ok((places.with_title(&id, &typed)?, ())),
                            |_, ()| {},
                        );
                    }
                }
                Row::Group(index) => {
                    self.as_mut().rust_mut().groups.rename(index, &typed);
                    self.save_groups();
                }
                Row::Device(_) | Row::Devices | Row::Remote => {}
            }
        }
        self.refresh();
    }

    fn set_hidden(self: Pin<&mut Self>, id: String, hidden: bool) {
        self.edit_places(
            move |places| Ok((places.with_hidden(&id, hidden)?, ())),
            |_, ()| {},
        );
    }

    fn remove_focused(mut self: Pin<&mut Self>) {
        match self.rust().focused.clone() {
            Some(Row::Place(id)) => {
                let Some(place) = self.rust().place(&id).cloned() else {
                    return;
                };
                if place.system {
                    return self.set_hidden(id, true);
                }
                self.as_mut().move_focus(1);
                self.edit_places(move |places| Ok((places.without(&id)?, ())), |_, ()| {});
            }
            Some(Row::Group(index)) => {
                self.as_mut().rust_mut().groups.remove(index);
                self.as_mut().rust_mut().focused = None;
                self.save_groups();
                self.refresh();
            }
            Some(Row::Device(_) | Row::Devices | Row::Remote) | None => {}
        }
    }

    fn shift_focused(mut self: Pin<&mut Self>, down: bool) {
        match self.rust().focused.clone() {
            Some(Row::Group(index)) => {
                let target = self
                    .as_mut()
                    .rust_mut()
                    .groups
                    .shift(index, if down { 1 } else { -1 });
                self.as_mut().rust_mut().focused = Some(Row::Group(target));
                self.save_groups();
                self.refresh();
            }
            Some(Row::Place(id)) if self.rust().groups.group_of(&id).is_some() => {
                if self.as_mut().rust_mut().groups.shift_place(&id, down) {
                    self.save_groups();
                    self.refresh();
                }
            }
            Some(Row::Place(id)) => {
                let show_hidden = self.rust().choice.show_hidden;
                let shown: Vec<String> = {
                    let rust = self.rust();
                    let layout = rust.groups.layout(&rust.places);
                    layout
                        .ungrouped
                        .iter()
                        .filter(|place| is_shown(place, show_hidden) && !is_remote(place))
                        .map(|place| place.id.clone())
                        .collect()
                };
                let Some(at) = shown.iter().position(|shown| *shown == id) else {
                    return;
                };
                let before = if down {
                    match shown.get(at + 1) {
                        Some(_) => shown.get(at + 2).cloned(),
                        None => return,
                    }
                } else {
                    match at.checked_sub(1) {
                        Some(previous) => Some(shown[previous].clone()),
                        None => return,
                    }
                };
                self.edit_places(
                    move |places| Ok((places.with_moved(&id, before.as_deref())?, ())),
                    |_, ()| {},
                );
            }
            Some(Row::Device(_) | Row::Devices | Row::Remote) | None => {}
        }
    }

    fn move_to_group(
        mut self: Pin<&mut Self>,
        id: &str,
        group: Option<usize>,
        before: Option<&str>,
    ) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.groups.assign(id, group, before);
            if let Some(index) = group {
                rust.groups.set_collapsed(index, false);
            }
        }
        self.save_groups();
        self.refresh();
    }

    fn cycle_group(self: Pin<&mut Self>) {
        let Some(Row::Place(id)) = self.rust().focused.clone() else {
            return;
        };
        let groups = self.rust().groups.groups.len();
        let next = match self.rust().groups.group_of(&id) {
            None if groups == 0 => return,
            None => Some(0),
            Some(index) if index + 1 < groups => Some(index + 1),
            Some(_) => None,
        };
        self.move_to_group(&id, next, None);
    }

    fn new_group(mut self: Pin<&mut Self>) -> usize {
        let index = self.as_mut().rust_mut().groups.add("");
        {
            let mut rust = self.as_mut().rust_mut();
            rust.focused = Some(Row::Group(index));
            rust.rename_requested = Some(Row::Group(index));
        }
        self.save_groups();
        self.refresh();
        index
    }

    fn set_show_hidden(mut self: Pin<&mut Self>, show: bool) {
        prefs::update(|settings| {
            let choice = settings.places();
            settings.set_places(PlacesChoice {
                show_hidden: show,
                ..choice
            });
        });
        crate::preferences::changed();
        self.as_mut().rust_mut().choice.show_hidden = show;
        self.refresh();
    }

    fn key(mut self: Pin<&mut Self>, key: &QString) -> bool {
        let focused = self.rust().focused.clone();
        match key.to_string().as_str() {
            "Up" => self.move_focus(-1),
            "Down" => self.move_focus(1),
            "Enter" => match focused {
                Some(row) => self.open_row(&row, false),
                None => return false,
            },
            "F2" => match focused {
                Some(row) => self.begin_rename(row),
                None => return false,
            },
            "Delete" => self.remove_focused(),
            "H" => {
                if let Some(Row::Place(id)) = focused
                    && let Some(hidden) = self.rust().place(&id).map(|place| place.hidden)
                {
                    self.set_hidden(id, !hidden);
                }
            }
            "Alt+Up" => self.shift_focused(false),
            "Alt+Down" => self.shift_focused(true),
            "Left" | "Right" => {
                let collapse = key.to_string() == "Left";
                match focused {
                    Some(Row::Group(index)) => {
                        self.as_mut()
                            .rust_mut()
                            .groups
                            .set_collapsed(index, collapse);
                        self.save_groups();
                        self.refresh();
                    }
                    Some(Row::Devices) => {
                        self.as_mut().rust_mut().groups.devices_folded = collapse;
                        self.save_groups();
                        self.refresh();
                    }
                    Some(Row::Remote) => {
                        self.as_mut().rust_mut().groups.remote_folded = collapse;
                        self.save_groups();
                        self.refresh();
                    }
                    _ => return false,
                }
            }
            "G" => self.cycle_group(),
            "Ctrl+Shift+G" => {
                self.new_group();
            }
            "E" => {
                let at = *self.focus_row();
                self.eject(at);
            }
            "ToggleHiddenPlaces" => {
                let show = !self.rust().choice.show_hidden;
                self.set_show_hidden(show);
            }
            _ => return false,
        }
        true
    }

    fn open_menu(mut self: Pin<&mut Self>, row: i32) -> bool {
        if self.rust().renaming.is_some() {
            return false;
        }
        let row = self.rust().row(row).cloned();
        let entries = {
            let rust = self.rust();
            let names: Vec<String> = rust.groups.groups.iter().map(|g| g.name.clone()).collect();
            match &row {
                Some(Row::Place(id)) => rust.place(id).map(|place| {
                    place_menu::place_menu(Target::Place {
                        system: place.system,
                        hidden: place.hidden,
                        group: rust.groups.group_of(id),
                        groups: &names,
                    })
                }),
                Some(Row::Group(index)) => rust.groups.groups.get(*index).map(|group| {
                    place_menu::place_menu(Target::Group {
                        collapsed: group.collapsed,
                    })
                }),
                Some(Row::Device(_) | Row::Devices | Row::Remote) => None,
                None => Some(place_menu::place_menu(Target::Panel {
                    can_add_shown: rust.current.is_some(),
                    showing_hidden: rust.choice.show_hidden,
                })),
            }
        };
        let Some(entries) = entries.filter(|entries| !entries.is_empty()) else {
            return false;
        };
        if row.is_some() {
            self.as_mut().rust_mut().focused = row.clone();
        }
        let json = menu::to_json(&entries).to_string();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.menu_entries = entries;
            rust.menu_row = row;
        }
        self.as_mut().set_menu(text(&json));
        self.refresh();
        true
    }

    fn run_menu(mut self: Pin<&mut Self>, path: &QString) {
        let command = menu::command_at(&self.rust().menu_entries, &path.to_string()).cloned();
        let row = self.as_mut().rust_mut().menu_row.take();
        self.as_mut().rust_mut().menu_entries.clear();
        self.as_mut().set_menu(QString::default());
        let Some(command) = command else {
            return;
        };
        let place = match &row {
            Some(Row::Place(id)) => Some(id.clone()),
            _ => None,
        };
        match command {
            PlaceCommand::Open | PlaceCommand::OpenInNewTab => {
                if let Some(row) = &row {
                    self.open_row(row, command == PlaceCommand::OpenInNewTab);
                }
            }
            PlaceCommand::MoveToGroup(index) => {
                if let Some(id) = place {
                    self.move_to_group(&id, Some(index), None);
                }
            }
            PlaceCommand::NewGroupWith => {
                if let Some(id) = place {
                    let index = self.as_mut().new_group();
                    self.as_mut()
                        .rust_mut()
                        .groups
                        .assign(&id, Some(index), None);
                    self.save_groups();
                    self.refresh();
                }
            }
            PlaceCommand::LeaveGroup => {
                if let Some(id) = place {
                    self.move_to_group(&id, None, None);
                }
            }
            PlaceCommand::MoveUp | PlaceCommand::MoveDown => {
                self.shift_focused(command == PlaceCommand::MoveDown);
            }
            PlaceCommand::Rename => {
                if let Some(row) = row {
                    self.begin_rename(row);
                }
            }
            PlaceCommand::Hide | PlaceCommand::Show => {
                if let Some(id) = place {
                    self.set_hidden(id, command == PlaceCommand::Hide);
                }
            }
            PlaceCommand::Remove | PlaceCommand::DeleteGroup => self.remove_focused(),
            PlaceCommand::Collapse | PlaceCommand::Expand => {
                if let Some(Row::Group(index)) = row {
                    self.as_mut()
                        .rust_mut()
                        .groups
                        .set_collapsed(index, command == PlaceCommand::Collapse);
                    self.save_groups();
                    self.refresh();
                }
            }
            PlaceCommand::NewGroup => {
                self.new_group();
            }
            PlaceCommand::AddShownFolder => {
                if let Some(folder) = self.rust().current.clone() {
                    self.add_place(folder, None);
                }
            }
            PlaceCommand::ToggleHiddenPlaces => {
                let show = !self.rust().choice.show_hidden;
                self.set_show_hidden(show);
            }
        }
    }

    fn add_folder(self: Pin<&mut Self>, folder: &QString) {
        let folder = PathBuf::from(folder.to_string());
        if !folder.as_os_str().is_empty() {
            self.add_place(folder, None);
        }
    }

    fn add_place(self: Pin<&mut Self>, folder: PathBuf, group: Option<usize>) {
        if self
            .rust()
            .places
            .iter()
            .any(|place| place.path.as_deref() == Some(&folder))
        {
            return hub::say(
                format!(
                    "\u{201c}{}\u{201d} is already in the places.",
                    folder.display()
                ),
                false,
            );
        }
        let title = match folder.file_name() {
            Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
            None => "/".to_owned(),
        };
        self.edit_places(
            move |places| places.with_added(&title, &folder, "folder", now()),
            move |mut model, id: String| {
                if group.is_some() {
                    model.as_mut().rust_mut().groups.assign(&id, group, None);
                    model.save_groups();
                }
                model.as_mut().rust_mut().focused = Some(Row::Place(id));
            },
        );
    }

    fn drop_row(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let (Some(from), Some(target)) =
            (self.rust().row(from).cloned(), self.rust().row(to).cloned())
        else {
            return;
        };
        match from {
            Row::Place(id) => match target {
                Row::Group(index) => self.move_to_group(&id, Some(index), None),
                Row::Device(_) | Row::Devices | Row::Remote => {}
                Row::Place(other) if other == id => {}
                Row::Place(other) => match self.rust().groups.group_of(&other) {
                    Some(index) => self.move_to_group(&id, Some(index), Some(&other)),
                    None => {
                        self.as_mut().move_to_group(&id, None, None);
                        self.edit_places(
                            move |places| Ok((places.with_moved(&id, Some(&other))?, ())),
                            |_, ()| {},
                        );
                    }
                },
            },
            Row::Group(from) => {
                let to = match &target {
                    Row::Group(index) => Some(*index),
                    Row::Place(id) => self.rust().groups.group_of(id),
                    Row::Device(_) | Row::Devices | Row::Remote => None,
                };
                let Some(to) = to.filter(|to| *to != from) else {
                    return;
                };
                let at = self.as_mut().rust_mut().groups.move_group(from, to);
                self.as_mut().rust_mut().focused = Some(Row::Group(at));
                self.save_groups();
                self.refresh();
            }
            Row::Device(_) | Row::Devices | Row::Remote => {}
        }
    }

    fn drop_folders(mut self: Pin<&mut Self>, uris: &QString, to: i32) {
        let group = match self.rust().row(to) {
            Some(Row::Group(index)) => Some(*index),
            Some(Row::Place(id)) => self.rust().groups.group_of(id),
            _ => None,
        };
        let folders = rustphin_platform::uri::paths_from_uri_list(uris.to_string().as_bytes());
        for folder in folders.into_iter().filter(|path| path.is_dir()) {
            self.as_mut().add_place(folder, group);
        }
    }

    fn palette_json(&self) -> QString {
        let places: Vec<serde_json::Value> = self
            .rust()
            .places
            .iter()
            .filter(|place| is_shown(place, false))
            .filter_map(|place| {
                Some(serde_json::json!({
                    "title": place.title,
                    "path": place.path.as_ref()?.to_string_lossy(),
                    "icon": icon_for(place),
                }))
            })
            .collect();
        text(&serde_json::Value::Array(places).to_string())
    }

    fn icon_for(&self, folder: &QString) -> QString {
        let folder = PathBuf::from(folder.to_string());
        text(
            self.rust()
                .places
                .iter()
                .find(|place| place.path.as_deref() == Some(folder.as_path()))
                .map_or("", icon_for),
        )
    }

    fn groups_json(&self) -> QString {
        let groups: Vec<serde_json::Value> = self
            .rust()
            .groups
            .groups
            .iter()
            .map(|group| serde_json::json!({ "name": group.name, "count": group.places.len() }))
            .collect();
        text(&serde_json::Value::Array(groups).to_string())
    }

    fn shift_group(mut self: Pin<&mut Self>, index: i32, delta: i32) {
        if let Ok(index) = usize::try_from(index) {
            self.as_mut().rust_mut().groups.shift(index, delta as isize);
            self.save_groups();
            self.refresh();
        }
    }

    fn delete_group(mut self: Pin<&mut Self>, index: i32) {
        if let Ok(index) = usize::try_from(index) {
            self.as_mut().rust_mut().groups.remove(index);
            self.save_groups();
            self.refresh();
        }
    }

    fn rename_group(mut self: Pin<&mut Self>, index: i32, name: &QString) {
        if let Ok(index) = usize::try_from(index) {
            self.as_mut()
                .rust_mut()
                .groups
                .rename(index, &name.to_string());
            self.save_groups();
            self.refresh();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_found_by_number_and_named_once() {
        for role in Role::ALL {
            assert_eq!(Role::from_number(role as i32), Some(role));
        }
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Role::ALL.len());
    }
}
