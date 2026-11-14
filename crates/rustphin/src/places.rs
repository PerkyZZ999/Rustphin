//! The places panel as a QML list model (W-114e): KDE's shared places
//! (`user-places.xbel`) with Rustphin's own groups (D-008, W-105), the
//! Remote section, and the storage volumes from UDisks2 with their usage.
//! Files are read and written on other threads; the rows, what they show
//! and what every key, menu entry, drop and rename does are decided by
//! `rustphin_view::places_panel::PlacesPanel` (W-127), whose effects this
//! adapter carries out.

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
use rustphin_core::{Fs as _, RealFs, WatchGuard};
use rustphin_platform::{
    config,
    devices::{self, Device, Devices, NoDevices, UDisks2},
    places::{EMPTY, Place, Places},
    remote::network_folder_address,
    xdg,
};
use rustphin_view::{
    commands::{self, Action},
    groups::GroupsFile,
    places_panel::{
        Effect, Opening, PlacesEdit, PlacesPanel, Row, device_name, icon_for, is_shown,
    },
    saved_file::SavedFile,
    settings::PlacesChoice,
};

use crate::{hub, prefs, qml_path};

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
    has_places: bool,
    focus_row: i32,
    renaming_row: i32,
    rename_text: QString,
    devices_note: QString,
    menu: QString,
    width: i32,

    started: bool,
    places_path: Option<PathBuf>,
    groups_path: Option<PathBuf>,
    groups_file: GroupsFile,
    places_load: u64,
    places_notice: Option<String>,
    panel: PlacesPanel,
    /// The rows the view was last told of; the panel's rows once laid out.
    shown: Vec<Row>,
    device_service: Arc<dyn Devices>,
    _watch: Option<WatchGuard>,
}

impl Default for PlacesModelRust {
    fn default() -> Self {
        let choice = prefs::settings().places();
        Self {
            count: 0,
            has_places: false,
            focus_row: -1,
            renaming_row: -1,
            rename_text: QString::default(),
            devices_note: QString::default(),
            menu: QString::default(),
            width: i32::try_from(choice.width).unwrap_or(240),
            started: false,
            places_path: xdg::data_dirs()
                .into_iter()
                .next()
                .map(|dir| dir.join("user-places.xbel")),
            groups_path: config::config_dir().map(|dir| dir.join("place-groups.toml")),
            groups_file: GroupsFile::default(),
            places_load: 0,
            places_notice: None,
            panel: PlacesPanel::new(choice, xdg::home()),
            shown: Vec::new(),
            device_service: Arc::new(NoDevices),
            _watch: None,
        }
    }
}

impl PlacesModelRust {
    /// Where `row` is shown, or -1.
    fn position(&self, row: Option<&Row>) -> i32 {
        row.and_then(|row| self.shown.iter().position(|shown| shown == row))
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1)
    }
}

fn text(words: &str) -> QString {
    QString::from(words)
}

/// The places the palette offers, as JSON: each shown folder's title,
/// `file://` URI (which keeps every byte) and icon.
pub fn palette_entries(places: &[Place]) -> serde_json::Value {
    places
        .iter()
        .filter(|place| is_shown(place, false))
        .filter_map(|place| {
            Some(serde_json::json!({
                "title": place.title,
                "uri": rustphin_platform::uri::file_uri(place.path.as_ref()?),
                "icon": icon_for(place),
            }))
        })
        .collect()
}

/// A row number from QML; none when negative.
fn at(row: i32) -> Option<usize> {
    usize::try_from(row).ok()
}

