// A number in steps (DESIGN.md, Settings, W-155): a slider and, beside
// it, the value in words in `data-md`. The value is told once it settles:
// when the handle is let go, or at each arrow key.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

RowLayout {
    id: box

    required property real from
    required property real to
    required property real stepSize
    required property real value
    // Each step's value in words, from `from` up.
    required property list<string> steps
    required property string name

    signal settled(int value)

    function focusSlider() {
        slider.forceActiveFocus()
    }

    spacing: Tokens.metrics.space_md

    Slider {
        id: slider

        Layout.preferredWidth: Tokens.metrics.slider
        from: box.from
        to: box.to
        stepSize: box.stepSize
        value: box.value
        snapMode: Slider.SnapAlways
        focusPolicy: Qt.StrongFocus
        Accessible.name: box.name
        onPressedChanged: {
            if (!pressed && Math.round(value) !== Math.round(box.value))
                box.settled(Math.round(value))
        }
        // The arrow keys move it without a press.
        onMoved: {
            if (!pressed)
                box.settled(Math.round(value))
        }

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + (slider.availableHeight - height) / 2
            width: slider.availableWidth
            height: Tokens.metrics.space_xs
            radius: Tokens.metrics.radius
            color: Tokens.colors.track

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: Tokens.metrics.radius
                color: Tokens.colors.button_primary
            }
        }
        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + (slider.availableHeight - height) / 2
            width: Tokens.metrics.icon_ui
            height: Tokens.metrics.icon_ui
            radius: Tokens.metrics.radius
            color: slider.pressed ? Tokens.colors.on_selection : Tokens.colors.text
            border.width: slider.visualFocus ? 2 : 0
            border.color: Tokens.colors.accent
        }
    }
    Rectangle {
        Layout.preferredWidth: Tokens.metrics.slider_value
        Layout.preferredHeight: Tokens.metrics.setting_control - Tokens.metrics.space_xs
        radius: Tokens.metrics.radius
        color: Tokens.colors.content
        border.color: Tokens.colors.border

        Text {
            textFormat: Text.PlainText
            anchors.centerIn: parent
            color: Tokens.colors.text
            font.family: Tokens.metrics.family_data
            font.pixelSize: Tokens.metrics.data_text
            text: box.steps[Math.round((slider.value - box.from) / Math.max(1, box.stepSize))] ?? ""
            Accessible.role: Accessible.StaticText
            Accessible.name: box.name + ": " + text
        }
    }
}
