//! The design's typefaces at the weights it uses, built into the binary.
//!
//! The pinned GPUI never applies a variable font's `wght` axis: a variable
//! Inter draws every weight as Regular and a variable Space Grotesk as Light,
//! its default. These static faces give the matcher, which picks the face
//! whose weight is nearest to the request, an exact face for each weight in
//! docs/DESIGN.md. They are instances of the fonts' variable releases; see
//! `assets/fonts/README.md`.

use std::borrow::Cow;

use crate::theme::text;

/// Every bundled face, in no particular order.
const FACES: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/SpaceGrotesk-SemiBold.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf"),
];

fn faces() -> Vec<Cow<'static, [u8]>> {
    FACES.into_iter().map(Cow::Borrowed).collect()
}

/// Adds the bundled faces to the text system. Without them text still
/// renders, in whatever the system offers under the same names.
pub(crate) fn register(text_system: &gpui_kit::TextSystem) {
    if let Err(error) = text_system.add_fonts(faces()) {
        tracing::warn!(%error, "could not load the bundled fonts; using the system's");
    }
}

/// The faces the first frame draws with: warmed before the window opens,
/// since each first match walks every installed font (about 90 ms each
/// with 9,000 system fonts).
pub(crate) const FIRST_FRAME: [(&str, gpui_kit::FontWeight); 4] = [
    (text::FAMILY_BODY, gpui_kit::FontWeight::NORMAL),
    (text::FAMILY_BODY, gpui_kit::FontWeight::SEMIBOLD),
    (text::FAMILY_HEADING, gpui_kit::FontWeight::SEMIBOLD),
    (text::FAMILY_DATA, gpui_kit::FontWeight::NORMAL),
];

/// The other faces docs/DESIGN.md uses, plus the medium weight the
/// component library sets on some labels: warmed once the window is up.
pub(crate) const LATER: [(&str, gpui_kit::FontWeight); 2] = [
    (text::FAMILY_BODY, gpui_kit::FontWeight::MEDIUM),
    (text::FAMILY_DATA, gpui_kit::FontWeight::MEDIUM),
];

#[cfg(test)]
mod tests {
    use gpui_kit::PlatformTextSystem as _;
    use gpui_wgpu::CosmicTextSystem;

    use super::faces;

    #[test]
    fn each_design_weight_has_a_face_of_exactly_that_weight()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // No system fonts: what resolves comes from the bundle alone.
        let text_system = CosmicTextSystem::new_without_system_fonts(super::text::FAMILY_BODY);
        text_system.add_fonts(faces())?;
        for (family, weight) in super::FIRST_FRAME.into_iter().chain(super::LATER) {
            let font = gpui_kit::Font {
                weight,
                ..gpui_kit::font(family)
            };
            let id = text_system.font_id(&font)?;
            let (found, _) = text_system.font_weight_and_style(id)?;
            assert_eq!(found, weight, "{family} at {weight:?}");
        }
        Ok(())
    }
}
