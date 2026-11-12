//! The GPUI user interface of Rustphin.
//!
//! This crate renders snapshots published by `rustphin-core` and sends it
//! commands. It never reads the filesystem itself.

mod browser;
mod commands;
mod controls;
mod diagnostics;
mod dialog;
mod filetypes;
mod folder_views;
mod fonts;
mod format;
mod groups;
mod history;
mod input;
mod launch;
mod layout;
mod location;
mod menu;
mod menu_view;
mod operations;
mod palette;
mod place_menu;
mod preferences;
mod properties;
mod recovery;
mod remote;
mod rename_dialog;
mod session;
mod settings;
mod settings_appearance;
mod settings_confirmations;
mod settings_controls;
mod settings_dialog;
mod settings_general;
mod settings_places;
mod settings_shortcuts;
mod settings_toolbar;
mod settings_views;
mod sidebar;
mod tabs;
mod theme;
mod thumbs;
mod titlebar;
mod toolbar_items;
mod workspace;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Bounds, Styled as _, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, px, size,
};

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
    let settings_text =
        settings_path
            .as_deref()
            .and_then(|path| match rustphin_platform::config::read(path) {
                Ok(text) => text,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "could not read the settings");
                    None
                }
            });
    let settings_saved = settings_text.is_some();
    let kde_single_click =
        rustphin_platform::kdeglobals::read(rustphin_platform::xdg::config_home().as_deref())
            .is_some_and(|text| rustphin_platform::kdeglobals::single_click(&text));
    let settings = settings_text
        .map(|text| Settings::parse(&text))
        .unwrap_or_default();
    // Each folder's own view (W-106c2), a small file beside the session.
    let views_path = rustphin_platform::config::state_dir().map(|dir| dir.join("views.toml"));
    let folder_views = views_path
        .as_deref()
        .and_then(|path| rustphin_platform::config::read(path).ok().flatten())
        .map(|text| folder_views::FolderViews::parse(&text))
        .unwrap_or_default();
    let session_path = rustphin_platform::config::state_dir().map(|dir| dir.join("session.toml"));
    let saved_session = session_path
        .as_deref()
        .map_or(Ok(None), rustphin_platform::config::read);
    let starts = start_folders(&options, &settings);
    let restored = session::restore(saved_session, &starts);
    let session_path = session_path.filter(|_| restored.can_save());
    let session = restored.session;
    let session_issue = restored.issue;
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
            cx.set_global(WindowDefaults {
                home: options.home.clone(),
                force_opaque: options.force_opaque,
            });
            // One store of settings for every window (D-029).
            preferences::init(
                preferences::Loaded {
                    settings: settings.clone(),
                    path: settings_path.clone(),
                    saved: settings_saved,
                    kde_single_click,
                    folder_views,
                    folder_path: views_path.clone(),
                },
                cx,
            );
            if let Some(issue) = session_issue {
                tracing::warn!(%issue, "saved session is protected for this run");
                operations::hub(cx).update(cx, |hub, cx| {
                    hub.say(issue.notice().to_owned(), true, cx);
                });
            }
            history::init(cx);
            remote::init(cx);
            // D-020: restore batch renames an earlier run left half done.
            recovery::start(&session, cx);
            if let Some(service) = rustphin_platform::file_manager::ServiceHandle::session() {
                tracing::info!("listening for requests to show files");
                cx.set_global(workspace::ShowService(service));
            } else {
                tracing::info!("not listening for requests to show files");
            }
            palette::bind_keys(cx);
            settings_dialog::bind_keys(cx);
            thumbs::init(cx);
            browser::bind_keys(cx);
            menu_view::bind_keys(cx);
            workspace::bind_keys(cx);
            sidebar::bind_keys(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            // Font matching can scan the installed font collection. Finish
            // that work off-thread before the first view needs its fonts.
            let text_system = cx.text_system().clone();
            let warming = cx.background_executor().spawn(async move {
                fonts::register(&text_system);
                warm_fonts(&text_system);
            });
            cx.spawn(async move |cx| {
                warming.await;
                cx.update(|cx| {
                    open_window(&options, settings, session, session_path, Vec::new(), cx);
                });
            })
            .detach();
        });
}

