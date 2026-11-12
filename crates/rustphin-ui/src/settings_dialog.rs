//! The Settings dialog (W-106, D-029, mockup 36).
//!
//! A dialog over the window: the list of pages on the left (a row of tabs
//! in a narrow window), the chosen page on the right. Every change applies
//! at once through the shared settings (`preferences`); there is no OK or
//! Apply. The pages arrive one work item at a time (W-106b–g).

use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Image,
    ImageFormat, InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render,
    Role, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    Window, actions,
    assets::IconName,
    component::{Disableable as _, Icon, button::ButtonVariants as _},
    component::{
        input::{InputEvent, InputState},
        slider::{SliderEvent, SliderState},
    },
    div, img,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    controls::Outlined as _,
    dialog,
    preferences::preferences,
    settings::Settings,
    settings_controls::Selects,
    settings_general::{DesktopFacts, MakeDefault},
    theme::{Theme, size, space, text},
};

/// The dialog while it has the keyboard.
const CONTEXT: &str = "SettingsDialog";

actions!(
    settings_dialog,
    [
        /// Close the dialog.
        CloseSettings,
        /// The page above in the list.
        PreviousPage,
        /// The page below in the list.
        NextPage,
        /// Scroll the page shown up by most of its height.
        ScrollUp,
        /// Scroll the page shown down by most of its height.
        ScrollDown,
    ]
);

/// The dialog's keys.
pub fn bind_keys(cx: &mut App) {
    let context = Some(CONTEXT);
    let pages = Some("SettingsDialog && !Input");
    cx.bind_keys([
        KeyBinding::new("escape", CloseSettings, context),
        KeyBinding::new("up", PreviousPage, pages),
        KeyBinding::new("down", NextPage, pages),
        KeyBinding::new("pageup", ScrollUp, pages),
        KeyBinding::new("pagedown", ScrollDown, pages),
    ]);
}

/// The logo, drawn at 64 pixels on the About page.
static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("../../../assets/rustphin.svg").to_vec(),
    ))
});

/// The licence's full text, shown on the About page.
const LICENCE: &str = include_str!("../../../LICENSE");

/// One page of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// How Rustphin starts and works with the desktop.
    General,
    /// How the window looks.
    Appearance,
    /// How folders are shown when they open.
    Views,
    /// Which buttons the toolbar shows.
    Toolbar,
    /// What the places panel shows.
    Places,
    /// Which actions ask before they act.
    Confirmations,
    /// The keys Rustphin understands.
    Shortcuts,
    /// Version, licence, credits, privacy and files.
    About,
}

impl Page {
    /// The pages in the list, top to bottom.
    pub const ALL: &'static [Page] = &[
        Page::General,
        Page::Appearance,
        Page::Views,
        Page::Toolbar,
        Page::Places,
        Page::Confirmations,
        Page::Shortcuts,
        Page::About,
    ];

    /// Its name in the list and as its heading.
    pub fn name(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Appearance => "Appearance",
            Page::Views => "Views",
            Page::Toolbar => "Toolbar",
            Page::Places => "Places",
            Page::Confirmations => "Confirmations",
            Page::Shortcuts => "Shortcuts",
            Page::About => "About",
        }
    }

    /// Its icon in the list.
    pub fn icon(self) -> IconName {
        match self {
            Page::General => IconName::Settings,
            Page::Appearance => IconName::Palette,
            Page::Views => IconName::LayoutList,
            Page::Toolbar => IconName::SlidersHorizontal,
            Page::Places => IconName::Bookmark,
            Page::Confirmations => IconName::CircleCheck,
            Page::Shortcuts => IconName::Keyboard,
            Page::About => IconName::Info,
        }
    }

    /// The sentence under its heading.
    pub fn description(self) -> &'static str {
        match self {
            Page::General => "How Rustphin starts and works with the rest of the desktop.",
            Page::Views => "How folders are shown when they open.",
            Page::Toolbar => "Which buttons the toolbar shows, and in which order.",
            Page::Places => "What the places panel shows.",
            Page::Confirmations => "Which actions ask before they act.",
            Page::Shortcuts => "The keys Rustphin understands. Changing them comes later.",
            Page::Appearance => {
                "How the window looks. The colours are Rustphin\u{2019}s dark design."
            }
            Page::About => "Rustphin and the work it builds on.",
        }
    }

    /// Whether it holds settings, and so a "Reset to defaults".
    pub fn has_settings(self) -> bool {
        match self {
            Page::General
            | Page::Appearance
            | Page::Views
            | Page::Toolbar
            | Page::Places
            | Page::Confirmations => true,
            Page::Shortcuts | Page::About => false,
        }
    }

    /// The page `delta` steps further down the list, stopping at its ends.
    pub fn step(self, delta: isize) -> Page {
        let at = Page::ALL.iter().position(|page| *page == self).unwrap_or(0);
        let to = at
            .saturating_add_signed(delta)
            .min(Page::ALL.len().saturating_sub(1));
        Page::ALL.get(to).copied().unwrap_or(self)
    }
}

