// Compress to… (W-111d, mockup 37): the archive's name, its format and
// how hard to compress; the line under them says what it makes.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(560, parent.width - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    visible: compress.open
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
    Component.onCompleted: compress.start()
    onOpened: {
        name.forceActiveFocus()
        name.selectAll()
    }
    onClosed: compress.cancel()

    CompressModel {
        id: compress
    }

    // A row of choices, one pressed.
    component Choices: Row {
        id: choices

        required property list<string> labels
        required property int chosen

        signal picked(int index)

        spacing: 0
        Accessible.role: Accessible.Grouping

        Repeater {
            model: choices.labels

            delegate: AbstractButton {
                id: choice

                required property string modelData
                required property int index

                implicitHeight: Tokens.metrics.button
                leftPadding: Tokens.metrics.space_md
                rightPadding: Tokens.metrics.space_md
                checkable: true
                checked: choices.chosen === index
                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData
                Accessible.checked: checked
                contentItem: Text {
                    textFormat: Text.PlainText
                    verticalAlignment: Text.AlignVCenter
                    color: choice.checked ? Tokens.colors.on_selection : Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: choice.modelData
                }
                background: Rectangle {
                    color: choice.checked ? Tokens.colors.selection : choice.hovered ? Tokens.colors.hover : "transparent"
                    border.color: choice.visualFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                    border.width: choice.visualFocus ? 2 : 1
                }
                onClicked: choices.picked(index)
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_md
        Accessible.role: Accessible.Dialog
        Accessible.name: compress.title

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: "file-archive"
                colour: Tokens.colors.text
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: compress.title
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Cancel")
                tip: qsTr("Cancel (Esc)")
                onClicked: dialog.close()
            }
        }
        GridLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            columns: 2
            columnSpacing: Tokens.metrics.space_md
            rowSpacing: Tokens.metrics.space_sm

            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: qsTr("Name")
            }
            TextField {
                id: name

                Layout.fillWidth: true
                implicitHeight: Tokens.metrics.button
                text: compress.name
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                Accessible.name: qsTr("Archive name")
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: Tokens.colors.content
                    border.width: name.activeFocus ? 2 : 1
                    border.color: name.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                }
                onTextEdited: compress.typeName(text)
                Keys.onReturnPressed: compress.accept()
                Keys.onEnterPressed: compress.accept()
            }
            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: qsTr("Format")
            }
            Choices {
                labels: ["ZIP", "TAR.GZ", "TAR.XZ", "TAR.ZST", "7Z"]
                chosen: compress.format
                Accessible.name: qsTr("Format")
                onPicked: index => compress.chooseFormat(index)
            }
            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: qsTr("Compression")
            }
            Choices {
                labels: [qsTr("Fast"), qsTr("Normal"), qsTr("Smallest")]
                chosen: compress.level
                Accessible.name: qsTr("Compression")
                onPicked: index => compress.chooseLevel(index)
            }
        }
        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            wrapMode: Text.Wrap
            color: compress.ready ? Tokens.colors.text_muted : Tokens.colors.danger
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.small_text
            text: compress.note
            Accessible.role: Accessible.StaticText
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.control_line
            color: Tokens.colors.chrome

            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Tokens.metrics.space_md
                spacing: Tokens.metrics.space_sm

                Item {
                    Layout.fillWidth: true
                }
                OutlinedButton {
                    text: qsTr("Cancel (Esc)")
                    onClicked: dialog.close()
                }
                OutlinedButton {
                    text: qsTr("Compress (Enter)")
                    iconName: "file-archive"
                    primary: true
                    enabled: compress.ready
                    onClicked: compress.accept()
                }
            }
        }
    }
}
