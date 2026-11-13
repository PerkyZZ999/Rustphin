//! One Rustphin per session (W-110a, D-031): a launch either becomes the
//! running Rustphin or hands its request to it, and requests from other
//! launches and other applications reach the right window, or a new one
//! when none is open.

use std::{path::PathBuf, process::ExitCode};

use gpui_kit::{AnyWindowHandle, App, Entity, Global, WeakEntity};
use rustphin_platform::{
    file_manager::{Call, ServiceHandle, Shown},
    instance::{self as platform, ClaimError, HandOver, Instance, Request},
};

use crate::workspace::Workspace;

/// What became of this launch.
pub(crate) enum Claim {
    /// This process is the running Rustphin.
    Running(Instance),
    /// The running Rustphin took the request; this process ends.
    HandedOver,
    /// No bus, or the running Rustphin did not answer: work alone.
    Alone,
}

/// The launch's Wayland activation token, so the window it asks for may
/// take the focus.
fn activation_token() -> Option<String> {
    std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
}

/// Becomes the running Rustphin, or hands `starts` to the one running.
pub(crate) fn claim(starts: &[PathBuf]) -> Claim {
    for _ in 0..2 {
        match Instance::claim() {
            Ok(instance) => return Claim::Running(instance),
            Err(ClaimError::Bus(error)) => {
                tracing::warn!(%error, "no session bus; this Rustphin runs on its own");
                return Claim::Alone;
            }
            Err(ClaimError::Taken) => {}
        }
        let request = Request::Open {
            folders: starts.to_vec(),
            activation: activation_token(),
        };
        match platform::hand_over(&request, platform::ANSWER_TIME) {
            HandOver::Taken => return Claim::HandedOver,
            // The running one ended between the two steps: claim again.
            HandOver::NoneRunning => continue,
            HandOver::NoAnswer(reason) => {
                tracing::warn!(%reason, "the running Rustphin did not answer; starting another");
                return Claim::Alone;
            }
        }
    }
    Claim::Alone
}

/// `rustphin --quit`: asks the running Rustphin to end.
pub(crate) fn quit_running() -> ExitCode {
    match platform::hand_over(&Request::Quit, platform::ANSWER_TIME) {
        HandOver::Taken => ExitCode::SUCCESS,
        HandOver::NoneRunning => {
            eprintln!("Rustphin is not running.");
            ExitCode::SUCCESS
        }
        HandOver::NoAnswer(reason) => {
            eprintln!("Rustphin did not answer: {reason}");
            ExitCode::FAILURE
        }
    }
}

/// The open windows and their workspaces, oldest first.
#[derive(Default)]
pub(crate) struct OpenWorkspaces(Vec<(AnyWindowHandle, WeakEntity<Workspace>)>);

impl Global for OpenWorkspaces {}

/// Records a window that has just opened.
pub(crate) fn register(window: AnyWindowHandle, workspace: WeakEntity<Workspace>, cx: &mut App) {
    cx.default_global::<OpenWorkspaces>()
        .0
        .push((window, workspace));
}

/// The workspace a request goes to: the active window's, else the one
/// opened last. `None` when no window is open.
pub(crate) fn target(cx: &mut App) -> Option<Entity<Workspace>> {
    let open = cx.default_global::<OpenWorkspaces>();
    open.0
        .retain(|(_, workspace)| workspace.upgrade().is_some());
    let entries = open.0.clone();
    let active = cx.active_window();
    entries
        .iter()
        .find(|(window, _)| Some(*window) == active)
        .or(entries.last())
        .and_then(|(_, workspace)| workspace.upgrade())
}

/// The window a request went to: the active one, else the newest.
fn target_window(cx: &mut App) -> Option<AnyWindowHandle> {
    let active = cx.active_window();
    let open = cx.default_global::<OpenWorkspaces>();
    open.0
        .retain(|(_, workspace)| workspace.upgrade().is_some());
    open.0
        .iter()
        .find(|(window, _)| Some(*window) == active)
        .or(open.0.last())
        .map(|(window, _)| *window)
}

/// The newest window.
fn newest_window(cx: &mut App) -> Option<AnyWindowHandle> {
    let open = cx.default_global::<OpenWorkspaces>();
    open.0
        .retain(|(_, workspace)| workspace.upgrade().is_some());
    open.0.last().map(|(window, _)| *window)
}

/// Gives `window` the launch's activation token, so KWin lets it take the
/// focus (W-110d), once it has drawn a frame.
fn activate(window: AnyWindowHandle, token: String, cx: &mut App) {
    let _ = window.update(cx, |_, window, _| {
        window.on_next_frame(move |window, _| {
            if let Err(error) = rustphin_platform::activation::activate(window, window, &token) {
                tracing::info!(%error, "the window could not be given the focus");
            }
        });
    });
}

