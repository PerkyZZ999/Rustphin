//! The controls of the Settings pages (docs/DESIGN.md "Settings", mockup
//! 36): segmented choices, drop-downs, small buttons and checkboxes. Each
//! is a library button underneath, so Tab reaches it and Space or Enter
//! uses it; a drop-down's list takes the arrow keys.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, Context, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    anchored,
    assets::IconName,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
    },
    deferred, div,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    controls::Outlined as _,
    menu_view::{self, MenuAccept, MenuClose, MenuDown, MenuUp},
    settings_dialog::SettingsDialog,
    theme::{Theme, size, space, text},
};

/// What a choice does when picked: it changes the dialog and the settings.
pub type Pick<T> = Rc<dyn Fn(&mut SettingsDialog, T, &mut Window, &mut Context<SettingsDialog>)>;

/// The drop-down that is open, if any, and the row the keyboard is on.
pub struct Selects {
    /// The open drop-down's id.
    pub open: Option<&'static str>,
    /// The row the keyboard is on in the open list.
    pub cursor: usize,
    /// Holds the keyboard while a list is open.
    pub focus: FocusHandle,
}

impl Selects {
    /// No list open.
    pub fn new(cx: &mut App) -> Self {
        Self {
            open: None,
            cursor: 0,
            focus: cx.focus_handle(),
        }
    }
}

/// A button 28 pixels high with the design's 12-pixel sides.
fn small_button(id: impl Into<gpui_kit::ElementId>, label: SharedString) -> Button {
    Button::new(id)
        // The library pads a custom-sized button by a fifth of the size.
        .with_size(px(space::MD * 5.))
        .h(px(size::SETTING_CONTROL))
        .accessibility_label(label.clone())
        .child(div().text_size(px(text::BODY)).child(label))
}

/// An outlined 28-pixel button, as "Use another…".
pub fn button(id: &'static str, label: impl Into<SharedString>) -> Button {
    small_button(id, label.into()).outlined()
}

/// Two or three choices side by side inside one outline; the current one
/// in `primary-subtle` with `primary-bright` words. `current` is `None`
/// when none applies (a choice left to KDE).
pub fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    label: &'static str,
    choices: &[(T, &'static str)],
    current: Option<T>,
    theme: Theme,
    pick: Pick<T>,
    cx: &mut Context<SettingsDialog>,
) -> AnyElement {
    div()
        .id(id)
        .role(Role::RadioGroup)
        .aria_label(label)
        .flex()
        .flex_none()
        .rounded(px(size::RADIUS))
        .border_1()
        .border_color(theme.border_strong)
        .overflow_hidden()
        .children(choices.iter().enumerate().map(|(index, (value, words))| {
            let value = *value;
            let on = current == Some(value);
            let pick = pick.clone();
            small_button((id, index), (*words).into())
                .ghost()
                .h(px(size::SETTING_CONTROL - 2. * size::HAIRLINE))
                .rounded(px(0.))
                .when(index > 0, |segment| {
                    segment.border_l_1().border_color(theme.border_strong)
                })
                .when(on, |segment| {
                    segment.bg(theme.current).text_color(theme.accent)
                })
                .when(!on, |segment| segment.text_color(theme.text))
                .toggled(on)
                .on_click(cx.listener(move |this, _, window, cx| {
                    pick(this, value, window, cx);
                }))
                .into_any_element()
        }))
        .into_any_element()
}

/// A 28-pixel drop-down showing the current choice; its list opens under
/// it with the context menu's surface, the current choice ticked.
#[allow(clippy::too_many_arguments)]
pub fn select<T: Copy + PartialEq + 'static>(
    id: &'static str,
    label: &'static str,
    choices: &[(T, &str)],
    current: T,
    selects: &Selects,
    theme: Theme,
    pick: Pick<T>,
    cx: &mut Context<SettingsDialog>,
) -> AnyElement {
    sized_select(
        size::SETTING_SELECT,
        id,
        label,
        choices,
        current,
        selects,
        theme,
        pick,
        cx,
    )
}

/// A [`select`] for short choices, 128 pixels at least, that leaves room
/// for a control beside it (Sort by and its direction).
#[allow(clippy::too_many_arguments)]
pub fn short_select<T: Copy + PartialEq + 'static>(
    id: &'static str,
    label: &'static str,
    choices: &[(T, &str)],
    current: T,
    selects: &Selects,
    theme: Theme,
    pick: Pick<T>,
    cx: &mut Context<SettingsDialog>,
) -> AnyElement {
    sized_select(
        size::SETTING_SELECT_SHORT,
        id,
        label,
        choices,
        current,
        selects,
        theme,
        pick,
        cx,
    )
}

