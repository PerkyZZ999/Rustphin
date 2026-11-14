//! Design tokens: the single source of colour, spacing, size and radius values.
//! They implement `docs/DESIGN.md`, which is the reference for the design.
//!
//! Views take every visual value from here. Colours are named for their role,
//! not their hue. Spacing follows a 4-pixel grid.

use gpui_kit::{App, BoxShadow, Global, Rgba, point, px};

pub use rustphin_view::metrics::space;

pub use rustphin_view::metrics::size;

/// Type scale in pixels (D-026); the sizes live in `rustphin_view::metrics`.
pub mod text {
    pub use rustphin_view::metrics::text::*;

    /// `size`, one of the sizes below, as drawn at the chosen text size.
    pub fn scaled(size: f32) -> gpui_kit::Pixels {
        gpui_kit::px(size * scale())
    }
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
        let c = rustphin_view::colors::Colors::obsidian(translucent);
        let g = |color: rustphin_view::colors::Color| Rgba {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
        Self {
            window: g(c.window),
            chrome: g(c.chrome),
            chrome_raised: g(c.chrome_raised),
            content: g(c.content),
            raised: g(c.raised),
            scrim: g(c.scrim),
            border: g(c.border),
            border_strong: g(c.border_strong),
            border_accent: g(c.border_accent),
            activity: g(c.activity),
            track: g(c.track),
            success: g(c.success),
            text: g(c.text),
            text_muted: g(c.text_muted),
            text_disabled: g(c.text_disabled),
            accent: g(c.accent),
            selection: g(c.selection),
            selection_hover: g(c.selection_hover),
            on_selection: g(c.on_selection),
            current: g(c.current),
            hover: g(c.hover),
            danger: g(c.danger),
            danger_surface: g(c.danger_surface),
            danger_border: g(c.danger_border),
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
        // Selected text in fields is highlighted in `primary`, as a selected row.
        components.selection = self.selection.into();
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

pub use rustphin_view::colors::HIDDEN_OPACITY;

pub use rustphin_view::metrics::HIDDEN_OPACITY_MIN_PERCENT;

static HIDDEN: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(HIDDEN_OPACITY.to_bits());

/// Draws hidden entries at `opacity` from now on (Settings, Appearance).
pub fn set_hidden_opacity(opacity: f32) {
    HIDDEN.store(opacity.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// The opacity hidden entries are drawn at.
pub fn hidden_opacity() -> f32 {
    f32::from_bits(HIDDEN.load(std::sync::atomic::Ordering::Relaxed))
}

/// `top` composited over an opaque `bottom`.
fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let to = |c: Rgba| rustphin_view::colors::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    };
    let mixed = rustphin_view::colors::over(to(top), to(bottom));
    Rgba {
        r: mixed.r,
        g: mixed.g,
        b: mixed.b,
        a: mixed.a,
    }
}
