//! The cxx-qt bridge for the command palette's model: declarations only.

pub use super::PaletteModelRust;

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

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// The command palette (Ctrl+Shift+P): commands, tabs, places and
        /// recent folders, ranked by what is typed.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count)]
        #[qproperty(i32, selected)]
        #[qproperty(QString, status)]
        type PaletteModel = super::PaletteModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut PaletteModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut PaletteModel>);

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &PaletteModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &PaletteModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &PaletteModel) -> QHash_i32_QByteArray;

        /// Gathers what can be chosen: the commands, the window's tabs and
        /// places (JSON lists of title and path), the recent folders.
        #[qinvokable]
        fn gather(self: Pin<&mut PaletteModel>, tabs: &QString, places: &QString);
        /// Ranks everything by `query`, in the background.
        #[qinvokable]
        fn search(self: Pin<&mut PaletteModel>, query: &QString);
        /// Moves the selection by `delta` rows, wrapping.
        #[qinvokable]
        #[cxx_name = "moveBy"]
        fn move_by(self: Pin<&mut PaletteModel>, delta: i32);
        /// Selects `row`.
        #[qinvokable]
        fn select(self: Pin<&mut PaletteModel>, row: i32);
        /// What the selected row does, as JSON: `{kind, id, index, path}`;
        /// empty while the results are not ready.
        #[qinvokable]
        fn chosen(self: &PaletteModel) -> QString;
        /// Forgets the recent folders (the palette's last row).
        #[qinvokable]
        #[cxx_name = "clearHistory"]
        fn clear_history(self: Pin<&mut PaletteModel>);
    }

    impl cxx_qt::Threading for PaletteModel {}
}
