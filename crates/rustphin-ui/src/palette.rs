//! Search and selection on a solid floating surface. Ranking runs off-thread.

use std::{sync::Arc, time::Instant};

use crate::theme::{Theme, size, space, text};
use gpui_kit::{
    AccessibleAction, App, AppContext as _, Context, Entity, EventEmitter, Focusable as _,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _, Render,
    Role, ScrollStrategy, StatefulInteractiveElement as _, Styled as _, Subscription, Task,
    UniformListScrollHandle, Window, actions,
    component::input::{Input, InputEvent, InputState},
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};
use rustphin_core::SearchQuery;

const VISIBLE_RESULTS: usize = 7;
actions!(palette, [Previous, Next, Confirm, Cancel]);

pub fn bind_keys(cx: &mut App) {
    let context = Some("Palette > Input");
    cx.bind_keys([
        KeyBinding::new("up", Previous, context),
        KeyBinding::new("shift-tab", Previous, context),
        KeyBinding::new("down", Next, context),
        KeyBinding::new("tab", Next, context),
        KeyBinding::new("enter", Confirm, context),
        KeyBinding::new("escape", Cancel, context),
    ]);
}

pub use rustphin_view::palette::{Candidate, Target};
use rustphin_view::palette::{Results, result_icon, split_label};

/// The icon of a command, as its palette row and toolbar button show it.
pub(crate) fn command_icon(label: &str) -> gpui_kit::assets::IconName {
    crate::icon_map::gpui_icon(rustphin_view::palette::command_icon(label))
}

pub enum PaletteEvent {
    Chosen(Target),
    Closed,
}

pub struct Palette {
    input: Entity<InputState>,
    candidates: Arc<Vec<Candidate>>,
    results: Results,
    search_task: Option<Task<()>>,
    query_started: Option<Instant>,
    scroll: UniformListScrollHandle,
    status: Option<String>,
    _input: Subscription,
}

impl EventEmitter<PaletteEvent> for Palette {}

