//! The cxx-qt bridge for the Compress and batch rename dialogs:
//! declarations only.

pub use super::{BatchModelRust, CompressModelRust};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// Compress to… (W-111d): name, format, level, and what it makes.
        #[qobject]
        #[qml_element]
        #[qproperty(QString, title)]
        #[qproperty(QString, name)]
        #[qproperty(i32, format)]
        #[qproperty(i32, level)]
        #[qproperty(QString, note)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, open)]
        type CompressModel = super::CompressModelRust;

        /// Batch rename (S9): the rule, its fields and the preview as JSON.
        #[qobject]
        #[qml_element]
        #[qproperty(QString, view)]
        #[qproperty(i32, mode)]
        #[qproperty(i32, case_change, cxx_name = "caseChange")]
        #[qproperty(bool, open)]
        type BatchModel = super::BatchModelRust;
    }

    unsafe extern "RustQt" {
        /// Takes the dialog the pane kept under `ticket` and opens.
        #[qinvokable]
        fn start(self: Pin<&mut CompressModel>, ticket: i32);
        /// The name typed.
        #[qinvokable]
        #[cxx_name = "typeName"]
        fn type_name(self: Pin<&mut CompressModel>, name: &QString);
        /// The format chosen (`FORMATS` order).
        #[qinvokable]
        #[cxx_name = "chooseFormat"]
        fn choose_format(self: Pin<&mut CompressModel>, index: i32);
        /// The level chosen (`LEVELS` order).
        #[qinvokable]
        #[cxx_name = "chooseLevel"]
        fn choose_level(self: Pin<&mut CompressModel>, index: i32);
        /// Starts compressing; false when the name is not usable.
        #[qinvokable]
        fn accept(self: Pin<&mut CompressModel>) -> bool;
        /// Closes without compressing.
        #[qinvokable]
        fn cancel(self: Pin<&mut CompressModel>);

        /// Takes the dialog the pane kept under `ticket` and opens.
        #[qinvokable]
        fn start(self: Pin<&mut BatchModel>, ticket: i32);
        /// One field changed: find, replace, regex, pattern, start, step.
        #[qinvokable]
        fn edit(self: Pin<&mut BatchModel>, field: &QString, value: &QString);
        /// Replace (0), Pattern (1) or Case (2).
        #[qinvokable]
        #[cxx_name = "chooseMode"]
        fn choose_mode(self: Pin<&mut BatchModel>, mode: i32);
        /// Lower (0), upper (1) or capitalised words (2).
        #[qinvokable]
        #[cxx_name = "chooseCase"]
        fn choose_case(self: Pin<&mut BatchModel>, change: i32);
        /// Renames the files; false when the preview is not ready.
        #[qinvokable]
        fn accept(self: Pin<&mut BatchModel>) -> bool;
        /// Closes without renaming.
        #[qinvokable]
        fn cancel(self: Pin<&mut BatchModel>);
    }
}
