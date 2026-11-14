//! The colour tokens of docs/DESIGN.md (the Obsidian theme), as plain
//! numbers both interfaces draw with (W-114c, D-017). Each toolkit converts
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
    /// drawn in the chrome's colour, reads as joined to the toolbar
    /// (`chrome-raised-*`).
    pub chrome_raised: Color,
    /// The file list's surface, layered over the window background. More
    /// solid than the chrome so columns of text stay readable (`content-*`).
    pub content: Color,
    /// Dialogs and other surfaces that float inside the window. Always solid
    /// (`surface-raised`).
    pub raised: Color,
    /// Dims the window behind a dialog.
    pub scrim: Color,
    /// Dividers between regions (`border`).
    pub border: Color,
    /// Outlines of controls: toolbar buttons, the location field, outlined
    /// buttons, checkboxes (`border-strong`).
    pub border_strong: Color,
    /// Outline of floating surfaces and of the default button in a dialog.
    pub border_accent: Color,
    /// Live activity and capacity: progress and usage bars (`secondary`).
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
    /// Accent for focus rings, selection bars and the active tab and pane on
    /// dark surfaces (`primary-bright`).
    pub accent: Color,
    /// Fill of the selected row (`primary`).
    pub selection: Color,
    /// Fill of a primary button under the pointer.
    pub selection_hover: Color,
    /// Text on [`Self::selection`].
    pub on_selection: Color,
    /// Fill of the current place: a location, quieter than a selection
    /// (`primary-subtle`).
    pub current: Color,
    /// Fill under the pointer.
    pub hover: Color,
    /// Text and icons of error messages.
    pub danger: Color,
    /// Background of error banners.
    pub danger_surface: Color,
    /// Outline of error banners and destructive buttons (`error-border`).
    pub danger_border: Color,
}

impl Colors {
    /// Every colour with its token name in code (`text_muted`), for
    /// interfaces that look colours up by name, such as QML.
    pub fn named(&self) -> [(&'static str, Color); 24] {
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
            ("danger", self.danger),
            ("danger_surface", self.danger_surface),
            ("danger_border", self.danger_border),
        ]
    }

    /// The Obsidian theme: glass (`translucent`) over a blurred desktop, or
    /// solid when blur is unavailable.
    pub fn obsidian(translucent: bool) -> Self {
        let pick = |glass: u32, solid: u32| {
            if translucent {
                Color::rgba(glass)
            } else {
                Color::rgb(solid)
            }
        };
        Self {
            window: pick(0x0d1524cc, 0x0d1524),
            chrome: pick(0x121b2d80, 0x121b2d),
            chrome_raised: pick(0x1a22368c, 0x1a2236),
            content: pick(0x0e17279e, 0x0e1727),
            raised: Color::rgb(0x111a2c),
            scrim: Color::rgba(0x04070e99),
            border: Color::rgba(0xffffff12),
            border_strong: Color::rgba(0xffffff24),
            border_accent: Color::rgba(0x5073eb8c),
            activity: Color::rgb(0x3bcff0),
            track: Color::rgb(0x333d52),
            success: Color::rgb(0x10b981),
            text: Color::rgb(0xe6e9f2),
            text_muted: Color::rgb(0xaeb8cc),
            text_disabled: Color::rgb(0x7c869a),
            accent: Color::rgb(0x8fa8ff),
            selection: Color::rgb(0x2d4cc8),
            selection_hover: Color::rgb(0x2843b4),
            on_selection: Color::rgb(0xf5f7fc),
            current: Color::rgb(0x1d2e6a),
            hover: Color::rgba(0xffffff12),
            danger: Color::rgb(0xff9b9b),
            danger_surface: Color::rgb(0x37202b),
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
            ("text on the current place", theme.text, theme.current),
            (
                "muted text on the current place",
                theme.text_muted,
                theme.current,
            ),
            ("text on an error line", theme.danger, theme.danger_surface),
            ("text on selection", theme.on_selection, theme.selection),
            (
                "text on hovered primary button",
                theme.on_selection,
                theme.selection_hover,
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
        // Non-text indicators need 3:1 (WCAG 1.4.11): selection and focus
        // bars, the active tab's underline, the current place's bar and icon,
        // the filled part of progress and usage bars, the success icon.
        // Disabled text is exempt from 1.4.3; DESIGN.md still keeps it at 3:1
        // so unavailable controls stay legible.
        for (name, foreground, background) in [
            ("accent on window", theme.accent, window),
            ("accent on chrome", theme.accent, chrome),
            ("accent on the tab strip", theme.accent, tab_strip),
            ("accent on content", theme.accent, content),
            ("accent on the current place", theme.accent, theme.current),
            ("progress on its track", theme.activity, theme.track),
            ("success on chrome", theme.success, chrome),
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
        let glass = Colors::obsidian(true);
        assert!(
            glass.window.a < 1.,
            "the glass window should be translucent"
        );
        assert_eq!(glass.raised.a, 1., "floating surfaces must be opaque");
        assert_eq!(Colors::obsidian(false).raised, glass.raised);
    }

    #[test]
    fn opaque_theme_meets_contrast_minimums() {
        assert_readable(Colors::obsidian(false));
    }

    #[test]
    fn translucent_theme_meets_contrast_minimums_over_a_white_window() {
        assert_readable(Colors::obsidian(true));
    }

    #[test]
    fn every_fading_settings_offers_keeps_hidden_names_readable() {
        for percent in (HIDDEN_OPACITY_MIN_PERCENT..=100).step_by(5) {
            let hidden = percent as f32 / 100.;
            assert_readable_with_hidden(Colors::obsidian(false), hidden);
            assert_readable_with_hidden(Colors::obsidian(true), hidden);
        }
    }
}
