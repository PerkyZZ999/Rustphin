//! The browser view: toolbar, entry list and status line for one location.

use crate::controls::{Outlined as _, tool_button};
use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    AccessibleAction, AnyElement, App, AppContext as _, AsyncApp, Bounds, ClickEvent, Context,
    Edges, Entity, EventEmitter, ExternalDragPayload, ExternalPaths, FileDragPaths, FocusHandle,
    Focusable as _, InteractiveElement, IntoElement, KeyBinding, KeyDownEvent, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point,
    Render, Role, ScrollHandle, ScrollStrategy, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, UniformListScrollHandle, WeakEntity, Window, actions, anchored,
    component::{
        Icon, IconName, Sizable as _,
        button::{Button, ButtonVariants as _},
        input::{Input, InputEvent, InputState},
        scroll::{Scrollbar, ScrollbarMode},
    },
    deferred, div, img, point,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    DirectoryModel, EntryKind, Fs, LoadError, ModelEvent, NameFilter, RealFs, Selection, Snapshot,
    SortKey, ViewOptions, display_name, ops::Request,
};
use rustphin_platform::{remote::RemoteAddress, uri::path_from_uri};

use crate::{
    diagnostics::FrameLog,
    filetypes::{Database, TypeInfo},
    format::{self, count, plural},
    layout::{self, Flow, Grid, Layout, Rect, Step, ViewMode},
    location::Location,
    menu::{self, MenuCommand, MenuEffect, MenuKey},
    menu_view::{MenuAccept, MenuClose, MenuDown, MenuLeft, MenuRight, MenuUp},
    operations::{self, hub},
    remote,
    settings::Settings,
    theme::{Theme, size, space, text},
    thumbs::{self, thumbs},
};

const KEY_CONTEXT: &str = "Browser";
/// The only action inside an empty-folder message.
const FOLDER_ACTION_CONTEXT: &str = "FolderStateAction";
/// The browser while no text field inside it has focus.
const LIST_CONTEXT: &str = "Browser && !Input";
/// How far the pointer must move with the button down before a rubber band starts.
const BAND_THRESHOLD: f32 = 4.;
/// A pause longer than this starts a new type-ahead search.
const TYPE_AHEAD_PAUSE: Duration = Duration::from_millis(1000);
/// Icons smaller than this stay type icons: a thumbnail would be a smudge.
const MIN_THUMBNAIL_EDGE: f32 = 24.;

actions!(
    browser,
    [
        SelectPrevious,
        SelectNext,
        SelectFirst,
        SelectLast,
        PageUp,
        PageDown,
        ExtendPrevious,
        ExtendNext,
        ExtendFirst,
        ExtendLast,
        ExtendPageUp,
        ExtendPageDown,
        SelectLeft,
        SelectRight,
        ExtendLeft,
        ExtendRight,
        SelectAll,
        InvertSelection,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        ShowIcons,
        ShowCompact,
        ShowDetails,
        Activate,
        GoUp,
        GoHome,
        GoBack,
        GoForward,
        Reload,
        ToggleHidden,
        ToggleFoldersFirst,
        ShowFilter,
        ShowHidden,
        EditLocation,
        CancelEdit,
        Rename,
        NewFolder,
        NewFile,
        MoveToTrash,
        DeletePermanently,
        ShowProperties,
        RecoverRenames,
        Copy,
        CopyLocation,
        Duplicate,
        Cut,
        Paste,
        OpenContextMenu,
        OpenTrash,
        RestoreTrash,
        EmptyTrash,
        TrashActions,
        SortName,
        SortSize,
        SortModified,
        SortType,
    ]
);

/// Registers the browser's keyboard shortcuts. They match Dolphin's defaults.
///
/// Keys that also edit text (arrows, Enter, Backspace, Home, End) only act on
/// the list while no text field has focus; otherwise typing a path would move
/// the selection or navigate.
pub fn bind_keys(cx: &mut App) {
    let list = Some(LIST_CONTEXT);
    let view = Some(KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", SelectPrevious, list),
        KeyBinding::new("down", SelectNext, list),
        KeyBinding::new("home", SelectFirst, list),
        KeyBinding::new("end", SelectLast, list),
        KeyBinding::new("pageup", PageUp, list),
        KeyBinding::new("pagedown", PageDown, list),
        KeyBinding::new("shift-up", ExtendPrevious, list),
        KeyBinding::new("shift-down", ExtendNext, list),
        KeyBinding::new("shift-home", ExtendFirst, list),
        KeyBinding::new("shift-end", ExtendLast, list),
        KeyBinding::new("shift-pageup", ExtendPageUp, list),
        KeyBinding::new("shift-pagedown", ExtendPageDown, list),
        KeyBinding::new("left", SelectLeft, list),
        KeyBinding::new("right", SelectRight, list),
        KeyBinding::new("shift-left", ExtendLeft, list),
        KeyBinding::new("shift-right", ExtendRight, list),
        KeyBinding::new("ctrl-a", SelectAll, list),
        KeyBinding::new("ctrl-shift-a", InvertSelection, list),
        KeyBinding::new("enter", Activate, list),
        // Dolphin goes back with Backspace (Up is Alt+Up).
        KeyBinding::new("backspace", GoBack, list),
        KeyBinding::new("alt-home", GoHome, view),
        KeyBinding::new("alt-up", GoUp, view),
        KeyBinding::new("alt-left", GoBack, view),
        // The Back and Forward keys of some keyboards and mice.
        KeyBinding::new("back", GoBack, view),
        KeyBinding::new("forward", GoForward, view),
        KeyBinding::new("alt-right", GoForward, view),
        KeyBinding::new("f5", Reload, view),
        KeyBinding::new("ctrl-h", ToggleHidden, view),
        KeyBinding::new("alt-.", ToggleHidden, view),
        KeyBinding::new("ctrl-l", EditLocation, view),
        // Dolphin's filter bar: Ctrl+I toggles it, / opens it from the list.
        KeyBinding::new("ctrl-i", ShowFilter, view),
        KeyBinding::new("/", ShowFilter, list),
        KeyBinding::new("f6", EditLocation, view),
        KeyBinding::new("alt-d", EditLocation, view),
        KeyBinding::new("ctrl-=", ZoomIn, view),
        KeyBinding::new("ctrl-+", ZoomIn, view),
        KeyBinding::new("ctrl--", ZoomOut, view),
        KeyBinding::new("ctrl-0", ZoomReset, view),
        KeyBinding::new("ctrl-1", ShowIcons, view),
        KeyBinding::new("ctrl-2", ShowCompact, view),
        KeyBinding::new("ctrl-3", ShowDetails, view),
        KeyBinding::new("escape", CancelEdit, view),
        KeyBinding::new("f2", Rename, list),
        KeyBinding::new("f10", NewFolder, list),
        KeyBinding::new("ctrl-shift-n", NewFolder, list),
        KeyBinding::new("ctrl-alt-n", NewFile, list),
        KeyBinding::new("delete", MoveToTrash, list),
        KeyBinding::new("shift-delete", DeletePermanently, list),
        KeyBinding::new("alt-enter", ShowProperties, list),
        KeyBinding::new("ctrl-c", Copy, list),
        KeyBinding::new("ctrl-x", Cut, list),
        KeyBinding::new("ctrl-v", Paste, list),
        KeyBinding::new("shift-f10", OpenContextMenu, list),
        KeyBinding::new("menu", OpenContextMenu, list),
        KeyBinding::new("ctrl-alt-c", CopyLocation, list),
        KeyBinding::new("ctrl-d", Duplicate, list),
        KeyBinding::new("enter", ShowHidden, Some(FOLDER_ACTION_CONTEXT)),
        KeyBinding::new("space", ShowHidden, Some(FOLDER_ACTION_CONTEXT)),
    ]);
}

/// A failed attempt to open a location, shown inline until dismissed.
struct Failure {
    path: PathBuf,
    error: LoadError,
    /// The folder being shown went away and the view moved to a parent,
    /// rather than an attempt to open something failing.
    moved: bool,
}

/// Where the button of a folder error shown in place leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// Back to the previous folder (Alt+Left).
    Back,
    /// Up to the containing folder (Alt+Up), when there is nothing to go back to.
    Up,
    /// Nowhere: no history and no parent.
    Nowhere,
}

impl Failure {
    /// Whether to explain the failure in place of the list, as mockup 28
    /// draws it, rather than in a banner over a listing. Only an attempt to
    /// open the folder now shown, with nothing listed for it, is said in
    /// place.
    fn in_place(&self, location: Option<&Path>, listed: bool) -> bool {
        !self.moved && !listed && location == Some(self.path.as_path())
    }

    /// The in-place explanation: what happened, then what can be done.
    fn explanation(&self, way: Way) -> String {
        let path = display_name(self.path.as_os_str().as_encoded_bytes());
        let (what, advice) = match &self.error {
            LoadError::PermissionDenied => (
                format!("You do not have permission to open {path}."),
                "Ask its owner for access",
            ),
            LoadError::NotFound => (format!("{path} does not exist."), "Check the address"),
            LoadError::NotADirectory => {
                return format!(
                    "{path} is a file, not a folder. Open it from the folder that contains it."
                );
            }
            LoadError::Io(reason) => (
                format!("{path} could not be read: {reason}."),
                "Try again with F5",
            ),
        };
        let way = match way {
            Way::Back => ", or go back to the previous folder",
            Way::Up => ", or go up to the folder that contains it",
            Way::Nowhere => "",
        };
        format!("{what} {advice}{way}.")
    }

    /// The status line's text while the failure is shown in place.
    fn status(&self) -> &'static str {
        match self.error {
            LoadError::PermissionDenied => "Not readable",
            LoadError::NotFound => "Does not exist",
            LoadError::NotADirectory => "Not a folder",
            LoadError::Io(_) => "Could not be read",
        }
    }

    /// What happened, in a sentence.
    fn message(&self) -> String {
        let path = display_name(self.path.as_os_str().as_encoded_bytes());
        if self.moved {
            format!("{path} no longer exists. Showing the nearest folder that does.")
        } else {
            format!("Could not open {path}: {}.", self.error)
        }
    }
}

/// What a browser asks of whatever contains it.
#[derive(Debug, Clone)]
pub enum BrowserEvent {
    /// Show this folder in a new tab (a middle click on it).
    OpenInNewTab(PathBuf),
    /// A folder was successfully listed, rather than merely requested.
    Visited(PathBuf),
    /// Activate the file at this path: open it, or ask before running it.
    Activate(PathBuf),
    /// Show properties for this entry.
    Properties {
        /// The entry.
        path: PathBuf,
        /// A readable type, such as "Plain text document".
        type_label: String,
        /// What the dialog's header shows until a thumbnail, if any, is ready.
        look: crate::properties::Look,
    },
    /// Open this file with the application whose desktop-file id is `id`.
    OpenWith {
        /// Desktop-file id, such as `org.kde.kate.desktop`.
        id: String,
        /// The file to open.
        path: PathBuf,
    },
    /// Rename these selected names together. `staying` is every other name
    /// in the folder.
    BatchRename {
        /// The folder that holds the files.
        directory: PathBuf,
        /// Selected names, in list order.
        selected: Vec<OsString>,
        /// Names in the folder that are not selected.
        staying: Vec<OsString>,
    },
}

impl EventEmitter<BrowserEvent> for Browser {}

/// One browsing view of the filesystem.
pub struct Browser {
    /// `None` when the model's worker pool could not be created.
    model: Option<DirectoryModel>,
    snapshot: Option<Arc<Snapshot>>,
    /// The newest request; older snapshots are ignored.
    generation: u64,
    /// Last successful visit announced to the workspace history.
    visited: Option<PathBuf>,
    selection: Selection,
    /// Invalidates worker results when input changes the selection.
    selection_revision: u64,
    /// Row of the selection's focus in the current snapshot, looked up once
    /// per snapshot instead of on every frame.
    focus_row: Option<usize>,
    /// Characters typed so far in a type-ahead search, and when the last arrived.
    typed: String,
    typed_at: Option<Instant>,
    /// Entry to select once the directory being opened has been listed.
    /// Names to select once the folder being opened has finished listing.
    select_on_arrival: Option<Vec<Vec<u8>>>,
    failure: Option<Failure>,
    editing_location: bool,
    /// A remote folder being mounted for this pane, and where the pane was
    /// when it was asked for. It counts only while the pane is still there.
    pending_remote: Option<(RemoteAddress, Option<PathBuf>)>,
    location_input: Entity<InputState>,
    /// The filter bar's field, and whether the bar is shown.
    filter_input: Entity<InputState>,
    filter_open: bool,
    /// The entry being renamed in place, by its current name.
    renaming: Option<Vec<u8>>,
    rename_input: Entity<InputState>,
    /// An entry to select as soon as it shows up in the list, and whether to
    /// start renaming it: the result of a create or rename this view asked for.
    select_when_seen: Option<(Vec<u8>, bool)>,
    /// The copies a duplicate this view asked for will create: selected
    /// together once all of them show up, as Dolphin does.
    select_all_when_seen: Vec<Vec<u8>>,
    /// An entry whose renaming starts at the next redraw, when a window is at hand.
    rename_requested: Option<Vec<u8>>,
    /// Take the keyboard focus back at the next redraw.
    refocus: bool,
    /// The trash directory whose contents this view is showing, if it is one.
    trash_dir: Option<PathBuf>,
    /// The size and free space of the volume holding the folder shown, and
    /// the folder it was read for.
    /// The folder, its volume's size and free space, and where that
    /// volume is mounted.
    free_space: Option<(PathBuf, u64, u64, Option<PathBuf>)>,
    /// The name of the volume holding the folder, from the places panel.
    volume: Option<String>,
    /// What the status line last derived from a snapshot and selection.
    status_sizes: std::cell::RefCell<Option<StatusSizes>>,
    /// Whether something in the folder shown is cut; refreshed every redraw.
    has_cut: bool,
    /// Set by an entry when the mouse button goes down on its draggable part.
    press_on_entry: bool,
    /// The user's other trash directories, on other volumes, while a trash is shown.
    other_trashes: Vec<PathBuf>,
    focus: FocusHandle,
    /// The context menu, while one is open.
    menu: Option<OpenMenu>,
    /// Holds the keyboard focus while the context menu is open.
    menu_focus: FocusHandle,
    /// The bounded message shown while a listing has no visible entries.
    folder_state_scroll: ScrollHandle,
    /// Contains only the hidden-files action; it is not an extra tab stop.
    folder_action_focus: FocusHandle,
    /// The system's file-type databases, once loaded in the background.
    types: Option<Arc<Database>>,
    /// Icon and description per MIME type, filled in as rows need them.
    type_info: HashMap<String, TypeInfo>,
    /// Types whose lookup is running, so each is requested once.
    types_pending: HashSet<String>,
    /// The zone dates are shown in.
    zone: TimeZone,
    mode: ViewMode,
    zoom: usize,
    /// Entries per line (details, icons) or per column (compact) as last
    /// drawn; 1 in details mode.
    lanes: usize,
    /// Bumped when the icon size changes, so lookups for the old size are dropped.
    icon_epoch: u64,
    /// Where view settings are stored; `None` when they must not be written.
    settings_path: Option<PathBuf>,
    /// The settings as last written, to skip writes that change nothing.
    saved: Settings,
    frames: FrameLog,
    /// The width this view is drawn at, in pixels; zero until known.
    width: f32,
    /// A rubber-band selection being dragged, if any.
    band: Option<Band>,
    /// Set when a band ends, so the click that the same press produces is ignored.
    ignore_click: bool,
    scroll: UniformListScrollHandle,
    /// The compact view's sideways scrolling.
    columns_scroll: ScrollHandle,
    /// The height the compact view's columns were last fitted to.
    columns_height: f32,
    /// The folder the compact view last showed; another one starts at the left.
    columns_folder: Option<PathBuf>,
    /// In split view, whether this pane is the active side; `None` alone.
    in_split: Option<bool>,
    /// KDE's administrator helper while the window acts as administrator
    /// (W-098); `None` reads the filesystem as the user.
    admin_fs: Option<Arc<dyn Fs>>,
    /// Which model events belong to: a new model (a switch into or out of
    /// administrator mode) ignores what the old one still sends.
    model_epoch: u64,
    _subscriptions: Vec<Subscription>,
}

impl Browser {
    /// Creates the view and starts listing `start`.
    pub fn new(
        start: PathBuf,
        settings: Settings,
        settings_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Never overwrite a file a later version of Rustphin wrote.
        let settings_path = settings_path.filter(|_| !settings.is_from_newer_version());
        let location_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter a folder path"));
        let rename_input = cx.new(|cx| InputState::new(window, cx));
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name"));
        let mut subscriptions = vec![
            cx.subscribe_in(&location_input, window, Self::on_location_input),
            cx.subscribe_in(&filter_input, window, Self::on_filter_input),
            cx.subscribe_in(&rename_input, window, Self::on_rename_input),
            // Cut entries are drawn dimmed, and the clipboard changes elsewhere.
            cx.observe(&hub(cx), |_, _, cx| cx.notify()),
            // Thumbnails arrive from background threads.
            cx.observe(&thumbs(cx), |_, _, cx| cx.notify()),
        ];
        // Whatever contains this view decides when it gets the keyboard focus:
        // a view created for a background tab must not take it.
        let focus = cx.focus_handle();
        let menu_focus = cx.focus_handle();
        subscriptions.push(cx.on_blur(&menu_focus, window, |this, _, cx| {
            if this.menu.take().is_some() {
                cx.notify();
            }
        }));

        let folder_action_focus = cx.focus_handle().tab_stop(false);
        subscriptions.push(cx.on_focus_in(&folder_action_focus, window, |this, _, cx| {
            this.folder_state_scroll.scroll_to_bottom();
            cx.notify();
        }));

        let mut this = Self {
            model: None,
            snapshot: None,
            generation: 0,
            visited: None,
            selection: Selection::default(),
            selection_revision: 0,
            focus_row: None,
            typed: String::new(),
            typed_at: None,
            select_on_arrival: None,
            failure: None,
            editing_location: false,
            pending_remote: None,
            location_input,
            filter_input,
            filter_open: false,
            renaming: None,
            rename_input,
            select_when_seen: None,
            select_all_when_seen: Vec::new(),
            rename_requested: None,
            refocus: false,
            trash_dir: None,
            free_space: None,
            volume: None,
            status_sizes: std::cell::RefCell::new(None),
            has_cut: false,
            press_on_entry: false,
            other_trashes: Vec::new(),
            focus,
            menu: None,
            menu_focus,
            folder_state_scroll: ScrollHandle::new(),
            folder_action_focus,
            types: None,
            type_info: HashMap::new(),
            types_pending: HashSet::new(),
            zone: TimeZone::system(),
            mode: settings.mode(),
            zoom: settings.zoom(),
            lanes: 1,
            icon_epoch: 0,
            settings_path,
            saved: settings.clone(),
            frames: FrameLog::from_env(cx),
            width: 0.,
            band: None,
            ignore_click: false,
            scroll: UniformListScrollHandle::new(),
            columns_scroll: ScrollHandle::new(),
            columns_height: 0.,
            columns_folder: None,
            in_split: None,
            admin_fs: None,
            model_epoch: 0,
            _subscriptions: subscriptions,
        };
        match DirectoryModel::new(Arc::new(RealFs)) {
            Ok((mut model, events)) => {
                model.set_options(settings.options());
                this.model = Some(model);
                this.load_file_types(cx);
                cx.spawn(async move |view, cx| Self::receive_events(view, events, 0, cx).await)
                    .detach();
                this.open(start);
            }
            Err(error) => {
                tracing::error!(%error, "could not start the directory model");
                this.failure = Some(Failure {
                    path: start,
                    error: error.into(),
                    moved: false,
                });
            }
        }
        this
    }

