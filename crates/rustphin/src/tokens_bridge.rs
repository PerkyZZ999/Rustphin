//! The cxx-qt bridge for the tokens object: declarations only, no logic.

pub use super::TokensRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qmap.h");
        /// A map of names to values, a QML JavaScript object.
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    // SAFETY: the declarations match `cpp/window.h`; the functions take at
    // most a flag and only call KDE's KWindowEffects on Qt's own windows.
    unsafe extern "C++" {
        include!("window.h");
        fn rustphin_blur_available() -> bool;
        fn rustphin_blur_windows(blur: bool);
    }

    extern "RustQt" {
        /// The colours and sizes of docs/DESIGN.md.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QMap_QString_QVariant, colors, READ, NOTIFY)]
        #[qproperty(QMap_QString_QVariant, metrics, READ, NOTIFY)]
        #[qproperty(bool, translucent, READ, NOTIFY)]
        #[qproperty(bool, blur_missing, cxx_name = "blurMissing", READ, NOTIFY)]
        type Tokens = super::TokensRust;
    }

    unsafe extern "RustQt" {
        /// Blurs the desktop behind the windows when the glass theme is in
        /// use, and stops blurring otherwise.
        #[qinvokable]
        #[cxx_name = "blurWindows"]
        fn blur_windows(self: &Tokens);
        /// Takes up the text size and the look Settings chose and whether
        /// the desktop offers blur now, when any of them changed.
        #[qinvokable]
        fn refresh(self: Pin<&mut Tokens>);
    }
}
