//! The cxx-qt bridge for the desktop object: declarations only.

pub use super::ShellRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// The desktop as a window sees it: opening files, asking before a
        /// program runs, more windows, quitting.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, launch)]
        type Shell = super::ShellRust;
    }

    unsafe extern "RustQt" {
        /// Opens a file: a document with its application; a program or
        /// shortcut after asking.
        #[qinvokable]
        fn activate(self: Pin<&mut Shell>, path: &QString);
        /// Answers the run question (`questions::Answer` numbers).
        #[qinvokable]
        #[cxx_name = "answerLaunch"]
        fn answer_launch(self: Pin<&mut Shell>, answer: i32);
        /// Opens another window on `path`.
        #[qinvokable]
        #[cxx_name = "openWindow"]
        fn open_window(self: Pin<&mut Shell>, path: &QString);
        /// Whether closing a window with several tabs asks first (Settings).
        #[qinvokable]
        #[cxx_name = "askBeforeClosingTabs"]
        fn ask_before_closing_tabs(self: &Shell) -> bool;
        /// The question before closing a window with `count` tabs, as JSON.
        #[qinvokable]
        #[cxx_name = "closeQuestion"]
        fn close_question(self: &Shell, count: i32) -> QString;
        /// Extracts the archives the pane kept under `ticket` into the
        /// folder at `url` (a `file://` address the folder dialog chose).
        #[qinvokable]
        #[cxx_name = "extractTo"]
        fn extract_to(self: Pin<&mut Shell>, url: &QString, ticket: i32);
        /// Closes every window and ends Rustphin.
        #[qinvokable]
        fn quit(self: Pin<&mut Shell>);
        /// The keyboard modifiers held now (a drop decides copy or move by
        /// them; QML's drop event does not carry them).
        #[qinvokable]
        #[cxx_name = "keyboardModifiers"]
        fn keyboard_modifiers(self: &Shell) -> i32;
        /// The window's shortcuts (`rustphin_view::commands`), as a JSON
        /// list of string rows: a command's id, "list" (only while the file
        /// list has the focus) or "window", then its sequences.
        #[qinvokable]
        fn shortcuts(self: &Shell) -> QString;
        /// What the command `id` acts on: "pane", "places", "window", or
        /// "" for no command.
        #[qinvokable]
        #[cxx_name = "commandTarget"]
        fn command_target(self: &Shell, id: &QString) -> QString;
    }

    impl cxx_qt::Threading for Shell {}
}
