//! The colour tokens of docs/DESIGN.md (the charcoal theme, D-038), as
//! plain numbers the interface draws with (W-114c, D-017). Each toolkit converts
//! [`Color`] to its own type. The contrast tests here keep every pairing
//! the design uses at WCAG AA.

/// An sRGB colour with alpha, each channel from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Opacity.
    pub a: f32,
}

impl Color {
    /// An opaque colour from `0xRRGGBB`.
    pub fn rgb(hex: u32) -> Self {
        Self::rgba(hex << 8 | 0xff)
    }

    /// A colour from `0xRRGGBBAA`.
    pub fn rgba(hex: u32) -> Self {
        let channel = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.;
        Self {
            r: channel(24),
            g: channel(16),
            b: channel(8),
            a: channel(0),
        }
    }

    /// The colour as QML writes it: `#AARRGGBB`.
    pub fn qml(self) -> String {
        let byte = |c: f32| (c.clamp(0., 1.) * 255.).round() as u8;
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            byte(self.a),
            byte(self.r),
            byte(self.g),
            byte(self.b)
        )
    }
}

/// `top` composited over an opaque `bottom`.
pub fn over(top: Color, bottom: Color) -> Color {
    let mix = |t: f32, b: f32| t * top.a + b * (1. - top.a);
    Color {
        r: mix(top.r, bottom.r),
        g: mix(top.g, bottom.g),
        b: mix(top.b, bottom.b),
        a: 1.,
    }
}

/// Opacity of a hidden entry (a name starting with a dot), shown faded as in
/// Dolphin so it reads as hidden at a glance; the leading dot says so too.
pub const HIDDEN_OPACITY: f32 = 0.6;

/// The colours of the theme, by role.
#[derive(Debug, Clone, Copy)]
pub struct Colors {
    /// Window background (`window-*`).
    pub window: Color,
    /// Toolbar, places panel and status line, layered over the window
    /// background (`chrome-*`).
    pub chrome: Color,
    /// The tab strip, one step lighter than the chrome so the active tab,
    /// drawn in the content's colour, reads as joined to the list
    /// (`chrome-raised-*`).
    pub chrome_raised: Color,
    /// The file list's surface, layered over the window background. More
    /// solid than the chrome so columns of text stay readable (`content-*`).
    pub content: Color,
    /// Menus, the palette and dialogs, which float inside the window. Always
    /// solid (`surface-raised`).
    pub raised: Color,
    /// Dims the window behind a dialog.
    pub scrim: Color,
    /// Dividers between regions (`border`).
    pub border: Color,
    /// Outlines of controls: toolbar buttons, the location field, outlined
    /// buttons, checkboxes (`border-strong`).
    pub border_strong: Color,
    /// Royal-blue outline of floating surfaces: menus, the palette, dialogs.
    pub border_accent: Color,
    /// Royal-blue halo just outside [`Self::border_accent`].
    pub glow: Color,
    /// The filled part of a progress bar (`progress`).
    pub activity: Color,
    /// The empty part of a progress or usage bar (`track`).
    pub track: Color,
    /// A completed state, such as a finished operation (`tertiary`).
    pub success: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text and inactive icons.
    pub text_muted: Color,
    /// Text and icons of disabled controls (`text-disabled`).
    pub text_disabled: Color,
    /// Royal blue lightened to reach 3:1 on every surface: focus rings, the
    /// focused entry's outline, the selection edge, the active tab's and
    /// pane's line, drop and rubber-band outlines (`primary-bright`).
    pub accent: Color,
    /// Neutral fill of selected entries, the current place, the chosen
    /// result, page or segment, and pressed or checked buttons.
    pub selection: Color,
    /// [`Self::selection`] under the pointer.
    pub selection_hover: Color,
    /// Text and icons on [`Self::selection`].
    pub on_selection: Color,
    /// Quiet fill of the focused entry when it is not selected.
    pub current: Color,
    /// Fill under the pointer.
    pub hover: Color,
    /// Fill of a dialog's default button: royal blue (`primary`).
    pub button_primary: Color,
    /// [`Self::button_primary`] under the pointer or pressed.
    pub button_primary_hover: Color,
    /// Fill of a destructive default button (`destructive`).
    pub button_destructive: Color,
    /// [`Self::button_destructive`] under the pointer or pressed.
    pub button_destructive_hover: Color,
    /// Words and icons on the filled buttons (`on-primary`).
    pub on_button: Color,
    /// Text and icons of error messages.
    pub danger: Color,
    /// Background of error banners.
    pub danger_surface: Color,
    /// Outline of error banners (`error-border`).
    pub danger_border: Color,
}

