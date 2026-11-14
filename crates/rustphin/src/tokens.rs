//! The design tokens as QML sees them: the `Tokens` singleton (W-114c,
//! D-017). Colours and sizes come from `rustphin-view` (`colors`, `metrics`),
//! which implement docs/DESIGN.md.

// cxx-qt generates the unsafe FFI glue between Rust and the C++ QObject
// for the tokens object; nothing in the bridge file is written by hand but
// declarations, and the code here stays under the lint (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "tokens_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString, QVariant};
use rustphin_view::{
    colors::Colors,
    layout::{DEFAULT_ZOOM, ViewMode},
    metrics::{size, space, text},
};

/// The Rust side of `Tokens`.
pub struct TokensRust {
    colors: QMap<QMapPair_QString_QVariant>,
    metrics: QMap<QMapPair_QString_QVariant>,
    translucent: bool,
}

impl Default for TokensRust {
    fn default() -> Self {
        let translucent = glass_wanted(std::env::var_os("RUSTPHIN_OPAQUE").as_deref())
            && qobject::rustphin_blur_available();
        let mut colors = QMap::default();
        for (name, color) in Colors::charcoal(translucent).named() {
            colors.insert(
                QString::from(name),
                QVariant::from(&QString::from(color.qml().as_str())),
            );
        }
        let mut metrics = QMap::default();
        for (name, value) in metric_values() {
            metrics.insert(QString::from(name), QVariant::from(&f64::from(value)));
        }
        for (name, family) in font_families() {
            metrics.insert(QString::from(name), QVariant::from(&QString::from(family)));
        }
        Self {
            colors,
            metrics,
            translucent,
        }
    }
}

impl qobject::Tokens {
    fn blur_windows(&self) {
        if *self.translucent() {
            qobject::rustphin_blur_windows();
        }
    }
}

/// Whether the glass theme may be used: not when `RUSTPHIN_OPAQUE` is set
/// to anything but "0" (AGENTS.md).
fn glass_wanted(opaque: Option<&std::ffi::OsStr>) -> bool {
    opaque.is_none_or(|value| value == "0")
}

