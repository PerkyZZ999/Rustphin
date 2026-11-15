//! The cxx-qt bridge for the places panel's model: declarations only.

pub use super::PlacesModelRust;

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
        include!("cxx-qt-lib/qlist.h");
        /// Role numbers.
        type QList_i32 = cxx_qt_lib::QList<i32>;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// The places panel (W-114e): places, groups, network places and
        /// volumes, one row each, and what can be done with them.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count)]
        #[qproperty(bool, has_places, cxx_name = "hasPlaces")]
        #[qproperty(i32, focus_row, cxx_name = "focusRow")]
        #[qproperty(i32, renaming_row, cxx_name = "renamingRow")]
        #[qproperty(QString, rename_text, cxx_name = "renameText")]
        #[qproperty(QString, devices_note, cxx_name = "devicesNote")]
        #[qproperty(QString, menu)]
        #[qproperty(i32, width)]
        // A drive's Properties shown, as JSON
        // (`rustphin_view::drive_properties::DriveSheet`); empty when none.
        #[qproperty(QString, drive)]
        #[qproperty(QString, drive_style, cxx_name = "driveStyle")]
        type PlacesModel = super::PlacesModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut PlacesModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut PlacesModel>);
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut PlacesModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );
        #[inherit]
        fn index(self: &PlacesModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &PlacesModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &PlacesModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &PlacesModel) -> QHash_i32_QByteArray;

        /// Show this folder, by its `file://` URI, in the active pane, or
        /// in a new tab.
        #[qsignal]
        #[cxx_name = "openFolder"]
        fn open_folder(self: Pin<&mut PlacesModel>, uri: QString, new_tab: bool);
        /// Show the trash in the active pane.
        #[qsignal]
        #[cxx_name = "openTrash"]
        fn open_trash(self: Pin<&mut PlacesModel>);
        /// Put `text` on the clipboard (a drive's Copy location, W-148).
        #[qsignal]
        #[cxx_name = "copyText"]
        fn copy_text(self: Pin<&mut PlacesModel>, text: QString);

        /// Starts reading the places, groups and volumes; once.
        #[qinvokable]
        fn start(self: Pin<&mut PlacesModel>);
        /// The folder the active pane shows, by its `file:` URI (empty for
        /// none), to mark its place.
        #[qinvokable]
        #[cxx_name = "setCurrent"]
        fn set_current(self: Pin<&mut PlacesModel>, uri: &QString);
        /// Opens the row: a place, a volume (mounting it), or folds a heading.
        #[qinvokable]
        fn activate(self: Pin<&mut PlacesModel>, row: i32, new_tab: bool);
        /// Gives the keyboard's focus to `row`.
        #[qinvokable]
        #[cxx_name = "focusOn"]
        fn focus_on(self: Pin<&mut PlacesModel>, row: i32);
        /// Moves the keyboard's focus by `delta` rows.
        #[qinvokable]
        #[cxx_name = "moveFocus"]
        fn move_focus(self: Pin<&mut PlacesModel>, delta: i32);
        /// Runs the command `id` (`rustphin_view::commands`) when it acts on
        /// the places panel's focused row; true when it was used.
        #[qinvokable]
        fn command(self: Pin<&mut PlacesModel>, id: &QString) -> bool;
        /// Builds the right-click menu for `row` (-1: the empty space) into `menu`.
        #[qinvokable]
        #[cxx_name = "openMenu"]
        fn open_menu(self: Pin<&mut PlacesModel>, row: i32) -> bool;
        /// Runs the menu entry at `path`; an empty path closes the menu.
        #[qinvokable]
        #[cxx_name = "runMenu"]
        fn run_menu(self: Pin<&mut PlacesModel>, path: &QString);
        /// Ends renaming; with `apply`, the row takes `text`.
        #[qinvokable]
        #[cxx_name = "finishRename"]
        fn finish_rename(self: Pin<&mut PlacesModel>, text: &QString, apply: bool);
        /// Adds the folder at the `file:` URI `uri` to the places
        /// (Ctrl+Shift+D).
        #[qinvokable]
        #[cxx_name = "addFolder"]
        fn add_folder(self: Pin<&mut PlacesModel>, uri: &QString);
        /// Drops the place at row `from` onto row `to` (reordering, grouping).
        #[qinvokable]
        #[cxx_name = "dropRow"]
        fn drop_row(self: Pin<&mut PlacesModel>, from: i32, to: i32);
        /// Drops dragged folders (a `text/uri-list`) on row `to` (-1: the end).
        #[qinvokable]
        #[cxx_name = "dropFolders"]
        fn drop_folders(self: Pin<&mut PlacesModel>, uris: &QString, to: i32);
        /// The places offered by the palette, as JSON: title, `file://`
        /// URI, icon.
        #[qinvokable]
        #[cxx_name = "paletteJson"]
        fn palette_json(self: &PlacesModel) -> QString;
        /// The Lucide icon of the place that is exactly the folder at the
        /// `file:` URI `uri`, if any.
        #[qinvokable]
        #[cxx_name = "iconFor"]
        fn icon_for(self: &PlacesModel, uri: &QString) -> QString;
        /// The groups as JSON (name and count), for Settings.
        #[qinvokable]
        #[cxx_name = "groupsJson"]
        fn groups_json(self: &PlacesModel) -> QString;
        /// Moves group `index` by `delta` (Settings, Places).
        #[qinvokable]
        #[cxx_name = "shiftGroup"]
        fn shift_group(self: Pin<&mut PlacesModel>, index: i32, delta: i32);
        /// Deletes group `index`; its places stay (Settings, Places).
        #[qinvokable]
        #[cxx_name = "deleteGroup"]
        fn delete_group(self: Pin<&mut PlacesModel>, index: i32);
        /// Renames group `index` (Settings, Places).
        #[qinvokable]
        #[cxx_name = "renameGroup"]
        fn rename_group(self: Pin<&mut PlacesModel>, index: i32, name: &QString);
        /// The panel was dragged to `width` pixels; Settings keeps it.
        #[qinvokable]
        fn resize(self: Pin<&mut PlacesModel>, width: f64);
        /// The drive's Properties were closed.
        #[qinvokable]
        #[cxx_name = "closeDrive"]
        fn close_drive(self: Pin<&mut PlacesModel>);
        /// Takes up changed settings (which sections show).
        #[qinvokable]
        #[cxx_name = "adoptSettings"]
        fn adopt_settings(self: Pin<&mut PlacesModel>);
    }

    impl cxx_qt::Threading for PlacesModel {}
}
