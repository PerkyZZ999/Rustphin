//! The cxx-qt bridge for a pane's list model: declarations only, no logic.

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
        /// The entries of the folder one pane shows, and what the pane
        /// decides about them: selection, view, rename, status.
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count)]
        #[qproperty(bool, loading)]
        #[qproperty(QString, location)]
        #[qproperty(QString, title)]
        #[qproperty(QString, total)]
        #[qproperty(QString, selection_text, cxx_name = "selectionText")]
        #[qproperty(QString, free_space, cxx_name = "freeSpace")]
        #[qproperty(QString, problem)]
        #[qproperty(QString, problem_text, cxx_name = "problemText")]
        #[qproperty(QString, problem_status, cxx_name = "problemStatus")]
        #[qproperty(QString, way_label, cxx_name = "wayLabel")]
        #[qproperty(QString, banner)]
        #[qproperty(bool, can_go_back, cxx_name = "canGoBack")]
        #[qproperty(bool, can_go_forward, cxx_name = "canGoForward")]
        #[qproperty(bool, can_go_up, cxx_name = "canGoUp")]
        #[qproperty(bool, show_hidden, cxx_name = "showHidden")]
        #[qproperty(QString, notice)]
        #[qproperty(i32, mode)]
        #[qproperty(i32, zoom)]
        #[qproperty(f64, icon_size, cxx_name = "iconSize")]
        #[qproperty(f64, row_height, cxx_name = "rowHeight")]
        #[qproperty(f64, cell_width, cxx_name = "cellWidth")]
        #[qproperty(f64, name_text, cxx_name = "nameText")]
        #[qproperty(f64, name_line, cxx_name = "nameLine")]
        #[qproperty(f64, data_text, cxx_name = "dataText")]
        #[qproperty(i32, focus_row, cxx_name = "focusRow")]
        #[qproperty(i32, selected_count, cxx_name = "selectedCount")]
        #[qproperty(i32, sort_column, cxx_name = "sortColumn")]
        #[qproperty(bool, descending)]
        #[qproperty(bool, folders_first, cxx_name = "foldersFirst")]
        #[qproperty(bool, show_size, cxx_name = "showSize")]
        #[qproperty(bool, show_modified, cxx_name = "showModified")]
        #[qproperty(bool, show_type, cxx_name = "showType")]
        #[qproperty(bool, shows_free_space, cxx_name = "showsFreeSpace")]
        #[qproperty(bool, shows_totals, cxx_name = "showsTotals")]
        #[qproperty(i32, renaming_row, cxx_name = "renamingRow")]
        #[qproperty(QString, rename_text, cxx_name = "renameText")]
        #[qproperty(i32, rename_stem, cxx_name = "renameStem")]
        #[qproperty(QString, filter_text, cxx_name = "filterText")]
        #[qproperty(bool, trash)]
        #[qproperty(bool, empty)]
        #[qproperty(QString, empty_title, cxx_name = "emptyTitle")]
        #[qproperty(QString, empty_text, cxx_name = "emptyText")]
        #[qproperty(QStringList, crumb_labels, cxx_name = "crumbLabels")]
        #[qproperty(QStringList, crumb_paths, cxx_name = "crumbPaths")]
        #[qproperty(bool, one_click, cxx_name = "oneClick")]
        #[qproperty(bool, frame_log, cxx_name = "frameLog")]
        #[qproperty(QString, menu)]
        #[qproperty(bool, scroll_bench, cxx_name = "scrollBench")]
        type FolderModel = super::FolderModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut FolderModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut FolderModel>);
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut FolderModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );
        #[inherit]
        fn index(self: &FolderModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &FolderModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &FolderModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &FolderModel) -> QHash_i32_QByteArray;

        /// A file was activated: the window opens it (or asks first).
        #[qsignal]
        #[cxx_name = "fileActivated"]
        fn file_activated(self: Pin<&mut FolderModel>, path: QString);
        /// A folder asks to be opened in a new tab (a middle click).
        #[qsignal]
        #[cxx_name = "openInNewTab"]
        fn open_in_new_tab(self: Pin<&mut FolderModel>, path: QString);
        /// The focused entry should be scrolled into view.
        #[qsignal]
        #[cxx_name = "revealRow"]
        fn reveal_row(self: Pin<&mut FolderModel>, row: i32);
        /// A folder was listed, rather than merely requested.
        #[qsignal]
        fn visited(self: Pin<&mut FolderModel>, path: QString);

        /// Shows the folder at `path`.
        #[qinvokable]
        fn open(self: Pin<&mut FolderModel>, path: &QString);
        /// Shows the folder at `path` with the entries `names` selected.
        #[qinvokable]
        fn reveal(self: Pin<&mut FolderModel>, path: &QString, names: &QStringList);
        /// Opens a typed location; false when there was nothing to open.
        #[qinvokable]
        #[cxx_name = "openTyped"]
        fn open_typed(self: Pin<&mut FolderModel>, text: &QString) -> bool;
        /// Goes back in the history.
        #[qinvokable]
        fn back(self: Pin<&mut FolderModel>);
        /// Goes forward in the history.
        #[qinvokable]
        fn forward(self: Pin<&mut FolderModel>);
        /// Opens the parent folder, selecting the one left.
        #[qinvokable]
        fn up(self: Pin<&mut FolderModel>);
        /// Opens the home folder.
        #[qinvokable]
        fn home(self: Pin<&mut FolderModel>);
        /// Lists the folder again.
        #[qinvokable]
        fn reload(self: Pin<&mut FolderModel>);
        /// Opens the entry at `row`: a folder here, a file through the window.
        #[qinvokable]
        fn activate(self: Pin<&mut FolderModel>, row: i32);
        /// Opens every selected entry.
        #[qinvokable]
        #[cxx_name = "activateSelected"]
        fn activate_selected(self: Pin<&mut FolderModel>);
        /// A middle click: a folder opens in a new tab.
        #[qinvokable]
        #[cxx_name = "middleClick"]
        fn middle_click(self: Pin<&mut FolderModel>, row: i32);
        /// Does what the toolbar item `id` (as Settings names it) asks.
        #[qinvokable]
        fn run(self: Pin<&mut FolderModel>, id: &QString);
        /// Sorts by `column` (0 name, 1 size, 2 modified, 3 type); the
        /// current column again reverses the order.
        #[qinvokable]
        #[cxx_name = "sortBy"]
        fn sort_by(self: Pin<&mut FolderModel>, column: i32);
        /// Sorts by `column` in the given direction.
        #[qinvokable]
        #[cxx_name = "setSort"]
        fn set_sort(self: Pin<&mut FolderModel>, column: i32, descending: bool);
        /// Lists folders before files, or mixes them.
        #[qinvokable]
        #[cxx_name = "toggleFoldersFirst"]
        fn toggle_folders_first(self: Pin<&mut FolderModel>);
        /// A click on `row`: alone, with Ctrl (1) toggled, with Shift (2)
        /// the range from the anchor.
        #[qinvokable]
        fn click(self: Pin<&mut FolderModel>, row: i32, modifiers: i32);
        /// A press on `row` that keeps a selection it belongs to (a drag may
        /// follow).
        #[qinvokable]
        fn press(self: Pin<&mut FolderModel>, row: i32, modifiers: i32);
        /// Moves the focus by keyboard (`layout::Step` order), `lanes` to a
        /// line or column; with `extend`, selects the range to it.
        #[qinvokable]
        #[cxx_name = "moveFocus"]
        fn move_focus(self: Pin<&mut FolderModel>, step: i32, extend: bool, lanes: i32);
        /// Selects what a rubber band from (x0, y0) to (x1, y1), in list
        /// coordinates, touches; with `add`, on top of `kept`.
        #[qinvokable]
        fn band(
            self: Pin<&mut FolderModel>,
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
            lanes: i32,
            add: bool,
        );
        /// Remembers the selection a rubber band adds to.
        #[qinvokable]
        #[cxx_name = "beginBand"]
        fn begin_band(self: Pin<&mut FolderModel>);
        /// Selects every entry.
        #[qinvokable]
        #[cxx_name = "selectAll"]
        fn select_all(self: Pin<&mut FolderModel>);
        /// Selects what is not selected, and the other way round.
        #[qinvokable]
        #[cxx_name = "invertSelection"]
        fn invert_selection(self: Pin<&mut FolderModel>);
        /// Selects nothing.
        #[qinvokable]
        #[cxx_name = "clearSelection"]
        fn clear_selection(self: Pin<&mut FolderModel>);
        /// Letters typed over the list jump to a name.
        #[qinvokable]
        #[cxx_name = "typeAhead"]
        fn type_ahead(self: Pin<&mut FolderModel>, text: &QString);
        /// Shows entries as icons (0), compact (1) or details (2).
        #[qinvokable]
        #[cxx_name = "chooseMode"]
        fn choose_mode(self: Pin<&mut FolderModel>, mode: i32);
        /// Zooms in (positive) or out.
        #[qinvokable]
        #[cxx_name = "zoomBy"]
        fn zoom_by(self: Pin<&mut FolderModel>, delta: i32);
        /// Sets the zoom level.
        #[qinvokable]
        #[cxx_name = "chooseZoom"]
        fn choose_zoom(self: Pin<&mut FolderModel>, zoom: i32);
        /// The pane is `width` pixels wide: which columns and status parts fit.
        #[qinvokable]
        #[cxx_name = "setWidth"]
        fn set_width(self: Pin<&mut FolderModel>, width: f64);
        /// Starts renaming the focused entry in place (F2).
        #[qinvokable]
        #[cxx_name = "beginRename"]
        fn begin_rename(self: Pin<&mut FolderModel>);
        /// Ends renaming; with `apply`, the entry takes `text`.
        #[qinvokable]
        #[cxx_name = "finishRename"]
        fn finish_rename(self: Pin<&mut FolderModel>, text: &QString, apply: bool);
        /// Shows only names containing `text`; empty shows everything.
        #[qinvokable]
        #[cxx_name = "setFilter"]
        fn set_filter(self: Pin<&mut FolderModel>, text: &QString);
        /// Takes up changed settings.
        #[qinvokable]
        #[cxx_name = "adoptSettings"]
        fn adopt_settings(self: Pin<&mut FolderModel>);
        /// Shows hidden files, or hides them.
        #[qinvokable]
        #[cxx_name = "setHidden"]
        fn set_hidden(self: Pin<&mut FolderModel>, show: bool);
        /// The selected entries' paths, in list order.
        #[qinvokable]
        #[cxx_name = "selectedPaths"]
        fn selected_paths(self: &FolderModel) -> QStringList;
        /// The path of the entry at `row`.
        #[qinvokable]
        #[cxx_name = "pathAt"]
        fn path_at(self: &FolderModel, row: i32) -> QString;
        /// Whether `row` is selected.
        #[qinvokable]
        #[cxx_name = "isSelected"]
        fn is_selected(self: &FolderModel, row: i32) -> bool;
        /// The selected entries as a `text/uri-list`, for a drag out.
        #[qinvokable]
        #[cxx_name = "selectedUris"]
        fn selected_uris(self: &FolderModel) -> QString;
        /// The row a drop over `row` goes into: `row` when it is a folder
        /// that can receive, else −1 (the drop goes into the folder shown).
        #[qinvokable]
        #[cxx_name = "dropRow"]
        fn drop_row(self: &FolderModel, row: i32) -> i32;
        /// Whether this pane takes drops at all (not the trash).
        #[qinvokable]
        #[cxx_name = "takesDrops"]
        fn takes_drops(self: &FolderModel) -> bool;
        /// Copies or moves the dropped `uris` into the folder at `row`, or
        /// the folder shown for −1. `action`: 0 ask, 1 copy, 2 move.
        #[qinvokable]
        #[cxx_name = "dropOn"]
        fn drop_on(self: Pin<&mut FolderModel>, row: i32, uris: &QString, action: i32);
        /// Says something on the status line until the folder changes.
        #[qinvokable]
        fn say(self: Pin<&mut FolderModel>, words: &QString);
        /// A frame reached the screen (only while frames are logged).
        #[qinvokable]
        #[cxx_name = "frameSwapped"]
        fn frame_swapped(self: Pin<&mut FolderModel>);
        /// One frame of the scroll benchmark.
        #[qinvokable]
        #[cxx_name = "benchTick"]
        fn bench_tick(self: Pin<&mut FolderModel>);
        /// Shows the trash.
        #[qinvokable]
        #[cxx_name = "openTrash"]
        fn open_trash(self: Pin<&mut FolderModel>);
        /// Takes up what the operations hub changed: cut entries, what an
        /// operation made, an administrator's changes.
        #[cxx_name = "hubChanged"]
        fn hub_changed(self: Pin<&mut FolderModel>);
        /// Puts the selection on the clipboard; with `cut`, to be moved.
        #[qinvokable]
        #[cxx_name = "copySelected"]
        fn copy_selected(self: Pin<&mut FolderModel>, cut: bool);
        /// Copies or moves the clipboard's files here.
        #[qinvokable]
        fn paste(self: Pin<&mut FolderModel>);
        /// Moves the selection to the trash (deletes it for good in the trash).
        #[qinvokable]
        #[cxx_name = "trashSelected"]
        fn trash_selected(self: Pin<&mut FolderModel>);
        /// Deletes the selection for good, after asking.
        #[qinvokable]
        #[cxx_name = "deleteSelected"]
        fn delete_selected(self: Pin<&mut FolderModel>);
        /// Copies the selection beside itself (Ctrl+D).
        #[qinvokable]
        #[cxx_name = "duplicateSelected"]
        fn duplicate_selected(self: Pin<&mut FolderModel>);
        /// Brings the selected entries back from the trash shown.
        #[qinvokable]
        #[cxx_name = "restoreSelected"]
        fn restore_selected(self: Pin<&mut FolderModel>);
        /// Empties the trash shown, after asking when Settings says so.
        #[qinvokable]
        #[cxx_name = "emptyTrash"]
        fn empty_trash(self: Pin<&mut FolderModel>);
        /// Runs the palette command `id` (`rustphin_view::commands`) that
        /// acts on this pane, `lanes` entries to a line; false when it is
        /// not a pane command.
        #[qinvokable]
        fn command(self: Pin<&mut FolderModel>, id: &QString, lanes: i32) -> bool;
        /// Builds the context menu into `menu`: for the selection when
        /// `on_selection`, else for the folder; "new" for the New button's
        /// menu, "trash" for the trash's actions. False when empty.
        #[qinvokable]
        #[cxx_name = "openMenu"]
        fn open_menu(self: Pin<&mut FolderModel>, which: &QString) -> bool;
        /// Runs the menu entry at `path`; an empty path closes the menu.
        #[qinvokable]
        #[cxx_name = "runMenu"]
        fn run_menu(self: Pin<&mut FolderModel>, path: &QString);
        /// Asks the window to show these entries' properties.
        #[qsignal]
        #[cxx_name = "propertiesRequested"]
        fn properties_requested(self: Pin<&mut FolderModel>, paths: QStringList);
        /// Asks the window for Compress's dialog (W-111d): the entries, the
        /// folder, the name offered and the names taken, as JSON.
        #[qsignal]
        #[cxx_name = "compressRequested"]
        fn compress_requested(self: Pin<&mut FolderModel>, json: QString);
        /// Asks the window for batch rename (several selected, F2), as JSON.
        #[qsignal]
        #[cxx_name = "batchRenameRequested"]
        fn batch_rename_requested(self: Pin<&mut FolderModel>, json: QString);
        /// Asks the window to choose a folder to extract into (W-113).
        #[qsignal]
        #[cxx_name = "extractToRequested"]
        fn extract_to_requested(self: Pin<&mut FolderModel>, json: QString);
        /// Asks the window to put text on the clipboard.
        #[qsignal]
        #[cxx_name = "copyText"]
        fn copy_text(self: Pin<&mut FolderModel>, text: QString);
        /// Makes an empty folder (or file) with a free name and renames it.
        #[qinvokable]
        #[cxx_name = "createEntry"]
        fn create_entry(self: Pin<&mut FolderModel>, folder: bool);
    }

    impl cxx_qt::Threading for FolderModel {}
}
