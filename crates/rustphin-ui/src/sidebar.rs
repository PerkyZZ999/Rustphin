//! The places panel: KDE's shared places, arranged in user-defined groups.
//!
//! Places come from the file Dolphin and KDE's file dialogs share and are
//! edited there in place. Groups are Rustphin's own and stored separately
//! (decision D-008). All file access happens off the UI thread.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable as _,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, actions,
    component::{
        Icon, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        input::{Input, InputEvent, InputState},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_core::{Fs as _, RealFs};
use rustphin_platform::{
    config,
    devices::{self, Device, Devices, NoDevices, UDisks2},
    places::{EMPTY, Place, Places, PlacesError},
};

use crate::{
    browser::DraggedEntries,
    format,
    groups::Groups,
    operations::hub,
    theme::{Theme, size, space, text},
};

const KEY_CONTEXT: &str = "Sidebar";
/// The panel while no text field inside it has focus.
const LIST_CONTEXT: &str = "Sidebar && !Input";
/// How long the shared file must be quiet before it is read again. Other
/// programs write next to it all the time.
const RELOAD_DELAY: Duration = Duration::from_millis(300);

actions!(
    sidebar,
    [
        FocusPrevious,
        FocusNext,
        Open,
        RenameRow,
        RemoveRow,
        HideRow,
        MoveRowUp,
        MoveRowDown,
        Collapse,
        Expand,
        NextGroup,
        NewGroup,
        CancelRename,
        Eject,
        ToggleHiddenPlaces,
    ]
);

/// Registers the panel's keyboard shortcuts.
pub fn bind_keys(cx: &mut App) {
    let list = Some(LIST_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", FocusPrevious, list),
        KeyBinding::new("down", FocusNext, list),
        KeyBinding::new("enter", Open, list),
        KeyBinding::new("f2", RenameRow, list),
        KeyBinding::new("delete", RemoveRow, list),
        KeyBinding::new("h", HideRow, list),
        KeyBinding::new("alt-up", MoveRowUp, list),
        KeyBinding::new("alt-down", MoveRowDown, list),
        KeyBinding::new("left", Collapse, list),
        KeyBinding::new("right", Expand, list),
        KeyBinding::new("g", NextGroup, list),
        KeyBinding::new("ctrl-shift-g", NewGroup, list),
        KeyBinding::new("e", Eject, list),
        KeyBinding::new("escape", CancelRename, Some(KEY_CONTEXT)),
    ]);
}

/// What the panel asks the window to do.
pub enum SidebarEvent {
    /// Show this folder in the active pane.
    Open(PathBuf),
    /// Show this folder in a new tab.
    OpenInNewTab(PathBuf),
    /// Show the trash in the active pane.
    OpenTrash,
}

/// One line of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    /// A place, by its identifier in the shared file.
    Place(String),
    /// A group heading, by the group's position.
    Group(usize),
    /// A storage volume, by the device service's name for it.
    Device(String),
}

/// A place being dragged within the panel.
struct DraggedPlace {
    id: String,
    title: SharedString,
}

impl Render for DraggedPlace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        div()
            .px(px(space::MD))
            .py(px(space::XS))
            .rounded(px(size::RADIUS))
            .bg(theme.selection)
            .text_color(theme.on_selection)
            .font_family(text::FAMILY_BODY)
            .text_size(px(text::SMALL))
            .child(self.title.clone())
    }
}

/// The places panel.
pub struct Sidebar {
    /// The shared places file; `None` when no data directory is known.
    places_path: Option<PathBuf>,
    /// Rustphin's groups file; `None` when it must not be written.
    groups_path: Option<PathBuf>,
    /// Every bookmark of the shared file, in its order.
    places: Vec<Place>,
    groups: Groups,
    show_hidden: bool,
    /// The row the keyboard acts on.
    focused: Option<Row>,
    /// The row whose name is being edited.
    renaming: Option<Row>,
    rename_input: Entity<InputState>,
    /// A group to start renaming once it is drawn: one that was just created.
    rename_requested: Option<Row>,
    /// The folder the active pane shows, to mark its place.
    current: Option<PathBuf>,
    /// Storage volumes, and how full the mounted ones are (size, free).
    devices: Vec<Device>,
    usage: HashMap<String, (u64, u64)>,
    device_service: Arc<dyn Devices>,
    /// The volume being mounted or unmounted, shown as busy.
    device_busy: Option<String>,
    focus: FocusHandle,
    _watch: Option<rustphin_core::WatchGuard>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

/// Whether the panel shows `place`: local folders and the trash, unless
/// another application keeps the bookmark to itself.
fn is_shown(place: &Place, show_hidden: bool) -> bool {
    let reachable = place.path.is_some() || place.href.starts_with("trash:");
    let ours = place
        .only_in_app
        .as_deref()
        .is_none_or(|app| app == "rustphin");
    reachable && ours && (show_hidden || !place.hidden)
}

/// The rows of the panel, top to bottom: places outside groups, then each
/// group's heading followed by its places unless it is collapsed.
fn rows(places: &[Place], groups: &Groups, show_hidden: bool) -> Vec<Row> {
    let layout = groups.layout(places);
    let mut rows: Vec<Row> = layout
        .ungrouped
        .iter()
        .filter(|place| is_shown(place, show_hidden))
        .map(|place| Row::Place(place.id.clone()))
        .collect();
    for (index, (group, members)) in layout.groups.iter().enumerate() {
        rows.push(Row::Group(index));
        if !group.collapsed {
            rows.extend(
                members
                    .iter()
                    .filter(|place| is_shown(place, show_hidden))
                    .map(|place| Row::Place(place.id.clone())),
            );
        }
    }
    rows
}

/// The Lucide icon standing in for a theme icon name.
fn icon_for(place: &Place) -> gpui_kit::assets::IconName {
    use gpui_kit::assets::IconName as I;
    let name = place.icon.as_deref().unwrap_or_default();
    let has = |part: &str| name.contains(part);
    if place.href.starts_with("trash:") || has("trash") {
        I::Trash
    } else if has("home") {
        I::House
    } else if has("desktop") {
        I::Monitor
    } else if has("download") {
        I::Download
    } else if has("document") {
        I::FileText
    } else if has("music") || has("audio") {
        I::Music
    } else if has("picture") || has("image") {
        I::Image
    } else if has("video") {
        I::Video
    } else if has("game") {
        I::Gamepad2
    } else {
        I::Folder
    }
}

/// The name shown for a volume: its label, or its size when it has none.
fn device_name(device: &Device) -> String {
    match &device.label {
        Some(label) => label.clone(),
        None if device.mount_point.as_deref() == Some(Path::new("/")) => "System".to_owned(),
        None => format!("{} volume", format::size(device.size)),
    }
}

/// How full a volume is, in words short enough for the panel.
fn usage_text(total: u64, free: u64) -> String {
    format!(
        "{} free of {}",
        format::size_short(free),
        format::size_short(total)
    )
}

/// Whether the panel offers to unmount or remove `device`: anything mounted
/// or unpluggable, except the volumes the system and the user's home run from.
fn can_eject(device: &Device, home: Option<&Path>) -> bool {
    match &device.mount_point {
        Some(mount) => mount != Path::new("/") && home.is_none_or(|home| !home.starts_with(mount)),
        None => device.removable && device.can_power_off(),
    }
}

/// The time now, in seconds since the Unix epoch; KDE builds identifiers from it.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

impl Sidebar {
    /// Local, visible bookmarks offered by the palette, even in folded groups.
    pub fn palette_places(&self) -> impl Iterator<Item = &Place> {
        self.places.iter().filter(|place| is_shown(place, false))
    }

