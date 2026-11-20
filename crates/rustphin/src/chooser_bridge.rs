//! The cxx-qt bridge for the application chooser: declarations only.

pub use super::ChooserModelRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// Open with › Other application… (W-169): every installed
        /// application, those for the file's type first, as JSON.
        #[qobject]
        #[qml_element]
        #[qproperty(QString, view)]
        #[qproperty(bool, remembering)]
        #[qproperty(bool, open)]
        type ChooserModel = super::ChooserModelRust;
    }

    impl cxx_qt::Threading for ChooserModel {}

    unsafe extern "RustQt" {
        /// Opens the chooser for the file at the `file:` URI `uri`;
        /// `remember` starts the "always" box ticked.
        #[qinvokable]
        fn start(self: Pin<&mut ChooserModel>, uri: &QString, remember: bool);
        /// The search typed.
        #[qinvokable]
        fn search(self: Pin<&mut ChooserModel>, text: &QString);
        /// Chooses the application with desktop-file id `id`.
        #[qinvokable]
        fn choose(self: Pin<&mut ChooserModel>, id: &QString);
        /// Ticks or clears the "always" box.
        #[qinvokable]
        fn keep(self: Pin<&mut ChooserModel>, on: bool);
        /// Opens the file with the chosen application; false when none is.
        #[qinvokable]
        fn accept(self: Pin<&mut ChooserModel>) -> bool;
        /// Closes without opening anything.
        #[qinvokable]
        fn cancel(self: Pin<&mut ChooserModel>);
    }
}
