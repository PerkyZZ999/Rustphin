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

/// What `--quit` says and how it ends, from the running Rustphin's answer.
fn quit_outcome(answer: &HandOver) -> (Option<String>, bool) {
    match answer {
        HandOver::Taken => (None, true),
        HandOver::NoneRunning => (Some("Rustphin is not running.".to_owned()), true),
        HandOver::NoAnswer(reason) => (Some(format!("Rustphin did not answer: {reason}")), false),
    }
}

fn folder_list(folders: &[PathBuf]) -> QStringList {
    folders
        .iter()
        .map(|folder| QString::from(folder.to_string_lossy().as_ref()))
        .collect()
}

/// What another application asked through FileManager1, as the shell's
/// calls: each folder with the names to select, or the properties of paths.
fn answer_file_manager(call: Call) {
    match call {
        Call::Show(shown) => {
            let in_tabs =
                prefs::settings().shown_folders() == rustphin_view::settings::ShownIn::Tab;
            for (index, shown) in shown.iter().enumerate() {
                let (folder, select) = reveal_args(shown);
                // Later folders join the window the first one opened.
                shell::qobject::rustphin_shell_reveal(&folder, &select, in_tabs || index > 0);
            }
        }
        Call::Properties(paths) => {
            if !paths.is_empty() {
                shell::qobject::rustphin_shell_properties(&folder_list(&paths));
            }
        }
    }
}

/// A shown folder and the names to select in it, as Qt text.
fn reveal_args(shown: &Shown) -> (QString, QStringList) {
    let names: QStringList = shown
        .select
        .iter()
        .map(|name| QString::from(name.to_string_lossy().as_ref()))
        .collect();
    (
        QString::from(shown.folder.to_string_lossy().as_ref()),
        names,
    )
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
    // Logs go to stderr; RUSTPHIN_LOG sets the level (AGENTS.md).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RUSTPHIN_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
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
    let keep = keeps_running(instance.is_some(), prefs::settings().keep_running());
    let mut app = QGuiApplication::new();
    shell::qobject::rustphin_shell_start(keep, &QString::from(home.to_string_lossy().as_ref()));
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
mod tests {
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
        let dir = std::env::temp_dir().join(format!("rustphin-launch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        let file = dir.join("note.txt");
        assert!(std::fs::write(&file, b"note").is_ok());
        assert_eq!(folder_tab(file), dir);
        assert_eq!(folder_tab(dir.clone()), dir);
        let missing = dir.join("gone");
        assert_eq!(folder_tab(missing.clone()), missing);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shown_file_reaches_qt_as_its_folder_and_name() {
        let shown = Shown {
            folder: PathBuf::from("/srv/data"),
            select: vec!["report.pdf".into(), "notes.txt".into()],
        };
        let (folder, select) = reveal_args(&shown);
        let names: Vec<String> = cxx_qt_lib::QList::<QString>::from(&select)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(folder.to_string(), "/srv/data");
        assert_eq!(names, ["report.pdf", "notes.txt"]);
    }

    #[test]
    fn folders_reach_qt_as_their_paths() {
        let list = folder_list(&[PathBuf::from("/srv"), PathBuf::from("/home/someone")]);
        let paths: Vec<String> = cxx_qt_lib::QList::<QString>::from(&list)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(paths, ["/srv", "/home/someone"]);
    }
}
