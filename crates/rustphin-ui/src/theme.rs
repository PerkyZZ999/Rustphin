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
    /// Height of the column headings. Entry rows take their height from the
    /// view layout.
    pub const ROW: f32 = 32.;
    /// The gap between grid cells, split as half a gap around each cell.
    pub const CELL_GAP: f32 = 8.;
    /// Height of the rename field inside a row.
    pub const RENAME_FIELD: f32 = 28.;
    /// The drag chip.
    pub const DRAG_CHIP: f32 = 28.;
    /// Height of the toolbar.
    pub const TOOLBAR: f32 = 52.;
    /// Toolbar buttons, the location field and Actions: 36 pixels square or
    /// tall.
    pub const CONTROL: f32 = 36.;
    /// Minimum pane width for the shortcut hint beside Actions.
    pub const TOOLBAR_HINT_MIN: f32 = 640.;
    /// Minimum pane width for Forward, Up and the labelled Actions button.
    pub const TOOLBAR_FULL_MIN: f32 = 560.;
    /// Minimum pane width for the Back button.
    pub const TOOLBAR_BACK_MIN: f32 = 300.;
    /// Height of the status line.
    pub const STATUS: f32 = 32.;
    /// Narrowest pane whose status line still shows the free space.
    pub const STATUS_FREE_SPACE_MIN: f32 = 600.;
    /// Below this pane width, a status line with a selection shows only it.
    pub const STATUS_SELECTION_ONLY_BELOW: f32 = 440.;
    /// Height of the filter bar above the status line.
    pub const FILTER_BAR: f32 = 40.;
    /// Height of the field in the filter bar, leaving room for its ring.
    pub const FILTER_FIELD: f32 = 32.;
    /// Widest the filter field grows.
    pub const FILTER_FIELD_MAX: f32 = 360.;
    /// Height of the tab strip.
    pub const TABS: f32 = 40.;
    /// Width of a tab.
    pub const TAB: f32 = 168.;
    /// A tab's close button.
    pub const TAB_CLOSE: f32 = 24.;
    /// Width of the new-tab button after the last tab.
    pub const TAB_ADD: f32 = 40.;
    /// Icons in toolbars and banners.
    pub const ICON: f32 = 16.;
    /// Sort arrows and other inline indicators.
    pub const ICON_SMALL: f32 = 12.;
    /// Width of the accent bar marking a selected row.
    pub const SELECTION_BAR: f32 = 2.;
    /// Width of the size column.
    pub const COLUMN_SIZE: f32 = 96.;
    /// Minimum pane width that leaves room for the size column beside names.
    pub const WIDTH_FOR_SIZE: f32 = 360.;
    /// Width of the modified column.
    pub const COLUMN_MODIFIED: f32 = 168.;
    /// Width of the type column.
    pub const COLUMN_TYPE: f32 = 160.;
    /// Narrowest pane that still shows the modified column.
    pub const WIDTH_FOR_MODIFIED: f32 = 600.;
    /// Narrowest pane that still shows the type column.
    pub const WIDTH_FOR_TYPE: f32 = 820.;
    /// Icons in empty and error states.
    pub const ICON_LARGE: f32 = 32.;
    /// Height of a volume's row in the places panel: a name, a usage bar
    /// and a line of numbers.
    pub const DEVICE_ROW: f32 = 56.;
    /// Height of a place row and of a section heading in the places panel.
    pub const PLACE_ROW: f32 = 32.;
    /// Place and device icons in the places panel.
    pub const PLACE_ICON: f32 = 20.;
    /// Small icon buttons: eject on a device row, close in a bar.
    pub const SMALL_BUTTON: f32 = 28.;
    /// Height of a usage bar under a volume's name.
    pub const USAGE: f32 = 4.;
    /// Width of the places panel.
    pub const SIDEBAR: f32 = 240.;
    /// Width of a context menu and of its application list.
    pub const MENU: f32 = 240.;
    /// Width of the label column beside a dialog fact.
    pub const FACT_LABEL: f32 = 72.;
    /// A hairline divider, such as a menu separator.
    pub const HAIRLINE: f32 = 1.;
    /// Tallest a context menu grows before it scrolls: twelve rows.
    pub const MENU_VISIBLE: f32 = 336.;
    /// Width of a dialog.
    pub const DIALOG: f32 = 480.;
    /// Width of the batch-rename dialog, which has a two-column preview.
    pub const RENAME_DIALOG: f32 = 640.;
    /// Maximum width of the command palette.
    pub const PALETTE: f32 = 640.;
    /// Height of a command result, including its second line.
    pub const PALETTE_ROW: f32 = 48.;
    /// Height of a progress bar.
    pub const PROGRESS: f32 = 6.;
    /// Narrowest a progress bar gets in the activity line.
    pub const PROGRESS_MIN: f32 = 80.;
    /// Height of an activity or result line under the panes.
    pub const LINE: f32 = 44.;
    /// Height of a text button in lines and dialogs.
    pub const BUTTON: f32 = 32.;
    /// Height of a dialog's title row.
    pub const DIALOG_TITLE: f32 = 48.;
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
    pub const DATA: f32 = 14.;
    /// Secondary information: status line, captions.
    pub const SMALL: f32 = 12.;
    /// Body text and list rows.
    pub const BODY: f32 = 14.;
    /// File and place names (`body-lg`).
    pub const NAME: f32 = 15.;
    /// Headings of empty, loading and error states (`headline-md`).
    pub const TITLE: f32 = 20.;
    /// Panel and dialog titles (`headline-sm`).
    pub const HEADING: f32 = 17.;
    /// Line height for body text.
    pub const LINE: f32 = 20.;
}

