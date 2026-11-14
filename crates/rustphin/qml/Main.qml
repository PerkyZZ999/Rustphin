// Rustphin's Qt window: its own title bar, the toolbar, the pane, the
// progress and notice bar and the operations' questions, drawn with the
// tokens of docs/DESIGN.md. State and decisions stay in Rust; this file
// lays out, binds and forwards keys.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import Rustphin

Window {
    id: root

    // The folders this window opens in tabs; none for a plain window.
    required property var startFolders
    // Names selected in the first of them (another application asked).
    property var startSelect: []
    // The window a launch opened first: it restores and saves the session.
    required property bool startWindow
    required property string home
    readonly property bool maximised: visibility === Window.Maximized || visibility === Window.FullScreen
    readonly property var page: pages.count > tabs.active ? pages.itemAt(tabs.active) : null
    readonly property var activePane: page ? page.activeItem : null
    readonly property FolderModel folder: activePane ? activePane.listing : null
    property bool placesShown: true
    // The window's tabs and places, under names delegates do not shadow.
    readonly property TabsModel tabsModel: tabs
    readonly property PlacesModel placesModel: places.model
    property bool closeAsked: false

    // Opens `folders` in new tabs; the last becomes active (another launch).
    function openTabs(folders) {
        for (const folder of folders)
            tabs.open(folder, true, [])
        requestActivate()
    }

    // Another application asked to show `folder` with `names` selected.
    function reveal(folder, names) {
        tabs.open(folder, true, names)
        requestActivate()
    }

    function focusActive() {
        if (activePane)
            activePane.focusList()
    }

    function closeTab(index: int) {
        if (!tabs.close(index))
            root.close()
        else
            Qt.callLater(focusActive)
    }

    width: 1000
    height: 700
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint
    color: Tokens.translucent ? "transparent" : Tokens.colors.window
    title: qsTr("%1 — Rustphin").arg(folder ? folder.title : "")

    // Frame timing for R-101 (RUSTPHIN_FRAME_LOG, RUSTPHIN_SCROLL_BENCH).
    onFrameSwapped: {
        if (root.folder && root.folder.frameLog)
            root.folder.frameSwapped()
    }
    FrameAnimation {
        running: root.folder !== null && root.folder.scrollBench && root.folder.count > 1000
                 && !root.folder.loading && root.activePane.view !== null
        onTriggered: {
            const view = root.activePane.view
            // The compact view scrolls sideways, the others down.
            if (root.folder.mode === 1) {
                if (view.atXEnd)
                    view.contentX = view.originX
                else
                    view.contentX += root.folder.cellWidth / 8
            } else if (view.atYEnd) {
                view.contentY = view.originY
            } else {
                view.contentY += root.folder.rowHeight / 2
            }
            root.folder.benchTick()
        }
    }

    Component.onCompleted: {
        Tokens.blurWindows()
        Operations.watch()
        Preferences.watch()
        tabs.restore(startFolders, startSelect, startWindow, home)
        Qt.callLater(focusActive)
    }

    // Several tabs close together only after asking, when Settings says so.
    onClosing: close => {
        if (tabs.count > 1 && Shell.askBeforeClosingTabs() && !closeAsked) {
            close.accepted = false
            closeQuestion.asking = true
        }
    }

    TabsModel {
        id: tabs

        onActiveChanged: Qt.callLater(root.focusActive)
    }

    // The context menu of `pane`: on its selection, or for its folder.
    function paneMenu(pane, row: int, x: real, y: real) {
        menuPane = pane
        if (pane.listing.openMenu(row >= 0 ? "selection" : "folder"))
            paneMenuView.openAt(pane, x, y)
    }

    property var menuPane: null

    // Runs a palette command (`rustphin_view::commands`) by its id: the
    // pane's own, the places panel's, or the window's.
    function runCommand(id: string) {
        const placeKeys = {
            "new-group": "Ctrl+Shift+G", "rename-row": "F2", "remove-row": "Delete",
            "hide-row": "H", "toggle-hidden-places": "ToggleHiddenPlaces",
            "move-row-up": "Alt+Up", "move-row-down": "Alt+Down", "collapse": "Left",
            "expand": "Right", "next-group": "G", "focus-previous": "Up",
            "focus-next": "Down", "open": "Enter", "eject": "E"
        }
        if (placeKeys[id] !== undefined) {
            placesShown = true
            places.model.key(placeKeys[id])
            places.focusPanel()
            return
        }
        if (activePane && folder.command(id, activePane.lanes()))
            return
        switch (id) {
        case "edit-location": toolbar.editLocation(); break
        case "show-filter": command("filter", null); break
        case "copy-to-other-view": tabs.transfer(folder.selectedPaths(), false); break
        case "move-to-other-view": tabs.transfer(folder.selectedPaths(), true); break
        case "undo": Operations.undo(); break
        case "stop-operations": Operations.cancel(); break
        case "dismiss-notice": Operations.dismissNotice(); break
        case "trash-actions": command("trash", toolbar); break
        case "new-tab": tabs.open(folder ? folder.location : home, true, []); break
        case "reopen-closed-tab": tabs.reopen(); break
        case "focus-other-view":
            if (page && page.split)
                tabs.setActivePane(page.key, page.activePane === 1 ? 0 : 1)
            break
        case "focus-places":
            placesShown = true
            places.focusPanel()
            break
        case "quit": Shell.quit(); break
        case "open-settings": openSettings(); break
        case "close-tab": closeTab(tabs.active); break
        case "next-tab": tabs.cycle(1); break
        case "previous-tab": tabs.cycle(-1); break
        case "move-tab-left": tabs.shiftActive(-1); break
        case "move-tab-right": tabs.shiftActive(1); break
        case "toggle-split": tabs.toggleSplit(); break
        case "other-pane": otherPane(); break
        case "toggle-sidebar": placesShown = !placesShown; break
        case "add-place": places.model.addFolder(folder.location); break
        case "open-palette": openPalette(); break
        }
    }

    // Opens a dialog a pane asked for.
    function openDialog(kind: string, data) {
        if (kind === "properties") {
            if (data.length > 0) {
                propertiesLoader.path = data[0]
                propertiesLoader.active = true
            }
        } else if (kind === "compress") {
            compressLoader.active = true
        } else if (kind === "batch") {
            batchLoader.active = true
        } else if (kind === "extractTo") {
            extractFolder.currentFolder = data.uri
            // A window of its own: opened once the menu that asked is gone,
            // or the menu is left open behind it and stops answering.
            Qt.callLater(() => extractFolder.open())
        }
    }

    function openPalette() {
        palette.show(tabs.paletteJson(), places.model.paletteJson())
    }

    // Tab: the next view, then the places panel, then back (W-076).
    function otherPane() {
        if (places.activeFocus) {
            if (page)
                tabs.setActivePane(page.key, 0)
            focusActive()
        } else if (page && page.split && page.activePane === 0) {
            tabs.setActivePane(page.key, 1)
            focusActive()
        } else if (placesShown) {
            places.focusPanel()
        } else if (page && page.split) {
            tabs.setActivePane(page.key, 0)
            focusActive()
        }
    }

    function openSettings() {
        settingsLoader.active = true
    }

    // Runs a toolbar or menu command the pane cannot run itself.
    function command(id: string, source: Item) {
        if (id === "new" || id === "trash") {
            menuPane = activePane
            if (folder.openMenu(id))
                paneMenuView.openAt(source, 0, source.height)
        } else if (id === "split") {
            tabs.toggleSplit()
        } else if (id === "admin") {
            Operations.toggleAdmin()
        } else if (id === "actions") {
            openPalette()
        } else if (id.indexOf("command:") === 0) {
            runCommand(id.slice(8))
        } else if (id === "view") {
            const p = source.mapToItem(root.contentItem, 0, source.height)
            viewMenu.x = p.x
            viewMenu.y = p.y
            viewMenu.open()
        } else if (id === "filter") {
            if (activePane.filterOpen)
                activePane.closeFilter()
            else
                activePane.showFilter()
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Tokens.colors.window
        // Without the compositor's frame, a hairline marks the window's edge.
        border.width: root.maximised ? 0 : 1
        border.color: Tokens.colors.border_strong
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: root.maximised ? 0 : 1
        spacing: 0

        TitleBar {
            Layout.fillWidth: true
            window: root
        }
        Toolbar {
            id: toolbar

            Layout.fillWidth: true
            pane: root.activePane
            onCommand: (id, source) => root.command(id, source)
        }
        // Acting as administrator: said across the window, with the way back.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.admin_bar
            visible: Operations.admin
            color: Tokens.colors.danger_surface
            border.color: Tokens.colors.danger_border
            Accessible.role: Accessible.AlertMessage
            Accessible.name: adminText.text

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_sm
                spacing: Tokens.metrics.space_md

                Lucide {
                    name: "shield-alert"
                    colour: Tokens.colors.danger
                }
                Text {
                    id: adminText

                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: Tokens.colors.danger
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: qsTr("Acting as administrator. Changes you make here can affect the whole system; be careful.")
                }
                OutlinedButton {
                    text: qsTr("Stop acting as administrator")
                    iconName: "shield-off"
                    implicitHeight: Tokens.metrics.small_button
                    onClicked: Operations.toggleAdmin()
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            PlacesPanel {
                id: places

                visible: root.placesShown
                Layout.fillHeight: true
                Layout.preferredWidth: places.model.width
                current: root.folder ? root.folder.location : ""
                onOtherPane: root.otherPane()
                onOpenFolder: (path, newTab) => {
                    if (newTab)
                        tabs.open(path, false, [])
                    else
                        root.folder.open(path)
                }
                onOpenTrash: root.folder.openTrash()
                onDone: root.focusActive()
            }
            Rectangle {
                visible: root.placesShown
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Tokens.colors.border
            }
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                TabStrip {
                    Layout.fillWidth: true
                    tabs: root.tabsModel
                    onNewTab: tabs.open(root.folder ? root.folder.location : root.home, true, [])
                    onCloseTab: index => root.closeTab(index)
                }
                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: tabs.active

                    Repeater {
                        id: pages

                        model: tabs

                        delegate: TabPage {
                            tabs: root.tabsModel
                            places: root.placesModel
                            onFileActivated: path => Shell.activate(path)
                            onOpenInNewTab: path => tabs.open(path, false, [])
                            onMenuRequested: (pane, row, x, y) => root.paneMenu(pane, row, x, y)
                            onCopyText: words => textClip.put(words)
                            onDialogRequested: (kind, data) => root.openDialog(kind, data)
                            onOtherPane: root.otherPane()
                        }
                    }
                }
            }
        }
        NoticeBar {
            Layout.fillWidth: true
        }
    }

    JsonMenu {
        id: paneMenuView

        json: root.menuPane ? root.menuPane.listing.menu : ""
        onChosen: path => root.menuPane.listing.runMenu(path)
        onClosed: Qt.callLater(() => {
            if (root.menuPane && root.menuPane.listing.menu.length > 0)
                root.menuPane.listing.runMenu("")
            root.focusActive()
        })
    }

    // Text for the clipboard (Copy location).
    TextEdit {
        id: textClip

        visible: false

        function put(words: string) {
            text = words
            selectAll()
            copy()
        }
    }

    Palette {
        id: palette

        onChosen: target => {
            if (target.kind === "command")
                root.runCommand(target.id)
            else if (target.kind === "tab")
                tabs.activate(target.index)
            else if (target.kind === "folder")
                root.folder.openTyped(target.path)
            Qt.callLater(root.focusActive)
        }
    }

    // Settings (W-106), opened on demand.
    Loader {
        id: settingsLoader

        active: false
        sourceComponent: SettingsDialog {
            shownFolder: root.folder ? root.folder.location : ""
            places: root.placesModel
            onClosed: {
                settingsLoader.active = false
                root.focusActive()
            }
        }
    }

    Loader {
        id: propertiesLoader

        property string path: ""

        active: false
        sourceComponent: PropertiesDialog {
            path: propertiesLoader.path
            onClosed: {
                propertiesLoader.active = false
                root.focusActive()
            }
        }
    }
    Loader {
        id: compressLoader

        active: false
        sourceComponent: CompressDialog {
            onClosed: {
                compressLoader.active = false
                root.focusActive()
            }
        }
    }
    Loader {
        id: batchLoader

        active: false
        sourceComponent: BatchDialog {
            onClosed: {
                batchLoader.active = false
                root.focusActive()
            }
        }
    }
    // Extract to…: the desktop's folder dialog, starting in the folder
    // shown (W-113).
    FolderDialog {
        id: extractFolder

        title: qsTr("Extract here")
        acceptLabel: qsTr("Extract here")
        onAccepted: Shell.extractTo(selectedFolder.toString())
    }

    ViewMenu {
        id: viewMenu

        folder: root.folder
        onClosed: root.activePane.focusList()
    }

    QuestionDialog {
        json: Operations.question
        onAnswered: (answer, all, password) => Operations.answer(answer, all, password)
        onClosed: root.activePane.focusList()
    }
    // Before closing a window with several tabs.
    QuestionDialog {
        property bool asking: false

        id: closeQuestion

        json: asking ? Shell.closeQuestion(tabs.count) : ""
        onAnswered: answer => {
            asking = false
            if (answer === 3) {
                root.closeAsked = true
                root.close()
            }
        }
    }

    QuestionDialog {
        json: Operations.question.length === 0 ? Shell.launch : ""
        onAnswered: answer => Shell.answerLaunch(answer)
        onClosed: root.activePane.focusList()
    }

    // Bands along the edges resize the window (4 pixels; 16-pixel corners).
    Repeater {
        model: [
            { edges: Qt.LeftEdge, shape: Qt.SizeHorCursor, l: 0, t: 16, w: 4, h: -32 },
            { edges: Qt.RightEdge, shape: Qt.SizeHorCursor, l: -4, t: 16, w: 4, h: -32 },
            { edges: Qt.TopEdge, shape: Qt.SizeVerCursor, l: 16, t: 0, w: -32, h: 4 },
            { edges: Qt.BottomEdge, shape: Qt.SizeVerCursor, l: 16, t: -4, w: -32, h: 4 },
            { edges: Qt.TopEdge | Qt.LeftEdge, shape: Qt.SizeFDiagCursor, l: 0, t: 0, w: 16, h: 16 },
            { edges: Qt.TopEdge | Qt.RightEdge, shape: Qt.SizeBDiagCursor, l: -16, t: 0, w: 16, h: 16 },
            { edges: Qt.BottomEdge | Qt.LeftEdge, shape: Qt.SizeBDiagCursor, l: 0, t: -16, w: 16, h: 16 },
            { edges: Qt.BottomEdge | Qt.RightEdge, shape: Qt.SizeFDiagCursor, l: -16, t: -16, w: 16, h: 16 }
        ]
        delegate: MouseArea {
            required property var modelData

            visible: !root.maximised
            x: modelData.l < 0 ? root.width + modelData.l : modelData.l
            y: modelData.t < 0 ? root.height + modelData.t : modelData.t
            width: modelData.w <= 0 ? root.width + modelData.w : modelData.w
            height: modelData.h <= 0 ? root.height + modelData.h : modelData.h
            cursorShape: modelData.shape
            onPressed: root.startSystemResize(modelData.edges)
        }
    }

    // The window's keys (KEYBOARD.md). Keys that also edit text belong to
    // the pane's list instead, so they never fire in a field.
    component Key: Shortcut {
        context: Qt.WindowShortcut
        // One press can match two of a key's sequences (Shift+= is also
        // "+"; Shift+, also "<"), which Qt calls ambiguous: no two keys
        // here share a sequence, so it is still this one.
        onActivatedAmbiguously: activated()
    }
    Key {
        // Not StandardKey.Back: on Linux it is Alt+Left again, and a key
        // listed twice is ambiguous, so neither fires.
        sequences: ["Alt+Left", "Back"]
        onActivated: root.folder.back()
    }
    Key {
        sequences: ["Alt+Right", "Forward"]
        onActivated: root.folder.forward()
    }
    Key {
        sequence: "Alt+Up"
        onActivated: root.folder.up()
    }
    Key {
        sequence: "Alt+Home"
        onActivated: root.folder.home()
    }
    Key {
        sequences: ["F5", "Refresh"]
        onActivated: root.folder.reload()
    }
    Key {
        sequences: ["Ctrl+H", "Alt+."]
        onActivated: root.folder.run("hidden")
    }
    Key {
        sequence: "Shift+F4"
        onActivated: root.folder.run("terminal")
    }
    Key {
        sequences: ["Ctrl+L", "F6", "Alt+D"]
        onActivated: toolbar.editLocation()
    }
    Key {
        sequence: "Ctrl+I"
        onActivated: root.command("filter", null)
    }
    Key {
        sequences: ["Ctrl+=", "Ctrl++"]
        onActivated: root.folder.zoomBy(1)
    }
    Key {
        sequence: "Ctrl+-"
        onActivated: root.folder.zoomBy(-1)
    }
    Key {
        sequence: "Ctrl+0"
        onActivated: root.folder.chooseZoom(2)
    }
    Key {
        sequence: "Ctrl+1"
        onActivated: root.folder.chooseMode(0)
    }
    Key {
        sequence: "Ctrl+2"
        onActivated: root.folder.chooseMode(1)
    }
    Key {
        sequence: "Ctrl+3"
        onActivated: root.folder.chooseMode(2)
    }
    Key {
        sequences: ["Ctrl+Shift+P"]
        onActivated: root.openPalette()
    }
    Key {
        // The layout decides whether Shift+, arrives as "<".
        sequences: ["Ctrl+Shift+,", "Ctrl+Shift+<", "Ctrl+<"]
        onActivated: root.openSettings()
    }
    Key {
        sequence: "Ctrl+Z"
        onActivated: Operations.undo()
    }
    Key {
        sequence: "Ctrl+Q"
        onActivated: Shell.quit()
    }
    Key {
        sequence: "Ctrl+T"
        onActivated: tabs.open(root.folder ? root.folder.location : root.home, true, [])
    }
    Key {
        sequence: "Ctrl+W"
        onActivated: root.closeTab(tabs.active)
    }
    Key {
        sequence: "Ctrl+Shift+T"
        onActivated: tabs.reopen()
    }
    Key {
        sequences: ["Ctrl+Tab", "Ctrl+PgDown"]
        onActivated: tabs.cycle(1)
    }
    Key {
        sequences: ["Ctrl+Shift+Tab", "Ctrl+PgUp"]
        onActivated: tabs.cycle(-1)
    }
    Key {
        sequence: "Ctrl+Shift+PgUp"
        onActivated: tabs.shiftActive(-1)
    }
    Key {
        sequence: "Ctrl+Shift+PgDown"
        onActivated: tabs.shiftActive(1)
    }
    Instantiator {
        model: 10
        delegate: Shortcut {
            required property int index
            context: Qt.WindowShortcut
            sequence: "Alt+" + index
            onActivated: tabs.activateNumber(index)
        }
    }
    Key {
        sequence: "F3"
        onActivated: tabs.toggleSplit()
    }
    Key {
        sequence: "Ctrl+F3"
        onActivated: {
            if (root.page && root.page.split)
                tabs.setActivePane(root.page.key, root.page.activePane === 1 ? 0 : 1)
        }
    }
    Key {
        sequence: "F9"
        onActivated: {
            root.placesShown = !root.placesShown
            if (!root.placesShown)
                root.focusActive()
        }
    }
    Key {
        sequence: "Ctrl+P"
        onActivated: {
            root.placesShown = true
            places.focusPanel()
        }
    }
    Key {
        sequence: "Ctrl+Shift+D"
        onActivated: places.model.addFolder(root.folder.location)
    }
    Key {
        sequence: "Ctrl+Shift+G"
        onActivated: places.model.key("Ctrl+Shift+G")
    }
    Key {
        sequence: "Ctrl+Alt+N"
        onActivated: root.folder.createEntry(false)
    }
    // The list's file commands; not while a field has the keyboard.
    component ListKey: Shortcut {
        context: Qt.WindowShortcut
        enabled: root.activePane && root.activePane.activeFocus && !(root.activeFocusItem instanceof TextInput)
    }
    ListKey {
        sequence: "Delete"
        onActivated: root.folder.trashSelected()
    }
    ListKey {
        sequence: "Shift+Delete"
        onActivated: root.folder.deleteSelected()
    }
    ListKey {
        sequence: "Ctrl+C"
        onActivated: root.folder.copySelected(false)
    }
    ListKey {
        sequence: "Ctrl+X"
        onActivated: root.folder.copySelected(true)
    }
    ListKey {
        sequence: "Ctrl+V"
        onActivated: root.folder.paste()
    }
    ListKey {
        sequence: "Ctrl+D"
        onActivated: root.folder.duplicateSelected()
    }
    ListKey {
        sequences: ["Alt+Return", "Alt+Enter"]
        onActivated: root.runCommand("show-properties")
    }
    ListKey {
        sequence: "Ctrl+Alt+C"
        onActivated: root.runCommand("copy-location")
    }
    ListKey {
        sequence: "Shift+F5"
        onActivated: tabs.transfer(root.folder.selectedPaths(), false)
    }
    ListKey {
        sequence: "Shift+F6"
        onActivated: tabs.transfer(root.folder.selectedPaths(), true)
    }
}
