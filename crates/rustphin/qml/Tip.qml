// A tooltip drawn from the tokens (DESIGN.md, Tooltips): what a control
// does, and its key, after the pointer has rested on it for 600 ms. It
// opens below the control, centred under it (W-156); Qt keeps it inside the
// window.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

ToolTip {
    id: tip

    delay: 600
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? parent.height + Tokens.metrics.space_xs : 0
    padding: Tokens.metrics.space_sm
    // A long one wraps instead of leaving the window.
    width: Math.min(implicitWidth, Tokens.metrics.state_text_max)

    contentItem: Text {
        textFormat: Text.PlainText
        wrapMode: Text.Wrap
        lineHeight: Tokens.metrics.text_line
        lineHeightMode: Text.FixedHeight
        text: tip.text
        color: Tokens.colors.text
        font.family: Tokens.metrics.family_body
        font.pixelSize: Tokens.metrics.body_text
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
}
