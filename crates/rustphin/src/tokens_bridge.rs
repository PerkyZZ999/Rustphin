//! The cxx-qt bridge for the tokens object: declarations only, no logic.

pub use super::TokensRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qmap.h");
        /// A map of names to values, a QML JavaScript object.
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    // SAFETY: the declarations match `cpp/window.h`; both functions take no
    // arguments and only call KDE's KWindowEffects on Qt's own windows.
    unsafe extern "C++" {
        include!("window.h");
        fn rustphin_blur_available() -> bool;
        fn rustphin_blur_windows();
    }

    extern "RustQt" {
        /// The colours and sizes of docs/DESIGN.md.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QMap_QString_QVariant, colors, READ, CONSTANT)]
        #[qproperty(QMap_QString_QVariant, metrics, READ, NOTIFY)]
        #[qproperty(bool, translucent, READ, CONSTANT)]
        type Tokens = super::TokensRust;
    }

    unsafe extern "RustQt" {
        /// Blurs the desktop behind the windows when the glass theme is in
        /// use; nothing otherwise.
        #[qinvokable]
        #[cxx_name = "blurWindows"]
        fn blur_windows(self: &Tokens);
        /// Takes up the text size Settings chose, when it changed.
        #[qinvokable]
        fn refresh(self: Pin<&mut Tokens>);
    }
}
