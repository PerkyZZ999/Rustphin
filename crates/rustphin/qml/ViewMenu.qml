// The View button's menu (W-107): the three modes with their keys, and the
// size with Smaller and Larger.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: menu

    required property FolderModel folder

    width: Tokens.metrics.menu
    padding: Tokens.metrics.space_xs
    focus: true
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }

    contentItem: ColumnLayout {
        spacing: 0
        Accessible.role: Accessible.PopupMenu
        Accessible.name: qsTr("View")
        Accessible.description: qsTr("Choose how entries are shown and how large. Escape closes.")

        ModeRow {
            name: qsTr("Icons")
            key: "Ctrl+1"
            iconName: "layout-grid"
            mode: 0
        }
        ModeRow {
            name: qsTr("Compact")
            key: "Ctrl+2"
            iconName: "columns-3"
            mode: 1
        }
        ModeRow {
            name: qsTr("Details")
            key: "Ctrl+3"
            iconName: "layout-list"
            mode: 2
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: Tokens.metrics.space_xs
            Layout.bottomMargin: Tokens.metrics.space_xs
            Layout.preferredHeight: 1
            color: Tokens.colors.border
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_sm
            spacing: Tokens.metrics.space_xs

            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.label_text
                font.capitalization: Font.AllUppercase
                text: qsTr("Size")
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.small_text
                text: qsTr("%1 px").arg(menu.folder.iconSize)
            }
            IconButton {
                iconName: "zoom-out"
                text: qsTr("Smaller (Ctrl+−)")
                enabled: menu.folder.zoom > 0
                onClicked: menu.folder.zoomBy(-1)
            }
            IconButton {
                iconName: "zoom-in"
                text: qsTr("Larger (Ctrl+=)")
                enabled: menu.folder.zoom < 6
                onClicked: menu.folder.zoomBy(1)
            }
        }
    }

    // One view mode: its icon, name and key, checked when chosen.
    component ModeRow: ItemDelegate {
        id: row

        required property string name
        required property string key
        required property string iconName
        required property int mode
        readonly property bool chosen: menu.folder.mode === mode

        Layout.fillWidth: true
        implicitHeight: Tokens.metrics.menu_row
        Accessible.role: Accessible.RadioButton
        Accessible.checked: chosen
        Accessible.name: qsTr("%1 (%2)").arg(name).arg(key)
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: row.highlighted || row.hovered ? Tokens.colors.hover : "transparent"
        }
        contentItem: RowLayout {
            spacing: Tokens.metrics.space_sm

            Lucide {
                name: row.iconName
                colour: Tokens.colors.text
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: row.name
            }
            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.small_text
                text: row.key
            }
            Lucide {
                name: row.chosen ? "check" : ""
                colour: Tokens.colors.accent
            }
        }
        onClicked: {
            menu.folder.chooseMode(row.mode)
            menu.close()
        }
    }
}
