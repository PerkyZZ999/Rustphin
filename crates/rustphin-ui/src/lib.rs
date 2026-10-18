//! The GPUI user interface of Rustphin.
//!
//! This crate renders snapshots published by `rustphin-core` and sends it
//! commands. It never reads the filesystem itself.

mod browser;
mod commands;
mod diagnostics;
mod filetypes;
mod format;
mod groups;
mod history;
mod launch;
mod layout;
mod operations;
mod palette;
mod rename_dialog;
mod session;
mod settings;
mod sidebar;
mod tabs;
mod theme;
mod thumbs;
mod workspace;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Bounds, Styled as _, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, px, size,
};
use rustphin_platform::blur::{BackgroundBlur, NoBlur, WaylandBlur};

use crate::{session::Session, settings::Settings, theme::Theme, workspace::Workspace};

pub use launch::launch_targets;

/// How the application should start.
#[derive(Debug, Clone)]
pub struct Options {
    /// Folders to open as tabs. Empty restores the previous session.
    pub starts: Vec<PathBuf>,
    /// The folder to show when there is neither a start folder nor a session.
    pub home: PathBuf,
    /// Draw opaque surfaces even when the compositor could blur.
    pub force_opaque: bool,
}

/// Runs the application until its last window closes.
pub fn run(options: Options) {
    // A small file read once before the window exists.
    let settings_path =
        rustphin_platform::config::config_dir().map(|dir| dir.join("settings.toml"));
    let settings = settings_path
        .as_deref()
        .and_then(|path| match rustphin_platform::config::read(path) {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "could not read the settings");
                None
            }
        })
        .map(|text| Settings::parse(&text))
        .unwrap_or_default();
    let session_path = rustphin_platform::config::state_dir().map(|dir| dir.join("session.toml"));
    // Explicit folders replace the remembered tabs for this run; they are
    // still overwritten by what is open when the tabs next change.
    let session = if options.starts.is_empty() {
        session_path
            .as_deref()
            .and_then(|path| rustphin_platform::config::read(path).ok().flatten())
            .map(|text| Session::parse(&text))
            .unwrap_or_default()
    } else {
        Session::folders(options.starts.clone())
    };
    gpui_kit::application()
        // The complete icon set: the default bundle lacks the view-mode icons.
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            gpui_kit::component::theme::Theme::change(
                gpui_kit::component::theme::ThemeMode::Dark,
                None,
                cx,
            );
            operations::init(cx);
            history::init(cx);
            if let Some(service) = rustphin_platform::file_manager::ServiceHandle::session() {
                tracing::info!("listening for requests to show files");
                cx.set_global(workspace::ShowService(service));
            } else {
                tracing::info!("not listening for requests to show files");
            }
            palette::bind_keys(cx);
            thumbs::init(cx);
            browser::bind_keys(cx);
            workspace::bind_keys(cx);
            sidebar::bind_keys(cx);
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
            let opened = cx.open_window(window_options, |window, cx| {
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
                let workspace = cx.new(|cx| {
                    Workspace::new(
                        session.clone(),
                        options.home.clone(),
                        session_path.clone(),
                        settings.clone(),
                        settings_path.clone(),
                        window,
                        cx,
                    )
                });
                // The component library's root paints its own opaque background
                // under the view. Clear it: the browser paints the window
                // background itself, translucent when the compositor blurs.
                cx.new(|cx| {
                    gpui_kit::base::Root::new(workspace, window, cx)
                        .bg(gpui_kit::transparent_black())
                })
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
