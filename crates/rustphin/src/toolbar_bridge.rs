//! The cxx-qt bridge for the toolbar: declarations only, no logic.

pub use super::ToolbarModelRust;

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
        include!("cxx-qt-lib/qhash.h");
        /// Role numbers and their names.
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
    }

    extern "RustQt" {
        /// The toolbar's items, as Settings arranges them, and which fit.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(bool, actions_words, cxx_name = "actionsWords", READ, NOTIFY)]
        #[qproperty(bool, actions_hint, cxx_name = "actionsHint", READ, NOTIFY)]
        type ToolbarModel = super::ToolbarModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut ToolbarModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
        );
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut ToolbarModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut ToolbarModel>);
        #[inherit]
        fn index(self: &ToolbarModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &ToolbarModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &ToolbarModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &ToolbarModel) -> QHash_i32_QByteArray;

        /// Decides what is drawn in a toolbar `width` pixels wide.
        #[qinvokable]
        fn fit(self: Pin<&mut ToolbarModel>, width: f64);
        /// Takes up the toolbar Settings arranges now, when it changed, and
        /// fits it to `width` pixels.
        #[qinvokable]
        fn reload(self: Pin<&mut ToolbarModel>, width: f64);
    }
}
