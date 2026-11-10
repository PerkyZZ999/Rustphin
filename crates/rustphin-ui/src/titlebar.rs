//! The window's own title bar and resize edges (W-096, mockup 35).
//!
//! The window asks the compositor to leave decorations to it. When the
//! compositor agrees (`Decorations::Client`), Rustphin draws a title bar in
//! its own style: the logo, the centred window title, and Minimise,
//! Maximise and Close on the right. Dragging it moves the window, a double
//! click maximises or restores it, and a right click opens the compositor's
//! window menu. Thin bands along the window's edges resize it. When the
//! compositor draws its own frame (`Decorations::Server`) nothing here is
//! drawn.

use std::sync::{Arc, LazyLock};

use gpui_kit::{
    AnyElement, App, CursorStyle, Decorations, Image, ImageFormat, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _,
    ResizeEdge, SharedString, Styled as _, Window,
    assets::IconName,
    component::{Icon, Sizable as _, button::Button, button::ButtonVariants as _},
    div, img, px,
};

use crate::theme::{Theme, size, space, text};

/// The small version of the logo, drawn at 16 pixels.
static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("../../../assets/rustphin-small.svg").to_vec(),
    ))
});

/// Whether the compositor left the window's decorations to Rustphin.
pub fn client_decorated(window: &Window) -> bool {
    matches!(window.window_decorations(), Decorations::Client { .. })
}

/// Set while the left button is down on the title bar, so moving the
/// pointer then hands the drag to the compositor.
struct DragState {
    pressed: bool,
}

/// The title bar, or nothing when the compositor draws its own.
pub fn title_bar(
    title: &str,
    theme: Theme,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if !client_decorated(window) {
        return None;
    }
    let maximized = window.is_maximized();
    let state = window.use_state(cx, |_, _| DragState { pressed: false });
    // A press on a window button is the button's alone. If it reached the
    // bar, the smallest movement before the release would hand the pointer
    // to the compositor as a window move, and the button would never see
    // its release: it stayed pressed and did nothing (W-104).
    let keep_press = |button: Button| {
        div()
            .flex_none()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(button)
    };
    let window_button = |id: &'static str, icon: IconName, label: &'static str| {
        Button::new(id)
            .ghost()
            .with_size(px(size::TITLE_BAR))
            .w(px(size::WINDOW_BUTTON))
            .h(px(size::TITLE_BAR))
            .rounded_none()
            .child(
                Icon::new(icon)
                    .size(px(size::ICON))
                    .text_color(theme.text_muted),
            )
            .tooltip(label)
            .accessibility_label(label)
    };
    let bar = div()
        .id("title-bar")
        .flex()
        .flex_none()
        .items_center()
        .h(px(size::TITLE_BAR))
        .pl(px(space::MD))
        .bg(theme.chrome)
        .border_b_1()
        .border_color(theme.border)
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(&state, |state, event: &MouseDownEvent, window, _| {
                if event.click_count >= 2 {
                    state.pressed = false;
                    window.zoom_window();
                } else {
                    state.pressed = true;
                }
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(&state, |state, _: &MouseUpEvent, _, _| {
                state.pressed = false
            }),
        )
        .on_mouse_move(
            window.listener_for(&state, |state, event: &MouseMoveEvent, window, _| {
                // Only a drag that is still going on moves the window: a
                // release outside the bar never reached it.
                let dragging = event.pressed_button == Some(MouseButton::Left);
                if state.pressed && dragging {
                    window.start_window_move();
                }
                state.pressed = false;
            }),
        )
        .on_mouse_down(MouseButton::Right, |event, window, _| {
            window.show_window_menu(event.position);
        })
        .child(img(LOGO.clone()).size(px(size::ICON)).flex_none())
        // Centred over the whole bar: the left padding matches the buttons
        // on the right.
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(px(size::WINDOW_BUTTON * 3. - size::ICON - space::MD))
                .truncate()
                .text_center()
                .text_size(px(text::BODY))
                .text_color(theme.text_muted)
                .child(SharedString::from(title.to_owned())),
        )
        .child(keep_press(
            window_button("window-minimize", IconName::Minus, "Minimise")
                .on_click(|_, window, _| window.minimize_window()),
        ))
        .child(keep_press(
            window_button(
                "window-maximize",
                if maximized {
                    IconName::Copy
                } else {
                    IconName::Square
                },
                if maximized { "Restore" } else { "Maximise" },
            )
            .on_click(|_, window, _| window.zoom_window()),
        ))
        .child(keep_press(
            window_button("window-close", IconName::X, "Close window")
                .on_click(|_, window, _| window.remove_window()),
        ));
    Some(bar.into_any_element())
}

/// Bands along the window's edges and corners that resize it, laid over
/// everything else. None while maximised or tiled against that edge.
pub fn resize_edges(window: &Window) -> Vec<AnyElement> {
    let Decorations::Client { tiling } = window.window_decorations() else {
        return Vec::new();
    };
    if window.is_maximized() {
        return Vec::new();
    }
    let band = px(size::RESIZE_EDGE);
    let corner = px(size::RESIZE_EDGE * 4.);
    let edge = |id: &'static str, edge: ResizeEdge, cursor: CursorStyle| {
        div().id(id).absolute().cursor(cursor).on_mouse_down(
            MouseButton::Left,
            move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(edge);
            },
        )
    };
    let (top, bottom, left, right) = (!tiling.top, !tiling.bottom, !tiling.left, !tiling.right);
    let mut edges = Vec::new();
    if top {
        edges.push(
            edge("resize-top", ResizeEdge::Top, CursorStyle::ResizeUpDown)
                .top_0()
                .left(corner)
                .right(corner)
                .h(band)
                .into_any_element(),
        );
    }
    if bottom {
        edges.push(
            edge(
                "resize-bottom",
                ResizeEdge::Bottom,
                CursorStyle::ResizeUpDown,
            )
            .bottom_0()
            .left(corner)
            .right(corner)
            .h(band)
            .into_any_element(),
        );
    }
    if left {
        edges.push(
            edge(
                "resize-left",
                ResizeEdge::Left,
                CursorStyle::ResizeLeftRight,
            )
            .left_0()
            .top(corner)
            .bottom(corner)
            .w(band)
            .into_any_element(),
        );
    }
    if right {
        edges.push(
            edge(
                "resize-right",
                ResizeEdge::Right,
                CursorStyle::ResizeLeftRight,
            )
            .right_0()
            .top(corner)
            .bottom(corner)
            .w(band)
            .into_any_element(),
        );
    }
    let corners = [
        (
            top && left,
            "resize-top-left",
            ResizeEdge::TopLeft,
            CursorStyle::ResizeUpLeftDownRight,
        ),
        (
            top && right,
            "resize-top-right",
            ResizeEdge::TopRight,
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            bottom && left,
            "resize-bottom-left",
            ResizeEdge::BottomLeft,
            CursorStyle::ResizeUpRightDownLeft,
        ),
        (
            bottom && right,
            "resize-bottom-right",
            ResizeEdge::BottomRight,
            CursorStyle::ResizeUpLeftDownRight,
        ),
    ];
    for (shown, id, which, cursor) in corners {
        if !shown {
            continue;
        }
        let element = edge(id, which, cursor).size(corner);
        let element = match which {
            ResizeEdge::TopLeft => element.top_0().left_0(),
            ResizeEdge::TopRight => element.top_0().right_0(),
            ResizeEdge::BottomLeft => element.bottom_0().left_0(),
            _ => element.bottom_0().right_0(),
        };
        edges.push(element.into_any_element());
    }
    edges
}
