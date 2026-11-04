//! Accessibility semantics attached to the text editor's actual focus handle.

use std::{cell::Cell, ops::Range, rc::Rc};

use gpui_kit::{
    AccessibleAction, App, Div, Entity, Focusable as _, InteractiveElement as _,
    ParentElement as _, Role, Stateful, StatefulInteractiveElement as _, Styled as _, Window,
    accesskit::{ActionData, Node, NodeId, TextPosition, TextSelection},
    component::input::{Input, InputState},
    div,
    prelude::FluentBuilder as _,
};

pub(crate) fn named(
    input: Input,
    state: &Entity<InputState>,
    label: &'static str,
    window: &Window,
    cx: &App,
) -> Stateful<Div> {
    let focus = state.focus_handle(cx);
    let text = window.is_a11y_active().then(|| {
        let state = state.read(cx);
        (state.value(), state.selected_range(), state.cursor())
    });
    let run_id: Rc<Cell<Option<NodeId>>> = Rc::new(Cell::new(None));
    let selection_run = run_id.clone();
    let focus_state = state.clone();
    let edit_state = state.clone();
    let selection_state = state.clone();
    div()
        .id(("named-input", state.entity_id()))
        .w_full()
        .track_focus(&focus)
        .role(Role::TextInput)
        .aria_label(label)
        .when_some(text, |this, (text, range, cursor)| {
            this.aria_value(text.clone())
                .a11y_synthetic_children(move |builder| {
                    let id = builder.synthetic_node_id("input-text");
                    let mut node = Node::new(Role::TextRun);
                    node.set_value(text.to_string());
                    node.set_character_lengths(
                        text.chars()
                            .map(|ch| ch.len_utf8() as u8)
                            .collect::<Vec<_>>(),
                    );
                    if !builder.push_child(id, node) {
                        tracing::warn!("could not expose the input text run");
                        return;
                    }
                    run_id.set(Some(id));
                    if let Some((anchor, focus)) = selection_positions(&text, range.clone(), cursor)
                    {
                        builder.parent_node().set_text_selection(TextSelection {
                            anchor: TextPosition {
                                node: id,
                                character_index: anchor,
                            },
                            focus: TextPosition {
                                node: id,
                                character_index: focus,
                            },
                        });
                    }
                })
        })
        .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            focus_state.update(cx, |state, cx| state.focus(window, cx));
        })
        .on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
            let Some(ActionData::Value(value)) = data else {
                return;
            };
            edit_state.update(cx, |state, cx| {
                state.replace_all(value.to_string(), window, cx)
            });
        })
        .on_a11y_action(
            AccessibleAction::SetTextSelection,
            move |data, window, cx| {
                let Some(ActionData::SetTextSelection(selection)) = data else {
                    return;
                };
                if selection_run.get() != Some(selection.anchor.node)
                    || selection_run.get() != Some(selection.focus.node)
                {
                    return;
                }
                selection_state.update(cx, |state, cx| {
                    let text = state.value();
                    let Some(anchor) = byte_offset(&text, selection.anchor.character_index) else {
                        return;
                    };
                    let Some(focus) = byte_offset(&text, selection.focus.character_index) else {
                        return;
                    };
                    state.set_selected_range(anchor.min(focus)..anchor.max(focus), cx);
                    state.focus(window, cx);
                });
            },
        )
        // The pinned component labels its frame, whose handle is separate
        // from the editor. Keep one semantic node on the editor's handle.
        .child(input.role(None))
}

fn byte_offset(text: &str, character: usize) -> Option<usize> {
    text.char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .nth(character)
}

fn character_offset(text: &str, byte: usize) -> Option<usize> {
    text.get(..byte).map(|prefix| prefix.chars().count())
}

fn selection_positions(text: &str, range: Range<usize>, cursor: usize) -> Option<(usize, usize)> {
    if range.is_empty() {
        let caret = character_offset(text, cursor)?;
        return Some((caret, caret));
    }
    let start = character_offset(text, range.start)?;
    let end = character_offset(text, range.end)?;
    Some(if cursor == range.start {
        (end, start)
    } else {
        (start, end)
    })
}

#[cfg(test)]
mod tests {
    use super::{byte_offset, character_offset, selection_positions};

    #[test]
    fn native_character_positions_round_trip_multibyte_text() {
        let text = "aé🦀e\u{301}";
        assert_eq!(byte_offset(text, 2), Some(3));
        assert_eq!(byte_offset(text, 3), Some(7));
        assert_eq!(byte_offset(text, 5), Some(10));
        assert_eq!(character_offset(text, 7), Some(3));
        assert_eq!(character_offset(text, 10), Some(5));
        assert_eq!(character_offset(text, 2), None);
        assert_eq!(byte_offset(text, 6), None);
    }

    #[test]
    fn selection_direction_preserves_the_active_caret() {
        assert_eq!(selection_positions("aé🦀", 1..7, 1), Some((3, 1)));
        assert_eq!(selection_positions("aé🦀", 1..7, 7), Some((1, 3)));
        assert_eq!(selection_positions("aé🦀", 3..3, 3), Some((2, 2)));
    }

    #[test]
    fn empty_text_has_a_caret_and_invalid_byte_positions_are_rejected() {
        assert_eq!(selection_positions("", 0..0, 0), Some((0, 0)));
        assert_eq!(byte_offset("", 0), Some(0));
        assert_eq!(selection_positions("é", 0..1, 1), None);
        assert_eq!(selection_positions("é", 0..0, 3), None);
        assert_eq!(byte_offset("", 1), None);
    }
}