#[allow(clippy::too_many_arguments)]
fn sized_select<T: Copy + PartialEq + 'static>(
    min_width: f32,
    id: &'static str,
    label: &'static str,
    choices: &[(T, &str)],
    current: T,
    selects: &Selects,
    theme: Theme,
    pick: Pick<T>,
    cx: &mut Context<SettingsDialog>,
) -> AnyElement {
    let shown = choices
        .iter()
        .find(|(value, _)| *value == current)
        .map_or(SharedString::default(), |(_, words)| {
            SharedString::from((*words).to_owned())
        });
    let at = choices
        .iter()
        .position(|(value, _)| *value == current)
        .unwrap_or(0);
    let open = selects.open == Some(id);
    let trigger = Button::new(id)
        .with_size(px(space::SM * 5.))
        .h(px(size::SETTING_CONTROL))
        .min_w(px(min_width))
        .outlined()
        .bg(theme.raised)
        .accessibility_label(SharedString::from(format!("{label}: {shown}")))
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .justify_between()
                .gap(px(space::SM))
                .text_size(px(text::BODY))
                .text_color(theme.text)
                .child(shown)
                .child(
                    Icon::new(IconName::ChevronDown)
                        .size(px(size::ICON_SMALL))
                        .text_color(theme.text_muted),
                ),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.selects.open = Some(id);
            this.selects.cursor = at;
            this.selects.focus.focus(window, cx);
            cx.notify();
        }));
    let list = open.then(|| {
        let values: Vec<T> = choices.iter().map(|(value, _)| *value).collect();
        let count = values.len();
        let accept = {
            let pick = pick.clone();
            let values = values.clone();
            cx.listener(
                move |this: &mut SettingsDialog, _: &MenuAccept, window, cx| {
                    let chosen = values.get(this.selects.cursor).copied();
                    this.close_select(window, cx);
                    if let Some(chosen) = chosen {
                        pick(this, chosen, window, cx);
                    }
                },
            )
        };
        deferred(
            // Anchored at the drop-down's top: the list starts under it.
            anchored()
                .offset(gpui_kit::point(
                    px(0.),
                    px(size::SETTING_CONTROL + space::XS),
                ))
                .snap_to_window()
                .child(
                    menu_view::column("settings-select-list", theme)
                        .role(Role::ListBox)
                        .aria_label(label)
                        .key_context(menu_view::CONTEXT)
                        .track_focus(&selects.focus)
                        .on_action(cx.listener(move |this, _: &MenuUp, _, cx| {
                            this.selects.cursor = this.selects.cursor.saturating_sub(1);
                            cx.notify();
                        }))
                        .on_action(cx.listener(move |this, _: &MenuDown, _, cx| {
                            this.selects.cursor = (this.selects.cursor + 1).min(count - 1);
                            cx.notify();
                        }))
                        .on_action(accept)
                        .on_action(cx.listener(|this, _: &MenuClose, window, cx| {
                            this.close_select(window, cx);
                        }))
                        .children(choices.iter().enumerate().map(|(index, (value, words))| {
                            let value = *value;
                            let words = SharedString::from((*words).to_owned());
                            let chosen = value == current;
                            let under = index == selects.cursor;
                            let pick = pick.clone();
                            div()
                                .id(("settings-select-row", index))
                                .role(Role::ListBoxOption)
                                .aria_label(words.clone())
                                .aria_selected(chosen)
                                .flex()
                                .flex_none()
                                .items_center()
                                .gap(px(space::MD))
                                .h(px(size::MENU_ROW))
                                .px(px(space::SM - size::SELECTION_BAR))
                                .rounded(px(size::RADIUS))
                                .border_l(px(size::SELECTION_BAR))
                                .border_color(gpui_kit::transparent_black())
                                .text_color(theme.text)
                                .when(under, |row| {
                                    row.bg(theme.selection)
                                        .border_color(theme.accent)
                                        .text_color(theme.on_selection)
                                })
                                .when(!under, |row| row.hover(|row| row.bg(theme.hover)))
                                .child(div().size(px(size::ICON)).flex_none().when(
                                    chosen,
                                    |slot| {
                                        slot.child(
                                            Icon::new(IconName::Check)
                                                .size(px(size::ICON))
                                                .text_color(if under {
                                                    theme.on_selection
                                                } else {
                                                    theme.accent
                                                }),
                                        )
                                    },
                                ))
                                .child(div().flex_1().min_w_0().truncate().child(words))
                                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                    if this.selects.cursor != index {
                                        this.selects.cursor = index;
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.close_select(window, cx);
                                    pick(this, value, window, cx);
                                }))
                        })),
                ),
        )
        .with_priority(1)
    });
    div()
        .flex()
        .flex_col()
        .flex_none()
        .child(trigger)
        .children(list)
        .into_any_element()
}

/// A checkbox with its words, in the body size.
pub fn checkbox(
    id: &'static str,
    label: &'static str,
    checked: bool,
    pick: Pick<bool>,
    cx: &mut Context<SettingsDialog>,
) -> AnyElement {
    Checkbox::new(id)
        .text_size(px(text::BODY))
        .label(label)
        .accessibility_label(label)
        .checked(checked)
        .on_click(cx.listener(move |this, checked: &bool, window, cx| {
            pick(this, *checked, window, cx);
        }))
        .into_any_element()
}

/// A hint after a control, in the small muted words.
pub fn hint(words: impl Into<SharedString>, theme: Theme) -> AnyElement {
    div()
        .text_size(px(text::SMALL))
        .text_color(theme.text_muted)
        .child(words.into())
        .into_any_element()
}

/// A line that says something is fine, with the success check.
pub fn status_ok(words: impl Into<SharedString>, theme: Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(space::SM))
        .child(
            Icon::new(IconName::CircleCheck)
                .size(px(size::ICON))
                .flex_none()
                .text_color(theme.success),
        )
        .child(words.into())
        .into_any_element()
}
