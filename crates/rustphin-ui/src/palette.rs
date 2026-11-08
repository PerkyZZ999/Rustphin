//! Search and selection on a solid floating surface. Ranking runs off-thread.

use std::{path::PathBuf, sync::Arc, time::Instant};

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

#[derive(Debug, Clone)]
pub enum Target {
    ClearHistory,
    Command(usize),
    Folder(PathBuf),
    /// A remote folder, opened again through kio-fuse.
    Remote(rustphin_platform::remote::RemoteAddress),
    Tab(usize),
}

#[derive(Clone)]
pub struct Candidate {
    pub label: String,
    pub detail: String,
    pub shortcut: String,
    pub search: String,
    pub target: Target,
}

impl Candidate {
    pub fn new(label: String, detail: String, shortcut: String, target: Target) -> Self {
        let search = format!("{label} {detail}");
        Self {
            label,
            detail,
            shortcut,
            search,
            target,
        }
    }
}

pub enum PaletteEvent {
    Chosen(Target),
    Closed,
}

/// Pure selection state: a query invalidates every previous result immediately.
#[derive(Default)]
struct Results {
    generation: u64,
    indices: Vec<usize>,
    selected: usize,
    pending: bool,
}

impl Results {
    fn begin(&mut self) -> u64 {
        self.generation += 1;
        self.indices.clear();
        self.selected = 0;
        self.pending = true;
        self.generation
    }

    fn complete(&mut self, generation: u64, indices: Vec<usize>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.indices = indices;
        self.pending = false;
        true
    }

    fn move_by(&mut self, delta: isize) {
        if !self.indices.is_empty() {
            self.selected =
                (self.selected as isize + delta).rem_euclid(self.indices.len() as isize) as usize;
        }
    }

    fn chosen(&self) -> Option<usize> {
        if self.pending {
            None
        } else {
            self.indices.get(self.selected).copied()
        }
    }

    fn select(&mut self, generation: u64, row: usize) -> bool {
        if generation != self.generation || self.pending || row >= self.indices.len() {
            return false;
        }
        self.selected = row;
        true
    }
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
                .placeholder("Search actions, places, tabs and recent folders")
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
            .gap(px(space::SM))
            .pr(px(space::SM))
            .rounded(px(size::RADIUS))
            .when(selected, |row| {
                row.bg(theme.selection).text_color(theme.on_selection)
            })
            .hover(|row| {
                row.bg(if selected {
                    theme.selection_hover
                } else {
                    theme.hover
                })
            })
            .child(
                div()
                    .w(px(size::SELECTION_BAR))
                    .h_full()
                    .when(selected, |bar| bar.bg(theme.on_selection)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().truncate().child(candidate.label.clone()))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(text::SMALL))
                            .text_color(if selected {
                                theme.on_selection
                            } else {
                                theme.text_muted
                            })
                            .child(candidate.detail.clone()),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_DATA)
                    .text_size(px(text::SMALL))
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
            .items_center()
            .justify_center()
            .p(px(space::LG))
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
                    .aria_label("Command palette")
                    .aria_description("Search actions and folders. Up, Down or Tab chooses a result; Enter runs it; Escape closes.")
                    .a11y_synthetic_children(|builder| builder.parent_node().set_modal())
                    .w_full()
                    .max_w(px(size::PALETTE))
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .p(px(space::MD))
                    .bg(theme.raised)
                    .shadow(vec![theme.floating_shadow()])
                    .rounded(px(size::RADIUS))
                    .border_1()
                    .border_color(theme.border_accent)
                    .text_color(theme.text)
                    .font_family(text::FAMILY_BODY)
                    .text_size(px(text::BODY))
                    .line_height(px(text::LINE))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .font_family(text::FAMILY_HEADING)
                            .child("Command palette"),
                    )
                    .child(crate::input::named(
                        Input::new(&self.input),
                        &self.input,
                        "Search actions and folders",
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
                            .text_size(px(text::SMALL))
                            .text_color(theme.text_muted)
                            .child("↑ ↓ or Tab to choose · Enter to run · Esc to close"),
                    )
                    .when_some(self.status.clone(), |surface, status| {
                        surface.child(
                            div()
                                .id("palette-notice")
                                .role(Role::Status)
                                .aria_label(status.clone())
                                .text_size(px(text::SMALL))
                                .text_color(theme.text_muted)
                                .child(status),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Results;

    #[test]
    fn a_selection_from_an_old_query_cannot_choose_a_new_result() {
        let mut results = Results::default();
        let old = results.begin();
        assert!(results.complete(old, vec![2, 4]));
        let latest = results.begin();
        assert!(results.complete(latest, vec![7, 9]));
        assert!(!results.select(old, 1));
        assert_eq!(results.chosen(), Some(7));
        assert!(results.select(latest, 1));
        assert_eq!(results.chosen(), Some(9));
    }

    #[test]
    fn pending_or_missing_results_cannot_be_selected() {
        let mut results = Results::default();
        let query = results.begin();
        assert!(!results.select(query, 0));
        assert_eq!(results.chosen(), None);
        assert!(results.complete(query, vec![7, 9]));
        assert!(!results.select(query, 2));
        assert_eq!(results.chosen(), Some(7));
    }

    #[test]
    fn a_late_query_cannot_replace_newer_results() {
        let mut results = Results::default();
        let old = results.begin();
        let latest = results.begin();
        assert!(results.complete(latest, vec![7]));
        assert!(!results.complete(old, vec![3]));
        assert_eq!(results.chosen(), Some(7));
    }

    #[test]
    fn typing_invalidates_the_previous_choice_until_the_new_query_finishes() {
        let mut results = Results::default();
        let first = results.begin();
        assert!(results.complete(first, vec![2, 4]));
        results.move_by(1);
        assert_eq!(results.chosen(), Some(4));
        let next = results.begin();
        assert_eq!(results.chosen(), None);
        assert!(results.complete(next, vec![8, 9]));
        assert_eq!(results.chosen(), Some(8));
    }

    #[test]
    fn navigation_wraps_and_empty_results_never_activate() {
        let mut results = Results::default();
        results.move_by(-1);
        assert_eq!(results.chosen(), None);
        let query = results.begin();
        assert!(results.complete(query, vec![2, 4]));
        results.move_by(-1);
        assert_eq!(results.chosen(), Some(4));
        results.move_by(1);
        assert_eq!(results.chosen(), Some(2));
    }
}
