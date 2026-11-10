//! The window's content: tabs, each showing one folder or two side by side.

use crate::controls::{Destructive as _, Outlined as _};
use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, Global,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _, Render,
    Role, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    Window, actions,
    component::{
        Disableable as _, Icon, IconName, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        scroll::{Scrollbar, ScrollbarMode},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    FileInfo, display_name,
    ops::{ConflictChoice, Recovery, Request},
};

use crate::{
    browser::{Browser, BrowserEvent, DraggedEntries, EditLocation},
    commands::{self, Scope},
    dialog, format,
    history::history,
    location::Location,
    operations::{self, Hub, Notice, Question, Running, hub},
    palette::{Candidate, Palette, PaletteEvent, Target},
    properties::{PropertiesDialog, SizeLine},
    remote,
    session::{Session, SessionSaves, SessionTab},
    settings::Settings,
    sidebar::{Sidebar, SidebarEvent},
    tabs::Tabs,
    theme::{Theme, size, space, text},
};

#[path = "appearance.rs"]
mod appearance;

/// Keeps the file-manager service connected for the life of the application.
pub(crate) struct ShowService(pub(crate) rustphin_platform::file_manager::ServiceHandle);
impl Global for ShowService {}

const KEY_CONTEXT: &str = "Workspace";
/// A dialog that is waiting for an answer.
const DIALOG_CONTEXT: &str = "Dialog";

actions!(
    workspace,
    [
        NewTab,
        CloseTab,
        NextTab,
        PreviousTab,
        MoveTabLeft,
        MoveTabRight,
        ToggleSplit,
        OtherPane,
        Undo,
        DialogAccept,
        DialogCancel,
        DialogSkip,
        DialogKeepBoth,
        DialogReplace,
        DialogToggleAll,
        DialogCopy,
        DialogOpen,
        ToggleSidebar,
        AddPlace,
        OpenPalette,
        StopOperations,
        DismissNotice,
        FocusPlaces,
        ReopenClosedTab,
        FocusOtherView,
        CopyToOtherView,
        MoveToOtherView,
        Quit,
        ActivateTab1,
        ActivateTab2,
        ActivateTab3,
        ActivateTab4,
        ActivateTab5,
        ActivateTab6,
        ActivateTab7,
        ActivateTab8,
        ActivateTab9,
        ActivateLastTab,
    ]
);

/// Whether Shift+F5 copies or Shift+F6 moves to the other view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transfer {
    Copy,
    Move,
}

/// The operation that sends the active view's selection into the other
/// view's folder, as Dolphin's copy and move to the other view, or why there is
/// nothing to do.
fn transfer_to_other_view(
    transfer: Transfer,
    selected: Vec<PathBuf>,
    from: Option<&Path>,
    to: Option<&Path>,
) -> Result<Request, &'static str> {
    let Some(into) = to else {
        return Err("Split the view with F3 to copy or move to the other side.");
    };
    if selected.is_empty() {
        return Err("Select what to copy or move first.");
    }
    if from == Some(into) {
        return Err(
            "Both views show the same folder. Open another folder in the other view first.",
        );
    }
    let into = into.to_path_buf();
    Ok(match transfer {
        Transfer::Copy => Request::Copy {
            sources: selected,
            into,
        },
        Transfer::Move => Request::Move {
            sources: selected,
            into,
        },
    })
}

/// How many closed tabs Ctrl+Shift+T can bring back, as in Dolphin.
const CLOSED_TAB_LIMIT: usize = 6;

/// Records a closed tab, newest first, forgetting the oldest past the limit.
/// A tab with no saved location cannot be reopened and is not kept.
fn remember_closed_tab(closed: &mut Vec<SessionTab>, tab: SessionTab) {
    if tab.panes.is_empty() {
        return;
    }
    closed.insert(0, tab);
    closed.truncate(CLOSED_TAB_LIMIT);
}

/// The tab Alt+`number` activates, as in Dolphin: Alt+1 to Alt+9 pick that
/// tab if it exists, Alt+0 the last one.
fn tab_for_number(number: usize, tabs: usize) -> Option<usize> {
    match number {
        0 => tabs.checked_sub(1),
        n => (n <= tabs).then(|| n - 1),
    }
}

/// A pane's file list, not a text field. Its actions bubble up to the
/// workspace. (An identifier matches one element's context, so
/// `Workspace && Browser` would never match.)
const LIST_IN_WORKSPACE: &str = "Browser && !Input && !Dialog && !Palette";

/// Workspace focus outside the panes: the places panel, the tab strip.
const OUTSIDE_PANES: &str = "Workspace && !Dialog && !Palette && !Browser && !Input";

/// Registers the tab and split-view shortcuts. They match Dolphin's defaults.
pub fn bind_keys(cx: &mut App) {
    let context = Some("Workspace && !Dialog && !Palette");
    cx.bind_keys([
        KeyBinding::new(
            "ctrl-shift-p",
            OpenPalette,
            Some("Workspace && !Dialog && !Palette"),
        ),
        // Dolphin enters a location from anywhere in the window. Inside a
        // pane the browser's own binding applies; text fields keep their keys.
        KeyBinding::new("ctrl-l", EditLocation, Some(OUTSIDE_PANES)),
        KeyBinding::new("f6", EditLocation, Some(OUTSIDE_PANES)),
        KeyBinding::new("alt-d", EditLocation, Some(OUTSIDE_PANES)),
        KeyBinding::new("ctrl-q", Quit, context),
        KeyBinding::new("ctrl-shift-t", ReopenClosedTab, context),
        KeyBinding::new("ctrl-p", FocusPlaces, context),
        KeyBinding::new("ctrl-f3", FocusOtherView, context),
        KeyBinding::new("shift-f5", CopyToOtherView, Some(LIST_IN_WORKSPACE)),
        KeyBinding::new("shift-f6", MoveToOtherView, Some(LIST_IN_WORKSPACE)),
        KeyBinding::new("alt-1", ActivateTab1, context),
        KeyBinding::new("alt-2", ActivateTab2, context),
        KeyBinding::new("alt-3", ActivateTab3, context),
        KeyBinding::new("alt-4", ActivateTab4, context),
        KeyBinding::new("alt-5", ActivateTab5, context),
        KeyBinding::new("alt-6", ActivateTab6, context),
        KeyBinding::new("alt-7", ActivateTab7, context),
        KeyBinding::new("alt-8", ActivateTab8, context),
        KeyBinding::new("alt-9", ActivateTab9, context),
        KeyBinding::new("alt-0", ActivateLastTab, context),
        KeyBinding::new("ctrl-t", NewTab, context),
        KeyBinding::new("ctrl-w", CloseTab, context),
        KeyBinding::new("ctrl-tab", NextTab, context),
        KeyBinding::new("ctrl-pagedown", NextTab, context),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, context),
        KeyBinding::new("ctrl-pageup", PreviousTab, context),
        KeyBinding::new("ctrl-shift-pageup", MoveTabLeft, context),
        KeyBinding::new("ctrl-shift-pagedown", MoveTabRight, context),
        KeyBinding::new("f3", ToggleSplit, context),
        KeyBinding::new("f9", ToggleSidebar, context),
        // Ctrl+D is Duplicate, as in Dolphin (D-021).
        KeyBinding::new(
            "ctrl-shift-d",
            AddPlace,
            Some("Workspace && !Input && !Dialog && !Palette"),
        ),
        // Tab moves between the two panes, except while typing in a field.
        KeyBinding::new(
            "tab",
            OtherPane,
            Some("Workspace && !Input && !Dialog && !Palette"),
        ),
        // Not while typing: there Ctrl+Z undoes the typing.
        KeyBinding::new(
            "ctrl-z",
            Undo,
            Some("Workspace && !Input && !Dialog && !Palette"),
        ),
        KeyBinding::new("enter", DialogAccept, Some(DIALOG_CONTEXT)),
        KeyBinding::new("escape", DialogCancel, Some(DIALOG_CONTEXT)),
        // A text field inside a dialog has its own context. Escape still cancels.
        KeyBinding::new("escape", DialogCancel, Some("Dialog > Input")),
        KeyBinding::new("s", DialogSkip, Some(DIALOG_CONTEXT)),
        KeyBinding::new("k", DialogKeepBoth, Some(DIALOG_CONTEXT)),
        KeyBinding::new("r", DialogReplace, Some(DIALOG_CONTEXT)),
        KeyBinding::new("a", DialogToggleAll, Some(DIALOG_CONTEXT)),
        KeyBinding::new("c", DialogCopy, Some(DIALOG_CONTEXT)),
        KeyBinding::new("o", DialogOpen, Some(DIALOG_CONTEXT)),
    ]);
}

/// One folder view inside a tab, with what keeps it connected to the workspace.
struct Pane {
    browser: Entity<Browser>,
    _subscriptions: [Subscription; 2],
}

/// A tab: one pane, or two when split.
struct Tab {
    panes: Vec<Pane>,
    active: usize,
}

impl Tab {
    fn browser(&self) -> &Entity<Browser> {
        &self.panes[self.active.min(self.panes.len() - 1)].browser
    }
}

/// The tab being dragged to a new position.
struct DraggedTab {
    index: usize,
    label: SharedString,
}

impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        div()
            .px(px(space::MD))
            .py(px(space::XS))
            .rounded(px(size::RADIUS))
            .bg(theme.selection)
            .text_color(theme.on_selection)
            .text_size(px(text::SMALL))
            .child(self.label.clone())
    }
}