    async fn receive_events(
        view: WeakEntity<Self>,
        events: flume::Receiver<ModelEvent>,
        epoch: u64,
        cx: &mut AsyncApp,
    ) {
        'events: while let Ok(event) = events.recv_async().await {
            // A replaced model's late events must not reach the new one.
            if !view
                .read_with(cx, |view, _| view.model_epoch == epoch)
                .unwrap_or(false)
            {
                break;
            }
            loop {
                let Ok(work) = view.update(cx, |view, _| view.selection_work(&event)) else {
                    break 'events;
                };
                let Some((revision, receive)) = work else {
                    if view
                        .update(cx, |view, cx| view.on_model_event(event, None, cx))
                        .is_err()
                    {
                        break 'events;
                    }
                    break;
                };
                let selection = match receive.recv_async().await {
                    Ok(selection) => selection,
                    Err(error) => {
                        let _ = view.update(cx, |view, cx| {
                            tracing::error!(%error, "selection worker stopped");
                            view.failure = view.location().map(|path| Failure {
                                path,
                                error: LoadError::Io(
                                    "Selection update failed; refresh the folder to try again."
                                        .into(),
                                ),
                                moved: false,
                            });
                            cx.notify();
                        });
                        break;
                    }
                };
                let Ok(applied) = view.update(cx, |view, cx| {
                    if view.selection_revision != revision {
                        return false;
                    }
                    view.on_model_event(event.clone(), Some(selection), cx);
                    true
                }) else {
                    break 'events;
                };
                if applied {
                    break;
                }
            }
        }
    }

    /// Keep the old view usable while a replacement's names are matched off-thread.
    fn selection_work(&self, event: &ModelEvent) -> Option<(u64, flume::Receiver<Selection>)> {
        let _task = self.frames.task("selection_schedule");
        let ModelEvent::Snapshot(new) = event else {
            return None;
        };
        let old = self.snapshot.as_ref()?;
        if new.generation() < self.generation
            || old.path() != new.path()
            || old.listing() == new.listing()
        {
            return None;
        }
        let receive = self.model.as_ref()?.carry_selection(
            self.selection.clone(),
            Arc::clone(old),
            Arc::clone(new),
        );
        Some((self.selection_revision, receive))
    }

    fn on_model_event(
        &mut self,
        event: ModelEvent,
        selection: Option<Selection>,
        cx: &mut Context<Self>,
    ) {
        let _task = self.frames.task("model_event");
        match event {
            // Generations above the last one asked for are re-reads the model
            // started itself after the directory changed.
            ModelEvent::Snapshot(snapshot) if snapshot.generation() >= self.generation => {
                self.generation = snapshot.generation();
                let same_place = self
                    .snapshot
                    .as_ref()
                    .filter(|shown| shown.path() == snapshot.path());
                // Keeps the selection through a re-sort, and by name through a
                // re-read of the same directory; a new directory starts empty.
                {
                    let _selection = self.frames.task("selection_snapshot");
                    if let Some(selection) = selection {
                        self.selection = selection;
                    } else {
                        self.selection.sync(&snapshot);
                    }
                    self.selection_revision += 1;
                }
                let arrived = same_place.is_none();
                if arrived {
                    self.dismiss_menu();
                    self.folder_state_scroll.set_offset(point(px(0.), px(0.)));
                    if self.renaming.take().is_some() {
                        self.refocus = true;
                    }
                    self.rename_requested = None;
                    self.select_when_seen = None;
                    self.select_all_when_seen.clear();
                    self.detect_trash(snapshot.path().to_owned(), cx);
                    self.measure_free_space(snapshot.path().to_owned(), cx);
                    // What the previous folder was waiting for is not wanted now.
                    thumbs(cx).update(cx, |thumbs, _| thumbs.drop_waiting());
                } else if same_place.is_some_and(|shown| shown.listing() != snapshot.listing()) {
                    // The folder was read again: its pictures may have changed.
                    let folder = snapshot.path().to_owned();
                    thumbs(cx).update(cx, |thumbs, _| thumbs.forget_folder(&folder));
                }
                let arrival = self.select_on_arrival.take_if(|_| !snapshot.is_loading());
                let arrived_names = arrival.is_some();
                if let Some(names) = arrival {
                    select_names(&mut self.selection, &snapshot, &names);
                }
                // The entry this view just created or renamed has shown up.
                let awaited = self
                    .select_when_seen
                    .as_ref()
                    .and_then(|(name, _)| snapshot.row_of(name));
                if let Some(row) = awaited {
                    self.selection.select_only(&snapshot, row);
                    if let Some((name, true)) = self.select_when_seen.take() {
                        self.rename_requested = Some(name);
                    }
                }
                // Every copy of a duplicate has shown up.
                if !self.select_all_when_seen.is_empty()
                    && self
                        .select_all_when_seen
                        .iter()
                        .all(|name| snapshot.row_of(name).is_some())
                {
                    let names = std::mem::take(&mut self.select_all_when_seen);
                    select_names(&mut self.selection, &snapshot, &names);
                }
                // A rename in progress ends if its entry is gone.
                if let Some(name) = &self.renaming
                    && snapshot.row_of(name).is_none()
                {
                    self.renaming = None;
                    // The name field is gone; the keyboard goes back to the list.
                    self.refocus = true;
                }
                self.focus_row = self.selection.focus_row(&snapshot);
                if !snapshot.is_loading() && self.visited.as_deref() != Some(snapshot.path()) {
                    let path = snapshot.path().to_owned();
                    self.visited = Some(path.clone());
                    cx.emit(BrowserEvent::Visited(path));
                }
                self.snapshot = Some(snapshot);
                // A live update must not move the view; only arriving does.
                if arrived || arrived_names || awaited.is_some() {
                    self.reveal_focus();
                }
            }
            ModelEvent::Moved { generation, from } if generation >= self.generation => {
                self.dismiss_menu();
                self.generation = generation;
                self.select_on_arrival = None;
                self.failure = Some(Failure {
                    path: from,
                    error: LoadError::NotFound,
                    moved: true,
                });
            }
            ModelEvent::Failed {
                generation,
                path,
                error,
            } if generation == self.generation => {
                self.select_on_arrival = None;
                // Shown in place, the status line still says how full its disk is.
                self.measure_free_space(path.clone(), cx);
                self.failure = Some(Failure {
                    path,
                    error,
                    moved: false,
                });
            }
            // Results of a request that has since been superseded.
            ModelEvent::Snapshot(_) | ModelEvent::Moved { .. } | ModelEvent::Failed { .. } => {
                return;
            }
        }
        cx.notify();
    }

    fn open(&mut self, path: PathBuf) {
        self.frames.opening(Some(&path));
        self.pending_remote = None;
        self.dismiss_menu();
        self.folder_state_scroll.set_offset(point(px(0.), px(0.)));
        if let Some(model) = &mut self.model {
            self.generation = model.open(path);
            self.failure = None;
        }
    }

    /// Only the listing for the requested location may be rendered or acted on.
    /// Keep the received snapshot for a failed navigation's history rollback.
    fn current_snapshot(&self) -> Option<&Arc<Snapshot>> {
        let location = self.location()?;
        self.snapshot
            .as_ref()
            .filter(|snapshot| snapshot.path() == location)
    }

    fn dismiss_menu(&mut self) {
        if self.menu.take().is_some() {
            self.refocus = true;
        }
    }

    /// Changes the selection against the current snapshot and redraws.
    fn change_selection(
        &mut self,
        change: impl FnOnce(&mut Selection, &Snapshot),
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.current_snapshot().cloned() else {
            return;
        };
        change(&mut self.selection, &snapshot);
        self.selection_revision += 1;
        self.focus_row = self.selection.focus_row(&snapshot);
        self.reveal_focus();
        cx.notify();
    }

    /// A click on a row: plain selects it alone, Ctrl toggles it, Shift
    /// selects the range from the anchor.
    fn click_row(&mut self, row: usize, modifiers: Modifiers, cx: &mut Context<Self>) {
        self.change_selection(
            |selection, snapshot| {
                if modifiers.shift {
                    selection.extend_to(snapshot, row);
                } else if modifiers.control {
                    selection.toggle(snapshot, row);
                } else {
                    selection.select_only(snapshot, row);
                }
            },
            cx,
        );
    }

    /// The height to fit the compact view's columns to: the list's height
    /// as last drawn, or an estimate before it has been. When the list turns
    /// out another height, the next frame draws again with that one.
    fn fit_columns(&mut self, window: &mut Window, cx: &mut Context<Self>) -> f32 {
        let drawn = f32::from(self.columns_scroll.bounds().size.height);
        self.columns_height = if drawn > 0. {
            drawn
        } else {
            f32::from(window.viewport_size().height) - size::TOOLBAR - size::STATUS
        };
        cx.on_next_frame(window, |this, _, cx| {
            let drawn = f32::from(this.columns_scroll.bounds().size.height);
            if this.mode == ViewMode::Compact && drawn > 0. && drawn != this.columns_height {
                cx.notify();
            }
        });
        self.columns_height
    }

    /// Another folder in the compact view starts at its first column; the
    /// same folder keeps its place, within the columns it still has.
    fn follow_columns_folder(&mut self, snapshot: Option<&Snapshot>, grid: Grid) {
        let folder = snapshot.map(|snapshot| snapshot.path());
        let offset = self.columns_scroll.offset();
        if self.columns_folder.as_deref() != folder {
            self.columns_folder = folder.map(Path::to_path_buf);
            self.columns_scroll.set_offset(point(px(0.), offset.y));
            return;
        }
        let viewport = f32::from(self.columns_scroll.bounds().size.width);
        let furthest = (grid.width() + space::SM * 2. - viewport).max(0.);
        if -f32::from(offset.x) > furthest {
            self.columns_scroll
                .set_offset(point(px(-furthest), offset.y));
        }
    }

    /// Where the entries of the current folder sit as last drawn.
    fn arrangement(&self) -> Grid {
        let len = self.current_snapshot().map_or(0, |snapshot| snapshot.len());
        Grid::new(
            self.mode.layout(self.zoom),
            self.mode.flow(),
            self.lanes,
            len,
        )
    }

    fn reveal_focus(&self) {
        let Some(row) = self.focus_row else { return };
        let grid = self.arrangement();
        match grid.flow {
            Flow::Across => self
                .scroll
                .scroll_to_item(row / grid.lanes, ScrollStrategy::Nearest),
            Flow::Down => {
                let viewport = f32::from(self.columns_scroll.bounds().size.width);
                if viewport <= 0. {
                    return;
                }
                let (left, right) = grid.column_span(row);
                let offset = layout::nearest_offset(
                    f32::from(self.columns_scroll.offset().x),
                    left,
                    right + space::SM * 2.,
                    viewport,
                );
                let furthest = (grid.width() + space::SM * 2. - viewport).max(0.);
                self.columns_scroll
                    .set_offset(point(px(offset.clamp(-furthest, 0.)), px(0.)));
            }
        }
    }

    /// Moves the focus by keyboard; with `extend`, selects the range to it.
    fn move_selection(&mut self, step: Step, extend: bool, cx: &mut Context<Self>) {
        let Some(row) = self.arrangement().step(self.focus_row, step) else {
            return;
        };
        self.change_selection(
            |selection, snapshot| {
                if extend {
                    selection.extend_to(snapshot, row);
                } else {
                    selection.select_only(snapshot, row);
                }
            },
            cx,
        );
    }

    /// A window position in list coordinates: relative to the first row's
    /// top-left corner, whatever the scroll position. `None` before the list
    /// has been drawn.
    fn list_point(&self, position: Point<Pixels>) -> Option<(f32, f32)> {
        let (bounds, offset) = self.list_viewport()?;
        Some((
            f32::from(position.x - bounds.origin.x - offset.x) - space::SM,
            f32::from(position.y - bounds.origin.y - offset.y) - space::XS,
        ))
    }

    /// The area the entries are drawn in and how far they are scrolled, in
    /// window coordinates. `None` before the list has been drawn.
    fn list_viewport(&self) -> Option<(Bounds<Pixels>, Point<Pixels>)> {
        let (bounds, offset) = match self.mode.flow() {
            Flow::Across => {
                let state = self.scroll.0.borrow();
                (state.base_handle.bounds(), state.base_handle.offset())
            }
            Flow::Down => (self.columns_scroll.bounds(), self.columns_scroll.offset()),
        };
        (bounds.size.height > px(0.)).then_some((bounds, offset))
    }

    /// Opens the context menu for `row`, or for empty space when `row` is
    /// `None`. A row that is not selected becomes the selection.
    fn show_menu(
        &mut self,
        row: Option<usize>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.renaming.is_some() || self.editing_location {
            return;
        }
        if let Some(row) = row {
            let selected = self
                .current_snapshot()
                .is_some_and(|snapshot| self.selection.contains(snapshot, row));
            if !selected {
                self.change_selection(
                    |selection, snapshot| selection.select_only(snapshot, row),
                    cx,
                );
            }
        }
        let entries = menu::context_menu(&self.menu_facts(row.is_some(), cx));
        let Some(state) = menu::initial(&entries) else {
            return;
        };
        self.menu = Some(OpenMenu {
            at,
            above: None,
            entries,
            state,
            name: "File actions",
            scroll: ScrollHandle::new(),
        });
        self.menu_focus.focus(window, cx);
        cx.notify();
    }

    fn menu_facts(&self, on_selection: bool, cx: &App) -> menu::MenuFacts {
        let snapshot = self.current_snapshot().filter(|_| on_selection);
        let rows = snapshot
            .map(|snapshot| self.selection.rows(snapshot))
            .unwrap_or_default();
        let single_container =
            rows.len() == 1 && snapshot.is_some_and(|snapshot| snapshot.is_container(rows[0]));
        let applications = if rows.len() == 1 && !single_container {
            snapshot
                .zip(self.types.as_ref())
                .map(|(snapshot, types)| {
                    types
                        .applications(types.mime_type(snapshot.name(rows[0]), false))
                        .into_iter()
                        .map(|app| menu::AppChoice {
                            icon: types.app_icon(&app.id),
                            id: app.id,
                            name: app.name,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let can_paste = hub(cx)
            .read(cx)
            .clip
            .as_ref()
            .is_some_and(|clip| !clip.paths.is_empty());
        menu::MenuFacts {
            on_selection,
            count: rows.len(),
            single_container,
            in_trash: self.trash_dir.is_some(),
            as_administrator: self.admin_fs.is_some(),
            can_paste,
            applications,
            types_ready: self.types.is_some(),
        }
    }

    /// Where a keyboard-opened menu appears: under the focused entry, kept
    /// inside the list, or at the list's corner without one.
    fn menu_anchor(&self, row: Option<usize>) -> Point<Pixels> {
        self.menu_anchors(row).0
    }

    /// Where a keyboard-opened menu starts below the focused entry, and the
    /// entry's top for opening upward; the list's corner without an entry.
    fn menu_anchors(&self, row: Option<usize>) -> (Point<Pixels>, Option<Point<Pixels>>) {
        let Some((bounds, offset)) = self.list_viewport() else {
            return (point(px(space::LG), px(size::TOOLBAR + space::LG)), None);
        };
        let corner = point(
            bounds.origin.x + px(space::LG),
            bounds.origin.y + px(space::LG),
        );
        let Some(row) = row else {
            return (corner, None);
        };
        // The inverse of `list_point`.
        let grid = self.arrangement();
        let (x, y) = grid.cell_bottom_left(row);
        let row_height = grid.layout.row_height;
        let top = bounds.origin.y;
        let bottom = bounds.origin.y + bounds.size.height;
        let clamp = |value: Pixels| value.clamp(top, bottom.max(top));
        let left = bounds.origin.x + offset.x + px(x + space::SM + space::LG);
        let below = clamp(bounds.origin.y + offset.y + px(y + space::XS));
        let above = clamp(bounds.origin.y + offset.y + px(y - row_height + space::XS));
        (point(left, below), Some(point(left, above)))
    }

    fn open_selection_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.is_some() {
            self.menu = None;
            self.focus.focus(window, cx);
            cx.notify();
            return;
        }
        if self.selection.is_empty() {
            let at = self.menu_anchor(None);
            self.show_menu(None, at, window, cx);
        } else if let Some(row) = self.focus_row {
            self.reveal_focus();
            let (at, above) = self.menu_anchors(Some(row));
            self.show_menu(Some(row), at, window, cx);
            if let Some(menu) = &mut self.menu {
                menu.above = above;
            }
        }
    }

    fn on_menu_key(&mut self, key: MenuKey, window: &mut Window, cx: &mut Context<Self>) {
        let Some(effect) = self
            .menu
            .as_mut()
            .map(|menu| menu::on_key(&menu.entries, &mut menu.state, key))
        else {
            return;
        };
        match effect {
            MenuEffect::Stay => {
                if let Some(menu) = &self.menu
                    && !menu.state.submenu
                {
                    menu.scroll.scroll_to_item(menu.state.cursor);
                }
                cx.notify();
            }
            MenuEffect::Close => {
                self.menu = None;
                self.focus.focus(window, cx);
                cx.notify();
            }
            MenuEffect::Run(command) => self.run_menu(command, window, cx),
        }
    }

    fn run_menu(&mut self, command: MenuCommand, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        match command {
            MenuCommand::Open => {
                if let Some(row) = self.focus_row {
                    self.activate_row(row, cx);
                }
                self.focus.focus(window, cx);
            }
            MenuCommand::OpenWith(app) => {
                if let Some(path) = self.selected_paths().into_iter().next()
                    && self.selection.len() == 1
                {
                    cx.emit(BrowserEvent::OpenWith { id: app.id, path });
                }
                self.focus.focus(window, cx);
            }
            MenuCommand::Cut => {
                self.pick_up_selected(true, cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Copy => {
                self.pick_up_selected(false, cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Paste => {
                self.paste(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::CopyLocation => {
                self.copy_location(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Duplicate => {
                self.duplicate_selected(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Rename => self.rename_focused(window, cx),
            MenuCommand::MoveToTrash => {
                self.trash_selected(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Delete => {
                self.delete_selected(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::Restore => {
                self.restore_selected(cx);
                self.focus.focus(window, cx);
            }
            MenuCommand::EmptyTrash => {
                if let Some(dir) = self.trash_dir.clone() {
                    hub(cx).update(cx, |hub, cx| hub.empty_trash(dir, cx));
                }
                self.focus.focus(window, cx);
            }
            MenuCommand::OpenTrash(files) => {
                self.open(files);
                self.focus.focus(window, cx);
            }
            MenuCommand::NewFolder => {
                self.create_entry(true, cx);
                // Creating focuses the new name once it appears.
            }
            MenuCommand::NewFile => self.create_entry(false, cx),
            MenuCommand::Properties => self.open_properties(window, cx),
        }
        cx.notify();
    }

    /// Puts the location of the first selected entry on the clipboard as
    /// text: its path, or its remote address inside a remote folder.
    fn copy_location(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.selected_paths().into_iter().next() else {
            hub(cx).update(cx, |hub, cx| {
                hub.say("Select an item to copy its location.".to_owned(), false, cx);
            });
            return;
        };
        let text =
            remote::address_of(&path, cx).unwrap_or_else(|| path.to_string_lossy().into_owned());
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
        let name = path
            .file_name()
            .map(|name| display_name(name.as_encoded_bytes()).into_owned())
            .unwrap_or_default();
        hub(cx).update(cx, |hub, cx| {
            hub.say(
                format!("Copied the location of \u{201c}{name}\u{201d}."),
                false,
                cx,
            );
        });
    }

    /// Asks for the properties of the one selected entry.
    fn open_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.current_snapshot().cloned() else {
            return;
        };
        let rows = self.selection.rows(&snapshot);
        if rows.len() != 1 {
            return;
        }
        let row = rows[0];
        let path = snapshot.entry_path(row);
        let type_label = self.properties_type(&snapshot, row);
        let look = self.properties_look(&snapshot, row);
        self.menu = None;
        cx.emit(BrowserEvent::Properties {
            path,
            type_label,
            look,
        });
        self.focus.focus(window, cx);
    }

    /// The type icon of the entry in `row`, and whether a thumbnail of it
    /// can be made.
    fn properties_look(&self, snapshot: &Snapshot, row: usize) -> crate::properties::Look {
        let container = snapshot.is_container(row);
        let mime = self.types.as_ref().map(|types| {
            types.mime_type(
                snapshot.link_target_name(row).unwrap_or(snapshot.name(row)),
                container,
            )
        });
        crate::properties::Look {
            folder: container,
            icon: mime
                .and_then(|mime| self.type_info.get(mime))
                .and_then(|info| info.icon.clone()),
            picture: !container && mime.is_some_and(thumbs::has_thumbnail),
        }
    }

    fn properties_type(&self, snapshot: &Snapshot, row: usize) -> String {
        if snapshot.kind(row) == EntryKind::Symlink {
            return "Symbolic link".to_owned();
        }
        let mime = self
            .types
            .as_ref()
            .map(|types| types.mime_type(snapshot.name(row), snapshot.is_container(row)));
        if let Some(description) = mime
            .and_then(|mime| self.type_info.get(mime))
            .map(|info| info.description.to_string())
            .filter(|text| !text.is_empty())
        {
            return description;
        }
        if snapshot.is_container(row) {
            "Folder".to_owned()
        } else {
            "File".to_owned()
        }
    }

    fn hover_menu(
        &mut self,
        cursor: usize,
        submenu: bool,
        sub_cursor: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = &mut self.menu else {
            return;
        };
        if menu.state.cursor == cursor
            && menu.state.submenu == submenu
            && (!submenu || menu.state.sub_cursor == sub_cursor)
        {
            return;
        }
        menu.state.cursor = cursor;
        menu.state.submenu = submenu;
        if submenu {
            menu.state.sub_cursor = sub_cursor;
        }
        cx.notify();
    }

    fn entry_right_click(
        row: usize,
        cx: &mut Context<Self>,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.show_menu(Some(row), event.position, window, cx);
        })
    }

    fn on_background_menu(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_menu(None, event.position, window, cx);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ignore_click = false;
        self.band = None;
        // A press outside the menu closes it and does not also select.
        if self.menu.is_some() {
            self.menu = None;
            self.ignore_click = true;
            self.press_on_entry = false;
            self.focus.focus(window, cx);
            cx.notify();
            return;
        }
        // The press landed on something draggable: no rubber band.
        if std::mem::take(&mut self.press_on_entry) {
            return;
        }
        let in_list = self
            .list_viewport()
            .is_some_and(|(bounds, _)| bounds.contains(&event.position));
        if in_list
            && !self.editing_location
            && let Some(start) = self.list_point(event.position)
        {
            self.band = Some(Band {
                start,
                pressed_at: event.position,
                pointer: event.position,
                active: false,
                add: event.modifiers.control,
            });
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut band) = self.band else {
            return;
        };
        if !event.dragging() {
            self.band = None;
            return;
        }
        band.pointer = event.position;
        let moved = event.position - band.pressed_at;
        if f32::from(moved.x).abs().max(f32::from(moved.y).abs()) >= BAND_THRESHOLD {
            band.active = true;
        }
        self.band = Some(band);
        if !band.active {
            return;
        }
        let (Some(end), Some(snapshot)) = (
            self.list_point(event.position),
            self.current_snapshot().cloned(),
        ) else {
            return;
        };
        let rows = self.arrangement().rows_in(Rect::between(band.start, end));
        self.selection.select_rows(&snapshot, rows, band.add);
        self.selection_revision += 1;
        // The view does not scroll to follow a band.
        self.focus_row = self.selection.focus_row(&snapshot);
        cx.notify();
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(band) = self.band.take() {
            self.ignore_click = band.active;
            cx.notify();
        }
    }

    /// The outline of the band being dragged, kept inside the list. Pointer
    /// positions are window coordinates, and this view may start anywhere in
    /// the window (after the places panel, or as the right pane of a split),
    /// so the outline is placed in the window, not in the view.
    fn band_outline(&self, theme: Theme) -> Option<impl IntoElement + use<>> {
        let band = self.band.filter(|band| band.active)?;
        let (list, _) = self.list_viewport()?;
        let (left, right) = (
            band.pressed_at.x.min(band.pointer.x).max(list.left()),
            band.pressed_at.x.max(band.pointer.x).min(list.right()),
        );
        let (top, bottom) = (
            band.pressed_at.y.min(band.pointer.y).max(list.top()),
            band.pressed_at.y.max(band.pointer.y).min(list.bottom()),
        );
        if right <= left || bottom <= top {
            return None;
        }
        Some(
            anchored().position(point(left, top)).child(
                div()
                    .w(right - left)
                    .h(bottom - top)
                    .rounded(px(size::RADIUS))
                    .border_1()
                    .border_color(theme.accent)
                    .bg(theme.hover),
            ),
        )
    }

    /// Jumps to the next entry whose name starts with what has been typed.
    fn type_ahead(&mut self, typed: &str, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self
            .typed_at
            .is_none_or(|at| now.duration_since(at) > TYPE_AHEAD_PAUSE)
        {
            self.typed.clear();
        }
        self.typed.push_str(typed);
        self.typed_at = Some(now);

        let (prefix, advance) = type_ahead_query(&self.typed);
        let from = self.focus_row.map_or(0, |row| row + usize::from(advance));
        let found = self
            .current_snapshot()
            .and_then(|snapshot| snapshot.find_prefix(prefix.as_bytes(), from));
        if let Some(row) = found {
            self.change_selection(
                |selection, snapshot| selection.select_only(snapshot, row),
                cx,
            );
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if self.editing_location
            || self.renaming.is_some()
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
        {
            return;
        }
        if let Some(typed) = event
            .keystroke
            .key_char
            .as_deref()
            .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
        {
            self.type_ahead(typed, cx);
        }
    }

    fn activate_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(snapshot) = self.current_snapshot() else {
            return;
        };
        if row >= snapshot.len() {
            return;
        }
        if snapshot.is_container(row) {
            self.open(snapshot.entry_path(row));
        } else {
            cx.emit(BrowserEvent::Activate(snapshot.entry_path(row)));
        }
        cx.notify();
    }

    fn go_up(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let came_from = model.location().and_then(|path| {
            path.file_name()
                .map(|name| name.as_encoded_bytes().to_vec())
        });
        if let Some(generation) = model.up() {
            self.frames.opening(model.location().as_deref());
            self.dismiss_menu();
            self.generation = generation;
            self.failure = None;
            self.select_on_arrival = came_from.map(|name| vec![name]);
            cx.notify();
        }
    }

    /// Where the volume holding the folder shown is mounted, once measured.
    pub(crate) fn volume_mount(&self) -> Option<PathBuf> {
        self.free_space
            .as_ref()
            .filter(|(path, ..)| self.location().as_deref() == Some(path.as_path()))
            .and_then(|(.., mount)| mount.clone())
    }

    /// Names the volume holding the folder, for the free space.
    pub(crate) fn set_volume(&mut self, name: Option<String>, cx: &mut Context<Self>) {
        if self.volume != name {
            self.volume = name;
            cx.notify();
        }
    }

    /// Reads the folder shown again, as F5 does.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        self.navigate(DirectoryModel::reload, cx);
    }

    fn navigate(
        &mut self,
        step: impl FnOnce(&mut DirectoryModel) -> Option<u64>,
        cx: &mut Context<Self>,
    ) {
        let before = self.location();
        if let Some(generation) = self.model.as_mut().and_then(step) {
            // Back and Forward open a folder; Reload keeps the one shown.
            let after = self.location();
            if after != before {
                self.frames.opening(after.as_deref());
            }
            self.dismiss_menu();
            self.generation = generation;
            self.failure = None;
            cx.notify();
        }
    }

    /// Reads the file-type databases on a background thread.
    fn load_file_types(&mut self, cx: &mut Context<Self>) {
        let loading = cx
            .background_executor()
            .spawn(async { Arc::new(Database::load()) });
        cx.spawn(async move |view, cx| {
            let database = loading.await;
            // The view may be gone by the time loading finishes.
            let _ = view.update(cx, |view, cx| {
                view.types = Some(database);
                cx.notify();
            });
        })
        .detach();
    }

    /// The icon and description for an entry, if already known. An unknown
    /// type is looked up in the background and the view redrawn when it arrives.
    fn type_info_for(
        &mut self,
        name: &[u8],
        is_dir: bool,
        cx: &mut Context<Self>,
    ) -> Option<TypeInfo> {
        let database = self.types.as_ref()?;
        let mime = database.mime_type(name, is_dir);
        if let Some(info) = self.type_info.get(mime) {
            return Some(info.clone());
        }
        if self.types_pending.insert(mime.to_owned()) {
            let mime = mime.to_owned();
            let database = Arc::clone(database);
            let icon_size = theme_icon_size(self.mode.layout(self.zoom).icon);
            let epoch = self.icon_epoch;
            let lookup = cx.background_executor().spawn({
                let mime = mime.clone();
                async move { database.resolve(&mime, icon_size) }
            });
            cx.spawn(async move |view, cx| {
                let info = lookup.await;
                let _ = view.update(cx, |view, cx| {
                    // Drop results for an icon size that is no longer shown.
                    if view.icon_epoch == epoch {
                        view.types_pending.remove(&mime);
                        view.type_info.insert(mime, info);
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        None
    }

    /// Stores the view settings if they changed, on a background thread.
    fn save_settings(&mut self, cx: &mut Context<Self>) {
        let (Some(path), Some(model)) = (self.settings_path.clone(), &self.model) else {
            return;
        };
        let settings = Settings::new(self.mode, self.zoom, model.options());
        if settings == self.saved {
            return;
        }
        self.saved = settings.clone();
        cx.background_executor()
            .spawn(async move {
                let text = settings.to_toml();
                if let Err(error) = rustphin_platform::config::write_atomic(&path, text.as_bytes())
                {
                    tracing::warn!(%error, path = %path.display(), "could not save the settings");
                }
            })
            .detach();
    }

    /// Forgets resolved icons after the icon size changed; rows ask again.
    fn icon_size_changed(&mut self) {
        self.icon_epoch += 1;
        self.type_info.clear();
        self.types_pending.clear();
    }

    fn set_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.icon_size_changed();
        // Only the details view shows sizes and dates.
        if let Some(model) = &mut self.model {
            let options = ViewOptions {
                details: mode == ViewMode::Details,
                ..model.options()
            };
            if let Some(generation) = model.set_options(options) {
                self.generation = generation;
            }
        }
        self.save_settings(cx);
        cx.notify();
    }

    fn set_zoom(&mut self, zoom: usize, cx: &mut Context<Self>) {
        if self.zoom != zoom {
            self.zoom = zoom;
            self.icon_size_changed();
            self.save_settings(cx);
            cx.notify();
        }
    }

    /// Orders by `key`; choosing the current key again reverses the direction.
    fn sort_by(&mut self, key: SortKey, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let options = model.options();
        let options = ViewOptions {
            sort: key,
            descending: options.sort == key && !options.descending,
            ..options
        };
        if let Some(generation) = model.set_options(options) {
            self.generation = generation;
            self.save_settings(cx);
            cx.notify();
        }
    }

    fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            self.set_hidden(!model.options().show_hidden, cx);
        }
    }

    /// Dolphin's folders-first setting: folders above files, or mixed in.
    fn toggle_folders_first(&mut self, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let options = model.options();
        let folders_first = !options.folders_first;
        if let Some(generation) = model.set_options(ViewOptions {
            folders_first,
            ..options
        }) {
            self.generation = generation;
            self.save_settings(cx);
            let text = if folders_first {
                "Folders are listed before files."
            } else {
                "Folders are sorted together with files."
            };
            hub(cx).update(cx, |hub, cx| hub.say(text.to_owned(), false, cx));
            cx.notify();
        }
    }

    fn show_hidden(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_hidden(true, cx);
        self.focus.focus(window, cx);
    }

    fn set_hidden(&mut self, show_hidden: bool, cx: &mut Context<Self>) {
        let Some(model) = &mut self.model else { return };
        let options = ViewOptions {
            show_hidden,
            ..model.options()
        };
        if let Some(generation) = model.set_options(options) {
            self.generation = generation;
            self.folder_state_scroll.set_offset(point(px(0.), px(0.)));
            self.save_settings(cx);
            cx.notify();
        }
    }

    /// Reads the filesystem through `admin` (KDE's administrator helper,
    /// W-098) or, with `None`, as the user. A change starts a new model at
    /// the same folder; asking for what is already in use does nothing.
    pub(crate) fn set_filesystem(&mut self, admin: Option<Arc<dyn Fs>>, cx: &mut Context<Self>) {
        let same = match (&self.admin_fs, &admin) {
            (None, None) => true,
            (Some(current), Some(wanted)) => Arc::ptr_eq(current, wanted),
            _ => false,
        };
        if same {
            return;
        }
        let location = self.location();
        let options = self
            .model
            .as_ref()
            .map(DirectoryModel::options)
            .unwrap_or_default();
        self.admin_fs = admin.clone();
        let fs: Arc<dyn Fs> = admin.unwrap_or_else(|| Arc::new(RealFs));
        match DirectoryModel::new(fs) {
            Ok((mut model, events)) => {
                model.set_options(options);
                self.model = Some(model);
                self.model_epoch += 1;
                self.generation = 0;
                self.snapshot = None;
                self.visited = None;
                self.selection = Selection::default();
                self.selection_revision += 1;
                self.focus_row = None;
                self.failure = None;
                let epoch = self.model_epoch;
                cx.spawn(async move |view, cx| {
                    Self::receive_events(view, events, epoch, cx).await;
                })
                .detach();
                if let Some(location) = location {
                    self.open(location);
                }
            }
            Err(error) => {
                tracing::error!(%error, "could not start the directory model");
            }
        }
        cx.notify();
    }

    /// Tells the view whether it is one side of a split, and the active
    /// one: each side then shows its own path bar.
    pub fn set_split(&mut self, split: Option<bool>) {
        self.in_split = split;
    }

    /// Tells the view how wide it is drawn, so columns and the location bar
    /// adapt to a pane and not to the whole window.
    pub fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    /// Gives this view the keyboard focus.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
    }

    /// Whether this view, or a field inside it, has the keyboard focus.
    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }

    /// The view's mode, zoom and options, as they would be saved.
    pub fn settings(&self) -> Settings {
        let options = self
            .model
            .as_ref()
            .map(DirectoryModel::options)
            .unwrap_or_default();
        Settings::new(self.mode, self.zoom, options)
    }

    /// The folder being shown.
    pub fn location(&self) -> Option<PathBuf> {
        self.model.as_ref().and_then(DirectoryModel::location)
    }

    /// The remote folder this pane is waiting for, if the user has not gone
    /// elsewhere since asking.
    fn pending_remote(&self) -> Option<&RemoteAddress> {
        self.pending_remote
            .as_ref()
            .filter(|(_, from)| *from == self.location())
            .map(|(address, _)| address)
    }

    /// Mounts `address` through kio-fuse and shows it here, unless the user
    /// goes elsewhere first. A restored tab keeps waiting for its address
    /// after a failure, so it is still saved by that address.
    pub(crate) fn open_remote(
        &mut self,
        address: RemoteAddress,
        keep_on_failure: bool,
        cx: &mut Context<Self>,
    ) {
        self.pending_remote = Some((address.clone(), self.location()));
        remote::open(address.clone(), cx, move |view: &mut Self, mounted, cx| {
            if view.pending_remote() != Some(&address) {
                return;
            }
            match mounted {
                Some(path) => view.open(path),
                None if !keep_on_failure => view.pending_remote = None,
                None => {}
            }
            cx.notify();
        });
    }

    /// Where this pane is, for the saved session: a remote folder by its
    /// address, since kio-fuse mount paths do not outlive the run.
    pub(crate) fn saved_location(&self, cx: &App) -> Option<Location> {
        if let Some(address) = self.pending_remote() {
            return Some(Location::Remote(address.clone()));
        }
        Some(remote::location_of(self.location()?, cx))
    }

    /// Shows the location as editable text with keyboard focus.
    pub(crate) fn edit_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // As Dolphin's Ctrl+L: pressed again while the whole location is
        // selected in the focused field, it goes back to the list.
        let input = self.location_input.read(cx);
        if self.editing_location
            && input.focus_handle(cx).is_focused(window)
            && selects_everything(&input.selected_range(), input.value().len())
        {
            self.stop_editing(window, cx);
            return;
        }
        // A folder opened through kio-fuse is shown by its remote address,
        // as Dolphin shows it, rather than by its local mount path.
        let current = if self.shows_trash() {
            "trash:/".to_owned()
        } else {
            self.location()
                .map(|p| {
                    remote::address_of(&p, cx).unwrap_or_else(|| p.to_string_lossy().into_owned())
                })
                .unwrap_or_default()
        };
        self.editing_location = true;
        self.location_input.update(cx, |input, cx| {
            input.set_value(current, window, cx);
            // Selected, so typing a new location replaces it (Dolphin).
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn stop_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_location {
            self.editing_location = false;
            self.focus.focus(window, cx);
            cx.notify();
        }
    }

    /// Opens the filter bar and puts the keyboard in it; with the bar
    /// already focused, closes it and shows everything again (Ctrl+I toggles).
    fn show_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self
            .filter_input
            .read(cx)
            .focus_handle(cx)
            .is_focused(window);
        if self.filter_open && focused {
            self.close_filter(window, cx);
            return;
        }
        self.filter_open = true;
        self.filter_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// Clears the filter, hides the bar and returns to the list.
    fn close_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter_open = false;
        self.filter_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.apply_filter("", cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    fn apply_filter(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(model) = &mut self.model
            && let Some(generation) = model.set_filter(NameFilter::new(text))
        {
            self.generation = generation;
            cx.notify();
        }
    }

    fn on_filter_input(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                self.apply_filter(&text, cx);
            }
            // Back to the list, keeping the filter.
            InputEvent::PressEnter { .. } => {
                self.focus.focus(window, cx);
                cx.notify();
            }
            InputEvent::Blur | InputEvent::Focus => {}
        }
    }

    fn on_location_input(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                let typed = input.read(cx).value();
                let typed = typed.trim();
                if names_trash(typed) {
                    self.open_trash(cx);
                } else if let Some(address) = RemoteAddress::parse(typed) {
                    self.open_remote(address, false, cx);
                } else if let Some(path) = path_from_uri(typed.as_bytes())
                    .or_else(|| expand_path(typed, self.location().as_deref()))
                {
                    self.open(path);
                }
                self.stop_editing(window, cx);
            }
            InputEvent::Blur => self.stop_editing(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    /// Whether this view is showing the contents of a trash.
    pub fn shows_trash(&self) -> bool {
        self.current_snapshot().is_some() && self.trash_dir.is_some()
    }

    /// Finds out in the background whether `path` shows a trash directory.
    /// Reads how much space the folder's volume has left, in the background.
    fn measure_free_space(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.free_space = None;
        cx.spawn(async move |view, cx| {
            let folder = path.clone();
            let usage = cx
                .background_spawn(async move {
                    rustphin_platform::devices::usage(&folder).map(|(total, free)| {
                        (
                            total,
                            free,
                            rustphin_platform::devices::mount_point_of(&folder),
                        )
                    })
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                if let Some((total, free, mount)) = usage
                    && view.location().as_deref() == Some(path.as_path())
                {
                    view.free_space = Some((path, total, free, mount));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn detect_trash(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.trash_dir = None;
        self.other_trashes.clear();
        let Some(trash) = hub(cx).read(cx).trash() else {
            return;
        };
        cx.spawn(async move |view, cx| {
            let shown = path.clone();
            let (dir, others) = cx
                .background_spawn(async move {
                    let dir = trash.dir_showing(&shown);
                    let others = match &dir {
                        Some(dir) => trash
                            .dirs()
                            .into_iter()
                            .filter(|other| other != dir)
                            .collect(),
                        None => Vec::new(),
                    };
                    (dir, others)
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                // Only if the view is still where it was when this started.
                if view.location().as_deref() == Some(path.as_path()) {
                    view.trash_dir = dir;
                    view.other_trashes = others;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Shows `folder`.
    pub fn go_to(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        self.open(folder);
        cx.notify();
    }

    /// Shows `folder` and selects `names` once its listing is ready.
    /// Names that are not in the listing are left unselected.
    pub fn reveal(&mut self, folder: PathBuf, names: Vec<Vec<u8>>, cx: &mut Context<Self>) {
        let here = self.location().as_deref() == Some(folder.as_path());
        let ready = self
            .current_snapshot()
            .is_some_and(|snapshot| snapshot.path() == folder.as_path() && !snapshot.is_loading());
        if here && ready {
            if let Some(snapshot) = self.current_snapshot().cloned() {
                select_names(&mut self.selection, &snapshot, &names);
                self.selection_revision += 1;
                self.focus_row = self.selection.focus_row(&snapshot);
                self.reveal_focus();
            }
        } else {
            self.select_on_arrival = Some(names);
            if !here {
                self.open(folder);
            }
        }
        cx.notify();
    }

    /// Shows the trash.
    pub fn open_trash(&mut self, cx: &mut Context<Self>) {
        let Some(trash) = hub(cx).read(cx).trash() else {
            return;
        };
        cx.spawn(async move |view, cx| {
            let folder = cx
                .background_spawn(async move { trash.home_files_dir() })
                .await;
            let _ = view.update(cx, |view, cx| {
                match folder {
                    Ok(folder) => view.open(folder),
                    Err(error) => tracing::warn!(%error, "the trash could not be opened"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn restore_selected(&mut self, cx: &mut Context<Self>) {
        let contents = self.selected_paths();
        if let Some(dir) = self.trash_dir.clone()
            && !contents.is_empty()
        {
            hub(cx).update(cx, |hub, cx| hub.restore(dir, contents, cx));
        }
    }

    fn show_trash_actions(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.trash_dir.is_none() {
            return;
        }
        let entries = menu::trash_actions(
            !self.selection.is_empty(),
            self.other_trashes
                .iter()
                .map(|dir| (trash_label(dir), dir.join("files"))),
        );
        let Some(state) = menu::initial(&entries) else {
            return;
        };
        self.menu = Some(OpenMenu {
            at,
            above: None,
            entries,
            state,
            name: "Trash actions",
            scroll: ScrollHandle::new(),
        });
        self.menu_focus.focus(window, cx);
        cx.notify();
    }

    /// Explains this location and keeps its actions in one bounded menu.
    fn trash_banner(&self, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("trash-banner")
            .role(Role::Group)
            .aria_label("Trash")
            .aria_description("Items stay here until the trash is emptied.")
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(space::XS))
            .px(px(space::SM))
            .py(px(space::SM))
            .bg(theme.chrome)
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(space::SM))
                    .child(
                        div().flex().flex_none().items_center().gap(px(space::SM))
                            .child(Icon::new(gpui_kit::assets::IconName::Trash).size(px(size::ICON)).flex_none())
                            .child("Trash"),
                    )
                    .child(
                        div().flex_none().child(
                            Button::new("trash-actions")
                                .outlined()
                                .small()
                                .label("Trash actions")
                                .tooltip("Restore, empty or browse another trash; also available in Actions")
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    this.show_trash_actions(event.position(), window, cx);
                                })),
                        ),
                    ),
            )
            .child(
                div().text_size(px(text::SMALL)).text_color(theme.text_muted)
                    .child("Items stay here until the trash is emptied."),
            )
    }

    /// The selected entries, as full paths.
    pub(crate) fn selected_paths(&self) -> Vec<PathBuf> {
        self.current_snapshot().map_or_else(Vec::new, |snapshot| {
            self.selection
                .rows(snapshot)
                .into_iter()
                .map(|row| snapshot.entry_path(row))
                .collect()
        })
    }

    /// Starts renaming the focused entry in place.
    fn rename_focused(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Names in the trash belong to its records.
        if self.trash_dir.is_some() {
            return;
        }
        if let Some(snapshot) = self.current_snapshot() {
            let rows = self.selection.rows(snapshot);
            if rows.len() >= 2 {
                let selected: Vec<OsString> = rows
                    .iter()
                    .map(|row| OsStr::from_bytes(snapshot.name(*row)).to_owned())
                    .collect();
                let chosen: HashSet<Vec<u8>> = selected
                    .iter()
                    .map(|name| name.as_bytes().to_vec())
                    .collect();
                let staying = (0..snapshot.len())
                    .filter(|row| !chosen.contains(snapshot.name(*row)))
                    .map(|row| OsStr::from_bytes(snapshot.name(row)).to_owned())
                    .collect();
                cx.emit(BrowserEvent::BatchRename {
                    directory: snapshot.path().to_owned(),
                    selected,
                    staying,
                });
                return;
            }
        }
        let name = self
            .current_snapshot()
            .zip(self.focus_row)
            .filter(|(snapshot, row)| self.selection.contains(snapshot, *row))
            .map(|(snapshot, row)| snapshot.name(row).to_vec());
        if let Some(name) = name {
            self.begin_rename(name, window, cx);
        }
    }

    /// Shows the name field over the entry called `name`, with the part
    /// before the extension selected.
    fn begin_rename(&mut self, name: Vec<u8>, window: &mut Window, cx: &mut Context<Self>) {
        let Some((snapshot, row)) = self
            .current_snapshot()
            .and_then(|snapshot| Some((snapshot, snapshot.row_of(&name)?)))
        else {
            return;
        };
        let shown = display_name(&name).into_owned();
        let stem = operations::stem_len(&shown, snapshot.is_container(row));
        self.rename_input.update(cx, |input, cx| {
            input.set_value(shown, window, cx);
            input.set_selected_range(0..stem, cx);
            input.focus(window, cx);
        });
        self.renaming = Some(name);
        cx.notify();
    }

    /// Ends renaming. With `apply`, the typed name is given to the entry.
    fn finish_rename(&mut self, apply: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(old) = self.renaming.take() else {
            return;
        };
        let typed = self.rename_input.read(cx).value().to_string();
        let unchanged = typed.is_empty() || typed == display_name(&old);
        if apply
            && !unchanged
            && let Some(folder) = self.location()
        {
            self.select_when_seen = Some((typed.clone().into_bytes(), false));
            hub(cx).update(cx, |hub, _| {
                hub.start(Request::Rename {
                    path: folder.join(OsStr::from_bytes(&old)),
                    new_name: typed.into(),
                });
            });
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
        match event {
            // Leaving the field keeps what was typed, as pressing Enter does.
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                self.finish_rename(true, window, cx);
            }
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    /// Creates an empty folder or file with a free default name and starts
    /// renaming it, so the name can be typed straight away.
    fn create_entry(&mut self, folder: bool, cx: &mut Context<Self>) {
        let (Some(parent), Some(snapshot)) = (self.location(), self.current_snapshot()) else {
            return;
        };
        if self.trash_dir.is_some() {
            return;
        }
        let base = if folder { "New Folder" } else { "New File" };
        let name = operations::free_name(base, |name| snapshot.row_of(name.as_bytes()).is_some());
        self.select_when_seen = Some((name.clone().into_bytes(), true));
        let name = name.into();
        hub(cx).update(cx, |hub, _| {
            hub.start(if folder {
                Request::CreateDir { parent, name }
            } else {
                Request::CreateFile { parent, name }
            });
        });
    }

    fn recover_renames(&mut self, cx: &mut Context<Self>) {
        if self.trash_dir.is_some() {
            return;
        }
        if let Some(directory) = self.location() {
            hub(cx).update(cx, |hub, _| {
                hub.start(Request::RecoverRenameBatch { directory });
            });
        }
    }

    fn trash_selected(&mut self, cx: &mut Context<Self>) {
        // What is already in the trash can only be deleted for good, and
        // so can anything as administrator: KDE's helper has no trash.
        if self.trash_dir.is_some() || self.admin_fs.is_some() {
            return self.delete_selected(cx);
        }
        let paths = self.selected_paths();
        if !paths.is_empty() {
            hub(cx).update(cx, |hub, _| hub.start(Request::Trash { paths }));
        }
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        // The question lists each with its icon and size, from the listing.
        let listed = self.current_snapshot().map_or_else(Vec::new, |snapshot| {
            self.selection
                .rows(snapshot)
                .into_iter()
                .map(|row| {
                    let folder = snapshot.is_container(row);
                    operations::Listed {
                        folder,
                        size: (!folder)
                            .then(|| snapshot.metadata(row).map(|metadata| metadata.size))
                            .flatten(),
                    }
                })
                .collect()
        });
        hub(cx).update(cx, |hub, cx| hub.delete_after_confirming(paths, listed, cx));
    }

    fn pick_up_selected(&mut self, cut: bool, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        hub(cx).update(cx, |hub, cx| hub.pick_up(paths, cut, cx));
    }

    /// Copies the selection beside itself as "name copy.ext" (Ctrl+D,
    /// Dolphin's "Duplicate Here", D-021) and selects the copies once listed.
    fn duplicate_selected(&mut self, cx: &mut Context<Self>) {
        if self.trash_dir.is_some() {
            return;
        }
        let Some(snapshot) = self.current_snapshot() else {
            return;
        };
        let rows = self.selection.rows(snapshot);
        if rows.is_empty() {
            return;
        }
        let names: Vec<Vec<u8>> = rows
            .iter()
            .map(|&row| snapshot.name(row).to_vec())
            .collect();
        let sources = rows.iter().map(|&row| snapshot.entry_path(row)).collect();
        // Hidden or filtered entries still take a name: ask the snapshot's
        // full listing, not the rows on screen.
        self.select_all_when_seen =
            operations::duplicate_names(&names, |name| snapshot.is_listed(name));
        hub(cx).update(cx, |hub, _| hub.start(Request::Duplicate { sources }));
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(into) = self.location()
            && self.trash_dir.is_none()
        {
            hub(cx).update(cx, |hub, cx| hub.paste(into, cx));
        }
    }

    /// The parts of the window's toolbar that act on this pane (W-096): the
    /// window places them between its own buttons, across its whole width.
    pub(crate) fn toolbar_parts(
        &mut self,
        layout: ToolbarLayout,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarParts {
        let can_back = self.model.as_ref().is_some_and(DirectoryModel::can_go_back);
        let can_forward = self
            .model
            .as_ref()
            .is_some_and(DirectoryModel::can_go_forward);
        let location = self.location();
        let can_up = location.as_deref().and_then(Path::parent).is_some();
        let has_folder = self.current_snapshot().is_some();
        let show_hidden = self
            .model
            .as_ref()
            .is_some_and(|model| model.options().show_hidden);
        // Each button gives the keyboard back to this pane, so its keys
        // keep working after a click.
        let navigation = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::XS))
            .when(layout.back, |group| {
                group.child(
                    tool_button(
                        "back",
                        gpui_kit::assets::IconName::ArrowLeft,
                        "Back (Alt+Left)",
                        can_back,
                        None,
                        &theme,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigate(DirectoryModel::back, cx);
                        this.focus(window, cx);
                    })),
                )
            })
            .when(layout.forward_and_up, |group| {
                group
                    .child(
                        tool_button(
                            "forward",
                            gpui_kit::assets::IconName::ArrowRight,
                            "Forward (Alt+Right)",
                            can_forward,
                            None,
                            &theme,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(DirectoryModel::forward, cx);
                            this.focus(window, cx);
                        })),
                    )
                    .child(
                        tool_button(
                            "up",
                            gpui_kit::assets::IconName::ArrowUp,
                            "Up one folder (Alt+Up)",
                            can_up,
                            None,
                            &theme,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.go_up(cx);
                            this.focus(window, cx);
                        })),
                    )
            })
            .when(layout.tools, |group| {
                group.child(
                    tool_button(
                        "reload",
                        gpui_kit::assets::IconName::RefreshCw,
                        "Reload (F5)",
                        location.is_some(),
                        None,
                        &theme,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigate(DirectoryModel::reload, cx);
                        this.focus(window, cx);
                    })),
                )
            })
            .into_any_element();

        let shows_trash = self.shows_trash();
        // The field carries this pane's key context, so Escape and Ctrl+L
        // reach the pane while the path is being typed.
        let field = div()
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &CancelEdit, window, cx| {
                if this.editing_location {
                    this.stop_editing(window, cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &EditLocation, window, cx| this.edit_location(window, cx)),
            )
            .flex_1()
            .min_w_0()
            .font_family(text::FAMILY_DATA);
        let location_part = if self.editing_location {
            field.child(crate::input::named(
                Input::new(&self.location_input).min_h(px(size::TOOL_BUTTON)),
                &self.location_input,
                "Folder location",
                window,
                cx,
            ))
        } else {
            field.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .h(px(size::TOOL_BUTTON))
                    .px(px(space::MD))
                    .overflow_hidden()
                    .rounded(px(size::RADIUS))
                    .bg(theme.content)
                    .border_1()
                    .border_color(theme.border_strong)
                    .text_size(px(text::DATA))
                    .child(
                        Icon::new(if shows_trash {
                            gpui_kit::assets::IconName::Trash
                        } else {
                            gpui_kit::assets::IconName::Folder
                        })
                        .size(px(size::ICON))
                        .flex_none()
                        .text_color(theme.text_muted),
                    )
                    .child(div().flex_1().min_w_0().child(self.breadcrumbs(
                        location.as_deref(),
                        // The field spans the window less the toolbar's
                        // buttons, about what a pane beside Places had.
                        crumbs_for_width(layout.width - size::SIDEBAR),
                        theme,
                        cx,
                    ))),
            )
        }
        .into_any_element();

        let new_folder = layout.tools.then(|| {
            tool_button(
                "new-folder",
                gpui_kit::assets::IconName::FolderPlus,
                "New folder (F10)",
                has_folder && !shows_trash,
                None,
                &theme,
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.create_entry(true, cx);
                this.focus(window, cx);
            }))
            .into_any_element()
        });

        let view_mode = layout.tools.then(|| {
            let mode = self.mode;
            let segment = |id: &'static str, icon, label: &'static str, target: ViewMode| {
                tool_button(id, icon, label, true, Some(mode == target), &theme)
                    .rounded_none()
                    .w(px(size::TOOL_BUTTON - 2.))
                    .h(px(size::TOOL_BUTTON - 2.))
            };
            div()
                .id("view-mode")
                .role(Role::RadioGroup)
                .aria_label("View mode")
                .flex()
                .flex_none()
                .rounded(px(size::RADIUS))
                .border_1()
                .border_color(theme.border_strong)
                .overflow_hidden()
                .child(
                    segment(
                        "view-icons",
                        gpui_kit::assets::IconName::LayoutGrid,
                        "Icons (Ctrl+1)",
                        ViewMode::Icons,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.set_mode(ViewMode::Icons, cx);
                        this.focus(window, cx);
                    })),
                )
                .child(
                    segment(
                        "view-compact",
                        gpui_kit::assets::IconName::Columns3,
                        "Compact (Ctrl+2)",
                        ViewMode::Compact,
                    )
                    .border_l_1()
                    .border_color(theme.border_strong)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.set_mode(ViewMode::Compact, cx);
                        this.focus(window, cx);
                    })),
                )
                .child(
                    segment(
                        "view-details",
                        gpui_kit::assets::IconName::List,
                        "Details (Ctrl+3)",
                        ViewMode::Details,
                    )
                    .border_l_1()
                    .border_color(theme.border_strong)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.set_mode(ViewMode::Details, cx);
                        this.focus(window, cx);
                    })),
                )
                .into_any_element()
        });

        let toggles = layout.tools.then(|| {
            let filter_open = self.filter_open;
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(space::XS))
                .child(
                    tool_button(
                        "show-hidden",
                        gpui_kit::assets::IconName::Eye,
                        if show_hidden {
                            "Hide hidden files (Ctrl+H)"
                        } else {
                            "Show hidden files (Ctrl+H)"
                        },
                        true,
                        Some(show_hidden),
                        &theme,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_hidden(cx);
                        this.focus(window, cx);
                    })),
                )
                .child(
                    tool_button(
                        "filter",
                        gpui_kit::assets::IconName::Funnel,
                        if filter_open {
                            "Close the filter (Ctrl+I)"
                        } else {
                            "Filter by name (Ctrl+I)"
                        },
                        has_folder,
                        Some(filter_open),
                        &theme,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_filter(window, cx);
                    })),
                )
                .into_any_element()
        });

        let open_palette = |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            window.dispatch_action(Box::new(crate::workspace::OpenPalette), cx);
        };
        let actions = if layout.labelled_actions {
            Button::new("actions")
                .outlined()
                // The library pads a custom-sized button by a fifth of the
                // size: 50 gives the 10-pixel padding.
                .with_size(px(space::SM * 5. + space::XS * 2.5))
                .h(px(size::TOOL_BUTTON))
                .tooltip("Search actions and folders (Ctrl+Shift+P)")
                .accessibility_label("Actions (Ctrl+Shift+P)")
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(space::SM))
                        .text_size(px(text::BODY))
                        .child("Actions")
                        .child(
                            Icon::new(IconName::ChevronDown)
                                .size(px(size::ICON))
                                .text_color(theme.text_muted),
                        )
                        .when(layout.shortcut_hint, |button| {
                            button.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .h(px(text::LINE))
                                    .ml(px(space::XS))
                                    .pl(px(space::SM))
                                    .border_l_1()
                                    .border_color(theme.border_strong)
                                    .font_family(text::FAMILY_DATA)
                                    .text_size(px(text::SMALL))
                                    .text_color(theme.text_muted)
                                    .child("Ctrl+Shift+P"),
                            )
                        }),
                )
                .on_click(open_palette)
                .into_any_element()
        } else {
            tool_button(
                "actions",
                gpui_kit::assets::IconName::Menu,
                "Actions (Ctrl+Shift+P)",
                true,
                None,
                &theme,
            )
            .on_click(open_palette)
            .into_any_element()
        };

        ToolbarParts {
            navigation,
            location: location_part,
            new_folder,
            view_mode,
            toggles,
            actions,
        }
    }

    /// In split view, the pane's own location above its list, as Dolphin
    /// shows one per side (W-096).
    fn path_bar(&self, active: bool, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let location = self.location();
        div()
            .id("path-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::PATH_BAR))
            .px(px(space::MD))
            .bg(theme.chrome)
            .border_b_1()
            .border_color(theme.border)
            .font_family(text::FAMILY_DATA)
            .text_size(px(text::DATA))
            .text_color(if active { theme.text } else { theme.text_muted })
            // A click on the bar makes this the side the toolbar acts on.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.focus(window, cx)),
            )
            .child(
                Icon::new(if self.shows_trash() {
                    gpui_kit::assets::IconName::Trash
                } else {
                    gpui_kit::assets::IconName::Folder
                })
                .size(px(size::ICON_SMALL))
                .flex_none()
                .text_color(theme.text_muted),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    // No buttons beside it: the whole pane is for the path.
                    .child(self.breadcrumbs(
                        location.as_deref(),
                        crumbs_for_width(self.width + size::SIDEBAR),
                        theme,
                        cx,
                    )),
            )
    }

    /// The current location as clickable ancestors, at most `shown` of
    /// them. Clicking the free space after them switches to the editable
    /// path field.
    fn breadcrumbs(
        &self,
        location: Option<&Path>,
        shown: usize,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Trash shows as Places names it; Ctrl+L still edits the address.
        if self.shows_trash() {
            return div()
                .id("breadcrumbs")
                .role(Role::Group)
                .aria_label("Folder location")
                .flex()
                .items_center()
                .cursor_text()
                .on_click(cx.listener(|this, _, window, cx| this.edit_location(window, cx)))
                .child(div().text_color(theme.text).child("Trash"));
        }
        let ancestors = location
            .map(|path| {
                let remote = remote::root_of(path, cx);
                crumb_trail(
                    path,
                    remote
                        .as_ref()
                        .map(|(root, address)| (root.as_path(), address.as_str())),
                )
            })
            .unwrap_or_default();
        let last = ancestors.len().saturating_sub(1);
        let after_root: Vec<bool> = (0..ancestors.len())
            .map(|index| index > 0 && ancestors[index - 1].1 == "/")
            .collect();
        let first_shown = ancestors.len().saturating_sub(shown);

        div()
            .id("breadcrumbs")
            .role(Role::Group)
            .aria_label("Folder location")
            .flex()
            .items_center()
            .overflow_hidden()
            .rounded(px(size::RADIUS))
            .cursor_text()
            .on_click(cx.listener(|this, _, window, cx| this.edit_location(window, cx)))
            .when(first_shown > 0, |crumbs| {
                crumbs.child(div().text_color(theme.text_muted).child("\u{2026}"))
            })
            .children(ancestors.into_iter().enumerate().skip(first_shown).map(
                |(index, (path, label))| {
                    let label: SharedString = label.into();
                    let current = index == last;
                    // No separator first, or right after the "/" crumb.
                    let separated = index > 0 && (index == first_shown || !after_root[index]);
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .when(separated, |crumb| {
                            crumb.child(div().text_color(theme.text_muted).child("/"))
                        })
                        .child(
                            div()
                                .id(("crumb", index))
                                .role(Role::Button)
                                .aria_label(label.clone())
                                .rounded(px(size::RADIUS))
                                .cursor_pointer()
                                .text_color(theme.text)
                                .when(!current, |crumb| crumb.hover(|crumb| crumb.bg(theme.hover)))
                                .child(label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    if !current {
                                        this.open(path.clone());
                                        cx.notify();
                                    }
                                })),
                        )
                },
            ))
    }

    /// The failure to explain in place of the list, if any.
    fn failure_in_place(&self) -> Option<&Failure> {
        let location = self.location();
        let listed = self.current_snapshot().is_some();
        self.failure
            .as_ref()
            .filter(|failure| failure.in_place(location.as_deref(), listed))
    }

    /// Where a folder error shown in place can lead: back, else up.
    fn way_out(&self) -> Way {
        let Some(model) = &self.model else {
            return Way::Nowhere;
        };
        if model.can_go_back() {
            Way::Back
        } else if model.location().is_some_and(|path| path.parent().is_some()) {
            Way::Up
        } else {
            Way::Nowhere
        }
    }

    /// Mockup 28: a folder that could not be opened, said in place of its
    /// list, with the way back.
    fn failure_state(&self, failure: &Failure, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        const HEADING: &str = "Could not open this folder";
        let way = self.way_out();
        let explanation = failure.explanation(way);
        let icon = if failure.error == LoadError::PermissionDenied {
            gpui_kit::assets::IconName::Lock
        } else {
            gpui_kit::assets::IconName::FolderX
        };
        let button = match way {
            Way::Back => Some(
                crate::dialog::button_with_icon(
                    "folder-error-back",
                    gpui_kit::assets::IconName::ArrowLeft,
                    "Back (Alt+Left)",
                )
                .accessibility_label("Back to the previous folder")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.navigate(DirectoryModel::back, cx);
                })),
            ),
            Way::Up => Some(
                crate::dialog::button_with_icon(
                    "folder-error-up",
                    gpui_kit::assets::IconName::ArrowUp,
                    "Up (Alt+Up)",
                )
                .accessibility_label("Up to the folder that contains it")
                .on_click(cx.listener(|this, _, _, cx| this.go_up(cx))),
            ),
            Way::Nowhere => None,
        };
        div()
            .id("folder-state")
            .role(Role::Alert)
            .aria_label(HEADING)
            .aria_description(explanation.clone())
            .size_full()
            .flex()
            .flex_col()
            .p(px(space::LG))
            .items_center()
            .justify_center()
            .gap(px(space::SM))
            .text_color(theme.text_muted)
            .child(
                Icon::new(icon)
                    .size(px(size::ICON_LARGE))
                    .flex_none()
                    .mb(px(space::SM)),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_HEADING)
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .text_size(px(text::TITLE))
                    .line_height(px(text::TITLE_LINE))
                    .text_color(theme.text)
                    .child(HEADING),
            )
            .child(
                div()
                    .flex_none()
                    .max_w(px(size::STATE_TEXT_MAX))
                    .text_center()
                    .child(explanation),
            )
            .children(button.map(|button| button.primary().mt(px(space::MD))))
            .into_any_element()
    }

    fn failure_banner(
        &self,
        failure: &Failure,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // A 44-pixel banner under the toolbar: icon, one line of text (read in
        // full by assistive technology) and a dismiss button.
        let message = failure.message();
        div()
            .id("failure-banner")
            .role(Role::Alert)
            .aria_label(message.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::LINE))
            .mx(px(space::SM))
            .mt(px(space::SM))
            .pl(px(space::MD))
            .pr(px(space::SM))
            .rounded(px(size::RADIUS))
            .bg(theme.danger_surface)
            .border_1()
            .border_color(theme.danger_border)
            .text_color(theme.danger)
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(px(size::ICON))
                    .flex_none(),
            )
            .child(div().flex_1().min_w_0().truncate().child(message))
            .child(
                Button::new("dismiss-failure")
                    .ghost()
                    .with_size(px(size::SMALL_BUTTON))
                    .size(px(size::SMALL_BUTTON))
                    .child(
                        Icon::new(IconName::Close)
                            .size(px(size::ICON))
                            .text_color(theme.danger),
                    )
                    .tooltip("Dismiss")
                    .accessibility_label("Dismiss")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.failure = None;
                        cx.notify();
                    })),
            )
    }

    /// One clickable column heading. The active one shows its direction with
    /// an arrow as well as brighter text.
    fn heading(
        &self,
        label: &'static str,
        key: SortKey,
        options: ViewOptions,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let active = options.sort == key;
        div()
            .id(label)
            .role(Role::Button)
            .aria_label(format!("Sort by {label}"))
            .when(active, |heading| {
                heading.aria_description(if options.descending {
                    "Sorted descending"
                } else {
                    "Sorted ascending"
                })
            })
            .flex()
            .items_center()
            .gap(px(space::XS))
            .h_full()
            .px(px(space::SM))
            .cursor_pointer()
            .text_color(if active { theme.text } else { theme.text_muted })
            .hover(|heading| heading.text_color(theme.text))
            .child(label)
            .when(active, |heading| {
                heading.child(if options.descending {
                    "\u{2193}"
                } else {
                    "\u{2191}"
                })
            })
            .on_click(cx.listener(move |this, _, _, cx| this.sort_by(key, cx)))
    }

    /// What a row or cell needs to know about the entry it shows.
    fn entry(&mut self, rows: &Snapshot, row: usize, cx: &mut Context<Self>) -> Entry {
        let container = rows.is_container(row);
        // The entry being renamed shows the hover fill and the bar, so the
        // field reads as text on the row rather than on the selection.
        let renaming = self.renaming.as_deref() == Some(rows.name(row));
        let link = rows.kind(row) == EntryKind::Symlink;
        // A link looks like what it points to, as in Dolphin.
        let type_name = rows
            .link_target_name(row)
            .unwrap_or(rows.name(row))
            .to_vec();
        Entry {
            name: display_name(rows.name(row)).into_owned().into(),
            container,
            link,
            selected: self.selection.contains(rows, row) && !renaming,
            focused: self.focus_row == Some(row) || renaming,
            renaming,
            cut: self.has_cut && hub(cx).read(cx).is_cut(&rows.entry_path(row)),
            hidden: rows.is_hidden(row),
            info: self.type_info_for(&type_name, container, cx),
            thumb: self.thumbnail_for(rows, row, container, cx),
        }
    }

    /// The thumbnail of the entry at `row`, if it is an image, the icons are
    /// large enough to show one, and it is ready. Asking is what gets it made,
    /// so only entries that are drawn ever get one.
    fn thumbnail_for(
        &mut self,
        rows: &Snapshot,
        row: usize,
        container: bool,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Path>> {
        let edge = self.mode.layout(self.zoom).icon;
        // No previews as administrator: they would read every picture as
        // root (docs/project/ADMIN_MODE.md).
        if container
            || edge < MIN_THUMBNAIL_EDGE
            || self.trash_dir.is_some()
            || self.admin_fs.is_some()
        {
            return None;
        }
        let type_name = rows.link_target_name(row).unwrap_or(rows.name(row));
        let mime = self.types.as_ref()?.mime_type(type_name, false);
        if !thumbs::has_thumbnail(mime) {
            return None;
        }
        let size = rustphin_platform::thumbnails::ThumbSize::for_edge(edge as u32);
        let file = rows.entry_path(row);
        thumbs(cx).update(cx, |thumbs, _| thumbs.get(&file, size))
    }

    /// The name of an entry on one line, or the field to edit it while it is
    /// being renamed.
    fn name_cell(&self, entry: &Entry, theme: Theme, window: &Window, cx: &App) -> gpui_kit::Div {
        if entry.renaming {
            // Clicks in the field place the cursor; they must not reach the
            // row underneath or start a rubber band.
            // As wide as the name it holds, as in Dolphin, up to the space
            // the row has: the text, 4 pixels each side, the outline and room
            // for the caret.
            let value = self.rename_input.read(cx).value();
            let run = gpui_kit::TextRun {
                len: value.len(),
                font: gpui_kit::font(text::FAMILY_BODY),
                color: theme.text.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let text_width = window
                .text_system()
                .layout_line(&value, px(text::NAME), &[run], None)
                .width;
            let width = text_width + px(space::XS * 2. + size::HAIRLINE * 2. + space::XS);
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                // Solid and in the normal text colour, whatever the row's state.
                .w(width)
                .max_w_full()
                .rounded(px(size::RADIUS))
                .bg(theme.raised)
                .text_color(theme.text)
                .child(crate::input::named(
                    Input::new(&self.rename_input)
                        .xsmall()
                        .min_h(px(size::RENAME_FIELD))
                        .px(px(space::XS))
                        .text_size(px(text::NAME)),
                    &self.rename_input,
                    "New name",
                    window,
                    cx,
                ))
        } else {
            div()
                .truncate()
                .when(entry.link, |name| name.italic())
                .child(entry.name.clone())
        }
    }

    /// The entries a drag that started on `row` carries: the whole selection
    /// when the row is part of it, otherwise that row alone.
    fn drag_paths(&self, row: usize) -> Vec<PathBuf> {
        match self.current_snapshot() {
            Some(snapshot) if row < snapshot.len() => {
                if self.selection.contains(snapshot, row) {
                    self.selected_paths()
                } else {
                    vec![snapshot.entry_path(row)]
                }
            }
            _ => Vec::new(),
        }
    }

    /// Makes `handle` the part of an entry that can be dragged: within the
    /// application to copy or move, and out of the window to other applications.
    fn drag_source(
        handle: gpui_kit::Stateful<gpui_kit::Div>,
        row: usize,
        icon: DragIcon,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let dragged = DraggedEntries {
            source: cx.entity().downgrade(),
            row,
        };
        handle
            // Seen before the view's own handler: this press is the start of
            // a possible drag, not of a rubber band.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.press_on_entry = true),
            )
            .on_drag(dragged, move |dragged, _, _, cx| {
                let label = operations::what(&dragged.paths(cx));
                let icon = icon.clone();
                cx.new(|_| DragPreview {
                    label: label.into(),
                    icon,
                })
            })
            .external_drag_payload(|dragged: &DraggedEntries, _, cx| {
                let paths = dragged.paths(cx);
                let folders = dragged.folders(cx);
                (!paths.is_empty()).then(|| {
                    ExternalDragPayload::Files(FileDragPaths::new(paths.into_iter().zip(folders)))
                })
            })
    }

    /// Makes `element` accept dropped entries: into the folder at `row`, or
    /// into the folder shown when `row` is `None`. A folder under the pointer
    /// is highlighted while something is dragged over it.
    fn drop_target<E: InteractiveElement>(
        element: E,
        row: Option<usize>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> E {
        let highlight = move |style: gpui_kit::StyleRefinement| match row {
            Some(_) => style.bg(theme.hover),
            None => style,
        };
        let view = cx.entity().downgrade();
        element
            .drag_over::<DraggedEntries>(move |style, _, _, _| highlight(style))
            .drag_over::<ExternalPaths>(move |style, _, _, _| highlight(style))
            // Not `cx.listener`: the dragged entries are read from their source
            // view, which may be this one, and a view cannot be read while it
            // is being updated.
            .on_drop({
                let view = view.clone();
                move |dragged: &DraggedEntries, window, cx| {
                    let sources = dragged.paths(cx);
                    let _ = view.update(cx, |this, cx| this.drop_into(row, sources, window, cx));
                }
            })
            .on_drop(move |dropped: &ExternalPaths, window, cx| {
                let sources = dropped.paths().to_vec();
                let _ = view.update(cx, |this, cx| this.drop_into(row, sources, window, cx));
            })
    }

    /// Copies or moves `sources` into the folder at `row`, or into the folder
    /// shown. Ctrl copies, Shift moves; without either the user is asked.
    fn drop_into(
        &mut self,
        row: Option<usize>,
        sources: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.trash_dir.is_some() {
            return;
        }
        let into = match (row, self.current_snapshot()) {
            (Some(row), Some(snapshot)) if row < snapshot.len() => Some(snapshot.entry_path(row)),
            (Some(_), _) => None,
            (None, _) => self.location(),
        };
        let Some(into) = into else { return };
        let modifiers = window.modifiers();
        let action = if modifiers.control {
            Some(operations::DropAction::Copy)
        } else if modifiers.shift {
            Some(operations::DropAction::Move)
        } else {
            None
        };
        hub(cx).update(cx, |hub, cx| hub.drop_files(sources, into, action, cx));
    }

    /// A middle click on a folder asks for it in a new tab.
    fn entry_middle_click(
        row: usize,
        cx: &mut Context<Self>,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
            let folder = this
                .current_snapshot()
                .filter(|snapshot| row < snapshot.len() && snapshot.is_container(row))
                .map(|snapshot| snapshot.entry_path(row));
            if let Some(path) = folder {
                cx.emit(BrowserEvent::OpenInNewTab(path));
            }
        })
    }

    /// Click behaviour shared by every view: select, and open on a plain
    /// double click.
    fn entry_click(
        row: usize,
        cx: &mut Context<Self>,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, event: &ClickEvent, _, cx| {
            if std::mem::take(&mut this.ignore_click) {
                return;
            }
            let modifiers = event.modifiers();
            this.click_row(row, modifiers, cx);
            let plain = !modifiers.shift && !modifiers.control;
            if plain && event.click_count() >= 2 {
                this.activate_row(row, cx);
            }
        })
    }

    /// The selectable collection excludes sibling controls such as sort headings.
    fn entry_list(
        &self,
        snapshot: &Snapshot,
        list: impl IntoElement,
        scrollbar: Option<Scrollbar>,
    ) -> impl IntoElement {
        div()
            .id("file-list")
            .relative()
            .role(Role::ListBox)
            .aria_size_of_set(snapshot.len())
            .aria_label(format!(
                "Files in {}",
                display_name(snapshot.path().as_os_str().as_bytes())
            ))
            .a11y_synthetic_children(|builder| builder.parent_node().set_multiselectable())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(list)
            .children(scrollbar)
    }

    /// The details view: column headings over a virtualised list of entries.
    fn list(
        &self,
        snapshot: &Arc<Snapshot>,
        layout: Layout,
        columns: Columns,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let rows = Arc::clone(snapshot);
        let options = snapshot.options();
        let zone = self.zone.clone();
        // Headings line up with the cells: the same 8-pixel side padding
        // around the list and inside every column.
        let headings = div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(size::ROW))
            .px(px(space::SM))
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(text::BODY))
            .child(
                self.heading("Name", SortKey::Name, options, theme, cx)
                    .flex_1()
                    .min_w_0(),
            )
            .when(columns.size, |headings| {
                headings.child(
                    self.heading("Size", SortKey::Size, options, theme, cx)
                        .w(px(size::COLUMN_SIZE))
                        .justify_end(),
                )
            })
            .when(columns.modified, |headings| {
                headings.child(
                    self.heading("Modified", SortKey::Modified, options, theme, cx)
                        .w(px(size::COLUMN_MODIFIED)),
                )
            })
            .when(columns.kind, |headings| {
                headings.child(
                    self.heading("Type", SortKey::Type, options, theme, cx)
                        .w(px(size::COLUMN_TYPE)),
                )
            });

        let list = uniform_list(
            "entries",
            snapshot.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                let _rows = this.frames.task("visible_rows");
                range
                    .map(|row| {
                        let entry = this.entry(&rows, row, cx);
                        let metadata = rows.metadata(row);
                        let secondary = entry.secondary(theme);
                        // Folders show a dash: a folder's own size says nothing useful.
                        let size_text = if entry.container {
                            "\u{2014}".to_owned()
                        } else {
                            metadata
                                .map(|metadata| format::size(metadata.size))
                                .unwrap_or_default()
                        };
                        let modified_text = metadata
                            .map(|metadata| format::timestamp(metadata.modified, &zone))
                            .unwrap_or_default();
                        let description = entry
                            .info
                            .as_ref()
                            .map(|info| info.description.clone())
                            .unwrap_or_default();
                        // Every column has 8 pixels of padding on each side,
                        // as the headings do.
                        let line = entry
                            .surface(row, theme)
                            .flex()
                            .items_center()
                            .size_full()
                            .text_size(px(text::NAME))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .px(px(space::SM))
                                    .gap(px(space::MD))
                                    // Icon and name are what gets dragged; the rest
                                    // of the row is free for a rubber band.
                                    .child(Self::drag_source(
                                        div()
                                            .id(("entry-handle", row))
                                            .flex()
                                            .items_center()
                                            .gap(px(space::MD))
                                            .min_w_0()
                                            .when(entry.renaming, |handle| handle.flex_1())
                                            .child(entry.icon(layout.icon, theme))
                                            .child(
                                                this.name_cell(&entry, theme, window, cx).min_w_0(),
                                            ),
                                        row,
                                        entry.drag_icon(),
                                        cx,
                                    )),
                            )
                            .when(columns.size, |line| {
                                line.child(
                                    div()
                                        .w(px(size::COLUMN_SIZE))
                                        .flex_none()
                                        .px(px(space::SM))
                                        .text_right()
                                        .font_family(text::FAMILY_DATA)
                                        .text_size(px(text::DATA))
                                        .text_color(secondary)
                                        .when(!size_text.is_empty(), |cell| cell.child(size_text)),
                                )
                            })
                            .when(columns.modified, |line| {
                                line.child(
                                    div()
                                        .w(px(size::COLUMN_MODIFIED))
                                        .flex_none()
                                        .px(px(space::SM))
                                        .font_family(text::FAMILY_DATA)
                                        .text_size(px(text::DATA))
                                        .text_color(secondary)
                                        .when(!modified_text.is_empty(), |cell| {
                                            cell.child(modified_text)
                                        }),
                                )
                            })
                            .when(columns.kind, |line| {
                                line.child(
                                    div()
                                        .w(px(size::COLUMN_TYPE))
                                        .flex_none()
                                        .px(px(space::SM))
                                        .truncate()
                                        .font_family(text::FAMILY_DATA)
                                        .text_size(px(text::DATA))
                                        .text_color(secondary)
                                        .when(!description.is_empty(), |cell| {
                                            cell.child(description)
                                        }),
                                )
                            });
                        let line = entry
                            .accessible(&rows, row, cx)
                            .w_full()
                            .h(px(layout.row_height))
                            .px(px(space::SM))
                            .child(line)
                            .on_click(Self::entry_click(row, cx))
                            .on_mouse_down(MouseButton::Middle, Self::entry_middle_click(row, cx))
                            .on_mouse_down(MouseButton::Right, Self::entry_right_click(row, cx));
                        if entry.container {
                            Self::drop_target(line, Some(row), theme, cx)
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.scroll)
        .flex_1()
        .py(px(space::XS));

        div()
            .flex()
            .flex_1()
            .flex_col()
            .min_h_0()
            .child(headings)
            .child(self.entry_list(
                snapshot,
                list,
                Some(styled_scrollbar(Scrollbar::vertical(&self.scroll), theme)),
            ))
    }

    /// One entry of the icons or compact view: a large icon over its name, or
    /// a small icon beside it.
    fn grid_cell(
        &mut self,
        rows: &Snapshot,
        row: usize,
        layout: Layout,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let stacked = self.mode == ViewMode::Icons;
        let cell_width = layout.cell_width.unwrap_or(layout.row_height);
        let entry = self.entry(rows, row, cx);
        let icon = entry.icon(layout.icon, theme);
        let surface = entry.surface(row, theme).size_full();
        let cell = if stacked {
            // Icon above a centred name of up to two lines.
            surface
                .flex()
                .flex_col()
                .items_center()
                .gap(px(space::XS))
                .py(px(space::SM))
                .px(px(space::XS))
                .text_size(px(text::NAME))
                .child(icon)
                .child(if entry.renaming {
                    self.name_cell(&entry, theme, window, cx).w_full()
                } else {
                    div()
                        .w_full()
                        .text_center()
                        .line_clamp(2)
                        .when(entry.link, |name| name.italic())
                        .child(entry.name.clone())
                })
        } else {
            surface
                .flex()
                .items_center()
                .gap(px(space::MD))
                .px(px(space::SM))
                .text_size(px(text::NAME))
                .child(icon)
                .child(
                    self.name_cell(&entry, theme, window, cx)
                        .when(!entry.renaming, |name| name.flex_1())
                        .min_w_0(),
                )
        };
        let is_folder = entry.container;
        let drag_icon = entry.drag_icon();
        let cell = Self::drag_source(
            entry
                .accessible(rows, row, cx)
                .flex_none()
                .w(px(cell_width))
                .h(px(layout.row_height))
                // Icon cells keep half the 8-pixel gap on each side; compact
                // columns start at their left edge with the gap after them,
                // and their rows touch, as list rows do.
                .when(stacked, |cell| cell.p(px(size::CELL_GAP / 2.)))
                .when(!stacked, |cell| cell.pr(px(size::CELL_GAP)))
                .child(cell)
                .on_click(Self::entry_click(row, cx))
                .on_mouse_down(MouseButton::Middle, Self::entry_middle_click(row, cx))
                .on_mouse_down(MouseButton::Right, Self::entry_right_click(row, cx)),
            row,
            drag_icon,
            cx,
        );
        if is_folder {
            Self::drop_target(cell, Some(row), theme, cx)
        } else {
            cell
        }
    }

    /// The icons view: a virtualised grid, `per_row` entries to a line,
    /// scrolling up and down.
    fn grid(
        &self,
        snapshot: &Arc<Snapshot>,
        layout: Layout,
        per_row: usize,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let rows = Arc::clone(snapshot);
        let len = snapshot.len();
        let list = uniform_list(
            "entries",
            len.div_ceil(per_row),
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                let _rows = this.frames.task("visible_rows");
                range
                    .map(|line| {
                        let first = line * per_row;
                        let cells = (first..(first + per_row).min(len))
                            .map(|row| this.grid_cell(&rows, row, layout, theme, window, cx))
                            .collect::<Vec<_>>();
                        // With the cells' half gaps: 16 pixels at the sides
                        // and 12 above, as in docs/DESIGN.md.
                        div()
                            .flex()
                            .w_full()
                            .h(px(layout.row_height))
                            .px(px(space::MD))
                            .children(cells)
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.scroll)
        .flex_1()
        .py(px(space::SM));
        let scrollbar = styled_scrollbar(Scrollbar::vertical(&self.scroll), theme);
        self.entry_list(snapshot, list, Some(scrollbar))
    }

    /// The compact view: entries fill columns from top to bottom and the
    /// view scrolls sideways, as in Dolphin. Only the columns in view are
    /// built, so a folder of any size costs the same to draw.
    fn columns(
        &mut self,
        snapshot: &Arc<Snapshot>,
        grid: Grid,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let _rows = self.frames.task("visible_rows");
        let pitch = grid.layout.cell_width.unwrap_or(0.);
        let shown = -f32::from(self.columns_scroll.offset().x);
        let viewport = f32::from(self.columns_scroll.bounds().size.width).max(self.width);
        let visible = grid.columns_between(shown - space::SM, shown - space::SM + viewport);
        let mut columns = Vec::with_capacity(visible.len());
        for column in visible {
            let first = column * grid.lanes;
            let cells = (first..(first + grid.lanes).min(grid.len))
                .map(|row| self.grid_cell(snapshot, row, grid.layout, theme, window, cx))
                .collect::<Vec<_>>();
            columns.push(
                div()
                    .absolute()
                    .left(px(space::SM + column as f32 * pitch))
                    .top(px(space::XS))
                    .flex()
                    .flex_col()
                    .children(cells),
            );
        }
        let content = div()
            .relative()
            .h_full()
            .w(px(grid.width() + space::SM * 2.))
            .children(columns);
        // A vertical wheel scrolls sideways when only that axis scrolls.
        let scroller = div()
            .id("entries")
            .flex_1()
            .min_h_0()
            .overflow_x_scroll()
            .track_scroll(&self.columns_scroll)
            .child(content);
        let scrollbar = styled_scrollbar(Scrollbar::horizontal(&self.columns_scroll), theme);
        self.entry_list(snapshot, scroller, Some(scrollbar))
    }

    /// Shown instead of the list when it has no rows: says why and what to do.
    fn empty_state(&self, snapshot: &Snapshot, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        if let Some(filter) = snapshot.filter()
            && !snapshot.is_loading()
        {
            return self
                .filter_empty_state(filter.text(), snapshot, theme, cx)
                .into_any_element();
        }
        let hidden = snapshot.total();
        let (icon, heading) = if snapshot.is_loading() {
            (IconName::Loader, "Loading\u{2026}")
        } else {
            if hidden > 0 {
                (IconName::EyeOff, "This folder has only hidden items")
            } else {
                (IconName::FolderOpen, "This folder is empty")
            }
        };
        let hidden_message = if hidden > 0 && !snapshot.is_loading() {
            format!(
                "{} hidden {} not shown.",
                count(hidden),
                plural(hidden, "item is", "items are")
            )
        } else {
            String::new()
        };
        // Why, under the heading: what is happening or what can be done.
        let reason = if snapshot.is_loading() {
            format!(
                "Reading {}. Entries appear as soon as the first ones are read.",
                snapshot.path().display()
            )
        } else if hidden == 0 && self.trash_dir.is_none() {
            "Drop files here, or make a new folder with F10.".to_owned()
        } else {
            String::new()
        };
        let spoken_reason = if reason.is_empty() {
            String::new()
        } else {
            format!("{reason} ")
        };
        let content = div()
            .flex()
            .flex_col()
            .w_full()
            .min_h_full()
            .p(px(space::LG))
            .items_center()
            .justify_center()
            .gap(px(space::SM))
            .text_color(theme.text_muted)
            .child(
                Icon::new(icon)
                    .size(px(size::ICON_LARGE))
                    .flex_none()
                    .mb(px(space::SM)),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_HEADING)
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .text_size(px(text::TITLE))
                    .line_height(px(text::TITLE_LINE))
                    .text_color(theme.text)
                    .child(heading),
            )
            .when(!reason.is_empty(), |state| {
                state.child(
                    div()
                        .flex_none()
                        .max_w(px(size::STATE_TEXT_MAX))
                        .text_center()
                        .child(reason),
                )
            })
            .when(hidden > 0 && !snapshot.is_loading(), |state| {
                state
                    .child(div().flex_none().child(hidden_message.clone()))
                    .child(
                        div()
                            .flex_none()
                            .max_w_full()
                            .mt(px(space::MD))
                            .track_focus(&self.folder_action_focus)
                            .key_context(FOLDER_ACTION_CONTEXT)
                            .child(
                                crate::dialog::button_with_icon(
                                    "show-hidden",
                                    gpui_kit::assets::IconName::Eye,
                                    "Show hidden",
                                )
                                .primary()
                                .max_w_full()
                                .accessibility_label("Show hidden files")
                                .tooltip("Show hidden files (Ctrl+H)")
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.show_hidden(window, cx);
                                    },
                                )),
                            ),
                    )
            });

        div()
            .relative()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .id("folder-state")
                    .role(Role::Status)
                    .aria_label(heading)
                    .aria_description(format!("{spoken_reason}{hidden_message} Use arrows, Page Up/Down or Home/End to scroll this message.").trim().to_owned())
                    .a11y_synthetic_children(|builder| builder.parent_node().set_live(gpui_kit::accesskit::Live::Polite))
                    .size_full()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.folder_state_scroll)
                    .child(content),
            )
            .child(styled_scrollbar(
                Scrollbar::vertical(&self.folder_state_scroll),
                theme,
            ))
            .into_any_element()
    }

    /// What an empty view says when the filter hides everything.
    fn filter_empty_state(
        &self,
        text: &str,
        snapshot: &Snapshot,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let heading = format!("Nothing matches \u{201c}{text}\u{201d}");
        let all = snapshot.unfiltered_len();
        let advice = format!(
            "Change the filter, or clear it to see all {} {}.",
            count(all),
            plural(all, "item", "items")
        );
        div()
            .id("folder-state")
            .role(Role::Status)
            .aria_label(heading.clone())
            .size_full()
            .flex()
            .flex_col()
            .p(px(space::LG))
            .items_center()
            .justify_center()
            .gap(px(space::SM))
            .text_color(theme.text_muted)
            .child(
                Icon::new(gpui_kit::assets::IconName::SearchX)
                    .size(px(size::ICON_LARGE))
                    .flex_none()
                    .mb(px(space::SM)),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_HEADING)
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .text_size(px(text::TITLE))
                    .line_height(px(text::TITLE_LINE))
                    .text_color(theme.text)
                    .child(heading),
            )
            .child(
                div()
                    .flex_none()
                    .max_w(px(size::STATE_TEXT_MAX))
                    .text_center()
                    .child(advice),
            )
            .child(
                crate::dialog::button_with_icon(
                    "clear-filter",
                    gpui_kit::assets::IconName::FunnelX,
                    "Clear filter",
                )
                .primary()
                .mt(px(space::MD))
                .tooltip("Clear the filter (Esc in the filter bar)")
                .on_click(cx.listener(|this, _, window, cx| this.close_filter(window, cx))),
            )
    }

    /// Empty views scroll their explanation instead of moving a file selection.
    fn scroll_folder_state(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        if self.editing_location
            || self.renaming.is_some()
            || self.menu.is_some()
            || !self
                .current_snapshot()
                .is_some_and(|snapshot| snapshot.is_empty())
        {
            return false;
        }
        let mut offset = self.folder_state_scroll.offset();
        let Some(y) = layout::scroll_offset(
            key,
            f32::from(offset.y),
            f32::from(self.folder_state_scroll.max_offset().y),
            f32::from(self.folder_state_scroll.bounds().size.height),
        ) else {
            return false;
        };
        offset.y = px(y);
        self.folder_state_scroll.set_offset(offset);
        cx.notify();
        true
    }

    /// The sizes the status line shows for `snapshot`, worked out once per
    /// snapshot and selection: summing half a million rows is not for every
    /// frame.
    fn status_sizes(&self, snapshot: &Arc<Snapshot>) -> StatusSizes {
        let key = (
            Arc::as_ptr(snapshot) as usize,
            snapshot.generation(),
            self.selection.revision(),
        );
        if let Some(cached) = *self.status_sizes.borrow()
            && cached.key == key
        {
            return cached;
        }
        let _task = self.frames.task("status_sizes");
        // Files only: a folder's own size says nothing useful.
        let sum = |rows: &mut dyn Iterator<Item = usize>| {
            rows.filter(|&row| !snapshot.is_container(row))
                .filter_map(|row| snapshot.metadata(row))
                .map(|metadata| metadata.size)
                .reduce(u64::saturating_add)
        };
        let sizes = StatusSizes {
            key,
            shown: sum(&mut (0..snapshot.len())),
            selected: if self.selection.is_empty() {
                None
            } else {
                sum(&mut self.selection.rows(snapshot).into_iter())
            },
        };
        *self.status_sizes.borrow_mut() = Some(sizes);
        sizes
    }

    fn status(&self, theme: Theme) -> impl IntoElement {
        let renaming_name = self
            .renaming
            .as_deref()
            .map(|name| display_name(name).into_owned());
        let (parts, selected) = match self.current_snapshot() {
            None => (
                StatusParts {
                    selection: None,
                    total: self
                        .failure_in_place()
                        .map_or("Loading\u{2026}", Failure::status)
                        .to_owned(),
                },
                false,
            ),
            Some(snapshot) => {
                let sizes = self.status_sizes(snapshot);
                let facts = StatusFacts {
                    shown: snapshot.len(),
                    unfiltered: snapshot.unfiltered_len(),
                    total: snapshot.total(),
                    selected: self.selection.len(),
                    filter: snapshot.filter().map(NameFilter::text),
                    loading: snapshot.is_loading(),
                    shown_size: sizes.shown,
                    selected_size: sizes.selected,
                    renaming: renaming_name.as_deref(),
                };
                (status_parts(&facts), facts.selected > 0)
            }
        };
        let zones = StatusZones::for_width(self.width, selected);
        let free = self
            .free_space
            .as_ref()
            .filter(|(path, ..)| self.location().as_deref() == Some(path.as_path()))
            .map(|(_, total, free, _)| free_text(*total, *free, self.volume.as_deref()));
        // Read in full by assistive technology, whatever fits on screen.
        let spoken = [
            parts.selection.clone(),
            Some(parts.total.clone()),
            free.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        let divider = || {
            div()
                .flex_none()
                .w(px(size::HAIRLINE))
                .h(px(space::LG))
                .bg(theme.border_strong)
        };
        let part = |text: String| div().min_w_0().truncate().child(text);
        div()
            .id("file-status")
            .role(Role::Label)
            .aria_value(spoken)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::STATUS))
            .px(px(space::MD))
            .bg(theme.chrome)
            .border_t_1()
            .border_color(theme.border)
            .font_family(text::FAMILY_DATA)
            .text_size(px(text::SMALL))
            .text_color(theme.text_muted)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(space::MD))
                    .overflow_hidden()
                    .when_some(parts.selection, |zone, selection| {
                        zone.child(part(selection).flex_none())
                            .when(zones.totals, |zone| zone.child(divider()))
                    })
                    .when(zones.totals, |zone| zone.child(part(parts.total))),
            )
            .when_some(free.filter(|_| zones.free_space), |line, free| {
                line.child(div().flex_none().child(free))
            })
    }

    /// The filter bar: an icon, the field and a close button.
    fn filter_bar(&self, theme: Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("filter-bar")
            .role(Role::Search)
            .aria_label("Filter by name")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::FILTER_BAR))
            .pl(px(space::MD))
            .pr(px(space::SM))
            .bg(theme.chrome)
            .border_t_1()
            .border_color(theme.border)
            .child(
                Icon::new(gpui_kit::assets::IconName::Funnel)
                    .size(px(size::ICON))
                    .flex_none()
                    .text_color(theme.text_muted),
            )
            // At most 360 pixels wide, as in the mockup.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .max_w(px(size::FILTER_FIELD_MAX))
                    .child(crate::input::named(
                        // 8 pixels inside, as the design's small field.
                        Input::new(&self.filter_input)
                            .min_h(px(size::FILTER_FIELD))
                            .px(px(space::SM)),
                        &self.filter_input,
                        "Filter by name",
                        window,
                        cx,
                    )),
            )
            .child(
                Button::new("close-filter")
                    .ghost()
                    .with_size(px(size::SMALL_BUTTON))
                    .size(px(size::SMALL_BUTTON))
                    .child(Icon::new(IconName::Close).size(px(size::ICON)))
                    .accessibility_label("Close the filter")
                    .tooltip("Close the filter (Esc)")
                    .on_click(cx.listener(|this, _, window, cx| this.close_filter(window, cx))),
            )
            .into_any_element()
    }

    /// The open context menu, in window coordinates, or nothing.
    fn menu_layer(
        &self,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let menu = self.menu.as_ref()?;
        let at = menu.at;
        let max_height = px(size::MENU_VISIBLE
            .min((f32::from(window.viewport_size().height) - space::SM * 2.).max(0.)));
        let entries = menu.entries.clone();
        let state = menu.state;
        let panel = self
            .menu_column(&entries, state.cursor, false, "menu-column", theme, cx)
            .track_scroll(&menu.scroll)
            .max_h(max_height);
        let submenu = state.submenu.then(|| {
            entries.get(state.cursor).and_then(|entry| match entry {
                menu::MenuEntry::Submenu { items, .. } => Some(
                    self.menu_column(items, state.sub_cursor, true, "menu-apps", theme, cx)
                        .mt(px(crate::menu_view::submenu_offset(&entries, state.cursor)))
                        .max_h(max_height),
                ),
                _ => None,
            })
        });
        // Open upward from the entry when the menu does not fit below it.
        let viewport = f32::from(window.viewport_size().height);
        let height = menu::column_height(
            &entries,
            size::MENU_ROW,
            1. + 2. * space::XS,
            2. * space::XS + 2.,
        )
        .min(f32::from(max_height));
        let (corner, at) = match menu.above {
            Some(above)
                if menu::opens_upward(
                    height,
                    viewport - space::SM - f32::from(at.y),
                    f32::from(above.y) - space::SM,
                ) =>
            {
                (gpui_kit::Anchor::BottomLeft, above)
            }
            _ => (gpui_kit::Anchor::TopLeft, at),
        };
        // A menu belongs to the window even when its browser pane is narrow.
        Some(
            deferred(
                anchored()
                    .anchor(corner)
                    .position(at)
                    .snap_to_window_with_margin(Edges::all(px(space::SM)))
                    .child(
                        div()
                            .id("context-menu")
                            .role(Role::Menu)
                            .aria_label(menu.name)
                            .aria_description(
                                "Up and Down choose an action. Enter runs it; Escape closes.",
                            )
                            .key_context(crate::menu_view::CONTEXT)
                            .track_focus(&self.menu_focus)
                            .on_action(cx.listener(|this, _: &MenuUp, window, cx| {
                                this.on_menu_key(MenuKey::Up, window, cx);
                            }))
                            .on_action(cx.listener(|this, _: &MenuDown, window, cx| {
                                this.on_menu_key(MenuKey::Down, window, cx);
                            }))
                            .on_action(cx.listener(|this, _: &MenuLeft, window, cx| {
                                this.on_menu_key(MenuKey::Left, window, cx);
                            }))
                            .on_action(cx.listener(|this, _: &MenuRight, window, cx| {
                                this.on_menu_key(MenuKey::Right, window, cx);
                            }))
                            .on_action(cx.listener(|this, _: &MenuAccept, window, cx| {
                                this.on_menu_key(MenuKey::Accept, window, cx);
                            }))
                            .on_action(cx.listener(|this, _: &MenuClose, window, cx| {
                                this.on_menu_key(MenuKey::Close, window, cx);
                            }))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                            .occlude()
                            .flex()
                            .items_start()
                            .child(panel)
                            .children(submenu.flatten()),
                    ),
            )
            .into_any_element(),
        )
    }

    /// One column of menu rows. `nested` is the application list.
    fn menu_column(
        &self,
        entries: &[menu::MenuEntry],
        cursor: usize,
        nested: bool,
        column: &'static str,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let rows = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| self.menu_row(entry, index, cursor, nested, theme, cx));
        crate::menu_view::column(column, theme)
            .when(nested, |column| {
                column.role(Role::Menu).aria_label("Open with applications")
            })
            .children(rows)
    }

    fn menu_row(
        &self,
        entry: &menu::MenuEntry,
        index: usize,
        cursor: usize,
        nested: bool,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let current = index == cursor;
        let active = current
            && self
                .menu
                .as_ref()
                .is_some_and(|menu| menu.state.submenu == nested);
        match entry {
            menu::MenuEntry::Separator => div()
                .h(px(size::HAIRLINE))
                .my(px(space::XS))
                .bg(theme.border)
                .into_any_element(),
            menu::MenuEntry::Submenu { label, .. } => crate::menu_view::row(
                label,
                menu::RowIcon::Symbol(gpui_kit::assets::IconName::AppWindow),
                None,
                true,
                true,
                current,
                theme,
            )
            .aria_expanded(current && self.menu.as_ref().is_some_and(|menu| menu.state.submenu))
            .aria_description(
                if current && self.menu.as_ref().is_some_and(|menu| menu.state.submenu) {
                    "Expanded application submenu. Left returns to File actions."
                } else {
                    "Application submenu. Right or activate to open."
                },
            )
            .when(active, |row| row.aria_active_descendant())
            .on_a11y_action(
                AccessibleAction::Focus,
                Self::accessible_menu_action(index, nested, false, cx),
            )
            .on_a11y_action(
                AccessibleAction::Click,
                Self::accessible_menu_action(index, nested, true, cx),
            )
            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                this.hover_menu(index, true, 0, cx);
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.on_menu_key(MenuKey::Right, window, cx);
            }))
            .into_any_element(),
            menu::MenuEntry::Item {
                label,
                command,
                enabled,
            } => {
                let command = command.clone();
                let enabled = *enabled;
                crate::menu_view::row(
                    label,
                    command.row_icon(),
                    command.shortcut(),
                    enabled,
                    false,
                    current && enabled,
                    theme,
                )
                .when(active && enabled, |row| row.aria_active_descendant())
                .when(enabled, |row| {
                    row.on_a11y_action(
                        AccessibleAction::Focus,
                        Self::accessible_menu_action(index, nested, false, cx),
                    )
                    .on_a11y_action(
                        AccessibleAction::Click,
                        Self::accessible_menu_action(index, nested, true, cx),
                    )
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if nested {
                            let cursor = this
                                .menu
                                .as_ref()
                                .map(|menu| menu.state.cursor)
                                .unwrap_or(0);
                            this.hover_menu(cursor, true, index, cx);
                        } else {
                            this.hover_menu(index, false, 0, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.run_menu(command.clone(), window, cx);
                    }))
                })
                .into_any_element()
            }
        }
    }

    fn accessible_menu_action(
        index: usize,
        nested: bool,
        activate: bool,
        cx: &Context<Self>,
    ) -> impl FnMut(Option<&gpui_kit::accesskit::ActionData>, &mut Window, &mut App) + use<> {
        let view = cx.entity().downgrade();
        move |_, window, cx| {
            let _ = view.update(cx, |this, cx| {
                let Some(menu) = &this.menu else {
                    return;
                };
                let cursor = if nested { menu.state.cursor } else { index };
                this.hover_menu(cursor, nested, index, cx);
                this.menu_focus.focus(window, cx);
                if activate {
                    this.on_menu_key(MenuKey::Accept, window, cx);
                }
            });
        }
    }
}

