// A plain icon button: a 16-pixel Lucide icon on its square, its name read
// out and shown as a tooltip (icon-only buttons always carry one). With
// `worded` its name is also written after the icon (a toolbar item set to
// show its words).
pragma ComponentBehavior: Bound

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
    // Whether the name is written after the icon.
    property bool worded: false
    // Room kept after the icon and the words, for a menu button's chevron.
    property real extra: 0

    // Turns the icon once round (Reload's acknowledgement, W-168).
    function turn() {
        turning.restart()
    }

    display: AbstractButton.IconOnly
    padding: 0
    focusPolicy: Qt.TabFocus
    implicitWidth: side + extra + (worded ? label.implicitWidth + Tokens.metrics.space_sm : 0)
    implicitHeight: side
    Accessible.name: text
    // A hidden button (another row's Eject) is not offered to screen readers.
    Accessible.ignored: !visible
    Accessible.checked: checked

    Tip {
        visible: button.hovered && button.tip.length > 0
        text: button.tip
    }

    contentItem: Item {
        // The icon in the middle; with words, in the middle of the leading
        // square, the words after it.
        Image {
            id: glyph

            x: ((button.worded ? button.side : parent.width) - button.iconSide) / 2
            anchors.verticalCenter: parent.verticalCenter
            source: "image://lucide/" + button.iconName + "/" + button.iconColour.toString().slice(1)
            sourceSize.width: button.iconSide
            sourceSize.height: button.iconSide

            RotationAnimation on rotation {
                id: turning

                running: false
                from: 0
                to: 360
                duration: 600
                easing.type: Easing.OutCubic
            }
        }
        Text {
            id: label

            textFormat: Text.PlainText
            x: button.side
            anchors.verticalCenter: parent.verticalCenter
            visible: button.worded
            text: button.text
            color: button.iconColour
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
        }
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: button.checked || button.down ? Tokens.colors.selection : button.hovered ? button.hoverFill : "transparent"
        border.width: button.visualFocus ? 2 : 0
        border.color: Tokens.colors.accent
    }
}
