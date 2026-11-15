// Rustphin's Qt window: its own title bar, the toolbar, the pane, the
// progress and notice bar and the operations' questions, drawn with the
// tokens of docs/DESIGN.md. State and decisions stay in Rust; this file
// lays out, binds and forwards keys.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import Rustphin

Window {
    id: root

    // The folders this window opens in tabs, as `file:` URIs; none for a
    // plain window.
    required property list<string> startFolders
    // Names selected in the first of them, percent-encoded (another
    // application asked).
    property list<string> startSelect: []
    // The window a launch opened first: it restores and saves the session.
    required property bool startWindow
    // The home folder's `file:` URI.
    required property string home
    readonly property bool maximised: visibility === Window.Maximized || visibility === Window.FullScreen
    // Whether Rustphin draws the window's edge and resizes it itself.
    readonly property bool framed: ownTitleBar && !maximised
    readonly property TabPage page: pages.count > tabList.active ? pages.itemAt(tabList.active) as TabPage : null
    readonly property Pane activePane: page ? page.activeItem : null
    readonly property FolderModel folder: activePane ? activePane.listing : null
    property bool placesShown: true
    // The window's tabs and places, under names delegates do not shadow.
    readonly property TabsModel tabsModel: tabList
    readonly property PlacesModel placesModel: places.model
    property bool closeAsked: false

    // Opens `folders` (`file:` URIs) in new tabs; the last becomes active
    // (another launch).
    function openTabs(folders: list<string>) {
        for (const folder of folders)
            tabList.open(folder, true, [])
        requestActivate()
    }

    // Another application asked to show `folder` (a `file:` URI) with
    // `names` (percent-encoded) selected.
    function reveal(folder: string, names: list<string>) {
        tabList.open(folder, true, names)
        requestActivate()
    }

    // Whether the keyboard is in a popup over the window: a dialog a menu
    // choice opened keeps it when the menu's own close hands focus back.
    function focusInPopup(): bool {
        for (let item = activeFocusItem; item; item = item.parent) {
            if (item === Overlay.overlay)
                return true
        }
        return false
    }

    function focusActive() {
        if (focusInPopup())
            return
        if (activePane)
            activePane.focusList()
    }

    function closeTab(index: int) {
        if (!tabList.close(index))
            root.close()
        else
            Qt.callLater(focusActive)
    }

    // A new window opens at the size the user last gave one, maximised
    // if that one was (W-160); read once, so the user's resizing stays.
    width: Shell.windowWidth()
    height: Shell.windowHeight()
    minimumWidth: 480
    minimumHeight: 320
    visibility: Shell.windowMaximised() ? Window.Maximized : Window.Windowed
    onWidthChanged: sizeKept.restart()
    onHeightChanged: sizeKept.restart()
    onVisibilityChanged: sizeKept.restart()

    // Remembers the size once a resize settles, for the next window.
    Timer {
        id: sizeKept

        interval: 500
        onTriggered: Shell.keepWindowSize(root.width, root.height, root.visibility)
    }
    // The window's number in calls about its rights, and whether it acts
    // as administrator: the mode belongs to the window (D-028) and ends
    // when it closes.
    readonly property int windowId: Operations.newWindow()
    readonly property bool admin: adminNow(Operations.revision)

    function adminNow(revision: int): bool {
        return Operations.adminIn(windowId)
    }

    // Rustphin's own title bar or the desktop's, as Settings said when the
    // window was made.
    readonly property bool ownTitleBar: Shell.ownTitleBar()
    // The blur status line was dismissed in this window.
    property bool blurNoticeDismissed: false
    flags: ownTitleBar ? Qt.Window | Qt.FramelessWindowHint : Qt.Window
    // Blur may have come or gone while another window was in front.
    onActiveChanged: {
        if (active)
            Tokens.refresh()
    }
    color: Tokens.translucent ? "transparent" : Tokens.colors.window
    // Selected text in every field takes the selection's colours, not the
    // desktop colour scheme's.
    palette.highlight: Tokens.colors.selection
    palette.highlightedText: Tokens.colors.on_selection
    title: admin ? qsTr("%1 (administrator) — Rustphin").arg(folder ? folder.title : "") : qsTr("%1 — Rustphin").arg(folder ? folder.title : "")

    // Frame timing for R-101 (RUSTPHIN_FRAME_LOG, RUSTPHIN_SCROLL_BENCH).
    onFrameSwapped: {
        if (root.folder && root.folder.frameLog)
            root.folder.frameSwapped()
    }
    FrameAnimation {
        running: root.folder !== null && root.folder.scrollBench && root.folder.count > 1000 && !root.folder.loading && root.activePane.view !== null
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
        tabList.restore(startFolders, startSelect, startWindow, home)
        Qt.callLater(focusActive)
    }

    // The blur status line comes back when blur goes missing again.
    Connections {
        target: Tokens
        function onBlurMissingChanged() {
            root.blurNoticeDismissed = false
        }
    }

    // A text size or a look chosen in Settings shows at once.
    Connections {
        target: Preferences
        function onRevisionChanged() {
            Tokens.refresh()
        }
    }

    // Several tabs close together only after asking, when Settings says so.
    onClosing: close => {
        if (tabList.count > 1 && Shell.askBeforeClosingTabs() && !closeAsked) {
            close.accepted = false
            closeQuestion.asking = true
        }
        if (close.accepted) {
            Operations.leaveAdmin(windowId)
            sizeKept.stop()
            Shell.keepWindowSize(width, height, visibility)
        }
    }

    TabsModel {
        id: tabList

        onActiveChanged: Qt.callLater(root.focusActive)
    }

    // The context menu of `pane`: on its selection, or for its folder.
    function paneMenu(pane: Pane, row: int, x: real, y: real) {
        menuPane = pane
        if (pane.listing.openMenu(row >= 0 ? "selection" : "folder"))
            paneMenuView.openAt(pane, x, y, pane.menuAbove)
    }

    property Pane menuPane: null

    // Runs a command (`rustphin_view::commands`) by its id, from a key, the
    // palette or the toolbar: the pane's own, the places panel's, or the
    // window's (below).
    function runCommand(id: string) {
        const target = Shell.commandTarget(id)
        if (target === "places") {
            placesShown = true
            places.model.command(id)
            places.focusPanel()
            return
        }
        if (target === "pane") {
            if (activePane)
                folder.command(id, activePane.lanes())
            return
        }
        if (id.indexOf("activate-tab-") === 0) {
            tabList.activateNumber(Number(id.slice(13)))
            return
        }
        switch (id) {
        case "activate-last-tab":
            tabList.activateNumber(0)
            break
        case "edit-location":
            toolbar.editLocation()
            break
        case "show-filter":
            command("filter", null)
            break
        case "copy-to-other-view":
            tabList.transfer(folder.selectedUris(), false, windowId)
            break
        case "move-to-other-view":
            tabList.transfer(folder.selectedUris(), true, windowId)
            break
        case "undo":
            Operations.undo(windowId)
            break
        case "stop-operations":
            Operations.cancel()
            break
        case "show-operations":
            noticeBar.transferDialog.open()
            break
        // The notice line's Details, for the keyboard.
        case "show-problems":
            if (Operations.problems.length > 0)
                noticeBar.problemsDialog.open()
            else if (root.folder)
                root.folder.say(qsTr("The last operation left nothing out."))
            break
        case "dismiss-notice":
            Operations.dismissNotice()
            break
        case "trash-actions":
            command("trash", toolbar)
            break
        case "new-tab":
            tabList.open(folder ? folder.locationUri : home, true, [])
            break
        case "reopen-closed-tab":
            tabList.reopen()
            break
        case "focus-other-view":
            if (page && page.split)
                tabList.setActivePane(page.key, page.activePane === 1 ? 0 : 1)
            break
        case "focus-places":
            placesShown = true
            places.focusPanel()
            break
        case "quit":
            Shell.quit()
            break
        case "open-settings":
            openSettings()
            break
        case "close-tab":
            closeTab(tabList.active)
            break
        case "next-tab":
            tabList.cycle(1)
            break
        case "previous-tab":
            tabList.cycle(-1)
            break
        case "move-tab-left":
            tabList.shiftActive(-1)
            break
        case "move-tab-right":
            tabList.shiftActive(1)
            break
        case "toggle-split":
            tabList.toggleSplit()
            break
        case "other-pane":
            otherPane(false)
            break
        case "toggle-sidebar":
            placesShown = !placesShown
            if (!placesShown)
                focusActive()
            break
        case "add-place":
            places.model.addFolder(folder.locationUri)
            break
        case "open-palette":
            openPalette()
            break
        }
    }

    // A window shortcut's command. Editing the location is not taken from
    // another text field (a place being renamed, the filter): the key would
    // end that edit.
    function runShortcut(id: string) {
        if (id === "edit-location" && !toolbar.editingLocation && root.activeFocusItem instanceof TextInput)
            return
        runCommand(id)
    }

    // Opens a dialog a pane asked for.
    function openDialog(kind: string, data) {
        if (kind === "properties") {
            if (data.length > 0) {
                propertiesLoader.path = data[0]
                propertiesLoader.active = true
            }
        } else if (kind === "compress") {
            compressLoader.ticket = data
            compressLoader.active = true
        } else if (kind === "batch") {
            batchLoader.ticket = data
            batchLoader.active = true
        } else if (kind === "extractTo") {
            extractFolder.ticket = data.ticket
            extractFolder.currentFolder = data.uri
            // A window of its own: opened once the menu that asked is gone,
            // or the menu is left open behind it and stops answering.
            Qt.callLater(() => extractFolder.open())
        }
    }

    function openPalette() {
        commandPalette.show(tabList.paletteJson(), places.model.paletteJson())
    }

    // Tab: the next view, then the places panel, then back; Shift+Tab,
    // `backward`, the other way (W-076, `workspace::next_focus`).
    function otherPane(backward: bool) {
        const split = page !== null && page.split
        const from = places.activeFocus ? 0 : (split && page.activePane === 1) ? 2 : 1
        const to = Shell.nextFocus(from, split, placesShown, backward)
        if (to === 0) {
            places.focusPanel()
            return
        }
        if (page)
            tabList.setActivePane(page.key, to - 1)
        focusActive()
    }

    // Settings closed: its window goes and the keyboard is back here.
    function closeSettings() {
        settingsLoader.active = false
        root.requestActivate()
        root.focusActive()
    }

    // Opens Settings, or brings its window forward when it is open.
    function openSettings() {
        if (settingsLoader.item !== null) {
            const settingsWindow = settingsLoader.item as Window
            settingsWindow.raise()
            settingsWindow.requestActivate()
            return
        }
        settingsLoader.active = true
    }

    // Runs a toolbar or menu command the pane cannot run itself.
    function command(id: string, source: Item) {
        if (id === "new" || id === "trash") {
            menuPane = activePane
            if (folder.openMenu(id))
                paneMenuView.openAt(source, 0, source.height, 0)
        } else if (id === "split") {
            tabList.toggleSplit()
        } else if (id === "admin") {
            Operations.toggleAdmin(windowId)
        } else if (id === "palette") {
            openPalette()
        } else if (id === "settings") {
            openSettings()
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
        border.width: root.framed ? 1 : 0
        border.color: Tokens.colors.border_strong
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: root.framed ? 1 : 0
        spacing: 0

        TitleBar {
            Layout.fillWidth: true
            visible: root.ownTitleBar
            window: root
        }
        Toolbar {
            id: toolbar

            Layout.fillWidth: true
            pane: root.activePane
            admin: root.admin
            onCommand: (id, source) => root.command(id, source)
        }
        // Acting as administrator: said across the window, with the way back.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.admin_bar
            visible: root.admin
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

                    textFormat: Text.PlainText
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
                    onClicked: Operations.toggleAdmin(root.windowId)
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
                current: root.folder ? root.folder.locationUri : ""
                onOtherPane: backward => root.otherPane(backward)
                onOpenFolder: (uri, newTab) => {
                    if (newTab)
                        tabList.open(uri, false, [])
                    else
                        root.folder.open(uri)
                }
                onOpenTrash: root.folder.openTrash()
                onCopyText: words => textClip.put(words)
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
                    onNewTab: tabList.open(root.folder ? root.folder.locationUri : root.home, true, [])
                    onCloseTab: index => root.closeTab(index)
                }
                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: tabList.active

                    Repeater {
                        id: pages

                        model: tabList

                        delegate: TabPage {
                            windowId: root.windowId
                            tabs: root.tabsModel
                            places: root.placesModel
                            onFileActivated: uri => Shell.activate(uri)
                            onOpenInNewTab: uri => tabList.open(uri, false, [])
                            onMenuRequested: (pane, row, x, y) => root.paneMenu(pane, row, x, y)
                            onCopyText: words => textClip.put(words)
                            onDialogRequested: (kind, data) => root.openDialog(kind, data)
                            onOtherPane: backward => root.otherPane(backward)
                        }
                    }
                }
            }
        }
        // Glass was wanted but the desktop offers no blur (R-109).
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.blur_status
            visible: Tokens.blurMissing && !root.blurNoticeDismissed
            color: Tokens.colors.chrome
            Accessible.role: Accessible.StaticText
            Accessible.name: blurText.text

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_xs
                spacing: Tokens.metrics.space_sm

                Lucide {
                    name: "info"
                    colour: Tokens.colors.text_muted
                }
                Text {
                    id: blurText

                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: qsTr("Background blur unavailable; using the opaque theme.")
                }
                IconButton {
                    side: Tokens.metrics.small_button
                    iconName: "x"
                    text: qsTr("Dismiss")
                    tip: qsTr("Dismiss this message")
                    onClicked: root.blurNoticeDismissed = true
                }
            }
        }
        NoticeBar {
            id: noticeBar

            windowId: root.windowId
            Layout.fillWidth: true
            onDialogClosed: root.focusActive()
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
        id: commandPalette

        onChosen: target => {
            if (target.kind === "command")
                root.runCommand(target.id)
            else if (target.kind === "tab")
                tabList.activate(target.index)
            else if (target.kind === "folder")
                root.folder.openTyped(target.location)
            Qt.callLater(root.focusAfterPalette)
        }
        onClosed: Qt.callLater(root.focusAfterPalette)
    }

    // Back to the pane after the palette, unless a places command took the
    // keyboard to the places panel (a new group's name being typed).
    function focusAfterPalette() {
        if (!places.activeFocus)
            focusActive()
    }

    // Settings (W-106), opened on demand in a window of its own (W-157).
    Loader {
        id: settingsLoader

        active: false
        // Shown at its opening size; afterwards the size is the user's.
        onLoaded: {
            const settingsWindow = item as Window
            settingsWindow.width = Tokens.metrics.settings_width
            settingsWindow.height = Tokens.metrics.settings_height
            settingsWindow.show()
            settingsWindow.requestActivate()
        }
        sourceComponent: SettingsWindow {
            owner: root
            shownFolder: root.folder ? root.folder.locationUri : ""
            places: root.placesModel
            onClosing: {
                Qt.callLater(root.closeSettings)
            }
        }
    }

    Loader {
        id: propertiesLoader

        property string path: ""

        active: false
        sourceComponent: PropertiesDialog {
            windowId: root.windowId
            path: propertiesLoader.path
            onClosed: {
                propertiesLoader.active = false
                root.focusActive()
            }
        }
    }
    Loader {
        id: compressLoader

        // The ticket the pane kept the dialog's plan under.
        property int ticket: 0

        active: false
        sourceComponent: CompressDialog {
            ticket: compressLoader.ticket
            onClosed: {
                compressLoader.active = false
                root.focusActive()
            }
        }
    }
    Loader {
        id: batchLoader

        // The ticket the pane kept the dialog's plan under.
        property int ticket: 0

        active: false
        sourceComponent: BatchDialog {
            ticket: batchLoader.ticket
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

        // The ticket the pane kept the archives under.
        property int ticket: 0

        title: qsTr("Extract here")
        acceptLabel: qsTr("Extract here")
        onAccepted: Shell.extractTo(selectedFolder.toString(), ticket)
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
    // Before closing a window with several tabList.
    QuestionDialog {
        id: closeQuestion
        property bool asking: false

        json: asking ? Shell.closeQuestion(tabList.count) : ""
        onClosed: root.activePane.focusList()
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
    ResizeBand {
        edges: Qt.LeftEdge
        shape: Qt.SizeHorCursor
        l: 0
        t: 16
        w: 4
        h: -32
    }
    ResizeBand {
        edges: Qt.RightEdge
        shape: Qt.SizeHorCursor
        l: -4
        t: 16
        w: 4
        h: -32
    }
    ResizeBand {
        edges: Qt.TopEdge
        shape: Qt.SizeVerCursor
        l: 16
        t: 0
        w: -32
        h: 4
    }
    ResizeBand {
        edges: Qt.BottomEdge
        shape: Qt.SizeVerCursor
        l: 16
        t: -4
        w: -32
        h: 4
    }
    ResizeBand {
        edges: Qt.TopEdge | Qt.LeftEdge
        shape: Qt.SizeFDiagCursor
        l: 0
        t: 0
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.TopEdge | Qt.RightEdge
        shape: Qt.SizeBDiagCursor
        l: -16
        t: 0
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.BottomEdge | Qt.LeftEdge
        shape: Qt.SizeBDiagCursor
        l: 0
        t: -16
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.BottomEdge | Qt.RightEdge
        shape: Qt.SizeFDiagCursor
        l: -16
        t: -16
        w: 16
        h: 16
    }

    // The window's keys, from the command table (`rustphin_view::commands`,
    // KEYBOARD.md). Keys that also edit text belong to the pane's and the
    // places panel's own handlers instead, so they never fire in a field.
    Instantiator {
        model: JSON.parse(Shell.shortcuts())
        delegate: Shortcut {
            // The command's id, "list" or "window", then its sequences.
            required property list<string> modelData

            context: Qt.WindowShortcut
            sequences: Array.from(modelData).slice(2)
            // The list's file commands: not while a field has the keyboard.
            enabled: modelData[1] !== "list" || (root.activePane !== null && root.activePane.activeFocus && !(root.activeFocusItem instanceof TextInput))
            onActivated: root.runShortcut(modelData[0])
            // One press can match two of a command's sequences (Shift+= is
            // also "+"; Shift+, also "<"), which Qt calls ambiguous: the
            // table gives no two commands one sequence, so it is still this.
            onActivatedAmbiguously: activated()
        }
    }

    // A band that resizes the window from `edges`. Negative `l`/`t` count
    // from the right or bottom; `w`/`h` of zero or less take the window's
    // size less that much.
    component ResizeBand: MouseArea {
        required property int edges
        required property int shape
        required property real l
        required property real t
        required property real w
        required property real h

        visible: root.framed
        x: l < 0 ? root.width + l : l
        y: t < 0 ? root.height + t : t
        width: w <= 0 ? root.width + w : w
        height: h <= 0 ? root.height + h : h
        cursorShape: shape
        onPressed: root.startSystemResize(edges)
    }
}
