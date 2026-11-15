// An on/off setting (DESIGN.md, Settings, W-155): a 36 × 20 track with the
// design's 2 px corners, royal blue and the knob on the right when on, the
// track colour and the knob on the left when off, so the state shows by
// position as well as colour. Disabled, it is drawn at half opacity.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Switch {
    id: control

    padding: 0
    focusPolicy: Qt.StrongFocus
    implicitWidth: Tokens.metrics.switch_width
    implicitHeight: Tokens.metrics.switch_height
    Accessible.role: Accessible.CheckBox
    Accessible.checked: checked

    indicator: Rectangle {
        implicitWidth: Tokens.metrics.switch_width
        implicitHeight: Tokens.metrics.switch_height
        radius: Tokens.metrics.radius
        opacity: control.enabled ? 1 : 0.5
        color: control.checked ? (control.hovered ? Tokens.colors.button_primary_hover : Tokens.colors.button_primary) : Tokens.colors.track
        // Focus: a white ring on the blue track, royal blue on the grey.
        border.width: control.visualFocus ? 2 : (control.checked ? 0 : 1)
        border.color: control.visualFocus ? (control.checked ? Tokens.colors.on_button : Tokens.colors.accent) : Tokens.colors.border_strong

        Rectangle {
            x: control.checked ? parent.width - width - Tokens.metrics.hairline * 2 : Tokens.metrics.hairline * 2
            y: Tokens.metrics.hairline * 2
            width: height
            height: parent.height - Tokens.metrics.hairline * 4
            radius: Tokens.metrics.radius
            color: control.checked ? Tokens.colors.on_button : Tokens.colors.text_muted

            Behavior on x {
                NumberAnimation {
                    duration: 120
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
    contentItem: Item {}
}
