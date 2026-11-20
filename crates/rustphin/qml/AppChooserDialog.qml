// Open with › Other application… (W-169, Dolphin's "Other Application…"):
// a search, the file type's applications then every other installed one,
// and a box that makes the choice the type's default.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    // The file to open, as its `file://` URI.
    required property string uri
    // The "always" box starts ticked.
    property bool remember: false

    // The list Rust sends as JSON, read once per change into typed parts.
    property string title: ""
    property string empty: ""
    property string rememberLabel: ""
    property bool ready: false
    property list<var> rows: []
    readonly property int chosenIndex: {
        for (let i = 0; i < rows.length; ++i) {
            if (rows[i].chosen)
                return i
        }
        return -1
    }

    function readView() {
        const view = chooser.view.length > 0 ? JSON.parse(chooser.view) : null
        title = view ? view.title : ""
        empty = view ? view.empty : ""
        rememberLabel = view ? view.rememberLabel : ""
        ready = view !== null && view.ready === true
        rows = view ? view.rows : []
        if (chosenIndex >= 0)
            list.positionViewAtIndex(chosenIndex, ListView.Contain)
    }

    // Up and Down move the choice through the rows shown.
    function step(delta: int) {
        if (rows.length === 0)
            return
        const from = chosenIndex < 0 ? (delta > 0 ? -1 : rows.length) : chosenIndex
        const to = Math.max(0, Math.min(rows.length - 1, from + delta))
        chooser.choose(rows[to].id)
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.rename_dialog, parent.width - 2 * Tokens.metrics.space_lg)
    height: Math.min(560, parent.height - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    visible: chooser.open
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: FloatingSurface {}
    Component.onCompleted: chooser.start(dialog.uri, dialog.remember)
    onClosed: chooser.cancel()

    ChooserModel {
        id: chooser

        onViewChanged: dialog.readView()
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_md
        Accessible.role: Accessible.Dialog
        Accessible.name: dialog.title
        Accessible.description: qsTr("Type to search, choose an application, then Open; Escape cancels.")

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: "app-window"
                colour: Tokens.colors.text
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: dialog.title
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Cancel")
                tip: qsTr("Cancel (Esc)")
                onClicked: dialog.close()
            }
        }
        TextField {
            id: search

            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            implicitHeight: Tokens.metrics.button
            leftPadding: Tokens.metrics.space_sm + Tokens.metrics.icon_ui + Tokens.metrics.space_sm
            placeholderText: qsTr("Search applications")
            color: Tokens.colors.text
            placeholderTextColor: Tokens.colors.text_muted
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            Accessible.name: qsTr("Search applications")
            Component.onCompleted: forceActiveFocus()
            background: Rectangle {
                radius: Tokens.metrics.radius
                color: Tokens.colors.content
                border.width: search.activeFocus ? 2 : 1
                border.color: search.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong

                Lucide {
                    x: Tokens.metrics.space_sm
                    anchors.verticalCenter: parent.verticalCenter
                    name: "search"
                    colour: Tokens.colors.text_muted
                }
            }
            onTextEdited: chooser.search(text)
            Keys.onUpPressed: dialog.step(-1)
            Keys.onDownPressed: dialog.step(1)
            Keys.onReturnPressed: chooser.accept()
            Keys.onEnterPressed: chooser.accept()
        }
        // The applications, the file type's first.
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            radius: Tokens.metrics.radius
            color: Tokens.colors.content
            border.color: Tokens.colors.border

            ListView {
                id: list

                anchors.fill: parent
                anchors.margins: 1
                clip: true
                model: dialog.rows
                boundsBehavior: Flickable.StopAtBounds
                Accessible.role: Accessible.List
                Accessible.name: qsTr("Applications")
                ScrollBar.vertical: Scroller {}

                delegate: Column {
                    id: appRow

                    required property int index
                    readonly property string appId: dialog.rows[index]?.id ?? ""
                    readonly property string name: dialog.rows[index]?.name ?? ""
                    readonly property string icon: dialog.rows[index]?.icon ?? ""
                    readonly property string heading: dialog.rows[index]?.heading ?? ""
                    readonly property bool chosen: dialog.rows[index]?.chosen === true

                    width: ListView.view.width

                    // The group the row starts: the file type's, or the rest.
                    Text {
                        textFormat: Text.PlainText
                        visible: appRow.heading.length > 0
                        width: parent.width
                        height: Tokens.metrics.heading_row
                        leftPadding: Tokens.metrics.space_md
                        verticalAlignment: Text.AlignBottom
                        bottomPadding: Tokens.metrics.space_xs
                        elide: Text.ElideRight
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: appRow.heading
                        Accessible.role: Accessible.Heading
                    }
                    Rectangle {
                        width: parent.width
                        height: Tokens.metrics.setting_row
                        color: appRow.chosen ? Tokens.colors.selection : rowArea.containsMouse ? Tokens.colors.hover : "transparent"
                        Accessible.role: Accessible.ListItem
                        Accessible.name: appRow.name
                        Accessible.selected: appRow.chosen

                        // The chosen row's edge, besides its fill (D-038).
                        Rectangle {
                            visible: appRow.chosen
                            width: Tokens.metrics.selection_bar
                            height: parent.height
                            color: Tokens.colors.accent
                        }
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Tokens.metrics.space_md
                            anchors.rightMargin: Tokens.metrics.space_md
                            spacing: Tokens.metrics.space_sm

                            Item {
                                implicitWidth: Tokens.metrics.icon_ui
                                implicitHeight: Tokens.metrics.icon_ui

                                Image {
                                    visible: appRow.icon.length > 0
                                    anchors.fill: parent
                                    source: visible ? appRow.icon : ""
                                    sourceSize.width: Tokens.metrics.icon_ui
                                    sourceSize.height: Tokens.metrics.icon_ui
                                }
                                Lucide {
                                    visible: appRow.icon.length === 0
                                    name: "app-window"
                                    colour: Tokens.colors.text_muted
                                }
                            }
                            Text {
                                textFormat: Text.PlainText
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                color: appRow.chosen ? Tokens.colors.on_selection : Tokens.colors.text
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.body_text
                                text: appRow.name
                            }
                        }
                        MouseArea {
                            id: rowArea

                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: chooser.choose(appRow.appId)
                            onDoubleClicked: {
                                chooser.choose(appRow.appId)
                                chooser.accept()
                            }
                        }
                    }
                }
            }
            // Said in place of an empty list: what happened, what to do.
            Text {
                textFormat: Text.PlainText
                visible: dialog.rows.length === 0 && dialog.empty.length > 0
                anchors.centerIn: parent
                width: parent.width - 2 * Tokens.metrics.space_lg
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: dialog.empty
            }
        }
        Check {
            id: always

            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            text: dialog.rememberLabel
            checked: chooser.remembering
            onToggled: chooser.keep(checked)
            contentItem: Text {
                textFormat: Text.PlainText
                leftPadding: always.indicator.width + Tokens.metrics.space_xs
                verticalAlignment: Text.AlignVCenter
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: always.text
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
                    text: qsTr("Open (Enter)")
                    iconName: "square-arrow-out-up-right"
                    primary: true
                    enabled: dialog.ready
                    onClicked: chooser.accept()
                }
            }
        }
    }
}