/// The sizes QML lays out with, by name: every size of docs/DESIGN.md,
/// and the type scale at the text size Settings chose.
fn metric_values() -> Vec<(&'static str, f32)> {
    let details = ViewMode::Details.layout(DEFAULT_ZOOM);
    let scale = crate::prefs::settings().text_size().scale();
    vec![
        ("space_xs", space::XS),
        ("space_sm", space::SM),
        ("space_md", space::MD),
        ("space_lg", space::LG),
        ("radius", size::RADIUS),
        ("toolbar", size::TOOLBAR),
        ("heading_row", size::ROW),
        ("row", details.row_height),
        ("icon", details.icon),
        ("name_text", details.name_text * scale),
        ("data_text", details.data_text * scale),
        ("body_text", text::BODY * scale),
        ("small_text", text::SMALL * scale),
        ("label_text", text::LABEL * scale),
        ("title_text", text::TITLE * scale),
        ("heading_text", text::HEADING * scale),
        ("text_scale", scale),
        ("line", text::LINE),
        ("hidden_opacity", rustphin_view::colors::HIDDEN_OPACITY),
        ("cell_gap", size::CELL_GAP),
        ("rename_field", size::RENAME_FIELD),
        ("control", size::CONTROL),
        ("status", size::STATUS),
        ("filter_bar", size::FILTER_BAR),
        ("filter_field", size::FILTER_FIELD),
        ("filter_field_max", size::FILTER_FIELD_MAX),
        ("tabs", size::TABS),
        ("tab", size::TAB),
        ("tab_min", size::TAB_MIN),
        ("tab_close_shown", size::TAB_CLOSE_SHOWN),
        ("tab_close", size::TAB_CLOSE),
        ("tab_add", size::TAB_ADD),
        ("icon_small", size::ICON_SMALL),
        ("icon_ui", size::ICON),
        ("icon_large", size::ICON_LARGE),
        ("selection_bar", size::SELECTION_BAR),
        ("column_size", size::COLUMN_SIZE),
        ("column_modified", size::COLUMN_MODIFIED),
        ("column_type", size::COLUMN_TYPE),
        ("device_row", size::DEVICE_ROW),
        ("place_row", size::PLACE_ROW),
        ("place_icon", size::PLACE_ICON),
        ("small_button", size::SMALL_BUTTON),
        ("state_text_max", size::STATE_TEXT_MAX),
        ("check_row", size::CHECK_ROW),
        ("usage", size::USAGE),
        ("sidebar", size::SIDEBAR),
        ("sidebar_min", size::SIDEBAR_MIN),
        ("sidebar_max", size::SIDEBAR_MAX),
        ("sidebar_grip", size::SIDEBAR_GRIP),
        ("toolbar_location_min", size::TOOLBAR_LOCATION_MIN),
        ("menu", size::MENU),
        ("menu_row", size::MENU_ROW),
        ("menu_visible", size::MENU_VISIBLE),
        ("dialog", size::DIALOG),
        ("rename_dialog", size::RENAME_DIALOG),
        ("dialog_title", size::DIALOG_TITLE),
        ("palette", size::PALETTE),
        ("palette_row", size::PALETTE_ROW),
        ("palette_field", size::PALETTE_FIELD),
        ("palette_icon", size::PALETTE_ICON),
        ("progress", size::PROGRESS),
        ("progress_min", size::PROGRESS_MIN),
        ("control_line", size::LINE),
        ("button", size::BUTTON),
        ("properties_label", size::PROPERTIES_LABEL),
        ("properties_thumb_width", size::PROPERTIES_THUMB_WIDTH),
        ("properties_thumb_height", size::PROPERTIES_THUMB_HEIGHT),
        ("icon_header", size::ICON_HEADER),
        ("fact_label", size::FACT_LABEL),
        ("settings_width", size::SETTINGS_WIDTH),
        ("settings_height", size::SETTINGS_HEIGHT),
        ("settings_nav", size::SETTINGS_NAV),
        ("settings_label", size::SETTINGS_LABEL),
        ("settings_narrow", size::SETTINGS_NARROW),
        ("setting_row", size::SETTING_ROW),
        ("setting_control", size::SETTING_CONTROL),
        ("setting_select", size::SETTING_SELECT),
        ("setting_select_short", size::SETTING_SELECT_SHORT),
        ("setting_field", size::SETTING_FIELD),
        ("key_cap", size::KEY_CAP),
        ("about_logo", size::ABOUT_LOGO),
        ("title_bar", size::TITLE_BAR),
        ("window_button", size::WINDOW_BUTTON),
        ("resize_edge", size::RESIZE_EDGE),
        ("tool_separator", size::TOOL_SEPARATOR),
        ("admin_bar", size::ADMIN_BAR),
        ("path_bar", size::PATH_BAR),
        ("tool_button", size::TOOL_BUTTON),
        ("scrollbar", size::SCROLLBAR),
        ("scrollbar_inset", size::SCROLLBAR_INSET),
        ("drag_chip", size::DRAG_CHIP),
        ("hairline", size::HAIRLINE),
    ]
}

/// The type families, by name.
fn font_families() -> [(&'static str, &'static str); 3] {
    [
        ("family_body", text::FAMILY_BODY),
        ("family_data", text::FAMILY_DATA),
        ("family_heading", text::FAMILY_HEADING),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustphin_opaque_turns_the_glass_off_unless_it_is_zero() {
        assert!(glass_wanted(None));
        assert!(glass_wanted(Some(std::ffi::OsStr::new("0"))));
        assert!(!glass_wanted(Some(std::ffi::OsStr::new("1"))));
        assert!(!glass_wanted(Some(std::ffi::OsStr::new(""))));
    }

    #[test]
    fn every_metric_qml_reads_has_a_distinct_name() {
        let mut names: Vec<&str> = metric_values().iter().map(|(name, _)| *name).collect();
        names.extend(font_families().iter().map(|(name, _)| *name));
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn every_colour_the_qml_reads_is_handed_to_it() -> std::io::Result<()> {
        // A misspelt or retired token reads as `undefined` in QML and draws
        // nothing, without a warning at build time.
        let known: Vec<&str> = Colors::charcoal(false)
            .named()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        let qml = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("qml");
        let mut checked = 0;
        for file in std::fs::read_dir(&qml)? {
            let path = file?.path();
            if path.extension().is_none_or(|ext| ext != "qml") {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            for (at, prefix) in source.match_indices("Tokens.colors.") {
                let rest = &source[at + prefix.len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    known.contains(&name.as_str()),
                    "{} reads Tokens.colors.{name}, which is not a colour",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(
            checked > 100,
            "only {checked} colour reads found in {}",
            qml.display()
        );
        Ok(())
    }
}
