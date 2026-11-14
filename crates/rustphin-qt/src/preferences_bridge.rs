//! The cxx-qt bridge for the settings object: declarations only.

pub use super::PreferencesRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        /// The shared settings as QML follows them: `revision` changes
        /// whenever they do, so views take them up.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision)]
        type Preferences = super::PreferencesRust;
    }

    unsafe extern "RustQt" {
        /// Follows the settings; call once from QML.
        #[qinvokable]
        fn watch(self: Pin<&mut Preferences>);
        /// Counts one more change.
        fn bump(self: Pin<&mut Preferences>);
    }

    impl cxx_qt::Threading for Preferences {}
}