/// The folders to open at start: those asked for, else what the General
/// settings choose (W-106c). None means the last session's tabs.
fn start_folders(options: &Options, settings: &Settings) -> Vec<PathBuf> {
    if !options.starts.is_empty() {
        return options.starts.clone();
    }
    match settings.start() {
        (settings::Start::Restore, _) => Vec::new(),
        (settings::Start::Home, _) => vec![options.home.clone()],
        (settings::Start::Folder, folder) => {
            match folder.and_then(location::Location::from_saved) {
                Some(location::Location::Folder(path)) => vec![path],
                // A remote or missing choice starts at home.
                _ => vec![options.home.clone()],
            }
        }
    }
}

/// What every window opens with, kept for windows opened later.
struct WindowDefaults {
    home: PathBuf,
    force_opaque: bool,
}

impl gpui_kit::Global for WindowDefaults {}

/// A window of its own for folders another application asked to show
/// (General, "In a new window"). It is not the saved session: its tabs are
/// not restored next time.
pub(crate) fn open_window_showing(
    shown: Vec<rustphin_platform::file_manager::Shown>,
    cx: &mut App,
) {
    let Some(first) = shown.first().map(|shown| shown.folder.clone()) else {
        return;
    };
    let Some(defaults) = cx.try_global::<WindowDefaults>() else {
        return;
    };
    let options = Options {
        starts: Vec::new(),
        home: defaults.home.clone(),
        force_opaque: defaults.force_opaque,
    };
    let settings = preferences::preferences(cx).read(cx).settings().clone();
    open_window(
        &options,
        settings,
        Session::folders(vec![first]),
        None,
        shown,
        cx,
    );
}

fn open_window(
    options: &Options,
    settings: Settings,
    session: Session,
    session_path: Option<PathBuf>,
    shown: Vec<rustphin_platform::file_manager::Shown>,
    cx: &mut App,
) {
    let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
    let window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(480.), px(320.))),
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: Some("rustphin".into()),
        // Rustphin draws its own title bar (W-096) unless Settings asks for
        // KDE's; a compositor that insists on its own frame still gets one,
        // and the bar steps aside.
        window_decorations: Some(match settings.title_bar() {
            settings::TitleBar::Rustphin => gpui_kit::WindowDecorations::Client,
            settings::TitleBar::Kde => gpui_kit::WindowDecorations::Server,
        }),
        ..Default::default()
    };
    let opened = cx.open_window(window_options, |window, cx| {
        // Glass starts only after the owned capability monitor confirms support.
        let theme = Theme::obsidian(false);
        theme.apply_to_components(cx);
        cx.set_global(theme);
        window.set_window_title("Rustphin");
        let workspace = cx.new(|cx| {
            Workspace::new(
                session.clone(),
                options.home.clone(),
                session_path.clone(),
                settings.clone(),
                window,
                cx,
            )
        });
        if !options.force_opaque {
            let opaque = settings.look() == settings::Look::Opaque;
            workspace.update(cx, |workspace, cx| {
                workspace.follow_blur(window, cx);
                workspace.choose_opaque(opaque, window, cx);
            });
        }
        if !shown.is_empty() {
            workspace.update(cx, |workspace, _| workspace.show_later(shown));
        }
        // The component library's root paints its own opaque background
        // under the view. Clear it: the browser paints the window
        // background itself, translucent when the compositor blurs.
        cx.new(|cx| {
            gpui_kit::base::Root::new(workspace, window, cx).bg(gpui_kit::transparent_black())
        })
    });
    match opened {
        Ok(_) => cx.activate(true),
        Err(error) => {
            tracing::error!(%error, "could not open the main window");
            cx.quit();
        }
    }
}

fn warm_fonts(text_system: &gpui_kit::TextSystem) {
    let started = std::time::Instant::now();
    let fonts: Vec<_> = [
        gpui_kit::Font::default(),
        gpui_kit::font(theme::text::FAMILY_BODY),
        gpui_kit::font(theme::text::FAMILY_DATA),
        gpui_kit::font(theme::text::FAMILY_HEADING),
    ]
    .into_iter()
    .flat_map(|font| {
        [
            gpui_kit::FontWeight::NORMAL,
            gpui_kit::FontWeight::MEDIUM,
            gpui_kit::FontWeight::SEMIBOLD,
        ]
        .map(|weight| gpui_kit::Font {
            weight,
            ..font.clone()
        })
    })
    .collect();
    text_system.prewarm_fonts(&fonts);
    tracing::info!(
        elapsed_ms = started.elapsed().as_secs_f64() * 1e3,
        "font caches warmed"
    );
}