/// What the dialog asks the window to do.
pub enum SettingsEvent {
    /// Close the dialog.
    Close,
    /// Close it and show this folder in a new tab.
    OpenFolder(PathBuf),
}

/// The dialog's state.
pub struct SettingsDialog {
    page: Page,
    focus: FocusHandle,
    page_scroll: ScrollHandle,
    /// The About page shows the licence's full text.
    licence: bool,
    /// The drop-down that is open.
    pub(crate) selects: Selects,
    /// The folder the window showed when Settings opened.
    pub(crate) shown: Option<PathBuf>,
    /// KDE's terminal and the default file manager, once read.
    pub(crate) desktop: Option<DesktopFacts>,
    /// "Make Rustphin the default", while it runs or after it failed.
    pub(crate) make_default: MakeDefault,
    /// The terminal command chosen here.
    pub(crate) terminal_input: Entity<InputState>,
    /// The terminal field is open though nothing is chosen yet.
    pub(crate) editing_terminal: bool,
    /// The default view's size.
    pub(crate) zoom_slider: Entity<SliderState>,
    /// Hidden files' opacity, in percent.
    pub(crate) hidden_slider: Entity<SliderState>,
    /// The places panel, whose groups the Places page changes.
    pub(crate) sidebar: Entity<crate::sidebar::Sidebar>,
    /// The name field for a group being renamed.
    pub(crate) group_input: Entity<InputState>,
    /// The group being renamed.
    pub(crate) renaming_group: Option<usize>,
    /// The places panel's width.
    pub(crate) width_slider: Entity<SliderState>,
    /// The search over the available toolbar actions.
    pub(crate) toolbar_search: Entity<InputState>,
    /// The row chosen in "Available actions".
    pub(crate) available_cursor: Option<usize>,
    /// The row chosen in "On the toolbar".
    pub(crate) toolbar_cursor: Option<usize>,
    /// The search over the shortcuts.
    pub(crate) shortcut_search: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for SettingsDialog {}

impl SettingsDialog {
    /// The dialog on its first page; `shown` is the folder the window
    /// shows, offered as the folder to start in.
    pub fn new(
        shown: Option<PathBuf>,
        sidebar: Entity<crate::sidebar::Sidebar>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let store = preferences(cx);
        let chosen = store
            .read(cx)
            .settings()
            .terminal()
            .unwrap_or_default()
            .to_owned();
        let terminal_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("konsole --separate");
            input.set_value(chosen, window, cx);
            input
        });
        let selects = Selects::new(cx);
        let zoom = store.read(cx).settings().zoom();
        let zoom_slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max((crate::layout::ZOOM_LEVELS - 1) as f32)
                .step(1.)
                .default_value(zoom as f32)
        });
        let hidden = store.read(cx).settings().hidden_opacity();
        let hidden_slider = cx.new(|_| {
            SliderState::new()
                .min(crate::theme::HIDDEN_OPACITY_MIN_PERCENT as f32)
                .max(100.)
                .step(5.)
                .default_value(hidden as f32)
        });
        let width = store.read(cx).settings().places().width;
        let width_slider = cx.new(|_| {
            // The maximum first: the library checks the minimum against
            // the current maximum, 100 by default, and panics below it.
            SliderState::new()
                .max(size::SIDEBAR_MAX)
                .min(size::SIDEBAR_MIN)
                .step(4.)
                .default_value(width as f32)
        });
        let group_input = cx.new(|cx| InputState::new(window, cx));
        let shortcut_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search shortcuts\u{2026}"));
        let toolbar_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search actions\u{2026}"));
        let subscriptions = vec![
            cx.subscribe(&width_slider, |_, _, event: &SliderEvent, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                let width = value.start().round().max(0.) as u32;
                if preferences(cx).read(cx).settings().places().width != width {
                    crate::settings_places::choose_width(cx, width);
                }
            }),
            cx.subscribe(&shortcut_search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&toolbar_search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.available_cursor = None;
                    cx.notify();
                }
            }),
            cx.subscribe(&group_input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.finish_group_rename(cx);
                }
            }),
            // Groups changed in the panel show here at once.
            cx.observe(&sidebar, |_, _, cx| cx.notify()),
            cx.subscribe(&hidden_slider, |_, _, event: &SliderEvent, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                let percent = value.start().round().max(0.) as u32;
                if preferences(cx).read(cx).settings().hidden_opacity() != percent {
                    crate::settings_appearance::choose_hidden(cx, percent);
                }
            }),
            // A drop-down's list closes when the focus leaves it.
            cx.on_focus_out(&selects.focus, window, |this, _, _, cx| {
                if this.selects.open.take().is_some() {
                    cx.notify();
                }
            }),
            // Every change shows at once, whichever window made it.
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe(&zoom_slider, |_, _, event: &SliderEvent, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                let zoom =
                    (value.start().round().max(0.) as usize).min(crate::layout::ZOOM_LEVELS - 1);
                if preferences(cx).read(cx).settings().zoom() != zoom {
                    preferences(cx).update(cx, |store, cx| {
                        store.update_default_view(
                            |settings| {
                                let (mode, options) = (settings.mode(), settings.options());
                                settings.set_view(mode, zoom, options);
                            },
                            cx,
                        );
                    });
                }
            }),
            cx.subscribe(&terminal_input, |this, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) && this.editing_terminal_allowed() {
                    let command = input.read(cx).value().to_string();
                    preferences(cx).update(cx, |store, cx| {
                        store.update(|settings| settings.set_terminal(&command), cx);
                    });
                }
            }),
        ];
        cx.spawn(async move |this, cx| {
            let facts = cx.background_spawn(async { DesktopFacts::read() }).await;
            this.update(cx, |this, cx| {
                this.desktop = Some(facts);
                cx.notify();
            })
            .ok();
        })
        .detach();
        Self {
            page: Page::ALL[0],
            focus: cx.focus_handle(),
            page_scroll: ScrollHandle::new(),
            licence: false,
            selects,
            shown,
            desktop: None,
            make_default: MakeDefault::Idle,
            terminal_input,
            editing_terminal: false,
            zoom_slider,
            hidden_slider,
            sidebar,
            group_input,
            renaming_group: None,
            width_slider,
            toolbar_search,
            available_cursor: None,
            toolbar_cursor: None,
            shortcut_search,
            _subscriptions: subscriptions,
        }
    }

    /// The terminal field writes the settings only while it is shown.
    fn editing_terminal_allowed(&self) -> bool {
        self.page == Page::General
    }

    /// Scrolls the page shown by most of its height, `direction` down when
    /// positive, as Page Down does in a list.
    fn scroll_page(&mut self, direction: f32, cx: &mut Context<Self>) {
        let height = f32::from(self.page_scroll.bounds().size.height);
        let max = f32::from(self.page_scroll.max_offset().y);
        let now = -f32::from(self.page_scroll.offset().y);
        let to = page_scroll_target(now, direction * height * 0.8, max);
        self.page_scroll
            .set_offset(gpui_kit::point(px(0.), px(-to)));
        cx.notify();
    }

    /// Closes the open drop-down and gives the keyboard back to the dialog.
    pub(crate) fn close_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.selects.open = None;
        // After the click that chose has finished: focusing during it left
        // the keyboard with the list being removed.
        cx.defer_in(window, |this, window, cx| this.focus.focus(window, cx));
        cx.notify();
    }

    /// Puts the shown page's settings back to their defaults.
    fn reset_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.page {
            Page::General => {
                self.editing_terminal = false;
                self.terminal_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                preferences(cx).update(cx, |store, cx| {
                    store.update(Settings::reset_general, cx);
                });
            }
            Page::Appearance => {
                preferences(cx).update(cx, |store, cx| {
                    store.update(Settings::reset_appearance, cx);
                });
            }
            Page::Views => {
                preferences(cx).update(cx, |store, cx| {
                    store.update_default_view(Settings::reset_views, cx);
                });
            }
            Page::Toolbar => {
                self.toolbar_cursor = None;
                preferences(cx).update(cx, |store, cx| {
                    store.update(Settings::reset_toolbar, cx);
                });
            }
            Page::Places => {
                // The groups are content, not settings: they stay.
                preferences(cx).update(cx, |store, cx| {
                    store.update(Settings::reset_places, cx);
                });
            }
            Page::Confirmations => {
                preferences(cx).update(cx, |store, cx| {
                    store.update(Settings::reset_confirmations, cx);
                });
            }
            Page::Shortcuts | Page::About => {}
        }
    }

    /// What takes the keyboard while the dialog is open.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn show(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            self.page = page;
            self.licence = false;
            self.renaming_group = None;
            self.page_scroll.set_offset(gpui_kit::point(px(0.), px(0.)));
            cx.notify();
        }
    }

    /// The list of pages on the left.
    fn page_list(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("settings-pages")
            .role(Role::TabList)
            .aria_label("Settings pages")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(size::SETTINGS_NAV))
            .p(px(space::SM))
            .border_r_1()
            .border_color(theme.border)
            .children(Page::ALL.iter().enumerate().map(|(index, page)| {
                let page = *page;
                let current = page == self.page;
                div()
                    .id(("settings-page", index))
                    .role(Role::Tab)
                    .aria_label(page.name())
                    .aria_selected(current)
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(space::MD))
                    .h(px(size::SETTING_ROW))
                    .pl(px(space::SM - size::SELECTION_BAR))
                    .pr(px(space::MD))
                    .rounded(px(size::RADIUS))
                    .border_l(px(size::SELECTION_BAR))
                    .border_color(gpui_kit::transparent_black())
                    .when(current, |row| {
                        row.bg(theme.current).border_color(theme.accent)
                    })
                    .when(!current, |row| row.hover(|row| row.bg(theme.hover)))
                    .child(
                        Icon::new(page.icon())
                            .size(px(size::ICON))
                            .flex_none()
                            .text_color(if current {
                                theme.accent
                            } else {
                                theme.text_muted
                            }),
                    )
                    .child(div().min_w_0().truncate().child(page.name()))
                    .on_click(cx.listener(move |this, _, _, cx| this.show(page, cx)))
                    .into_any_element()
            }))
            .into_any_element()
    }

    /// The pages as a row of tabs above the page, in a narrow window.
    fn page_tabs(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("settings-tabs")
            .role(Role::TabList)
            .aria_label("Settings pages")
            .flex()
            .flex_none()
            .h(px(size::TABS))
            .overflow_x_scroll()
            .border_b_1()
            .border_color(theme.border)
            .children(Page::ALL.iter().enumerate().map(|(index, page)| {
                let page = *page;
                let current = page == self.page;
                div()
                    .id(("settings-tab", index))
                    .role(Role::Tab)
                    .aria_label(page.name())
                    .aria_selected(current)
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(space::SM))
                    .px(px(space::MD))
                    .border_b(px(size::SELECTION_BAR))
                    .border_color(if current {
                        theme.accent
                    } else {
                        gpui_kit::transparent_black().into()
                    })
                    .text_color(if current {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    .when(!current, |tab| tab.hover(|tab| tab.bg(theme.hover)))
                    .child(Icon::new(page.icon()).size(px(size::ICON)).flex_none())
                    .child(page.name())
                    .on_click(cx.listener(move |this, _, _, cx| this.show(page, cx)))
                    .into_any_element()
            }))
            .into_any_element()
    }

    /// The page shown: its heading and sentence, then its settings.
    fn page(&mut self, theme: Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let content = match self.page {
            Page::General => self.general(theme, window, cx),
            Page::Appearance => self.appearance(theme, window, cx),
            Page::Views => self.views(theme, window, cx),
            Page::Toolbar => self.toolbar_page(theme, window, cx),
            Page::Places => self.places(theme, window, cx),
            Page::Confirmations => self.confirmations(theme, window, cx),
            Page::Shortcuts => self.shortcuts(theme, window, cx),
            Page::About => self.about(theme, cx),
        };
        div()
            .id("settings-page")
            .role(Role::TabPanel)
            .aria_label(self.page.name())
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap(px(space::LG))
            .px(px(space::LG + space::SM))
            .py(px(space::LG))
            .overflow_y_scroll()
            .track_scroll(&self.page_scroll)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::XS))
                    .child(
                        div()
                            .font_family(text::FAMILY_HEADING)
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_size(text::scaled(text::HEADING))
                            .line_height(text::scaled(text::HEADING_LINE))
                            .child(self.page.name()),
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .child(self.page.description()),
                    ),
            )
            .children(content)
            .into_any_element()
    }

    /// Version, licence, credits, privacy and where the settings are.
    fn about(&self, theme: Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if self.licence {
            return vec![
                div()
                    .flex()
                    .child(
                        dialog::button_with_icon(
                            "settings-licence-back",
                            IconName::ArrowLeft,
                            "Back",
                        )
                        .outlined()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.licence = false;
                            cx.notify();
                        })),
                    )
                    .into_any_element(),
                div()
                    .font_family(text::FAMILY_DATA)
                    .text_size(text::scaled(text::SMALL))
                    .text_color(theme.text)
                    .child(LICENCE)
                    .into_any_element(),
            ];
        }
        let store = preferences(cx).read(cx);
        let path = store.path().map(|path| path.to_owned());
        let saved = store.saved();
        let shown_path: SharedString = match &path {
            Some(path) if saved => path.display().to_string().into(),
            Some(path) => {
                format!("{} (made when you first change a setting)", path.display()).into()
            }
            None => "Not saved: a newer Rustphin wrote the settings file.".into(),
        };
        let folder = path
            .filter(|_| saved)
            .and_then(|path| path.parent().map(|folder| folder.to_owned()));
        vec![
            div()
                .flex()
                .items_center()
                .gap(px(space::LG))
                .child(img(LOGO.clone()).size(px(size::ABOUT_LOGO)).flex_none())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::XS))
                        .child(
                            div()
                                .font_family(text::FAMILY_HEADING)
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .text_size(text::scaled(text::TITLE))
                                .line_height(text::scaled(text::TITLE_LINE))
                                .child("Rustphin"),
                        )
                        .child(
                            div()
                                .font_family(text::FAMILY_DATA)
                                .text_size(text::scaled(text::SMALL))
                                .text_color(theme.text_muted)
                                .child(concat!("Version ", env!("CARGO_PKG_VERSION"))),
                        )
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .child("A file manager for KDE Plasma, written in Rust."),
                        ),
                )
                .into_any_element(),
            group(
                "Licence and credits",
                vec![
                    setting_row(
                        "Licence",
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::SM))
                            .child("GPL-3.0-or-later")
                            .child(
                                dialog::button("settings-show-licence", "Show the licence")
                                    .outlined()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.licence = true;
                                        cx.notify();
                                    })),
                            )
                            .into_any_element(),
                    ),
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::XS))
                        .child("Follows the behaviour of Dolphin and KDE.")
                        .child(div().text_color(theme.text_muted).child(
                            "Built with GPUI and GPUI-Kit. Icons: Lucide (ISC). Fonts: Inter, \
                             Space Grotesk and JetBrains Mono (SIL Open Font License).",
                        ))
                        .into_any_element(),
                ],
                theme,
            ),
            group(
                "Privacy and files",
                vec![
                    div()
                        .flex()
                        .items_center()
                        .gap(px(space::SM))
                        .child(
                            Icon::new(IconName::CircleCheck)
                                .size(px(size::ICON))
                                .text_color(theme.success),
                        )
                        .child("No telemetry. Rustphin never uses the network.")
                        .into_any_element(),
                    setting_row(
                        "Settings",
                        div()
                            .min_w_0()
                            .font_family(text::FAMILY_DATA)
                            .text_size(text::scaled(text::SMALL))
                            .child(shown_path)
                            .into_any_element(),
                    ),
                    setting_row(
                        "",
                        dialog::button_with_icon(
                            "settings-open-folder",
                            IconName::FolderOpen,
                            "Open the settings folder",
                        )
                        .outlined()
                        .disabled(folder.is_none())
                        .on_click(cx.listener(move |_, _, _, cx| {
                            if let Some(folder) = folder.clone() {
                                cx.emit(SettingsEvent::OpenFolder(folder));
                            }
                        }))
                        .into_any_element(),
                    ),
                ],
                theme,
            ),
        ]
    }
}

