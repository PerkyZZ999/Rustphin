//! Rustphin, a file manager for KDE Plasma on Wayland, with a Qt 6 and QML
//! interface (D-034). `rustphin [folder…]` opens windows on the folders
//! (a file opens the folder that holds it), or the saved tabs;
//! `--background` starts without a window and `--quit` ends the running
//! one (W-110, D-031).
//!
//! One Rustphin runs per session: later launches hand their folders to it
//! over the session bus and exit.

mod admin;
mod desktop;
mod dialogs;
mod folder;
mod history;
mod hub;
mod info;
mod operations;
mod palette;
mod places;
mod post;
mod preferences;
mod prefs;
mod qml_path;
mod remote;
mod settings;
mod tabs;
mod tokens;
mod toolbar;
mod types;

// cxx-qt generates the unsafe FFI glue for the shell's calls; the bridge
// file holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "app_bridge.rs"]
mod shell;

use std::{path::PathBuf, process::ExitCode};

use cxx_qt_lib::{QGuiApplication, QString, QStringList};
use rustphin_platform::file_manager::{Call, Shown};
use rustphin_platform::instance::{self, ANSWER_TIME, ClaimError, HandOver, Instance, Request};
use rustphin_view::launch::{Command, parse_launch};

/// The bus name later launches hand their folders to.
const BUS_NAME: &str = instance::NAME;

/// Whether closing the last window leaves the process running: only for
/// the process that serves later launches, and when Settings allows it.
fn keeps_running(owns_name: bool, setting: bool) -> bool {
    owns_name && setting
}

/// This process serves later launches (it owns the bus name).
static SERVES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Tells the shell what closing the last window does now, after Settings
/// changed (W-158: the choice used to wait for the next start). Qt thread.
pub(crate) fn follow_keep_running() {
    let serves = SERVES.load(std::sync::atomic::Ordering::Relaxed);
    shell::qobject::rustphin_shell_keep_running(keeps_running(
        serves,
        prefs::settings().keep_running(),
    ));
}

/// What `--quit` says and how it ends, from the running Rustphin's answer.
fn quit_outcome(answer: &HandOver) -> (Option<String>, bool) {
    match answer {
        HandOver::Taken => (None, true),
        HandOver::NoneRunning => (Some("Rustphin is not running.".to_owned()), true),
        HandOver::NoAnswer(reason) => (Some(format!("Rustphin did not answer: {reason}")), false),
    }
}

/// Folders for QML, as `file://` URIs that keep every byte (W-132).
fn folder_list(folders: &[PathBuf]) -> QStringList {
    qml_path::list_to_qml(folders)
}

/// What another application asked through FileManager1, as the shell's
/// calls: each folder with the names to select, or the properties of paths.
fn answer_file_manager(call: Call) {
    match call {
        Call::Show { shown, activation } => {
            let in_tabs =
                prefs::settings().shown_folders() == rustphin_view::settings::ShownIn::Tab;
            // The caller's token lets the window take the focus; it is good
            // for one activation, so only the first folder uses it.
            let mut token = QString::from(activation.as_str());
            for (index, shown) in shown.iter().enumerate() {
                let (folder, select) = reveal_args(shown);
                // Later folders join the window the first one opened.
                shell::qobject::rustphin_shell_reveal(
                    &folder,
                    &select,
                    in_tabs || index > 0,
                    &token,
                );
                token = QString::default();
            }
        }
        Call::Properties { paths, activation } => {
            if !paths.is_empty() {
                shell::qobject::rustphin_shell_properties(
                    &folder_list(&paths),
                    &QString::from(activation.as_str()),
                );
            }
        }
    }
}

/// A shown folder and the names to select in it, for QML: the folder's
/// `file://` URI and the names percent-encoded, so no byte is lost (W-132).
fn reveal_args(shown: &Shown) -> (QString, QStringList) {
    let names = qml_path::names_to_qml(shown.select.iter().map(|name| name.as_encoded_bytes()));
    (qml_path::to_qml(&shown.folder), names)
}

/// A directory stays as it is. An existing file becomes the folder around
/// it, so a tab always has a directory to list.
fn folder_tab(path: PathBuf) -> PathBuf {
    let is_file = std::fs::metadata(&path).is_ok_and(|meta| meta.is_file());
    if is_file {
        path.parent()
            .map(std::path::Path::to_path_buf)
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(path)
    } else {
        path
    }
}

