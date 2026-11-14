//! Builds the QML module, its resources (Lucide icons, the bundled fonts)
//! and the C++ side of the bridges. Qt 6 is found through `qmake6` (or
//! `QMAKE`); KDE's KWindowSystem gives the blur.

use std::path::Path;

use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlModule};
use qt_build_utils::QResourceFile;

/// The small logo the title bar shows (the GPUI title bar's too).
const LOGO: &str = "../../assets/rustphin-small.svg";

/// The fonts the GPUI interface bundles; the Qt one uses the same files.
const FONTS: &str = "../rustphin-ui/assets/fonts";

/// Every `.ttf` file in `dir` aliased as `fonts/<name>`, so QML and C++
/// find them under the module's `fonts/` folder.
fn fonts(dir: &Path) -> Vec<QResourceFile> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "ttf"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            Some(QResourceFile::new(&path).alias(format!("fonts/{name}")))
        })
        .collect()
}

/// Every `.qml` file in `qml/`.
fn qml_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir("qml")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.ends_with(".qml").then(|| format!("qml/{name}"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Every `.svg` file in `icons/`.
fn icons() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir("icons")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.ends_with(".svg").then(|| format!("icons/{name}"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

// cxx-qt-build marks changing the C++ compiler's options unsafe, since some
// options break the ABI between Rust and C++. Only an include path is added.
#[allow(unsafe_code)]
fn main() {
    let resources = QResources::new()
        .resource(QResource::new().files(icons()))
        .resource(QResource::new().files(fonts(Path::new(FONTS))))
        .resource(QResource::new().file(QResourceFile::new(LOGO).alias("logo.svg")));
    let builder = CxxQtBuilder::new_qml_module(QmlModule::new("Rustphin").qml_files(qml_files()))
        .qt_module("Gui")
        .qt_module("Quick")
        .qt_module("Svg")
        .files([
            "src/app_bridge.rs",
            "src/desktop_bridge.rs",
            "src/folder_bridge.rs",
            "src/operations_bridge.rs",
            "src/tokens_bridge.rs",
            "src/toolbar_bridge.rs",
        ])
        .cpp_file("cpp/app.cpp")
        .cpp_file("cpp/window.cpp")
        .include_dir("cpp")
        .qrc_resources(resources);
    // SAFETY: adds KDE's KWindowSystem headers to the include path; no option
    // that changes how C++ code is compiled or linked.
    let builder = unsafe {
        builder.cc_builder(|cc| {
            cc.include("/usr/include/KF6/KWindowSystem");
        })
    };
    builder.build();
    println!("cargo::rustc-link-lib=KF6WindowSystem");
    println!("cargo::rerun-if-changed=cpp");
    println!("cargo::rerun-if-changed=icons");
    println!("cargo::rerun-if-changed=qml");
    println!("cargo::rerun-if-changed={FONTS}");
    println!("cargo::rerun-if-changed={LOGO}");
}
