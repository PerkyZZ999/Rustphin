//! Small adjustments to the component library's controls so they look as
//! docs/DESIGN.md draws them.

use gpui_kit::{
    ElementId, ParentElement as _, SharedString, Styled as _,
    assets::IconName,
    component::{
        Disableable as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
    },
    prelude::FluentBuilder as _,
    px,
};

use crate::theme::{Theme, size};

/// A plain toolbar button (`toolbar-button`): an icon on no fill, the hover
/// fill under the pointer. `on` marks a toggle that is switched on with
/// `toolbar-toggle-on`; screen readers hear it as pressed.
pub fn tool_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    enabled: bool,
    on: Option<bool>,
    theme: &Theme,
) -> Button {
    let label = label.into();
    let lit = on == Some(true);
    Button::new(id)
        .ghost()
        .with_size(px(size::TOOL_BUTTON))
        .size(px(size::TOOL_BUTTON))
        .when(lit, |button| button.bg(theme.current))
        .when_some(on, Button::toggled)
        .child(
            Icon::new(icon)
                .size(px(size::ICON))
                .text_color(if !enabled {
                    theme.text_disabled
                } else if lit {
                    theme.accent
                } else {
                    theme.text
                }),
        )
        .disabled(!enabled)
        .tooltip(label.clone())
        .accessibility_label(label)
}

/// The design's outlined button.
pub trait Outlined {
    /// An outline with no fill of its own until hovered. The library tints
    /// outlined buttons with the field colour; the design leaves them clear,
    /// and the library's hover fill (half the outline colour) is the
    /// design's `hover`.
    fn outlined(self) -> Self;
}

impl Outlined for Button {
    fn outlined(self) -> Self {
        self.outline().bg(gpui_kit::transparent_black())
    }
}

/// The design's destructive button.
pub trait Destructive {
    /// Error text on the error surface inside an error outline. The library
    /// draws its danger button without the outline.
    fn destructive(self, theme: &Theme) -> Self;
}

impl Destructive for Button {
    fn destructive(self, theme: &Theme) -> Self {
        self.danger().border_1().border_color(theme.danger_border)
    }
}
