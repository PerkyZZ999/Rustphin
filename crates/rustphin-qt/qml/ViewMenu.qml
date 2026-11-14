// The View button's menu (W-107): the three modes with their keys, and the
// size with Smaller and Larger.
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

        Repeater {
            model: [
                { name: qsTr("Icons"), key: "Ctrl+1", icon: "layout-grid", mode: 0 },
                { name: qsTr("Compact"), key: "Ctrl+2", icon: "columns-3", mode: 1 },
                { name: qsTr("Details"), key: "Ctrl+3", icon: "layout-list", mode: 2 }
            ]
            delegate: ItemDelegate {
                id: row

                required property var modelData
                readonly property bool chosen: menu.folder.mode === modelData.mode

                Layout.fillWidth: true
                implicitHeight: Tokens.metrics.menu_row
                Accessible.role: Accessible.RadioButton
                Accessible.checked: chosen
                Accessible.name: qsTr("%1 (%2)").arg(modelData.name).arg(modelData.key)
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: row.highlighted || row.hovered ? Tokens.colors.hover : "transparent"
                }
                contentItem: RowLayout {
                    spacing: Tokens.metrics.space_sm

                    Lucide {
                        name: row.modelData.icon
                        colour: Tokens.colors.text
                    }
                    Text {
                        Layout.fillWidth: true
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: row.modelData.name
                    }
                    Text {
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.small_text
                        text: row.modelData.key
                    }
                    Lucide {
                        name: row.chosen ? "check" : ""
                        colour: Tokens.colors.accent
                    }
                }
                onClicked: {
                    menu.folder.chooseMode(modelData.mode)
                    menu.close()
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: Tokens.metrics.space_xs
            Layout.bottomMargin: Tokens.metrics.space_xs
            height: 1
            color: Tokens.colors.border
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_sm
            spacing: Tokens.metrics.space_xs

            Text {
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.label_text
                font.capitalization: Font.AllUppercase
                text: qsTr("Size")
            }
            Text {
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
}
