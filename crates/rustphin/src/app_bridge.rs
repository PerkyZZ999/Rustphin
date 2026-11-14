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
        /// Shows another launch's folders: as tabs of the window last used
        /// when `in_tabs` and there is one, else in a new window. Qt thread.
        fn rustphin_shell_show(folders: &QStringList, activation: &QString, in_tabs: bool);
        /// Shows `folder` with `select` selected, for another application:
        /// a tab of the window last used when `in_tabs`, else a new window.
        /// Qt thread.
        fn rustphin_shell_reveal(folder: &QString, select: &QStringList, in_tabs: bool);
        /// Opens Properties on `paths` in the window last used. Qt thread.
        fn rustphin_shell_properties(paths: &QStringList);
        /// Closes every window and ends the process. Any thread.
        fn rustphin_shell_quit();
        /// Runs `run` on the GUI thread, from its event loop. Any thread.
        fn rustphin_post(run: fn());
    }
}
