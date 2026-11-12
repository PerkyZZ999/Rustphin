//! How a context menu looks and which keys move in it, shared by the file
//! view's menu and the places panel's (DESIGN.md "Context menu"). What a
//! menu offers is decided in `menu`, away from drawing.

use gpui_kit::{
    App, InteractiveElement as _, IntoElement as _, KeyBinding, ParentElement as _, Role,
    SharedString, StatefulInteractiveElement as _, Styled as _, actions,
    component::{Icon, IconName},
    div, img,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    menu::RowIcon,
    theme::{Theme, size, space, text},
};

/// A context menu while it has the keyboard focus.
pub(crate) const CONTEXT: &str = "ContextMenu";

actions!(
    menu,
    [
        /// The previous row.
        MenuUp,
        /// The next row.
        MenuDown,
        /// Back from a second menu.
        MenuLeft,
        /// Into a second menu.
        MenuRight,
        /// Run the row.
        MenuAccept,
        /// Close the menu.
        MenuClose,
    ]
);

/// The keys of every context menu.
pub fn bind_keys(cx: &mut App) {
    let menu = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", MenuUp, menu),
        KeyBinding::new("down", MenuDown, menu),
        KeyBinding::new("left", MenuLeft, menu),
        KeyBinding::new("right", MenuRight, menu),
        KeyBinding::new("enter", MenuAccept, menu),
        KeyBinding::new("escape", MenuClose, menu),
    ]);
}

/// One column of a menu, without its rows: 240 px, 4 px padding, the
/// raised surface with its accent border and shadow.
pub(crate) fn column(id: &'static str, theme: Theme) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .w(px(size::MENU))
        .overflow_y_scroll()
        .p(px(space::XS))
        .rounded(px(size::RADIUS))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border_accent)
        .shadow(vec![theme.floating_shadow()])
        .font_family(text::FAMILY_BODY)
        .text_size(text::scaled(text::BODY))
}

/// How far down a second menu starts so its first row lines up with the
/// row that opened it, the `cursor`-th of `entries`.
pub(crate) fn submenu_offset<C>(entries: &[crate::menu::MenuEntry<C>], cursor: usize) -> f32 {
    crate::menu::column_height(
        &entries[..cursor.min(entries.len())],
        size::MENU_ROW,
        size::HAIRLINE + 2. * space::XS,
        0.,
    )
}

/// One row of a menu: icon, words, the key that does the same, and a
/// chevron when it opens a second menu. `current` is the row under the
/// pointer or keyboard.
pub(crate) fn row(
    label: &str,
    icon: RowIcon,
    key: Option<&'static str>,
    enabled: bool,
    submenu: bool,
    current: bool,
    theme: Theme,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let quiet = if current {
        theme.on_selection
    } else if enabled {
        theme.text_muted
    } else {
        theme.text_disabled
    };
    div()
        .id(SharedString::from(format!(
            "menu-{}-{label}",
            if submenu { "sub" } else { "item" },
        )))
        .role(Role::MenuItem)
        .aria_label(label.to_owned())
        .when(!enabled, |row| {
            row.aria_description("Unavailable.")
                .a11y_synthetic_children(|builder| builder.parent_node().set_disabled())
        })
        .when(submenu, |row| {
            row.a11y_synthetic_children(|builder| {
                builder
                    .parent_node()
                    .set_has_popup(gpui_kit::accesskit::HasPopup::Menu)
            })
        })
        .flex()
        .flex_none()
        .items_center()
        .gap(px(space::MD))
        .h(px(size::MENU_ROW))
        .px(px(space::SM - size::SELECTION_BAR))
        .rounded(px(size::RADIUS))
        .border_l(px(size::SELECTION_BAR))
        .border_color(gpui_kit::transparent_black())
        .when(current, |row| row.border_color(theme.accent))
        .when(current, |row| {
            row.bg(theme.selection).text_color(theme.on_selection)
        })
        .when(!current && enabled, |row| row.text_color(theme.text))
        .when(!enabled, |row| row.text_color(theme.text_disabled))
        .child(match icon {
            RowIcon::Symbol(icon) => Icon::new(icon)
                .size(px(size::ICON))
                .flex_none()
                .into_any_element(),
            // A file the renderer cannot draw falls back to the outline.
            RowIcon::Image(path) => {
                use gpui_kit::StyledImage as _;
                img(path)
                    .size(px(size::ICON))
                    .flex_none()
                    .with_fallback(|| {
                        Icon::new(gpui_kit::assets::IconName::AppWindow)
                            .size(px(size::ICON))
                            .into_any_element()
                    })
                    .into_any_element()
            }
        })
        .child(div().flex_1().min_w_0().truncate().child(label.to_owned()))
        .when_some(key, |row, key| {
            row.child(
                div()
                    .flex_none()
                    .font_family(text::FAMILY_DATA)
                    .text_size(text::scaled(text::SMALL))
                    .text_color(quiet)
                    .child(key),
            )
        })
        .when(submenu, |row| {
            row.child(
                Icon::new(IconName::ChevronRight)
                    .size(px(size::ICON_SMALL))
                    .text_color(if current {
                        theme.on_selection
                    } else {
                        theme.text_muted
                    }),
            )
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{MenuCommand, MenuEntry};

    #[test]
    fn a_second_menu_starts_level_with_the_row_that_opened_it() {
        let item = || MenuEntry::Item {
            label: "x".to_owned(),
            command: MenuCommand::Open,
            enabled: true,
        };
        let entries = vec![
            item(),
            item(),
            MenuEntry::Separator,
            MenuEntry::Submenu {
                label: "Open with".to_owned(),
                items: vec![item()],
            },
        ];
        assert_eq!(submenu_offset(&entries, 0), 0.);
        assert_eq!(
            submenu_offset(&entries, 3),
            2. * size::MENU_ROW + size::HAIRLINE + 2. * space::XS
        );
    }
}
