//! The Appearance page of Settings (W-106d, mockup 36b): glass or opaque,
//! whose title bar, the text size and how faded hidden files are. The
//! design stays dark; there is no light theme or accent choice.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window,
    component::slider::Slider, div, px,
};

use crate::{
    preferences::preferences,
    settings::{Look, Settings, TextSize, TitleBar},
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group, setting_row},
    theme::{Theme, size, space, text},
};

/// Changes one of the page's values, keeping the others.
fn choose(
    cx: &mut Context<SettingsDialog>,
    change: impl FnOnce(&mut Look, &mut TitleBar, &mut TextSize, &mut u32),
) {
    preferences(cx).update(cx, |store, cx| {
        store.update(
            |settings: &mut Settings| {
                let (mut look, mut bar, mut text, mut hidden) = (
                    settings.look(),
                    settings.title_bar(),
                    settings.text_size(),
                    settings.hidden_opacity(),
                );
                change(&mut look, &mut bar, &mut text, &mut hidden);
                settings.set_appearance(look, bar, text, hidden);
            },
            cx,
        );
    });
}

impl SettingsDialog {
    /// The Appearance page's groups.
    pub(crate) fn appearance(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let settings = preferences(cx).read(cx).settings().clone();
        let hidden = settings.hidden_opacity();
        if self.hidden_slider.read(cx).value().start() != hidden as f32 {
            self.hidden_slider
                .update(cx, |slider, cx| slider.set_value(hidden as f32, window, cx));
        }
        let pick_look: Pick<Look> = Rc::new(|_, chosen, _, cx| {
            choose(cx, |look, _, _, _| *look = chosen);
        });
        let pick_bar: Pick<TitleBar> = Rc::new(|_, chosen, _, cx| {
            choose(cx, |_, bar, _, _| *bar = chosen);
        });
        let pick_text: Pick<TextSize> = Rc::new(|_, chosen, _, cx| {
            choose(cx, |_, _, text, _| *text = chosen);
        });
        let with_hint = |control: AnyElement, words: &'static str| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(space::SM))
                .child(control)
                .child(controls::hint(words, theme))
                .into_any_element()
        };
        let window_group = vec![
            setting_row(
                "Look",
                with_hint(
                    controls::segmented(
                        "settings-look",
                        "Look",
                        &[(Look::Glass, "Glass"), (Look::Opaque, "Opaque")],
                        Some(settings.look()),
                        theme,
                        pick_look,
                        cx,
                    ),
                    "Blur shows only where the desktop offers it.",
                ),
            ),
            setting_row(
                "Title bar",
                with_hint(
                    controls::segmented(
                        "settings-title-bar",
                        "Title bar",
                        &[
                            (TitleBar::Rustphin, "Rustphin\u{2019}s own"),
                            (TitleBar::Kde, "KDE\u{2019}s"),
                        ],
                        Some(settings.title_bar()),
                        theme,
                        pick_bar,
                        cx,
                    ),
                    "Applies to new windows.",
                ),
            ),
        ];
        let text_group = vec![
            setting_row(
                "Text size",
                controls::segmented(
                    "settings-text-size",
                    "Text size",
                    &[
                        (TextSize::Smaller, "Smaller"),
                        (TextSize::Normal, "Normal"),
                        (TextSize::Larger, "Larger"),
                    ],
                    Some(settings.text_size()),
                    theme,
                    pick_text,
                    cx,
                ),
            ),
            setting_row(
                "Hidden files are drawn at",
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::MD))
                    .child(
                        div().w(px(size::SETTING_SELECT)).child(
                            Slider::new(&self.hidden_slider)
                                .horizontal()
                                .bg(theme.accent)
                                .text_color(theme.text),
                        ),
                    )
                    .child(
                        div()
                            .font_family(text::FAMILY_DATA)
                            .text_size(text::scaled(text::SMALL))
                            .child(format!("{hidden} %")),
                    )
                    .into_any_element(),
            ),
        ];
        vec![
            group("Window", window_group, theme),
            group("Text and files", text_group, theme),
        ]
    }
}

/// Sets hidden files' opacity from the slider, kept readable by the
/// settings.
pub(crate) fn choose_hidden(cx: &mut Context<SettingsDialog>, percent: u32) {
    choose(cx, |_, _, _, hidden| *hidden = percent);
}
