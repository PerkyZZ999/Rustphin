// The toolbar across the window (W-096, W-114c2b): the items Settings
// arranges, through ToolbarModel, acting on the active pane.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Rectangle {
    id: bar

    required property Pane pane
    // Whether the window acts as administrator.
    required property bool admin
    readonly property FolderModel folder: pane ? pane.listing : null
    property bool editingLocation: false

    // The window does what the pane cannot (menus, palette, split).
    signal command(string id, var source)

    // Opens the location field; asked again while it is open, it gives
    // the breadcrumbs and the keyboard back to the list (Dolphin's Ctrl+L).
    function editLocation() {
        if (editingLocation) {
            editingLocation = false
            pane.focusList()
        } else {
            editingLocation = true
        }
    }

    height: Tokens.metrics.toolbar
    color: Tokens.colors.chrome

    onWidthChanged: toolbarItems.fit(width)
    Component.onCompleted: toolbarItems.fit(width)

    ToolbarModel {
        id: toolbarItems
    }

    // Settings, Toolbar applies at once.
    Connections {
        target: Preferences
        function onRevisionChanged() {
            toolbarItems.reload(bar.width)
        }
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
                required property bool words

                visible: shown
                Layout.fillWidth: kind === "location"
                Layout.minimumWidth: kind === "location" ? Tokens.metrics.toolbar_location_min : -1
                Layout.alignment: Qt.AlignVCenter
                sourceComponent: kind === "separator" ? separator : kind === "location" ? location : button

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

                        iconName: entry.itemId === "admin" && bar.admin ? "shield-alert" : entry.icon
                        text: entry.itemId === "admin" ? (bar.admin ? qsTr("Stop acting as administrator") : Operations.adminPending ? qsTr("Waiting for the password\u2026") : Operations.adminInstalled ? entry.name : qsTr("Act as administrator (needs KDE's kio-admin package)")) : entry.name
                        tip: entry.itemId === "admin" ? text : entry.tip
                        iconColour: entry.itemId === "admin" && bar.admin ? Tokens.colors.danger : enabled ? Tokens.colors.text : Tokens.colors.text_disabled
                        enabled: entry.available && bar.folder !== null && (entry.itemId === "admin" ? bar.admin || (Operations.adminInstalled && !Operations.adminPending) : entry.itemId === "back" ? bar.folder.canGoBack : entry.itemId === "forward" ? bar.folder.canGoForward : entry.itemId === "up" ? bar.folder.canGoUp : true)
                        checkable: entry.itemId === "hidden" || entry.itemId === "filter"
                        checked: entry.itemId === "hidden" ? (bar.folder !== null && bar.folder.showHidden) : entry.itemId === "filter" ? (bar.pane !== null && bar.pane.filterOpen) : false
                        worded: entry.words
                        extra: entry.kind === "menu" ? Tokens.metrics.tool_menu_button - Tokens.metrics.tool_button : 0
                        onClicked: {
                            if (entry.kind === "menu" || ["filter", "split", "admin", "settings", "palette"].indexOf(entry.itemId) >= 0 || entry.itemId.indexOf("command:") === 0)
                                bar.command(entry.itemId, tool)
                            else
                                bar.folder.run(entry.itemId)
                            // Settings and the palette take the keyboard
                            // themselves; giving it back to the list would
                            // leave them deaf to Escape.
                            if (entry.kind !== "menu" && entry.itemId !== "settings" && entry.itemId !== "palette" && bar.pane)
                                bar.pane.focusList()
                        }

                        // Reload turns its icon whenever the folder is
                        // reloaded, by F5 too (W-168).
                        Connections {
                            target: entry.itemId === "reload" ? bar.folder : null
                            function onReloadAsked() {
                                tool.turn()
                            }
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
                        onEditRequested: bar.editingLocation = true
                        onEditEnded: bar.editingLocation = false
                    }
                }
            }
        }
    }
}
