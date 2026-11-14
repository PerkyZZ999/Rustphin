//! The cxx-qt bridge for a window's tabs: declarations only.

pub use super::TabsModelRust;

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
        include!("cxx-qt-lib/qstringlist.h");
        /// A list of texts.
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qhash.h");
        /// Role numbers and their names.
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qlist.h");
        /// Role numbers.
        type QList_i32 = cxx_qt_lib::QList<i32>;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// A window's tabs (W-114e): each with one pane or two side by
        /// side, the active one, closed tabs for Ctrl+Shift+T, and the
        /// session saved as it changes in the start window.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count)]
        #[qproperty(i32, active)]
        #[qproperty(bool, saves)]
        type TabsModel = super::TabsModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        fn begin_insert_rows(
            self: Pin<&mut TabsModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endInsertRows"]
        fn end_insert_rows(self: Pin<&mut TabsModel>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        fn begin_remove_rows(
            self: Pin<&mut TabsModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        fn end_remove_rows(self: Pin<&mut TabsModel>);
        #[inherit]
        #[cxx_name = "beginMoveRows"]
        fn begin_move_rows(
            self: Pin<&mut TabsModel>,
            source_parent: &QModelIndex,
            source_first: i32,
            source_last: i32,
            destination_parent: &QModelIndex,
            destination_child: i32,
        ) -> bool;
        #[inherit]
        #[cxx_name = "endMoveRows"]
        fn end_move_rows(self: Pin<&mut TabsModel>);
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut TabsModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );
        #[inherit]
        fn index(self: &TabsModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &TabsModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &TabsModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &TabsModel) -> QHash_i32_QByteArray;

        /// Opens the window's first tabs: `folders` each in a tab (the names
        /// in `select` selected in the first), or with none, the saved
        /// session when this is the start window and Settings restores it,
        /// else `home`.
        #[qinvokable]
        fn restore(
            self: Pin<&mut TabsModel>,
            folders: &QStringList,
            select: &QStringList,
            start: bool,
            home: &QString,
        );
        /// Opens a tab on `location` after the active one; `activate`
        /// makes it the active tab. Names in `select` are selected.
        #[qinvokable]
        fn open(
            self: Pin<&mut TabsModel>,
            location: &QString,
            activate: bool,
            select: &QStringList,
        );
        /// Closes the tab at `index`; false when it is the last (close the window).
        #[qinvokable]
        fn close(self: Pin<&mut TabsModel>, index: i32) -> bool;
        /// Opens the tab closed last again; false when there is none.
        #[qinvokable]
        fn reopen(self: Pin<&mut TabsModel>) -> bool;
        /// Makes the tab at `index` active.
        #[qinvokable]
        fn activate(self: Pin<&mut TabsModel>, index: i32);
        /// Alt+number: 1–9 pick that tab, 0 the last.
        #[qinvokable]
        #[cxx_name = "activateNumber"]
        fn activate_number(self: Pin<&mut TabsModel>, number: i32);
        /// The next (+1) or previous (−1) tab, wrapping.
        #[qinvokable]
        fn cycle(self: Pin<&mut TabsModel>, step: i32);
        /// Moves the active tab left (−1) or right (+1).
        #[qinvokable]
        #[cxx_name = "shiftActive"]
        fn shift_active(self: Pin<&mut TabsModel>, step: i32);
        /// Moves the tab at `from` to `to` (a drag in the strip).
        #[qinvokable]
        #[cxx_name = "moveTab"]
        fn move_tab(self: Pin<&mut TabsModel>, from: i32, to: i32);
        /// A pane shows a new folder: its path, title and icon.
        #[qinvokable]
        #[cxx_name = "setPane"]
        fn set_pane(
            self: Pin<&mut TabsModel>,
            key: i32,
            side: i32,
            path: &QString,
            title: &QString,
            icon: &QString,
        );
        /// The pane on `side` of tab `key` has the keyboard.
        #[qinvokable]
        #[cxx_name = "setActivePane"]
        fn set_active_pane(self: Pin<&mut TabsModel>, key: i32, side: i32);
        /// Splits the active tab into two panes, or joins them (F3).
        #[qinvokable]
        #[cxx_name = "toggleSplit"]
        fn toggle_split(self: Pin<&mut TabsModel>);
        /// The row of the tab `key`; −1 when it is gone.
        #[qinvokable]
        #[cxx_name = "rowOf"]
        fn row_of(self: &TabsModel, key: i32) -> i32;
        /// The tabs for the palette, as JSON: title and path of each.
        #[qinvokable]
        #[cxx_name = "paletteJson"]
        fn palette_json(self: &TabsModel) -> QString;
        /// Copies (or moves) the active view's selection to the other view.
        #[qinvokable]
        #[cxx_name = "transfer"]
        fn transfer(self: Pin<&mut TabsModel>, selected: &QStringList, moving: bool);
    }
}
