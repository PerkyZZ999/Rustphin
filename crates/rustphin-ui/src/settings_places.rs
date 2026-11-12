//! The Places page of Settings (W-106e2, mockup 36e): what the places
//! panel shows, its groups and its width. The groups are the panel's own
//! (`place-groups.toml`); this page changes them through the panel.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window,
    assets::IconName,
    component::{input::Input, slider::Slider},
    div, px,
};

use crate::{
    controls::tool_button,
    preferences::preferences,
    settings::PlacesChoice,
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group, setting_row},
    theme::{Theme, size, space, text},
};

/// Changes one of the page's choices, keeping the others.
fn choose(cx: &mut Context<SettingsDialog>, change: impl FnOnce(&mut PlacesChoice)) {
    preferences(cx).update(cx, |store, cx| {
        store.update(
            |settings| {
                let mut choice = settings.places();
                change(&mut choice);
                settings.set_places(choice);
            },
            cx,
        );
    });
}

/// Sets the panel's width from the slider.
pub(crate) fn choose_width(cx: &mut Context<SettingsDialog>, width: u32) {
    choose(cx, |choice| choice.width = width);
}

impl SettingsDialog {
    /// The Places page's groups.
    pub(crate) fn places(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let choice = preferences(cx).read(cx).settings().places();
        if self.width_slider.read(cx).value().start() != choice.width as f32 {
            self.width_slider.update(cx, |slider, cx| {
                slider.set_value(choice.width as f32, window, cx)
            });
        }
        let section = |id: &'static str,
                       label: &'static str,
                       checked: bool,
                       set: fn(&mut PlacesChoice, bool),
                       cx: &mut Context<Self>| {
            let pick: Pick<bool> = Rc::new(move |_, on, _, cx| {
                choose(cx, |choice| set(choice, on));
            });
            controls::checkbox(id, label, checked, pick, cx)
        };
        let pick_hidden: Pick<bool> = Rc::new(|_, show, _, cx| {
            choose(cx, |choice| choice.show_hidden = show);
        });
        let sections = vec![
            setting_row(
                "Show",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(space::LG))
                    .child(section(
                        "settings-show-devices",
                        "Devices",
                        choice.devices,
                        |c, on| c.devices = on,
                        cx,
                    ))
                    .child(section(
                        "settings-show-remote",
                        "Remote",
                        choice.remote,
                        |c, on| c.remote = on,
                        cx,
                    ))
                    .into_any_element(),
            ),
            setting_row(
                "Hidden places",
                controls::segmented(
                    "settings-hidden-places",
                    "Hidden places",
                    &[(false, "Keep them hidden"), (true, "Show them faded")],
                    Some(choice.show_hidden),
                    theme,
                    pick_hidden,
                    cx,
                ),
            ),
        ];

