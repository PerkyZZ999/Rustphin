// One of two or three short options side by side (DESIGN.md, Settings,
// W-155): the chosen segment in `selection`, the others muted. Each option
// is `{id, words, icon}`; `chosen` tells which one was picked.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Rectangle {
    id: segments

    required property list<var> options
    required property string value
    property string name: ""

    signal chosen(string id)

    // Puts the keyboard on the chosen segment.
    function focusChosen() {
        for (let index = 0; index < choices.count; index++) {
            const segment = choices.itemAt(index) as AbstractButton
            if (segment !== null && segment.checked)
                segment.forceActiveFocus()
        }
    }

    implicitWidth: row.implicitWidth + 2 * Tokens.metrics.hairline * 2
    implicitHeight: Tokens.metrics.setting_control
    radius: Tokens.metrics.radius
    color: Tokens.colors.content
    border.color: Tokens.colors.border_strong
    Accessible.role: Accessible.Grouping
    Accessible.name: name

    Row {
        id: row

        x: Tokens.metrics.hairline * 2
        y: Tokens.metrics.hairline * 2

        Repeater {
            id: choices

            model: segments.options

            delegate: AbstractButton {
                id: segment

                required property int index
                readonly property string segmentId: segments.options[index]?.id ?? ""
                readonly property string words: segments.options[index]?.words ?? ""
                readonly property string iconName: segments.options[index]?.icon ?? ""

                checked: segmentId === segments.value
                height: segments.height - Tokens.metrics.hairline * 4
                leftPadding: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
                rightPadding: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
                focusPolicy: Qt.TabFocus
                text: words
                Accessible.role: Accessible.RadioButton
                Accessible.name: words
                Accessible.checked: checked
                Keys.onLeftPressed: segments.chosen(segments.options[Math.max(0, index - 1)].id)
                Keys.onRightPressed: segments.chosen(segments.options[Math.min(segments.options.length - 1, index + 1)].id)

                contentItem: Row {
                    spacing: Tokens.metrics.space_xs

                    Lucide {
                        visible: segment.iconName.length > 0
                        anchors.verticalCenter: parent.verticalCenter
                        name: segment.iconName
                        side: Tokens.metrics.icon_small
                        colour: segment.checked ? Tokens.colors.on_selection : Tokens.colors.text_muted
                    }
                    Text {
                        textFormat: Text.PlainText
                        anchors.verticalCenter: parent.verticalCenter
                        color: segment.checked ? Tokens.colors.on_selection : segment.hovered ? Tokens.colors.text : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: segment.words
                    }
                }
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: segment.checked ? Tokens.colors.selection : segment.hovered ? Tokens.colors.hover : "transparent"
                    border.width: segment.visualFocus ? 2 : 0
                    border.color: Tokens.colors.accent
                }
                onClicked: {
                    if (segment.segmentId !== segments.value)
                        segments.chosen(segment.segmentId)
                }
            }
        }
    }
}
