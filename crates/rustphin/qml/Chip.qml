// A setting of a row of chips, on or off (DESIGN.md, Settings, W-155): on,
// it is filled with `selection` and carries a check mark; off, it is
// outlined, its words muted.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

AbstractButton {
    id: chip

    checkable: true
    implicitHeight: Tokens.metrics.setting_control
    leftPadding: Tokens.metrics.space_md
    rightPadding: Tokens.metrics.space_md
    focusPolicy: Qt.TabFocus
    Accessible.role: Accessible.CheckBox
    Accessible.name: text
    Accessible.checked: checked

    contentItem: Row {
        spacing: Tokens.metrics.space_xs

        Lucide {
            visible: chip.checked
            anchors.verticalCenter: parent.verticalCenter
            name: "check"
            side: Tokens.metrics.icon_small
            colour: Tokens.colors.on_selection
        }
        Text {
            textFormat: Text.PlainText
            anchors.verticalCenter: parent.verticalCenter
            color: chip.checked ? Tokens.colors.on_selection : chip.hovered ? Tokens.colors.text : Tokens.colors.text_muted
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            text: chip.text
        }
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: chip.checked ? Tokens.colors.selection : chip.hovered ? Tokens.colors.hover : "transparent"
        border.width: chip.visualFocus ? 2 : 1
        border.color: chip.visualFocus ? Tokens.colors.accent : chip.checked ? Tokens.colors.selection : Tokens.colors.border_strong
    }
}