impl Palette {
    pub fn new(
        candidates: Vec<Candidate>,
        status: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search actions, places, tabs and recent folders\u{2026}")
        });
        let subscription =
            cx.subscribe(
                &input,
                |this: &mut Self, _, event: &InputEvent, cx| match event {
                    InputEvent::Change => this.search(cx),
                    InputEvent::PressEnter { .. } => this.confirm(cx),
                    InputEvent::Blur | InputEvent::Focus => {}
                },
            );
        input.read(cx).focus_handle(cx).focus(window, cx);
        let mut palette = Self {
            input,
            candidates: Arc::new(candidates),
            results: Results::default(),
            search_task: None,
            query_started: None,
            scroll: UniformListScrollHandle::new(),
            status,
            _input: subscription,
        };
        palette.search(cx);
        palette
    }

    pub fn set_candidates(
        &mut self,
        candidates: Vec<Candidate>,
        status: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.candidates = Arc::new(candidates);
        self.status = status;
        self.search(cx);
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        self.query_started = Some(Instant::now());
        let generation = self.results.begin();
        let query = SearchQuery::new(&self.input.read(cx).value());
        let candidates = self.candidates.clone();
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        let ranked = cx.background_executor().spawn(async move {
            query.rank(
                candidates.iter().map(|candidate| candidate.search.as_str()),
                usize::MAX,
            )
        });
        self.search_task = Some(cx.spawn(async move |palette, cx| {
            let results = ranked.await;
            let _ = palette.update(cx, |palette, cx| {
                if palette.results.complete(generation, results) {
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.results.move_by(delta);
        self.scroll
            .scroll_to_item(self.results.selected, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn confirm(&self, cx: &mut Context<Self>) {
        if let Some(index) = self.results.chosen() {
            cx.emit(PaletteEvent::Chosen(self.candidates[index].target.clone()));
        }
    }

    fn row(&self, row: usize, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let candidate = &self.candidates[self.results.indices[row]];
        let selected = self.results.selected == row;
        let (category, name) = split_label(&candidate.label);
        let trail = if category.is_empty() {
            candidate.detail.clone()
        } else {
            format!("{category} \u{203a} {}", candidate.detail)
        };
        let quiet = if selected {
            theme.on_selection
        } else {
            theme.text_muted
        };
        let generation = self.results.generation;
        let palette = cx.entity().downgrade();
        let select = move |_: Option<&gpui_kit::accesskit::ActionData>,
                           window: &mut Window,
                           cx: &mut App| {
            let _ = palette.update(cx, |this, cx| {
                if !this.results.select(generation, row) {
                    return;
                }
                this.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
                this.input.focus_handle(cx).focus(window, cx);
                cx.notify();
            });
        };
        div()
            .id(("palette-result", row))
            .role(Role::ListBoxOption)
            .aria_label(candidate.label.clone())
            .aria_description(format!(
                "{}. Select, then press Enter to run.",
                candidate.detail
            ))
            .aria_keyshortcuts(candidate.shortcut.clone())
            .aria_selected(selected)
            .aria_position_in_set(row)
            .on_a11y_action(AccessibleAction::Focus, select.clone())
            .on_a11y_action(AccessibleAction::Click, select)
            .w_full()
            .flex()
            .items_center()
            .h(px(size::PALETTE_ROW))
            .gap(px(space::MD))
            .pl(px(space::MD - size::SELECTION_BAR))
            .pr(px(space::MD))
            .rounded(px(size::RADIUS))
            // The selection look: fill and bar, as in menus and the list.
            .border_l(px(size::SELECTION_BAR))
            .border_color(gpui_kit::transparent_black())
            .when(selected, |row| {
                row.bg(theme.selection)
                    .border_color(theme.accent)
                    .text_color(theme.on_selection)
            })
            .hover(|row| {
                row.bg(if selected {
                    theme.selection_hover
                } else {
                    theme.hover
                })
            })
            .child(
                gpui_kit::component::Icon::new(crate::icon_map::gpui_icon(result_icon(
                    &candidate.label,
                )))
                .size(px(size::PALETTE_ICON))
                .flex_none()
                .text_color(quiet),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .text_size(text::scaled(text::NAME))
                            .child(name.to_owned()),
                    )
                    .child(
                        div()
                            .truncate()
                            .font_family(text::FAMILY_DATA)
                            .text_size(text::scaled(text::SMALL))
                            .line_height(text::scaled(text::LINE - space::XS))
                            .text_color(quiet)
                            .child(trail),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_DATA)
                    .text_size(text::scaled(text::SMALL))
                    .text_color(quiet)
                    .child(candidate.shortcut.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.results.select(generation, row) {
                    this.confirm(cx);
                }
            }))
    }
}

impl Render for Palette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        let result_status = if self.results.pending {
            "Searching…".to_owned()
        } else if let Some(index) = self.results.chosen() {
            format!(
                "{} results. Selected {} of {}: {}. Enter to run, Escape to close.",
                self.results.indices.len(),
                self.results.selected + 1,
                self.results.indices.len(),
                self.candidates[index].label,
            )
        } else {
            "No matches. Try another action name or folder path.".to_owned()
        };
        if !self.results.pending
            && let Some(started) = self.query_started.take()
        {
            tracing::debug!(
                elapsed_us = started.elapsed().as_micros(),
                results = self.results.indices.len(),
                "palette results rendered"
            );
        }
        // Keep at least one results row, and fit the list on short windows.
        let height =
            (f32::from(window.viewport_size().height) - size::TOOLBAR * 4. - space::LG * 2.)
                .min(size::PALETTE_ROW * VISIBLE_RESULTS as f32)
                .max(size::PALETTE_ROW);
        let results = uniform_list(
            "palette-results",
            self.results.indices.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|row| this.row(row, theme, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .w_full()
        .h(px(height.min(
            size::PALETTE_ROW * self.results.indices.len().max(1) as f32,
        )))
        .track_scroll(&self.scroll);
        let results = div()
            .id("palette-result-list")
            .role(Role::ListBox)
            .aria_label("Search results")
            .aria_size_of_set(self.results.indices.len())
            .map(|list| {
                if self.results.indices.is_empty() {
                    list.child(div().p(px(space::MD)).text_color(theme.text_muted).child(
                        if self.results.pending {
                            "Searching…"
                        } else {
                            "No matches. Try another action name or folder path."
                        },
                    ))
                } else {
                    list.child(results)
                }
            });
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(theme.scrim)
            .flex()
            .items_start()
            .justify_center()
            .p(px(space::LG))
            // Near the top, just under the toolbar's height.
            .pt(px(size::TOOLBAR + space::LG))
            .key_context("Palette")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.emit(PaletteEvent::Closed)),
            )
            .on_action(cx.listener(|this, _: &Previous, _, cx| this.move_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &Next, _, cx| this.move_selection(1, cx)))
            .on_action(cx.listener(|this, _: &Confirm, _, cx| this.confirm(cx)))
            .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(PaletteEvent::Closed)))
            .child(
                div()
                    .id("command-palette")
                    .role(Role::Dialog)
                    .aria_label("Actions")
                    .aria_description("Search actions and folders. Up, Down or Tab chooses a result; Enter runs it; Escape closes.")
                    .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                    .w_full()
                    .max_w(px(size::PALETTE))
                    .flex()
                    .flex_col()
                    .gap(px(space::MD))
                    .p(px(space::MD))
                    .bg(theme.raised)
                    .shadow(vec![theme.floating_shadow()])
                    .rounded(px(size::RADIUS))
                    .border_1()
                    .border_color(theme.border_accent)
                    .text_color(theme.text)
                    .font_family(text::FAMILY_BODY)
                    .text_size(text::scaled(text::BODY))
                    .line_height(text::scaled(text::LINE))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(space::XS))
                            .child(
                                div()
                                    .font_family(text::FAMILY_HEADING)
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .text_size(text::scaled(text::HEADING))
                                    .line_height(text::scaled(text::HEADING_LINE))
                                    .child("Actions"),
                            )
                            .child(
                                div()
                                    .font_family(text::FAMILY_DATA)
                                    .text_size(text::scaled(text::SMALL))
                                    .text_color(theme.text_muted)
                                    .child("Ctrl+Shift+P"),
                            ),
                    )
                    .child(crate::input::named(
                        // The search field sits on the darker content fill.
                        Input::new(&self.input)
                            .min_h(px(size::PALETTE_FIELD))
                            .gap(px(space::SM))
                            .bg(theme.content)
                            .text_size(text::scaled(text::NAME))
                            .prefix(
                                gpui_kit::component::Icon::new(gpui_kit::assets::IconName::Search)
                                    .size(px(size::ICON))
                                    .text_color(theme.text_muted),
                            ),
                        &self.input,
                        "Search actions, places, tabs and recent folders",
                        window,
                        cx,
                    ))
                    .child(results)
                    .child(
                        div()
                            .id("palette-status")
                            .role(Role::Status)
                            .aria_label(result_status)
                            .a11y_synthetic_children(|builder| {
                                builder.parent_node().set_live(gpui_kit::accesskit::Live::Polite);
                            })
                            // Read out, not shown: the palette's description
                            // already says which keys work, and the design
                            // has no footer.
                            .absolute()
                            .size_0()
                            .overflow_hidden(),
                    )
                    .when_some(self.status.clone(), |surface, status| {
                        surface.child(
                            div()
                                .id("palette-notice")
                                .role(Role::Status)
                                .aria_label(status.clone())
                                .text_size(text::scaled(text::SMALL))
                                .text_color(theme.text_muted)
                                .child(status),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use crate::commands;

    #[test]
    fn every_command_with_its_own_icon_is_a_real_command() {
        // A renamed command must not silently lose its icon.
        let labels: Vec<&str> = commands::commands().iter().map(|c| c.label).collect();
        for (command, _) in rustphin_view::palette::COMMAND_ICONS {
            assert!(labels.contains(command), "{command} is not a command");
        }
    }
}
