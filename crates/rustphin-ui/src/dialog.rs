//! The parts every dialog shares (docs/DESIGN.md, Dialogs): a dimmed
//! window, a solid outlined panel, a 48-pixel title row with an icon, the
//! title and a close button, a scrolling body, and a right-aligned action row
//! that wraps.

use gpui_kit::{
    AnyElement, ClickEvent, InteractiveElement as _, IntoElement, ParentElement as _, Pixels,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, Window,
    component::{
        Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
    },
    div, px,
};

use crate::theme::{Theme, size, space, text};

/// The dimmed window with `panel` centred in it, at least 16 pixels from
/// every edge.
pub fn scrim(panel: impl IntoElement, theme: Theme) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .occlude()
        .flex()
        .items_center()
        .justify_center()
        .p(px(space::LG))
        .bg(theme.scrim)
        .child(panel)
        .into_any_element()
}

/// The panel itself: solid, outlined, with the accent glow. `width` is the
/// widest it grows; it never outgrows the window.
pub fn panel(
    panel: Stateful<gpui_kit::Div>,
    width: f32,
    theme: Theme,
    window: &Window,
) -> Stateful<gpui_kit::Div> {
    panel
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(width))
        .max_h(max_height(window))
        .rounded(px(size::RADIUS))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border_accent)
        .shadow(vec![theme.floating_shadow()])
        .text_color(theme.text)
        .font_family(text::FAMILY_BODY)
        .text_size(px(text::BODY))
        .line_height(px(text::LINE))
}

/// The tallest a panel may be: the window less the 16-pixel margins.
pub fn max_height(window: &Window) -> Pixels {
    px((f32::from(window.viewport_size().height) - space::LG * 2.).max(0.))
}

/// The title row: an icon, the title (the question or the fact), and a
/// close button that does what Escape does.
pub fn title_row(
    icon: gpui_kit::assets::IconName,
    title: impl Into<SharedString>,
    close_label: &'static str,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
    theme: Theme,
) -> gpui_kit::Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(space::MD))
        .h(px(size::DIALOG_TITLE))
        .pl(px(space::LG))
        .pr(px(space::SM))
        .border_b_1()
        .border_color(theme.border)
        .child(
            Icon::new(icon)
                .size(px(size::ICON))
                .flex_none()
                .text_color(theme.text_muted),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(text::FAMILY_HEADING)
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_size(px(text::HEADING))
                .line_height(px(text::HEADING_LINE))
                .child(title.into()),
        )
        .child(
            Button::new("dialog-close")
                .ghost()
                .with_size(px(size::SMALL_BUTTON))
                .size(px(size::SMALL_BUTTON))
                .child(Icon::new(gpui_kit::component::IconName::Close).size(px(size::ICON)))
                .tooltip(close_label)
                .accessibility_label(close_label)
                .on_click(on_close),
        )
}

/// The body between title and actions, with the 16-pixel inset.
pub fn body() -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .flex_shrink_1()
        .min_h_0()
        .p(px(space::LG))
}

/// The action row: right-aligned, wrapping when the panel is narrow.
pub fn actions(theme: Theme) -> gpui_kit::Div {
    div()
        .flex()
        .flex_none()
        .flex_wrap()
        .justify_end()
        .gap(px(space::SM))
        .px(px(space::LG))
        .pt(px(space::MD))
        .pb(px(space::LG))
        .border_t_1()
        .border_color(theme.border)
}

/// A 32-pixel text button whose label shows its key ("Close (Esc)"). The
/// caller picks the variant: primary for the safe default, danger for a
/// destructive one, outline for the others.
pub fn button(id: &'static str, label: impl Into<SharedString>) -> Button {
    let label = label.into();
    Button::new(id)
        // The library pads a custom-sized button by a fifth of the size:
        // 80 gives the 16-pixel padding.
        .with_size(px(space::LG * 5.))
        .h(px(size::BUTTON))
        .accessibility_label(label.clone())
        .child(div().text_size(px(text::BODY)).child(label))
}

/// A dialog button that leads with the action's 16-pixel icon, 8 pixels
/// before its words (W-097). The icon repeats the words, so the button's
/// accessible name stays the words alone.
pub fn button_with_icon(
    id: &'static str,
    icon: gpui_kit::assets::IconName,
    label: impl Into<SharedString>,
) -> Button {
    let label = label.into();
    Button::new(id)
        .with_size(px(space::LG * 5.))
        .h(px(size::BUTTON))
        .accessibility_label(label.clone())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(space::SM))
                .text_size(px(text::BODY))
                .child(Icon::new(icon).size(px(size::ICON)).flex_none())
                .child(label),
        )
}

/// The fixed label column beside a fact in a question or Properties.
pub fn fact(
    id: &'static str,
    label: &'static str,
    label_width: f32,
    value: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<gpui_kit::Div> {
    let value = value.into();
    div()
        .id(id)
        .role(gpui_kit::Role::Label)
        .aria_value(format!("{label}: {value}"))
        .flex()
        .gap(px(space::SM))
        .child(
            div()
                .w(px(label_width))
                .flex_none()
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .font_family(text::FAMILY_DATA)
                .text_size(px(text::DATA))
                .child(value),
        )
}
