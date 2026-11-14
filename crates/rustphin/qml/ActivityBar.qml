// A progress bar of the activity line and of a transfer card (DESIGN.md):
// `fraction` from 0 to 1, or negative while the size is unknown, when it
// sweeps to show that the operation runs.
import QtQuick
import Rustphin

Rectangle {
    id: bar

    property real fraction: -1
    // Whether the operation is on screen: the sweep stops otherwise.
    property bool running: true

    implicitHeight: Tokens.metrics.progress
    radius: Tokens.metrics.radius
    color: Tokens.colors.track

    Rectangle {
        id: done

        height: parent.height
        radius: parent.radius
        color: Tokens.colors.activity
        width: bar.fraction < 0 ? parent.width / 4 : parent.width * Math.min(1, bar.fraction)
    }
    SequentialAnimation {
        running: bar.fraction < 0 && bar.running && bar.visible
        loops: Animation.Infinite
        onStopped: done.x = 0

        NumberAnimation {
            target: done
            property: "x"
            from: 0
            to: bar.width - done.width
            duration: 900
            easing.type: Easing.InOutQuad
        }
        NumberAnimation {
            target: done
            property: "x"
            to: 0
            duration: 900
            easing.type: Easing.InOutQuad
        }
    }
}
