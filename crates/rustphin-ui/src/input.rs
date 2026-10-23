//! Accessibility semantics attached to the text editor's actual focus handle.

use gpui_kit::{
    AccessibleAction, App, Div, Entity, EntityInputHandler as _, Focusable as _,
    InteractiveElement as _, ParentElement as _, Role, Stateful, StatefulInteractiveElement as _,
    Styled as _, Window,
    accesskit::ActionData,
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
    let value = window.is_a11y_active().then(|| state.read(cx).value());
    let focus_state = state.clone();
    let edit_state = state.clone();
    div()
        .id(("named-input", state.entity_id()))
        .w_full()
        .track_focus(&focus)
        .role(Role::TextInput)
        .aria_label(label)
        .when_some(value, |this, value| this.aria_value(value))
        .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            focus_state.update(cx, |state, cx| state.focus(window, cx));
        })
        .on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
            let Some(ActionData::Value(value)) = data else {
                return;
            };
            edit_state.update(cx, |state, cx| {
                let end = state.value().encode_utf16().count();
                state.replace_text_in_range(Some(0..end), value, window, cx);
            });
        })
        // The pinned component labels its frame, whose handle is separate
        // from the editor. Keep one semantic node on the editor's handle.
        .child(input.role(None))
}
