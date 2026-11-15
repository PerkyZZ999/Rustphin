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

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString, QVariant};
use rustphin_view::{
    colors::Colors,
    layout::{DEFAULT_ZOOM, ViewMode},
    metrics::{size, space, text},
    settings::{TextSize, Theme},
};

/// The Rust side of `Tokens`.
pub struct TokensRust {
    colors: QMap<QMapPair_QString_QVariant>,
    metrics: QMap<QMapPair_QString_QVariant>,
    translucent: bool,
    blur_missing: bool,
    /// The text size `metrics` was made for.
    text_size: TextSize,
}

impl Default for TokensRust {
    fn default() -> Self {
        let theme = theme_now();
        tracing::info!(
            glass = theme.glass,
            blur_missing = theme.blur_missing,
            "theme chosen"
        );
        let text_size = crate::prefs::settings().text_size();
        Self {
            colors: colors_map(theme.glass),
            metrics: metrics_map(text_size),
            translucent: theme.glass,
            blur_missing: theme.blur_missing,
            text_size,
        }
    }
}

/// The theme the settings, the run and the desktop give at this moment.
fn theme_now() -> Theme {
    let forced_opaque = !glass_wanted(std::env::var_os("RUSTPHIN_OPAQUE").as_deref());
    crate::prefs::settings()
        .look()
        .theme(forced_opaque, qobject::rustphin_blur_available())
}

/// Every colour as QML reads it, for the glass or the opaque theme.
fn colors_map(glass: bool) -> QMap<QMapPair_QString_QVariant> {
    let mut colors = QMap::default();
    for (name, color) in Colors::charcoal(glass).named() {
        colors.insert(
            QString::from(name),
            QVariant::from(&QString::from(color.qml().as_str())),
        );
    }
    colors
}

/// Every size and font family as QML reads them, at `text_size`.
fn metrics_map(text_size: TextSize) -> QMap<QMapPair_QString_QVariant> {
    let mut metrics = QMap::default();
    for (name, value) in metric_values(text_size.scale()) {
        metrics.insert(QString::from(name), QVariant::from(&f64::from(value)));
    }
    for (name, family) in font_families() {
        metrics.insert(QString::from(name), QVariant::from(&QString::from(family)));
    }
    metrics
}

impl qobject::Tokens {
    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let text_size = crate::prefs::settings().text_size();
        if text_size != self.text_size {
            self.as_mut().rust_mut().text_size = text_size;
            self.as_mut().rust_mut().metrics = metrics_map(text_size);
            self.as_mut().metrics_changed();
        }
        let theme = theme_now();
        if theme.glass != self.translucent || theme.blur_missing != self.blur_missing {
            tracing::info!(
                glass = theme.glass,
                blur_missing = theme.blur_missing,
                "theme changed"
            );
        }
        if theme.blur_missing != self.blur_missing {
            self.as_mut().rust_mut().blur_missing = theme.blur_missing;
            self.as_mut().blur_missing_changed();
        }
        if theme.glass != self.translucent {
            self.as_mut().rust_mut().translucent = theme.glass;
            self.as_mut().rust_mut().colors = colors_map(theme.glass);
            self.as_mut().colors_changed();
            self.as_mut().translucent_changed();
            qobject::rustphin_blur_windows(theme.glass);
        }
    }

    fn blur_windows(&self) {
        qobject::rustphin_blur_windows(*self.translucent());
    }
}

/// Whether the glass theme may be used: not when `RUSTPHIN_OPAQUE` is set
/// to anything but "0" (AGENTS.md).
fn glass_wanted(opaque: Option<&std::ffi::OsStr>) -> bool {
    opaque.is_none_or(|value| value == "0")
}