/// Every open window.
fn windows(cx: &mut App) -> Vec<AnyWindowHandle> {
    let open = cx.default_global::<OpenWorkspaces>();
    open.0
        .retain(|(_, workspace)| workspace.upgrade().is_some());
    open.0.iter().map(|(window, _)| *window).collect()
}

/// Shows `shown` in the target window, or in a new window when none is open.
fn show(shown: Vec<Shown>, cx: &mut App) {
    if shown.is_empty() {
        return;
    }
    match target(cx) {
        Some(workspace) if !shown_in_new_window(cx) => {
            workspace.update(cx, |workspace, cx| workspace.queue_shows(shown, cx));
        }
        // No window, or General says "In a new window" (W-106c).
        _ => crate::open_window_showing(shown, cx),
    }
}

/// Whether General shows other applications' folders in a new window.
fn shown_in_new_window(cx: &App) -> bool {
    crate::preferences::preferences(cx)
        .read(cx)
        .settings()
        .shown_folders()
        == crate::settings::ShownIn::Window
}

/// Answers one request from another launch.
fn answer(request: Request, cx: &mut App) {
    match request {
        Request::Open {
            folders,
            activation,
        } => {
            let window = if folders.is_empty() {
                if target(cx).is_some() {
                    crate::open_new_window(cx);
                } else {
                    crate::open_start_window(cx);
                }
                newest_window(cx)
            } else {
                let had_window = target(cx).is_some();
                show(
                    folders
                        .into_iter()
                        .map(|folder| Shown {
                            folder,
                            select: Vec::new(),
                        })
                        .collect(),
                    cx,
                );
                // Tabs go to the target window; with none, or when
                // Settings opens them in a new window, the newest has them.
                if had_window && !shown_in_new_window(cx) {
                    target_window(cx)
                } else {
                    newest_window(cx)
                }
            };
            if let (Some(window), Some(token)) = (window, activation) {
                activate(window, token, cx);
            }
        }
        Request::Quit => quit(cx),
    }
}

/// Closes every window and ends the process. Sessions are saved as they
/// change, so nothing is lost.
pub(crate) fn quit(cx: &mut App) {
    for window in windows(cx) {
        let _ = window.update(cx, |_, window, _| window.remove_window());
    }
    cx.quit();
}

/// Answers another application's `FileManager1` request.
fn answer_call(call: Call, cx: &mut App) {
    match call {
        Call::Show(shown) => show(shown, cx),
        Call::Properties(paths) => {
            if target(cx).is_none() {
                crate::open_start_window(cx);
            }
            if let Some(workspace) = target(cx) {
                workspace.update(cx, |workspace, cx| workspace.queue_properties(paths, cx));
            }
        }
    }
}

/// Keeps the running Rustphin's name and answers its requests.
pub(crate) fn serve(instance: Instance, cx: &mut App) {
    let incoming = instance.receiver();
    cx.set_global(Running(instance));
    cx.spawn(async move |cx| {
        while let Ok(request) = incoming.recv_async().await {
            cx.update(|cx| answer(request, cx));
        }
    })
    .detach();
}

/// Answers `FileManager1` requests for as long as the service runs.
pub(crate) fn serve_file_manager(service: ServiceHandle, cx: &mut App) {
    let incoming = service.receiver();
    cx.set_global(FileManagerService(service));
    cx.spawn(async move |cx| {
        while let Ok(call) = incoming.recv_async().await {
            cx.update(|cx| answer_call(call, cx));
        }
    })
    .detach();
}

/// Whether closing the last window leaves Rustphin running: it is the
/// running Rustphin (others hand their launches to it) and Settings keeps
/// it running (D-031).
pub(crate) fn keeps_running(cx: &App) -> bool {
    cx.has_global::<Running>()
        && crate::preferences::preferences(cx)
            .read(cx)
            .settings()
            .keep_running()
}

/// The last window has closed and Rustphin stays: let go of what only
/// windows need.
pub(crate) fn without_windows(cx: &mut App) {
    tracing::info!("last window closed; Rustphin keeps running");
    crate::thumbs::thumbs(cx).update(cx, |thumbs, _| thumbs.release());
}

/// The running Rustphin's bus name, held for the life of the process.
struct Running(#[allow(dead_code, reason = "held to keep the bus name")] Instance);
impl Global for Running {}

/// The `FileManager1` service, held for the life of the process.
struct FileManagerService(
    #[allow(dead_code, reason = "held to keep the service on the bus")] ServiceHandle,
);
impl Global for FileManagerService {}
