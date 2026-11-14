// Rustphin's Qt window: its own title bar, the toolbar, the pane, the
// progress and notice bar and the operations' questions, drawn with the
// tokens of docs/DESIGN.md. State and decisions stay in Rust; this file
// lays out, binds and forwards keys.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Window {
    id: root

    required property string startFolder
    readonly property bool maximised: visibility === Window.Maximized || visibility === Window.FullScreen
    property var activePane: pane
    readonly property FolderModel folder: activePane ? activePane.listing : null

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
        pane.focusList()
    }

    // Runs a toolbar or menu command the pane cannot run itself.
    function command(id: string, source: Item) {
        if (id === "view") {
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
        Pane {
            id: pane

            Layout.fillWidth: true
            Layout.fillHeight: true
            startFolder: root.startFolder
            focus: true
            onFileActivated: path => Shell.activate(path)
            onOpenInNewTab: path => Shell.openWindow(path)
        }
        NoticeBar {
            Layout.fillWidth: true
        }
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
    }
    Key {
        sequences: ["Alt+Left", StandardKey.Back]
        onActivated: root.folder.back()
    }
    Key {
        sequences: ["Alt+Right", StandardKey.Forward]
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
        sequences: [StandardKey.Refresh, "F5"]
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
        sequence: "Ctrl+Z"
        onActivated: Operations.undo()
    }
    Key {
        sequence: "Ctrl+Q"
        onActivated: Shell.quit()
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
}
