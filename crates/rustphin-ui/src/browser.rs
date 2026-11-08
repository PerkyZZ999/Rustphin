//! The browser view: toolbar, entry list and status line for one location.

use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{
    AccessibleAction, AnyElement, App, AppContext as _, AsyncApp, ClickEvent, Context, Edges,
    Entity, EventEmitter, ExternalDragPayload, ExternalPaths, FileDragPaths, FocusHandle,
    Focusable as _, InteractiveElement, IntoElement, KeyBinding, KeyDownEvent, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point,
    Render, Role, ScrollHandle, ScrollStrategy, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, UniformListScrollHandle, WeakEntity, Window, actions, anchored,
    component::{
        Disableable as _, Icon, IconName, Sizable as _,
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
    DirectoryModel, EntryKind, LoadError, ModelEvent, RealFs, Selection, Snapshot, SortKey,
    ViewOptions, display_name, ops::Request,
};
use rustphin_platform::{remote::RemoteAddress, uri::path_from_uri};

use crate::{
    diagnostics::FrameLog,
    filetypes::{Database, TypeInfo},
    format::{self, count, plural},
    layout::{self, Layout, Rect, ViewMode},
    location::Location,
    menu::{self, MenuCommand, MenuEffect, MenuKey},
    operations::{self, hub},
    remote,
    settings::Settings,
    theme::{Theme, size, space, text},
    thumbs::{self, thumbs},
};

const KEY_CONTEXT: &str = "Browser";
/// The selection's context menu, while it has the keyboard focus.
const MENU_CONTEXT: &str = "ContextMenu";
/// The only action inside an empty-folder message.
const FOLDER_ACTION_CONTEXT: &str = "FolderStateAction";
/// The browser while no text field inside it has focus.
const LIST_CONTEXT: &str = "Browser && !Input";
/// How far the pointer must move with the button down before a rubber band starts.
const BAND_THRESHOLD: f32 = 4.;
/// A pause longer than this starts a new type-ahead search.
const TYPE_AHEAD_PAUSE: Duration = Duration::from_millis(1000);
/// Icons smaller than this stay type icons: a thumbnail would be a smudge.
const MIN_THUMBNAIL_EDGE: f32 = 32.;
/// Rows moved by Page Up and Page Down.
const PAGE_ROWS: isize = 15;

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
        Cut,
        Paste,
        OpenContextMenu,
        MenuUp,
        MenuDown,
        MenuLeft,
        MenuRight,
        MenuAccept,
        MenuClose,
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
        KeyBinding::new("enter", ShowHidden, Some(FOLDER_ACTION_CONTEXT)),
        KeyBinding::new("space", ShowHidden, Some(FOLDER_ACTION_CONTEXT)),
    ]);
    let menu = Some(MENU_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", MenuUp, menu),
        KeyBinding::new("down", MenuDown, menu),
        KeyBinding::new("left", MenuLeft, menu),
        KeyBinding::new("right", MenuRight, menu),
        KeyBinding::new("enter", MenuAccept, menu),
        KeyBinding::new("escape", MenuClose, menu),
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

impl Failure {
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
    /// The entry being renamed in place, by its current name.
    renaming: Option<Vec<u8>>,
    rename_input: Entity<InputState>,
    /// An entry to select as soon as it shows up in the list, and whether to
    /// start renaming it: the result of a create or rename this view asked for.
    select_when_seen: Option<(Vec<u8>, bool)>,
    /// An entry whose renaming starts at the next redraw, when a window is at hand.
    rename_requested: Option<Vec<u8>>,
    /// Take the keyboard focus back at the next redraw.
    refocus: bool,
    /// The trash directory whose contents this view is showing, if it is one.
    trash_dir: Option<PathBuf>,
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
    /// Entries per row of the list or grid as last drawn; 1 in details mode.
    per_row: usize,
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
        let mut subscriptions = vec![
            cx.subscribe_in(&location_input, window, Self::on_location_input),
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
            renaming: None,
            rename_input,
            select_when_seen: None,
            rename_requested: None,
            refocus: false,
            trash_dir: None,
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
            per_row: 1,
            icon_epoch: 0,
            settings_path,
            saved: settings.clone(),
            frames: FrameLog::from_env(cx),
            width: 0.,
            band: None,
            ignore_click: false,
            scroll: UniformListScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        match DirectoryModel::new(Arc::new(RealFs)) {
            Ok((mut model, events)) => {
                model.set_options(settings.options());
                this.model = Some(model);
                this.load_file_types(cx);
                cx.spawn(async move |view, cx| Self::receive_events(view, events, cx).await)
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
        cx: &mut AsyncApp,
    ) {
        'events: while let Ok(event) = events.recv_async().await {
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
                    self.detect_trash(snapshot.path().to_owned(), cx);
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

    fn reveal_focus(&self) {
        if let Some(row) = self.focus_row {
            self.scroll
                .scroll_to_item(row / self.per_row.max(1), ScrollStrategy::Nearest);
        }
    }

    /// Moves the focus by keyboard; with `extend`, selects the range to it.
    fn move_selection(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        let len = self.current_snapshot().map_or(0, |snapshot| snapshot.len());
        let Some(row) = target_row(self.focus_row, len, motion) else {
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
        let state = self.scroll.0.borrow();
        let bounds = state.base_handle.bounds();
        if bounds.size.height <= px(0.) {
            return None;
        }
        let offset = state.base_handle.offset();
        Some((
            f32::from(position.x - bounds.origin.x) - space::SM,
            f32::from(position.y - bounds.origin.y - offset.y) - space::XS,
        ))
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
        let state = self.scroll.0.borrow();
        let bounds = state.base_handle.bounds();
        if bounds.size.height <= px(0.) {
            return (point(px(space::LG), px(size::TOOLBAR + space::LG)), None);
        }
        let corner = point(
            bounds.origin.x + px(space::LG),
            bounds.origin.y + px(space::LG),
        );
        let Some(row) = row else {
            return (corner, None);
        };
        // The inverse of `list_point`.
        let layout = self.mode.layout(self.zoom);
        let (x, y) = layout.cell_bottom_left(row, self.per_row);
        let offset = state.base_handle.offset();
        let top = bounds.origin.y;
        let bottom = bounds.origin.y + bounds.size.height;
        let clamp = |value: Pixels| value.clamp(top, bottom.max(top));
        let left = bounds.origin.x + px(x + space::SM + space::LG);
        let below = clamp(bounds.origin.y + offset.y + px(y + space::XS));
        let above = clamp(bounds.origin.y + offset.y + px(y - layout.row_height + space::XS));
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
        self.menu = None;
        cx.emit(BrowserEvent::Properties { path, type_label });
        self.focus.focus(window, cx);
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
            .scroll
            .0
            .borrow()
            .base_handle
            .bounds()
            .contains(&event.position);
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
        let layout = self.mode.layout(self.zoom);
        let rows = layout.rows_in(Rect::between(band.start, end), self.per_row, snapshot.len());
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

    /// The outline of the band being dragged, in window coordinates.
    fn band_outline(&self, theme: Theme) -> Option<impl IntoElement + use<>> {
        let band = self.band.filter(|band| band.active)?;
        let (left, right) = (
            band.pressed_at.x.min(band.pointer.x),
            band.pressed_at.x.max(band.pointer.x),
        );
        let (top, bottom) = (
            band.pressed_at.y.min(band.pointer.y),
            band.pressed_at.y.max(band.pointer.y),
        );
        Some(
            div()
                .absolute()
                .left(left)
                .top(top)
                .w(right - left)
                .h(bottom - top)
                .rounded(px(space::XS))
                .border_1()
                .border_color(theme.accent)
                .bg(theme.hover),
        )
    }

    /// Left and right move between neighbours in a grid; a list has none.
    fn move_sideways(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        if self.per_row > 1 {
            self.move_selection(Motion::By(delta), extend, cx);
        }
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
            self.dismiss_menu();
            self.generation = generation;
            self.failure = None;
            self.select_on_arrival = came_from.map(|name| vec![name]);
            cx.notify();
        }
    }

    fn navigate(
        &mut self,
        step: impl FnOnce(&mut DirectoryModel) -> Option<u64>,
        cx: &mut Context<Self>,
    ) {
        if let Some(generation) = self.model.as_mut().and_then(step) {
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

    /// Dolphin's "Folders First": folders above files, or mixed in.
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
        // A folder opened through kio-fuse is shown by its remote address,
        // as Dolphin shows it, rather than by its local mount path.
        let current = self
            .location()
            .map(|p| remote::address_of(&p, cx).unwrap_or_else(|| p.to_string_lossy().into_owned()))
            .unwrap_or_default();
        self.editing_location = true;
        self.location_input.update(cx, |input, cx| {
            input.set_value(current, window, cx);
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
                if let Some(address) = RemoteAddress::parse(typed) {
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
                                .outline()
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
        // What is already in the trash can only be deleted for good.
        if self.trash_dir.is_some() {
            return self.delete_selected(cx);
        }
        let paths = self.selected_paths();
        if !paths.is_empty() {
            hub(cx).update(cx, |hub, _| hub.start(Request::Trash { paths }));
        }
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        hub(cx).update(cx, |hub, cx| hub.delete_after_confirming(paths, cx));
    }

    fn pick_up_selected(&mut self, cut: bool, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        hub(cx).update(cx, |hub, cx| hub.pick_up(paths, cut, cx));
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(into) = self.location()
            && self.trash_dir.is_none()
        {
            hub(cx).update(cx, |hub, cx| hub.paste(into, cx));
        }
    }

    fn toolbar(&self, theme: Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_back = self.model.as_ref().is_some_and(DirectoryModel::can_go_back);
        let can_forward = self
            .model
            .as_ref()
            .is_some_and(DirectoryModel::can_go_forward);
        let location = self.location();
        let can_up = location.as_deref().and_then(Path::parent).is_some();

        let navigation = Navigation::for_width(self.width);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::XS))
            .h(px(size::TOOLBAR))
            .px(px(space::SM))
            .bg(theme.chrome)
            .border_b_1()
            .border_color(theme.border)
            .when(navigation.back, |bar| {
                bar.child(
                    Button::new("back")
                        .ghost()
                        .small()
                        .icon(IconName::ArrowLeft)
                        .tooltip("Back (Alt+Left)")
                        .accessibility_label("Back (Alt+Left)")
                        .disabled(!can_back)
                        .on_click(
                            cx.listener(|this, _, _, cx| this.navigate(DirectoryModel::back, cx)),
                        ),
                )
            })
            .when(navigation.forward_and_up, |bar| {
                bar.child(
                    Button::new("forward")
                        .ghost()
                        .small()
                        .icon(IconName::ArrowRight)
                        .tooltip("Forward (Alt+Right)")
                        .accessibility_label("Forward (Alt+Right)")
                        .disabled(!can_forward)
                        .on_click(
                            cx.listener(|this, _, _, cx| {
                                this.navigate(DirectoryModel::forward, cx)
                            }),
                        ),
                )
                .child(
                    Button::new("up")
                        .ghost()
                        .small()
                        .icon(IconName::ArrowUp)
                        .tooltip("Up one folder (Alt+Up)")
                        .accessibility_label("Up one folder (Alt+Up)")
                        .disabled(!can_up)
                        .on_click(cx.listener(|this, _, _, cx| this.go_up(cx))),
                )
            })
            .child(div().flex_1().min_w_0().px(px(space::XS)).map(|bar| {
                if self.editing_location {
                    bar.child(crate::input::named(
                        Input::new(&self.location_input).small(),
                        &self.location_input,
                        "Folder location",
                        window,
                        cx,
                    ))
                } else {
                    bar.child(self.breadcrumbs(location.as_deref(), theme, cx))
                }
            }))
            .child(
                Button::new("actions")
                    .ghost()
                    .small()
                    .label("Actions")
                    .tooltip("Search actions and folders (Ctrl+Shift+P)")
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(crate::workspace::OpenPalette), cx);
                    }),
            )
    }

    /// The current location as clickable ancestors. Clicking the free space
    /// after them switches to the editable path field.
    fn breadcrumbs(
        &self,
        location: Option<&Path>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
        let first_shown = ancestors.len().saturating_sub(crumbs_for_width(self.width));

        div()
            .id("breadcrumbs")
            .role(Role::Group)
            .aria_label("Folder location")
            .flex()
            .items_center()
            .h(px(size::ROW))
            .overflow_hidden()
            .rounded(px(size::RADIUS))
            .cursor_text()
            .on_click(cx.listener(|this, _, window, cx| this.edit_location(window, cx)))
            .when(first_shown > 0, |crumbs| {
                crumbs.child(
                    div()
                        .px(px(space::XS))
                        .text_color(theme.text_muted)
                        .child("\u{2026}"),
                )
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
                                .px(px(space::XS))
                                .rounded(px(size::RADIUS))
                                .cursor_pointer()
                                .text_color(if current {
                                    theme.text
                                } else {
                                    theme.text_muted
                                })
                                .hover(|crumb| crumb.bg(theme.hover).text_color(theme.text))
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

    fn failure_banner(
        &self,
        failure: &Failure,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .mx(px(space::SM))
            .mt(px(space::SM))
            .px(px(space::MD))
            .py(px(space::SM))
            .rounded(px(size::RADIUS))
            .bg(theme.danger_surface)
            .border_1()
            .border_color(theme.danger)
            .text_color(theme.danger)
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(px(size::ICON))
                    .flex_none(),
            )
            .child(div().flex_1().min_w_0().child(failure.message()))
            .child(
                Button::new("dismiss-failure")
                    .ghost()
                    .xsmall()
                    .label("Dismiss")
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
            .cursor_pointer()
            .text_color(if active { theme.text } else { theme.text_muted })
            .hover(|heading| heading.text_color(theme.text))
            .child(label)
            .when(active, |heading| {
                heading.child(
                    Icon::new(if options.descending {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronUp
                    })
                    .size(px(size::ICON_SMALL)),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| this.sort_by(key, cx)))
    }

    /// What a row or cell needs to know about the entry it shows.
    fn entry(&mut self, rows: &Snapshot, row: usize, cx: &mut Context<Self>) -> Entry {
        let container = rows.is_container(row);
        Entry {
            name: display_name(rows.name(row)).into_owned().into(),
            container,
            selected: self.selection.contains(rows, row),
            focused: self.focus_row == Some(row),
            renaming: self.renaming.as_deref() == Some(rows.name(row)),
            cut: self.has_cut && hub(cx).read(cx).is_cut(&rows.entry_path(row)),
            info: self.type_info_for(rows.name(row), container, cx),
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
        if container || edge < MIN_THUMBNAIL_EDGE || self.trash_dir.is_some() {
            return None;
        }
        let mime = self.types.as_ref()?.mime_type(rows.name(row), false);
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
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                // Solid and in the normal text colour, whatever the row's state.
                .rounded(px(size::RADIUS))
                .bg(theme.raised)
                .text_color(theme.text)
                .child(crate::input::named(
                    Input::new(&self.rename_input).xsmall(),
                    &self.rename_input,
                    "New name",
                    window,
                    cx,
                ))
        } else {
            div().truncate().child(entry.name.clone())
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
            .on_drag(dragged, |dragged, _, _, cx| {
                let label = operations::what(&dragged.paths(cx));
                cx.new(|_| DragPreview(label.into()))
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
    fn entry_list(&self, snapshot: &Snapshot, list: impl IntoElement) -> impl IntoElement {
        div()
            .id("file-list")
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
        // Headings line up with the row text: same gutter, accent bar and padding.
        let gutter = space::SM * 2. + size::SELECTION_BAR;

        let headings = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::ROW))
            .px(px(gutter))
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(text::SMALL))
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
                        // Folders show no size: a folder's own size says nothing useful.
                        let size_text = metadata
                            .filter(|_| !entry.container)
                            .map(|metadata| format::size(metadata.size))
                            .unwrap_or_default();
                        let modified_text = metadata
                            .map(|metadata| format::timestamp(metadata.modified, &zone))
                            .unwrap_or_default();
                        let description = entry
                            .info
                            .as_ref()
                            .map(|info| info.description.clone())
                            .unwrap_or_default();
                        let line = entry
                            .surface(theme)
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .size_full()
                            .px(px(space::SM))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap(px(space::SM))
                                    // Icon and name are what gets dragged; the rest
                                    // of the row is free for a rubber band.
                                    .child(Self::drag_source(
                                        div()
                                            .id(("entry-handle", row))
                                            .flex()
                                            .items_center()
                                            .gap(px(space::SM))
                                            .min_w_0()
                                            .when(entry.renaming, |handle| handle.flex_1())
                                            .child(entry.icon(layout.icon, theme))
                                            .child(
                                                this.name_cell(&entry, theme, window, cx)
                                                    .min_w_0()
                                                    .when(entry.renaming, |name| name.flex_1()),
                                            ),
                                        row,
                                        cx,
                                    )),
                            )
                            .when(columns.size, |line| {
                                line.child(
                                    div()
                                        .w(px(size::COLUMN_SIZE))
                                        .flex_none()
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
                                        .truncate()
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
            .child(self.entry_list(snapshot, list))
    }

    /// The icons and compact views: a virtualised grid, `per_row` entries wide.
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
        let cell_width = layout.cell_width.unwrap_or(layout.row_height);
        let stacked = self.mode == ViewMode::Icons;

        let list = uniform_list(
            "entries",
            len.div_ceil(per_row),
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                let _rows = this.frames.task("visible_rows");
                range
                    .map(|line| {
                        let first = line * per_row;
                        let cells = (first..(first + per_row).min(len)).map(|row| {
                            let entry = this.entry(&rows, row, cx);
                            let icon = entry.icon(layout.icon, theme);
                            let surface = entry.surface(theme).size_full();
                            let cell = if stacked {
                                // Icon above a centred name of up to two lines.
                                surface
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(space::SM))
                                    .p(px(space::SM))
                                    .child(icon)
                                    .child(if entry.renaming {
                                        this.name_cell(&entry, theme, window, cx).w_full()
                                    } else {
                                        div()
                                            .w_full()
                                            .text_center()
                                            .line_clamp(2)
                                            .child(entry.name.clone())
                                    })
                            } else {
                                surface
                                    .flex()
                                    .items_center()
                                    .gap(px(space::SM))
                                    .px(px(space::SM))
                                    .child(icon)
                                    .child(
                                        this.name_cell(&entry, theme, window, cx)
                                            .flex_1()
                                            .min_w_0(),
                                    )
                            };
                            let is_folder = entry.container;
                            let cell = Self::drag_source(
                                entry
                                    .accessible(&rows, row, cx)
                                    .flex_none()
                                    .w(px(cell_width))
                                    .h_full()
                                    .p(px(space::XS / 2.))
                                    .child(cell)
                                    .on_click(Self::entry_click(row, cx))
                                    .on_mouse_down(
                                        MouseButton::Middle,
                                        Self::entry_middle_click(row, cx),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        Self::entry_right_click(row, cx),
                                    ),
                                row,
                                cx,
                            );
                            if is_folder {
                                Self::drop_target(cell, Some(row), theme, cx)
                            } else {
                                cell
                            }
                        });
                        div()
                            .flex()
                            .w_full()
                            .h(px(layout.row_height))
                            .px(px(space::SM))
                            .children(cells)
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.scroll)
        .flex_1()
        .py(px(space::XS));
        self.entry_list(snapshot, list)
    }

    /// Shown instead of the list when it has no rows: says why and what to do.
    fn empty_state(
        &self,
        snapshot: &Snapshot,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let hidden = snapshot.total();
        let (icon, heading) = if snapshot.is_loading() {
            (IconName::Loader, "Loading\u{2026}")
        } else {
            (
                IconName::FolderOpen,
                if hidden > 0 {
                    "This folder has only hidden items"
                } else {
                    "This folder is empty"
                },
            )
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
        let content = div()
            .flex()
            .flex_col()
            .w_full()
            .min_h_full()
            .p(px(space::LG))
            .items_center()
            .justify_center()
            .gap(px(space::MD))
            .text_color(theme.text_muted)
            .child(Icon::new(icon).size(px(size::ICON_LARGE)).flex_none())
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_HEADING)
                    .text_size(px(text::HEADING))
                    .text_color(theme.text)
                    .child(heading),
            )
            .when(hidden > 0 && !snapshot.is_loading(), |state| {
                state
                    .child(div().flex_none().child(hidden_message.clone()))
                    .child(
                        div()
                            .flex_none()
                            .max_w_full()
                            .track_focus(&self.folder_action_focus)
                            .key_context(FOLDER_ACTION_CONTEXT)
                            .child(
                                Button::new("show-hidden")
                                    .primary()
                                    .small()
                                    .max_w_full()
                                    .px(px(space::XS))
                                    .label("Show hidden")
                                    .accessibility_label("Show hidden files")
                                    .tooltip("Show hidden files (Ctrl+H)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_hidden(window, cx);
                                    })),
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
                    .aria_description(format!("{hidden_message} Use arrows, Page Up/Down or Home/End to scroll this message.").trim().to_owned())
                    .a11y_synthetic_children(|builder| builder.parent_node().set_live(gpui_kit::accesskit::Live::Polite))
                    .size_full()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.folder_state_scroll)
                    .child(content),
            )
            .child(
                Scrollbar::vertical(&self.folder_state_scroll)
                    .mode(ScrollbarMode::Always)
                    .styles(|styles| {
                        styles
                            .track(|style| style.bg(gpui_kit::transparent_black()))
                            .thumb(|style| style.bg(theme.text_muted).radius(px(size::RADIUS)))
                            .thumb_hover(|style| style.bg(theme.accent))
                            .thumb_active(|style| style.bg(theme.accent))
                    }),
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

    fn status(&self, theme: Theme) -> impl IntoElement {
        let message = match self.current_snapshot() {
            None => "Loading\u{2026}".to_owned(),
            Some(snapshot) => {
                let shown = snapshot.len();
                let hidden = snapshot.total() - shown;
                let selected = self.selection.len();
                let mut message = if selected > 0 {
                    format!(
                        "{} of {} {} selected",
                        count(selected),
                        count(shown),
                        plural(shown, "item", "items")
                    )
                } else {
                    format!("{} {}", count(shown), plural(shown, "item", "items"))
                };
                if hidden > 0 {
                    message.push_str(&format!(", {} hidden", count(hidden)));
                }
                if snapshot.is_loading() {
                    message.push_str(" \u{2014} loading\u{2026}");
                }
                message
            }
        };
        div()
            .id("file-status")
            .role(Role::Label)
            .aria_value(message.clone())
            .flex()
            .flex_none()
            .items_center()
            .h(px(size::STATUS))
            .px(px(space::LG))
            .bg(theme.chrome)
            .border_t_1()
            .border_color(theme.border)
            .font_family(text::FAMILY_DATA)
            .text_size(px(text::SMALL))
            .text_color(theme.text_muted)
            .child(div().flex_1().min_w_0().truncate().child(message))
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
                        .max_h(max_height),
                ),
                _ => None,
            })
        });
        // Open upward from the entry when the menu does not fit below it.
        let viewport = f32::from(window.viewport_size().height);
        let height = menu::column_height(
            &entries,
            size::ROW,
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
                            .key_context(MENU_CONTEXT)
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
        div()
            .id(column)
            .when(nested, |column| {
                column.role(Role::Menu).aria_label("Open with applications")
            })
            .flex()
            .flex_col()
            .w(px(size::MENU))
            .overflow_y_scroll()
            .p(px(space::XS))
            .rounded(px(size::RADIUS))
            .bg(theme.raised)
            .border_1()
            .border_color(theme.border_accent)
            .shadow(vec![theme.floating_shadow()])
            .font_family(text::FAMILY_BODY)
            .text_size(px(text::BODY))
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
                .h(px(1.))
                .my(px(space::XS))
                .bg(theme.border)
                .into_any_element(),
            menu::MenuEntry::Submenu { label, .. } => self
                .menu_button(label, true, true, current, theme, cx)
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
                self.menu_button(label, enabled, false, current && enabled, theme, cx)
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
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.run_menu(command.clone(), window, cx);
                            },
                        ))
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

    fn menu_button(
        &self,
        label: &str,
        enabled: bool,
        submenu: bool,
        current: bool,
        theme: Theme,
        _: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        div()
            .id(SharedString::from(format!(
                "menu-{}-{label}",
                if submenu { "sub" } else { "item" },
            )))
            .role(Role::MenuItem)
            .aria_label(label.to_owned())
            .when(!enabled, |row| {
                row.aria_description("Unavailable.")
                    .a11y_synthetic_children(|builder| builder.parent_node().set_disabled())
            })
            .when(submenu, |row| {
                row.a11y_synthetic_children(|builder| {
                    builder
                        .parent_node()
                        .set_has_popup(gpui_kit::accesskit::HasPopup::Menu)
                })
            })
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::ROW))
            .px(px(space::SM))
            .rounded(px(size::RADIUS))
            .border_l(px(size::SELECTION_BAR))
            .border_color(gpui_kit::transparent_black())
            .when(current, |row| row.border_color(theme.accent))
            .when(current, |row| {
                row.bg(theme.selection).text_color(theme.on_selection)
            })
            .when(!current && enabled, |row| row.text_color(theme.text))
            .when(!enabled, |row| row.text_color(theme.text_muted))
            .child(div().flex_1().min_w_0().truncate().child(label.to_owned()))
            .when(submenu, |row| {
                row.child(
                    Icon::new(IconName::ChevronRight)
                        .size(px(size::ICON_SMALL))
                        .text_color(if current {
                            theme.on_selection
                        } else {
                            theme.text_muted
                        }),
                )
            })
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
        let frame_started = self.frames.begin();
        let theme = *cx.global::<Theme>();
        let snapshot = self.current_snapshot().cloned();
        // A pane in a split is narrower than the window; whatever contains
        // this view says how wide it is.
        if self.width <= 0. {
            self.width = f32::from(window.viewport_size().width);
        }
        let width = self.width;
        let columns = Columns::for_width(width);
        let layout = self.mode.layout(self.zoom);
        // Cells fill the width inside the list's side gutters.
        self.per_row = layout.columns(width - space::SM * 2.);
        let per_row = self.per_row;
        let lines = snapshot
            .as_ref()
            .map_or(0, |snapshot| snapshot.len().div_ceil(per_row));
        if let Some(line) = self.frames.next_bench_row(lines) {
            self.scroll.scroll_to_item(line, ScrollStrategy::Top);
            window.request_animation_frame();
        }

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
                    this.move_selection(Motion::By(-(this.per_row as isize)), false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                if !this.scroll_folder_state("down", cx) {
                    this.move_selection(Motion::By(this.per_row as isize), false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectFirst, _, cx| {
                if !this.scroll_folder_state("home", cx) {
                    this.move_selection(Motion::First, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &SelectLast, _, cx| {
                if !this.scroll_folder_state("end", cx) {
                    this.move_selection(Motion::Last, false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PageUp, _, cx| {
                if !this.scroll_folder_state("pageup", cx) {
                    this.move_selection(Motion::By(-PAGE_ROWS * this.per_row as isize), false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &PageDown, _, cx| {
                if !this.scroll_folder_state("pagedown", cx) {
                    this.move_selection(Motion::By(PAGE_ROWS * this.per_row as isize), false, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &ExtendPrevious, _, cx| {
                this.move_selection(Motion::By(-(this.per_row as isize)), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendNext, _, cx| {
                this.move_selection(Motion::By(this.per_row as isize), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendFirst, _, cx| {
                this.move_selection(Motion::First, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendLast, _, cx| {
                this.move_selection(Motion::Last, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageUp, _, cx| {
                this.move_selection(Motion::By(-PAGE_ROWS * this.per_row as isize), true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendPageDown, _, cx| {
                this.move_selection(Motion::By(PAGE_ROWS * this.per_row as isize), true, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| {
                this.move_sideways(-1, false, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| {
                this.move_sideways(1, false, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendLeft, _, cx| {
                this.move_sideways(-1, true, cx);
            }))
            .on_action(cx.listener(|this, _: &ExtendRight, _, cx| {
                this.move_sideways(1, true, cx);
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
            .on_action(cx.listener(|this, _: &CancelEdit, window, cx| {
                if this.renaming.is_some() {
                    this.finish_rename(false, window, cx);
                } else if this.editing_location {
                    this.stop_editing(window, cx);
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
            .child(self.toolbar(theme, window, cx))
            .when_some(self.failure.as_ref(), |view, failure| {
                view.child(self.failure_banner(failure, theme, cx))
            })
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
                        Some(snapshot) if !snapshot.is_empty() => {
                            if self.mode == ViewMode::Details {
                                content.child(self.list(snapshot, layout, columns, theme, cx))
                            } else {
                                content.child(self.grid(snapshot, layout, per_row, theme, cx))
                            }
                        }
                        Some(snapshot) => content.child(
                            self.entry_list(snapshot, self.empty_state(snapshot, theme, cx)),
                        ),
                        None => content,
                    }),
            )
            .child(self.status(theme))
            .children(self.band_outline(theme))
            .children(self.menu_layer(theme, window, cx));
        self.frames.end(frame_started);
        view
    }
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

/// What follows the pointer during a drag: the name of the entry, or how many.
struct DragPreview(SharedString);

impl Render for DragPreview {
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
            .child(self.0.clone())
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

/// A keyboard movement of the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    /// Up (negative) or down (positive) by a number of rows.
    By(isize),
    First,
    Last,
}

/// The row a motion lands on in a list of `len` rows, or `None` for an empty
/// list. With nothing selected, moving down starts at the top and moving up at
/// the bottom.
fn target_row(current: Option<usize>, len: usize, motion: Motion) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match (motion, current) {
        (Motion::First, _) => 0,
        (Motion::Last, _) => last,
        (Motion::By(delta), Some(row)) => row.saturating_add_signed(delta).min(last),
        (Motion::By(delta), None) if delta >= 0 => 0,
        (Motion::By(_), None) => last,
    })
}

/// Turns typed text into an absolute path: expands a leading `~` and resolves
/// relative paths against `base`. Returns `None` for empty input.
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
    selected: bool,
    /// Has the keyboard focus.
    focused: bool,
    /// Its name is being edited in place.
    renaming: bool,
    /// Cut and waiting to be pasted.
    cut: bool,
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
            .aria_description(if self.container { "Folder" } else { "File" })
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
    fn surface(&self, theme: Theme) -> gpui_kit::Div {
        div()
            .rounded(px(size::RADIUS))
            .border_l(px(size::SELECTION_BAR))
            .border_color(gpui_kit::transparent_black())
            .text_color(theme.text)
            .when(self.selected, |surface| {
                surface
                    .bg(theme.selection)
                    .border_color(theme.accent)
                    .text_color(theme.on_selection)
            })
            .when(!self.selected, |surface| {
                surface
                    .when(self.focused, |surface| surface.border_color(theme.accent))
                    .hover(|surface| surface.bg(theme.hover))
            })
            // Cut entries are on their way out: drawn faded until pasted.
            .when(self.cut, |surface| surface.opacity(0.5))
    }
}

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

/// Which navigation buttons fit the pane. The rest stay on their shortcuts
/// and in the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Navigation {
    back: bool,
    forward_and_up: bool,
}

impl Navigation {
    /// Forward and Up leave first. Back leaves only when the pane can no
    /// longer hold it beside the location and Actions.
    fn for_width(width: f32) -> Self {
        Self {
            back: width >= size::TOOLBAR_BACK_MIN,
            forward_and_up: width >= size::TOOLBAR_FULL_MIN,
        }
    }
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

    #[test]
    fn repeating_one_letter_cycles_and_a_longer_prefix_narrows() {
        assert_eq!(type_ahead_query("f"), ("f", true));
        assert_eq!(type_ahead_query("fff"), ("f", true));
        assert_eq!(type_ahead_query("fi"), ("fi", false));
        assert_eq!(type_ahead_query("\u{e9}\u{e9}"), ("\u{e9}", true));
        assert_eq!(type_ahead_query(""), ("", false));
    }

    #[test]
    fn end_selects_the_last_row_even_with_nothing_selected() {
        assert_eq!(target_row(None, 500_000, Motion::Last), Some(499_999));
        assert_eq!(target_row(Some(3), 10, Motion::Last), Some(9));
    }

    #[test]
    fn home_selects_the_first_row() {
        assert_eq!(target_row(Some(7), 10, Motion::First), Some(0));
        assert_eq!(target_row(None, 10, Motion::First), Some(0));
    }

    #[test]
    fn stepping_is_clamped_to_the_list() {
        assert_eq!(target_row(Some(0), 10, Motion::By(-1)), Some(0));
        assert_eq!(target_row(Some(9), 10, Motion::By(1)), Some(9));
        assert_eq!(target_row(Some(4), 10, Motion::By(15)), Some(9));
        assert_eq!(target_row(Some(4), 10, Motion::By(-15)), Some(0));
        assert_eq!(target_row(Some(4), 10, Motion::By(2)), Some(6));
    }

    #[test]
    fn first_step_without_a_selection_enters_from_the_matching_edge() {
        assert_eq!(target_row(None, 10, Motion::By(1)), Some(0));
        assert_eq!(target_row(None, 10, Motion::By(-1)), Some(9));
    }

    #[test]
    fn an_empty_list_has_no_target() {
        assert_eq!(target_row(None, 0, Motion::Last), None);
        assert_eq!(target_row(None, 0, Motion::By(1)), None);
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
    fn narrow_panes_drop_forward_and_up_then_back() {
        let wide = Navigation::for_width(size::TOOLBAR_FULL_MIN);
        assert!(wide.back && wide.forward_and_up);
        let medium = Navigation::for_width(size::TOOLBAR_FULL_MIN - 1.);
        assert!(medium.back && !medium.forward_and_up);
        let back_only = Navigation::for_width(size::TOOLBAR_BACK_MIN);
        assert!(back_only.back && !back_only.forward_and_up);
        let narrow = Navigation::for_width(size::TOOLBAR_BACK_MIN - 1.);
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
