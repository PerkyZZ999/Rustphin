// One look of the window, drawn small (DESIGN.md, Settings, W-155): a
// window over a desktop, the desktop showing through it in the glass look
// and hidden by it in the opaque one. The chosen tile has a royal-blue edge
// and a check mark. Drawn from colours that do not change with the theme,
// so both tiles look the same whichever look is in use.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

AbstractButton {
    id: tile

    required property bool glass

    padding: Tokens.metrics.space_sm
    focusPolicy: Qt.TabFocus
    text: glass ? qsTr("Glass") : qsTr("Opaque")
    Accessible.role: Accessible.RadioButton
    Accessible.name: text
    Accessible.checked: checked

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_sm

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.look_tile
            radius: Tokens.metrics.radius
            color: Tokens.colors.track
            clip: true

            // The desktop's wallpaper behind the window.
            Rectangle {
                x: parent.width * 0.5
                y: Tokens.metrics.space_md
                width: parent.width * 0.5
                height: parent.height
                radius: Tokens.metrics.radius
                color: Tokens.colors.button_primary
            }
            // The window: see-through in the glass look.
            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_sm
                anchors.rightMargin: parent.width / 5
                anchors.topMargin: Tokens.metrics.space_sm
                anchors.bottomMargin: -Tokens.metrics.radius
                radius: Tokens.metrics.radius
                color: Tokens.colors.raised
                opacity: tile.glass ? 0.72 : 1
                border.color: Tokens.colors.border_strong

                Rectangle {
                    width: parent.width / 4
                    height: parent.height
                    color: Tokens.colors.border
                }
                Column {
                    x: parent.width / 4 + Tokens.metrics.space_sm
                    y: Tokens.metrics.space_sm
                    spacing: Tokens.metrics.space_xs

                    Repeater {
                        model: [0.6, 0.4, 0.5]

                        delegate: Rectangle {
                            id: bar

                            required property real modelData

                            width: (tile.availableWidth * 0.75 - 2 * Tokens.metrics.space_sm) * bar.modelData
                            height: Tokens.metrics.space_xs
                            radius: Tokens.metrics.radius
                            color: Tokens.colors.border_strong
                        }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true

            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: tile.text
            }
            Lucide {
                visible: tile.checked
                name: "check"
                side: Tokens.metrics.icon_small
                colour: Tokens.colors.accent
            }
        }
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: tile.checked ? Tokens.colors.current : tile.hovered ? Tokens.colors.hover : "transparent"
        border.width: tile.checked || tile.visualFocus ? 2 : 1
        border.color: tile.checked || tile.visualFocus ? Tokens.colors.accent : tile.hovered ? Tokens.colors.border_strong : Tokens.colors.border
    }
}
