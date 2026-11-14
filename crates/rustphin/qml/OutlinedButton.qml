// A text button with the design's outline; `primary` fills it with royal
// blue (a dialog's default), `destructive` with red. An optional Lucide icon
// goes before the label.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Button {
    id: plain

    property string iconName: ""
    // What the button does, when its word alone does not say.
    property string tip: ""
    property bool primary: false
    property bool destructive: false
    readonly property bool filled: primary || destructive
    readonly property bool lit: hovered || down
    readonly property color labelColour: !enabled ? Tokens.colors.text_disabled : filled ? Tokens.colors.on_button : Tokens.colors.text

    implicitHeight: Tokens.metrics.button
    leftPadding: Tokens.metrics.space_md
    rightPadding: Tokens.metrics.space_md
    Accessible.name: text
    Accessible.description: tip
    ToolTip.visible: hovered && tip.length > 0
    ToolTip.delay: 600
    ToolTip.text: tip

    contentItem: Row {
        spacing: Tokens.metrics.space_sm

        Image {
            visible: plain.iconName.length > 0
            anchors.verticalCenter: parent.verticalCenter
            source: visible ? "image://lucide/" + plain.iconName + "/" + plain.labelColour.toString().slice(1) : ""
            sourceSize.width: Tokens.metrics.icon_ui
            sourceSize.height: Tokens.metrics.icon_ui
        }
        Text {
            textFormat: Text.PlainText
            anchors.verticalCenter: parent.verticalCenter
            text: plain.text
            color: plain.labelColour
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
        }
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        // A disabled default keeps its fill at half opacity (DESIGN.md).
        opacity: !plain.enabled && plain.filled ? 0.5 : 1
        color: plain.destructive ? (plain.lit ? Tokens.colors.button_destructive_hover : Tokens.colors.button_destructive) : plain.primary ? (plain.lit ? Tokens.colors.button_primary_hover : Tokens.colors.button_primary) : plain.down ? Tokens.colors.selection : plain.hovered ? Tokens.colors.hover : "transparent"
        border.width: plain.visualFocus ? 2 : (plain.filled ? 0 : 1)
        // Focus: a white ring inside a filled button, royal blue otherwise.
        border.color: plain.visualFocus ? (plain.filled ? Tokens.colors.on_button : Tokens.colors.accent) : Tokens.colors.border_strong
    }
}
