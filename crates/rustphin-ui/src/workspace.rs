//! The window's content: tabs, each showing one folder or two side by side.

use std::path::{Path, PathBuf};

use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, KeyBinding,
    MouseButton, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, actions,
    component::{
        IconName, Sizable as _,
        button::{Button, ButtonVariants as _},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use rustphin_core::display_name;

use crate::{
    browser::{Browser, BrowserEvent},
    session::{Session, SessionTab},
    settings::Settings,
    tabs::Tabs,
    theme::{Theme, size, space, text},
};

const KEY_CONTEXT: &str = "Workspace";
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
    ]
);

/// Registers the tab and split-view shortcuts. They match Dolphin's defaults.
pub fn bind_keys(cx: &mut App) {
    let context = Some(KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("ctrl-t", NewTab, context),
        KeyBinding::new("ctrl-w", CloseTab, context),
        KeyBinding::new("ctrl-tab", NextTab, context),
        KeyBinding::new("ctrl-pagedown", NextTab, context),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, context),
        KeyBinding::new("ctrl-pageup", PreviousTab, context),
        KeyBinding::new("ctrl-shift-pageup", MoveTabLeft, context),
        KeyBinding::new("ctrl-shift-pagedown", MoveTabRight, context),
        KeyBinding::new("f3", ToggleSplit, context),
        // Tab moves between the two panes, except while typing in a field.
        KeyBinding::new("tab", OtherPane, Some("Workspace && !Input")),
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
        let workspace = Self {
            tabs,
            settings_path,
            session_path,
            saved: Session::default(),
            title: String::new(),
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
        if self.tabs.active().panes.len() == 2 {
            self.edit_tab(index, |tab| tab.active = 1 - tab.active);
            self.focus_active(window, cx);
            cx.notify();
        }
    }

    /// Applies `edit` to the tab at `index`, if there is one.
    fn edit_tab(&mut self, index: usize, edit: impl FnOnce(&mut Tab)) {
        if let Some(tab) = self.tabs.get_mut(index) {
            edit(tab);
        }
    }

    /// The label of a tab: the name of the folder its active pane shows.
    fn label(tab: &Tab, cx: &App) -> SharedString {
        match tab.browser().read(cx).location() {
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

        let title = format!("{} \u{2014} Rustphin", Self::label(self.tabs.active(), cx));
        if title != self.title {
            window.set_window_title(&title);
            self.title = title;
        }

        let tab = self.tabs.active();
        let split = tab.panes.len() > 1;
        // Panes share the window's width equally.
        let pane_width = f32::from(window.viewport_size().width) / tab.panes.len() as f32;
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
            .flex()
            .flex_col()
            .size_full()
            // The one window background, so gaps and dividers between panes sit on it.
            .bg(theme.window)
            // A single tab needs no tab strip: fewer things on screen by default.
            .when(self.tabs.len() > 1, |workspace| {
                workspace.child(self.tab_strip(theme, cx))
            })
            .child(div().flex().flex_1().min_h_0().children(panes))
    }
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
