// Properties (mockup 25): the entry's facts, its size (a folder is
// measured as the dialog stays open) and its permissions, changed with
// Apply. Esc or Close leaves unapplied changes.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    required property string path
    readonly property var facts: info.facts.length > 0 ? JSON.parse(info.facts) : []
    readonly property var permissions: info.permissions.length > 0 ? JSON.parse(info.permissions) : null

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.dialog, parent.width - 2 * Tokens.metrics.space_lg)
    height: Math.min(implicitHeight, parent.height - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
    Component.onCompleted: {
        info.open(path)
        open()
    }
    onClosed: info.close()

    PropertiesModel {
        id: info
    }

    contentItem: ColumnLayout {
        spacing: 0
        Accessible.role: Accessible.Dialog
        Accessible.name: qsTr("Properties: %1").arg(info.title)
        Accessible.description: qsTr("File properties and permissions. Apply saves permission changes; Close or Escape discards unapplied changes.")
        Keys.onReturnPressed: info.apply()
        Keys.onEnterPressed: info.apply()

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: info.icon
                colour: Tokens.colors.text
            }
            Text {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: qsTr("%1 — Properties").arg(info.title)
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close")
                tip: qsTr("Close (Esc)")
                onClicked: dialog.close()
            }
        }
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: body.implicitHeight + 2 * Tokens.metrics.space_lg
            contentHeight: body.implicitHeight + 2 * Tokens.metrics.space_lg
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            ColumnLayout {
                id: body

                x: Tokens.metrics.space_lg
                y: Tokens.metrics.space_lg
                width: parent.width - 2 * Tokens.metrics.space_lg
                spacing: Tokens.metrics.space_md

                Text {
                    visible: info.loading || info.error.length > 0
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: info.error.length > 0 ? Tokens.colors.danger : Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: info.error.length > 0 ? info.error : qsTr("Reading properties…")
                    Accessible.role: Accessible.StaticText
                }
                RowLayout {
                    visible: !info.loading && info.error.length === 0
                    spacing: Tokens.metrics.space_md

                    Item {
                        implicitWidth: Tokens.metrics.properties_thumb_width
                        implicitHeight: Tokens.metrics.properties_thumb_height

                        // The type's icon from the theme, as in the views;
                        // the outline icon only when there is none.
                        Image {
                            anchors.centerIn: parent
                            visible: info.thumbnail.length === 0 && info.themeIcon.length > 0
                            width: Tokens.metrics.icon_header
                            height: Tokens.metrics.icon_header
                            source: visible ? "image://theme/" + info.themeIcon : ""
                            sourceSize.width: Tokens.metrics.icon_header
                            sourceSize.height: Tokens.metrics.icon_header
                            Accessible.ignored: true
                        }
                        Lucide {
                            anchors.centerIn: parent
                            visible: info.thumbnail.length === 0 && info.themeIcon.length === 0
                            name: info.icon
                            side: Tokens.metrics.icon_header
                        }
                        Image {
                            anchors.fill: parent
                            visible: info.thumbnail.length > 0
                            source: info.thumbnail
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                        }
                    }
                    ColumnLayout {
                        Text {
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: info.title
                        }
                        Text {
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.small_text
                            text: info.kind
                        }
                    }
                }
                Repeater {
                    model: dialog.facts

                    delegate: RowLayout {
                        required property var modelData

                        Layout.fillWidth: true
                        spacing: Tokens.metrics.space_sm
                        Accessible.role: Accessible.StaticText
                        Accessible.name: modelData[0] + ": " + modelData[1]

                        Text {
                            Layout.preferredWidth: Tokens.metrics.properties_label
                            Layout.alignment: Qt.AlignTop
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: modelData[0]
                        }
                        Text {
                            Layout.fillWidth: true
                            wrapMode: Text.WrapAnywhere
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.data_text
                            text: modelData[1]
                        }
                    }
                }
                Rectangle {
                    visible: dialog.permissions !== null
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Tokens.colors.border
                }
                RowLayout {
                    visible: dialog.permissions !== null
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_sm

                    Text {
                        Layout.preferredWidth: Tokens.metrics.properties_label
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: qsTr("Permissions")
                    }
                    Text {
                        Layout.fillWidth: true
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: dialog.permissions ? dialog.permissions.words : ""
                    }
                }
                RowLayout {
                    visible: dialog.permissions !== null
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_md
                    Accessible.role: Accessible.Grouping
                    Accessible.name: qsTr("Permissions")

                    Repeater {
                        model: dialog.permissions ? dialog.permissions.classes : []

                        delegate: ColumnLayout {
                            id: permissionClass

                            required property var modelData
                            required property int index

                            Layout.fillWidth: true
                            Layout.preferredWidth: 1
                            spacing: 2
                            Accessible.role: Accessible.Grouping
                            Accessible.name: qsTr("%1 permissions").arg(modelData.name)

                            Text {
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                color: Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.label_text
                                text: permissionClass.modelData.heading
                            }
                            Repeater {
                                model: [qsTr("Read"), qsTr("Write"), qsTr("Execute")]

                                delegate: CheckBox {
                                    id: box

                                    required property string modelData
                                    required property int index

                                    text: modelData
                                    checked: permissionClass.modelData.bits[index]
                                    enabled: dialog.permissions.editable
                                    Accessible.name: permissionClass.modelData.name + " " + modelData
                                    onToggled: info.setBit(permissionClass.index, index, checked)
                                    contentItem: Text {
                                        leftPadding: box.indicator.width + Tokens.metrics.space_xs
                                        verticalAlignment: Text.AlignVCenter
                                        color: box.enabled ? Tokens.colors.text : Tokens.colors.text_disabled
                                        font.family: Tokens.metrics.family_body
                                        font.pixelSize: Tokens.metrics.body_text
                                        text: box.text
                                    }
                                }
                            }
                        }
                    }
                }
                Text {
                    visible: dialog.permissions !== null && dialog.permissions.note.length > 0
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: dialog.permissions ? dialog.permissions.note : ""
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
                    text: qsTr("Close (Esc)")
                    onClicked: dialog.close()
                }
                OutlinedButton {
                    text: qsTr("Apply (Enter)")
                    primary: true
                    enabled: info.dirty
                    onClicked: info.apply()
                }
            }
        }
    }
}
