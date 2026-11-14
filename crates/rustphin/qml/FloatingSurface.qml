// The surface of a menu, the palette or a dialog (DESIGN.md, Elevation):
// solid, outlined in royal blue with a soft halo just outside (D-038).
import QtQuick
import Rustphin

Rectangle {
    radius: Tokens.metrics.radius
    color: Tokens.colors.raised
    border.color: Tokens.colors.border_accent

    Rectangle {
        z: -1
        anchors.fill: parent
        anchors.margins: -2
        radius: parent.radius + 2
        color: "transparent"
        border.width: 2
        border.color: Tokens.colors.glow
    }
}
