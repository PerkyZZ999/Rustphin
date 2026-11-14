// Rustphin's Qt window (W-114c): the toolbar's location, the details list
// and the status line, drawn with the tokens of docs/DESIGN.md. Logic stays
// in Rust (FolderModel, Tokens); this file only lays out and binds.
import QtQuick
import QtQuick.Window
import Rustphin

Window {
    id: root

    required property string startFolder

    readonly property var colors: Tokens.colors
    readonly property var metrics: Tokens.metrics

    width: 1000
    height: 700
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    color: Tokens.translucent ? "transparent" : colors.window
    title: (folder.location.split("/").pop() || "/") + " — Rustphin"

    Component.onCompleted: {
        Tokens.blurWindows()
        folder.open(startFolder)
    }

    FolderModel {
        id: folder
    }

    Rectangle {
        anchors.fill: parent
        color: root.colors.window
    }

    // The toolbar: for now only the location.
    Rectangle {
        id: toolbar
        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
        }
        height: root.metrics.toolbar
        color: root.colors.chrome

        Text {
            anchors {
                left: parent.left
                right: parent.right
                verticalCenter: parent.verticalCenter
                leftMargin: root.metrics.space_lg
                rightMargin: root.metrics.space_lg
            }
            elide: Text.ElideMiddle
            color: root.colors.text
            font.family: root.metrics.family_data
            font.pixelSize: root.metrics.data_text
            text: folder.location
            Accessible.role: Accessible.StaticText
            Accessible.name: "Location: " + folder.location
        }
    }

    // Column headings; clicking one sorts by it.
    Row {
        id: headings
        anchors {
            top: toolbar.bottom
            left: parent.left
            right: parent.right
        }
        height: root.metrics.heading_row

        Repeater {
            model: [
                { label: "Name", column: 0 },
                { label: "Size", column: 1 },
                { label: "Modified", column: 2 }
            ]
            delegate: Item {
                required property var modelData
                width: modelData.column === 0 ? headings.width - 360 : 180
                height: headings.height
                Accessible.role: Accessible.ColumnHeader
                Accessible.name: "Sort by " + modelData.label

                Text {
                    // Sizes are right-aligned, so their heading is too.
                    anchors {
                        verticalCenter: parent.verticalCenter
                        left: parent.modelData.column === 1 ? undefined : parent.left
                        right: parent.modelData.column === 1 ? parent.right : undefined
                        leftMargin: root.metrics.space_lg
                        rightMargin: root.metrics.space_lg
                    }
                    color: root.colors.text_muted
                    font.family: root.metrics.family_body
                    font.pixelSize: root.metrics.small_text
                    text: parent.modelData.label
                }
                TapHandler {
                    onTapped: folder.sortBy(parent.modelData.column, false)
                }
            }
        }
    }

    Rectangle {
        anchors {
            top: headings.bottom
            left: parent.left
            right: parent.right
            bottom: status.top
        }
        color: root.colors.content

        ListView {
            id: list
            anchors.fill: parent
            model: folder
            clip: true
            focus: true
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            keyNavigationEnabled: true
            Accessible.role: Accessible.List
            Accessible.name: "Entries"

            delegate: Rectangle {
                id: row
                required property int index
                required property string name
                required property string size
                required property string modified
                required property bool folder
                required property bool hidden

                readonly property bool current: ListView.isCurrentItem

                width: ListView.view.width
                height: root.metrics.row
                color: current ? root.colors.selection
                               : (hover.hovered ? root.colors.hover : "transparent")
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: current

                Item {
                    anchors.fill: parent
                    opacity: row.hidden ? root.metrics.hidden_opacity : 1

                    Rectangle {
                        x: root.metrics.space_lg
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.metrics.icon
                        height: root.metrics.icon
                        radius: root.metrics.radius
                        color: row.folder ? root.colors.accent : root.colors.text_muted
                        opacity: 0.8
                    }
                    Text {
                        x: root.metrics.space_lg + root.metrics.icon + root.metrics.space_sm
                        width: parent.width - x - 360
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        color: row.current ? root.colors.on_selection : root.colors.text
                        font.family: root.metrics.family_body
                        font.pixelSize: root.metrics.name_text
                        text: row.name
                    }
                    Text {
                        x: parent.width - 360
                        width: 180 - root.metrics.space_lg
                        horizontalAlignment: Text.AlignRight
                        anchors.verticalCenter: parent.verticalCenter
                        color: row.current ? root.colors.on_selection : root.colors.text_muted
                        font.family: root.metrics.family_data
                        font.pixelSize: root.metrics.data_text
                        text: row.size
                    }
                    Text {
                        x: parent.width - 180 + root.metrics.space_lg
                        anchors.verticalCenter: parent.verticalCenter
                        color: row.current ? root.colors.on_selection : root.colors.text_muted
                        font.family: root.metrics.family_data
                        font.pixelSize: root.metrics.data_text
                        text: row.modified
                    }
                }
                HoverHandler { id: hover }
                TapHandler {
                    onTapped: {
                        list.currentIndex = row.index
                        list.forceActiveFocus()
                    }
                }
            }
        }

        // Said in place when the folder cannot be shown (as mockup 28).
        Text {
            anchors.centerIn: parent
            width: parent.width - 2 * root.metrics.space_lg
            visible: folder.problem.length > 0
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            color: root.colors.danger
            font.family: root.metrics.family_body
            font.pixelSize: root.metrics.body_text
            text: folder.problem
        }
    }

    Rectangle {
        id: status
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        height: root.metrics.heading_row
        color: root.colors.chrome

        Text {
            anchors {
                verticalCenter: parent.verticalCenter
                left: parent.left
                leftMargin: root.metrics.space_md
            }
            color: root.colors.text_muted
            font.family: root.metrics.family_data
            font.pixelSize: root.metrics.small_text
            text: folder.summary
        }
    }
}
