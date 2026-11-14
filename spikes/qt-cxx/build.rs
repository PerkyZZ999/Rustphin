use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // SAFETY (build script): only adds include paths for KDE's headers.
    let builder = unsafe {
        CxxQtBuilder::new_qml_module(QmlModule::new("spike").qml_files(["qml/Main.qml"]))
            .qt_module("Gui")
            .qt_module("Quick")
            .file("src/bridge.rs")
            .cpp_file("cpp/helpers.cpp")
            .include_dir("cpp")
            .cc_builder(|cc| {
                cc.include("/usr/include/KF6/KWindowSystem");
            })
    };
    builder.build();
    println!("cargo::rustc-link-lib=KF6WindowSystem");
    println!("cargo::rerun-if-changed=cpp/helpers.cpp");
    println!("cargo::rerun-if-changed=cpp/helpers.h");
}