/// Everything shown in one window.
pub struct Workspace {
    appearance: Option<appearance::Appearance>,
    tabs: Tabs<Tab>,
    /// Recently closed tabs, newest first, for Ctrl+Shift+T.
    closed_tabs: Vec<SessionTab>,
    settings_path: Option<PathBuf>,
    /// Where the open tabs are stored; `None` when they must not be written.
    session_path: Option<PathBuf>,
    /// The session as last queued, to skip writes that change nothing.
    saved: Session,
    session_saves: SessionSaves,
    title: String,
    /// Holds the keyboard focus while a dialog is open.
    dialog_focus: FocusHandle,
    /// The question and facts scroll independently of their heading/actions.
    question_scroll: ScrollHandle,
    /// Whether a dialog was open at the last redraw, to move the focus when
    /// one opens or the last one closes.
    dialog_open: bool,
    /// The "do this for all" box of the conflict dialog.
    apply_to_all: bool,
    /// The zone dates are shown in.
    zone: TimeZone,
    sidebar: Entity<Sidebar>,
    sidebar_visible: bool,
    _hub: Subscription,
    _sidebar: Subscription,
    palette: Option<Entity<Palette>>,
    palette_subscription: Option<Subscription>,
    palette_from_sidebar: bool,
    /// Batch rename, when several files are being renamed together.
    pub(crate) rename: Option<crate::rename_dialog::RenameDialog>,
    /// Folders other applications asked to be shown, applied on the next frame.
    pending_shows: Vec<rustphin_platform::file_manager::Shown>,
    /// A program or shortcut waiting for confirmation before it runs.
    launch: Option<LaunchPrompt>,
    /// Properties of one entry, while that dialog is open.
    pub(crate) properties: Option<PropertiesDialog>,
    /// Redraws the properties dialog when a thumbnail is ready, while it is open.
    properties_thumbs: Option<Subscription>,
    /// The error details dialog: what the last operation left out.
    error_details: Option<operations::Problems>,
    /// Distinguishes a properties read from one the user has already closed.
    property_token: u64,
    /// Paths other applications asked to see properties for.
    pending_properties: Vec<PathBuf>,
    _history: Subscription,
    _places: Subscription,
}

