//! The GPUI user interface of Rustphin.
//!
//! This crate renders snapshots published by `rustphin-core` and sends it
//! commands. It never reads the filesystem itself.

mod archive_tools;
use rustphin_view::archives;
mod browser;
mod commands;
mod compress_dialog;
mod controls;
mod diagnostics;
mod dialog;
mod filetypes;
use rustphin_view::folder_views;
mod fonts;
use rustphin_view::format;
use rustphin_view::groups;
mod history;
mod icon_map;
mod input;
mod instance;
use rustphin_view::launch;
use rustphin_view::layout;
use rustphin_view::location;
use rustphin_view::menu;
mod menu_view;
mod operations;
mod palette;
use rustphin_view::place_menu;
mod preferences;
mod properties;
mod recovery;
mod remote;
mod rename_dialog;
use rustphin_view::session;
use rustphin_view::settings;
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
use rustphin_view::tabs;
mod theme;
mod thumbs;
mod titlebar;
use rustphin_view::toolbar_items;
mod workspace;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Bounds, Styled as _, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, px, size,
};

use crate::{session::Session, settings::Settings, theme::Theme, workspace::Workspace};

pub use launch::{Command, Launch, launch_targets, parse_launch};

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

/// What a launch needs: its command, the folders named, home and the look.
pub struct Start {
    /// What the command line asked for.
    pub command: Command,
    /// Folders to open as tabs. Empty restores the previous session.
    pub starts: Vec<PathBuf>,
    /// The folder to show when there is neither a start folder nor a session.
    pub home: PathBuf,
    /// Draw opaque surfaces even when the compositor could blur.
    pub force_opaque: bool,
}

/// Starts Rustphin, or hands the launch to the one already running
/// (W-110a): `--quit` ends it, anything else opens a window in it.
pub fn start(start: Start) -> std::process::ExitCode {
    if start.command == Command::Quit {
        return instance::quit_running();
    }
    if start.command == Command::Background {
        // At login: become the running Rustphin without a window, or do
        // nothing when one already runs.
        return match rustphin_platform::instance::Instance::claim() {
            Ok(running) => {
                run_as(
                    Options {
                        starts: Vec::new(),
                        home: start.home,
                        force_opaque: start.force_opaque,
                    },
                    Some(running),
                    true,
                );
                std::process::ExitCode::SUCCESS
            }
            Err(rustphin_platform::instance::ClaimError::Taken) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                tracing::warn!(%error, "cannot run in the background");
                std::process::ExitCode::FAILURE
            }
        };
    }
    let instance = match instance::claim(&start.starts) {
        instance::Claim::HandedOver => return std::process::ExitCode::SUCCESS,
        instance::Claim::Running(instance) => Some(instance),
        instance::Claim::Alone => None,
    };
    run_as(
        Options {
            starts: start.starts,
            home: start.home,
            force_opaque: start.force_opaque,
        },
        instance,
        false,
    );
    std::process::ExitCode::SUCCESS
}

/// Runs the application until its last window closes.
pub fn run(options: Options) {
    run_as(options, None, false);
}

