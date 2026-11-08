//! Design tokens: the single source of colour, spacing, size and radius values.
//! They implement `docs/DESIGN.md`, which is the reference for the design.
//!
//! Views take every visual value from here. Colours are named for their role,
//! not their hue. Spacing follows a 4-pixel grid.

use gpui_kit::{App, BoxShadow, Global, Rgba, point, px, rgb, rgba};

/// Spacing scale in pixels (4-pixel grid).
pub mod space {
    /// Hairline gaps inside a control.
    pub const XS: f32 = 4.;
    /// Gap between an icon and its label; padding inside rows.
    pub const SM: f32 = 8.;
    /// Padding inside bars and banners.
    pub const MD: f32 = 12.;
    /// Padding around the content area.
    pub const LG: f32 = 16.;
}

/// Fixed sizes in pixels.
pub mod size {
    /// Height of the column headings: a 20-pixel text line plus 4 pixels above
    /// and below. Entry rows take their height from the view layout.
    pub const ROW: f32 = 28.;
    /// Height of the toolbar.
    pub const TOOLBAR: f32 = 44.;
    /// Minimum pane width for all three navigation buttons.
    pub const TOOLBAR_FULL_MIN: f32 = 320.;
    /// Minimum pane width for the Back button.
    pub const TOOLBAR_BACK_MIN: f32 = 240.;
    /// Height of the status line.
    pub const STATUS: f32 = 28.;
    /// Height of the filter bar above the status line.
    pub const FILTER_BAR: f32 = 36.;
    /// Height of the tab strip.
    pub const TABS: f32 = 32.;
    /// Icons in toolbars and banners.
    pub const ICON: f32 = 16.;
    /// Sort arrows and other inline indicators.
    pub const ICON_SMALL: f32 = 12.;
    /// Width of the accent bar marking a selected row.
    pub const SELECTION_BAR: f32 = 2.;
    /// Width of the size column.
    pub const COLUMN_SIZE: f32 = 88.;
    /// Minimum pane width that leaves room for the size column beside names.
    pub const WIDTH_FOR_SIZE: f32 = 320.;
    /// Width of the modified column.
    pub const COLUMN_MODIFIED: f32 = 144.;
    /// Width of the type column.
    pub const COLUMN_TYPE: f32 = 184.;
    /// Narrowest window that still shows the modified column.
    pub const WIDTH_FOR_MODIFIED: f32 = 560.;
    /// Narrowest window that still shows the type column.
    pub const WIDTH_FOR_TYPE: f32 = 760.;
    /// Icons in empty and error states.
    pub const ICON_LARGE: f32 = 32.;
    /// Height of a volume's row in the places panel: a name, a usage bar
    /// and a line of numbers.
    pub const DEVICE_ROW: f32 = 48.;
    /// Width of the places panel.
    pub const SIDEBAR: f32 = 220.;
    /// Width of a context menu and of its application list.
    pub const MENU: f32 = 240.;
    /// Width of the label column beside a dialog fact.
    pub const FACT_LABEL: f32 = 72.;
    /// A hairline divider, such as a menu separator.
    pub const HAIRLINE: f32 = 1.;
    /// Tallest a context menu grows before it scrolls: twelve rows.
    pub const MENU_VISIBLE: f32 = 336.;
    /// Width of a dialog.
    pub const DIALOG: f32 = 440.;
    /// Width of the batch-rename dialog, which has a two-column preview.
    pub const RENAME_DIALOG: f32 = 640.;
    /// Maximum width of the command palette.
    pub const PALETTE: f32 = 640.;
    /// Height of a command result, including its second line.
    pub const PALETTE_ROW: f32 = 48.;
    /// Height of a progress bar.
    pub const PROGRESS: f32 = 4.;
    /// Width of a progress bar in the activity line.
    pub const PROGRESS_WIDTH: f32 = 160.;
    /// Corner radius shared by controls, rows and panels.
    pub const RADIUS: f32 = 2.;
}

/// Type scale in pixels.
pub mod text {
    /// Interface and file-name text (`body-md`, `body-sm`).
    pub const FAMILY_BODY: &str = "Inter";
    /// Sizes, dates, paths and counts (`data-md`, `data-sm`).
    pub const FAMILY_DATA: &str = "JetBrains Mono";
    /// Headings in empty and error states (`headline-sm`).
    pub const FAMILY_HEADING: &str = "Space Grotesk";
    /// Data cells (`data-md`).
    pub const DATA: f32 = 13.;
    /// The smallest text: figures under a usage bar.
    pub const TINY: f32 = 11.;
    /// Secondary information: status line, captions.
    pub const SMALL: f32 = 12.;
    /// Body text and list rows.
    pub const BODY: f32 = 14.;
    /// Headings in empty and error states.
    pub const HEADING: f32 = 16.;
    /// Line height for body text.
    pub const LINE: f32 = 20.;
}

/// The colours of the active theme.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Window background.
    pub window: Rgba,
    /// Toolbar and status line, layered over the window background.
    pub chrome: Rgba,
    /// The file list's surface, layered over the window background. More
    /// solid than the chrome so columns of text stay readable.
    pub content: Rgba,
    /// Dialogs and other surfaces that float inside the window. Always solid.
    pub raised: Rgba,
    /// Dims the window behind a dialog.
    pub scrim: Rgba,
    /// Dividers and outlines.
    pub border: Rgba,
    /// Outline of floating surfaces and of the default button in a dialog.
    pub border_accent: Rgba,
    /// Live activity: the filled part of a progress bar.
    pub activity: Rgba,
    /// Primary text.
    pub text: Rgba,
    /// Secondary text and inactive icons.
    pub text_muted: Rgba,
    /// Accent for icons, links and the focus ring on dark surfaces.
    pub accent: Rgba,
    /// Fill of the selected row.
    pub selection: Rgba,
    /// Fill of a primary button under the pointer.
    pub selection_hover: Rgba,
    /// Text on [`Self::selection`].
    pub on_selection: Rgba,
    /// Fill under the pointer.
    pub hover: Rgba,
    /// Text and icons of error messages.
    pub danger: Rgba,
    /// Background of error banners.
    pub danger_surface: Rgba,
}

