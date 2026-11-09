//! Small adjustments to the component library's controls so they look as
//! docs/DESIGN.md draws them.

use gpui_kit::{
    Styled as _,
    component::button::{Button, ButtonVariants as _},
};

use crate::theme::Theme;

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
