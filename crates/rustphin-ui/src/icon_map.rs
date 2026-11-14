//! GPUI-Kit's drawing of the view's toolkit-free icons (W-114b2).

use gpui_kit::assets::IconName;
use rustphin_view::icons::Icon;

macro_rules! to_gpui {
    ($($name:ident,)*) => {
        /// GPUI-Kit's bundled Lucide icon for `icon`.
        pub fn gpui_icon(icon: Icon) -> IconName {
            match icon {
                $(Icon::$name => IconName::$name,)*
            }
        }
    };
}

rustphin_view::for_each_icon!(to_gpui);
