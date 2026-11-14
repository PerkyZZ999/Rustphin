//! The Toolbar page of Settings (W-106f, mockup 36d), as Dolphin's
//! "Configure Toolbars…": what is available, what is on the toolbar, and
//! the buttons to move items between and along the lists.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{Icon, input::Input},
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    controls::tool_button,
    preferences::preferences,
    settings_controls::{self as controls, Pick},
    settings_dialog::{SettingsDialog, group},
    theme::{Theme, size, space, text},
    toolbar_items::{BUILT_IN, Item, Toolbar},
};

/// How tall each list is drawn; it scrolls beyond.
const LIST_HEIGHT: f32 = 256.;

/// The icon an item shows in the lists and the preview.
pub fn item_icon(item: &Item) -> IconName {
    crate::icon_map::gpui_icon(item.icon())
}

/// What "Available actions" offers: the built-in buttons and every palette
/// command not on the toolbar yet (a separator always), matching `search`.
pub fn available(toolbar: &Toolbar, search: &str) -> Vec<Item> {
    let search = search.trim().to_lowercase();
    BUILT_IN
        .iter()
        .cloned()
        .chain(
            crate::commands::commands()
                .iter()
                .map(|command| Item::Command(command.label.to_owned())),
        )
        .filter(|item| !toolbar.contains(item))
        .filter(|item| search.is_empty() || item.name().to_lowercase().contains(&search))
        .collect()
}

/// Changes the toolbar and saves it.
fn change(cx: &mut Context<SettingsDialog>, edit: impl FnOnce(&mut Toolbar)) {
    preferences(cx).update(cx, |store, cx| {
        store.update(
            |settings| {
                let mut toolbar = settings.toolbar();
                edit(&mut toolbar);
                settings.set_toolbar(&toolbar);
            },
            cx,
        );
    });
}

impl SettingsDialog {
    /// The Toolbar page's groups.
    pub(crate) fn toolbar_page(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let toolbar = preferences(cx).read(cx).settings().toolbar();
        let search = self.toolbar_search.read(cx).value().to_string();
        let offered = available(&toolbar, &search);
        self.available_cursor = self.available_cursor.filter(|at| *at < offered.len());
        self.toolbar_cursor = self.toolbar_cursor.filter(|at| *at < toolbar.items.len());

        let preview = div()
            .id("toolbar-preview")
            .role(gpui_kit::Role::Image)
            .aria_label("Preview of the toolbar")
            .flex()
            .items_center()
            .gap(px(space::XS))
            .h(px(size::TOOLBAR))
            .px(px(space::SM))
            .rounded(px(size::RADIUS))
            .bg(theme.chrome)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .children(toolbar.items.iter().map(|(item, words)| {
                match item {
                    Item::Separator => div()
                        .flex_none()
                        .w(px(size::HAIRLINE))
                        .h(px(size::TOOL_SEPARATOR))
                        .mx(px(space::XS))
                        .bg(theme.border_strong)
                        .into_any_element(),
                    Item::Location => div()
                        .flex_1()
                        .min_w(px(space::LG * 4.))
                        .h(px(size::SETTING_CONTROL))
                        .rounded(px(size::RADIUS))
                        .bg(theme.content)
                        .border_1()
                        .border_color(theme.border_strong)
                        .into_any_element(),
                    other => div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(space::XS))
                        .px(px(space::XS))
                        .text_size(text::scaled(text::SMALL))
                        .child(Icon::new(item_icon(other)).size(px(size::ICON)))
                        .when(*words || *other == Item::Actions, |row| {
                            row.child(other.name())
                        })
                        .into_any_element(),
                }
            }))
            .into_any_element();

