// The progress of a file operation and the latest notice, over the bottom
// of the window: what is happening, how far, and Undo or Stop.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Rustphin

ColumnLayout {
    id: bar

    spacing: 0
    visible: Operations.progressVisible || Operations.notice.length > 0

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Tokens.metrics.control_line
        visible: Operations.progressVisible
        color: Tokens.colors.chrome_raised
        Accessible.role: Accessible.ProgressBar
        Accessible.name: Operations.progressLabel

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Tokens.metrics.space_md
            anchors.rightMargin: Tokens.metrics.space_xs
            spacing: Tokens.metrics.space_sm

            Lucide {
                name: Operations.progressIcon
                colour: Tokens.colors.text_muted
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: Operations.progressLabel + (Operations.runningCount > 1 ? qsTr(" (and %1 more)").arg(Operations.runningCount - 1) : "")
                }
                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.small_text
                    text: Operations.progressDetail
                }
            }
            Rectangle {
                Layout.preferredWidth: Math.max(Tokens.metrics.progress_min, bar.width / 5)
                Layout.preferredHeight: Tokens.metrics.progress
                radius: Tokens.metrics.radius
                color: Tokens.colors.track

                Rectangle {
                    id: done

                    height: parent.height
                    radius: parent.radius
                    color: Tokens.colors.activity
                    width: Operations.progressFraction < 0 ? parent.width / 4 : parent.width * Operations.progressFraction
                }
                // An operation whose size is unknown sweeps to show it runs.
                SequentialAnimation {
                    running: Operations.progressFraction < 0 && Operations.progressVisible
                    loops: Animation.Infinite
                    onStopped: done.x = 0

                    NumberAnimation {
                        target: done
                        property: "x"
                        from: 0
                        to: done.parent.width - done.width
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
            IconButton {
                iconName: "circle-stop"
                text: qsTr("Stop")
                tip: qsTr("Stop this operation")
                onClicked: Operations.cancel()
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Tokens.metrics.control_line
        visible: Operations.notice.length > 0
        color: Operations.noticeError ? Tokens.colors.danger_surface : Tokens.colors.chrome_raised
        border.width: Operations.noticeError ? 1 : 0
        border.color: Tokens.colors.danger_border
        Accessible.role: Accessible.AlertMessage
        Accessible.name: Operations.notice

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Tokens.metrics.space_md
            anchors.rightMargin: Tokens.metrics.space_xs
            spacing: Tokens.metrics.space_sm

            Lucide {
                name: Operations.noticeError ? "triangle-alert" : "circle-check"
                colour: Operations.noticeError ? Tokens.colors.danger : Tokens.colors.success
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                elide: Text.ElideRight
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: Operations.notice
            }
            OutlinedButton {
                visible: Operations.problems.length > 0
                text: qsTr("Details")
                onClicked: bar.problemsDialog.open()
            }
            OutlinedButton {
                visible: Operations.noticeUndo
                text: qsTr("Undo (Ctrl+Z)")
                iconName: "undo-2"
                onClicked: Operations.undo()
            }
            IconButton {
                iconName: "x"
                text: qsTr("Dismiss")
                tip: qsTr("Dismiss this message")
                onClicked: Operations.dismissNotice()
            }
        }
    }

    // Keyboard focus goes back to the window when the problems dialog closes.
    signal problemsClosed

    // What the operation left out: a dialog over the window, kept out of
    // this layout.
    readonly property ProblemsDialog problemsDialog: ProblemsDialog {
        onClosed: bar.problemsClosed()
    }
}
