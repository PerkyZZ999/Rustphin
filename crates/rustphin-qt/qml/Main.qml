// Rustphin's Qt window (W-114c): toolbar, details list and status line,
// drawn with the tokens of docs/DESIGN.md. State and decisions stay in Rust
// (FolderModel, Tokens); this file lays out and binds.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Window {
    id: root

    required property string startFolder

    readonly property var colors: Tokens.colors
    readonly property var metrics: Tokens.metrics

    // A bundled Lucide icon in one of the theme's colours.
    function lucide(name: string, colour: string): string {
        return "image://lucide/" + name + "/" + colour.slice(1)
    }

    width: 1000
    height: 700
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    color: Tokens.translucent ? "transparent" : colors.window
    title: qsTr("%1 — Rustphin").arg(listing.location.split("/").pop() || "/")

    Component.onCompleted: {
        Tokens.blurWindows()
        listing.open(startFolder)
    }

    FolderModel {
        id: listing
    }

    // A toolbar button: a 16-pixel Lucide icon on a 36-pixel square, its
    // name read out and shown as a tooltip with its key.
    component ToolbarButton: ToolButton {
        id: button

        required property string iconName
        property string keys: ""

        display: AbstractButton.IconOnly
        focusPolicy: Qt.TabFocus
        implicitWidth: 36
        implicitHeight: 36
        ToolTip.visible: hovered
        ToolTip.delay: 600
        ToolTip.text: keys.length > 0 ? qsTr("%1 (%2)").arg(text).arg(keys) : text

        contentItem: Image {
            source: root.lucide(button.iconName,
                                button.enabled ? root.colors.text : root.colors.text_disabled)
            sourceSize.width: 16
            sourceSize.height: 16
            fillMode: Image.Pad
        }
        background: Rectangle {
            radius: root.metrics.radius
            color: button.down ? root.colors.current
                               : button.hovered && button.enabled ? root.colors.hover : "transparent"
            border.width: button.visualFocus ? 2 : 0
            border.color: root.colors.accent
        }
    }

    Rectangle {
        anchors.fill: parent
        color: root.colors.window
    }

    Rectangle {
        id: toolbar

        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
        }
        height: root.metrics.toolbar
        color: root.colors.chrome

        Row {
            id: navigation

            anchors {
                left: parent.left
                leftMargin: root.metrics.space_sm
                verticalCenter: parent.verticalCenter
            }
            spacing: root.metrics.space_xs

            ToolbarButton {
                iconName: "arrow-left"
                text: qsTr("Back")
                keys: qsTr("Alt+Left")
                enabled: listing.canGoBack
                onClicked: listing.back()
            }
            ToolbarButton {
                iconName: "arrow-right"
                text: qsTr("Forward")
                keys: qsTr("Alt+Right")
                enabled: listing.canGoForward
                onClicked: listing.forward()
            }
            ToolbarButton {
                iconName: "arrow-up"
                text: qsTr("Up one folder")
                keys: qsTr("Alt+Up")
                enabled: listing.canGoUp
                onClicked: listing.up()
            }
            ToolbarButton {
                iconName: "refresh-cw"
                text: qsTr("Reload folder")
                keys: qsTr("F5")
                onClicked: listing.reload()
            }
            ToolbarButton {
                iconName: "house"
                text: qsTr("Home folder")
                keys: qsTr("Alt+Home")
                onClicked: listing.home()
            }
        }

        // The location, shown as the field it will become (W-114e).
        Rectangle {
            anchors {
                left: navigation.right
                right: parent.right
                leftMargin: root.metrics.space_sm
                rightMargin: root.metrics.space_sm
                verticalCenter: parent.verticalCenter
            }
            height: 32
            radius: root.metrics.radius
            color: root.colors.content
            border.width: 1
            border.color: root.colors.border_strong

            Text {
                anchors {
                    fill: parent
                    leftMargin: root.metrics.space_md
                    rightMargin: root.metrics.space_md
                }
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideMiddle
                color: root.colors.text
                font.family: root.metrics.family_data
                font.pixelSize: root.metrics.data_text
                text: listing.location
                Accessible.role: Accessible.StaticText
                Accessible.name: qsTr("Location: %1").arg(listing.location)
            }
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
                { label: qsTr("Name"), column: 0 },
                { label: qsTr("Size"), column: 1 },
                { label: qsTr("Modified"), column: 2 }
            ]
            delegate: Item {
                id: heading

                required property var modelData

                width: modelData.column === 0 ? headings.width - 360 : 180
                height: headings.height
                Accessible.role: Accessible.ColumnHeader
                Accessible.name: qsTr("Sort by %1").arg(modelData.label)

                Text {
                    // Sizes are right-aligned, so their heading is too.
                    anchors {
                        verticalCenter: parent.verticalCenter
                        left: heading.modelData.column === 1 ? undefined : parent.left
                        right: heading.modelData.column === 1 ? parent.right : undefined
                        leftMargin: root.metrics.space_lg
                        rightMargin: root.metrics.space_lg
                    }
                    color: root.colors.text_muted
                    font.family: root.metrics.family_body
                    font.pixelSize: root.metrics.small_text
                    text: heading.modelData.label
                }
                TapHandler {
                    onTapped: listing.sortBy(heading.modelData.column, false)
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
            model: listing
            clip: true
            focus: true
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            keyNavigationEnabled: true
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Entries")

            // A newly shown folder starts with nothing highlighted, as in Dolphin.
            Connections {
                target: listing
                function onLocationChanged() {
                    list.currentIndex = -1
                }
            }

            Keys.onReturnPressed: listing.activate(currentIndex)
            Keys.onEnterPressed: listing.activate(currentIndex)

            delegate: Rectangle {
                id: row

                required property int index
                required property string name
                required property string size
                required property string modified
                required property bool folder
                required property bool hidden
                required property string icon

                readonly property bool current: ListView.isCurrentItem
                // Hidden entries are faded, as in Dolphin; their dot says so too.
                readonly property real fade: hidden ? root.metrics.hidden_opacity : 1

                width: ListView.view.width
                height: root.metrics.row
                color: current ? root.colors.selection
                               : hover.hovered ? root.colors.hover : "transparent"
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: current

                Image {
                    x: root.metrics.space_lg
                    anchors.verticalCenter: parent.verticalCenter
                    source: "image://theme/" + row.icon
                    sourceSize.width: root.metrics.icon
                    sourceSize.height: root.metrics.icon
                    opacity: row.fade
                    Accessible.ignored: true
                }
                Text {
                    x: root.metrics.space_lg + root.metrics.icon + root.metrics.space_sm
                    width: parent.width - x - 360
                    anchors.verticalCenter: parent.verticalCenter
                    elide: Text.ElideRight
                    opacity: row.fade
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
                    opacity: row.fade
                    color: row.current ? root.colors.on_selection : root.colors.text_muted
                    font.family: root.metrics.family_data
                    font.pixelSize: root.metrics.data_text
                    text: row.size
                }
                Text {
                    x: parent.width - 180 + root.metrics.space_lg
                    anchors.verticalCenter: parent.verticalCenter
                    opacity: row.fade
                    color: row.current ? root.colors.on_selection : root.colors.text_muted
                    font.family: root.metrics.family_data
                    font.pixelSize: root.metrics.data_text
                    text: row.modified
                }
                HoverHandler {
                    id: hover
                }
                TapHandler {
                    onTapped: {
                        list.currentIndex = row.index
                        list.forceActiveFocus()
                    }
                    onDoubleTapped: listing.activate(row.index)
                }
            }
        }

        // Said in place when the folder cannot be shown (mockup 28), with
        // the ways on.
        Column {
            anchors.centerIn: parent
            width: parent.width - 2 * root.metrics.space_lg
            spacing: root.metrics.space_md
            visible: listing.problem.length > 0

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                color: root.colors.danger
                font.family: root.metrics.family_body
                font.pixelSize: root.metrics.body_text
                text: listing.problem
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: root.metrics.space_sm

                Button {
                    visible: listing.canGoUp
                    text: qsTr("Go up")
                    onClicked: listing.up()
                }
                Button {
                    text: qsTr("Home folder")
                    onClicked: listing.home()
                }
            }
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
            text: listing.summary
        }
    }

    Shortcut {
        sequences: ["Backspace", "Alt+Left", StandardKey.Back]
        onActivated: listing.back()
    }
    Shortcut {
        sequences: ["Alt+Right", StandardKey.Forward]
        onActivated: listing.forward()
    }
    Shortcut {
        sequence: "Alt+Up"
        onActivated: listing.up()
    }
    Shortcut {
        sequence: "Alt+Home"
        onActivated: listing.home()
    }
    Shortcut {
        sequences: [StandardKey.Refresh]
        onActivated: listing.reload()
    }
}
