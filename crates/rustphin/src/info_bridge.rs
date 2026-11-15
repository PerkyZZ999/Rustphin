//! The cxx-qt bridge for the Properties dialog's object: declarations only.

pub use super::PropertiesModelRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// One entry's properties and permissions (W-114f).
        #[qobject]
        #[qml_element]
        #[qproperty(i32, window)]
        #[qproperty(QString, title)]
        #[qproperty(QString, kind)]
        #[qproperty(QString, icon)]
        #[qproperty(QString, theme_icon, cxx_name = "themeIcon")]
        #[qproperty(QString, thumbnail)]
        #[qproperty(QString, facts)]
        #[qproperty(QString, permissions)]
        #[qproperty(QString, error)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, dirty)]
        type PropertiesModel = super::PropertiesModelRust;
    }

    unsafe extern "RustQt" {
        /// Reads the entry at the `file:` URI `uri` in the background.
        #[qinvokable]
        fn open(self: Pin<&mut PropertiesModel>, uri: &QString);
        /// Turns one permission on or off: `group` (the class) 0–2, `which` 0 read,
        /// 1 write, 2 execute.
        #[qinvokable]
        #[cxx_name = "setBit"]
        fn set_bit(self: Pin<&mut PropertiesModel>, group: i32, which: i32, on: bool);
        /// Saves changed permissions through the operation engine.
        #[qinvokable]
        fn apply(self: Pin<&mut PropertiesModel>);
        /// Stops measuring a folder.
        #[qinvokable]
        fn close(self: Pin<&mut PropertiesModel>);
        /// Shows the measuring's progress.
        fn tick(self: Pin<&mut PropertiesModel>);
    }

    impl cxx_qt::Threading for PropertiesModel {}
}