/// The menu opened for a selection or for empty space.
struct OpenMenu {
    at: Point<Pixels>,
    /// For a menu opened at an entry by keyboard: the entry's top-left,
    /// where the menu ends when it opens upward.
    above: Option<Point<Pixels>>,
    entries: Vec<menu::MenuEntry>,
    state: menu::MenuState,
    name: &'static str,
    scroll: ScrollHandle,
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frame_started = self.frames.begin(window.refresh_interval(cx));
        let theme = *cx.global::<Theme>();
        // Opening another folder clears the filter in the model; the bar follows.
        if self.filter_open
            && self
                .model
                .as_ref()
                .is_some_and(|model| model.filter().is_none())
            && !self.filter_input.read(cx).value().is_empty()
        {
            self.filter_open = false;
            self.filter_input
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        let snapshot = self.current_snapshot().cloned();
        if let Some(snapshot) = snapshot.as_ref().filter(|snapshot| !snapshot.is_empty()) {
            self.frames.entries_shown(snapshot.path());
        }
        // A pane in a split is narrower than the window; whatever contains
        // this view says how wide it is.
        if self.width <= 0. {
            self.width = f32::from(window.viewport_size().width);
        }
        let width = self.width;
        let columns = Columns::for_width(width);
        let layout = self.mode.layout(self.zoom);
        let len = snapshot.as_ref().map_or(0, |snapshot| snapshot.len());
        self.lanes = match self.mode.flow() {
            // Cells fill the width inside the list's side gutters.
            Flow::Across => layout.columns(width - space::SM * 2.),
            // Rows fill the height inside the top and bottom padding.
            Flow::Down => layout.rows(self.fit_columns(window, cx) - space::XS * 2.),
        };
        let per_row = self.lanes;
        let grid = Grid::new(layout, self.mode.flow(), self.lanes, len);
        if self.mode == ViewMode::Compact {
            self.follow_columns_folder(snapshot.as_deref(), grid);
        }
        if let Some(line) = self.frames.next_bench_row(grid.lines()) {
            match grid.flow {
                Flow::Across => self.scroll.scroll_to_item(line, ScrollStrategy::Top),
                Flow::Down => {
                    let pitch = layout.cell_width.unwrap_or(0.);
                    self.columns_scroll
                        .set_offset(point(px(-(line as f32) * pitch), px(0.)));
                }
            }
            window.request_animation_frame();
        }
        let failed_here = self
            .failure_in_place()
            .map(|failure| self.failure_state(failure, theme, cx));
        let compact = match &snapshot {
            Some(snapshot) if !snapshot.is_empty() && self.mode == ViewMode::Compact => Some(
                self.columns(snapshot, grid, theme, window, cx)
                    .into_any_element(),
            ),
            _ => None,
        };

        self.has_cut = self
            .location()
            .is_some_and(|folder| hub(cx).read(cx).has_cut_in(&folder));
        if std::mem::take(&mut self.refocus) {
            cx.defer_in(window, |this, window, cx| this.focus.focus(window, cx));
        }
        // An entry this view created has appeared: let its name be typed.
        if let Some(name) = self.rename_requested.take() {
            cx.defer_in(window, |this, window, cx| {
                this.begin_rename(name, window, cx)
            });
        }

        let view = div()
            .id("file-browser")
            .role(Role::Group)
            .aria_label("File browser")
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SelectPrevious, _, cx| {
                if !this.scroll_folder_state("up", cx) {
                    this.move_selection(Step::Up, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                if !this.scroll_folder_state("down", cx) {
                    this.move_selection(Step::Down, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectFirst, _, cx| {
                if !this.scroll_folder_state("home", cx) {
                    this.move_selection(Step::First, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectLast, _, cx| {
                if !this.scroll_folder_state("end", cx) {
                    this.move_selection(Step::Last, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PageUp, _, cx| {
                if !this.scroll_folder_state("pageup", cx) {
                    this.move_selection(Step::PageUp, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PageDown, _, cx| {
                if !this.scroll_folder_state("pagedown", cx) {
                    this.move_selection(Step::PageDown, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ExtendPrevious, _, cx| {
                this.move_selection(Step::Up, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendNext, _, cx| {
                this.move_selection(Step::Down, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendFirst, _, cx| {
                this.move_selection(Step::First, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendLast, _, cx| {
                this.move_selection(Step::Last, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageUp, _, cx| {
                this.move_selection(Step::PageUp, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageDown, _, cx| {
                this.move_selection(Step::PageDown, true, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| {
                this.move_selection(Step::Left, false, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| {
                this.move_selection(Step::Right, false, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendLeft, _, cx| {
                this.move_selection(Step::Left, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendRight, _, cx| {
                this.move_selection(Step::Right, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| {
                this.set_zoom(layout::step_zoom(this.zoom, 1), cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| {
                this.set_zoom(layout::step_zoom(this.zoom, -1), cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomReset, _, cx| {
                this.set_zoom(layout::DEFAULT_ZOOM, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowIcons, _, cx| {
                this.set_mode(ViewMode::Icons, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowCompact, _, cx| {
                this.set_mode(ViewMode::Compact, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowDetails, _, cx| {
                this.set_mode(ViewMode::Details, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                this.change_selection(Selection::select_all, cx);
            }))
            .on_action(cx.listener(|this, _: &InvertSelection, _, cx| {
                this.change_selection(Selection::invert, cx);
            }))
            .on_action(cx.listener(|this, _: &Activate, _, cx| {
                if let Some(row) = this.focus_row {
                    this.activate_row(row, cx);
                }
            }))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &GoUp, _, cx| this.go_up(cx)))
            .on_action(cx.listener(|this, _: &CopyLocation, _, cx| this.copy_location(cx)))
            .on_action(cx.listener(|this, _: &Duplicate, _, cx| this.duplicate_selected(cx)))
            .on_action(cx.listener(|this, _: &GoHome, _, cx| {
                if let Some(home) = rustphin_platform::xdg::home() {
                    this.go_to(home, cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &GoBack, _, cx| this.navigate(DirectoryModel::back, cx)),
            )
            .on_action(
                cx.listener(|this, _: &GoForward, _, cx| {
                    this.navigate(DirectoryModel::forward, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &Reload, _, cx| this.navigate(DirectoryModel::reload, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleHidden, _, cx| this.toggle_hidden(cx)))
            .on_action(cx.listener(|this, _: &ToggleFoldersFirst, _, cx| {
                this.toggle_folders_first(cx);
            }))
            .on_action(cx.listener(|this, _: &ShowHidden, window, cx| this.show_hidden(window, cx)))
            .on_action(cx.listener(|this, _: &OpenTrash, _, cx| this.open_trash(cx)))
            .on_action(cx.listener(|this, _: &RestoreTrash, _, cx| this.restore_selected(cx)))
            .on_action(cx.listener(|this, _: &TrashActions, window, cx| {
                this.show_trash_actions(this.menu_anchor(None), window, cx);
            }))
            .on_action(cx.listener(|this, _: &EmptyTrash, _, cx| {
                if let Some(dir) = this.trash_dir.clone().filter(|_| this.shows_trash()) {
                    hub(cx).update(cx, |hub, cx| hub.empty_trash(dir, cx));
                }
            }))
            .on_action(cx.listener(|this, _: &SortName, _, cx| this.sort_by(SortKey::Name, cx)))
            .on_action(cx.listener(|this, _: &SortSize, _, cx| this.sort_by(SortKey::Size, cx)))
            .on_action(
                cx.listener(|this, _: &SortModified, _, cx| this.sort_by(SortKey::Modified, cx)),
            )
            .on_action(cx.listener(|this, _: &SortType, _, cx| this.sort_by(SortKey::Type, cx)))
            .on_action(
                cx.listener(|this, _: &EditLocation, window, cx| this.edit_location(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Rename, window, cx| {
                this.rename_focused(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NewFolder, _, cx| this.create_entry(true, cx)))
            .on_action(cx.listener(|this, _: &NewFile, _, cx| this.create_entry(false, cx)))
            .on_action(cx.listener(|this, _: &RecoverRenames, _, cx| this.recover_renames(cx)))
            .on_action(cx.listener(|this, _: &MoveToTrash, _, cx| this.trash_selected(cx)))
            .on_action(cx.listener(|this, _: &DeletePermanently, _, cx| {
                this.delete_selected(cx);
            }))
            .on_action(cx.listener(|this, _: &ShowProperties, window, cx| {
                this.open_properties(window, cx);
            }))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.pick_up_selected(false, cx)))
            .on_action(cx.listener(|this, _: &Cut, _, cx| this.pick_up_selected(true, cx)))
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &OpenContextMenu, window, cx| {
                this.open_selection_menu(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowFilter, window, cx| {
                this.show_filter(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CancelEdit, window, cx| {
                let in_filter = this
                    .filter_input
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window);
                if this.renaming.is_some() {
                    this.finish_rename(false, window, cx);
                } else if this.editing_location {
                    this.stop_editing(window, cx);
                } else if this.filter_open && in_filter {
                    this.close_filter(window, cx);
                } else {
                    this.change_selection(|selection, _| selection.clear(), cx);
                }
            }))
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .text_color(theme.text)
            .font_family(text::FAMILY_BODY)
            .text_size(px(text::BODY))
            .line_height(px(text::LINE))
            .when_some(self.in_split, |view, active| {
                view.child(self.path_bar(active, theme, cx))
            })
            .when_some(
                self.failure.as_ref().filter(|_| failed_here.is_none()),
                |view, failure| view.child(self.failure_banner(failure, theme, cx)),
            )
            .when(self.trash_dir.is_some() && snapshot.is_some(), |view| {
                view.child(self.trash_banner(theme, cx))
            })
            .child(
                // The file list sits on a more solid surface than the glass
                // toolbar and status line, so columns of text stay readable.
                // Dropping on the view itself puts the entries in this folder.
                Self::drop_target(div(), None, theme, cx)
                    .flex()
                    .flex_1()
                    .flex_col()
                    .min_h_0()
                    .bg(theme.content)
                    .on_mouse_down(MouseButton::Right, cx.listener(Self::on_background_menu))
                    .map(|content| match &snapshot {
                        _ if compact.is_some() => content.children(compact),
                        Some(snapshot) if !snapshot.is_empty() => {
                            if self.mode == ViewMode::Details {
                                content.child(self.list(snapshot, layout, columns, theme, cx))
                            } else {
                                content.child(self.grid(snapshot, layout, per_row, theme, cx))
                            }
                        }
                        Some(snapshot) => content.child(self.entry_list(
                            snapshot,
                            self.empty_state(snapshot, theme, cx),
                            None,
                        )),
                        None => content.children(failed_here),
                    }),
            )
            .when(self.filter_open, |view| {
                view.child(self.filter_bar(theme, window, cx))
            })
            .child(self.status(theme))
            .children(self.band_outline(theme))
            .children(self.menu_layer(theme, window, cx));
        self.frames.end(frame_started);
        view
    }
}

/// docs/DESIGN.md's scrollbar: 8 px, 2 px in from the edge, a half-opaque
/// `text-muted` thumb on a clear track, `primary-bright` while hovered or
/// dragged. Always shown, as Breeze does, so the length of a list is seen
/// before scrolling it.
fn styled_scrollbar(scrollbar: Scrollbar, theme: Theme) -> Scrollbar {
    let resting = gpui_kit::Rgba {
        a: theme.text_muted.a * 0.5,
        ..theme.text_muted
    };
    scrollbar.mode(ScrollbarMode::Always).styles(|styles| {
        styles
            .track(|style| {
                style
                    .bg(gpui_kit::transparent_black())
                    .width(px(size::SCROLLBAR))
            })
            .thumb(|style| {
                style
                    .bg(resting)
                    .width(px(size::SCROLLBAR - 2. * size::SCROLLBAR_INSET))
                    .inset(px(size::SCROLLBAR_INSET))
                    .radius(px(size::RADIUS))
            })
            .thumb_hover(|style| style.bg(theme.accent))
            .thumb_active(|style| style.bg(theme.accent))
    })
}

/// Entries being dragged out of a view. What they are is looked up when
/// needed, so starting a drag costs nothing however many are selected.
pub struct DraggedEntries {
    source: gpui_kit::WeakEntity<Browser>,
    row: usize,
}

impl DraggedEntries {
    /// The dragged entries, as full paths.
    pub fn paths(&self, cx: &App) -> Vec<PathBuf> {
        self.source
            .upgrade()
            .map_or_else(Vec::new, |browser| browser.read(cx).drag_paths(self.row))
    }

    /// The folders among the dragged entries.
    pub fn folder_paths(&self, cx: &App) -> Vec<PathBuf> {
        self.paths(cx)
            .into_iter()
            .zip(self.folders(cx))
            .filter_map(|(path, is_folder)| is_folder.then_some(path))
            .collect()
    }

    /// For each of [`Self::paths`], whether it is a folder.
    fn folders(&self, cx: &App) -> Vec<bool> {
        let Some(browser) = self.source.upgrade() else {
            return Vec::new();
        };
        let browser = browser.read(cx);
        let Some(snapshot) = browser.current_snapshot() else {
            return Vec::new();
        };
        if self.row < snapshot.len() && !browser.selection.contains(snapshot, self.row) {
            return vec![snapshot.is_container(self.row)];
        }
        browser
            .selection
            .rows(snapshot)
            .into_iter()
            .map(|row| snapshot.is_container(row))
            .collect()
    }
}

/// The picture a drag chip shows for the entry it started on.
#[derive(Clone)]
enum DragIcon {
    /// A thumbnail or the type's icon from the system theme.
    Image(Arc<Path>),
    Folder,
    File,
}

/// What follows the pointer during a drag: the first entry's thumbnail or
/// icon, then its name or how many. No copy or move sign: the drop asks.
struct DragPreview {
    label: SharedString,
    icon: DragIcon,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::StyledImage as _;
        let theme = *cx.global::<Theme>();
        let icon = match &self.icon {
            DragIcon::Image(path) => img(path.clone())
                .size(px(size::ICON))
                .flex_none()
                .object_fit(gpui_kit::ObjectFit::Contain)
                .into_any_element(),
            DragIcon::Folder | DragIcon::File => Icon::new(match self.icon {
                DragIcon::Folder => IconName::Folder,
                _ => IconName::File,
            })
            .size(px(size::ICON))
            .flex_none()
            .into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::DRAG_CHIP))
            .px(px(space::SM))
            .rounded(px(size::RADIUS))
            .bg(theme.selection)
            .text_color(theme.on_selection)
            .font_family(text::FAMILY_BODY)
            .text_size(px(text::BODY))
            .child(icon)
            .child(self.label.clone())
    }
}

/// A short name for a trash directory: "Home trash" for the one in the data
/// directory, otherwise "Trash on “volume”" after the volume's top folder.
fn trash_label(dir: &Path) -> String {
    let in_volume = dir
        .file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(b".Trash"));
    let parent = dir.parent();
    // `$top/.Trash/$uid` or `$top/.Trash-$uid`.
    let top = match parent {
        Some(shared) if shared.file_name() == Some(OsStr::new(".Trash")) => shared.parent(),
        Some(top) if in_volume => Some(top),
        _ => None,
    };
    match top {
        Some(top) => {
            let name = top
                .file_name()
                .map_or_else(|| "/".into(), |name| display_name(name.as_encoded_bytes()));
            format!("Trash on \u{201c}{name}\u{201d}")
        }
        None => "Home trash".to_owned(),
    }
}

/// What a type-ahead buffer searches for: the prefix, and whether to start
/// after the focused row. Pressing one letter repeatedly cycles through the
/// names starting with it; anything else narrows the search in place.
fn type_ahead_query(typed: &str) -> (&str, bool) {
    let mut chars = typed.chars();
    match chars.next() {
        Some(first) if chars.all(|c| c == first) => (&typed[..first.len_utf8()], true),
        _ => (typed, false),
    }
}

/// Turns typed text into an absolute path: expands a leading `~` and resolves
/// relative paths against `base`. Returns `None` for empty input.
/// Whether a typed location is a trash address, which Rustphin opens itself,
/// as Dolphin does, instead of asking kio-fuse to mount it. Folders inside the
/// trash have no address of their own here, so any trash address opens the
/// trash.
fn names_trash(typed: &str) -> bool {
    typed
        .split_once(':')
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("trash"))
}

fn expand_path(typed: &str, base: Option<&Path>) -> Option<PathBuf> {
    if typed.is_empty() {
        return None;
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let path = match (typed.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home,
        (Some(rest), Some(home)) if rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(typed),
    };
    Some(match base {
        Some(base) if path.is_relative() => base.join(path),
        _ => path,
    })
}

/// The folders the location bar offers, outermost first, with their labels.
/// Inside a remote folder the trail starts at its mount, labelled with the
/// remote address, instead of kio-fuse's local mount path.
fn crumb_trail(path: &Path, remote: Option<(&Path, &str)>) -> Vec<(PathBuf, String)> {
    let mut trail: Vec<(PathBuf, String)> = path
        .ancestors()
        .map(|ancestor| {
            let label = match ancestor.file_name() {
                Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
                None => "/".to_owned(),
            };
            (ancestor.to_path_buf(), label)
        })
        .collect();
    trail.reverse();
    if let Some((root, address)) = remote.filter(|(root, _)| path.starts_with(root)) {
        trail.retain(|(ancestor, _)| ancestor.starts_with(root));
        if let Some(first) = trail.first_mut() {
            first.1 = address.to_owned();
        }
    }
    trail
}

/// What the status line counts.
struct StatusFacts<'a> {
    /// Rows shown.
    shown: usize,
    /// Rows there would be without the name filter.
    unfiltered: usize,
    /// Entries listed, including hidden ones.
    total: usize,
    selected: usize,
    filter: Option<&'a str>,
    loading: bool,
    /// Total size of the files shown, when any is known.
    shown_size: Option<u64>,
    /// Total size of the selected files, when any is known.
    selected_size: Option<u64>,
    /// The name being edited in place, if any.
    renaming: Option<&'a str>,
}

/// The sizes behind the status line, with the snapshot and selection they
/// were worked out for.
#[derive(Debug, Clone, Copy)]
struct StatusSizes {
    key: (usize, u64, u64),
    shown: Option<u64>,
    selected: Option<u64>,
}

/// The status line's words: the selection, when there is one, and the
/// totals ("12 items (150.4 MiB), 1 hidden").
#[derive(Debug, PartialEq)]
struct StatusParts {
    selection: Option<String>,
    total: String,
}

fn status_parts(facts: &StatusFacts<'_>) -> StatusParts {
    let items = |n: usize| plural(n, "item", "items");
    let sized = |text: String, size: Option<u64>| match size {
        Some(size) => format!("{text} ({})", format::size(size)),
        None => text,
    };
    // While a name is edited, the status says how to finish, as Dolphin's does.
    let renaming = facts.renaming.map(|name| {
        format!("Renaming \u{201c}{name}\u{201d}: Enter keeps the new name, Esc cancels.")
    });
    let selection = renaming.or_else(|| {
        (facts.selected > 0).then(|| {
            sized(
                format!(
                    "{} {} selected",
                    count(facts.selected),
                    items(facts.selected)
                ),
                facts.selected_size,
            )
        })
    });
    let mut total = match facts.filter {
        Some(text) => format!(
            "{} of {} {} match \u{201c}{text}\u{201d}",
            count(facts.shown),
            count(facts.unfiltered),
            items(facts.unfiltered)
        ),
        None => sized(
            format!("{} {}", count(facts.shown), items(facts.shown)),
            facts.shown_size,
        ),
    };
    let hidden = facts.total.saturating_sub(facts.unfiltered);
    if hidden > 0 {
        total.push_str(&format!(", {} hidden", count(hidden)));
    }
    if facts.loading {
        total.push_str(" \u{2014} loading\u{2026}");
    }
    StatusParts { selection, total }
}

/// Which parts of the status line fit a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusZones {
    /// The free space on the right.
    free_space: bool,
    /// The totals after the selection.
    totals: bool,
}

impl StatusZones {
    /// The free space leaves first; then, while entries are selected, the
    /// totals, so the selection is never cut off.
    fn for_width(width: f32, selected: bool) -> Self {
        Self {
            free_space: width >= size::STATUS_FREE_SPACE_MIN,
            totals: !selected || width >= size::STATUS_SELECTION_ONLY_BELOW,
        }
    }
}

/// A rubber-band selection in progress.
#[derive(Debug, Clone, Copy)]
struct Band {
    /// Where the drag began, in list coordinates, so it stays put if the list scrolls.
    start: (f32, f32),
    /// Where the button went down and where the pointer is now, in window coordinates.
    pressed_at: Point<Pixels>,
    pointer: Point<Pixels>,
    /// Whether the pointer has moved far enough for this to be a band and not a click.
    active: bool,
    /// Ctrl was held: add to the selection instead of replacing it.
    add: bool,
}

/// One entry as a row or cell draws it.
struct Entry {
    name: SharedString,
    container: bool,
    /// A symbolic link: its name is drawn in italics.
    link: bool,
    selected: bool,
    /// Has the keyboard focus.
    focused: bool,
    /// Its name is being edited in place.
    renaming: bool,
    /// Cut and waiting to be pasted.
    cut: bool,
    /// Hidden (a dot name), shown because hidden entries are.
    hidden: bool,
    /// Its thumbnail, shown in place of the type icon.
    thumb: Option<Arc<Path>>,
    info: Option<TypeInfo>,
}

impl Entry {
    /// Semantics shared by the three virtualised entry layouts.
    fn accessible(
        &self,
        rows: &Snapshot,
        row: usize,
        cx: &mut Context<Browser>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let view = cx.entity().downgrade();
        let focus_view = view.clone();
        div()
            .id(("file", rows.entry_id(row) as usize))
            .role(Role::ListBoxOption)
            .aria_label(self.name.clone())
            .aria_description(match (self.link, self.container) {
                (true, true) => "Link to a folder",
                (true, false) => "Link to a file",
                (false, true) => "Folder",
                (false, false) => "File",
            })
            .aria_selected(self.selected)
            .aria_position_in_set(row)
            .when(self.focused && !self.renaming, |entry| {
                entry.aria_active_descendant()
            })
            .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                let _ = focus_view.update(cx, |this, cx| {
                    this.change_selection(
                        |selection, snapshot| selection.select_only(snapshot, row),
                        cx,
                    );
                    this.focus.focus(window, cx);
                });
            })
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.change_selection(
                        |selection, snapshot| selection.toggle(snapshot, row),
                        cx,
                    );
                    this.focus.focus(window, cx);
                });
            })
    }

    /// What the drag chip shows for this entry.
    fn drag_icon(&self) -> DragIcon {
        match (
            &self.thumb,
            self.info.as_ref().and_then(|info| info.icon.clone()),
        ) {
            (Some(thumb), _) => DragIcon::Image(thumb.clone()),
            (None, Some(icon)) => DragIcon::Image(icon),
            (None, None) if self.container => DragIcon::Folder,
            (None, None) => DragIcon::File,
        }
    }

    /// Colour for secondary text and the placeholder icon.
    fn secondary(&self, theme: Theme) -> gpui_kit::Rgba {
        if self.selected {
            theme.on_selection
        } else {
            theme.text_muted
        }
    }

    /// The file-type icon from the system theme, or a generic one until it is
    /// known or when the theme has none.
    fn icon(&self, edge: f32, theme: Theme) -> AnyElement {
        // A thumbnail keeps the picture's proportions inside the icon's square.
        if let Some(thumb) = &self.thumb {
            use gpui_kit::StyledImage as _;
            return img(thumb.clone())
                .size(px(edge))
                .flex_none()
                .object_fit(gpui_kit::ObjectFit::Contain)
                .into_any_element();
        }
        match self.info.as_ref().and_then(|info| info.icon.clone()) {
            Some(path) => img(path).size(px(edge)).flex_none().into_any_element(),
            None => Icon::new(if self.container {
                IconName::Folder
            } else {
                IconName::File
            })
            .size(px(edge))
            .flex_none()
            .text_color(if self.container && !self.selected {
                theme.accent
            } else {
                self.secondary(theme)
            })
            .into_any_element(),
        }
    }

    /// The entry's background and outline. Selection is a fill plus an accent
    /// bar, and keyboard focus without selection the bar alone, so neither is
    /// shown by colour only.
    /// The entry's fill, selection bar and hover. It carries an id because
    /// GPUI keeps an element's hover state, and redraws when the pointer
    /// enters or leaves, only for elements with one (W-100).
    fn surface(&self, row: usize, theme: Theme) -> gpui_kit::Stateful<gpui_kit::Div> {
        div()
            .id(("entry-surface", row))
            .relative()
            .rounded(px(size::RADIUS))
            .text_color(theme.text)
            // The bar lies over the entry's left edge, so selecting it moves
            // nothing.
            .when(self.selected || self.focused, |surface| {
                surface.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(size::SELECTION_BAR))
                        .rounded_l(px(size::RADIUS))
                        .bg(theme.accent),
                )
            })
            .when(self.selected, |surface| {
                surface.bg(theme.selection).text_color(theme.on_selection)
            })
            .when(self.renaming, |surface| surface.bg(theme.hover))
            .when(!self.selected && !self.renaming, |surface| {
                surface.hover(|surface| surface.bg(theme.hover))
            })
            .when_some(
                entry_opacity(self.hidden, self.cut, self.selected),
                |surface, opacity| surface.opacity(opacity),
            )
    }
}

/// The status line's free space: "Free: 1.8 TiB of 3.6 TiB (Data)", the
/// volume's name when the places panel knows it.
fn free_text(total: u64, free: u64, volume: Option<&str>) -> String {
    let text = format!("Free: {} of {}", format::size(free), format::size(total));
    match volume {
        Some(name) => format!("{text} ({name})"),
        None => text,
    }
}

/// How faded an entry is drawn, if at all. Cut entries are on their way
/// out: half opacity until pasted. Hidden ones are faded as in Dolphin,
/// except while selected, where the selection's own contrast must hold.
fn entry_opacity(hidden: bool, cut: bool, selected: bool) -> Option<f32> {
    let hidden = if hidden && !selected {
        crate::theme::HIDDEN_OPACITY
    } else {
        1.
    };
    let cut = if cut { CUT_OPACITY } else { 1. };
    let opacity = hidden * cut;
    (opacity < 1.).then_some(opacity)
}

/// Opacity of an entry that was cut and is waiting to be pasted.
const CUT_OPACITY: f32 = 0.5;

fn select_names(selection: &mut Selection, snapshot: &Snapshot, names: &[Vec<u8>]) {
    let mut selected = false;
    for name in names {
        if let Some(row) = snapshot.row_of(name) {
            if selected {
                selection.toggle(snapshot, row);
            } else {
                selection.select_only(snapshot, row);
                selected = true;
            }
        }
    }
}

/// Whether `selection` covers all `len` bytes of a field's text.
fn selects_everything(selection: &std::ops::Range<usize>, len: usize) -> bool {
    selection.start == 0 && selection.end == len
}

/// The size to ask the icon theme for, given the size drawn. Themes ship a
/// fixed set of sizes; asking for one of them avoids scaling a neighbour.
fn theme_icon_size(drawn: f32) -> u32 {
    const SIZES: [u32; 9] = [16, 22, 24, 32, 48, 64, 96, 128, 256];
    SIZES
        .into_iter()
        .find(|&size| size as f32 >= drawn)
        .unwrap_or(256)
}

/// How many trailing folders of the location fit the view; the rest collapse
/// into an ellipsis. The current folder is always shown.
fn crumbs_for_width(width: f32) -> usize {
    match width {
        w if w >= 1000. => 4,
        w if w >= 780. => 3,
        w if w >= 520. => 2,
        _ => 1,
    }
}

/// Which parts of the toolbar fit the window's width. The rest stay on
/// their keys and in the Actions palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ToolbarLayout {
    pub back: bool,
    pub forward_and_up: bool,
    /// Actions with its label and chevron, rather than an icon button.
    pub labelled_actions: bool,
    /// The Ctrl+Shift+P hint beside the Actions label.
    pub shortcut_hint: bool,
    /// Reload, New folder, the view mode and the toggles.
    pub tools: bool,
    /// The window's width.
    pub width: f32,
}

impl ToolbarLayout {
    /// The shortcut hint leaves first; then the view tools; then Forward and
    /// Up, and Actions becomes an icon. Back leaves only when the window can
    /// no longer hold it beside the location and Actions.
    pub(crate) fn for_width(width: f32) -> Self {
        let full = width >= size::TOOLBAR_FULL_MIN;
        Self {
            back: width >= size::TOOLBAR_BACK_MIN,
            forward_and_up: full,
            labelled_actions: full,
            shortcut_hint: width >= size::TOOLBAR_HINT_MIN,
            tools: width >= size::TOOLBAR_TOOLS_MIN,
            width,
        }
    }
}

/// The toolbar's parts that act on one pane, for the window to place.
pub(crate) struct ToolbarParts {
    pub navigation: AnyElement,
    pub location: AnyElement,
    pub new_folder: Option<AnyElement>,
    pub view_mode: Option<AnyElement>,
    pub toggles: Option<AnyElement>,
    pub actions: AnyElement,
}

/// Which optional columns fit the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    size: bool,
    modified: bool,
    kind: bool,
}

impl Columns {
    /// Narrow panes drop type, modified and then size to preserve names.
    fn for_width(width: f32) -> Self {
        Self {
            size: width >= size::WIDTH_FOR_SIZE,
            modified: width >= size::WIDTH_FOR_MODIFIED,
            kind: width >= size::WIDTH_FOR_TYPE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_space_names_its_volume_when_the_places_panel_knows_it() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            free_text(4 * gib, gib, Some("Data")),
            "Free: 1.0 GiB of 4.0 GiB (Data)"
        );
        assert_eq!(free_text(4 * gib, gib, None), "Free: 1.0 GiB of 4.0 GiB");
    }

    #[test]
    fn a_field_is_wholly_selected_only_from_its_start_to_its_end() {
        assert!(selects_everything(&(0..5), 5));
        assert!(selects_everything(&(0..0), 0));
        assert!(!selects_everything(&(0..4), 5));
        assert!(!selects_everything(&(1..5), 5));
    }

    #[test]
    fn messages_say_what_happened_to_which_folder() {
        let failed = Failure {
            path: PathBuf::from("/data/locked"),
            error: LoadError::PermissionDenied,
            moved: false,
        };
        assert_eq!(
            failed.message(),
            "Could not open /data/locked: you do not have permission to open this location."
        );
        let gone = Failure {
            path: PathBuf::from("/data/old"),
            error: LoadError::NotFound,
            moved: true,
        };
        assert_eq!(
            gone.message(),
            "/data/old no longer exists. Showing the nearest folder that does."
        );
    }

    fn failure(path: &str, error: LoadError, moved: bool) -> Failure {
        Failure {
            path: PathBuf::from(path),
            error,
            moved,
        }
    }

    #[test]
    fn a_folder_that_could_not_be_opened_is_explained_in_place() {
        let locked = failure("/data/locked", LoadError::PermissionDenied, false);
        let here = Some(Path::new("/data/locked"));
        // Opened and failed: nothing is listed for it, so it is said in place.
        assert!(locked.in_place(here, false));
        // A shown folder that failed to re-read keeps its listing and a banner.
        assert!(!locked.in_place(here, true));
        // The view moved on, or out of a folder that vanished: a banner.
        assert!(!locked.in_place(Some(Path::new("/data")), false));
        let gone = failure("/data/locked", LoadError::NotFound, true);
        assert!(!gone.in_place(here, false));
    }

    #[test]
    fn the_in_place_error_says_why_and_where_its_button_leads() {
        let locked = failure("/data/locked", LoadError::PermissionDenied, false);
        assert_eq!(
            locked.explanation(Way::Back),
            "You do not have permission to open /data/locked. Ask its owner \
             for access, or go back to the previous folder."
        );
        assert_eq!(
            locked.explanation(Way::Up),
            "You do not have permission to open /data/locked. Ask its owner \
             for access, or go up to the folder that contains it."
        );
        assert_eq!(
            locked.explanation(Way::Nowhere),
            "You do not have permission to open /data/locked. Ask its owner for access."
        );
        assert_eq!(
            failure("/no/such", LoadError::NotFound, false).explanation(Way::Back),
            "/no/such does not exist. Check the address, or go back to the previous folder."
        );
        assert_eq!(
            failure("/notes.txt", LoadError::NotADirectory, false).explanation(Way::Nowhere),
            "/notes.txt is a file, not a folder. Open it from the folder that contains it."
        );
        assert_eq!(
            failure("/mnt/x", LoadError::Io("Input/output error".into()), false)
                .explanation(Way::Back),
            "/mnt/x could not be read: Input/output error. Try again with F5, \
             or go back to the previous folder."
        );
    }

    #[test]
    fn the_status_line_names_the_failure_while_it_is_shown_in_place() {
        let status = |error| failure("/x", error, false).status();
        assert_eq!(status(LoadError::PermissionDenied), "Not readable");
        assert_eq!(status(LoadError::NotFound), "Does not exist");
        assert_eq!(status(LoadError::NotADirectory), "Not a folder");
        assert_eq!(status(LoadError::Io("broken".into())), "Could not be read");
    }

    #[test]
    fn hidden_entries_are_faded_unless_selected_and_cut_ones_fade_further() {
        let hidden = crate::theme::HIDDEN_OPACITY;
        assert_eq!(entry_opacity(false, false, false), None);
        assert_eq!(entry_opacity(false, false, true), None);
        assert_eq!(entry_opacity(true, false, false), Some(hidden));
        assert_eq!(entry_opacity(true, false, true), None);
        assert_eq!(entry_opacity(false, true, true), Some(CUT_OPACITY));
        assert_eq!(entry_opacity(true, true, false), Some(hidden * CUT_OPACITY));
    }

    #[test]
    fn repeating_one_letter_cycles_and_a_longer_prefix_narrows() {
        assert_eq!(type_ahead_query("f"), ("f", true));
        assert_eq!(type_ahead_query("fff"), ("f", true));
        assert_eq!(type_ahead_query("fi"), ("fi", false));
        assert_eq!(type_ahead_query("\u{e9}\u{e9}"), ("\u{e9}", true));
        assert_eq!(type_ahead_query(""), ("", false));
    }

    #[test]
    fn icons_are_requested_at_the_theme_size_at_or_above_the_drawn_size() {
        assert_eq!(theme_icon_size(16.), 16);
        assert_eq!(theme_icon_size(20.), 22);
        assert_eq!(theme_icon_size(64.), 64);
        assert_eq!(theme_icon_size(192.), 256);
        assert_eq!(theme_icon_size(999.), 256);
    }

    #[test]
    fn narrower_views_show_fewer_folders_of_the_location_but_always_one() {
        assert_eq!(crumbs_for_width(1280.), 4);
        assert_eq!(crumbs_for_width(899.), 3);
        assert_eq!(crumbs_for_width(639.), 2);
        assert_eq!(crumbs_for_width(479.), 1);
        assert_eq!(crumbs_for_width(0.), 1);
    }

    #[test]
    fn a_typed_trash_address_opens_the_trash_not_a_mount() {
        for typed in ["trash:/", "trash:", "trash:///", "TRASH:/", "trash:/old"] {
            assert!(names_trash(typed), "{typed}");
        }
        // A folder called "trash", or one inside it, is an ordinary path.
        for typed in ["trash", "/trash", "trash/", "./trash:", "sftp:/"] {
            assert!(!names_trash(typed), "{typed}");
        }
    }

    #[test]
    fn narrow_windows_drop_the_hint_then_the_tools_then_forward_and_up_then_back() {
        let at = ToolbarLayout::for_width;
        let widest = at(size::TOOLBAR_HINT_MIN);
        assert!(
            widest.shortcut_hint
                && widest.tools
                && widest.labelled_actions
                && widest.forward_and_up
        );
        let wide = at(size::TOOLBAR_HINT_MIN - 1.);
        assert!(!wide.shortcut_hint && wide.tools && wide.labelled_actions);
        let tools = at(size::TOOLBAR_TOOLS_MIN);
        assert!(tools.tools && tools.forward_and_up);
        let no_tools = at(size::TOOLBAR_TOOLS_MIN - 1.);
        assert!(!no_tools.tools && no_tools.forward_and_up && no_tools.labelled_actions);
        let full = at(size::TOOLBAR_FULL_MIN);
        assert!(full.back && full.forward_and_up && full.labelled_actions);
        let medium = at(size::TOOLBAR_FULL_MIN - 1.);
        assert!(medium.back && !medium.forward_and_up && !medium.labelled_actions);
        let back_only = at(size::TOOLBAR_BACK_MIN);
        assert!(back_only.back && !back_only.forward_and_up);
        let narrow = at(size::TOOLBAR_BACK_MIN - 1.);
        assert!(!narrow.back && !narrow.forward_and_up);
    }

    #[test]
    fn narrow_panes_drop_size_before_names_disappear() {
        assert!(!Columns::for_width(size::WIDTH_FOR_SIZE - 1.).size);
        assert!(Columns::for_width(size::WIDTH_FOR_SIZE).size);
    }

    #[test]
    fn narrow_windows_drop_the_type_column_then_the_date() {
        let wide = Columns::for_width(1200.);
        assert!(wide.modified && wide.kind);
        let medium = Columns::for_width(size::WIDTH_FOR_TYPE - 1.);
        assert!(medium.modified && !medium.kind);
        let narrow = Columns::for_width(size::WIDTH_FOR_MODIFIED - 1.);
        assert!(!narrow.modified && !narrow.kind);
    }

    #[test]
    fn trash_directories_are_named_after_their_volume() {
        assert_eq!(
            trash_label(Path::new("/mnt/data/.Trash-1000")),
            "Trash on \u{201c}data\u{201d}"
        );
        assert_eq!(
            trash_label(Path::new("/media/usb stick/.Trash/1000")),
            "Trash on \u{201c}usb stick\u{201d}"
        );
        assert_eq!(
            trash_label(Path::new("/.Trash-1000")),
            "Trash on \u{201c}/\u{201d}"
        );
        assert_eq!(
            trash_label(Path::new("/home/u/.local/share/Trash")),
            "Home trash"
        );
    }

    #[test]
    fn the_status_counts_matches_hidden_and_selected_entries() {
        const MIB: u64 = 1024 * 1024;
        let facts = |shown, unfiltered, total, selected, filter| StatusFacts {
            shown,
            unfiltered,
            total,
            selected,
            filter,
            loading: false,
            shown_size: None,
            selected_size: None,
            renaming: None,
        };
        let parts = |facts: &StatusFacts<'_>| {
            let parts = status_parts(facts);
            (parts.selection, parts.total)
        };
        assert_eq!(parts(&facts(7, 7, 7, 0, None)), (None, "7 items".into()));
        // The selection has its own part, before the totals.
        assert_eq!(
            parts(&StatusFacts {
                shown_size: Some(150 * MIB),
                selected_size: Some(6 * MIB),
                ..facts(7, 7, 9, 2, None)
            }),
            (
                Some("2 items selected (6.0 MiB)".into()),
                "7 items (150.0 MiB), 2 hidden".into()
            )
        );
        assert_eq!(
            parts(&facts(2, 7, 9, 0, Some("txt"))),
            (
                None,
                "2 of 7 items match \u{201c}txt\u{201d}, 2 hidden".into()
            )
        );
        assert_eq!(
            parts(&facts(2, 7, 7, 1, Some("txt"))),
            (
                Some("1 item selected".into()),
                "2 of 7 items match \u{201c}txt\u{201d}".into()
            )
        );
        let loading = StatusFacts {
            loading: true,
            ..facts(1, 1, 1, 0, None)
        };
        assert_eq!(parts(&loading).1, "1 item \u{2014} loading\u{2026}");
    }

    #[test]
    fn while_renaming_the_status_line_says_which_keys_finish() {
        let renaming = StatusFacts {
            shown: 12,
            unfiltered: 12,
            total: 12,
            selected: 1,
            filter: None,
            loading: false,
            shown_size: None,
            selected_size: Some(6),
            renaming: Some("IMG_4821.jpg"),
        };
        let parts = status_parts(&renaming);
        assert_eq!(
            parts.selection.as_deref(),
            Some("Renaming \u{201c}IMG_4821.jpg\u{201d}: Enter keeps the new name, Esc cancels.")
        );
        assert_eq!(parts.total, "12 items");
    }

    #[test]
    fn the_status_line_drops_free_space_then_the_totals_as_the_pane_narrows() {
        let wide = StatusZones::for_width(size::STATUS_FREE_SPACE_MIN, true);
        assert!(wide.free_space && wide.totals);
        let medium = StatusZones::for_width(size::STATUS_FREE_SPACE_MIN - 1., true);
        assert!(!medium.free_space && medium.totals);
        let narrow = StatusZones::for_width(size::STATUS_SELECTION_ONLY_BELOW - 1., true);
        assert!(!narrow.free_space && !narrow.totals);
        // Without a selection the totals stay, since they are all there is.
        let unselected = StatusZones::for_width(size::STATUS_SELECTION_ONLY_BELOW - 1., false);
        assert!(unselected.totals);
    }

    #[test]
    fn the_crumb_trail_starts_at_a_remote_mount() {
        let labels = |path: &str, remote: Option<(&Path, &str)>| -> Vec<String> {
            crumb_trail(Path::new(path), remote)
                .into_iter()
                .map(|(_, label)| label)
                .collect()
        };
        assert_eq!(labels("/srv/data", None), ["/", "srv", "data"]);
        let mount = Some((
            Path::new("/run/kio-fuse-x/sftp/host/projects"),
            "sftp://me@host/projects",
        ));
        assert_eq!(
            labels("/run/kio-fuse-x/sftp/host/projects/sub dir", mount),
            ["sftp://me@host/projects", "sub dir"]
        );
        let trail = crumb_trail(
            Path::new("/run/kio-fuse-x/sftp/host/projects/sub dir"),
            mount,
        );
        assert_eq!(trail[0].0, Path::new("/run/kio-fuse-x/sftp/host/projects"));
        // A mount elsewhere does not change a local trail.
        assert_eq!(labels("/srv/data", mount), ["/", "srv", "data"]);
    }

    #[test]
    fn typed_paths_are_made_absolute() {
        let base = Path::new("/srv/data");
        assert_eq!(expand_path("", Some(base)), None);
        assert_eq!(expand_path("/etc", Some(base)), Some(PathBuf::from("/etc")));
        assert_eq!(
            expand_path("sub/dir", Some(base)),
            Some(PathBuf::from("/srv/data/sub/dir"))
        );
        assert_eq!(
            expand_path("~weird", Some(base)),
            Some(PathBuf::from("/srv/data/~weird"))
        );
    }
}
