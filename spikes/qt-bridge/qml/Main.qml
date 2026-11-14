import QtQuick
import QtQuick.Window
import qtbridgespike

// W-114a spike: a details list over the core, as Rustphin draws it, with
// frame measurements. Arguments: <folder> [--scroll]
Window {
    id: win
    width: 1000
    height: 700
    visible: true
    color: "transparent"
    title: "QtBridge spike"

    readonly property var args: Qt.application.arguments
    readonly property bool bench: args.indexOf("--scroll") >= 0

    Entries { id: entries }

    onFrameSwapped: entries.swapped()

    Component.onCompleted: {
        const path = args.length > 1 && args[1].startsWith("/") ? args[1] : "/tmp"
        entries.open(path)
    }

    Rectangle {
        anchors.fill: parent
        color: "#d9111a2c"
    }

    Row {
        id: header
        height: 32
        x: 16
        spacing: 16
        Repeater {
            model: ["Name", "Size", "Modified", "Type"]
            delegate: Rectangle {
                required property int index
                required property string modelData
                width: 96
                height: 28
                y: 2
                radius: 2
                color: tap.pressed ? "#2d4fd6" : "#1d2740"
                Accessible.role: Accessible.Button
                Accessible.name: "Sort by " + modelData
                Text {
                    anchors.centerIn: parent
                    color: "#e6ebf5"
                    text: modelData
                }
                TapHandler {
                    id: tap
                    onTapped: entries.sort_by(index)
                }
            }
        }
    }

    ListView {
        id: list
        anchors {
            top: header.bottom
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        model: entries
        clip: true
        focus: true
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        highlightMoveDuration: 0
        Accessible.role: Accessible.List
        Accessible.name: "Files"

        delegate: Rectangle {
            id: row
            required property int index
            required property string name
            required property bool folder

            width: ListView.view.width
            height: 32
            color: ListView.isCurrentItem ? "#2d4fd6" : (hover.hovered ? "#1d2740" : "transparent")
            Accessible.role: Accessible.ListItem
            Accessible.name: name
            Accessible.selected: ListView.isCurrentItem


            Rectangle {
                x: 16
                anchors.verticalCenter: parent.verticalCenter
                width: 16
                height: 16
                radius: 2
                color: row.folder ? "#e8b84a" : "#8aa0c8"
            }
            Text {
                x: 44
                width: parent.width - 400
                anchors.verticalCenter: parent.verticalCenter
                elide: Text.ElideRight
                color: "#e6ebf5"
                font.pixelSize: 14
                text: row.name
            }
            HoverHandler { id: hover }
            TapHandler { onTapped: list.currentIndex = row.index }
        }
    }

    // Opens the folder again in the running window (R-101 as D-022 reads it).
    Shortcut {
        sequence: "F5"
        onActivated: entries.open(win.args.length > 1 ? win.args[1] : "/tmp")
    }

    FrameAnimation {
        running: win.bench
        onTriggered: {
            if (entries.count() <= 1000 || !entries.settled())
                return
            entries.tick()
            list.contentY = (list.contentY + 40) % (list.contentHeight - list.height)
        }
    }
}