impl qobject::PlacesModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let view = at(index.row())
            .and_then(|at| rust.shown.get(at))
            .and_then(|row| rust.panel.view(row));
        let (Some(view), Some(role)) = (view, Role::from_number(role)) else {
            return QVariant::default();
        };
        let words = |words: &str| QVariant::from(&QString::from(words));
        match role {
            Role::Kind => words(view.kind.name()),
            Role::Title => words(&view.title),
            Role::Icon => words(view.icon),
            Role::Current => QVariant::from(&view.current),
            Role::Hidden => QVariant::from(&view.hidden),
            Role::Collapsed => QVariant::from(&view.collapsed),
            Role::Detail => words(&view.detail),
            Role::Usage => QVariant::from(&view.usage),
            Role::Busy => QVariant::from(&view.busy),
            Role::Ejectable => QVariant::from(&view.ejectable),
            Role::Focused => QVariant::from(&view.focused),
            Role::Renaming => QVariant::from(&view.renaming),
            Role::Grouped => QVariant::from(&view.grouped),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Carries out what the panel decided, in order.
    fn apply(mut self: Pin<&mut Self>, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Relayout => self.as_mut().relayout(),
                Effect::Focus => self.as_mut().focus_changed(),
                Effect::Rename(name) => self.as_mut().rename_opened(&name),
                Effect::Menu(json) => self.as_mut().set_menu(text(&json)),
                Effect::SaveGroups => self.save_groups(),
                Effect::EditPlaces(edit) => self.as_mut().edit_places(edit),
                Effect::Open { opening, new_tab } => self.as_mut().open(opening, new_tab),
                Effect::Eject(device) => self.as_mut().eject_device(device),
                Effect::ShowHidden(show) => {
                    prefs::update(|settings| {
                        let choice = settings.places();
                        settings.set_places(PlacesChoice {
                            show_hidden: show,
                            ..choice
                        });
                    });
                    crate::preferences::changed();
                }
                Effect::Say(words) => hub::say(words, false),
            }
        }
    }

    /// Lays the rows out again, or redraws them where they are.
    fn relayout(mut self: Pin<&mut Self>) {
        let rows = self.rust().panel.rows().to_vec();
        // The same rows (a write of our own came back through the watch):
        // redraw them in place. A reset would rebuild the delegates and
        // end a rename in progress, as if its field had lost focus.
        if rows == self.rust().shown {
            self.as_mut().redraw_all();
            return self.after_layout();
        }
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.count = i32::try_from(rows.len()).unwrap_or(i32::MAX);
            rust.shown = rows;
        }
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        self.after_layout();
    }

    /// Every row's data changed, its position did not.
    fn redraw_all(mut self: Pin<&mut Self>) {
        self.as_mut().redraw_role(None);
    }

    /// One role of every row changed (all roles with `None`).
    fn redraw_role(mut self: Pin<&mut Self>, role: Option<Role>) {
        let last = *self.count() - 1;
        if last >= 0 {
            let first = self.index(0, 0, &QModelIndex::default());
            let last = self.index(last, 0, &QModelIndex::default());
            let roles: cxx_qt_lib::QList<i32> = role.map(|role| role as i32).into_iter().collect();
            self.as_mut().data_changed(&first, &last, &roles);
        }
    }

    /// What follows laying the rows out: the focused and renamed rows, the
    /// notes, and a group just made opening its name field.
    fn after_layout(mut self: Pin<&mut Self>) {
        let rust = self.rust();
        let has_places = rust.panel.has_places();
        let focus = rust.position(rust.panel.focused());
        let renaming = rust.position(rust.panel.renaming());
        let note = rust.panel.devices_note();
        self.as_mut().set_has_places(has_places);
        self.as_mut().set_focus_row(focus);
        self.as_mut().set_renaming_row(renaming);
        self.as_mut().set_devices_note(text(note));
        // A group made a moment ago is named straight away.
        let effects = self.as_mut().rust_mut().panel.laid_out();
        self.apply(effects);
    }

    /// Redraws which row has the keyboard, without laying rows out again
    /// (a reset would drop a press in progress).
    fn focus_changed(mut self: Pin<&mut Self>) {
        let at = self.rust().position(self.rust().panel.focused());
        self.as_mut().set_focus_row(at);
        self.redraw_role(Some(Role::Focused));
    }

    /// The name field opens on the panel's renamed row with `name` in it.
    fn rename_opened(mut self: Pin<&mut Self>, name: &str) {
        let at = self.rust().position(self.rust().panel.renaming());
        self.as_mut().set_rename_text(text(name));
        self.as_mut().set_renaming_row(at);
        if at >= 0 && at < *self.count() {
            let index = self.index(at, 0, &QModelIndex::default());
            let roles: cxx_qt_lib::QList<i32> = std::iter::once(Role::Renaming as i32).collect();
            self.as_mut().data_changed(&index, &index, &roles);
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
            let read = config::read(&path);
            let _ = thread.queue(move |mut model| {
                // Changes are not written until the file has been read, nor
                // over a file that could not be used (W-140).
                let (groups, issue) = model.as_mut().rust_mut().groups_file.loaded(read);
                if let Some(issue) = issue {
                    tracing::warn!(%issue, "the place groups file is kept for this run");
                    hub::say(issue.notice(SavedFile::Groups), true);
                }
                if let Some(groups) = groups {
                    let effects = model.as_mut().rust_mut().panel.set_groups(groups);
                    model.apply(effects);
                }
            });
        });
    }

    fn save_groups(&self) {
        if !self.rust().groups_file.may_save() {
            return;
        }
        let Some(path) = self.rust().groups_path.clone() else {
            return;
        };
        let text = self.rust().panel.groups().to_toml();
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
                        let effects = model.as_mut().rust_mut().panel.set_places(places);
                        model.apply(effects);
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

    /// Changes the shared places file on another thread, tells the panel,
    /// then reads the file again.
    fn edit_places(self: Pin<&mut Self>, edit: PlacesEdit) {
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
                let (new_text, added) = edit
                    .apply(&places, now())
                    .map_err(|error| error.to_string())?;
                if new_text != text {
                    config::write_atomic(&path, new_text.as_bytes())
                        .map_err(|error| error.to_string())?;
                }
                Ok::<_, String>(added)
            })();
            let _ = thread.queue(move |mut model| match written {
                Ok(added) => {
                    let effects = model.as_mut().rust_mut().panel.edited(&edit, added);
                    model.as_mut().apply(effects);
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
                    let effects = model.as_mut().rust_mut().panel.set_devices(devices, usage);
                    model.apply(effects);
                }
                Err(error) => {
                    tracing::warn!(%error, "the volumes could not be listed");
                    model.devices_failed();
                }
            });
        });
    }

    fn devices_failed(mut self: Pin<&mut Self>) {
        let effects = self.as_mut().rust_mut().panel.devices_failed();
        self.apply(effects);
    }

    /// The volume is no longer being mounted or ejected.
    fn not_busy(mut self: Pin<&mut Self>) {
        let effects = self.as_mut().rust_mut().panel.set_busy(None);
        self.as_mut().apply(effects);
        self.reload_devices();
    }

    fn set_current(mut self: Pin<&mut Self>, uri: &QString) {
        let folder = qml_path::from_qml(uri);
        if self.as_mut().rust_mut().panel.set_current(folder) {
            self.redraw_role(Some(Role::Current));
        }
    }

    fn resize(mut self: Pin<&mut Self>, width: f64) {
        let width = width.max(0.) as u32;
        prefs::update(|settings| {
            let choice = settings.places();
            settings.set_places(PlacesChoice { width, ..choice });
        });
        let choice = prefs::settings().places();
        let effects = self.as_mut().rust_mut().panel.set_choice(choice);
        self.as_mut()
            .set_width(i32::try_from(choice.width).unwrap_or(240));
        self.apply(effects);
    }

    fn adopt_settings(mut self: Pin<&mut Self>) {
        let choice = prefs::settings().places();
        let effects = self.as_mut().rust_mut().panel.set_choice(choice);
        self.as_mut()
            .set_width(i32::try_from(choice.width).unwrap_or(240));
        self.apply(effects);
    }

    fn focus_on(mut self: Pin<&mut Self>, row: i32) {
        let effects = self.as_mut().rust_mut().panel.focus_on(at(row));
        self.apply(effects);
    }

    fn move_focus(mut self: Pin<&mut Self>, delta: i32) {
        let effects = self.as_mut().rust_mut().panel.move_focus(delta as isize);
        self.apply(effects);
    }

    fn activate(mut self: Pin<&mut Self>, row: i32, new_tab: bool) {
        let Some(row) = at(row) else {
            return;
        };
        let effects = self.as_mut().rust_mut().panel.activate(row, new_tab);
        self.apply(effects);
    }

    fn open(self: Pin<&mut Self>, opening: Opening, new_tab: bool) {
        match opening {
            Opening::Folder(path) => self.open_folder(qml_path::to_qml(&path), new_tab),
            Opening::Trash => self.open_trash(),
            Opening::Remote(address) => self.open_remote(address, new_tab),
            Opening::NetworkFolder(name) => {
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
            Opening::Mount(device) => {
                let service = self.rust().device_service.clone();
                let thread = self.qt_thread();
                std::thread::spawn(move || {
                    let name = device_name(&device);
                    let mounted = service.mount(&device);
                    let _ = thread.queue(move |mut model| {
                        match mounted {
                            Ok(path) => {
                                model.as_mut().open_folder(qml_path::to_qml(&path), new_tab)
                            }
                            Err(error) => hub::say(
                                format!("\u{201c}{name}\u{201d} could not be mounted: {error}."),
                                true,
                            ),
                        }
                        model.not_busy();
                    });
                });
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
                    model.open_folder(qml_path::to_qml(&path), new_tab);
                });
            }
        });
    }

    fn eject(mut self: Pin<&mut Self>, row: i32) {
        let Some(row) = at(row) else {
            return;
        };
        let effects = self.as_mut().rust_mut().panel.eject(row);
        self.apply(effects);
    }

    /// Unmounts the volume, and powers its drive off when it can be.
    fn eject_device(self: Pin<&mut Self>, device: Device) {
        let service = self.rust().device_service.clone();
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
            let _ = thread.queue(|model| model.not_busy());
        });
    }

    fn finish_rename(mut self: Pin<&mut Self>, typed: &QString, apply: bool) {
        let effects = self
            .as_mut()
            .rust_mut()
            .panel
            .finish_rename(&typed.to_string(), apply);
        self.apply(effects);
    }

    fn command(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let Some(Action::Places(command)) = commands::parse(&id.to_string()) else {
            return false;
        };
        match self.as_mut().rust_mut().panel.run(command) {
            Some(effects) => {
                self.apply(effects);
                true
            }
            None => false,
        }
    }

    fn open_menu(mut self: Pin<&mut Self>, row: i32) -> bool {
        match self.as_mut().rust_mut().panel.open_menu(at(row)) {
            Some(effects) => {
                self.apply(effects);
                true
            }
            None => false,
        }
    }

    fn run_menu(mut self: Pin<&mut Self>, path: &QString) {
        let effects = self.as_mut().rust_mut().panel.run_menu(&path.to_string());
        self.apply(effects);
    }

    fn add_folder(mut self: Pin<&mut Self>, uri: &QString) {
        if let Some(folder) = qml_path::from_qml(uri) {
            let effects = self.as_mut().rust_mut().panel.add_folder(folder, None);
            self.apply(effects);
        }
    }

    fn drop_row(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let (Some(from), Some(to)) = (at(from), at(to)) else {
            return;
        };
        let effects = self.as_mut().rust_mut().panel.drop_row(from, to);
        self.apply(effects);
    }

    fn drop_folders(mut self: Pin<&mut Self>, uris: &QString, to: i32) {
        let folders = rustphin_platform::uri::paths_from_uri_list(uris.to_string().as_bytes())
            .into_iter()
            .filter(|path| path.is_dir())
            .collect();
        let effects = self.as_mut().rust_mut().panel.drop_folders(folders, at(to));
        self.apply(effects);
    }

    fn palette_json(&self) -> QString {
        text(&palette_entries(self.rust().panel.places()).to_string())
    }

    fn icon_for(&self, uri: &QString) -> QString {
        let Some(folder) = qml_path::from_qml(uri) else {
            return QString::default();
        };
        text(
            self.rust()
                .panel
                .places()
                .iter()
                .find(|place| place.path.as_deref() == Some(folder.as_path()))
                .map_or("", icon_for),
        )
    }

    fn groups_json(&self) -> QString {
        let groups: Vec<serde_json::Value> = self
            .rust()
            .panel
            .groups()
            .groups
            .iter()
            .map(|group| serde_json::json!({ "name": group.name, "count": group.places.len() }))
            .collect();
        text(&serde_json::Value::Array(groups).to_string())
    }

    fn shift_group(mut self: Pin<&mut Self>, index: i32, delta: i32) {
        if let Some(index) = at(index) {
            let effects = self
                .as_mut()
                .rust_mut()
                .panel
                .shift_group(index, delta as isize);
            self.apply(effects);
        }
    }

    fn delete_group(mut self: Pin<&mut Self>, index: i32) {
        if let Some(index) = at(index) {
            let effects = self.as_mut().rust_mut().panel.delete_group(index);
            self.apply(effects);
        }
    }

    fn rename_group(mut self: Pin<&mut Self>, index: i32, name: &QString) {
        if let Some(index) = at(index) {
            let effects = self
                .as_mut()
                .rust_mut()
                .panel
                .rename_group(index, &name.to_string());
            self.apply(effects);
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