        let groups = self.sidebar.read(cx).group_summaries();
        let count = groups.len();
        let mut rows: Vec<AnyElement> = groups
            .into_iter()
            .enumerate()
            .map(|(index, (name, places))| {
                self.group_row(index, count, name, places, theme, window, cx)
            })
            .collect();
        rows.push(controls::hint(
            if count == 0 {
                "No groups yet. Make one with New group in the places panel."
            } else {
                "Deleting a group keeps its places."
            },
            theme,
        ));
        let panel = vec![setting_row(
            "Width",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(space::MD))
                .child(
                    div().w(px(size::SETTING_SELECT)).child(
                        Slider::new(&self.width_slider)
                            .horizontal()
                            .bg(theme.accent)
                            .text_color(theme.text),
                    ),
                )
                .child(
                    div()
                        .font_family(text::FAMILY_DATA)
                        .text_size(text::scaled(text::SMALL))
                        .child(format!("{} px", choice.width)),
                )
                .child(controls::hint("or drag the panel\u{2019}s edge", theme))
                .into_any_element(),
        )];
        vec![
            group("Sections", sections, theme),
            group(
                "Groups",
                vec![
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::XS))
                        .children(rows)
                        .into_any_element(),
                ],
                theme,
            ),
            group("Panel", panel, theme),
        ]
    }

    /// One group: its name (or the field renaming it), how many places,
    /// and its buttons.
    #[allow(clippy::too_many_arguments)]
    fn group_row(
        &self,
        index: usize,
        count: usize,
        name: String,
        places: usize,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let renaming = self.renaming_group == Some(index);
        let label: SharedString = name.clone().into();
        let name_part = if renaming {
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(size::SETTING_FIELD))
                .child(crate::input::named(
                    Input::new(&self.group_input).min_h(px(size::SETTING_CONTROL)),
                    &self.group_input,
                    "Group name",
                    window,
                    cx,
                ))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(space::SM))
                .child(div().min_w_0().truncate().child(label.clone()))
                .child(controls::hint(
                    format!(
                        "{places} {}",
                        crate::format::plural(places, "place", "places")
                    ),
                    theme,
                ))
                .into_any_element()
        };
        let sidebar = self.sidebar.clone();
        let up_sidebar = sidebar.clone();
        let down_sidebar = sidebar.clone();
        let delete_sidebar = sidebar.clone();
        div()
            .id(("settings-group", index))
            .flex()
            .items_center()
            .gap(px(space::XS))
            .min_h(px(size::SETTING_ROW))
            .px(px(space::SM))
            .rounded(px(size::RADIUS))
            .border_1()
            .border_color(theme.border)
            .child(name_part)
            .child(
                tool_button(
                    ("settings-group-up", index),
                    IconName::ArrowUp,
                    format!("Move {name} up"),
                    index > 0,
                    None,
                    &theme,
                )
                .on_click(move |_, _, cx| {
                    up_sidebar.update(cx, |sidebar, cx| sidebar.shift_group(index, -1, cx));
                }),
            )
            .child(
                tool_button(
                    ("settings-group-down", index),
                    IconName::ArrowDown,
                    format!("Move {name} down"),
                    index + 1 < count,
                    None,
                    &theme,
                )
                .on_click(move |_, _, cx| {
                    down_sidebar.update(cx, |sidebar, cx| sidebar.shift_group(index, 1, cx));
                }),
            )
            .child(
                tool_button(
                    ("settings-group-rename", index),
                    if renaming {
                        IconName::Check
                    } else {
                        IconName::Pencil
                    },
                    if renaming {
                        format!("Keep the name of {name} (Enter)")
                    } else {
                        format!("Rename {name}")
                    },
                    true,
                    None,
                    &theme,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.renaming_group == Some(index) {
                        this.finish_group_rename(cx);
                    } else {
                        this.start_group_rename(index, window, cx);
                    }
                })),
            )
            .child(
                tool_button(
                    ("settings-group-delete", index),
                    IconName::Trash,
                    format!("Delete {name}; its places stay"),
                    true,
                    None,
                    &theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.renaming_group = None;
                    delete_sidebar.update(cx, |sidebar, cx| sidebar.delete_group(index, cx));
                    cx.notify();
                })),
            )
            .into_any_element()
    }

    /// Opens the name field on group `index`.
    fn start_group_rename(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .sidebar
            .read(cx)
            .group_summaries()
            .get(index)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();
        self.renaming_group = Some(index);
        self.group_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
        });
        let focus = gpui_kit::Focusable::focus_handle(self.group_input.read(cx), cx);
        focus.focus(window, cx);
        cx.notify();
    }

    /// Keeps the name typed for the group being renamed; an empty name is
    /// ignored, as in the panel.
    pub(crate) fn finish_group_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.renaming_group.take() {
            let name = self.group_input.read(cx).value().to_string();
            self.sidebar
                .update(cx, |sidebar, cx| sidebar.rename_group(index, &name, cx));
        }
        cx.notify();
    }
}
