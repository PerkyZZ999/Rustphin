//! The cxx-qt bridge for the operations object: declarations only.

pub use super::OperationsRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// File operations as a window shows them: the progress line, the
        /// question waiting for an answer, the latest notice.
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, progress_visible, cxx_name = "progressVisible")]
        #[qproperty(QString, progress_label, cxx_name = "progressLabel")]
        #[qproperty(QString, progress_detail, cxx_name = "progressDetail")]
        #[qproperty(QString, progress_icon, cxx_name = "progressIcon")]
        #[qproperty(f64, progress_fraction, cxx_name = "progressFraction")]
        #[qproperty(i32, running_count, cxx_name = "runningCount")]
        #[qproperty(QString, question)]
        #[qproperty(QString, notice)]
        #[qproperty(bool, notice_error, cxx_name = "noticeError")]
        #[qproperty(bool, notice_undo, cxx_name = "noticeUndo")]
        #[qproperty(QString, problems)]
        #[qproperty(bool, can_paste, cxx_name = "canPaste")]
        #[qproperty(i32, revision)]
        #[qproperty(bool, admin)]
        #[qproperty(bool, admin_pending, cxx_name = "adminPending")]
        #[qproperty(bool, admin_installed, cxx_name = "adminInstalled")]
        type Operations = super::OperationsRust;
    }

    unsafe extern "RustQt" {
        /// Follows the hub; call once from QML.
        #[qinvokable]
        fn watch(self: Pin<&mut Operations>);
        /// Takes up the hub's state.
        fn refresh(self: Pin<&mut Operations>);
        /// Stops the operation shown.
        #[qinvokable]
        fn cancel(self: Pin<&mut Operations>);
        /// Reverts the latest operation.
        #[qinvokable]
        fn undo(self: Pin<&mut Operations>);
        /// Answers the question shown (`questions::Answer` numbers).
        #[qinvokable]
        fn answer(self: Pin<&mut Operations>, answer: i32, apply_to_all: bool, password: &QString);
        /// Starts or stops acting as administrator.
        #[qinvokable]
        #[cxx_name = "toggleAdmin"]
        fn toggle_admin(self: Pin<&mut Operations>);
        /// Dismisses the notice.
        #[qinvokable]
        #[cxx_name = "dismissNotice"]
        fn dismiss_notice(self: Pin<&mut Operations>);
        /// Copies or moves dropped `paths` into `into`: 0 asks, 1 copies, 2 moves.
        #[qinvokable]
        #[cxx_name = "dropFiles"]
        fn drop_files(self: Pin<&mut Operations>, uris: &QString, into: &QString, action: i32);
    }

    impl cxx_qt::Threading for Operations {}
}
