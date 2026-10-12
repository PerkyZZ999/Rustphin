//! The GPUI user interface of Rustphin.
//!
//! This crate renders snapshots published by `rustphin-core` and sends it
//! commands. It never reads the filesystem itself.

mod browser;
mod theme;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Bounds, WindowBackgroundAppearance, WindowBounds, WindowOptions, px, size,
};
use rustphin_platform::blur::{BackgroundBlur, NoBlur, WaylandBlur};

use crate::{browser::Browser, theme::Theme};

/// How the application should start.
#[derive(Debug, Clone)]
pub struct Options {
    /// The directory to show first.
    pub start: PathBuf,
    /// Draw opaque surfaces even when the compositor could blur.
    pub force_opaque: bool,
}

/// Runs the application until its last window closes.
pub fn run(options: Options) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            gpui_kit::component::theme::Theme::change(
                gpui_kit::component::theme::ThemeMode::Dark,
                None,
                cx,
            );
            browser::bind_keys(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.), px(320.))),
                window_background: if options.force_opaque {
                    WindowBackgroundAppearance::Opaque
                } else {
                    WindowBackgroundAppearance::Transparent
                },
                app_id: Some("rustphin".into()),
                ..Default::default()
            };
            let opened = gpui_kit::open_window(window_options, cx, |window, cx| {
                let blur: &dyn BackgroundBlur = if options.force_opaque {
                    &NoBlur
                } else {
                    &WaylandBlur
                };
                let translucent = match blur.enable(window, window) {
                    Ok(()) => true,
                    Err(reason) => {
                        tracing::info!(%reason, "using the opaque theme");
                        window.set_background_appearance(WindowBackgroundAppearance::Opaque);
                        false
                    }
                };
                let theme = Theme::obsidian(translucent);
                theme.apply_to_components(cx);
                cx.set_global(theme);
                window.set_window_title("Rustphin");
                cx.new(|cx| Browser::new(options.start.clone(), window, cx))
            });
            match opened {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    tracing::error!(%error, "could not open the main window");
                    cx.quit();
                }
            }
        });
}