impl Global for Theme {}

impl Theme {
    /// The dark "obsidian" theme with royal-blue accents.
    ///
    /// With `translucent`, the window background is see-through for compositor
    /// blur; otherwise it is solid.
    pub fn obsidian(translucent: bool) -> Self {
        Self {
            window: if translucent {
                rgba(0x0b0f17b8)
            } else {
                rgb(0x0b0f17)
            },
            chrome: if translucent {
                rgba(0x0b0f1733)
            } else {
                rgb(0x101521)
            },
            content: if translucent {
                rgba(0x0b0f1799)
            } else {
                rgb(0x0b0f17)
            },
            raised: rgb(0x0f172a),
            scrim: rgba(0x04060a99),
            border: rgba(0xffffff1f),
            border_accent: rgba(0x4169e173),
            activity: rgb(0x38bdf8),
            text: rgb(0xdfe2ef),
            text_muted: rgb(0xc3cad9),
            accent: rgb(0x8fa8ff),
            selection: rgb(0x3457d5),
            selection_hover: rgb(0x2f4fc4),
            on_selection: rgb(0xf4f6fb),
            hover: rgba(0xffffff14),
            danger: rgb(0xff9b9b),
            danger_surface: rgb(0x3a1a1f),
        }
    }

    /// The tight accent glow used by solid floating surfaces (D-017).
    pub fn floating_shadow(&self) -> BoxShadow {
        BoxShadow {
            color: self.selection.into(),
            offset: point(px(0.), px(0.)),
            blur_radius: px(space::LG),
            spread_radius: px(-size::RADIUS),
            inset: false,
        }
    }

    /// Makes the component library's own widgets use these tokens.
    pub fn apply_to_components(&self, cx: &mut App) {
        let components = gpui_kit::component::theme::Theme::global_mut(cx);
        components.primary = self.selection.into();
        components.primary_hover = self.selection_hover.into();
        components.primary_active = self.selection.into();
        components.primary_foreground = self.on_selection.into();
        components.button_primary = self.selection.into();
        components.button_primary_hover = self.selection_hover.into();
        components.button_primary_active = self.selection_hover.into();
        components.button_primary_foreground = self.on_selection.into();
        // Text fields are solid, so typing stays readable on a selected row.
        components.background = self.raised.into();
        components.foreground = self.text.into();
        components.input = self.border_accent.into();
        components.caret = self.text.into();
        components.ring = self.accent.into();
        components.radius = px(size::RADIUS);
        components.font_family = text::FAMILY_BODY.into();
        components.mono_font_family = text::FAMILY_DATA.into();
        // Components read their colours from the tokens, which are only
        // derived from the fields above when a whole theme is applied.
        components.tokens = components.colors.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative luminance of an opaque sRGB colour (WCAG 2.x).
    fn luminance(color: Rgba) -> f32 {
        let channel = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn over(top: Rgba, bottom: Rgba) -> Rgba {
        let mix = |t: f32, b: f32| t * top.a + b * (1. - top.a);
        Rgba {
            r: mix(top.r, bottom.r),
            g: mix(top.g, bottom.g),
            b: mix(top.b, bottom.b),
            a: 1.,
        }
    }

    fn contrast(foreground: Rgba, background: Rgba) -> f32 {
        let (a, b) = (luminance(foreground), luminance(background));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Worst case for a translucent window: a white window directly behind it.
    const WHITE: Rgba = Rgba {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };

    fn assert_readable(theme: Theme) {
        let window = over(theme.window, WHITE);
        let chrome = over(theme.chrome, window);
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
        ] {
            let ratio = contrast(foreground, background);
            assert!(
                ratio >= 4.5,
                "{name}: {ratio:.2}:1 is below WCAG AA (4.5:1)"
            );
        }
        // Non-text indicators need 3:1 (WCAG 1.4.11).
        // The selected row is marked by an accent bar as well as its fill.
        let ratio = contrast(theme.accent, window);
        assert!(ratio >= 3., "accent on window: {ratio:.2}:1 is below 3:1");
        // The filled part of a progress bar against its track on the chrome.
        let track = over(theme.hover, chrome);
        let ratio = contrast(theme.activity, track);
        assert!(
            ratio >= 3.,
            "progress on its track: {ratio:.2}:1 is below 3:1"
        );
    }

    #[test]
    fn floating_surfaces_stay_solid_in_the_translucent_theme() {
        // GPUI cannot blur inside the window (D-017), so menus, the palette
        // and dialogs must not let the content behind them show through.
        let glass = Theme::obsidian(true);
        assert!(
            glass.window.a < 1.,
            "the glass window should be translucent"
        );
        assert_eq!(glass.raised.a, 1., "floating surfaces must be opaque");
        assert_eq!(Theme::obsidian(false).raised, glass.raised);
    }

    #[test]
    fn opaque_theme_meets_contrast_minimums() {
        assert_readable(Theme::obsidian(false));
    }

    #[test]
    fn translucent_theme_meets_contrast_minimums_over_a_white_window() {
        assert_readable(Theme::obsidian(true));
    }
}
