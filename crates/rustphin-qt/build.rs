//! Builds the QML module and the C++ side of the bridges. Qt 6 is found
//! through `qmake6` (or `QMAKE`); KDE's KWindowSystem gives the blur.

use cxx_qt_build::{CxxQtBuilder, QmlModule};

// cxx-qt-build marks changing the C++ compiler's options unsafe, since some
// options break the ABI between Rust and C++. Only an include path is added.
#[allow(unsafe_code)]
fn main() {
    let builder =
        CxxQtBuilder::new_qml_module(QmlModule::new("Rustphin").qml_files(["qml/Main.qml"]))
            .qt_module("Gui")
            .qt_module("Quick")
            .files(["src/folder_bridge.rs", "src/tokens_bridge.rs"])
            .cpp_file("cpp/window.cpp")
            .include_dir("cpp");
    // SAFETY: adds KDE's KWindowSystem headers to the include path; no option
    // that changes how C++ code is compiled or linked.
    let builder = unsafe {
        builder.cc_builder(|cc| {
            cc.include("/usr/include/KF6/KWindowSystem");
        })
    };
    builder.build();
    println!("cargo::rustc-link-lib=KF6WindowSystem");
    println!("cargo::rerun-if-changed=cpp/window.cpp");
    println!("cargo::rerun-if-changed=cpp/window.h");
}
