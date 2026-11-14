//! The cxx-qt bridge for the application's windows: declarations only.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        /// A list of texts.
        type QStringList = cxx_qt_lib::QStringList;

        include!("app.h");
        /// Registers fonts and the icon theme, and how closing the last
        /// window is handled. Once, on the GUI thread, before the loop.
        fn rustphin_shell_start(keep_running: bool, home: &QString);
        /// Opens a window on each folder (one at home for none). Any thread.
        fn rustphin_shell_open(folders: &QStringList, activation: &QString);
        /// Closes every window and ends the process. Any thread.
        fn rustphin_shell_quit();
    }
}
