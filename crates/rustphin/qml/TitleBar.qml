// The window's own title bar (W-114c2b): logo, centred title, Minimise,
// Maximise or Restore, Close. Dragging moves the window; a double click
// maximises or restores it; a right-click opens KWin's window menu, which
// KWin places at the window's top-left corner.
pragma ComponentBehavior: Bound

import QtQuick
import Rustphin

Rectangle {
    id: bar

    required property Window window
    readonly property bool maximised: window.visibility === Window.Maximized || window.visibility === Window.FullScreen

    function toggleMaximised() {
        if (maximised)
            window.showNormal()
        else
            window.showMaximized()
    }

    height: Tokens.metrics.title_bar
    color: Tokens.colors.chrome

    Image {
        anchors {
            left: parent.left
            leftMargin: Tokens.metrics.space_md
            verticalCenter: parent.verticalCenter
        }
        source: "qrc:/qt/qml/Rustphin/logo.svg"
        sourceSize.width: Tokens.metrics.icon_ui
        sourceSize.height: Tokens.metrics.icon_ui
        Accessible.ignored: true
    }
    Text {
        textFormat: Text.PlainText
        anchors.centerIn: parent
        width: parent.width - 2 * windowButtons.width
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: Tokens.colors.text_muted
        font.family: Tokens.metrics.family_body
        font.pixelSize: Tokens.metrics.body_text
        text: bar.window.title
    }
    DragHandler {
        target: null
        grabPermissions: PointerHandler.TakeOverForbidden
        onActiveChanged: {
            if (active)
                bar.window.startSystemMove()
        }
    }
    TapHandler {
        onDoubleTapped: bar.toggleMaximised()
    }
    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: {
            bar.window.requestActivate()
            Shell.windowMenu()
        }
    }
    Row {
        id: windowButtons

        anchors {
            right: parent.right
            top: parent.top
            bottom: parent.bottom
        }

        IconButton {
            iconName: "minus"
            text: qsTr("Minimise")
            side: Tokens.metrics.title_bar
            implicitWidth: Tokens.metrics.window_button
            onClicked: bar.window.showMinimized()
        }
        IconButton {
            iconName: bar.maximised ? "copy" : "square"
            text: bar.maximised ? qsTr("Restore") : qsTr("Maximise")
            side: Tokens.metrics.title_bar
            implicitWidth: Tokens.metrics.window_button
            onClicked: bar.toggleMaximised()
        }
        IconButton {
            iconName: "x"
            text: qsTr("Close window")
            side: Tokens.metrics.title_bar
            implicitWidth: Tokens.metrics.window_button
            hoverFill: Tokens.colors.danger_surface
            onClicked: bar.window.close()
        }
    }
}