/// The colours of the active theme.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Window background (`window-*`).
    pub window: Rgba,
    /// Toolbar, places panel and status line, layered over the window
    /// background (`chrome-*`).
    pub chrome: Rgba,
    /// The tab strip, one step lighter than the chrome so the active tab,
    /// drawn in the chrome's colour, reads as joined to the toolbar
    /// (`chrome-raised-*`).
    pub chrome_raised: Rgba,
    /// The file list's surface, layered over the window background. More
    /// solid than the chrome so columns of text stay readable (`content-*`).
    pub content: Rgba,
    /// Dialogs and other surfaces that float inside the window. Always solid
    /// (`surface-raised`).
    pub raised: Rgba,
    /// Dims the window behind a dialog.
    pub scrim: Rgba,
    /// Dividers between regions (`border`).
    pub border: Rgba,
    /// Outlines of controls: toolbar buttons, the location field, outlined
    /// buttons, checkboxes (`border-strong`).
    pub border_strong: Rgba,
    /// Outline of floating surfaces and of the default button in a dialog.
    pub border_accent: Rgba,
    /// Live activity and capacity: progress and usage bars (`secondary`).
    pub activity: Rgba,
    /// The empty part of a progress or usage bar (`track`).
    pub track: Rgba,
    /// A completed state, such as a finished operation (`tertiary`).
    pub success: Rgba,
    /// Primary text.
    pub text: Rgba,
    /// Secondary text and inactive icons.
    pub text_muted: Rgba,
    /// Text and icons of disabled controls (`text-disabled`).
    pub text_disabled: Rgba,
    /// Accent for focus rings, selection bars and the active tab and pane on
    /// dark surfaces (`primary-bright`).
    pub accent: Rgba,
    /// Fill of the selected row (`primary`).
    pub selection: Rgba,
    /// Fill of a primary button under the pointer.
    pub selection_hover: Rgba,
    /// Text on [`Self::selection`].
    pub on_selection: Rgba,
    /// Fill of the current place: a location, quieter than a selection
    /// (`primary-subtle`).
    pub current: Rgba,
    /// Fill under the pointer.
    pub hover: Rgba,
    /// Text and icons of error messages.
    pub danger: Rgba,
    /// Background of error banners.
    pub danger_surface: Rgba,
    /// Outline of error banners and destructive buttons (`error-border`).
    pub danger_border: Rgba,
}

impl Global for Theme {}

impl Theme {
    /// The dark navy "obsidian" theme with royal-blue accents.
    ///
    /// With `translucent`, the window, chrome and content are see-through for
    /// compositor blur; otherwise they are solid.
    pub fn obsidian(translucent: bool) -> Self {
        let pick = |glass: u32, solid: u32| {
            if translucent { rgba(glass) } else { rgb(solid) }
        };
        Self {
            window: pick(0x0d1524cc, 0x0d1524),
            chrome: pick(0x121b2d80, 0x121b2d),
            chrome_raised: pick(0x1a22368c, 0x1a2236),
            content: pick(0x0e17279e, 0x0e1727),
            raised: rgb(0x111a2c),
            scrim: rgba(0x04070e99),
            border: rgba(0xffffff12),
            border_strong: rgba(0xffffff24),
            border_accent: rgba(0x5073eb8c),
            activity: rgb(0x3bcff0),
            track: rgb(0x333d52),
            success: rgb(0x10b981),
            text: rgb(0xe6e9f2),
            text_muted: rgb(0xaeb8cc),
            text_disabled: rgb(0x7c869a),
            accent: rgb(0x8fa8ff),
            selection: rgb(0x2d4cc8),
            selection_hover: rgb(0x2843b4),
            on_selection: rgb(0xf5f7fc),
            current: rgb(0x1d2e6a),
            hover: rgba(0xffffff12),
            danger: rgb(0xff9b9b),
            danger_surface: rgb(0x37202b),
            danger_border: rgba(0xff9b9b73),
        }
    }

    /// The tight accent glow used by solid floating surfaces (D-017).
    pub fn floating_shadow(&self) -> BoxShadow {
        BoxShadow {
            // `primary` at about 35 %.
            color: Rgba {
                a: 0.35,
                ..self.selection
            }
            .into(),
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
        // Outlined buttons and fields share the control outline; focus adds
        // the bright ring.
        components.input = self.border_strong.into();
        components.border = self.border_strong.into();
        components.muted_foreground = self.text_muted.into();
        components.caret = self.text.into();
        components.ring = self.accent.into();
        // Destructive buttons: error text on the error surface.
        let danger_hover = over(self.hover, self.danger_surface);
        components.danger = self.danger.into();
        components.button_danger = self.danger_surface.into();
        components.button_danger_hover = danger_hover.into();
        components.button_danger_active = danger_hover.into();
        components.button_danger_foreground = self.danger.into();
        components.scrollbar_thumb = Rgba {
            a: 0.5,
            ..self.text_muted
        }
        .into();
        components.scrollbar_thumb_hover = self.accent.into();
        components.radius = px(size::RADIUS);
        components.font_family = text::FAMILY_BODY.into();
        components.mono_font_family = text::FAMILY_DATA.into();
        // Components read their colours from the tokens, which are only
        // derived from the fields above when a whole theme is applied.
        components.tokens = components.colors.into();
    }
}

/// `top` composited over an opaque `bottom`.
fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let mix = |t: f32, b: f32| t * top.a + b * (1. - top.a);
    Rgba {
        r: mix(top.r, bottom.r),
        g: mix(top.g, bottom.g),
        b: mix(top.b, bottom.b),
        a: 1.,
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
