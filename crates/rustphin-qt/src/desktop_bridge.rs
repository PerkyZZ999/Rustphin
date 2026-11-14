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
        /// Closes every window and ends Rustphin.
        #[qinvokable]
        fn quit(self: Pin<&mut Shell>);
    }

    impl cxx_qt::Threading for Shell {}
}