impl Workspace {
    /// Creates a workspace showing the tabs of `session`, or one tab on
    /// `fallback` if it has none.
    pub fn new(
        session: Session,
        fallback: PathBuf,
        session_path: Option<PathBuf>,
        settings: Settings,
        settings_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let session = if session.tabs.is_empty() {
            Session::single(fallback.clone())
        } else {
            session
        };
        let tabs: Vec<Tab> = session
            .tabs
            .iter()
            .map(|tab| Tab {
                panes: tab
                    .panes
                    .iter()
                    .map(|location| {
                        Self::pane_at(
                            location,
                            &fallback,
                            settings.clone(),
                            settings_path.clone(),
                            window,
                            cx,
                        )
                    })
                    .collect(),
                active: tab.active_pane,
            })
            .collect();
        let tabs = Tabs::from_parts(tabs, session.active).unwrap_or_else(|| {
            Tabs::new(Tab {
                panes: vec![Self::pane(
                    fallback,
                    settings,
                    settings_path.clone(),
                    window,
                    cx,
                )],
                active: 0,
            })
        });
        let sidebar = cx.new(|cx| Sidebar::new(window, cx));
        let sidebar_events = cx.subscribe_in(&sidebar, window, Self::on_sidebar_event);
        let workspace = Self {
            appearance: None,
            tabs,
            closed_tabs: Vec::new(),
            settings_path,
            session_path,
            saved: Session::default(),
            session_saves: SessionSaves::default(),
            title: String::new(),
            dialog_focus: cx.focus_handle(),
            question_scroll: ScrollHandle::new(),
            dialog_open: false,
            apply_to_all: false,
            zone: TimeZone::system(),
            // Progress, questions and messages change outside this view.
            _hub: cx.observe(&hub(cx), |_, _, cx| cx.notify()),
            sidebar: sidebar.clone(),
            sidebar_visible: true,
            _sidebar: sidebar_events,
            palette: None,
            palette_subscription: None,
            palette_from_sidebar: false,
            rename: None,
            pending_shows: Vec::new(),
            launch: None,
            properties: None,
            error_details: None,
            properties_thumbs: None,
            property_token: 0,
            pending_properties: Vec::new(),
            _history: cx.observe(&history(cx), |this, _, cx| this.refresh_palette(cx)),
            _places: cx.observe(&sidebar, |this, _, cx| this.refresh_palette(cx)),
        };
        if let Some(service) = cx.try_global::<ShowService>() {
            let incoming = service.0.receiver();
            cx.spawn(async move |this, cx| {
                loop {
                    let queue = incoming.clone();
                    let next = cx
                        .background_spawn(async move { queue.recv_async().await })
                        .await;
                    let Ok(call) = next else { break };
                    if this
                        .update(cx, |this, cx| {
                            match call {
                                rustphin_platform::file_manager::Call::Show(shown) => {
                                    this.pending_shows.extend(shown);
                                }
                                rustphin_platform::file_manager::Call::Properties(paths) => {
                                    this.pending_properties.extend(paths);
                                }
                            }
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        workspace.focus_active(window, cx);
        workspace
    }

    /// One tab as the session and the closed-tab list keep it.
    fn saved_tab(tab: &Tab, cx: &App) -> SessionTab {
        SessionTab {
            panes: tab
                .panes
                .iter()
                .filter_map(|pane| pane.browser.read(cx).saved_location(cx))
                .collect(),
            active_pane: tab.active,
        }
    }

    /// The open tabs and panes as a session.
    fn session(&self, cx: &App) -> Session {
        let tabs = self
            .tabs
            .iter()
            .map(|tab| Self::saved_tab(tab, cx))
            .filter(|tab| !tab.panes.is_empty())
            .collect();
        Session::new(tabs, self.tabs.active_index())
    }

    /// Stores the session if it changed, on a background thread.
    fn save_session(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.session_path.clone() else {
            return;
        };
        let session = self.session(cx);
        if session == self.saved || session.tabs.is_empty() {
            return;
        }
        let save = match self.session_saves.prepare(session.clone()) {
            Ok(save) => save,
            Err(error) => {
                tracing::warn!(%error, "could not queue the session save");
                return;
            }
        };
        self.saved = session;
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = save.write(&path) {
                    tracing::warn!(%error, path = %path.display(), "could not save the session");
                }
            })
            .detach();
    }

    /// A pane showing a saved location. A remote one shows `fallback` until
    /// kio-fuse has mounted it again, as Dolphin reconnects its tabs.
    fn pane_at(
        location: &Location,
        fallback: &Path,
        settings: Settings,
        settings_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Pane {
        let (path, remote) = match location {
            Location::Folder(path) => (path.clone(), None),
            Location::Remote(address) => (fallback.to_path_buf(), Some(address)),
        };
        let pane = Self::pane(path, settings, settings_path, window, cx);
        if let Some(address) = remote {
            pane.browser.update(cx, |browser, cx| {
                browser.open_remote(address.clone(), true, cx);
            });
        }
        pane
    }

    fn pane(
        path: PathBuf,
        settings: Settings,
        settings_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Pane {
        let browser = cx.new(|cx| Browser::new(path, settings, settings_path, window, cx));
        let subscriptions = [
            // Tab labels and the window title follow the folder being shown.
            cx.observe(&browser, |_, _, cx| cx.notify()),
            cx.subscribe_in(&browser, window, Self::on_browser_event),
        ];
        Pane {
            browser,
            _subscriptions: subscriptions,
        }
    }

    fn on_browser_event(
        &mut self,
        _: &Entity<Browser>,
        event: &BrowserEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            BrowserEvent::OpenInNewTab(path) => self.open_tab(path.clone(), false, window, cx),
            BrowserEvent::Visited(path) => {
                let location = remote::location_of(path.clone(), cx);
                history(cx).read(cx).visit(location);
            }
            BrowserEvent::Activate(path) => self.consider_launch(path.clone(), cx),
            BrowserEvent::Properties {
                path,
                type_label,
                look,
            } => {
                self.open_properties(path.clone(), type_label.clone(), look.clone(), cx);
            }
            BrowserEvent::OpenWith { id, path } => {
                self.open_with(id.clone(), path.clone(), cx);
            }
            BrowserEvent::BatchRename {
                directory,
                selected,
                staying,
            } => {
                if !hub(cx).read(cx).questions.is_empty() {
                    return;
                }
                self.palette = None;
                self.palette_subscription = None;
                self.rename = Some(crate::rename_dialog::RenameDialog::open(
                    directory.clone(),
                    selected.clone(),
                    staying.clone(),
                    window,
                    cx,
                ));
                cx.notify();
            }
        }
    }

    /// Opens the properties dialog and reads the entry off the UI thread.
    fn open_properties(
        &mut self,
        path: PathBuf,
        type_label: String,
        look: crate::properties::Look,
        cx: &mut Context<Self>,
    ) {
        if self.properties.is_some()
            || self.launch.is_some()
            || self.rename.is_some()
            || !hub(cx).read(cx).questions.is_empty()
        {
            return;
        }
        self.property_token += 1;
        let token = self.property_token;
        // A picture's thumbnail may arrive while the dialog is open.
        self.properties_thumbs = look
            .picture
            .then(|| cx.observe(&crate::thumbs::thumbs(cx), |_, _, cx| cx.notify()));
        self.properties = Some(PropertiesDialog::loading(
            path.clone(),
            type_label,
            look,
            token,
        ));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_spawn(async move { rustphin_platform::properties::read(&path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(dialog) = this
                    .properties
                    .as_mut()
                    .filter(|dialog| dialog.token == token)
                else {
                    return;
                };
                match read {
                    Ok(facts) => {
                        let directory =
                            facts.kind == rustphin_platform::properties::Kind::Directory;
                        dialog.set_facts(facts);
                        cx.notify();
                        if directory {
                            this.measure_folder(token, cx);
                        }
                    }
                    Err(error) => {
                        dialog.error = Some(format!("Could not read this item: {error}."));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    /// Measures a folder's size without blocking the window.
    fn measure_folder(&mut self, token: u64, cx: &mut Context<Self>) {
        let Some(dialog) = self
            .properties
            .as_ref()
            .filter(|dialog| dialog.token == token)
        else {
            return;
        };
        let path = dialog.path.clone();
        let cancel = Arc::clone(&dialog.cancel);
        let seen = Arc::clone(&dialog.seen);
        // Shows how many entries the walk has found while it runs.
        cx.spawn({
            async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(250))
                        .await;
                    let running = this
                        .update(cx, |this, cx| {
                            let waiting = this.properties.as_ref().is_some_and(|dialog| {
                                dialog.token == token && dialog.size == SizeLine::Calculating
                            });
                            if waiting {
                                cx.notify();
                            }
                            waiting
                        })
                        .unwrap_or(false);
                    if !running {
                        break;
                    }
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let measured = cx
                .background_spawn(async move {
                    rustphin_platform::properties::folder_size(&path, &cancel, &seen)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(dialog) = this
                    .properties
                    .as_mut()
                    .filter(|dialog| dialog.token == token)
                else {
                    return;
                };
                match measured {
                    Ok(size) if size.cancelled => {}
                    Ok(size) => {
                        dialog.size = SizeLine::Folder {
                            bytes: size.bytes,
                            unreadable: size.unreadable,
                            items: size.items,
                        };
                        cx.notify();
                    }
                    Err(error) => {
                        dialog.size =
                            SizeLine::Failed(format!("The size could not be calculated: {error}."));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    /// Writes the edited permissions through the operation engine, then closes.
    pub(crate) fn apply_properties(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.properties.as_ref() else {
            return;
        };
        if !dialog.editable || dialog.mode == dialog.original {
            return;
        }
        let path = dialog.path.clone();
        let mode = dialog.mode & 0o7777;
        self.close_properties(cx);
        hub(cx).update(cx, |hub, _| hub.start(Request::SetMode { path, mode }));
    }

    /// Closes the properties dialog and stops a folder measurement.
    pub(crate) fn close_properties(&mut self, cx: &mut Context<Self>) {
        if let Some(dialog) = self.properties.take() {
            dialog.cancel.store(true, Ordering::Relaxed);
        }
        self.properties_thumbs = None;
        cx.notify();
    }

    /// Classifies a file off the UI thread, opens a document, or asks before
    /// running a program or a shortcut.
    fn consider_launch(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.launch.is_some() || !hub(cx).read(cx).questions.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let outcome = cx.background_spawn(async move { classify(&path) }).await;
            let _ = this.update(cx, |this, cx| match outcome {
                LaunchOutcome::Opened(Ok(())) => {}
                LaunchOutcome::Opened(Err(message)) => {
                    hub(cx).update(cx, |hub, cx| hub.say(message, true, cx));
                }
                LaunchOutcome::Ask {
                    path,
                    name,
                    program,
                } => {
                    this.launch = Some(LaunchPrompt {
                        path,
                        name,
                        program,
                    });
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Opens a file with one chosen application, off the UI thread.
    fn open_with(&mut self, id: String, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { rustphin_platform::apps::open_with(&id, &path) })
                .await;
            if let Err(error) = result {
                let _ = this.update(cx, |_, cx| {
                    hub(cx).update(cx, |hub, cx| hub.say(error.to_string(), true, cx));
                });
            }
        })
        .detach();
    }

    /// The confirmation before a program or a shortcut runs.
    fn launch_dialog(
        &self,
        prompt: &LaunchPrompt,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let name = prompt.name.clone();
        let program = prompt.program;
        let body = if program {
            "This file is a program. Running it executes it on this computer. Open hands it to the default application instead."
        } else {
            "This is an application shortcut. Running it starts that application."
        };
        let location = prompt
            .path
            .parent()
            .map(|folder| folder.display().to_string());
        let panel = dialog::panel(
            div()
                .id("launch-question")
                .role(Role::Dialog)
                .aria_label(format!("Run “{name}”?"))
                .aria_description(format!(
                    "{body} Enter or Escape keeps it; R runs it{}.",
                    if program { "; O opens it" } else { "" }
                ))
                .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                .key_context(DIALOG_CONTEXT)
                .track_focus(&self.dialog_focus)
                .on_key_down(cx.listener(Self::scroll_question))
                // The safe choice is the default, as Dolphin focuses Cancel
                // (D-024): Enter keeps the file; running it takes R.
                .on_action(cx.listener(|this, _: &DialogAccept, _, cx| {
                    this.answer_launch(false, false, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogCancel, _, cx| {
                    this.answer_launch(false, false, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogReplace, _, cx| {
                    this.answer_launch(true, false, cx);
                }))
                .on_action(cx.listener(move |this, _: &DialogOpen, _, cx| {
                    if program {
                        this.answer_launch(false, true, cx);
                    }
                })),
            size::DIALOG,
            theme,
            window,
        )
        .child(dialog::title_row(
            gpui_kit::assets::IconName::Terminal,
            format!("Run “{name}”?"),
            "Cancel (Esc)",
            cx.listener(|this, _, _, cx| this.answer_launch(false, false, cx)),
            theme,
        ))
        .child(
            dialog::body().child(
                self.question_body(
                    body.to_owned(),
                    location
                        .map(|folder| {
                            dialog::fact(
                                "fact-location",
                                "Location",
                                size::FACT_LABEL,
                                folder,
                                theme,
                            )
                            .into_any_element()
                        })
                        .into_iter()
                        .collect(),
                    theme,
                ),
            ),
        )
        .child(
            dialog::actions(theme)
                .when(program, |row| {
                    row.child(
                        dialog::button("launch-open", "Open (O)")
                            .outlined()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.answer_launch(false, true, cx);
                            })),
                    )
                })
                .child(
                    dialog::button("launch-run", "Run (R)")
                        .destructive(&theme)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.answer_launch(true, false, cx);
                        })),
                )
                .child(
                    // Cancel leads, as in Dolphin: Enter never runs anything.
                    dialog::button("launch-cancel", "Cancel (Enter)")
                        .primary()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.answer_launch(false, false, cx);
                        })),
                ),
        );
        dialog::scrim(panel, theme)
    }

    /// Applies the run dialog: run the file, open it, or leave it alone.
    fn answer_launch(&mut self, run: bool, open: bool, cx: &mut Context<Self>) {
        let Some(prompt) = self.launch.take() else {
            return;
        };
        self.question_scroll.set_offset(gpui_kit::Point::default());
        if run || open {
            let path = prompt.path;
            let program = prompt.program;
            cx.spawn(async move |this, cx| {
                let result = cx
                    .background_spawn(async move {
                        if open {
                            rustphin_platform::apps::open_with_default(&path)
                        } else if program {
                            rustphin_platform::apps::run_program(&path)
                        } else {
                            rustphin_platform::apps::run_desktop(&path)
                        }
                    })
                    .await;
                if let Err(error) = result {
                    let _ = this.update(cx, |_, cx| {
                        hub(cx).update(cx, |hub, cx| hub.say(error.to_string(), true, cx));
                    });
                }
            })
            .detach();
        }
        cx.notify();
    }

    fn palette_candidates(&self, cx: &App) -> Vec<Candidate> {
        let mut candidates: Vec<_> = commands::commands()
            .iter()
            .enumerate()
            .map(|(index, command)| {
                Candidate::new(
                    command.label.to_owned(),
                    match command.scope {
                        Scope::Browser => "Action · active pane",
                        Scope::Sidebar => "Action · selected place or group",
                    }
                    .to_owned(),
                    command.shortcut.to_owned(),
                    Target::Command(index),
                )
            })
            .collect();
        for (index, tab) in self.tabs.iter().enumerate() {
            if let Some(path) = tab.browser().read(cx).location() {
                candidates.push(Candidate::new(
                    format!("Tab: {}", Self::label(tab, cx)),
                    display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
                    String::new(),
                    Target::Tab(index),
                ));
            }
        }
        for place in self.sidebar.read(cx).palette_places() {
            if let Some(path) = &place.path {
                candidates.push(Candidate::new(
                    format!("Place: {}", display_name(place.title.as_bytes())),
                    display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
                    String::new(),
                    Target::Folder(path.clone()),
                ));
            }
        }
        for location in history(cx).read(cx).locations.paths() {
            candidates.push(match location {
                Location::Folder(path) => Candidate::new(
                    format!("Recent: {}", folder_label(path)),
                    display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
                    String::new(),
                    Target::Folder(path.clone()),
                ),
                Location::Remote(address) => {
                    let shown = address.to_string();
                    Candidate::new(
                        format!("Recent: {}", remote::short_name(&shown)),
                        shown,
                        String::new(),
                        Target::Remote(address.clone()),
                    )
                }
            });
        }
        candidates.push(Candidate::new(
            "History: Clear recent folders".to_owned(),
            "Forget saved folder visits on this computer".to_owned(),
            String::new(),
            Target::ClearHistory,
        ));
        candidates
    }

    fn refresh_palette(&mut self, cx: &mut Context<Self>) {
        if let Some(palette) = self.palette.clone() {
            let candidates = self.palette_candidates(cx);
            let status = history(cx).read(cx).status.clone();
            palette.update(cx, |palette, cx| {
                palette.set_candidates(candidates, status, cx)
            });
        }
    }

    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some()
            || self.rename.is_some()
            || self.properties.is_some()
            || self.launch.is_some()
            || !hub(cx).read(cx).questions.is_empty()
        {
            return;
        }
        self.palette_from_sidebar = self.sidebar.read(cx).is_focused(window, cx);
        let candidates = self.palette_candidates(cx);
        let status = history(cx).read(cx).status.clone();
        let palette = cx.new(|cx| Palette::new(candidates, status, window, cx));
        self.palette_subscription = Some(cx.subscribe_in(&palette, window, Self::on_palette_event));
        self.palette = Some(palette);
        cx.notify();
    }

    fn on_palette_event(
        &mut self,
        _: &Entity<Palette>,
        event: &PaletteEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, PaletteEvent::Chosen(Target::ClearHistory)) {
            history(cx).read(cx).clear();
            return;
        }
        self.palette = None;
        self.palette_subscription = None;
        if self.palette_from_sidebar {
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.focus(window, cx));
        } else {
            self.focus_active(window, cx);
        }
        match event {
            PaletteEvent::Closed | PaletteEvent::Chosen(Target::ClearHistory) => {}
            PaletteEvent::Chosen(Target::Folder(path)) => {
                self.tabs
                    .active()
                    .browser()
                    .update(cx, |browser, cx| browser.go_to(path.clone(), cx));
                self.focus_active(window, cx);
            }
            PaletteEvent::Chosen(Target::Remote(address)) => {
                self.tabs.active().browser().update(cx, |browser, cx| {
                    browser.open_remote(address.clone(), false, cx)
                });
                self.focus_active(window, cx);
            }
            PaletteEvent::Chosen(Target::Tab(index)) => self.activate(*index, window, cx),
            PaletteEvent::Chosen(Target::Command(index)) => {
                if let Some(command) = commands::commands().get(*index) {
                    let action = command.action;
                    match command.scope {
                        Scope::Browser => {
                            self.focus_active(window, cx);
                            window.dispatch_action(action(), cx);
                        }
                        Scope::Sidebar => {
                            self.sidebar_visible = true;
                            let sidebar = self.sidebar.clone();
                            window.on_next_frame(move |window, cx| {
                                sidebar.update(cx, |sidebar, cx| sidebar.focus(window, cx));
                                window.dispatch_action(action(), cx);
                            });
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    fn on_sidebar_event(
        &mut self,
        _: &Entity<Sidebar>,
        event: &SidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SidebarEvent::Open(path) => {
                let path = path.clone();
                self.tabs
                    .active()
                    .browser()
                    .update(cx, |browser, cx| browser.go_to(path, cx));
            }
            SidebarEvent::OpenInNewTab(path) => self.open_tab(path.clone(), false, window, cx),
            SidebarEvent::OpenTrash => {
                self.tabs
                    .active()
                    .browser()
                    .update(cx, |browser, cx| browser.open_trash(cx));
            }
        }
    }

    /// Opens `folder` and selects `names`. The active pane is reused when it
    /// already shows that folder; otherwise a new tab does.
    fn open_showing(
        &mut self,
        folder: PathBuf,
        names: Vec<Vec<u8>>,
        activate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let showing =
            self.tabs.active().browser().read(cx).location().as_deref() == Some(folder.as_path());
        if showing {
            self.tabs.active().browser().update(cx, |browser, cx| {
                browser.reveal(folder, names, cx);
            });
            if activate {
                self.focus_active(window, cx);
            }
            return;
        }
        let settings = self.tabs.active().browser().read(cx).settings();
        let pane = Self::pane(
            folder.clone(),
            settings,
            self.settings_path.clone(),
            window,
            cx,
        );
        pane.browser
            .update(cx, |browser, cx| browser.reveal(folder, names, cx));
        let previous = self.tabs.active_index();
        self.tabs.open(Tab {
            panes: vec![pane],
            active: 0,
        });
        if activate {
            self.focus_active(window, cx);
        } else {
            self.tabs.activate(previous);
        }
        cx.notify();
    }

    /// Opens `path` in a new tab beside the active one, using the active
    /// folder's view settings. With `activate`, the new tab is shown.
    fn open_tab(
        &mut self,
        path: PathBuf,
        activate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings = self.tabs.active().browser().read(cx).settings();
        let pane = Self::pane(path, settings, self.settings_path.clone(), window, cx);
        let previous = self.tabs.active_index();
        self.tabs.open(Tab {
            panes: vec![pane],
            active: 0,
        });
        if activate {
            self.focus_active(window, cx);
        } else {
            self.tabs.activate(previous);
        }
        cx.notify();
    }

    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let location = self.tabs.active().browser().read(cx).location();
        if let Some(path) = location {
            self.open_tab(path, true, window, cx);
        }
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() == 1 {
            // Closing the only tab closes the window, as in Dolphin.
            window.remove_window();
            return;
        }
        if let Some(closed) = self.tabs.close(index) {
            let saved = Self::saved_tab(&closed, cx);
            remember_closed_tab(&mut self.closed_tabs, saved);
        }
        self.focus_active(window, cx);
        cx.notify();
    }

    /// Opens the most recently closed tab again, split panes included.
    fn reopen_closed_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(saved) = self.closed_tabs.first().cloned() else {
            hub(cx).update(cx, |hub, cx| {
                hub.say("There are no closed tabs to reopen.".to_owned(), false, cx);
            });
            return;
        };
        self.closed_tabs.remove(0);
        let current = self.tabs.active().browser().read(cx);
        let settings = current.settings();
        let fallback = current
            .location()
            .or_else(rustphin_platform::xdg::home)
            .unwrap_or_else(|| PathBuf::from("/"));
        let panes = saved
            .panes
            .iter()
            .map(|location| {
                Self::pane_at(
                    location,
                    &fallback,
                    settings.clone(),
                    self.settings_path.clone(),
                    window,
                    cx,
                )
            })
            .collect::<Vec<_>>();
        self.tabs.open(Tab {
            active: saved.active_pane.min(panes.len().saturating_sub(1)),
            panes,
        });
        self.focus_active(window, cx);
        cx.notify();
    }

    fn transfer_to_other_view(&mut self, transfer: Transfer, cx: &mut Context<Self>) {
        let tab = self.tabs.active();
        let active = tab.active.min(tab.panes.len().saturating_sub(1));
        let from = tab.panes[active].browser.read(cx);
        let (selected, here) = (from.selected_paths(), from.location());
        let there = (tab.panes.len() == 2)
            .then(|| tab.panes[1 - active].browser.read(cx).location())
            .flatten();
        let outcome = transfer_to_other_view(transfer, selected, here.as_deref(), there.as_deref());
        hub(cx).update(cx, |hub, cx| match outcome {
            Ok(request) => hub.start(request),
            Err(reason) => hub.say(reason.to_owned(), false, cx),
        });
    }

    fn activate_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = tab_for_number(number, self.tabs.len()) {
            self.activate(index, window, cx);
        }
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.activate(index);
        self.focus_active(window, cx);
        cx.notify();
    }

    pub(crate) fn focus_active(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs
            .active()
            .browser()
            .update(cx, |browser, cx| browser.focus(window, cx));
    }

    /// Splits the active tab into two panes on the same folder, or closes the
    /// pane that is not active.
    fn toggle_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.tabs.active_index();
        let tab = self.tabs.active();
        if tab.panes.len() == 1 {
            let (location, settings) = {
                let browser = tab.browser().read(cx);
                (browser.location(), browser.settings())
            };
            let Some(path) = location else {
                return;
            };
            let pane = Self::pane(path, settings, self.settings_path.clone(), window, cx);
            self.edit_tab(index, |tab| {
                tab.panes.push(pane);
                tab.active = 1;
            });
        } else {
            self.edit_tab(index, |tab| {
                let keep = tab.active.min(tab.panes.len() - 1);
                tab.panes.swap(0, keep);
                tab.panes.truncate(1);
                tab.active = 0;
            });
        }
        self.focus_active(window, cx);
        cx.notify();
    }

    fn other_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.tabs.active_index();
        let panes = self.tabs.active().panes.len();
        let in_sidebar = self.sidebar.read(cx).is_focused(window, cx);
        let active = self.tabs.active().active;
        if in_sidebar {
            // From the places panel to the first pane.
            self.edit_tab(index, |tab| tab.active = 0);
            self.focus_active(window, cx);
        } else if active + 1 < panes {
            self.edit_tab(index, |tab| tab.active = active + 1);
            self.focus_active(window, cx);
        } else if self.sidebar_visible {
            // From the last pane on to the places panel.
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.focus(window, cx));
        } else if panes > 1 {
            self.edit_tab(index, |tab| tab.active = 0);
            self.focus_active(window, cx);
        }
        cx.notify();
    }

    /// Applies `edit` to the tab at `index`, if there is one.
    fn edit_tab(&mut self, index: usize, edit: impl FnOnce(&mut Tab)) {
        if let Some(tab) = self.tabs.get_mut(index) {
            edit(tab);
        }
    }

    /// The label of a tab: the name of the folder its active pane shows.
    fn label(tab: &Tab, cx: &App) -> SharedString {
        let browser = tab.browser().read(cx);
        match browser.location() {
            Some(_) if browser.shows_trash() => "Trash".into(),
            Some(path) => folder_label(&path).into(),
            None => "New tab".into(),
        }
    }

    /// The icon of a tab: its folder's place icon when it is a place (Home,
    /// Pictures, Trash…), otherwise a folder.
    fn tab_icon(&self, tab: &Tab, cx: &App) -> gpui_kit::assets::IconName {
        let browser = tab.browser().read(cx);
        if browser.shows_trash() {
            return gpui_kit::assets::IconName::Trash;
        }
        browser
            .location()
            .and_then(|path| self.sidebar.read(cx).icon_for_folder(&path))
            .unwrap_or(gpui_kit::assets::IconName::Folder)
    }

    fn tab_strip(&self, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tabs.active_index();
        let tabs: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let label = Self::label(tab, cx);
                let is_active = index == active;
                let icon = self.tab_icon(tab, cx);
                div()
                    .id(("tab", index))
                    .role(Role::Tab)
                    .aria_label(label.clone())
                    .aria_selected(is_active)
                    .aria_position_in_set(index)
                    .group("tab")
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(space::XS))
                    .h_full()
                    .w(px(size::TAB))
                    .pl(px(space::MD))
                    .pr(px(space::SM))
                    .cursor_pointer()
                    .border_r_1()
                    .border_color(theme.border)
                    .text_color(if is_active {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    // The active tab takes the toolbar's surface and an
                    // underline, never colour alone.
                    .when(is_active, |tab| {
                        tab.bg(theme.chrome).child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(px(size::SELECTION_BAR))
                                .bg(theme.accent),
                        )
                    })
                    .when(!is_active, |tab| {
                        tab.hover(|tab| tab.bg(theme.hover).text_color(theme.text))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap(px(space::SM))
                            .child(Icon::new(icon).size(px(size::ICON)).flex_none())
                            .child(div().min_w_0().truncate().child(label.clone())),
                    )
                    .child(
                        Button::new(("close-tab", index))
                            .ghost()
                            .with_size(px(size::TAB_CLOSE))
                            .size(px(size::TAB_CLOSE))
                            .child(Icon::new(IconName::Close).size(px(size::ICON_SMALL)))
                            .tooltip("Close tab (Ctrl+W)")
                            .accessibility_label(format!("Close {label} tab (Ctrl+W)"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.close_tab(index, window, cx);
                            })),
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.activate(index, window, cx)),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(move |this, _, window, cx| this.close_tab(index, window, cx)),
                    )
                    .on_drag(DraggedTab { index, label }, |dragged, _, _, cx| {
                        cx.new(|_| DraggedTab {
                            index: dragged.index,
                            label: dragged.label.clone(),
                        })
                    })
                    .drag_over::<DraggedTab>(move |tab, _, _, _| tab.bg(theme.hover))
                    // Entries dropped on a tab go into the folder it shows.
                    .drag_over::<DraggedEntries>(move |tab, _, _, _| tab.bg(theme.hover))
                    .on_drop(
                        cx.listener(move |this, dragged: &DraggedEntries, window, cx| {
                            let into = this
                                .tabs
                                .iter()
                                .nth(index)
                                .and_then(|tab| tab.browser().read(cx).location());
                            if let Some(into) = into {
                                let sources = dragged.paths(cx);
                                let modifiers = window.modifiers();
                                let action = if modifiers.control {
                                    Some(operations::DropAction::Copy)
                                } else if modifiers.shift {
                                    Some(operations::DropAction::Move)
                                } else {
                                    None
                                };
                                hub(cx).update(cx, |hub, cx| {
                                    hub.drop_files(sources, into, action, cx)
                                });
                            }
                        }),
                    )
                    .on_drop(cx.listener(move |this, dragged: &DraggedTab, _, cx| {
                        this.tabs.move_tab(dragged.index, index);
                        cx.notify();
                    }))
            })
            .collect();

        div().flex_none().child(
            div()
                .flex()
                .items_center()
                .h(px(size::TABS))
                .bg(theme.chrome_raised)
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(text::BODY))
                .child(
                    div()
                        .id("folder-tabs")
                        .role(Role::TabList)
                        .aria_label("Open folders")
                        .aria_size_of_set(tabs.len())
                        .aria_orientation(gpui_kit::Orientation::Horizontal)
                        .flex()
                        .items_center()
                        .h_full()
                        .children(tabs),
                )
                .child(
                    div().flex_1().h_full().child(
                        Button::new("new-tab")
                            .ghost()
                            .w(px(size::TAB_ADD))
                            .h_full()
                            .icon(Icon::new(IconName::Plus).size(px(size::ICON)))
                            .tooltip("New tab (Ctrl+T)")
                            .accessibility_label("New tab (Ctrl+T)")
                            .on_click(cx.listener(|this, _, window, cx| this.new_tab(window, cx))),
                    ),
                ),
        )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.pending_properties.is_empty()
            && self.properties.is_none()
            && self.launch.is_none()
            && self.rename.is_none()
            && hub(cx).read(cx).questions.is_empty()
        {
            let path = self.pending_properties.remove(0);
            self.open_properties(path, "File".to_owned(), Default::default(), cx);
        }
        if !self.pending_shows.is_empty() {
            let pending = std::mem::take(&mut self.pending_shows);
            for (index, shown) in pending.into_iter().enumerate() {
                let names = shown
                    .select
                    .iter()
                    .map(|name| name.as_bytes().to_vec())
                    .collect();
                self.open_showing(shown.folder, names, index == 0, window, cx);
            }
        }
        let theme = *cx.global::<Theme>();

        // The pane that holds the keyboard focus is the active one.
        let index = self.tabs.active_index();
        let focused = self
            .tabs
            .active()
            .panes
            .iter()
            .position(|pane| pane.browser.read(cx).is_focused(window, cx));
        if let Some(focused) = focused
            && focused != self.tabs.active().active
        {
            self.edit_tab(index, |tab| tab.active = focused);
        }

        // Every change to tabs, panes or folders leads here, so this is where
        // the session is kept up to date.
        self.save_session(cx);

        // A dialog takes the keyboard while it is open and gives it back to
        // the active pane when the last one closes.
        let hub = hub(cx);
        let dialog_open = !hub.read(cx).questions.is_empty()
            || self.launch.is_some()
            || self.properties.is_some()
            || self.error_details.is_some();
        if dialog_open && self.palette.is_some() {
            self.palette = None;
            self.palette_subscription = None;
        }
        if dialog_open {
            self.rename = None;
        }
        if dialog_open && !self.dialog_focus.is_focused(window) {
            let focus = self.dialog_focus.clone();
            cx.defer_in(window, move |_, window, cx| focus.focus(window, cx));
        } else if !dialog_open && self.dialog_open {
            self.apply_to_all = false;
            cx.defer_in(window, |this, window, cx| this.focus_active(window, cx));
        }
        self.dialog_open = dialog_open;

        // The places panel marks the place of the folder being shown.
        let location = self.tabs.active().browser().read(cx).location();
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_current(location, cx));

        let title = format!("{} \u{2014} Rustphin", Self::label(self.tabs.active(), cx));
        if title != self.title {
            window.set_window_title(&title);
            self.title = title;
        }
        let title_bar = crate::titlebar::title_bar(&self.title.clone(), theme, window, cx);
        let resize_edges = crate::titlebar::resize_edges(window);
        // Without the compositor's frame, a hairline marks the window's edge.
        let framed = crate::titlebar::client_decorated(window) && !window.is_maximized();

        let tab = self.tabs.active();
        let split = tab.panes.len() > 1;
        // Panes share equally the width the places panel leaves.
        let panel_width = if self.sidebar_visible {
            size::SIDEBAR
        } else {
            0.
        };
        let pane_width =
            (f32::from(window.viewport_size().width) - panel_width) / tab.panes.len() as f32;
        for pane in &tab.panes {
            pane.browser
                .update(cx, |browser, _| browser.set_width(pane_width));
        }
        let panes = tab.panes.iter().enumerate().map(|(index, pane)| {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .h_full()
                .when(index > 0, |pane| {
                    pane.border_l_1().border_color(theme.border)
                })
                // In a split, a line marks the pane that keyboard input goes to.
                .when(split, |pane| {
                    pane.child(
                        div()
                            .h(px(size::SELECTION_BAR))
                            .flex_none()
                            .when(index == tab.active, |marker| marker.bg(theme.accent)),
                    )
                })
                .child(div().flex_1().min_h_0().child(pane.browser.clone()))
        });

        let activity = self.activity(&hub, theme, cx);
        let question = hub.read(cx).questions.front().cloned();
        let dialog = if let Some(question) = question {
            Some(self.dialog(&question, theme, window, cx))
        } else if let Some(prompt) = self.launch.as_ref() {
            Some(self.launch_dialog(prompt, theme, window, cx))
        } else {
            self.error_details
                .as_ref()
                .map(|problems| self.error_details_dialog(problems, theme, window, cx))
        };

        div()
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &EditLocation, window, cx| {
                this.tabs
                    .active()
                    .browser()
                    .update(cx, |browser, cx| browser.edit_location(window, cx));
            }))
            .on_action(cx.listener(|this, _: &NewTab, window, cx| this.new_tab(window, cx)))
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| {
                this.close_tab(this.tabs.active_index(), window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| {
                this.tabs.cycle(1);
                this.focus_active(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PreviousTab, window, cx| {
                this.tabs.cycle(-1);
                this.focus_active(window, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &MoveTabLeft, _, cx| {
                this.tabs.shift_active(-1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &MoveTabRight, _, cx| {
                this.tabs.shift_active(1);
                cx.notify();
            }))
            .on_action(
                cx.listener(|this, _: &ToggleSplit, window, cx| this.toggle_split(window, cx)),
            )
            .on_action(cx.listener(|this, _: &OtherPane, window, cx| this.other_pane(window, cx)))
            .on_action(cx.listener(|this, _: &ReopenClosedTab, window, cx| {
                this.reopen_closed_tab(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusOtherView, window, cx| {
                let index = this.tabs.active_index();
                if this.tabs.active().panes.len() == 2 {
                    this.edit_tab(index, |tab| tab.active = 1 - tab.active.min(1));
                    this.focus_active(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &CopyToOtherView, _, cx| {
                this.transfer_to_other_view(Transfer::Copy, cx);
            }))
            .on_action(cx.listener(|this, _: &MoveToOtherView, _, cx| {
                this.transfer_to_other_view(Transfer::Move, cx);
            }))
            // Quitting is closing the window, with the same session save.
            .on_action(|_: &Quit, window, _| window.remove_window())
            .on_action(cx.listener(|this, _: &FocusPlaces, window, cx| {
                this.sidebar_visible = true;
                let sidebar = this.sidebar.clone();
                window.on_next_frame(move |window, cx| {
                    sidebar.update(cx, |sidebar, cx| sidebar.focus(window, cx));
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ActivateTab1, w, cx| this.activate_number(1, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab2, w, cx| this.activate_number(2, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab3, w, cx| this.activate_number(3, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab4, w, cx| this.activate_number(4, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab5, w, cx| this.activate_number(5, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab6, w, cx| this.activate_number(6, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab7, w, cx| this.activate_number(7, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab8, w, cx| this.activate_number(8, w, cx)))
            .on_action(cx.listener(|this, _: &ActivateTab9, w, cx| this.activate_number(9, w, cx)))
            .on_action(
                cx.listener(|this, _: &ActivateLastTab, w, cx| this.activate_number(0, w, cx)),
            )
            .on_action(|_: &Undo, _, cx| operations::hub(cx).update(cx, Hub::undo))
            .on_action(cx.listener(|this, _: &ToggleSidebar, window, cx| {
                this.sidebar_visible = !this.sidebar_visible;
                if !this.sidebar_visible {
                    this.focus_active(window, cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &AddPlace, _, cx| {
                let folder = this.tabs.active().browser().read(cx).location();
                if let Some(folder) = folder {
                    this.sidebar
                        .update(cx, |sidebar, cx| sidebar.add_place(folder, None, cx));
                }
            }))
            .on_action(
                cx.listener(|this, _: &OpenPalette, window, cx| this.open_palette(window, cx)),
            )
            .on_action(|_: &DismissNotice, _, cx| {
                operations::hub(cx).update(cx, Hub::dismiss_notice)
            })
            .on_action(|_: &StopOperations, _, cx| {
                operations::hub(cx).update(cx, |hub, _| {
                    let ids: Vec<_> = hub.running.iter().map(|running| running.id).collect();
                    for id in ids {
                        hub.cancel(id);
                    }
                });
            })
            .relative()
            .flex()
            .flex_col()
            .size_full()
            // The one window background, so gaps and dividers between panes sit on it.
            .bg(theme.window)
            .when(framed, |workspace| {
                workspace.border_1().border_color(theme.border_strong)
            })
            .children(title_bar)
            // A single tab needs no tab strip: fewer things on screen by default.
            .when(self.tabs.len() > 1, |workspace| {
                workspace.child(self.tab_strip(theme, cx))
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.sidebar_visible, |row| row.child(self.sidebar.clone()))
                    .children(panes),
            )
            // Window lines under the panes' status lines: activity and
            // results first, the blur status last.
            .children(activity)
            .children(self.blur_status(theme))
            .children(dialog)
            .when(self.rename.is_some(), |workspace| {
                let element = self
                    .rename
                    .as_ref()
                    .map(|dialog| dialog.render(theme, window, cx));
                workspace.children(element)
            })
            .when(self.properties.is_some(), |workspace| {
                let element = self
                    .properties
                    .as_ref()
                    .map(|dialog| dialog.render(&self.dialog_focus, &self.zone, theme, window, cx));
                workspace.children(element)
            })
            .children(self.palette.clone())
            .children(resize_edges)
    }
}

impl Workspace {
    /// What file operations are doing and what they last did, as lines under
    /// the panes. Nothing is shown while there is nothing to say.
    fn activity(
        &self,
        hub: &Entity<Hub>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let (running, notice, problems) = {
            let hub = hub.read(cx);
            let running: Vec<AnyElement> = hub
                .running
                .iter()
                .filter(|running| running.is_visible())
                .map(|running| Self::progress_line(running, theme).into_any_element())
                .collect();
            (running, hub.notice.clone(), hub.problems.clone())
        };
        let notice = notice.map(|notice| self.notice_line(&notice, problems, theme, cx));
        if running.is_empty() && notice.is_none() {
            return None;
        }
        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .bg(theme.chrome)
                .text_size(px(text::BODY))
                .children(running)
                .children(notice),
        )
    }

    /// An outlined text button for the lines under the panes, with its key
    /// in `data-sm` after the label.
    fn line_button(
        id: &'static str,
        label: &'static str,
        key: Option<&'static str>,
        theme: Theme,
    ) -> Button {
        Button::new(id)
            .outlined()
            // The library pads a custom-sized button by a fifth of the size:
            // 80 gives the 16-pixel padding.
            .with_size(px(space::LG * 5.))
            .h(px(size::BUTTON))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .text_size(px(text::BODY))
                    .child(label)
                    .when_some(key, |button, key| {
                        button.child(
                            div()
                                .font_family(text::FAMILY_DATA)
                                .text_size(px(text::SMALL))
                                .text_color(theme.text_muted)
                                .child(key),
                        )
                    }),
            )
    }

    /// A 28-pixel icon button with a tooltip.
    fn icon_button(id: &'static str, icon: IconName, label: &'static str) -> Button {
        Button::new(id)
            .ghost()
            .with_size(px(size::SMALL_BUTTON))
            .size(px(size::SMALL_BUTTON))
            .child(Icon::new(icon).size(px(size::ICON)))
            .tooltip(label)
            .accessibility_label(label)
    }

    fn progress_line(running: &Running, theme: Theme) -> impl IntoElement + use<> {
        let id = running.id;
        let fraction = running.fraction().unwrap_or(0.).clamp(0., 1.);
        div()
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::LINE))
            .pl(px(space::MD))
            .pr(px(space::SM))
            .border_t_1()
            .border_color(theme.border)
            .child(
                Icon::new(running.icon)
                    .size(px(size::ICON))
                    .flex_none()
                    .text_color(theme.text_muted),
            )
            .child(
                div()
                    .flex_shrink_1()
                    .min_w_0()
                    .truncate()
                    .child(running.label.clone()),
            )
            .child(
                // The bar says how far; the numbers beside it say the same in text.
                div()
                    .flex_1()
                    .min_w(px(size::PROGRESS_MIN))
                    .h(px(size::PROGRESS))
                    .rounded(px(size::RADIUS))
                    .bg(theme.track)
                    .child(
                        div()
                            .h_full()
                            .w(gpui_kit::relative(fraction))
                            .rounded(px(size::RADIUS))
                            .bg(theme.activity),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_DATA)
                    .text_size(px(text::SMALL))
                    .text_color(theme.text_muted)
                    .child(running.detail()),
            )
            .child(
                Button::new(("stop-operation", id.as_u64()))
                    .outlined()
                    .with_size(px(space::LG * 5.))
                    .h(px(size::BUTTON))
                    .child(div().text_size(px(text::BODY)).child("Stop"))
                    .tooltip("Stop. What is already done stays.")
                    .on_click(move |_, _, cx| hub(cx).update(cx, |hub, _| hub.cancel(id))),
            )
    }

    fn notice_line(
        &self,
        notice: &Notice,
        problems: Option<operations::Problems>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let (icon, color) = if notice.is_error {
            (IconName::TriangleAlert, theme.danger)
        } else if notice.offers_undo {
            (IconName::CircleCheck, theme.success)
        } else {
            (IconName::Info, theme.text_muted)
        };
        let live = if notice.is_error {
            gpui_kit::accesskit::Live::Assertive
        } else {
            gpui_kit::accesskit::Live::Polite
        };
        div()
            .id("activity-notice")
            .role(if notice.is_error {
                Role::Alert
            } else {
                Role::Status
            })
            .aria_label(notice.text.clone())
            .a11y_synthetic_children(move |builder| builder.parent_node().set_live(live))
            .flex()
            .items_center()
            .gap(px(space::MD))
            .min_h(px(size::LINE))
            .pl(px(space::MD))
            .pr(px(space::SM))
            .py(px(space::XS))
            .border_t_1()
            .border_color(theme.border)
            .when(notice.is_error, |line| line.bg(theme.danger_surface))
            .text_color(if notice.is_error {
                theme.danger
            } else {
                theme.text
            })
            .child(
                Icon::new(icon)
                    .size(px(size::ICON))
                    .flex_none()
                    .text_color(color),
            )
            .child(div().flex_1().min_w_0().child(notice.text.clone()))
            .when_some(problems, |line, problems| {
                // In the error colours, as the line it sits on.
                line.child(
                    Button::new("notice-details")
                        .destructive(&theme)
                        .outlined()
                        .with_size(px(space::LG * 5.))
                        .h(px(size::BUTTON))
                        .child(div().text_size(px(text::BODY)).child("Details\u{2026}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.error_details = Some(problems.clone());
                            cx.notify();
                        })),
                )
            })
            .when(notice.offers_undo, |line| {
                line.child(
                    Self::line_button("undo-notice", "Undo", Some("Ctrl+Z"), theme)
                        .on_click(|_, _, cx| hub(cx).update(cx, Hub::undo)),
                )
            })
            .child(
                Self::icon_button("dismiss-notice", IconName::Close, "Dismiss")
                    .on_click(|_, _, cx| operations::hub(cx).update(cx, Hub::dismiss_notice)),
            )
    }

    fn close_error_details(
        &mut self,
        _: &gpui_kit::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error_details = None;
        cx.notify();
    }

    /// What an operation left out, one entry per row, with a way to copy
    /// the list (docs/DESIGN.md, error details dialog).
    fn error_details_dialog(
        &self,
        problems: &operations::Problems,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let left_out = problems.entries.len();
        let summary = format!(
            "{} {} left out. Everything else is done.",
            format::count(left_out),
            format::plural(left_out, "entry was", "entries were")
        );
        let list_text = error_list_text(problems);
        let rows: Vec<AnyElement> = problems
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let name = entry
                    .path
                    .file_name()
                    .map(|name| display_name(name.as_encoded_bytes()).into_owned())
                    .unwrap_or_else(|| entry.path.display().to_string());
                let folder = entry
                    .path
                    .parent()
                    .map(|folder| folder.display().to_string())
                    .unwrap_or_default();
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::XS / 2.))
                    .px(px(space::MD))
                    .py(px(space::SM))
                    .when(index > 0, |row| row.border_t_1().border_color(theme.border))
                    .child(div().child(name))
                    .child(
                        div()
                            .font_family(text::FAMILY_DATA)
                            .text_size(px(text::SMALL))
                            .text_color(theme.text_muted)
                            .child(folder),
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .child(entry.reason.clone()),
                    )
                    .into_any_element()
            })
            .collect();
        let copy = list_text.clone();
        let panel = dialog::panel(
            div()
                .id("error-details")
                .role(Role::Dialog)
                .aria_label(problems.title.clone())
                .aria_description(summary.clone())
                .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                .key_context(DIALOG_CONTEXT)
                .track_focus(&self.dialog_focus)
                .on_key_down(cx.listener(Self::scroll_question))
                .on_action(cx.listener(|this, _: &DialogAccept, _, cx| {
                    this.error_details = None;
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &DialogCancel, _, cx| {
                    this.error_details = None;
                    cx.notify();
                }))
                .on_action(move |_: &DialogCopy, _, cx| {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(copy.clone()));
                }),
            size::DIALOG,
            theme,
            window,
        )
        .child(dialog::title_row(
            gpui_kit::assets::IconName::TriangleAlert,
            problems.title.clone(),
            "Close (Esc)",
            cx.listener(Self::close_error_details),
            theme,
        ))
        .child(dialog::body().child(self.question_body(
            summary,
            vec![
                div()
                    .flex()
                    .flex_col()
                    .rounded(px(size::RADIUS))
                    .border_1()
                    .border_color(theme.border)
                    .children(rows)
                    .into_any_element(),
            ],
            theme,
        )))
        .child(
            dialog::actions(theme)
                .child(
                    dialog::button("error-details-copy", "Copy list (C)")
                        .outlined()
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                list_text.clone(),
                            ));
                        }),
                )
                .child(
                    dialog::button("error-details-done", "Close (Esc)")
                        .primary()
                        .on_click(cx.listener(Self::close_error_details)),
                ),
        );
        dialog::scrim(panel, theme)
    }

    /// One side of a conflict in words: "1.2 MiB, modified 2026-10-12 10:17".
    fn describe(&self, info: &FileInfo) -> String {
        let modified = format::timestamp(
            info.modified_ns.div_euclid(1_000_000_000) as i64,
            &self.zone,
        );
        if info.kind == rustphin_core::EntryKind::Directory {
            format!("folder, modified {modified}")
        } else {
            format!("{}, modified {modified}", format::size(info.size))
        }
    }

    /// The dialog for the question at the front of the queue, over a dimmed window.
    fn dialog(
        &self,
        question: &Question,
        theme: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use gpui_kit::assets::IconName as I;
        let fact = |id: &'static str, label: &'static str, value: String| {
            dialog::fact(id, label, size::FACT_LABEL, value, theme)
        };
        // The names of a few entries, one per row, as the question is about them.
        let name_list = |paths: &[PathBuf]| {
            div()
                .flex()
                .flex_col()
                .rounded(px(size::RADIUS))
                .border_1()
                .border_color(theme.border)
                .children(paths.iter().enumerate().map(|(index, path)| {
                    div()
                        .flex()
                        .items_center()
                        .px(px(space::MD))
                        .min_h(px(size::CONTROL))
                        .when(index > 0, |row| row.border_t_1().border_color(theme.border))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .font_family(text::FAMILY_DATA)
                                .text_size(px(text::DATA))
                                .child(folder_label(path)),
                        )
                }))
                .into_any_element()
        };
        let verb = |id: rustphin_core::ops::OpId, hub: &Hub| {
            hub.running
                .iter()
                .find(|running| running.id == id)
                .map_or("change", |running| running.verb)
        };
        let description = match question {
            Question::Conflict { conflict, .. } => format!(
                "Existing: {}. Incoming: {}. Choose an available action. Keep both uses another name; Stop cancels.",
                self.describe(&conflict.target_info), self.describe(&conflict.source_info),
            ),
            Question::Failed { message, .. } => format!("{message}. Skip leaves this entry out and continues with the rest. Try again retries it; Stop cancels."),
            Question::Drop { .. } => "Choose move or copy, or Cancel to keep the files where they are.".to_owned(),
            Question::ConfirmDelete { .. } => "This does not use the trash. It cannot be undone. Cancel or Escape leaves it as it is.".to_owned(),
            Question::ConfirmEmpty { .. } => "Everything in the trash is deleted for good. It cannot be undone. Cancel or Escape leaves it as it is.".to_owned(),
        };
        let cancel = |label: &'static str| {
            dialog::button("dialog-cancel", label)
                .outlined()
                .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx)))
        };
        let (icon, title, sentence, body, buttons): (
            I,
            String,
            String,
            Vec<AnyElement>,
            Vec<Button>,
        ) = match question {
            Question::Conflict { conflict, .. } => {
                let folder = conflict.target_info.kind == rustphin_core::EntryKind::Directory;
                let name = folder_label(&conflict.target);
                let place = conflict
                    .target
                    .parent()
                    .map(folder_label)
                    .unwrap_or_default();
                let all = self.apply_to_all;
                (
                    I::Copy,
                    format!("\u{201c}{name}\u{201d} already exists"),
                    format!(
                        "{} with this name is already in \u{201c}{place}\u{201d}. Keep both gives the new one another name; Stop cancels the rest.",
                        if folder { "A folder" } else { "A file" }
                    ),
                    vec![
                        fact(
                            "fact-existing",
                            "Existing",
                            self.describe(&conflict.target_info),
                        )
                        .into_any_element(),
                        fact(
                            "fact-incoming",
                            "Incoming",
                            self.describe(&conflict.source_info),
                        )
                        .into_any_element(),
                        fact(
                            "fact-keep-both",
                            "Keep both",
                            folder_label(&conflict.keep_both),
                        )
                        .into_any_element(),
                        Checkbox::new("apply-to-all")
                            .text_size(px(text::BODY))
                            .label("Do the same for all other conflicts (A)")
                            .checked(all)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.apply_to_all = *checked;
                                cx.notify();
                            }))
                            .into_any_element(),
                    ],
                    vec![
                        cancel("Stop (Esc)"),
                        dialog::button("dialog-skip", "Skip (S)")
                            .outlined()
                            .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Skip, cx))),
                        dialog::button(
                            "dialog-replace",
                            if folder { "Merge (R)" } else { "Replace (R)" },
                        )
                        .outlined()
                        .disabled(!conflict.can_overwrite)
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Replace, cx))),
                        dialog::button("dialog-accept", "Keep both (Enter)")
                            .primary()
                            .on_click(
                                cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx)),
                            ),
                    ],
                )
            }
            Question::Failed { id, path, message } => (
                I::TriangleAlert,
                format!(
                    "Couldn't {} \u{201c}{}\u{201d}",
                    verb(*id, hub(cx).read(cx)),
                    folder_label(path)
                ),
                format!(
                    "{}. Skip leaves this entry out and continues with the rest.",
                    operations::upper_first(message)
                ),
                path.parent()
                    .map(|folder| {
                        fact("fact-from", "From", folder.display().to_string()).into_any_element()
                    })
                    .into_iter()
                    .collect(),
                vec![
                    cancel("Stop (Esc)"),
                    dialog::button("dialog-skip", "Skip (S)")
                        .outlined()
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Skip, cx))),
                    dialog::button("dialog-accept", "Try again (Enter)")
                        .primary()
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::Drop { sources, into } => (
                I::ArrowLeftRight,
                "Move or copy?".to_owned(),
                format!(
                    "Move or copy {} into \u{201c}{}\u{201d}?",
                    operations::what(sources),
                    folder_label(into)
                ),
                vec![
                    name_list(sources),
                    div()
                        .text_color(theme.text_muted)
                        .child(
                            "Hold Shift while dropping to move, Ctrl to copy, without being asked.",
                        )
                        .into_any_element(),
                ],
                vec![
                    cancel("Cancel (Esc)"),
                    dialog::button("dialog-copy", "Copy here (C)")
                        .outlined()
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Copy, cx))),
                    dialog::button("dialog-accept", "Move here (Enter)")
                        .primary()
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::ConfirmDelete { paths } => (
                I::CircleX,
                "Delete for good?".to_owned(),
                format!(
                    "Delete {} for good? This does not use the trash. It cannot be undone.",
                    operations::what(paths)
                ),
                vec![name_list(paths)],
                vec![
                    cancel("Cancel (Esc)"),
                    dialog::button("dialog-accept", "Delete for good (Enter)")
                        .destructive(&theme)
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::ConfirmEmpty { entries, .. } => (
                I::Trash,
                "Empty the trash?".to_owned(),
                format!(
                    "It holds {} {}. Everything in the trash is deleted for good. It cannot be undone.",
                    format::count(*entries),
                    format::plural(*entries, "item", "items")
                ),
                Vec::new(),
                vec![
                    cancel("Cancel (Esc)"),
                    dialog::button("dialog-accept", "Empty the trash (Enter)")
                        .destructive(&theme)
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
        };
        let panel = dialog::panel(
            div()
                .id("operation-question")
                .role(Role::Dialog)
                .aria_label(title.clone())
                .aria_description(description)
                .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                .key_context(DIALOG_CONTEXT)
                .track_focus(&self.dialog_focus)
                .on_key_down(cx.listener(Self::scroll_question))
                .on_action(cx.listener(|this, _: &DialogAccept, _, cx| {
                    this.answer(Answer::Accept, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogCancel, _, cx| {
                    this.answer(Answer::Cancel, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogSkip, _, cx| {
                    this.answer(Answer::Skip, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogKeepBoth, _, cx| {
                    this.answer(Answer::Accept, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogReplace, _, cx| {
                    this.answer(Answer::Replace, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogCopy, _, cx| {
                    this.answer(Answer::Copy, cx);
                }))
                .on_action(cx.listener(|this, _: &DialogToggleAll, _, cx| {
                    this.apply_to_all = !this.apply_to_all;
                    cx.notify();
                })),
            size::DIALOG,
            theme,
            window,
        )
        .child(dialog::title_row(
            icon,
            title,
            "Close (Esc)",
            cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx)),
            theme,
        ))
        .child(dialog::body().child(self.question_body(sentence, body, theme)))
        .child(dialog::actions(theme).children(buttons));
        dialog::scrim(panel, theme)
    }

    fn scroll_question(
        &mut self,
        event: &gpui_kit::KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return;
        }
        let mut offset = self.question_scroll.offset();
        if let Some(y) = crate::layout::scroll_offset(
            &event.keystroke.key,
            f32::from(offset.y),
            f32::from(self.question_scroll.max_offset().y),
            f32::from(self.question_scroll.bounds().size.height),
        ) {
            offset.y = px(y);
            self.question_scroll.set_offset(offset);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn question_body(
        &self,
        question: String,
        facts: Vec<AnyElement>,
        theme: Theme,
    ) -> impl IntoElement {
        div()
            .relative()
            .flex()
            .flex_col()
            .min_h_0()
            .flex_shrink_1()
            // The scrollbar lies in the body's right padding, so the content
            // keeps the full width.
            .mr(px(-space::LG))
            .child(
                div()
                    .id("question-content")
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.question_scroll)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(space::MD))
                            .pr(px(space::LG))
                            .child(div().flex_none().child(question))
                            .children(facts),
                    ),
            )
            .child(
                Scrollbar::vertical(&self.question_scroll)
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

    /// Applies a dialog button or its key to the question being shown.
    /// Answers that do not fit the question are ignored.
    fn answer(&mut self, answer: Answer, cx: &mut Context<Self>) {
        let all = self.apply_to_all;
        hub(cx).update(cx, |hub, cx| match (hub.questions.front(), answer) {
            (Some(Question::Conflict { conflict, .. }), answer) => {
                let choice = match answer {
                    Answer::Accept => ConflictChoice::KeepBoth,
                    Answer::Skip => ConflictChoice::Skip,
                    Answer::Cancel => ConflictChoice::Cancel,
                    Answer::Replace if conflict.can_overwrite => ConflictChoice::Overwrite,
                    Answer::Replace | Answer::Copy => return,
                };
                hub.resolve(choice, all, cx);
            }
            (Some(Question::Failed { .. }), Answer::Accept) => hub.recover(Recovery::Retry, cx),
            (Some(Question::Failed { .. }), Answer::Skip) => hub.recover(Recovery::Skip, cx),
            (Some(Question::Failed { .. }), Answer::Cancel) => hub.recover(Recovery::Cancel, cx),
            (Some(Question::ConfirmDelete { .. }), Answer::Accept) => hub.confirm_delete(true, cx),
            (Some(Question::ConfirmDelete { .. }), Answer::Cancel) => hub.confirm_delete(false, cx),
            (Some(Question::ConfirmEmpty { .. }), Answer::Accept) => hub.confirm_empty(true, cx),
            (Some(Question::ConfirmEmpty { .. }), Answer::Cancel) => hub.confirm_empty(false, cx),
            (Some(Question::Drop { .. }), Answer::Accept) => {
                hub.answer_drop(Some(operations::DropAction::Move), cx);
            }
            (Some(Question::Drop { .. }), Answer::Copy) => {
                hub.answer_drop(Some(operations::DropAction::Copy), cx);
            }
            (Some(Question::Drop { .. }), Answer::Cancel) => hub.answer_drop(None, cx),
            _ => {}
        });
        self.apply_to_all = false;
        self.question_scroll.set_offset(gpui_kit::Point::default());
        cx.notify();
    }
}

/// What a dialog button means, before it is matched to the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The default button: keep both, try again, or delete.
    Accept,
    Skip,
    Replace,
    /// Copy the dropped entries.
    Copy,
    /// Stop the operation, or keep the files.
    Cancel,
}

/// The name shown for a folder: its last component, or `/` for the root.
/// A program or shortcut the user has been asked about.
struct LaunchPrompt {
    path: PathBuf,
    /// The name in the question.
    name: String,
    /// An executable file, rather than an application shortcut.
    program: bool,
}

/// The background classification of one activation.
enum LaunchOutcome {
    /// A document was handed to the default application, or classification failed.
    Opened(Result<(), String>),
    /// Ask before running.
    Ask {
        path: PathBuf,
        name: String,
        program: bool,
    },
}

fn classify(path: &Path) -> LaunchOutcome {
    match rustphin_platform::apps::launch_kind(path) {
        rustphin_platform::apps::LaunchKind::Document => LaunchOutcome::Opened(
            rustphin_platform::apps::open_with_default(path).map_err(|error| error.to_string()),
        ),
        rustphin_platform::apps::LaunchKind::Program => LaunchOutcome::Ask {
            path: path.to_owned(),
            name: file_label(path),
            program: true,
        },
        rustphin_platform::apps::LaunchKind::Desktop { name } => LaunchOutcome::Ask {
            path: path.to_owned(),
            name,
            program: false,
        },
        rustphin_platform::apps::LaunchKind::Unreadable(message) => {
            LaunchOutcome::Opened(Err(message))
        }
    }
}

/// The error details as plain text for the clipboard: the title, then one
/// line per entry with its full path and the reason.
fn error_list_text(problems: &operations::Problems) -> String {
    let mut text = format!("{}\n", problems.title);
    for entry in &problems.entries {
        text.push_str(&format!("{}: {}\n", entry.path.display(), entry.reason));
    }
    text
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| display_name(name.as_bytes()).into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn folder_label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
        None => "/".to_owned(),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_error_list_names_every_entry_left_out_with_its_reason() {
        use rustphin_core::ops::Skipped;
        let problems = operations::Problems {
            title: "Copying 2 items to \u{201c}Backup\u{201d}".into(),
            entries: vec![
                Skipped {
                    path: "/home/u/a.jpg".into(),
                    reason: "permission denied".into(),
                },
                Skipped {
                    path: "/home/u/b.pdf".into(),
                    reason: "it changed while it was being copied".into(),
                },
            ],
        };
        assert_eq!(
            error_list_text(&problems),
            "Copying 2 items to \u{201c}Backup\u{201d}\n\
             /home/u/a.jpg: permission denied\n\
             /home/u/b.pdf: it changed while it was being copied\n"
        );
    }

    use super::*;

    #[test]
    fn the_other_view_receives_the_selection_or_a_reason() {
        let a = vec![PathBuf::from("/left/a"), PathBuf::from("/left/b")];
        let left = Some(Path::new("/left"));
        let right = Some(Path::new("/right"));
        assert!(matches!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, right),
            Ok(Request::Copy { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(matches!(
            transfer_to_other_view(Transfer::Move, a.clone(), left, right),
            Ok(Request::Move { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, None)
                .is_err_and(|reason| reason.contains("F3"))
        );
        assert!(
            transfer_to_other_view(Transfer::Copy, Vec::new(), left, right)
                .is_err_and(|reason| reason.contains("Select"))
        );
        assert!(
            transfer_to_other_view(Transfer::Move, a, left, left)
                .is_err_and(|reason| reason.contains("same folder"))
        );
    }

    #[test]
    fn closed_tabs_are_kept_newest_first_up_to_the_limit() {
        let tab = |path: &str| SessionTab {
            panes: vec![Location::Folder(PathBuf::from(path))],
            active_pane: 0,
        };
        let mut closed = Vec::new();
        for index in 0..CLOSED_TAB_LIMIT + 2 {
            remember_closed_tab(&mut closed, tab(&format!("/{index}")));
        }
        remember_closed_tab(&mut closed, SessionTab::default());
        assert_eq!(closed.len(), CLOSED_TAB_LIMIT);
        assert_eq!(closed[0], tab(&format!("/{}", CLOSED_TAB_LIMIT + 1)));
        assert_eq!(closed[CLOSED_TAB_LIMIT - 1], tab("/2"));
    }

    #[test]
    fn alt_numbers_pick_existing_tabs_and_alt_0_the_last() {
        assert_eq!(tab_for_number(1, 3), Some(0));
        assert_eq!(tab_for_number(3, 3), Some(2));
        assert_eq!(tab_for_number(4, 3), None);
        assert_eq!(tab_for_number(0, 3), Some(2));
        assert_eq!(tab_for_number(0, 0), None);
        assert_eq!(tab_for_number(1, 0), None);
    }

    #[test]
    fn folders_are_labelled_by_their_last_component() {
        assert_eq!(
            folder_label(Path::new("~/Documents")),
            "Documents"
        );
        assert_eq!(folder_label(Path::new("/")), "/");
        assert_eq!(folder_label(Path::new("/tmp/a b")), "a b");
    }
}
