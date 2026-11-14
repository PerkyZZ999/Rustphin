// The command palette (Ctrl+Shift+P, mockup 12): a field and up to seven
// results; Up, Down and Tab move, Enter runs, Esc closes.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: commandPalette

    // Called with what was chosen (a `{kind, …}` object).
    signal chosen(var target)

    function show(tabsJson: string, placesJson: string) {
        results.gather(tabsJson, placesJson)
        field.text = ""
        results.search("")
        open()
        field.forceActiveFocus()
    }

    function confirm() {
        const json = results.chosen()
        if (json.length === 0)
            return
        const target = JSON.parse(json)
        if (target.kind === "clear") {
            results.clearHistory()
            return
        }
        close()
        chosen(target)
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Tokens.metrics.title_bar + Tokens.metrics.toolbar
    width: Math.min(Tokens.metrics.palette, parent.width - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: Tokens.metrics.space_sm
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: FloatingSurface {}

    PaletteModel {
        id: results
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_sm
        Accessible.role: Accessible.Dialog
        Accessible.name: qsTr("Command palette")

        TextField {
            id: field

            Layout.fillWidth: true
            implicitHeight: Tokens.metrics.palette_field
            placeholderText: qsTr("Search actions, places, tabs and recent folders…")
            placeholderTextColor: Tokens.colors.text_muted
            color: Tokens.colors.text
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            leftPadding: Tokens.metrics.space_md + Tokens.metrics.icon_ui + Tokens.metrics.space_sm
            Accessible.name: qsTr("Search")
            background: Rectangle {
                radius: Tokens.metrics.radius
                color: Tokens.colors.content
                border.width: 2
                border.color: Tokens.colors.accent
            }
            onTextChanged: results.search(text)
            Keys.onUpPressed: results.moveBy(-1)
            Keys.onDownPressed: results.moveBy(1)
            Keys.onTabPressed: results.moveBy(1)
            Keys.onBacktabPressed: results.moveBy(-1)
            Keys.onReturnPressed: commandPalette.confirm()
            Keys.onEnterPressed: commandPalette.confirm()
            Keys.onEscapePressed: commandPalette.close()

            Lucide {
                x: Tokens.metrics.space_md
                anchors.verticalCenter: parent.verticalCenter
                name: "search"
            }
        }
        ListView {
            id: list

            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(count, 7) * Tokens.metrics.palette_row
            model: results
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            currentIndex: results.selected
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Results")

            delegate: Rectangle {
                id: row

                required property int index
                required property string label
                required property string name
                required property string trail
                required property string shortcut
                required property string icon
                required property bool chosen

                width: ListView.view.width
                height: Tokens.metrics.palette_row
                radius: Tokens.metrics.radius
                color: chosen ? (rowArea.containsMouse ? Tokens.colors.selection_hover : Tokens.colors.selection) : rowArea.containsMouse ? Tokens.colors.hover : "transparent"
                Accessible.role: Accessible.ListItem
                Accessible.name: label
                Accessible.description: qsTr("%1. Select, then press Enter to run.").arg(trail)
                Accessible.selected: chosen

                Rectangle {
                    visible: row.chosen
                    width: Tokens.metrics.selection_bar
                    height: parent.height
                    color: Tokens.colors.accent
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_md
                    anchors.rightMargin: Tokens.metrics.space_md
                    spacing: Tokens.metrics.space_md

                    Lucide {
                        name: row.icon
                        side: Tokens.metrics.palette_icon
                        colour: row.chosen ? Tokens.colors.on_selection : Tokens.colors.text_muted
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            color: row.chosen ? Tokens.colors.on_selection : Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: row.name
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            elide: Text.ElideMiddle
                            color: row.chosen ? Tokens.colors.on_selection : Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: row.trail
                        }
                    }
                    Text {
                        textFormat: Text.PlainText
                        visible: row.shortcut.length > 0
                        color: row.chosen ? Tokens.colors.on_selection : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.small_text
                        text: row.shortcut
                    }
                }
                MouseArea {
                    id: rowArea

                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        results.select(row.index)
                        commandPalette.confirm()
                    }
                }
            }
        }
        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            visible: list.count === 0 || results.status.length > 0
            wrapMode: Text.Wrap
            lineHeight: Tokens.metrics.text_line
            lineHeightMode: Text.FixedHeight
            color: Tokens.colors.text_muted
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.small_text
            text: list.count === 0 ? qsTr("Nothing matches. Try fewer letters.") : results.status
        }
    }
}
