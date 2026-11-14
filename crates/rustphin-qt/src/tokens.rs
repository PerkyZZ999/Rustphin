//! The design tokens as QML sees them: the `Tokens` singleton (W-114c,
//! D-017). Colours and sizes come from `rustphin-view`, the same values the
//! GPUI interface draws with.

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
        for (name, color) in Colors::obsidian(translucent).named() {
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

/// The sizes QML lays out with, by name.
fn metric_values() -> Vec<(&'static str, f32)> {
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
        ("name_text", details.name_text),
        ("data_text", details.data_text),
        ("body_text", text::BODY),
        ("small_text", text::SMALL),
        ("hidden_opacity", rustphin_view::colors::HIDDEN_OPACITY),
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
}