impl Colors {
    /// Every colour with its token name in code (`text_muted`), for
    /// interfaces that look colours up by name, such as QML.
    pub fn named(&self) -> [(&'static str, Color); 30] {
        [
            ("window", self.window),
            ("chrome", self.chrome),
            ("chrome_raised", self.chrome_raised),
            ("content", self.content),
            ("raised", self.raised),
            ("scrim", self.scrim),
            ("border", self.border),
            ("border_strong", self.border_strong),
            ("border_accent", self.border_accent),
            ("glow", self.glow),
            ("activity", self.activity),
            ("track", self.track),
            ("success", self.success),
            ("text", self.text),
            ("text_muted", self.text_muted),
            ("text_disabled", self.text_disabled),
            ("accent", self.accent),
            ("selection", self.selection),
            ("selection_hover", self.selection_hover),
            ("on_selection", self.on_selection),
            ("current", self.current),
            ("hover", self.hover),
            ("button_primary", self.button_primary),
            ("button_primary_hover", self.button_primary_hover),
            ("button_destructive", self.button_destructive),
            ("button_destructive_hover", self.button_destructive_hover),
            ("on_button", self.on_button),
            ("danger", self.danger),
            ("danger_surface", self.danger_surface),
            ("danger_border", self.danger_border),
        ]
    }

    /// The charcoal theme (D-038): glass (`translucent`) over a blurred
    /// desktop, or solid when blur is unavailable.
    pub fn charcoal(translucent: bool) -> Self {
        let pick = |glass: u32, solid: u32| {
            if translucent {
                Color::rgba(glass)
            } else {
                Color::rgb(solid)
            }
        };
        Self {
            window: pick(0x141516cc, 0x141516),
            chrome: pick(0x18191b80, 0x18191b),
            chrome_raised: pick(0x2021248c, 0x202124),
            content: pick(0x1112139e, 0x111213),
            raised: Color::rgb(0x1d1e21),
            scrim: Color::rgba(0x00000099),
            border: Color::rgba(0xffffff12),
            border_strong: Color::rgba(0xffffff24),
            border_accent: Color::rgba(0x4169e18c),
            glow: Color::rgba(0x4169e133),
            activity: Color::rgb(0x7b98f2),
            track: Color::rgb(0x34363b),
            success: Color::rgb(0x10b981),
            text: Color::rgb(0xececed),
            text_muted: Color::rgb(0xaeb1b8),
            text_disabled: Color::rgb(0x7d8087),
            accent: Color::rgb(0x7b98f2),
            selection: Color::rgb(0x3b3d42),
            selection_hover: Color::rgb(0x45474d),
            on_selection: Color::rgb(0xf5f5f6),
            current: Color::rgb(0x2a2c30),
            hover: Color::rgba(0xffffff0f),
            button_primary: Color::rgb(0x4169e1),
            button_primary_hover: Color::rgb(0x3a5dc8),
            button_destructive: Color::rgb(0xc42f2f),
            button_destructive_hover: Color::rgb(0xab2929),
            on_button: Color::rgb(0xffffff),
            danger: Color::rgb(0xff9b9b),
            danger_surface: Color::rgb(0x3a1f22),
            danger_border: Color::rgba(0xff9b9b73),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::HIDDEN_OPACITY_MIN_PERCENT;

    /// `color` drawn at `opacity`, as an element with that opacity draws it.
    fn faded(color: Color, opacity: f32) -> Color {
        Color {
            a: color.a * opacity,
            ..color
        }
    }

    /// Relative luminance of an opaque sRGB colour (WCAG 2.x).
    fn luminance(color: Color) -> f32 {
        let channel = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn contrast(foreground: Color, background: Color) -> f32 {
        let (a, b) = (luminance(foreground), luminance(background));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Worst case for a translucent window: a white window directly behind it.
    const WHITE: Color = Color {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };

    fn assert_readable(theme: Colors) {
        assert_readable_with_hidden(theme, HIDDEN_OPACITY);
    }

    fn assert_readable_with_hidden(theme: Colors, hidden: f32) {
        let window = over(theme.window, WHITE);
        let chrome = over(theme.chrome, window);
        let tab_strip = over(theme.chrome_raised, window);
        let content = over(theme.content, window);
        let hover = over(theme.hover, content);
        let line = over(theme.chrome_raised, window);
        // A drive card in the places panel (W-148), and under the pointer.
        let card = over(theme.chrome_raised, chrome);
        let card_hover = over(theme.hover, card);
        for (name, foreground, background) in [
            ("text on window", theme.text, window),
            ("text on content", theme.text, content),
            ("muted text on content", theme.text_muted, content),
            (
                "danger text on content",
                theme.danger,
                over(theme.danger_surface, content),
            ),
            ("muted text on window", theme.text_muted, window),
            ("text on chrome", theme.text, chrome),
            ("muted text on chrome", theme.text_muted, chrome),
            ("text on hover", theme.text, hover),
            ("active tab text on the tab strip", theme.text, tab_strip),
            (
                "inactive tab text on the tab strip",
                theme.text_muted,
                tab_strip,
            ),
            (
                "inactive tab text under the pointer",
                theme.text_muted,
                over(theme.hover, tab_strip),
            ),
            (
                "the focused entry's name on its fill",
                theme.text,
                theme.current,
            ),
            (
                "the focused entry's size and date on its fill",
                theme.text_muted,
                theme.current,
            ),
            ("muted text under the pointer", theme.text_muted, hover),
            ("a drive's name on its card", theme.text, card),
            ("a drive's figures on its card", theme.text_muted, card),
            ("a drive's name under the pointer", theme.text, card_hover),
            (
                "a drive's figures under the pointer",
                theme.text_muted,
                card_hover,
            ),
            ("text on an error line", theme.danger, theme.danger_surface),
            ("text on selection", theme.on_selection, theme.selection),
            (
                "text on a selection under the pointer",
                theme.on_selection,
                theme.selection_hover,
            ),
            (
                "words on the default button",
                theme.on_button,
                theme.button_primary,
            ),
            (
                "words on the default button under the pointer",
                theme.on_button,
                theme.button_primary_hover,
            ),
            (
                "words on a destructive button",
                theme.on_button,
                theme.button_destructive,
            ),
            (
                "words on a destructive button under the pointer",
                theme.on_button,
                theme.button_destructive_hover,
            ),
            ("danger on its surface", theme.danger, theme.danger_surface),
            ("text on a dialog", theme.text, theme.raised),
            (
                "text on palette hover",
                theme.text,
                over(theme.hover, theme.raised),
            ),
            (
                "muted text on palette hover",
                theme.text_muted,
                over(theme.hover, theme.raised),
            ),
            ("muted text on a dialog", theme.text_muted, theme.raised),
            ("danger text on a dialog", theme.danger, theme.raised),
            (
                "a hidden entry's name on content",
                over(faded(theme.text, hidden), content),
                content,
            ),
            (
                "a hidden entry's name under the pointer",
                over(faded(theme.text, hidden), hover),
                hover,
            ),
        ] {
            let ratio = contrast(foreground, background);
            assert!(
                ratio >= 4.5,
                "{name}: {ratio:.2}:1 is below WCAG AA (4.5:1)"
            );
        }
        // Non-text indicators need 3:1 (WCAG 1.4.11): focus rings and
        // outlines, the selection edge beside the surface and beside the
        // selection's own fill, the active tab's and pane's line, the filled
        // part of progress and usage bars, the success icon. Disabled text is
        // exempt from 1.4.3; DESIGN.md still keeps it at 3:1 so unavailable
        // controls stay legible.
        for (name, foreground, background) in [
            ("accent on window", theme.accent, window),
            ("accent on chrome", theme.accent, chrome),
            ("accent on the tab strip", theme.accent, tab_strip),
            ("accent on content", theme.accent, content),
            ("focus ring on a dialog", theme.accent, theme.raised),
            (
                "focus ring under the pointer in a dialog",
                theme.accent,
                over(theme.hover, theme.raised),
            ),
            (
                "focus outline on the focused entry",
                theme.accent,
                theme.current,
            ),
            ("focus outline under the pointer", theme.accent, hover),
            (
                "selection edge beside a selection",
                theme.accent,
                theme.selection,
            ),
            (
                "selection edge beside a selection under the pointer",
                theme.accent,
                theme.selection_hover,
            ),
            (
                "focus ring inside the default button",
                theme.on_button,
                theme.button_primary,
            ),
            (
                "focus ring inside a destructive button",
                theme.on_button,
                theme.button_destructive,
            ),
            ("progress on its track", theme.activity, theme.track),
            ("progress on the activity line", theme.activity, line),
            ("usage on its track", theme.activity, theme.track),
            ("focus outline on a drive card", theme.accent, card),
            (
                "focus outline on a drive card under the pointer",
                theme.accent,
                card_hover,
            ),
            ("a nearly full volume's usage", theme.danger, theme.track),
            ("success on chrome", theme.success, chrome),
            ("success on the notice line", theme.success, line),
            ("disabled text on chrome", theme.text_disabled, chrome),
            ("disabled text on content", theme.text_disabled, content),
            (
                "a hidden entry's size and date on content",
                over(faded(theme.text_muted, hidden), content),
                content,
            ),
            (
                "disabled text on a dialog",
                theme.text_disabled,
                theme.raised,
            ),
        ] {
            let ratio = contrast(foreground, background);
            assert!(ratio >= 3., "{name}: {ratio:.2}:1 is below 3:1");
        }
    }

    #[test]
    fn colours_read_as_designed_and_write_as_qml_does() {
        let glass = Color::rgba(0x0d1524cc);
        assert_eq!(glass.qml(), "#cc0d1524");
        assert_eq!(Color::rgb(0xff9b9b).qml(), "#ffff9b9b");
        assert_eq!(Color::rgb(0x2d4cc8).a, 1.);
    }

    #[test]
    fn floating_surfaces_stay_solid_in_the_translucent_theme() {
        // Blur is behind the window only (D-017), so menus, the palette
        // and dialogs must not let the content behind them show through.
        let glass = Colors::charcoal(true);
        assert!(
            glass.window.a < 1.,
            "the glass window should be translucent"
        );
        assert_eq!(glass.raised.a, 1., "floating surfaces must be opaque");
        assert_eq!(Colors::charcoal(false).raised, glass.raised);
    }

    #[test]
    fn only_the_accent_and_status_colours_carry_a_hue() {
        // D-038: surfaces, text and selection are charcoal and grey; royal
        // blue is kept to buttons, rings, edges, glows and progress.
        let hue = |c: Color| c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b);
        for theme in [Colors::charcoal(false), Colors::charcoal(true)] {
            for (name, color) in theme.named() {
                let accent = matches!(
                    name,
                    "border_accent"
                        | "glow"
                        | "activity"
                        | "accent"
                        | "button_primary"
                        | "button_primary_hover"
                        | "button_destructive"
                        | "button_destructive_hover"
                        | "success"
                        | "danger"
                        | "danger_surface"
                        | "danger_border"
                );
                assert_eq!(
                    hue(color) > 12. / 255.,
                    accent,
                    "{name} should {}carry a hue",
                    if accent { "" } else { "not " }
                );
            }
        }
    }

    #[test]
    fn opaque_theme_meets_contrast_minimums() {
        assert_readable(Colors::charcoal(false));
    }

    #[test]
    fn translucent_theme_meets_contrast_minimums_over_a_white_window() {
        assert_readable(Colors::charcoal(true));
    }

    #[test]
    fn every_fading_settings_offers_keeps_hidden_names_readable() {
        for percent in (HIDDEN_OPACITY_MIN_PERCENT..=100).step_by(5) {
            let hidden = percent as f32 / 100.;
            assert_readable_with_hidden(Colors::charcoal(false), hidden);
            assert_readable_with_hidden(Colors::charcoal(true), hidden);
        }
    }
}
