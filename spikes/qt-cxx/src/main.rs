//! W-114a spike (throwaway): Qt Quick through cxx-qt over rustphin-core.

mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    // `--background`: no QML loaded, to measure a windowless process.
    let background = std::env::args().any(|arg| arg == "--background");
    let mut engine = QQmlApplicationEngine::new();
    if !background && let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/spike/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
