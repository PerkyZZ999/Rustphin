// A menu built from Rust's JSON (`menu::to_json`): items with their key and
// icon, separators and submenus. Choosing an entry reports its path; Qt's
// Menu gives the arrow keys, Enter, Esc and submenus.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

Menu {
    id: menu

    property string json: ""
    // The entry chosen, by its path ("2", "3.1").
    signal chosen(string path)

    function build(target, entries) {
        for (const entry of entries) {
            if (entry.kind === "separator") {
                target.addItem(separatorComponent.createObject(target))
            } else if (entry.kind === "submenu") {
                const sub = submenuComponent.createObject(target, { title: entry.label, enabled: entry.enabled })
                build(sub, entry.items)
                target.addMenu(sub)
            } else {
                const item = itemComponent.createObject(target, {
                    text: entry.label,
                    shortcutText: entry.shortcut,
                    iconSource: entry.icon,
                    enabled: entry.enabled,
                    path: entry.path
                })
                item.triggered.connect(() => menu.chosen(item.path))
                target.addItem(item)
            }
        }
    }

    function clear(target) {
        while (target.count > 0)
            target.takeItem(0).destroy()
    }

    // Shows the menu at a point of `owner`.
    function openAt(owner: Item, x: real, y: real) {
        clear(menu)
        if (json.length === 0)
            return
        build(menu, JSON.parse(json))
        menu.popup(owner, x, y)
    }

    width: Tokens.metrics.menu
    padding: Tokens.metrics.space_xs
    background: Rectangle {
        implicitWidth: Tokens.metrics.menu
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }

    Component {
        id: separatorComponent

        MenuSeparator {
            contentItem: Rectangle {
                implicitHeight: 1
                color: Tokens.colors.border
            }
        }
    }

    Component {
        id: submenuComponent

        Menu {
            width: Tokens.metrics.menu
            padding: Tokens.metrics.space_xs
            background: Rectangle {
                implicitWidth: Tokens.metrics.menu
                radius: Tokens.metrics.radius
                color: Tokens.colors.raised
                border.color: Tokens.colors.border_strong
            }
            delegate: MenuRow {}
        }
    }

    Component {
        id: itemComponent

        MenuRow {}
    }

    delegate: MenuRow {}

    // One row: icon, label, key; a chevron for a submenu.
    component MenuRow: MenuItem {
        id: row

        property string shortcutText: ""
        property string iconSource: ""
        property string path: ""

        implicitHeight: Tokens.metrics.menu_row
        implicitWidth: Tokens.metrics.menu - 2 * Tokens.metrics.space_xs
        leftPadding: Tokens.metrics.space_sm
        rightPadding: Tokens.metrics.space_sm
        Accessible.name: shortcutText.length > 0 ? text + " (" + shortcutText + ")" : text

        background: Rectangle {
            radius: Tokens.metrics.radius
            color: row.highlighted ? Tokens.colors.hover : "transparent"
        }
        arrow: Lucide {
            x: row.width - width - Tokens.metrics.space_sm
            anchors.verticalCenter: parent.verticalCenter
            visible: row.subMenu !== null
            name: "chevron-right"
            side: Tokens.metrics.icon_small
        }
        indicator: Item {}
        contentItem: Row {
            spacing: Tokens.metrics.space_sm

            Item {
                width: Tokens.metrics.icon_ui
                height: Tokens.metrics.icon_ui
                anchors.verticalCenter: parent.verticalCenter

                Lucide {
                    visible: row.iconSource.length > 0 && row.iconSource.indexOf("file:") !== 0
                    name: visible ? row.iconSource : ""
                    colour: row.enabled ? Tokens.colors.text_muted : Tokens.colors.text_disabled
                }
                Image {
                    visible: row.iconSource.indexOf("file:") === 0
                    anchors.fill: parent
                    source: visible ? row.iconSource : ""
                    sourceSize.width: Tokens.metrics.icon_ui
                    sourceSize.height: Tokens.metrics.icon_ui
                }
            }
            Text {
                width: row.availableWidth - Tokens.metrics.icon_ui - keyText.width - 2 * Tokens.metrics.space_sm
                anchors.verticalCenter: parent.verticalCenter
                elide: Text.ElideRight
                color: row.enabled ? Tokens.colors.text : Tokens.colors.text_disabled
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: row.text
            }
            Text {
                id: keyText

                anchors.verticalCenter: parent.verticalCenter
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.small_text
                text: row.shortcutText
            }
        }
    }
}
