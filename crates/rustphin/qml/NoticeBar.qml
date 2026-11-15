// The progress of a file operation and the latest notice, over the bottom
// of the window: what is happening, how far, Details and Stop, then what
// was done, with Undo.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Rustphin

ColumnLayout {
    id: bar

    // The window's number: Undo reverts what was done with its rights.
    required property int windowId

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
            ActivityBar {
                Layout.preferredWidth: Math.max(Tokens.metrics.progress_min, bar.width / 5)
                fraction: Operations.progressFraction
                running: Operations.progressVisible
            }
            OutlinedButton {
                text: qsTr("Details")
                tip: qsTr("Show every running operation")
                onClicked: bar.transferDialog.open()
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
                onClicked: Operations.undo(bar.windowId)
            }
            IconButton {
                iconName: "x"
                text: qsTr("Dismiss")
                tip: qsTr("Dismiss this message")
                onClicked: Operations.dismissNotice()
            }
        }
    }

    // Keyboard focus goes back to the window when one of its dialogs closes.
    signal dialogClosed

    // What the operation left out: a dialog over the window, kept out of
    // this layout.
    readonly property ProblemsDialog problemsDialog: ProblemsDialog {
        onClosed: bar.dialogClosed()
    }

    // Every running operation, from the activity line's Details.
    readonly property TransferDialog transferDialog: TransferDialog {
        onClosed: bar.dialogClosed()
    }
}
