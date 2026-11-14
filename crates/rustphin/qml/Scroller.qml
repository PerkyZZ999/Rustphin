// A scrollbar drawn from the tokens (DESIGN.md, Scrollbars): a thumb in
// the muted text colour, shown while there is more than fits.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

ScrollBar {
    id: bar

    policy: ScrollBar.AsNeeded
    padding: Tokens.metrics.scrollbar_inset
    minimumSize: 0.08

    contentItem: Rectangle {
        implicitWidth: Tokens.metrics.scrollbar
        implicitHeight: Tokens.metrics.scrollbar
        radius: Tokens.metrics.radius
        color: Tokens.colors.text_muted
        opacity: bar.pressed ? 0.8 : bar.hovered ? 0.65 : 0.5
        visible: bar.size < 1
    }
}
