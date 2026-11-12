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
    AnyElement, App, Context, EventEmitter, FocusHandle, Image, ImageFormat,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, Role,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Window, actions,
    assets::IconName,
    component::{Disableable as _, Icon, button::ButtonVariants as _},
    div, img,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    controls::Outlined as _,
    dialog,
    preferences::preferences,
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
    /// Version, licence, credits, privacy and files.
    About,
}

impl Page {
    /// The pages in the list, top to bottom.
    pub const ALL: &'static [Page] = &[Page::About];

    /// Its name in the list and as its heading.
    pub fn name(self) -> &'static str {
        match self {
            Page::About => "About",
        }
    }

    /// Its icon in the list.
    pub fn icon(self) -> IconName {
        match self {
            Page::About => IconName::Info,
        }
    }

    /// The sentence under its heading.
    pub fn description(self) -> &'static str {
        match self {
            Page::About => "Rustphin and the work it builds on.",
        }
    }

    /// Whether it holds settings, and so a "Reset to defaults".
    pub fn has_settings(self) -> bool {
        match self {
            Page::About => false,
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
}

impl EventEmitter<SettingsEvent> for SettingsDialog {}

impl SettingsDialog {
    /// The dialog on its first page.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            page: Page::ALL[0],
            focus: cx.focus_handle(),
            page_scroll: ScrollHandle::new(),
            licence: false,
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
    fn page(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = match self.page {
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
                            .text_size(px(text::HEADING))
                            .line_height(px(text::HEADING_LINE))
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
                    .text_size(px(text::SMALL))
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
                                .text_size(px(text::TITLE))
                                .line_height(px(text::TITLE_LINE))
                                .child("Rustphin"),
                        )
                        .child(
                            div()
                                .font_family(text::FAMILY_DATA)
                                .text_size(px(text::SMALL))
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
                            .text_size(px(text::SMALL))
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
                .text_size(px(text::LABEL))
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
        let page = self.page(theme, cx);
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
                .on_action(cx.listener(|this, _: &PreviousPage, _, cx| {
                    let page = this.page.step(-1);
                    this.show(page, cx);
                }))
                .on_action(cx.listener(|this, _: &NextPage, _, cx| {
                    let page = this.page.step(1);
                    this.show(page, cx);
                }))
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
    fn the_licence_shown_is_the_gpl_the_crate_declares() {
        assert!(LICENCE.contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(LICENCE.contains("Version 3"));
    }
}