    /// Creates the panel and starts reading the places.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let places_path = rustphin_platform::xdg::data_dirs()
            .into_iter()
            .next()
            .map(|dir| dir.join("user-places.xbel"));
        let groups_path = config::config_dir().map(|dir| dir.join("place-groups.toml"));
        let rename_input = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![cx.subscribe_in(&rename_input, window, Self::on_rename_input)];

        // Follow edits made by Dolphin and file dialogs.
        let (changed, changes) = flume::unbounded::<()>();
        let watch = places_path
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
        cx.spawn(async move |panel, cx| {
            while changes.recv_async().await.is_ok() {
                cx.background_executor().timer(RELOAD_DELAY).await;
                while changes.try_recv().is_ok() {}
                if panel.update(cx, |panel, cx| panel.reload(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        let mut panel = Self {
            places_path,
            groups_path,
            places: Vec::new(),
            groups: Groups::default(),
            show_hidden: false,
            focused: None,
            renaming: None,
            rename_input,
            rename_requested: None,
            current: None,
            devices: Vec::new(),
            usage: HashMap::new(),
            device_service: Arc::new(NoDevices),
            device_busy: None,
            focus: cx.focus_handle(),
            _watch: watch,
            _subscriptions: subscriptions,
        };
        panel.load_groups(cx);
        panel.reload(cx);
        panel.connect_devices(cx);
        panel
    }

    /// Gives the panel the keyboard focus.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
    }

    /// Whether the panel, or a field inside it, has the keyboard focus.
    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }

    /// Tells the panel which folder the active pane shows.
    pub fn set_current(&mut self, folder: Option<PathBuf>, cx: &mut Context<Self>) {
        if self.current != folder {
            self.current = folder;
            cx.notify();
        }
    }

    fn report(error: impl std::fmt::Display, cx: &mut Context<Self>) {
        let text = format!("The places could not be changed: {error}.");
        hub(cx).update(cx, |hub, cx| hub.say(text, true, cx));
    }

    /// Reads the groups file once, in the background.
    fn load_groups(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.groups_path.clone() else {
            return;
        };
        cx.spawn(async move |panel, cx| {
            let text = cx
                .background_spawn(async move { config::read(&path) })
                .await;
            let _ = panel.update(cx, |panel, cx| {
                if let Ok(Some(text)) = text {
                    panel.groups = Groups::parse(&text);
                    // Never overwrite a file a later version of Rustphin wrote.
                    if panel.groups.is_from_newer_version() {
                        panel.groups_path = None;
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Stores the groups, in the background.
    fn save_groups(&self, cx: &mut Context<Self>) {
        let Some(path) = self.groups_path.clone() else {
            return;
        };
        let text = self.groups.to_toml();
        cx.background_spawn(async move {
            if let Err(error) = config::write_atomic(&path, text.as_bytes()) {
                tracing::warn!(%error, path = %path.display(), "could not save the place groups");
            }
        })
        .detach();
    }

    /// Reads the shared places file again, in the background.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.places_path.clone() else {
            return;
        };
        cx.spawn(async move |panel, cx| {
            let read = cx
                .background_spawn(async move {
                    let text = config::read(&path)?.unwrap_or_else(|| EMPTY.to_owned());
                    Ok::<_, std::io::Error>(Places::parse(&text))
                })
                .await;
            let _ = panel.update(cx, |panel, cx| match read {
                Ok(Ok(places)) => {
                    let places: Vec<Place> = places.places().cloned().collect();
                    if places != panel.places {
                        panel.places = places;
                        // Only now is it known which places are really gone.
                        if panel.groups.forget_missing(&panel.places) {
                            panel.save_groups(cx);
                        }
                        cx.notify();
                    }
                }
                // Keep showing what was read last; another program may be
                // in the middle of rewriting the file.
                Ok(Err(error)) => tracing::warn!(%error, "the places file could not be read"),
                Err(error) => tracing::warn!(%error, "the places file could not be read"),
            });
        })
        .detach();
    }

    /// Applies `edit` to the shared file as it is on disk now, writes the
    /// result atomically and reads it back. `then` runs after a successful
    /// write with whatever `edit` returned beside the new text.
    fn edit_places<T: Send + 'static>(
        &mut self,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&Places) -> Result<(String, T), PlacesError> + Send + 'static,
        then: impl FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
    ) {
        let Some(path) = self.places_path.clone() else {
            return;
        };
        cx.spawn(async move |panel, cx| {
            let written = cx
                .background_spawn(async move {
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
                })
                .await;
            let _ = panel.update(cx, |panel, cx| match written {
                Ok(value) => {
                    then(panel, value, cx);
                    panel.reload(cx);
                }
                Err(error) => Self::report(error, cx),
            });
        })
        .detach();
    }

    /// Adds `folder` to the places, in the group at `group` if given.
    pub fn add_place(&mut self, folder: PathBuf, group: Option<usize>, cx: &mut Context<Self>) {
        if self
            .places
            .iter()
            .any(|place| place.path.as_deref() == Some(&folder))
        {
            let text = format!(
                "\u{201c}{}\u{201d} is already in the places.",
                folder.display()
            );
            hub(cx).update(cx, |hub, cx| hub.say(text, false, cx));
            return;
        }
        let title = match folder.file_name() {
            Some(name) => rustphin_core::display_name(name.as_encoded_bytes()).into_owned(),
            None => "/".to_owned(),
        };
        self.edit_places(
            cx,
            move |places| places.with_added(&title, &folder, "folder", now()),
            move |panel, id: String, cx| {
                if group.is_some() {
                    panel.groups.assign(&id, group, None);
                    panel.save_groups(cx);
                }
                panel.focused = Some(Row::Place(id));
            },
        );
    }

    fn place(&self, id: &str) -> Option<&Place> {
        self.places.iter().find(|place| place.id == id)
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = rows(&self.places, &self.groups, self.show_hidden);
        rows.extend(
            self.devices
                .iter()
                .map(|device| Row::Device(device.id.clone())),
        );
        rows
    }

    fn move_focus(&mut self, delta: isize, cx: &mut Context<Self>) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let at = self
            .focused
            .as_ref()
            .and_then(|focused| rows.iter().position(|row| row == focused));
        let target = match at {
            Some(at) => at.saturating_add_signed(delta).min(rows.len() - 1),
            None if delta < 0 => rows.len() - 1,
            None => 0,
        };
        self.focused = Some(rows[target].clone());
        cx.notify();
    }

    fn open(&mut self, row: &Row, new_tab: bool, cx: &mut Context<Self>) {
        match row {
            Row::Place(id) => {
                let Some(place) = self.place(id) else { return };
                if let Some(path) = place.path.clone() {
                    cx.emit(if new_tab {
                        SidebarEvent::OpenInNewTab(path)
                    } else {
                        SidebarEvent::Open(path)
                    });
                } else if place.href.starts_with("trash:") {
                    cx.emit(SidebarEvent::OpenTrash);
                }
            }
            Row::Group(index) => {
                let collapsed = self.groups.groups.get(*index).is_some_and(|g| g.collapsed);
                self.set_collapsed(*index, !collapsed, cx);
            }
            Row::Device(id) => self.open_device(id, new_tab, cx),
        }
    }

    /// Connects to the device service in the background and starts
    /// following it.
    fn connect_devices(&mut self, cx: &mut Context<Self>) {
        let (changed, changes) = flume::unbounded::<()>();
        cx.spawn(async move |panel, cx| {
            let service = cx
                .background_spawn(async move {
                    let service = UDisks2::connect()?;
                    service.watch(Box::new(move || {
                        let _ = changed.send(());
                    }))?;
                    Ok::<_, devices::DeviceError>(service)
                })
                .await;
            match service {
                Ok(service) => {
                    let _ = panel.update(cx, |panel, cx| {
                        panel.device_service = Arc::new(service);
                        panel.reload_devices(cx);
                    });
                }
                Err(error) => {
                    tracing::warn!(%error, "no device service; volumes are not listed");
                    return;
                }
            }
            while changes.recv_async().await.is_ok() {
                // One plug or mount sends several signals.
                cx.background_executor().timer(RELOAD_DELAY).await;
                while changes.try_recv().is_ok() {}
                if panel
                    .update(cx, |panel, cx| panel.reload_devices(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Lists the volumes again, in the background.
    fn reload_devices(&mut self, cx: &mut Context<Self>) {
        let service = self.device_service.clone();
        cx.spawn(async move |panel, cx| {
            let listed = cx
                .background_spawn(async move {
                    let devices = service.list()?;
                    let usage: HashMap<String, (u64, u64)> = devices
                        .iter()
                        .filter_map(|device| {
                            let usage = devices::usage(device.mount_point.as_deref()?)?;
                            Some((device.id.clone(), usage))
                        })
                        .collect();
                    Ok::<_, devices::DeviceError>((devices, usage))
                })
                .await;
            let _ = panel.update(cx, |panel, cx| match listed {
                Ok((devices, usage)) => {
                    panel.devices = devices;
                    panel.usage = usage;
                    cx.notify();
                }
                Err(error) => tracing::warn!(%error, "the volumes could not be listed"),
            });
        })
        .detach();
    }

    fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// Shows a volume, mounting it first if it is not mounted.
    fn open_device(&mut self, id: &str, new_tab: bool, cx: &mut Context<Self>) {
        let Some(device) = self.device(id).cloned() else {
            return;
        };
        let emit = move |path: PathBuf, cx: &mut Context<Self>| {
            cx.emit(if new_tab {
                SidebarEvent::OpenInNewTab(path)
            } else {
                SidebarEvent::Open(path)
            });
        };
        if let Some(path) = device.mount_point.clone() {
            return emit(path, cx);
        }
        let service = self.device_service.clone();
        self.device_busy = Some(device.id.clone());
        cx.notify();
        cx.spawn(async move |panel, cx| {
            let name = device_name(&device);
            let mounted = cx
                .background_spawn(async move { service.mount(&device) })
                .await;
            let _ = panel.update(cx, |panel, cx| {
                panel.device_busy = None;
                match mounted {
                    Ok(path) => emit(path, cx),
                    Err(error) => {
                        let text = format!("\u{201c}{name}\u{201d} could not be mounted: {error}.");
                        hub(cx).update(cx, |hub, cx| hub.say(text, true, cx));
                    }
                }
                panel.reload_devices(cx);
            });
        })
        .detach();
    }

    /// Unmounts a volume and, when its drive allows it, powers the drive off
    /// so it can be unplugged.
    fn eject(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(device) = self.device(id).cloned() else {
            return;
        };
        if !can_eject(&device, rustphin_platform::xdg::home().as_deref()) {
            return;
        }
        let service = self.device_service.clone();
        self.device_busy = Some(device.id.clone());
        cx.notify();
        cx.spawn(async move |panel, cx| {
            let name = device_name(&device);
            let done = cx
                .background_spawn(async move {
                    if device.mount_point.is_some() {
                        service.unmount(&device)?;
                    }
                    if device.removable && device.can_power_off() {
                        service.power_off(&device)?;
                        return Ok(true);
                    }
                    Ok::<_, devices::DeviceError>(false)
                })
                .await;
            let _ = panel.update(cx, |panel, cx| {
                panel.device_busy = None;
                let (text, is_error) = match done {
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
                hub(cx).update(cx, |hub, cx| hub.say(text, is_error, cx));
                panel.reload_devices(cx);
            });
        })
        .detach();
    }

    fn device_row(
        &self,
        device: &Device,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use gpui_kit::assets::IconName as I;
        let row = Row::Device(device.id.clone());
        let is_current = device.mount_point.is_some() && device.mount_point == self.current;
        let busy = self.device_busy.as_deref() == Some(device.id.as_str());
        let usage = self.usage.get(&device.id).copied();
        let ejectable = can_eject(device, rustphin_platform::xdg::home().as_deref());
        let (click_row, middle_row) = (row.clone(), row.clone());
        let eject_id = device.id.clone();
        let name: SharedString = device_name(device).into();
        self.row_surface(&row, is_current, theme, window, cx)
            .id(SharedString::from(format!("device-{}", device.id)))
            .h(px(size::DEVICE_ROW))
            // A volume that is not mounted is listed, but fainter.
            .when(device.mount_point.is_none(), |line| {
                line.text_color(theme.text_muted)
            })
            .child(
                Icon::new(if busy {
                    I::Loader
                } else if device.removable {
                    I::Usb
                } else {
                    I::HardDrive
                })
                .size(px(size::ICON))
                .flex_none()
                .text_color(if is_current {
                    theme.accent
                } else {
                    theme.text_muted
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XS / 2.))
                    .child(div().truncate().child(name))
                    // How full it is: a bar, and the same in words on hover
                    // and in the line under it.
                    .when_some(usage, |column, (total, free)| {
                        let used = 1. - free as f32 / total as f32;
                        column
                            .child(
                                div()
                                    .w_full()
                                    .h(px(size::PROGRESS / 2.))
                                    .rounded(px(size::RADIUS))
                                    .bg(theme.hover)
                                    .child(
                                        div()
                                            .h_full()
                                            .w(gpui_kit::relative(used.clamp(0., 1.)))
                                            .rounded(px(size::RADIUS))
                                            .bg(theme.activity),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family(text::FAMILY_DATA)
                                    .text_size(px(text::TINY))
                                    .text_color(theme.text_muted)
                                    .truncate()
                                    .child(usage_text(total, free)),
                            )
                    }),
            )
            .when(ejectable && !busy, |line| {
                line.child(
                    Button::new(SharedString::from(format!("eject-{}", device.id)))
                        .ghost()
                        .xsmall()
                        .icon(I::Eject)
                        .tooltip(if device.removable && device.can_power_off() {
                            "Safely remove (E)"
                        } else {
                            "Unmount (E)"
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.eject(&eject_id.clone(), cx);
                        })),
                )
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focused = Some(click_row.clone());
                this.focus.focus(window, cx);
                this.open(&click_row.clone(), false, cx);
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, _, cx| this.open(&middle_row.clone(), true, cx)),
            )
            .into_any_element()
    }

    fn set_collapsed(&mut self, index: usize, collapsed: bool, cx: &mut Context<Self>) {
        self.groups.set_collapsed(index, collapsed);
        self.save_groups(cx);
        cx.notify();
    }

    fn begin_rename(&mut self, row: Row, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &row {
            Row::Place(id) => self.place(id).map(|place| place.title.clone()),
            Row::Group(index) => self.groups.groups.get(*index).map(|g| g.name.clone()),
            // A volume's label belongs to its filesystem.
            Row::Device(_) => None,
        };
        let Some(name) = name else { return };
        let len = name.len();
        self.rename_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.set_selected_range(0..len, cx);
            input.focus(window, cx);
        });
        self.renaming = Some(row);
        cx.notify();
    }

    fn finish_rename(&mut self, apply: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.renaming.take() else {
            return;
        };
        let typed = self.rename_input.read(cx).value().trim().to_owned();
        if apply && !typed.is_empty() {
            match row {
                Row::Place(id) => {
                    if self.place(&id).is_some_and(|place| place.title != typed) {
                        self.edit_places(
                            cx,
                            move |places| Ok((places.with_title(&id, &typed)?, ())),
                            |_, (), _| {},
                        );
                    }
                }
                Row::Group(index) => {
                    self.groups.rename(index, &typed);
                    self.save_groups(cx);
                }
                Row::Device(_) => {}
            }
        }
        if self
            .rename_input
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            self.focus.focus(window, cx);
        }
        cx.notify();
    }

    fn on_rename_input(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
            self.finish_rename(true, window, cx);
        }
    }

    /// Removes the focused row: a place leaves the places (a built-in one is
    /// hidden instead, since KDE would only put it back), a group is
    /// dissolved and its places stay.
    fn remove_focused(&mut self, cx: &mut Context<Self>) {
        match self.focused.clone() {
            Some(Row::Place(id)) => {
                let Some(place) = self.place(&id) else { return };
                if place.system {
                    return self.set_hidden(&id, true, cx);
                }
                self.move_focus(1, cx);
                self.edit_places(
                    cx,
                    move |places| Ok((places.without(&id)?, ())),
                    |_, (), _| {},
                );
            }
            Some(Row::Group(index)) => {
                self.groups.remove(index);
                self.focused = None;
                self.save_groups(cx);
                cx.notify();
            }
            // A volume is not removed from the list; `E` unmounts it.
            Some(Row::Device(_)) | None => {}
        }
    }

    fn set_hidden(&mut self, id: &str, hidden: bool, cx: &mut Context<Self>) {
        let id = id.to_owned();
        self.edit_places(
            cx,
            move |places| Ok((places.with_hidden(&id, hidden)?, ())),
            |_, (), _| {},
        );
    }

    fn toggle_hidden_focused(&mut self, cx: &mut Context<Self>) {
        if let Some(Row::Place(id)) = self.focused.clone()
            && let Some(hidden) = self.place(&id).map(|place| place.hidden)
        {
            self.set_hidden(&id, !hidden, cx);
        }
    }

    /// Moves the focused row one position up or down: a group among the
    /// groups, a place within its group or, outside groups, within the
    /// shared file.
    fn shift_focused(&mut self, down: bool, cx: &mut Context<Self>) {
        match self.focused.clone() {
            Some(Row::Group(index)) => {
                let target = self.groups.shift(index, if down { 1 } else { -1 });
                self.focused = Some(Row::Group(target));
                self.save_groups(cx);
                cx.notify();
            }
            Some(Row::Place(id)) if self.groups.group_of(&id).is_some() => {
                if self.groups.shift_place(&id, down) {
                    self.save_groups(cx);
                    cx.notify();
                }
            }
            Some(Row::Place(id)) => {
                // Among the places shown outside groups.
                let layout = self.groups.layout(&self.places);
                let shown: Vec<&str> = layout
                    .ungrouped
                    .iter()
                    .filter(|place| is_shown(place, self.show_hidden))
                    .map(|place| place.id.as_str())
                    .collect();
                let Some(at) = shown.iter().position(|shown| *shown == id) else {
                    return;
                };
                // Moving down means landing before the place after the next.
                let before = if down {
                    match shown.get(at + 1) {
                        Some(_) => shown.get(at + 2).map(|id| (*id).to_owned()),
                        None => return,
                    }
                } else {
                    match at.checked_sub(1) {
                        Some(previous) => Some(shown[previous].to_owned()),
                        None => return,
                    }
                };
                self.edit_places(
                    cx,
                    move |places| Ok((places.with_moved(&id, before.as_deref())?, ())),
                    |_, (), _| {},
                );
            }
            Some(Row::Device(_)) | None => {}
        }
    }

    /// Sends the focused place to the next group: from outside the groups to
    /// the first, then on to each following one, and from the last back out.
    fn cycle_group(&mut self, cx: &mut Context<Self>) {
        let Some(Row::Place(id)) = self.focused.clone() else {
            return;
        };
        let next = match self.groups.group_of(&id) {
            None if self.groups.groups.is_empty() => return,
            None => Some(0),
            Some(index) if index + 1 < self.groups.groups.len() => Some(index + 1),
            Some(_) => None,
        };
        self.move_to_group(&id, next, None, cx);
    }

    /// Puts the place `id` in the group at `group` (unfolding it), or outside
    /// all groups.
    fn move_to_group(
        &mut self,
        id: &str,
        group: Option<usize>,
        before: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.groups.assign(id, group, before);
        if let Some(index) = group {
            self.groups.set_collapsed(index, false);
        }
        self.save_groups(cx);
        cx.notify();
    }

    fn new_group(&mut self, cx: &mut Context<Self>) {
        let index = self.groups.add("");
        self.focused = Some(Row::Group(index));
        self.rename_requested = Some(Row::Group(index));
        self.save_groups(cx);
        cx.notify();
    }

    /// A place dragged within the panel was dropped on `target`.
    fn drop_place(&mut self, id: &str, target: &Row, cx: &mut Context<Self>) {
        match target {
            Row::Group(index) => self.move_to_group(id, Some(*index), None, cx),
            Row::Device(_) => {}
            Row::Place(other) if other == id => {}
            Row::Place(other) => match self.groups.group_of(other) {
                Some(index) => self.move_to_group(id, Some(index), Some(other), cx),
                None => {
                    // Out of any group, and before `other` in the shared file.
                    self.move_to_group(id, None, None, cx);
                    let (id, other) = (id.to_owned(), other.clone());
                    self.edit_places(
                        cx,
                        move |places| Ok((places.with_moved(&id, Some(&other))?, ())),
                        |_, (), _| {},
                    );
                }
            },
        }
    }

    /// Folders dragged from a file view were dropped on `target`, or on the
    /// panel itself.
    fn drop_folders(
        &mut self,
        folders: Vec<PathBuf>,
        target: Option<&Row>,
        cx: &mut Context<Self>,
    ) {
        let group = match target {
            Some(Row::Group(index)) => Some(*index),
            Some(Row::Place(id)) => self.groups.group_of(id),
            Some(Row::Device(_)) | None => None,
        };
        for folder in folders {
            self.add_place(folder, group, cx);
        }
    }

    /// Makes `element` accept places dragged within the panel and folders
    /// dragged from a file view.
    fn drop_target<E: InteractiveElement>(
        element: E,
        target: Option<Row>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> E {
        let panel = cx.entity().downgrade();
        let highlight = target.is_some();
        let folder_target = target.clone();
        element
            .drag_over::<DraggedPlace>(move |style, _, _, _| {
                if highlight {
                    style.bg(theme.hover)
                } else {
                    style
                }
            })
            .drag_over::<DraggedEntries>(move |style, _, _, _| {
                if highlight {
                    style.bg(theme.hover)
                } else {
                    style
                }
            })
            .on_drop({
                let panel = panel.clone();
                move |dragged: &DraggedPlace, _, cx| {
                    if let Some(target) = &target {
                        let _ =
                            panel.update(cx, |panel, cx| panel.drop_place(&dragged.id, target, cx));
                    }
                }
            })
            // Read the dragged entries before updating the panel: their
            // source view must not be read during another update.
            .on_drop(move |dragged: &DraggedEntries, _, cx| {
                let folders = dragged.folder_paths(cx);
                let _ = panel.update(cx, |panel, cx| {
                    panel.drop_folders(folders, folder_target.as_ref(), cx);
                });
            })
    }

    fn row_surface(
        &self,
        row: &Row,
        is_current: bool,
        theme: Theme,
        window: &Window,
        cx: &App,
    ) -> gpui_kit::Div {
        let keyboard_here = self.focused.as_ref() == Some(row) && self.is_focused(window, cx);
        div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::ROW))
            .mx(px(space::XS))
            .px(px(space::SM))
            .rounded(px(size::RADIUS))
            .border_l(px(size::SELECTION_BAR))
            .border_color(gpui_kit::transparent_black())
            .when(is_current, |row| {
                row.bg(theme.hover).border_color(theme.accent)
            })
            // The row the keyboard acts on, shown while the panel has the focus.
            .when(keyboard_here, |row| row.border_color(theme.accent))
            .hover(|row| row.bg(theme.hover))
    }

    fn name_or_field(&self, row: &Row, name: SharedString, theme: Theme) -> gpui_kit::Div {
        if self.renaming.as_ref() == Some(row) {
            div()
                .flex_1()
                .min_w_0()
                .rounded(px(size::RADIUS))
                .bg(theme.raised)
                .text_color(theme.text)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(Input::new(&self.rename_input).xsmall())
        } else {
            div().flex_1().min_w_0().truncate().child(name)
        }
    }

    fn place_row(
        &self,
        place: &Place,
        indented: bool,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let row = Row::Place(place.id.clone());
        let is_current = place.path.is_some() && place.path == self.current;
        let title: SharedString = place.title.clone().into();
        let dragged = DraggedPlace {
            id: place.id.clone(),
            title: title.clone(),
        };
        let click_row = row.clone();
        let middle_row = row.clone();
        let line = self
            .row_surface(&row, is_current, theme, window, cx)
            .id(SharedString::from(format!("place-{}", place.id)))
            .when(indented, |line| line.pl(px(space::LG + space::SM)))
            // A hidden place is only listed while hidden ones are shown.
            .when(place.hidden, |line| line.opacity(0.5))
            .child(
                Icon::new(icon_for(place))
                    .size(px(size::ICON))
                    .flex_none()
                    .text_color(if is_current {
                        theme.accent
                    } else {
                        theme.text_muted
                    }),
            )
            .child(self.name_or_field(&row, title, theme))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focused = Some(click_row.clone());
                this.focus.focus(window, cx);
                this.open(&click_row.clone(), false, cx);
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, _, cx| this.open(&middle_row.clone(), true, cx)),
            )
            .on_drag(dragged, |dragged, _, _, cx| {
                cx.new(|_| DraggedPlace {
                    id: dragged.id.clone(),
                    title: dragged.title.clone(),
                })
            });
        Self::drop_target(line, Some(row), theme, cx).into_any_element()
    }

    fn group_row(
        &self,
        index: usize,
        group: &crate::groups::Group,
        members: usize,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use gpui_kit::assets::IconName as I;
        let row = Row::Group(index);
        let (name, collapsed) = (group.name.as_str(), group.collapsed);
        let click_row = row.clone();
        let line = self
            .row_surface(&row, false, theme, window, cx)
            .id(("group", index))
            .mt(px(space::SM))
            .text_color(theme.text_muted)
            .font_family(text::FAMILY_HEADING)
            .text_size(px(text::SMALL))
            // The arrow, not colour, says whether the group is open.
            .child(
                Icon::new(if collapsed {
                    I::ChevronRight
                } else {
                    I::ChevronDown
                })
                .size(px(size::ICON_SMALL))
                .flex_none(),
            )
            .child(self.name_or_field(&row, name.to_owned().into(), theme))
            .when(collapsed && members > 0, |line| {
                line.child(
                    div()
                        .flex_none()
                        .font_family(text::FAMILY_DATA)
                        .child(members.to_string()),
                )
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focused = Some(click_row.clone());
                this.focus.focus(window, cx);
                this.open(&click_row.clone(), false, cx);
            }));
        Self::drop_target(line, Some(row), theme, cx).into_any_element()
    }
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::assets::IconName as I;
        let theme = *cx.global::<Theme>();
        if let Some(row) = self.rename_requested.take() {
            cx.defer_in(window, |this, window, cx| {
                this.begin_rename(row, window, cx)
            });
        }

        // Owned copies: building rows needs the panel mutably for listeners.
        let places = self.places.clone();
        let groups = self.groups.clone();
        let layout = groups.layout(&places);
        let mut lines: Vec<AnyElement> = Vec::new();
        for place in layout
            .ungrouped
            .iter()
            .filter(|place| is_shown(place, self.show_hidden))
        {
            lines.push(self.place_row(place, false, theme, window, cx));
        }
        for (index, (group, members)) in layout.groups.iter().enumerate() {
            let shown: Vec<&&Place> = members
                .iter()
                .filter(|place| is_shown(place, self.show_hidden))
                .collect();
            lines.push(self.group_row(index, group, shown.len(), theme, window, cx));
            if !group.collapsed {
                for place in shown {
                    lines.push(self.place_row(place, true, theme, window, cx));
                }
            }
        }
        if lines.is_empty() {
            lines.push(
                div()
                    .p(px(space::MD))
                    .text_size(px(text::SMALL))
                    .text_color(theme.text_muted)
                    .child(
                        "No places yet. Add the folder shown with Ctrl+D, or drag a folder here.",
                    )
                    .into_any_element(),
            );
        }
        let devices = self.devices.clone();
        if !devices.is_empty() {
            lines.push(
                div()
                    .mt(px(space::SM))
                    .mx(px(space::XS))
                    .px(px(space::SM))
                    .h(px(size::ROW))
                    .flex()
                    .items_center()
                    .font_family(text::FAMILY_HEADING)
                    .text_size(px(text::SMALL))
                    .text_color(theme.text_muted)
                    .child("Devices")
                    .into_any_element(),
            );
            for device in &devices {
                lines.push(self.device_row(device, theme, window, cx));
            }
        }
        let any_hidden = places.iter().any(|place| place.hidden);
        let current = self.current.clone();

        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::XS))
            .h(px(size::TOOLBAR))
            .px(px(space::SM))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px(px(space::XS))
                    .font_family(text::FAMILY_HEADING)
                    .text_size(px(text::SMALL))
                    .text_color(theme.text_muted)
                    .child("Places"),
            )
            .child(
                Button::new("add-place")
                    .ghost()
                    .small()
                    .icon(I::BookmarkPlus)
                    .tooltip("Add the folder shown to the places (Ctrl+D)")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(folder) = current.clone() {
                            this.add_place(folder, None, cx);
                        }
                    })),
            )
            .child(
                Button::new("new-group")
                    .ghost()
                    .small()
                    .icon(I::ListPlus)
                    .tooltip("New group (Ctrl+Shift+G)")
                    .on_click(cx.listener(|this, _, _, cx| this.new_group(cx))),
            )
            .when(any_hidden, |header| {
                header.child(
                    Button::new("show-hidden-places")
                        .ghost()
                        .small()
                        .icon(if self.show_hidden { I::Eye } else { I::EyeOff })
                        .selected(self.show_hidden)
                        .tooltip(if self.show_hidden {
                            "Stop listing hidden places"
                        } else {
                            "List hidden places too"
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_hidden = !this.show_hidden;
                            cx.notify();
                        })),
                )
            });

        let list = Self::drop_target(div().id("places-list"), None, theme, cx)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .py(px(space::XS))
            .children(lines);

        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &FocusPrevious, _, cx| this.move_focus(-1, cx)))
            .on_action(cx.listener(|this, _: &FocusNext, _, cx| this.move_focus(1, cx)))
            .on_action(cx.listener(|this, _: &Open, _, cx| {
                if let Some(row) = this.focused.clone() {
                    this.open(&row, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &RenameRow, window, cx| {
                if let Some(row) = this.focused.clone() {
                    this.begin_rename(row, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &RemoveRow, _, cx| this.remove_focused(cx)))
            .on_action(cx.listener(|this, _: &HideRow, _, cx| this.toggle_hidden_focused(cx)))
            .on_action(cx.listener(|this, _: &MoveRowUp, _, cx| this.shift_focused(false, cx)))
            .on_action(cx.listener(|this, _: &MoveRowDown, _, cx| this.shift_focused(true, cx)))
            .on_action(cx.listener(|this, _: &Collapse, _, cx| {
                if let Some(Row::Group(index)) = this.focused {
                    this.set_collapsed(index, true, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Expand, _, cx| {
                if let Some(Row::Group(index)) = this.focused {
                    this.set_collapsed(index, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &NextGroup, _, cx| this.cycle_group(cx)))
            .on_action(cx.listener(|this, _: &NewGroup, _, cx| this.new_group(cx)))
            .on_action(cx.listener(|this, _: &ToggleHiddenPlaces, _, cx| {
                this.show_hidden = !this.show_hidden;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Eject, _, cx| {
                if let Some(Row::Device(id)) = this.focused.clone() {
                    this.eject(&id, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CancelRename, window, cx| {
                this.finish_rename(false, window, cx);
            }))
            .flex()
            .flex_col()
            .flex_none()
            .w(px(size::SIDEBAR))
            .h_full()
            .bg(theme.chrome)
            .border_r_1()
            .border_color(theme.border)
            .text_color(theme.text)
            .font_family(text::FAMILY_BODY)
            .text_size(px(text::BODY))
            .line_height(px(text::LINE))
            .child(header)
            .child(list)
    }
}

#[cfg(test)]
mod tests {
    use crate::groups::Group;

    use super::*;

    fn place(id: &str, href: &str) -> Place {
        Place {
            id: id.to_owned(),
            title: id.to_owned(),
            href: href.to_owned(),
            path: rustphin_platform::uri::path_from_uri(href.as_bytes()),
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        }
    }

    #[test]
    fn local_folders_and_the_trash_are_shown_and_other_places_are_not() {
        let mut restricted = place("r", "file:///x");
        restricted.only_in_app = Some("dolphin".into());
        let mut ours = place("o", "file:///x");
        ours.only_in_app = Some("rustphin".into());
        let mut hidden = place("h", "file:///x");
        hidden.hidden = true;

        assert!(is_shown(&place("a", "file:///home/u"), false));
        assert!(is_shown(&place("t", "trash:/"), false));
        assert!(is_shown(&ours, false));
        assert!(!is_shown(&place("n", "remote:/"), false));
        assert!(!is_shown(&place("s", "recentlyused:/files"), false));
        assert!(!is_shown(&restricted, false));
        assert!(!is_shown(&hidden, false));
        assert!(is_shown(&hidden, true));
    }

    #[test]
    fn rows_list_ungrouped_places_then_each_group_and_skip_collapsed_members() {
        let mut hidden = place("hidden", "file:///h");
        hidden.hidden = true;
        let places = [
            place("home", "file:///home/u"),
            place("work-a", "file:///a"),
            place("net", "remote:/"),
            place("work-b", "file:///b"),
            place("media", "file:///m"),
            hidden,
        ];
        let mut groups = Groups::default();
        groups.groups = vec![
            Group {
                name: "Work".into(),
                collapsed: false,
                places: vec!["work-b".into(), "work-a".into(), "hidden".into()],
            },
            Group {
                name: "Media".into(),
                collapsed: true,
                places: vec!["media".into()],
            },
        ];
        let p = |id: &str| Row::Place(id.to_owned());

        assert_eq!(
            rows(&places, &groups, false),
            [
                p("home"),
                Row::Group(0),
                p("work-b"),
                p("work-a"),
                Row::Group(1)
            ]
        );
        assert_eq!(
            rows(&places, &groups, true),
            [
                p("home"),
                Row::Group(0),
                p("work-b"),
                p("work-a"),
                p("hidden"),
                Row::Group(1)
            ]
        );
    }

    fn device(mount: Option<&str>, removable: bool, power_off: bool) -> Device {
        Device {
            id: "/dev/x".into(),
            label: None,
            mount_point: mount.map(PathBuf::from),
            size: 64 * 1024 * 1024 * 1024,
            removable,
            power_off: power_off.then(|| "/drive".to_owned()),
        }
    }

    #[test]
    fn the_system_and_home_volumes_are_never_offered_for_unmounting() {
        let home = Some(Path::new("/home/u"));
        assert!(!can_eject(&device(Some("/"), false, false), home));
        assert!(!can_eject(&device(Some("/home"), false, false), home));
        assert!(!can_eject(&device(Some("/home/u"), false, false), home));
        assert!(can_eject(&device(Some("/mnt/data"), false, false), home));
        assert!(can_eject(&device(Some("/home/u/stick"), true, true), home));
        // Not mounted: only a drive that can be powered off has anything to do.
        assert!(can_eject(&device(None, true, true), home));
        assert!(!can_eject(&device(None, true, false), home));
        assert!(!can_eject(&device(None, false, false), home));
    }

    #[test]
    fn a_volume_without_a_label_is_named_by_its_size_and_usage_is_put_in_words() {
        let mut volume = device(None, true, true);
        assert_eq!(device_name(&volume), "64.0 GiB volume");
        volume.label = Some("STICK".into());
        assert_eq!(device_name(&volume), "STICK");
        assert_eq!(
            usage_text(64 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
            "16 GiB free of 64 GiB"
        );
        assert_eq!(device_name(&device(Some("/"), false, false)), "System");
    }

    #[test]
    fn theme_icon_names_map_to_bundled_icons() {
        use gpui_kit::assets::IconName as I;
        let with_icon = |icon: &str| {
            let mut place = place("x", "file:///x");
            place.icon = Some(icon.to_owned());
            icon_for(&place)
        };
        assert_eq!(with_icon("user-home-symbolic"), I::House);
        assert_eq!(with_icon("folder-downloads"), I::Download);
        assert_eq!(with_icon("folder-documents"), I::FileText);
        assert_eq!(with_icon("applications-games-symbolic"), I::Gamepad2);
        assert_eq!(with_icon("something-else"), I::Folder);
        assert_eq!(icon_for(&place("t", "trash:/")), I::Trash);
    }
}
