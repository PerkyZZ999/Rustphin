// A Lucide icon in one of the theme's colours, at `side` pixels.
pragma ComponentBehavior: Bound

import QtQuick
import Rustphin

Image {
    id: icon

    required property string name
    property color colour: Tokens.colors.text_muted
    property int side: Tokens.metrics.icon_ui

    source: name.length > 0 ? "image://lucide/" + name + "/" + colour.toString().slice(1) : ""
    sourceSize.width: side
    sourceSize.height: side
    width: side
    height: side
    Accessible.ignored: true
}
