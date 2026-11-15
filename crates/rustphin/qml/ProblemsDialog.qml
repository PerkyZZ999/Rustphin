// What a file operation left out (the notice's Details): one entry per
// line with why. Opened over the window by the notice bar.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: problemsDialog

    // Rust's list as JSON, read once per change into typed parts.
    property string title: ""
    property list<string> paths: []
    property list<string> reasons: []

    function readProblems() {
        const read = Operations.problems.length > 0 ? JSON.parse(Operations.problems) : null
        title = read ? read.title : ""
        paths = read ? read.entries.map(entry => entry.path) : []
        reasons = read ? read.entries.map(entry => entry.reason) : []
    }

    Connections {
        target: Operations
        function onProblemsChanged() {
            problemsDialog.readProblems()
        }
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.dialog, parent ? parent.width - 32 : Tokens.metrics.dialog)
    modal: true
    focus: true
    padding: Tokens.metrics.space_lg
    onOpened: closeButton.forceActiveFocus()
    background: FloatingSurface {}
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_md
        Accessible.role: Accessible.Dialog
        Accessible.name: problemsDialog.title

        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            lineHeight: Tokens.metrics.title_line
            lineHeightMode: Text.FixedHeight
            color: Tokens.colors.text
            font.family: Tokens.metrics.family_heading
            font.pixelSize: Tokens.metrics.title_text
            text: problemsDialog.title.length > 0 ? qsTr("%1: what was left out").arg(problemsDialog.title) : ""
        }
        ListView {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(contentHeight, Tokens.metrics.menu_visible)
            clip: true
            model: problemsDialog.paths
            delegate: ColumnLayout {
                id: problem

                required property int index

                width: ListView.view.width
                spacing: 2

                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    elide: Text.ElideMiddle
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.data_text
                    text: problemsDialog.paths[problem.index] ?? ""
                }
                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    Layout.bottomMargin: Tokens.metrics.space_sm
                    wrapMode: Text.Wrap
                    lineHeight: Tokens.metrics.text_line
                    lineHeightMode: Text.FixedHeight
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: problemsDialog.reasons[problem.index] ?? ""
                }
            }
        }
        OutlinedButton {
            id: closeButton

            Layout.alignment: Qt.AlignRight
            text: qsTr("Close")
            onClicked: problemsDialog.close()
            // The only stop: Tab stays here, and shows the ring.
            Keys.onTabPressed: focusReason = Qt.TabFocusReason
            Keys.onBacktabPressed: focusReason = Qt.BacktabFocusReason
        }
    }
}