/// The sizes QML lays out with, by name: every size of docs/DESIGN.md,
/// and the type scale multiplied by `scale` (the text size Settings chose).
fn metric_values(scale: f32) -> Vec<(&'static str, f32)> {
    let details = ViewMode::Details.layout(DEFAULT_ZOOM);
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
        ("text_line", text::LINE * scale),
        ("heading_line", text::HEADING_LINE * scale),
        ("title_line", text::TITLE_LINE * scale),
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
        ("device_card", size::DEVICE_CARD),
        ("place_row", size::PLACE_ROW),
        ("place_separator", size::PLACE_SEPARATOR),
        ("place_icon", size::PLACE_ICON),
        ("small_button", size::SMALL_BUTTON),
        ("state_text_max", size::STATE_TEXT_MAX),
        ("check_row", size::CHECK_ROW),
        ("usage", size::USAGE),
        ("drive_ring", size::DRIVE_RING),
        ("drive_ring_stroke", size::DRIVE_RING_STROKE),
        ("swatch", size::SWATCH),
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
        ("compress_dialog", size::COMPRESS_DIALOG),
        ("tool_menu_button", size::TOOL_MENU_BUTTON),
        ("dialog_title", size::DIALOG_TITLE),
        ("palette", size::PALETTE),
        ("palette_row", size::PALETTE_ROW),
        ("palette_field", size::PALETTE_FIELD),
        ("palette_icon", size::PALETTE_ICON),
        ("progress", size::PROGRESS),
        ("progress_min", size::PROGRESS_MIN),
        ("blur_status", size::BLUR_STATUS),
        ("speed_graph", size::SPEED_GRAPH),
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

    /// Qt's own checkbox draws in the desktop colour scheme's colours; the
    /// design's is `Check.qml` (DESIGN.md, Checkboxes).
    #[test]
    fn every_checkbox_is_drawn_in_the_design_s_colours() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("qml");
        let mut checked = 0;
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ending| ending != "qml") {
                continue;
            }
            checked += 1;
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if path.file_name().is_some_and(|name| name == "Check.qml") {
                assert!(text.contains("palette.highlight: Tokens.colors.accent"));
                continue;
            }
            assert!(
                !text.contains("CheckBox {"),
                "{} uses Qt's CheckBox instead of Check",
                path.display()
            );
        }
        assert!(checked > 20, "only {checked} QML files found");
    }

    #[test]
    fn rustphin_opaque_turns_the_glass_off_unless_it_is_zero() {
        assert!(glass_wanted(None));
        assert!(glass_wanted(Some(std::ffi::OsStr::new("0"))));
        assert!(!glass_wanted(Some(std::ffi::OsStr::new("1"))));
        assert!(!glass_wanted(Some(std::ffi::OsStr::new(""))));
    }

    #[test]
    fn every_metric_qml_reads_has_a_distinct_name() {
        let mut names: Vec<&str> = metric_values(1.).iter().map(|(name, _)| *name).collect();
        names.extend(font_families().iter().map(|(name, _)| *name));
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    /// Checks that every `Tokens.<group>.<name>` the QML reads is in
    /// `known`: a misspelt or retired token reads as `undefined` in QML and
    /// draws nothing, without a warning at build time.
    fn check_reads(group: &str, known: &[&str], at_least: usize) -> std::io::Result<()> {
        let prefix = format!("Tokens.{group}.");
        let qml = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("qml");
        let mut checked = 0;
        for file in std::fs::read_dir(&qml)? {
            let path = file?.path();
            if path.extension().is_none_or(|ext| ext != "qml") {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            for (at, found) in source.match_indices(&prefix) {
                let rest = &source[at + found.len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    known.contains(&name.as_str()),
                    "{} reads {prefix}{name}, which is not handed to QML",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(
            checked > at_least,
            "only {checked} reads of {prefix} found in {}",
            qml.display()
        );
        Ok(())
    }

    #[test]
    fn the_text_size_scales_the_type_and_its_lines_and_nothing_else() {
        let value = |scale: f32, name: &str| {
            metric_values(scale)
                .into_iter()
                .find(|(known, _)| *known == name)
                .map(|(_, value)| value)
        };
        for name in [
            "body_text",
            "small_text",
            "title_text",
            "text_line",
            "heading_line",
        ] {
            assert_eq!(
                value(1.25, name),
                value(1., name).map(|size| size * 1.25),
                "{name}"
            );
        }
        assert_eq!(value(1.25, "text_scale"), Some(1.25));
        for name in ["row", "icon", "toolbar", "space_md", "dialog"] {
            assert_eq!(value(1.25, name), value(1., name), "{name}");
        }
    }

    #[test]
    fn every_colour_the_qml_reads_is_handed_to_it() -> std::io::Result<()> {
        let known: Vec<&str> = Colors::charcoal(false)
            .named()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        check_reads("colors", &known, 100)
    }

    #[test]
    fn every_size_and_font_the_qml_reads_is_handed_to_it() -> std::io::Result<()> {
        let mut known: Vec<&str> = metric_values(1.).iter().map(|(name, _)| *name).collect();
        known.extend(font_families().iter().map(|(name, _)| *name));
        check_reads("metrics", &known, 300)
    }
}
