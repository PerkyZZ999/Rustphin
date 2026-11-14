// Batch rename (S9, mockup 23): replace, a pattern with a counter, or a
// change of case, with every new name previewed before Rename.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    readonly property var shown: batch.view.length > 0 ? JSON.parse(batch.view) : null

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.rename_dialog, parent.width - 2 * Tokens.metrics.space_lg)
    height: Math.min(560, parent.height - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    visible: batch.open
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
    Component.onCompleted: batch.start()
    onClosed: batch.cancel()

    BatchModel {
        id: batch
    }

    component Field: TextField {
        id: input

        required property string field
        property string label

        Layout.fillWidth: true
        implicitHeight: Tokens.metrics.button
        placeholderText: label
        color: Tokens.colors.text
        placeholderTextColor: Tokens.colors.text_muted
        font.family: Tokens.metrics.family_data
        font.pixelSize: Tokens.metrics.data_text
        Accessible.name: label
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: Tokens.colors.content
            border.width: input.activeFocus ? 2 : 1
            border.color: input.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
        }
        onTextEdited: batch.edit(field, text)
        Keys.onReturnPressed: batch.accept()
        Keys.onEnterPressed: batch.accept()
    }

    component Segment: AbstractButton {
        id: segment

        required property bool on

        implicitHeight: Tokens.metrics.button
        leftPadding: Tokens.metrics.space_md
        rightPadding: Tokens.metrics.space_md
        checkable: true
        checked: on
        Accessible.role: Accessible.RadioButton
        Accessible.name: text
        Accessible.checked: on
        contentItem: Text {
            verticalAlignment: Text.AlignVCenter
            color: segment.on ? Tokens.colors.on_selection : Tokens.colors.text
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            text: segment.text
        }
        background: Rectangle {
            color: segment.on ? Tokens.colors.selection : segment.hovered ? Tokens.colors.hover : "transparent"
            border.color: segment.visualFocus ? Tokens.colors.accent : Tokens.colors.border_strong
            border.width: segment.visualFocus ? 2 : 1
        }
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_md
        Accessible.role: Accessible.Dialog
        Accessible.name: dialog.shown ? dialog.shown.title : ""
        Accessible.description: qsTr("Review the preview before Rename; Escape cancels.")

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: "pencil"
                colour: Tokens.colors.text
            }
            Text {
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: dialog.shown ? dialog.shown.title : ""
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Cancel")
                tip: qsTr("Cancel (Esc)")
                onClicked: dialog.close()
            }
        }
        Row {
            Layout.leftMargin: Tokens.metrics.space_lg

            Segment {
                text: qsTr("Replace")
                on: batch.mode === 0
                onClicked: batch.chooseMode(0)
            }
            Segment {
                text: qsTr("Pattern")
                on: batch.mode === 1
                onClicked: batch.chooseMode(1)
            }
            Segment {
                text: qsTr("Case")
                on: batch.mode === 2
                onClicked: batch.chooseMode(2)
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            spacing: Tokens.metrics.space_sm

            RowLayout {
                visible: batch.mode === 0
                spacing: Tokens.metrics.space_sm

                Field {
                    field: "find"
                    label: qsTr("Text to find")
                    Component.onCompleted: forceActiveFocus()
                }
                Field {
                    field: "replace"
                    label: qsTr("Replace with")
                }
                CheckBox {
                    id: regex

                    text: qsTr("Regular expression")
                    onToggled: batch.edit("regex", checked ? "true" : "false")
                    contentItem: Text {
                        leftPadding: regex.indicator.width + Tokens.metrics.space_xs
                        verticalAlignment: Text.AlignVCenter
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: regex.text
                    }
                }
            }
            ColumnLayout {
                visible: batch.mode === 1
                Layout.fillWidth: true

                Field {
                    field: "pattern"
                    label: qsTr("Pattern")
                    text: "@-#"
                }
                Text {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: qsTr("# is the counter. The number of # signs is how many digits it has. @ is the file's current name.")
                }
                RowLayout {
                    Field {
                        field: "start"
                        label: qsTr("Start")
                        text: "1"
                    }
                    Field {
                        field: "step"
                        label: qsTr("Step")
                        text: "1"
                    }
                }
            }
            Row {
                visible: batch.mode === 2

                Segment {
                    text: qsTr("Lower case")
                    on: batch.caseChange === 0
                    onClicked: batch.chooseCase(0)
                }
                Segment {
                    text: qsTr("Upper case")
                    on: batch.caseChange === 1
                    onClicked: batch.chooseCase(1)
                }
                Segment {
                    text: qsTr("Capitalise words")
                    on: batch.caseChange === 2
                    onClicked: batch.chooseCase(2)
                }
            }
            Text {
                visible: dialog.shown !== null && (dialog.shown.problem.length > 0 || dialog.shown.rowProblems.length > 0)
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Tokens.colors.danger
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: dialog.shown ? (dialog.shown.problem + " " + dialog.shown.rowProblems).trim() : ""
                Accessible.role: Accessible.AlertMessage
            }
            Text {
                visible: dialog.shown !== null && dialog.shown.unchanged
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: qsTr("Nothing would change. Edit the rule to give the files new names.")
            }
        }
        // The preview: each current name and the name it gets.
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            radius: Tokens.metrics.radius
            color: Tokens.colors.content
            border.color: Tokens.colors.border

            ListView {
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                model: dialog.shown ? dialog.shown.rows : []
                boundsBehavior: Flickable.StopAtBounds
                Accessible.role: Accessible.List
                Accessible.name: qsTr("Rename preview")
                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AsNeeded
                }
                header: RowLayout {
                    width: ListView.view.width
                    height: Tokens.metrics.heading_row

                    Text {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        leftPadding: Tokens.metrics.space_md
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: qsTr("Current name")
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: qsTr("New name")
                    }
                }
                delegate: RowLayout {
                    required property var modelData

                    width: ListView.view.width
                    height: Tokens.metrics.setting_row
                    Accessible.role: Accessible.ListItem
                    Accessible.name: qsTr("%1 becomes %2").arg(modelData.from).arg(modelData.to)
                    Accessible.description: modelData.problems

                    Text {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        leftPadding: Tokens.metrics.space_md
                        elide: Text.ElideMiddle
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: modelData.from
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        elide: Text.ElideMiddle
                        color: modelData.problems.length > 0 ? Tokens.colors.danger
                             : modelData.changes ? Tokens.colors.text : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: "→ " + modelData.to
                    }
                }
            }
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
                    text: qsTr("Rename (Enter)")
                    iconName: "pencil"
                    primary: true
                    enabled: dialog.shown !== null && dialog.shown.ready
                    onClicked: batch.accept()
                }
            }
        }
    }
}
