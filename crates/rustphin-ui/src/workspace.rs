//! The window's content: tabs, each showing one folder or two side by side.

use std::path::{Path, PathBuf};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, KeyBinding, MouseButton, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, actions,
    component::{
        Disableable as _, Icon, IconName, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use jiff::tz::TimeZone;
use rustphin_core::{
    FileInfo, display_name,
    ops::{ConflictChoice, Recovery},
};

use crate::{
    browser::{Browser, BrowserEvent, DraggedEntries},
    commands::{self, Scope},
    format,
    history::history,
    operations::{self, Hub, Notice, Question, Running, hub},
    palette::{Candidate, Palette, PaletteEvent, Target},
    session::{Session, SessionTab},
    settings::Settings,
    sidebar::{Sidebar, SidebarEvent},
    tabs::Tabs,
    theme::{Theme, size, space, text},
};

const KEY_CONTEXT: &str = "Workspace";
/// A dialog that is waiting for an answer.
const DIALOG_CONTEXT: &str = "Dialog";
/// Widest a tab's label may be before it is cut short.
const TAB_MAX_WIDTH: f32 = 200.;

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
        ToggleSidebar,
        AddPlace,
        OpenPalette,
        StopOperations,
        DismissNotice,
    ]
);

/// Registers the tab and split-view shortcuts. They match Dolphin's defaults.
pub fn bind_keys(cx: &mut App) {
    let context = Some("Workspace && !Dialog && !Palette");
    cx.bind_keys([
        KeyBinding::new(
            "ctrl-shift-p",
            OpenPalette,
            Some("Workspace && !Dialog && !Palette"),
        ),
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
        KeyBinding::new(
            "ctrl-d",
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
        KeyBinding::new("s", DialogSkip, Some(DIALOG_CONTEXT)),
        KeyBinding::new("k", DialogKeepBoth, Some(DIALOG_CONTEXT)),
        KeyBinding::new("r", DialogReplace, Some(DIALOG_CONTEXT)),
        KeyBinding::new("a", DialogToggleAll, Some(DIALOG_CONTEXT)),
        KeyBinding::new("c", DialogCopy, Some(DIALOG_CONTEXT)),
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
    tabs: Tabs<Tab>,
    settings_path: Option<PathBuf>,
    /// Where the open tabs are stored; `None` when they must not be written.
    session_path: Option<PathBuf>,
    /// The session as last written, to skip writes that change nothing.
    saved: Session,
    title: String,
    /// Holds the keyboard focus while a dialog is open.
    dialog_focus: FocusHandle,
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
        // Never overwrite a file a later version of Rustphin wrote.
        let session_path = session_path.filter(|_| !session.is_from_newer_version());
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
                    .map(|path| {
                        Self::pane(
                            path.clone(),
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
            tabs,
            settings_path,
            session_path,
            saved: Session::default(),
            title: String::new(),
            dialog_focus: cx.focus_handle(),
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
            _history: cx.observe(&history(cx), |this, _, cx| this.refresh_palette(cx)),
            _places: cx.observe(&sidebar, |this, _, cx| this.refresh_palette(cx)),
        };
        workspace.focus_active(window, cx);
        workspace
    }

    /// The open tabs and panes as a session.
    fn session(&self, cx: &App) -> Session {
        let tabs = self
            .tabs
            .iter()
            .map(|tab| SessionTab {
                panes: tab
                    .panes
                    .iter()
                    .filter_map(|pane| pane.browser.read(cx).location())
                    .collect(),
                active_pane: tab.active,
            })
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
        self.saved = session.clone();
        cx.background_executor()
            .spawn(async move {
                let text = session.to_toml();
                if let Err(error) = rustphin_platform::config::write_atomic(&path, text.as_bytes())
                {
                    tracing::warn!(%error, path = %path.display(), "could not save the session");
                }
            })
            .detach();
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
            BrowserEvent::Visited(path) => history(cx).read(cx).visit(path.clone()),
        }
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
        for path in history(cx).read(cx).locations.paths() {
            candidates.push(Candidate::new(
                format!("Recent: {}", folder_label(path)),
                display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
                String::new(),
                Target::Folder(path.clone()),
            ));
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
        if self.palette.is_some() || !hub(cx).read(cx).questions.is_empty() {
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
        self.tabs.close(index);
        self.focus_active(window, cx);
        cx.notify();
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.activate(index);
        self.focus_active(window, cx);
        cx.notify();
    }

    fn focus_active(&self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn tab_strip(&self, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tabs.active_index();
        let tabs: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let label = Self::label(tab, cx);
                let is_active = index == active;
                div()
                    .id(("tab", index))
                    .group("tab")
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(space::XS))
                    .h_full()
                    .max_w(px(TAB_MAX_WIDTH))
                    .pl(px(space::MD))
                    .pr(px(space::XS))
                    .cursor_pointer()
                    // The active tab is brighter and underlined, never colour alone.
                    .border_b_2()
                    .border_color(if is_active {
                        theme.accent
                    } else {
                        gpui_kit::transparent_black().into()
                    })
                    .text_color(if is_active {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    .when(!is_active, |tab| {
                        tab.hover(|tab| tab.bg(theme.hover).text_color(theme.text))
                    })
                    .child(div().min_w_0().truncate().child(label.clone()))
                    .child(
                        Button::new(("close-tab", index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip("Close tab (Ctrl+W)")
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
                .px(px(space::XS))
                .bg(theme.chrome)
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(text::SMALL))
                .children(tabs)
                .child(
                    div().px(px(space::XS)).child(
                        Button::new("new-tab")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip("New tab (Ctrl+T)")
                            .on_click(cx.listener(|this, _, window, cx| this.new_tab(window, cx))),
                    ),
                ),
        )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let dialog_open = !hub.read(cx).questions.is_empty();
        if dialog_open && self.palette.is_some() {
            self.palette = None;
            self.palette_subscription = None;
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

        let activity = self.activity(hub.read(cx), theme);
        let question = hub.read(cx).questions.front().cloned();
        let dialog = question.map(|question| self.dialog(&question, theme, cx));

        div()
            .key_context(KEY_CONTEXT)
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
            .children(activity)
            .children(dialog)
            .children(self.palette.clone())
    }
}

impl Workspace {
    /// What file operations are doing and what they last did, as lines under
    /// the panes. Nothing is shown while there is nothing to say.
    fn activity(&self, hub: &Hub, theme: Theme) -> Option<impl IntoElement + use<>> {
        let running: Vec<AnyElement> = hub
            .running
            .iter()
            .filter(|running| running.is_visible())
            .map(|running| Self::progress_line(running, theme).into_any_element())
            .collect();
        let notice = hub
            .notice
            .as_ref()
            .map(|notice| Self::notice_line(notice, theme));
        if running.is_empty() && notice.is_none() {
            return None;
        }
        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .bg(theme.chrome)
                .text_size(px(text::SMALL))
                .children(running)
                .children(notice),
        )
    }

    fn progress_line(running: &Running, theme: Theme) -> impl IntoElement + use<> {
        let id = running.id;
        let fraction = running.fraction().unwrap_or(0.).clamp(0., 1.);
        div()
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(size::STATUS))
            .px(px(space::LG))
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(running.label.clone()),
            )
            .child(
                // The bar says how far; the numbers beside it say the same in text.
                div()
                    .flex_none()
                    .w(px(size::PROGRESS_WIDTH))
                    .h(px(size::PROGRESS))
                    .rounded(px(size::RADIUS))
                    .bg(theme.hover)
                    .child(
                        div()
                            .h_full()
                            .w(px(size::PROGRESS_WIDTH * fraction))
                            .rounded(px(size::RADIUS))
                            .bg(theme.activity),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_DATA)
                    .text_color(theme.text_muted)
                    .child(running.detail()),
            )
            .child(
                Button::new(("stop-operation", id.as_u64()))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip("Stop. What is already done stays.")
                    .on_click(move |_, _, cx| hub(cx).update(cx, |hub, _| hub.cancel(id))),
            )
    }

    fn notice_line(notice: &Notice, theme: Theme) -> impl IntoElement + use<> {
        let (icon, color) = if notice.is_error {
            (IconName::TriangleAlert, theme.danger)
        } else {
            (IconName::Info, theme.text_muted)
        };
        div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .min_h(px(size::STATUS))
            .px(px(space::LG))
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
            .when(notice.offers_undo, |line| {
                line.child(
                    Button::new("undo-notice")
                        .ghost()
                        .xsmall()
                        .label("Undo (Ctrl+Z)")
                        .on_click(|_, _, cx| hub(cx).update(cx, Hub::undo)),
                )
            })
            .child(
                Button::new("dismiss-notice")
                    .ghost()
                    .xsmall()
                    .label("Dismiss")
                    .on_click(|_, _, cx| operations::hub(cx).update(cx, Hub::dismiss_notice)),
            )
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
    fn dialog(&self, question: &Question, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let fact = |label: &'static str, value: String| {
            div()
                .flex()
                .gap(px(space::SM))
                .child(
                    div()
                        .w(px(72.))
                        .flex_none()
                        .text_color(theme.text_muted)
                        .child(label),
                )
                .child(
                    div()
                        .font_family(text::FAMILY_DATA)
                        .text_size(px(text::DATA))
                        .child(value),
                )
        };
        let (title, body, buttons): (String, Vec<AnyElement>, Vec<Button>) = match question {
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
                    format!("\u{201c}{name}\u{201d} already exists in \u{201c}{place}\u{201d}"),
                    vec![
                        fact("Existing", self.describe(&conflict.target_info)).into_any_element(),
                        fact("Incoming", self.describe(&conflict.source_info)).into_any_element(),
                        Checkbox::new("apply-to-all")
                            .label("Do the same for all other conflicts (A)")
                            .checked(all)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.apply_to_all = *checked;
                                cx.notify();
                            }))
                            .into_any_element(),
                    ],
                    vec![
                        Button::new("dialog-cancel")
                            .ghost()
                            .label("Stop (Esc)")
                            .on_click(
                                cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx)),
                            ),
                        Button::new("dialog-skip")
                            .outline()
                            .label("Skip (S)")
                            .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Skip, cx))),
                        Button::new("dialog-replace")
                            .outline()
                            .label(if folder { "Merge (R)" } else { "Replace (R)" })
                            .disabled(!conflict.can_overwrite)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.answer(Answer::Replace, cx)),
                            ),
                        Button::new("dialog-accept")
                            .primary()
                            .label("Keep both (Enter)")
                            .on_click(
                                cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx)),
                            ),
                    ],
                )
            }
            Question::Failed { path, message, .. } => (
                format!(
                    "\u{201c}{}\u{201d} could not be processed",
                    folder_label(path)
                ),
                vec![
                    div().child(format!("{message}.")).into_any_element(),
                    div()
                        .text_color(theme.text_muted)
                        .child("Skip leaves this entry out and continues with the rest.")
                        .into_any_element(),
                ],
                vec![
                    Button::new("dialog-cancel")
                        .ghost()
                        .label("Stop (Esc)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx))),
                    Button::new("dialog-skip")
                        .outline()
                        .label("Skip (S)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Skip, cx))),
                    Button::new("dialog-accept")
                        .primary()
                        .label("Try again (Enter)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::Drop { sources, into } => (
                format!(
                    "Move or copy {} into \u{201c}{}\u{201d}?",
                    operations::what(sources),
                    folder_label(into)
                ),
                vec![
                    div()
                        .text_color(theme.text_muted)
                        .child(
                            "Hold Shift while dropping to move, Ctrl to copy, without being asked.",
                        )
                        .into_any_element(),
                ],
                vec![
                    Button::new("dialog-cancel")
                        .ghost()
                        .label("Cancel (Esc)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx))),
                    Button::new("dialog-copy")
                        .outline()
                        .label("Copy here (C)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Copy, cx))),
                    Button::new("dialog-accept")
                        .primary()
                        .label("Move here (Enter)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::ConfirmDelete { paths } => (
                format!("Delete {} for good?", operations::what(paths)),
                vec![
                    div()
                        .child("This does not use the trash. It cannot be undone.")
                        .into_any_element(),
                ],
                vec![
                    Button::new("dialog-cancel")
                        .outline()
                        .label("Keep (Esc)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx))),
                    Button::new("dialog-accept")
                        .danger()
                        .label("Delete for good (Enter)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
            Question::ConfirmEmpty { entries, .. } => (
                format!(
                    "Empty the trash? It holds {} {}.",
                    format::count(*entries),
                    format::plural(*entries, "item", "items")
                ),
                vec![
                    div()
                        .child("Everything in it is deleted for good. It cannot be undone.")
                        .into_any_element(),
                ],
                vec![
                    Button::new("dialog-cancel")
                        .outline()
                        .label("Keep (Esc)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Cancel, cx))),
                    Button::new("dialog-accept")
                        .danger()
                        .label("Empty the trash (Enter)")
                        .on_click(cx.listener(|this, _, _, cx| this.answer(Answer::Accept, cx))),
                ],
            ),
        };
        div()
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.scrim)
            .child(
                div()
                    .key_context(DIALOG_CONTEXT)
                    .track_focus(&self.dialog_focus)
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
                    }))
                    .flex()
                    .flex_col()
                    .gap(px(space::MD))
                    .w(px(size::DIALOG))
                    .p(px(space::LG))
                    .rounded(px(size::RADIUS))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border_accent)
                    .shadow(vec![theme.floating_shadow()])
                    .text_color(theme.text)
                    .font_family(text::FAMILY_BODY)
                    .text_size(px(text::BODY))
                    .line_height(px(text::LINE))
                    .child(
                        div()
                            .font_family(text::FAMILY_HEADING)
                            .text_size(px(text::HEADING))
                            .child(title),
                    )
                    .children(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(space::SM))
                            .pt(px(space::SM))
                            .children(buttons.into_iter().map(|button| button.small())),
                    ),
            )
            .into_any_element()
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
fn folder_label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
        None => "/".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