fn main() -> ExitCode {
    // Before any thread starts: freed listings otherwise stay spread over
    // many arenas (W-115).
    let arenas_limited = rustphin_platform::memory::limit_arenas();
    // Logs go to stderr; RUSTPHIN_LOG sets the level (AGENTS.md).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RUSTPHIN_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    if !arenas_limited {
        tracing::warn!("the allocator kept its default arenas");
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let mut launch = parse_launch(std::env::args_os(), &cwd);
    launch.targets = launch.targets.into_iter().map(folder_tab).collect();
    let activation = std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .filter(|token| !token.is_empty());
    match launch.command {
        Command::Quit => {
            let (words, ok) = quit_outcome(&instance::hand_over_to(
                BUS_NAME,
                &Request::Quit,
                ANSWER_TIME,
            ));
            if let Some(words) = words {
                eprintln!("{words}");
            }
            return if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        Command::Open => {
            let request = Request::Open {
                folders: launch.targets.clone(),
                activation: activation.clone(),
            };
            match instance::hand_over_to(BUS_NAME, &request, ANSWER_TIME) {
                HandOver::Taken => return ExitCode::SUCCESS,
                HandOver::NoneRunning => {}
                HandOver::NoAnswer(reason) => {
                    eprintln!("The running Rustphin did not answer ({reason}); starting another.");
                }
            }
        }
        Command::Background => {}
    }
    let instance = match Instance::claim_as(BUS_NAME) {
        Ok(instance) => Some(instance),
        // Another one started meanwhile; a background start has nothing to add.
        Err(ClaimError::Taken) if launch.command == Command::Background => {
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("Rustphin will not serve later launches: {error}.");
            None
        }
    };
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    SERVES.store(instance.is_some(), std::sync::atomic::Ordering::Relaxed);
    let keep = keeps_running(instance.is_some(), prefs::settings().keep_running());
    let mut app = QGuiApplication::new();
    shell::qobject::rustphin_shell_start(keep, &qml_path::to_qml(&home));
    if launch.command == Command::Open {
        shell::qobject::rustphin_shell_open(
            &folder_list(&launch.targets),
            &QString::from(activation.as_deref().unwrap_or_default()),
        );
    }
    if instance.is_some() {
        history::recover_interrupted_renames();
    }
    if let Some(instance) = &instance {
        let requests = instance.receiver();
        std::thread::spawn(move || {
            while let Ok(request) = requests.recv() {
                match request {
                    Request::Open {
                        folders,
                        activation,
                    } => post::on_qt(move || {
                        // General: other applications' folders open as tabs
                        // of the open window, or in a window of their own.
                        let in_tabs = prefs::settings().shown_folders()
                            == rustphin_view::settings::ShownIn::Tab;
                        shell::qobject::rustphin_shell_show(
                            &folder_list(&folders),
                            &QString::from(activation.as_deref().unwrap_or_default()),
                            in_tabs,
                        );
                    }),
                    Request::Quit => shell::qobject::rustphin_shell_quit(),
                }
            }
        });
    }
    // Other applications' "show this file" requests (FileManager1), only
    // in the process that serves later launches.
    let file_manager = instance
        .as_ref()
        .and_then(|_| rustphin_platform::file_manager::ServiceHandle::session());
    if let Some(service) = &file_manager {
        tracing::info!("listening for requests to show files");
        let calls = service.receiver();
        std::thread::spawn(move || {
            while let Ok(call) = calls.recv() {
                post::on_qt(move || answer_file_manager(call));
            }
        });
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    drop(file_manager);
    drop(instance);
    ExitCode::SUCCESS
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rustphin_view::{location::Location, workspace::PaneStart};

    use super::*;

    #[test]
    fn only_the_serving_process_keeps_running_and_only_when_allowed() {
        assert!(keeps_running(true, true));
        assert!(!keeps_running(true, false));
        assert!(!keeps_running(false, true));
    }

    #[test]
    fn quit_says_when_nothing_runs_and_fails_when_nothing_answers() {
        assert_eq!(quit_outcome(&HandOver::Taken), (None, true));
        assert_eq!(
            quit_outcome(&HandOver::NoneRunning),
            (Some("Rustphin is not running.".to_owned()), true)
        );
        assert!(!quit_outcome(&HandOver::NoAnswer("slow".into())).1);
    }

    #[test]
    fn a_file_opens_the_folder_that_contains_it() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let file = dir.join("note.txt");
        std::fs::write(&file, b"note").unwrap();
        assert_eq!(folder_tab(file), dir);
        assert_eq!(folder_tab(dir.to_owned()), dir);
        let missing = dir.join("gone");
        assert_eq!(folder_tab(missing.clone()), missing);
    }

    #[test]
    fn a_shown_file_reaches_qml_as_its_folder_and_name_byte_for_byte() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let folder = PathBuf::from(OsStr::from_bytes(b"/srv/caf\xe9"));
        let shown = Shown {
            folder: folder.clone(),
            select: vec![OsStr::from_bytes(b"n\xe9.txt").into(), "notes.txt".into()],
        };
        let (uri, select) = reveal_args(&shown);
        // What the pane reads back when it selects them (FolderModel.reveal).
        assert_eq!(qml_path::from_qml(&uri), Some(folder));
        assert_eq!(
            qml_path::names_from_qml(&select),
            [b"n\xe9.txt".to_vec(), b"notes.txt".to_vec()]
        );
    }

    #[test]
    fn launch_folders_reach_qml_byte_for_byte() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let odd = PathBuf::from(OsStr::from_bytes(b"/home/someone/caf\xe9"));
        let list = folder_list(&[PathBuf::from("/srv"), odd.clone()]);
        // What the tabs read back (PaneStart::parse) is the same folder.
        let back: Vec<_> = cxx_qt_lib::QList::<QString>::from(&list)
            .iter()
            .map(|text| PaneStart::parse(&text.to_string()))
            .collect();
        assert_eq!(
            back,
            [
                PaneStart::At(Location::Folder(PathBuf::from("/srv"))),
                PaneStart::At(Location::Folder(odd)),
            ]
        );
    }
}
