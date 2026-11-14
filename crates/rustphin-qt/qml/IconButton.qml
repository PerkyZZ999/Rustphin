// A plain icon button: a 16-pixel Lucide icon on its square, its name read
// out and shown as a tooltip (icon-only buttons always carry one).
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

ToolButton {
    id: button

    required property string iconName
    property string tip: text
    property int side: Tokens.metrics.tool_button
    property int iconSide: Tokens.metrics.icon_ui
    property color iconColour: enabled ? Tokens.colors.text : Tokens.colors.text_disabled
    property color hoverFill: Tokens.colors.hover

    display: AbstractButton.IconOnly
    focusPolicy: Qt.TabFocus
    implicitWidth: side
    implicitHeight: side
    ToolTip.visible: hovered && tip.length > 0
    ToolTip.delay: 600
    ToolTip.text: tip
    Accessible.name: text
    Accessible.checked: checked

    contentItem: Image {
        source: "image://lucide/" + button.iconName + "/" + button.iconColour.toString().slice(1)
        sourceSize.width: button.iconSide
        sourceSize.height: button.iconSide
        fillMode: Image.Pad
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: button.checked || button.down ? Tokens.colors.current
             : button.hovered ? button.hoverFill : "transparent"
        border.width: button.visualFocus ? 2 : 0
        border.color: Tokens.colors.accent
    }
}
