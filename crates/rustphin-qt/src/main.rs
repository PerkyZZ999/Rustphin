//! Rustphin's Qt 6 interface (W-114c, D-034), built beside the GPUI one
//! until it reaches parity. `rustphin-qt [folder…]` opens windows on the
//! folders, or at home; `--background` starts without a window and
//! `--quit` ends the running one (W-110, D-031).
//!
//! One Qt Rustphin runs per session: later launches hand their folders to
//! it and exit. Until the switch (W-114i) it owns its own bus name, so the
//! GPUI Rustphin in use never receives its launches.

mod desktop;
mod folder;
mod hub;
mod operations;
mod post;
mod prefs;
mod tokens;
mod toolbar;

// cxx-qt generates the unsafe FFI glue for the shell's calls; the bridge
// file holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "app_bridge.rs"]
mod shell;

use std::{path::PathBuf, process::ExitCode};

use cxx_qt_lib::{QGuiApplication, QString, QStringList};
use rustphin_platform::instance::{self, ANSWER_TIME, ClaimError, HandOver, Instance, Request};
use rustphin_view::launch::{Command, parse_launch};

/// The Qt interface's bus name until it replaces the GPUI one.
const BUS_NAME: &str = "org.rustphin.RustphinQt";

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

fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let launch = parse_launch(std::env::args_os(), &cwd);
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
    if let Some(instance) = &instance {
        let requests = instance.receiver();
        std::thread::spawn(move || {
            while let Ok(request) = requests.recv() {
                match request {
                    Request::Open {
                        folders,
                        activation,
                    } => shell::qobject::rustphin_shell_open(
                        &folder_list(&folders),
                        &QString::from(activation.as_deref().unwrap_or_default()),
                    ),
                    Request::Quit => shell::qobject::rustphin_shell_quit(),
                }
            }
        });
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
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
    fn folders_reach_qt_as_their_paths() {
        let list = folder_list(&[PathBuf::from("/srv"), PathBuf::from("/home/someone")]);
        let paths: Vec<String> = cxx_qt_lib::QList::<QString>::from(&list)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(paths, ["/srv", "/home/someone"]);
    }
}
