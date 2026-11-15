//! The cxx-qt bridge for the Settings dialog's object: declarations only.

pub use super::SettingsModelRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// The Settings dialog (W-106, W-114f): its pages as JSON, built
        /// in Rust from the settings and the desktop's facts.
        #[qobject]
        #[qml_element]
        #[qproperty(i32, page)]
        #[qproperty(QString, pages)]
        #[qproperty(QString, groups)]
        #[qproperty(QString, toolbar)]
        #[qproperty(QString, about)]
        #[qproperty(i32, revision)]
        type SettingsModel = super::SettingsModelRust;
    }

    unsafe extern "RustQt" {
        /// Reads the desktop's facts and shows the first page; `shown` is
        /// the `file:` URI of the folder the window shows, or empty.
        #[qinvokable]
        fn start(self: Pin<&mut SettingsModel>, shown: &QString);
        /// Shows page `page`.
        #[qinvokable]
        #[cxx_name = "choosePage"]
        fn choose_page(self: Pin<&mut SettingsModel>, page: i32);
        /// Row `key` changed to `value`.
        #[qinvokable]
        fn set(self: Pin<&mut SettingsModel>, key: &QString, value: &QString);
        /// Puts the page's settings back to their defaults.
        #[qinvokable]
        fn reset(self: Pin<&mut SettingsModel>);
        /// The Shortcuts page's groups matching `search`, as JSON.
        #[qinvokable]
        fn shortcuts(self: &SettingsModel, search: &QString) -> QString;
        /// The numbers of the pages the dialog's search `query` finds, as
        /// JSON.
        #[qinvokable]
        fn matching(self: &SettingsModel, query: &QString) -> QString;
        /// The Toolbar page's available actions matching `search`, as JSON.
        #[qinvokable]
        fn available(self: &SettingsModel, search: &QString) -> QString;
        /// Puts available action `index` (of `search`'s list) on the
        /// toolbar after `after`; returns where it went.
        #[qinvokable]
        #[cxx_name = "addItem"]
        fn add_item(self: Pin<&mut SettingsModel>, search: &QString, index: i32, after: i32)
        -> i32;
        /// Takes toolbar item `index` off.
        #[qinvokable]
        #[cxx_name = "removeItem"]
        fn remove_item(self: Pin<&mut SettingsModel>, index: i32);
        /// Moves toolbar item `index` by `delta`; returns where it went.
        #[qinvokable]
        #[cxx_name = "shiftItem"]
        fn shift_item(self: Pin<&mut SettingsModel>, index: i32, delta: i32) -> i32;
        /// Shows toolbar item `index`'s words, or not.
        #[qinvokable]
        #[cxx_name = "setWords"]
        fn set_words(self: Pin<&mut SettingsModel>, index: i32, words: bool);
        /// Rustphin's licence.
        #[qinvokable]
        fn licence(self: &SettingsModel) -> QString;
        /// Opens the folder that holds the settings file.
        #[qinvokable]
        #[cxx_name = "openFolder"]
        fn open_folder(self: &SettingsModel);
    }

    impl cxx_qt::Threading for SettingsModel {}
}