        let available_rows: Vec<AnyElement> = offered
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let current = self.available_cursor == Some(index);
                let item_for_add = item.clone();
                self.list_row(("available", index), item, None, current, theme)
                    .on_click(
                        cx.listener(move |this, event: &gpui_kit::ClickEvent, _, cx| {
                            this.available_cursor = Some(index);
                            if event.click_count() >= 2 {
                                this.add_to_toolbar(item_for_add.clone(), cx);
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element()
            })
            .collect();
        let toolbar_rows: Vec<AnyElement> = toolbar
            .items
            .iter()
            .enumerate()
            .map(|(index, (item, _))| {
                let current = self.toolbar_cursor == Some(index);
                let note = item.is_fixed().then_some("always there");
                self.list_row(("on-toolbar", index), item, note, current, theme)
                    .on_click(
                        cx.listener(move |this, event: &gpui_kit::ClickEvent, _, cx| {
                            this.toolbar_cursor = Some(index);
                            if event.click_count() >= 2 {
                                this.remove_from_toolbar(index, cx);
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element()
            })
            .collect();

        let chosen = self
            .toolbar_cursor
            .and_then(|at| toolbar.items.get(at).cloned());
        let can_add = self.available_cursor.is_some();
        let can_remove = chosen.as_ref().is_some_and(|(item, _)| !item.is_fixed());
        let can_left = self.toolbar_cursor.is_some_and(|at| at > 0);
        let can_right = self
            .toolbar_cursor
            .is_some_and(|at| at + 1 < toolbar.items.len());
        let words_on = chosen.as_ref().is_some_and(|(_, words)| *words);
        let words_possible = chosen
            .as_ref()
            .is_some_and(|(item, _)| item.can_show_words());
        let pick_words: Pick<bool> = Rc::new(|this, on, _, cx| {
            if let Some(at) = this.toolbar_cursor {
                change(cx, |toolbar| toolbar.set_words(at, on));
            }
        });

        let list =
            |id: &'static str, label: &'static str, rows: Vec<AnyElement>, empty: &'static str| {
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XS))
                    .child(
                        div()
                            .text_size(text::scaled(text::SMALL))
                            .text_color(theme.text_muted)
                            .child(label),
                    )
                    .child(
                        div()
                            .id(id)
                            .role(gpui_kit::Role::ListBox)
                            .aria_label(label)
                            .flex()
                            .flex_col()
                            .h(px(LIST_HEIGHT))
                            .overflow_y_scroll()
                            .p(px(space::XS))
                            .rounded(px(size::RADIUS))
                            .border_1()
                            .border_color(theme.border)
                            .when(rows.is_empty(), |list| {
                                list.child(controls::hint(empty, theme))
                            })
                            .children(rows),
                    )
            };

        let middle = div()
            .flex()
            .flex_col()
            .flex_none()
            .justify_center()
            .gap(px(space::SM))
            .pt(px(space::LG + space::XS))
            .child(
                tool_button(
                    "toolbar-add",
                    IconName::ArrowRight,
                    "Add to the toolbar",
                    can_add,
                    None,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    let toolbar = preferences(cx).read(cx).settings().toolbar();
                    let search = this.toolbar_search.read(cx).value().to_string();
                    let item = this
                        .available_cursor
                        .and_then(|at| available(&toolbar, &search).get(at).cloned());
                    if let Some(item) = item {
                        this.add_to_toolbar(item, cx);
                    }
                })),
            )
            .child(
                tool_button(
                    "toolbar-remove",
                    IconName::ArrowLeft,
                    "Take off the toolbar",
                    can_remove,
                    None,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(at) = this.toolbar_cursor {
                        this.remove_from_toolbar(at, cx);
                    }
                })),
            );

        let below = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(space::SM))
            .child(
                tool_button(
                    "toolbar-left",
                    IconName::ArrowUp,
                    "Move left on the toolbar",
                    can_left,
                    None,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.shift_on_toolbar(-1, cx))),
            )
            .child(
                tool_button(
                    "toolbar-right",
                    IconName::ArrowDown,
                    "Move right on the toolbar",
                    can_right,
                    None,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.shift_on_toolbar(1, cx))),
            )
            .when(words_possible, |row| {
                row.child(controls::checkbox(
                    "toolbar-words",
                    "Show its words",
                    words_on,
                    pick_words,
                    cx,
                ))
            });

        let lists = div()
            .flex()
            .gap(px(space::SM))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XS))
                    .child(crate::input::named(
                        Input::new(&self.toolbar_search).min_h(px(size::SETTING_CONTROL)),
                        &self.toolbar_search,
                        "Search the available actions",
                        window,
                        cx,
                    ))
                    .child(list(
                        "toolbar-available",
                        "Available actions",
                        available_rows,
                        "Nothing matches; everything else is on the toolbar.",
                    )),
            )
            .child(middle)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XS))
                    .pt(px(size::SETTING_CONTROL + space::XS))
                    .child(list("toolbar-chosen", "On the toolbar", toolbar_rows, ""))
                    .child(below),
            );

        vec![
            group("Preview", vec![preview], theme),
            group("Items", vec![lists.into_any_element()], theme),
        ]
    }

    /// A row of either list: icon, name and a note.
    fn list_row(
        &self,
        id: (&'static str, usize),
        item: &Item,
        note: Option<&'static str>,
        current: bool,
        theme: Theme,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let name: SharedString = item.name().into();
        div()
            .id(id)
            .role(gpui_kit::Role::ListBoxOption)
            .aria_label(name.clone())
            .aria_selected(current)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(space::SM))
            .h(px(size::SETTING_ROW))
            .px(px(space::SM - size::SELECTION_BAR))
            .rounded(px(size::RADIUS))
            .border_l(px(size::SELECTION_BAR))
            .border_color(gpui_kit::transparent_black())
            .when(current, |row| {
                row.bg(theme.selection)
                    .border_color(theme.accent)
                    .text_color(theme.on_selection)
            })
            .when(!current, |row| row.hover(|row| row.bg(theme.hover)))
            .child(Icon::new(item_icon(item)).size(px(size::ICON)).flex_none())
            .child(div().flex_1().min_w_0().truncate().child(name))
            .when_some(note, |row, note| {
                row.child(
                    div()
                        .flex_none()
                        .text_size(text::scaled(text::SMALL))
                        .text_color(if current {
                            theme.on_selection
                        } else {
                            theme.text_muted
                        })
                        .child(note),
                )
            })
    }

    /// Puts `item` on the toolbar after the chosen item, or before Actions.
    fn add_to_toolbar(&mut self, item: Item, cx: &mut Context<Self>) {
        let after = self.toolbar_cursor;
        let mut placed = None;
        change(cx, |toolbar| placed = toolbar.add(item, after));
        if placed.is_some() {
            self.toolbar_cursor = placed;
        }
        cx.notify();
    }

    /// Takes the item at `index` off the toolbar, unless it must stay.
    fn remove_from_toolbar(&mut self, index: usize, cx: &mut Context<Self>) {
        change(cx, |toolbar| {
            toolbar.remove(index);
        });
        cx.notify();
    }

    /// Moves the chosen item along the toolbar.
    fn shift_on_toolbar(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(at) = self.toolbar_cursor {
            let mut to = at;
            change(cx, |toolbar| to = toolbar.shift(at, delta));
            self.toolbar_cursor = Some(to);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_actions_leave_out_what_is_on_the_toolbar_and_follow_the_search() {
        let toolbar = Toolbar::default();
        let offered = available(&toolbar, "");
        assert!(!offered.contains(&Item::Back), "Back is on the toolbar");
        assert!(offered.contains(&Item::Separator), "a separator always");
        assert!(offered.iter().any(|item| matches!(item, Item::Command(_))));
        let found = available(&toolbar, "  COPY LOCATION ");
        assert!(!found.is_empty());
        assert!(
            found
                .iter()
                .all(|item| item.name().to_lowercase().contains("copy location"))
        );
        assert!(available(&toolbar, "no such action at all").is_empty());
    }
}
