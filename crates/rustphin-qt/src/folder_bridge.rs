//! The cxx-qt bridge for the list model: declarations only, no logic.

pub use super::FolderModelRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        /// Qt's list model base class.
        type QAbstractListModel;

        include!("cxx-qt-lib/qmodelindex.h");
        /// A row of a model.
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        /// Any value QML can read.
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qhash.h");
        /// Role numbers and their names.
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
    }

    extern "RustQt" {
        /// The entries of the folder being shown.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count, READ, NOTIFY)]
        #[qproperty(bool, loading, READ, NOTIFY)]
        #[qproperty(QString, location, READ, NOTIFY)]
        #[qproperty(QString, summary, READ, NOTIFY)]
        #[qproperty(QString, problem, READ, NOTIFY)]
        #[qproperty(bool, can_go_back, cxx_name = "canGoBack", READ, NOTIFY)]
        #[qproperty(bool, can_go_forward, cxx_name = "canGoForward", READ, NOTIFY)]
        #[qproperty(bool, can_go_up, cxx_name = "canGoUp", READ, NOTIFY)]
        #[qproperty(bool, show_hidden, cxx_name = "showHidden", READ, NOTIFY)]
        #[qproperty(QString, notice, READ, NOTIFY)]
        type FolderModel = super::FolderModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut FolderModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut FolderModel>);

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &FolderModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &FolderModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &FolderModel) -> QHash_i32_QByteArray;

        /// Shows the folder at `path`.
        #[qinvokable]
        fn open(self: Pin<&mut FolderModel>, path: &QString);
        /// Sorts by `column` (0 name, 1 size, 2 modified, 3 type).
        #[qinvokable]
        #[cxx_name = "sortBy"]
        fn sort_by(self: Pin<&mut FolderModel>, column: i32, descending: bool);
        /// Goes back in the history.
        #[qinvokable]
        fn back(self: Pin<&mut FolderModel>);
        /// Goes forward in the history.
        #[qinvokable]
        fn forward(self: Pin<&mut FolderModel>);
        /// Opens the parent folder.
        #[qinvokable]
        fn up(self: Pin<&mut FolderModel>);
        /// Opens the home folder.
        #[qinvokable]
        fn home(self: Pin<&mut FolderModel>);
        /// Lists the folder again.
        #[qinvokable]
        fn reload(self: Pin<&mut FolderModel>);
        /// Opens the entry at `row` if it is a folder.
        #[qinvokable]
        fn activate(self: Pin<&mut FolderModel>, row: i32);
        /// Does what the toolbar item `id` (as Settings names it) asks.
        #[qinvokable]
        fn run(self: Pin<&mut FolderModel>, id: &QString);
    }

    impl cxx_qt::Threading for FolderModel {}
}
