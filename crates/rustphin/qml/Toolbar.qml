// The toolbar across the window (W-096, W-114c2b): the items Settings
// arranges, through ToolbarModel, acting on the active pane.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Rectangle {
    id: bar

    required property var pane
    readonly property FolderModel folder: pane ? pane.listing : null
    property bool editingLocation: false

    // The window does what the pane cannot (menus, palette, split).
    signal command(string id, var source)

    function editLocation() {
        editingLocation = true
    }

    height: Tokens.metrics.toolbar
    color: Tokens.colors.chrome

    onWidthChanged: toolbarItems.fit(width)
    Component.onCompleted: toolbarItems.fit(width)

    ToolbarModel {
        id: toolbarItems
    }

    RowLayout {
        anchors {
            fill: parent
            leftMargin: Tokens.metrics.space_sm
            rightMargin: Tokens.metrics.space_sm
        }
        spacing: Tokens.metrics.space_xs

        Repeater {
            model: toolbarItems

            delegate: Loader {
                id: entry

                required property string kind
                required property string itemId
                required property string name
                required property string tip
                required property string icon
                required property bool shown
                required property bool available

                visible: shown
                Layout.fillWidth: kind === "location"
                Layout.minimumWidth: kind === "location" ? Tokens.metrics.toolbar_location_min : -1
                Layout.alignment: Qt.AlignVCenter
                sourceComponent: kind === "separator" ? separator
                               : kind === "location" ? location
                               : kind === "actions" ? actions
                               : button

                Component {
                    id: separator

                    Item {
                        implicitWidth: 1 + 2 * Tokens.metrics.space_xs
                        implicitHeight: Tokens.metrics.tool_separator
                        Accessible.ignored: true

                        Rectangle {
                            anchors.centerIn: parent
                            width: 1
                            height: Tokens.metrics.tool_separator
                            color: Tokens.colors.border
                        }
                    }
                }

                Component {
                    id: button

                    IconButton {
                        id: tool

                        iconName: entry.itemId === "admin" && Operations.admin ? "shield-alert" : entry.icon
                        text: entry.itemId === "admin" ? (Operations.admin ? qsTr("Stop acting as administrator")
                                                         : Operations.adminPending ? qsTr("Waiting for the password\u2026")
                                                         : Operations.adminInstalled ? entry.name
                                                         : qsTr("Act as administrator (needs KDE's kio-admin package)"))
                                                      : entry.name
                        tip: entry.itemId === "admin" ? text : entry.tip
                        iconColour: entry.itemId === "admin" && Operations.admin ? Tokens.colors.danger
                                  : enabled ? Tokens.colors.text : Tokens.colors.text_disabled
                        enabled: entry.available && bar.folder !== null
                                 && (entry.itemId === "admin" ? Operations.admin || (Operations.adminInstalled && !Operations.adminPending)
                                   : entry.itemId === "back" ? bar.folder.canGoBack
                                   : entry.itemId === "forward" ? bar.folder.canGoForward
                                   : entry.itemId === "up" ? bar.folder.canGoUp
                                   : true)
                        checkable: entry.itemId === "hidden" || entry.itemId === "filter"
                        checked: entry.itemId === "hidden" ? (bar.folder !== null && bar.folder.showHidden)
                               : entry.itemId === "filter" ? (bar.pane !== null && bar.pane.filterOpen)
                               : false
                        implicitWidth: entry.kind === "menu" ? 48 : Tokens.metrics.tool_button
                        onClicked: {
                            if (entry.kind === "menu" || ["filter", "split", "admin"].indexOf(entry.itemId) >= 0
                                    || entry.itemId.indexOf("command:") === 0)
                                bar.command(entry.itemId, tool)
                            else
                                bar.folder.run(entry.itemId)
                            if (entry.kind !== "menu" && bar.pane)
                                bar.pane.focusList()
                        }

                        // A menu button shows a 12-pixel chevron after its icon.
                        Lucide {
                            visible: entry.kind === "menu"
                            anchors {
                                right: parent.right
                                rightMargin: Tokens.metrics.space_xs
                                verticalCenter: parent.verticalCenter
                            }
                            name: "chevron-down"
                            side: Tokens.metrics.icon_small
                            colour: Tokens.colors.text_disabled
                        }
                    }
                }

                Component {
                    id: location

                    LocationBar {
                        pane: bar.pane
                        editing: bar.editingLocation
                        onEditingChanged: bar.editingLocation = editing
                    }
                }

                Component {
                    id: actions

                    OutlinedButton {
                        id: actionsButton

                        enabled: entry.available
                        text: entry.name
                        ToolTip.visible: hovered
                        ToolTip.delay: 600
                        ToolTip.text: entry.tip
                        onClicked: bar.command("actions", actionsButton)

                        contentItem: Row {
                            spacing: Tokens.metrics.space_sm

                            Lucide {
                                visible: !toolbarItems.actionsWords
                                anchors.verticalCenter: parent.verticalCenter
                                name: "menu"
                                colour: Tokens.colors.text_muted
                            }
                            Text {
                                visible: toolbarItems.actionsWords
                                anchors.verticalCenter: parent.verticalCenter
                                color: Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.body_text
                                text: entry.name
                            }
                            Text {
                                visible: toolbarItems.actionsWords && toolbarItems.actionsHint
                                anchors.verticalCenter: parent.verticalCenter
                                color: Tokens.colors.text_disabled
                                font.family: Tokens.metrics.family_data
                                font.pixelSize: Tokens.metrics.small_text
                                text: "Ctrl+Shift+P"
                            }
                        }
                    }
                }
            }
        }
    }
}
