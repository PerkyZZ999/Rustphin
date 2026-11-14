//! The desktop as a window sees it (W-114d, W-114g): opening a file with
//! its application, asking first before running a program or starting a
//! shortcut (Settings, Confirmations), more windows, quitting.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "desktop_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use rustphin_platform::apps::{self, LaunchKind};
use rustphin_view::{
    commands::{self, Target},
    questions::{self, Answer},
    settings::Programs,
};

use crate::{hub, prefs};

/// A file waiting for the user to say whether to run it.
struct Pending {
    path: PathBuf,
    program: bool,
}

/// The Rust side of `Shell`.
#[derive(Default)]
pub struct ShellRust {
    launch: QString,
    pending: Option<Pending>,
}

/// The name of the file at `path`.
fn file_label(path: &Path) -> String {
    questions::label(path)
}

/// What answering the run question does, given the choice made once for
/// all in Settings: `Some(answer)` decides without asking.
fn decided(choice: Programs, program: bool) -> Option<Answer> {
    match (choice, program) {
        (Programs::Run, _) => Some(Answer::Run),
        (Programs::Open, true) => Some(Answer::Open),
        _ => None,
    }
}

/// Runs, opens or leaves the file, in the background; failures are said.
fn carry_out(path: PathBuf, program: bool, answer: Answer) {
    let run = match answer {
        Answer::Run => true,
        Answer::Open => false,
        _ => return,
    };
    std::thread::spawn(move || {
        let result = if !run {
            apps::open_with_default(&path)
        } else if program {
            apps::run_program(&path)
        } else {
            apps::run_desktop(&path)
        };
        if let Err(error) = result {
            hub::say(error.to_string(), true);
        }
    });
}

impl qobject::Shell {
    fn activate(self: Pin<&mut Self>, path: &QString) {
        if self.rust().pending.is_some() {
            return;
        }
        let path = PathBuf::from(path.to_string());
        let thread = self.qt_thread();
        std::thread::spawn(move || match apps::launch_kind(&path) {
            LaunchKind::Document => {
                if let Err(error) = apps::open_with_default(&path) {
                    hub::say(error.to_string(), true);
                }
            }
            LaunchKind::Unreadable(message) => hub::say(message, true),
            LaunchKind::Program => {
                let _ = thread.queue(move |shell| shell.ask(path, file_label_owned(), true));
            }
            LaunchKind::Desktop { name } => {
                let _ = thread.queue(move |shell| shell.ask(path, name, false));
            }
        });
    }

    /// Asks before running, unless Settings already answered.
    fn ask(mut self: Pin<&mut Self>, path: PathBuf, name: String, program: bool) {
        let name = if name.is_empty() {
            file_label(&path)
        } else {
            name
        };
        if let Some(answer) = decided(prefs::settings().confirmations().programs, program) {
            return carry_out(path, program, answer);
        }
        let location = path
            .parent()
            .map(|folder| folder.display().to_string())
            .unwrap_or_default();
        let shown = questions::launch(&name, &location, program);
        self.as_mut().rust_mut().pending = Some(Pending { path, program });
        self.set_launch(QString::from(
            &serde_json::to_string(&shown).unwrap_or_default(),
        ));
    }

    fn answer_launch(mut self: Pin<&mut Self>, answer: i32) {
        let pending = self.as_mut().rust_mut().pending.take();
        self.as_mut().set_launch(QString::default());
        if let (Some(pending), Some(answer)) = (pending, Answer::from_number(answer)) {
            carry_out(pending.path, pending.program, answer);
        }
    }

    fn open_window(self: Pin<&mut Self>, path: &QString) {
        let list: cxx_qt_lib::QList<QString> = std::iter::once(path.clone()).collect();
        crate::shell::qobject::rustphin_shell_open(&QStringList::from(&list), &QString::default());
    }

    fn ask_before_closing_tabs(&self) -> bool {
        prefs::settings().confirmations().close_tabs
    }

    fn close_question(&self, count: i32) -> QString {
        let shown = questions::close_window(usize::try_from(count).unwrap_or(0));
        QString::from(&serde_json::to_string(&shown).unwrap_or_default())
    }

    fn extract_to(self: Pin<&mut Self>, url: &QString, ticket: i32) {
        let dialog = crate::hub::with(|hub| hub.dialogs.take(ticket));
        let Some(rustphin_view::selection_actions::Dialog::ExtractTo(extract)) = dialog else {
            return;
        };
        let Some(into) = rustphin_platform::uri::path_from_uri(url.to_string().as_bytes()) else {
            return;
        };
        for request in extract.requests(&into) {
            crate::hub::start(request);
        }
    }

    fn quit(self: Pin<&mut Self>) {
        crate::shell::qobject::rustphin_shell_quit();
    }

    fn keyboard_modifiers(&self) -> i32 {
        crate::shell::qobject::rustphin_keyboard_modifiers()
    }

    fn shortcuts(&self) -> QString {
        QString::from(&shortcuts_json())
    }

    fn command_target(&self, id: &QString) -> QString {
        QString::from(target_name(&id.to_string()))
    }
}

/// The window's shortcuts for QML (`Main.qml` builds a `Shortcut` from
/// each): rows of strings, the command's id, "list" or "window", then its
/// sequences.
fn shortcuts_json() -> String {
    let rows: Vec<Vec<&str>> = commands::window_shortcuts()
        .into_iter()
        .map(|shortcut| {
            let catcher = if shortcut.list { "list" } else { "window" };
            [shortcut.id, catcher]
                .into_iter()
                .chain(shortcut.keys)
                .collect()
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_owned())
}

/// What the command `id` acts on, as QML names it.
fn target_name(id: &str) -> &'static str {
    match commands::find(id).map(commands::Command::target) {
        Some(Target::Pane) => "pane",
        Some(Target::Places) => "places",
        Some(Target::Window) => "window",
        None => "",
    }
}

/// An empty name: the file's own name is used.
fn file_label_owned() -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qml_learns_each_command_s_target_and_the_window_s_shortcuts() {
        assert_eq!(target_name("go-back"), "pane");
        assert_eq!(target_name("rename-row"), "places");
        assert_eq!(target_name("quit"), "window");
        assert_eq!(target_name("no-such-command"), "");
        let rows: Vec<Vec<String>> = serde_json::from_str(&shortcuts_json()).unwrap_or_default();
        let has = |row: &[&str]| rows.iter().any(|found| found == row);
        assert!(has(&["go-back", "window", "Alt+Left", "Back"]));
        assert!(has(&["copy", "list", "Ctrl+C"]));
    }

    /// The window's commands run in QML (Main.qml's `runCommand`): each
    /// needs its case there, or its key and palette entry do nothing.
    #[test]
    fn main_qml_runs_every_window_command() {
        let main = include_str!("../qml/Main.qml");
        for command in commands::COMMANDS {
            if command.target() != Target::Window {
                continue;
            }
            let handled = main.contains(&format!("case \"{}\":", command.id))
                || (command.id.starts_with("activate-tab-")
                    && main.contains("id.indexOf(\"activate-tab-\") === 0"));
            assert!(handled, "Main.qml does not run {}", command.id);
        }
    }

    #[test]
    fn settings_may_answer_the_run_question_for_good() {
        assert_eq!(decided(Programs::Run, true), Some(Answer::Run));
        assert_eq!(decided(Programs::Run, false), Some(Answer::Run));
        assert_eq!(decided(Programs::Open, true), Some(Answer::Open));
        // A shortcut has nothing to open it with: it is asked about.
        assert_eq!(decided(Programs::Open, false), None);
        assert_eq!(decided(Programs::Ask, true), None);
    }
}