/// [`run`], as the running Rustphin when `instance` holds the bus name;
/// with no window when `background` (W-110c).
fn run_as(
    options: Options,
    mut instance: Option<rustphin_platform::instance::Instance>,
    background: bool,
) {
    // The toolkit's own font scan runs one file at a time; read the same
    // files on several threads first, so it finds them in memory (W-109b).
    let warmup = rustphin_platform::font_warmup::start(|outcome, elapsed, threads| {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        match outcome {
            rustphin_platform::font_warmup::Outcome::Read(report) => tracing::info!(
                files = report.files,
                faces = report.faces,
                threads,
                elapsed_ms,
                "font files pre-read"
            ),
            rustphin_platform::font_warmup::Outcome::AlreadyInMemory => {
                tracing::info!(elapsed_ms, "font files already in memory; not pre-read")
            }
        }
    });
    if let Err(error) = warmup {
        tracing::warn!(%error, "could not start reading the fonts ahead");
    }
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
            // Font matching walks the installed font collection: start the
            // faces the first frame needs at once, off-thread, so the work
            // below overlaps it; the window opens when they are ready.
            let text_system = cx.text_system().clone();
            let warming = cx.background_executor().spawn(async move {
                fonts::register(&text_system);
                warm_fonts(&text_system, &fonts::FIRST_FRAME, "first-frame");
            });
            gpui_kit::init(cx);
            // File types and icons, once for every pane, while fonts warm.
            filetypes::init(cx);
            gpui_kit::component::theme::Theme::change(
                gpui_kit::component::theme::ThemeMode::Dark,
                None,
                cx,
            );
            operations::init(cx);
            cx.set_global(WindowDefaults {
                home: options.home.clone(),
                force_opaque: options.force_opaque,
                session_path: session_path.clone(),
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
            if let Some(running) = instance.take() {
                instance::serve(running, cx);
            }
            // Keep the login entry pointing at this program (W-110c).
            if settings.start_at_login() {
                cx.background_spawn(async {
                    if let Err(error) = rustphin_platform::autostart::set_for_this_program(true) {
                        tracing::warn!(%error, "could not refresh the login entry");
                    }
                })
                .detach();
            }
            remote::init(cx);
            // D-020: restore batch renames an earlier run left half done.
            recovery::start(&session, cx);
            if let Some(service) = rustphin_platform::file_manager::ServiceHandle::session() {
                tracing::info!("listening for requests to show files");
                instance::serve_file_manager(service, cx);
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
            // Rustphin decides when to quit, not the toolkit (W-110b).
            cx.set_quit_mode(gpui_kit::QuitMode::Explicit);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    if instance::keeps_running(cx) {
                        instance::without_windows(cx);
                    } else {
                        cx.quit();
                    }
                }
            })
            .detach();

            cx.spawn(async move |cx| {
                warming.await;
                let text_system = cx.update(|cx| {
                    if background {
                        tracing::info!("started in the background; no window");
                    } else {
                        open_window(&options, settings, session, session_path, Vec::new(), cx);
                    }
                    cx.text_system().clone()
                });
                // The faces later screens use, while the window is up.
                cx.background_spawn(async move {
                    warm_fonts(&text_system, &fonts::LATER, "later");
                })
                .await;
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
    /// Where the session is saved; the start window saves it.
    session_path: Option<PathBuf>,
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

/// The window a plain launch opens when no window is open: what the start
/// settings choose, the saved session included, saving it again as the
/// first window did (W-110a).
pub(crate) fn open_start_window(cx: &mut App) {
    let Some(defaults) = cx.try_global::<WindowDefaults>() else {
        return;
    };
    let options = Options {
        starts: Vec::new(),
        home: defaults.home.clone(),
        force_opaque: defaults.force_opaque,
    };
    let session_path = defaults.session_path.clone();
    let settings = preferences::preferences(cx).read(cx).settings().clone();
    let starts = start_folders(&options, &settings);
    let saved = session_path
        .as_deref()
        .map_or(Ok(None), rustphin_platform::config::read);
    let restored = session::restore(saved, &starts);
    let session_path = session_path.filter(|_| restored.can_save());
    open_window(
        &options,
        settings,
        restored.session,
        session_path,
        Vec::new(),
        cx,
    );
}

/// The window a plain launch opens beside open ones, as Dolphin's new
/// window: the start folder, or home when the start restores a session.
pub(crate) fn open_new_window(cx: &mut App) {
    let Some(defaults) = cx.try_global::<WindowDefaults>() else {
        return;
    };
    let home = defaults.home.clone();
    let settings = preferences::preferences(cx).read(cx).settings().clone();
    let options = Options {
        starts: Vec::new(),
        home: home.clone(),
        force_opaque: defaults.force_opaque,
    };
    let folder = match settings.start() {
        (settings::Start::Folder, Some(folder)) => match location::Location::from_saved(folder) {
            Some(location::Location::Folder(path)) => path,
            _ => home,
        },
        _ => home,
    };
    open_window(
        &options,
        settings,
        Session::folders(vec![folder]),
        None,
        Vec::new(),
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
        instance::register(window.window_handle(), workspace.downgrade(), cx);
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

fn warm_fonts(
    text_system: &gpui_kit::TextSystem,
    faces: &[(&str, gpui_kit::FontWeight)],
    which: &'static str,
) {
    let started = std::time::Instant::now();
    let fonts: Vec<_> = faces
        .iter()
        .map(|(family, weight)| gpui_kit::Font {
            weight: *weight,
            ..gpui_kit::font(*family)
        })
        .collect();
    text_system.prewarm_fonts(&fonts);
    tracing::info!(
        which,
        elapsed_ms = started.elapsed().as_secs_f64() * 1e3,
        "font caches warmed"
    );
}
