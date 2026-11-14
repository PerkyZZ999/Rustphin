// The location in the toolbar: the folder's crumbs, each opening its
// folder; a click on the empty part, Ctrl+L, F6 or Alt+D edits it as text.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Rectangle {
    id: bar

    required property Pane pane
    readonly property FolderModel folder: pane ? pane.listing : null
    // Owned by the toolbar, which binds it; the bar asks for changes
    // through these signals so that binding survives (W-116).
    property bool editing: false
    signal editRequested
    signal editEnded
    // Trailing crumbs shown: at most as many as the window's width allows,
    // fewer while they do not fit the bar; the rest collapse into "…".
    readonly property int most: Window.width >= 1000 ? 4 : Window.width >= 780 ? 3 : Window.width >= 520 ? 2 : 1
    property int dropped: 0
    readonly property int fits: Math.max(1, most - dropped)
    readonly property real room: width - crumbs.x - Tokens.metrics.space_sm

    // Starts again from the most whenever there may be more room; the row
    // then drops one more leading crumb each time it is laid out too wide.
    function refit() {
        dropped = 0
        trim()
    }
    function trim() {
        if (crumbs.implicitWidth > room && fits > 1)
            dropped += 1
    }

    onWidthChanged: refit()
    onMostChanged: refit()
    Connections {
        target: bar.folder
        function onCrumbLabelsChanged() {
            bar.refit()
        }
    }

    implicitHeight: Tokens.metrics.tool_button
    clip: true
    radius: Tokens.metrics.radius
    color: Tokens.colors.content
    border.width: field.activeFocus ? 2 : 1
    border.color: field.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong

    onEditingChanged: {
        if (editing) {
            field.text = folder ? folder.location : ""
            field.forceActiveFocus()
            field.selectAll()
        }
    }

    Lucide {
        id: folderIcon

        anchors {
            left: parent.left
            leftMargin: Tokens.metrics.space_sm
            verticalCenter: parent.verticalCenter
        }
        name: bar.folder && bar.folder.trash ? "trash" : "folder"
    }

    // Clicking beside the crumbs edits the location.
    MouseArea {
        anchors.fill: parent
        visible: !bar.editing
        onClicked: bar.editRequested()
    }

    Row {
        id: crumbs

        anchors {
            left: folderIcon.right
            leftMargin: Tokens.metrics.space_xs
            verticalCenter: parent.verticalCenter
        }
        visible: !bar.editing && bar.folder !== null
        spacing: 0
        onImplicitWidthChanged: bar.trim()
        Accessible.role: Accessible.Grouping
        Accessible.name: qsTr("Location: %1").arg(bar.folder ? bar.folder.location : "")

        Text {
            textFormat: Text.PlainText
            visible: bar.folder !== null && bar.folder.crumbLabels.length > bar.fits
            anchors.verticalCenter: parent.verticalCenter
            leftPadding: Tokens.metrics.space_xs
            rightPadding: Tokens.metrics.space_xs
            color: Tokens.colors.text_muted
            font.family: Tokens.metrics.family_data
            font.pixelSize: Tokens.metrics.data_text
            text: "…"
        }
        Repeater {
            model: bar.folder ? bar.folder.crumbLabels.slice(Math.max(0, bar.folder.crumbLabels.length - bar.fits)) : []

            delegate: Row {
                id: crumb

                required property string modelData
                required property int index
                readonly property int at: bar.folder.crumbLabels.length - Math.min(bar.fits, bar.folder.crumbLabels.length) + index
                readonly property bool last: at === bar.folder.crumbLabels.length - 1

                Lucide {
                    visible: crumb.index > 0
                    anchors.verticalCenter: parent.verticalCenter
                    name: "chevron-right"
                    side: Tokens.metrics.icon_small
                }
                AbstractButton {
                    id: link

                    anchors.verticalCenter: parent.verticalCenter
                    implicitHeight: Tokens.metrics.tool_button - 8
                    leftPadding: Tokens.metrics.space_xs
                    rightPadding: Tokens.metrics.space_xs
                    focusPolicy: Qt.NoFocus
                    Accessible.name: crumb.modelData
                    contentItem: Text {
                        textFormat: Text.PlainText
                        verticalAlignment: Text.AlignVCenter
                        color: crumb.last ? Tokens.colors.text : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        font.weight: crumb.last ? Font.DemiBold : Font.Normal
                        text: crumb.modelData
                    }
                    background: Rectangle {
                        radius: Tokens.metrics.radius
                        color: link.hovered ? Tokens.colors.hover : "transparent"
                    }
                    onClicked: {
                        bar.folder.open(bar.folder.crumbPaths[crumb.at])
                        bar.pane.focusList()
                    }
                }
            }
        }
    }

    TextField {
        id: field

        anchors {
            left: folderIcon.right
            right: parent.right
            verticalCenter: parent.verticalCenter
            leftMargin: Tokens.metrics.space_xs
            rightMargin: Tokens.metrics.space_xs
        }
        visible: bar.editing
        color: Tokens.colors.text
        selectionColor: Tokens.colors.selection
        selectedTextColor: Tokens.colors.on_selection
        font.family: Tokens.metrics.family_data
        font.pixelSize: Tokens.metrics.data_text
        Accessible.name: qsTr("Location")
        background: null
        function accept() {
            bar.folder.openTyped(text)
            bar.editEnded()
            bar.pane.focusList()
        }

        Keys.onReturnPressed: accept()
        Keys.onEnterPressed: accept()
        Keys.onEscapePressed: {
            bar.editEnded()
            bar.pane.focusList()
        }
        onActiveFocusChanged: {
            if (!activeFocus)
                bar.editEnded()
        }
    }
}