/// Where a page scrolled by `by` from `now` ends, within `0..=max`.
fn page_scroll_target(now: f32, by: f32, max: f32) -> f32 {
    (now + by).clamp(0., max.max(0.))
}

/// A group of settings under its capital label and a hairline.
pub fn group(label: &'static str, rows: Vec<AnyElement>, theme: Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(
            div()
                .pb(px(space::XS))
                .mb(px(space::XS))
                .border_b_1()
                .border_color(theme.border)
                .font_family(text::FAMILY_DATA)
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_size(text::scaled(text::LABEL))
                .text_color(theme.text_muted)
                .child(label.to_uppercase()),
        )
        .children(rows)
        .into_any_element()
}

/// One setting: its label in the label column, its control after it.
pub fn setting_row(label: &'static str, control: AnyElement) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(space::LG))
        .min_h(px(size::SETTING_ROW))
        .child(div().w(px(size::SETTINGS_LABEL)).flex_none().child(label))
        .child(div().flex().flex_1().min_w_0().child(control))
        .into_any_element()
}

impl Render for SettingsDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        let narrow = f32::from(window.viewport_size().width) < size::SETTINGS_NARROW;
        let height = px(size::SETTINGS_HEIGHT).min(dialog::max_height(window));
        let page = self.page(theme, window, cx);
        let middle = if narrow {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.page_tabs(theme, cx))
                .child(page)
        } else {
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(self.page_list(theme, cx))
                .child(page)
        };
        let reset = self.page.has_settings().then(|| {
            dialog::button_with_icon("settings-reset", IconName::RotateCcw, "Reset to defaults")
                .outlined()
                .on_click(cx.listener(|this, _, window, cx| this.reset_page(window, cx)))
        });
        let panel = dialog::panel(
            div().id("settings-dialog"),
            size::SETTINGS_WIDTH,
            theme,
            window,
        )
        .h(height)
        .role(Role::Dialog)
        .aria_label("Settings")
        // Window shortcuts stay quiet while it is open, as for any dialog;
        // the dialogs' letter keys do not apply here (workspace.rs).
        .key_context("Dialog SettingsDialog")
        .child(
            div()
                .track_focus(&self.focus)
                .on_action(cx.listener(|_, _: &CloseSettings, _, cx| {
                    cx.emit(SettingsEvent::Close);
                }))
                // Escape in a text field is the dialogs' Cancel (workspace.rs).
                .on_action(cx.listener(|_, _: &crate::workspace::DialogCancel, _, cx| {
                    cx.emit(SettingsEvent::Close);
                }))
                .on_action(cx.listener(|this, _: &PreviousPage, _, cx| {
                    let page = this.page.step(-1);
                    this.show(page, cx);
                }))
                .on_action(cx.listener(|this, _: &NextPage, _, cx| {
                    let page = this.page.step(1);
                    this.show(page, cx);
                }))
                .on_action(cx.listener(|this, _: &ScrollUp, _, cx| this.scroll_page(-1., cx)))
                .on_action(cx.listener(|this, _: &ScrollDown, _, cx| this.scroll_page(1., cx)))
                .flex()
                .flex_col()
                .size_full()
                .child(dialog::title_row(
                    IconName::Settings,
                    "Settings",
                    "Close (Esc)",
                    cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close)),
                    theme,
                ))
                .child(middle)
                .child(
                    dialog::actions(theme)
                        .justify_between()
                        .child(match reset {
                            Some(reset) => reset.into_any_element(),
                            None => div().into_any_element(),
                        })
                        .child(
                            dialog::button("settings-close", "Close (Esc)")
                                .primary()
                                .on_click(cx.listener(|_, _, _, cx| {
                                    cx.emit(SettingsEvent::Close);
                                })),
                        ),
                ),
        );
        dialog::scrim(panel, theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_and_down_move_through_the_list_and_stop_at_its_ends() {
        assert_eq!(Page::General.step(1), Page::Appearance);
        assert_eq!(Page::About.step(-1), Page::Shortcuts);
        assert_eq!(Page::General.step(-1), Page::General);
        assert_eq!(Page::About.step(1), Page::About);
        assert_eq!(Page::General.step(10), Page::About);
    }

    #[test]
    fn page_down_and_up_stop_at_the_ends_of_the_page() {
        assert_eq!(page_scroll_target(0., 400., 900.), 400.);
        assert_eq!(page_scroll_target(800., 400., 900.), 900.);
        assert_eq!(page_scroll_target(100., -400., 900.), 0.);
        // A page that fits does not move.
        assert_eq!(page_scroll_target(0., 400., 0.), 0.);
    }

    #[test]
    fn the_licence_shown_is_the_gpl_the_crate_declares() {
        assert!(LICENCE.contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(LICENCE.contains("Version 3"));
    }
}
