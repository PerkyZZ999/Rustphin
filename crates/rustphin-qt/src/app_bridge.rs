//! The cxx-qt bridge for the application set-up: declarations only.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        /// The QML engine.
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        include!("app.h");
        /// Registers fonts, the icon theme fallback and the image providers.
        fn rustphin_install(engine: Pin<&mut QQmlApplicationEngine>);
    }
}
