//! Rustphin's Qt 6 interface (W-114c, D-034), built beside the GPUI one
//! until it reaches parity. `rustphin-qt [folder]` shows a folder; with no
//! argument, the home folder.

mod folder;
mod tokens;

// cxx-qt generates the unsafe FFI glue for the set-up call; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "app_bridge.rs"]
mod setup;

use std::path::PathBuf;

use cxx_qt_lib::{
    QGuiApplication, QMap, QMapPair_QString_QVariant, QQmlApplicationEngine, QString, QUrl,
    QVariant,
};

/// The folder to show first: the first argument, or the home folder.
fn start_folder(argument: Option<std::ffi::OsString>, home: Option<PathBuf>) -> PathBuf {
    argument
        .map(PathBuf::from)
        .or(home)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn main() {
    let folder = start_folder(
        std::env::args_os().nth(1),
        std::env::var_os("HOME").map(PathBuf::from),
    );
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        let mut properties = QMap::<QMapPair_QString_QVariant>::default();
        properties.insert(
            QString::from("startFolder"),
            QVariant::from(&QString::from(folder.to_string_lossy().as_ref())),
        );
        engine.as_mut().set_initial_properties(&properties);
        setup::qobject::rustphin_install(engine.as_mut());
        engine.load(&QUrl::from("qrc:/qt/qml/Rustphin/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_argument_wins_then_home_then_the_root() {
        let home = Some(PathBuf::from("/home/someone"));
        assert_eq!(
            start_folder(Some("/srv".into()), home.clone()),
            PathBuf::from("/srv")
        );
        assert_eq!(start_folder(None, home), PathBuf::from("/home/someone"));
        assert_eq!(start_folder(None, None), PathBuf::from("/"));
    }
}
