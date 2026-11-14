// A text button with the design's outline; `primary` fills it with the
// accent, `destructive` with the danger colour. An optional Lucide icon
// goes before the label.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Button {
    id: plain

    property string iconName: ""
    property bool primary: false
    property bool destructive: false
    readonly property color labelColour: !enabled ? Tokens.colors.text_disabled
                                       : primary || destructive ? Tokens.colors.on_selection
                                       : Tokens.colors.text

    implicitHeight: Tokens.metrics.button
    leftPadding: Tokens.metrics.space_md
    rightPadding: Tokens.metrics.space_md
    Accessible.name: text

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
            anchors.verticalCenter: parent.verticalCenter
            text: plain.text
            color: plain.labelColour
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
        }
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: plain.destructive ? (plain.down ? Qt.darker(Tokens.colors.danger, 1.2) : Tokens.colors.danger)
             : plain.primary ? (plain.down ? Qt.darker(Tokens.colors.accent, 1.2) : Tokens.colors.accent)
             : plain.down ? Tokens.colors.current : plain.hovered ? Tokens.colors.hover : "transparent"
        border.width: plain.visualFocus ? 2 : (plain.primary || plain.destructive ? 0 : 1)
        border.color: plain.visualFocus ? Tokens.colors.on_selection : Tokens.colors.border_strong
    }
}
