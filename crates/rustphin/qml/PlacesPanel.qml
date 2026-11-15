// The places panel (W-102, W-105, W-114e): places outside groups, each
// group with its places, the Remote section and the volumes as drive cards
// (W-148); a menu on each row, its keys while it has the keyboard, rename
// in place, dragging to reorder and folders dropped from the views.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

FocusScope {
    id: panel

    property alias model: places
    // The `file:` URI of the folder the active pane shows.
    property string current: ""
    property int dragFrom: -1

    // A place's folder, by its `file:` URI, to show here or in a new tab.
    signal openFolder(string uri, bool newTab)
    signal openTrash
    // A drive's Copy location: text for the clipboard.
    signal copyText(string words)
    signal done
    // Tab: the keyboard goes back to the views.
    signal otherPane(bool backward)

    function focusPanel() {
        list.forceActiveFocus()
        if (places.focusRow < 0)
            places.moveFocus(1)
    }

    onCurrentChanged: places.setCurrent(current)
    Component.onCompleted: {
        places.start()
        places.setCurrent(current)
    }

    PlacesModel {
        id: places

        onOpenFolder: (uri, newTab) => panel.openFolder(uri, newTab)
        onOpenTrash: panel.openTrash()
        onCopyText: words => panel.copyText(words)
    }

    Connections {
        target: Preferences
        function onRevisionChanged() {
            places.adoptSettings()
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Tokens.colors.chrome
        Accessible.role: Accessible.Pane
        Accessible.name: qsTr("Places and devices")
        Accessible.description: qsTr("Add folders with Ctrl+Shift+D. Use arrow keys to choose a place and Enter to open it.")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.heading_row + Tokens.metrics.space_sm
            Layout.leftMargin: Tokens.metrics.space_md
            Layout.rightMargin: Tokens.metrics.space_xs

            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.label_text
                font.capitalization: Font.AllUppercase
                font.letterSpacing: 0.5
                text: qsTr("Places")
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "list-plus"
                text: qsTr("New group")
                tip: qsTr("New group (Ctrl+Shift+G)")
                side: Tokens.metrics.small_button
                onClicked: {
                    places.command("new-group")
                    list.forceActiveFocus()
                }
            }
        }

        ListView {
            id: list

            Layout.fillWidth: true
            Layout.fillHeight: true
            model: places
            clip: true
            focus: true
            keyNavigationEnabled: false
            boundsBehavior: Flickable.StopAtBounds
            currentIndex: places.focusRow
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Places")
            ScrollBar.vertical: Scroller {}

            // The panel's keys run its commands from the table
            // (`rustphin_view::commands`, KEYBOARD.md). Alt+Up and Alt+Down
            // move rows here, so the panel takes them before the window's
            // Alt+Up (up one folder) does.
            Keys.onShortcutOverride: event => {
                event.accepted = event.modifiers === Qt.AltModifier && (event.key === Qt.Key_Up || event.key === Qt.Key_Down)
            }
            Keys.onPressed: event => {
                const alt = event.modifiers === Qt.AltModifier
                const plain = event.modifiers === Qt.NoModifier
                let id = ""
                switch (event.key) {
                case Qt.Key_Up:
                    id = alt ? "move-row-up" : "focus-previous"
                    break
                case Qt.Key_Down:
                    id = alt ? "move-row-down" : "focus-next"
                    break
                case Qt.Key_Left:
                    id = "collapse"
                    break
                case Qt.Key_Right:
                    id = "expand"
                    break
                case Qt.Key_Return:
                case Qt.Key_Enter:
                    id = "open"
                    break
                case Qt.Key_F2:
                    id = "rename-row"
                    break
                case Qt.Key_Delete:
                    id = "remove-row"
                    break
                case Qt.Key_H:
                    id = plain ? "hide-row" : ""
                    break
                case Qt.Key_G:
                    id = plain ? "next-group" : ""
                    break
                case Qt.Key_Tab:
                case Qt.Key_Backtab:
                    panel.otherPane(event.key === Qt.Key_Backtab)
                    event.accepted = true
                    return
                case Qt.Key_Menu:
                    panel.menuAtFocus()
                    event.accepted = true
                    return
                case Qt.Key_F10:
                    if (event.modifiers & Qt.ShiftModifier) {
                        panel.menuAtFocus()
                        event.accepted = true
                    }
                    return
                case Qt.Key_Escape:
                    panel.done()
                    event.accepted = true
                    return
                }
                if (id.length > 0 && places.command(id)) {
                    event.accepted = true
                    if (id === "open")
                        panel.done()
                }
            }

            delegate: Item {
                id: row

                required property int index
                required property string kind
                required property string title
                required property string icon
                required property bool current
                required property bool placeHidden
                required property bool collapsed
                required property string detail
                required property real usage
                required property bool focused
                required property bool renaming
                required property bool grouped
                required property string figures
                required property string percent
                required property bool nearlyFull
                required property string description

                readonly property bool heading: kind === "group" || kind === "heading"
                readonly property bool device: kind === "device"
                // A separator (W-152): a hairline, moved and removed as a row.
                readonly property bool separator: kind === "separator"
                readonly property bool pointed: rowArea.containsMouse || drop.containsDrag

                width: ListView.view.width
                height: device ? Tokens.metrics.device_row : separator ? Tokens.metrics.place_separator : Tokens.metrics.place_row
                Accessible.role: heading ? Accessible.Heading : separator ? Accessible.Separator : Accessible.ListItem
                Accessible.name: title
                Accessible.selected: current
                Accessible.focused: focused
                Accessible.description: heading ? (collapsed ? qsTr("Collapsed section. Activate to expand.") : qsTr("Expanded section. Activate to collapse.")) : device ? description : ""

                Rectangle {
                    id: rowFill

                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_xs
                    anchors.rightMargin: Tokens.metrics.space_xs
                    // A drive card leaves a gap under it (W-148).
                    anchors.bottomMargin: row.device ? Tokens.metrics.device_row - Tokens.metrics.device_card : 0
                    radius: Tokens.metrics.radius
                    color: row.current ? Tokens.colors.selection : row.device ? Tokens.colors.chrome_raised : row.pointed ? Tokens.colors.hover : "transparent"
                    border.width: row.focused && list.activeFocus ? 1 : 0
                    border.color: Tokens.colors.accent

                    // The pointer over a drive card lightens it as it does a row.
                    Rectangle {
                        visible: row.device && row.pointed && !row.current
                        anchors.fill: parent
                        radius: parent.radius
                        color: Tokens.colors.hover
                    }
                    // The current place's royal-blue edge (D-038).
                    Rectangle {
                        visible: row.current
                        width: Tokens.metrics.selection_bar
                        height: parent.height
                        color: Tokens.colors.accent
                    }
                }
                Rectangle {
                    visible: row.separator
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Tokens.metrics.space_md + (row.grouped ? Tokens.metrics.space_md : 0)
                    anchors.rightMargin: Tokens.metrics.space_md
                    height: Tokens.metrics.hairline
                    color: Tokens.colors.border_strong
                }
                // A drive card (W-148): icon, name and figures; the usage
                // bar; the filesystem and its traits, and how full it is.
                ColumnLayout {
                    visible: row.device
                    anchors.fill: rowFill
                    anchors.margins: Tokens.metrics.space_sm
                    spacing: 0

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Tokens.metrics.line
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            name: row.icon
                            side: Tokens.metrics.place_icon
                            colour: row.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            Layout.preferredHeight: Tokens.metrics.line
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                            color: row.current ? Tokens.colors.on_selection : Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            font.weight: Font.DemiBold
                            text: row.title
                        }
                        Text {
                            textFormat: Text.PlainText
                            visible: row.figures.length > 0
                            Layout.preferredHeight: Tokens.metrics.line
                            verticalAlignment: Text.AlignVCenter
                            color: row.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: row.figures
                        }
                    }
                    // The bar's place is kept when there is no bar, so the
                    // last line sits at the same height on every card.
                    Item {
                        Layout.fillWidth: true
                        Layout.topMargin: Tokens.metrics.space_xs
                        Layout.preferredHeight: Tokens.metrics.usage

                        Rectangle {
                            visible: row.usage >= 0
                            anchors.fill: parent
                            radius: Tokens.metrics.radius
                            color: Tokens.colors.track

                            Rectangle {
                                height: parent.height
                                width: parent.width * Math.max(0, Math.min(1, row.usage))
                                radius: parent.radius
                                // From 90 % used; "NN% used" says it in words too.
                                color: row.nearlyFull ? Tokens.colors.danger : Tokens.colors.activity
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Tokens.metrics.line
                        spacing: Tokens.metrics.space_sm

                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            Layout.preferredHeight: Tokens.metrics.line
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                            color: row.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.small_text
                            text: row.detail
                        }
                        Text {
                            textFormat: Text.PlainText
                            visible: row.percent.length > 0
                            Layout.preferredHeight: Tokens.metrics.line
                            verticalAlignment: Text.AlignVCenter
                            color: row.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: row.percent
                        }
                    }
                }
                RowLayout {
                    visible: !row.device && !row.separator
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_md + (row.grouped ? Tokens.metrics.space_md : 0)
                    anchors.rightMargin: Tokens.metrics.space_sm
                    spacing: Tokens.metrics.space_sm
                    opacity: row.placeHidden ? Tokens.metrics.hidden_opacity : 1

                    Lucide {
                        visible: row.heading
                        name: row.collapsed ? "chevron-right" : "chevron-down"
                        side: Tokens.metrics.icon_small
                    }
                    Lucide {
                        visible: !row.heading
                        name: row.icon
                        side: Tokens.metrics.place_icon
                        colour: row.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                    }
                    Text {
                        textFormat: Text.PlainText
                        visible: !row.renaming
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: row.current ? Tokens.colors.on_selection : row.heading ? Tokens.colors.text_muted : Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: row.heading ? Tokens.metrics.label_text : Tokens.metrics.body_text
                        font.capitalization: row.heading ? Font.AllUppercase : Font.MixedCase
                        // A folded group says how many places it hides.
                        text: row.kind === "group" && row.collapsed ? qsTr("%1 (%2)").arg(row.title).arg(row.detail) : row.title
                    }
                    Loader {
                        Layout.fillWidth: true
                        active: row.renaming
                        visible: active
                        sourceComponent: TextField {
                            id: field

                            property bool finished: false

                            function finish(apply: bool) {
                                if (finished)
                                    return
                                finished = true
                                places.finishRename(text, apply)
                                list.forceActiveFocus()
                            }

                            text: places.renameText
                            placeholderText: qsTr("Group name")
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            implicitHeight: Tokens.metrics.rename_field - 4
                            Accessible.name: qsTr("New name")
                            background: Rectangle {
                                radius: Tokens.metrics.radius
                                color: Tokens.colors.content
                                border.width: 2
                                border.color: Tokens.colors.accent
                            }
                            Component.onCompleted: {
                                forceActiveFocus()
                                selectAll()
                            }
                            Keys.onReturnPressed: finish(true)
                            Keys.onEnterPressed: finish(true)
                            Keys.onEscapePressed: finish(false)
                            onActiveFocusChanged: {
                                if (!activeFocus)
                                    finish(true)
                            }
                        }
                    }
                }

                MouseArea {
                    id: rowArea

                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                    enabled: !row.renaming
                    drag.target: dragProxy
                    drag.threshold: 6
                    onPressed: mouse => {
                        // The drag moves the chip by the pointer's travel:
                        // start it under the pointer, or a first drag
                        // drops where the chip last was (W-128).
                        const at = rowArea.mapToItem(dragProxy.parent, mouse.x, mouse.y)
                        dragProxy.x = at.x - dragProxy.Drag.hotSpot.x
                        dragProxy.y = at.y - dragProxy.Drag.hotSpot.y
                        list.forceActiveFocus()
                        places.focusOn(row.index)
                        if (mouse.button === Qt.RightButton)
                            panel.menuAt(row.index, rowArea, mouse.x, mouse.y)
                    }
                    onClicked: mouse => {
                        if (mouse.button === Qt.LeftButton)
                            places.activate(row.index, false)
                        else if (mouse.button === Qt.MiddleButton)
                            places.activate(row.index, true)
                    }
                    drag.onActiveChanged: {
                        if (drag.active) {
                            panel.dragFrom = row.index
                            dragProxy.text = row.title
                            dragProxy.Drag.active = true
                        } else {
                            dragProxy.Drag.drop()
                            dragProxy.Drag.active = false
                            panel.dragFrom = -1
                        }
                    }
                }
                // Places and groups land here when dragged; folders from
                // the views are added as places.
                DropArea {
                    id: drop

                    anchors.fill: parent
                    keys: ["rustphin/place", "text/uri-list"]
                    onDropped: event => {
                        if (event.hasUrls)
                            places.dropFolders(event.urls.map(url => url.toString()).join("\r\n"), row.index)
                        else if (panel.dragFrom >= 0)
                            places.dropRow(panel.dragFrom, row.index)
                        event.accept()
                    }
                }
            }
            // With no places of its own (devices aside), how to add one.
            header: Text {
                textFormat: Text.PlainText
                visible: !places.hasPlaces
                width: ListView.view.width
                height: visible ? implicitHeight : 0
                leftPadding: Tokens.metrics.space_md
                rightPadding: Tokens.metrics.space_md
                bottomPadding: Tokens.metrics.space_sm
                wrapMode: Text.Wrap
                lineHeight: Tokens.metrics.text_line
                lineHeightMode: Text.FixedHeight
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.small_text
                text: qsTr("No places yet. Add the folder shown with Ctrl+Shift+D, or drag a folder here.")
            }
            footer: Column {
                width: ListView.view.width
                spacing: Tokens.metrics.space_sm
                topPadding: Tokens.metrics.space_sm

                Text {
                    textFormat: Text.PlainText
                    visible: places.devicesNote.length > 0
                    width: parent.width - 2 * Tokens.metrics.space_md
                    x: Tokens.metrics.space_md
                    wrapMode: Text.Wrap
                    lineHeight: Tokens.metrics.text_line
                    lineHeightMode: Text.FixedHeight
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: places.devicesNote
                }
            }

            // The empty space: the panel's own menu, and drops at the end.
            MouseArea {
                anchors.left: parent.left
                anchors.right: parent.right
                // Children of a ListView live in its content: below the
                // last row, down to the bottom of the visible part.
                y: list.originY + list.contentHeight
                height: Math.max(0, list.contentY + list.height - y)
                acceptedButtons: Qt.RightButton
                onPressed: mouse => panel.menuAt(-1, this, mouse.x, mouse.y)
            }
            DropArea {
                anchors.left: parent.left
                anchors.right: parent.right
                // Children of a ListView live in its content: below the
                // last row, down to the bottom of the visible part.
                y: list.originY + list.contentHeight
                height: Math.max(0, list.contentY + list.height - y)
                keys: ["text/uri-list"]
                onDropped: event => {
                    places.dropFolders(event.urls.map(url => url.toString()).join("\r\n"), -1)
                    event.accept()
                }
            }
        }
    }

    // What is dragged: the row's name.
    Rectangle {
        id: dragProxy

        property string text: ""

        visible: Drag.active
        width: proxyText.implicitWidth + 2 * Tokens.metrics.space_md
        height: Tokens.metrics.drag_chip
        radius: Tokens.metrics.radius
        color: Tokens.colors.selection
        Drag.keys: ["rustphin/place"]
        Drag.hotSpot.x: 8
        Drag.hotSpot.y: 8

        Text {
            id: proxyText

            textFormat: Text.PlainText
            anchors.centerIn: parent
            color: Tokens.colors.on_selection
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.small_text
            text: dragProxy.text
        }
    }

    function menuAt(row: int, owner: Item, x: real, y: real) {
        if (places.openMenu(row))
            menu.openAt(owner, x, y, y)
    }

    // The Menu key or Shift+F10: the menu of the row with the keyboard,
    // under it.
    function menuAtFocus() {
        const item = list.currentItem
        menuAt(places.focusRow, item ? item : list, 0, item ? item.height : Tokens.metrics.place_row)
    }

    JsonMenu {
        id: menu

        json: places.menu
        onChosen: path => places.runMenu(path)
        onClosed: Qt.callLater(() => {
            if (places.menu.length > 0)
                places.runMenu("")
        })
    }

    // The panel's edge: dragging it sets its width.
    MouseArea {
        anchors.right: parent.right
        width: Tokens.metrics.sidebar_grip
        height: parent.height
        cursorShape: Qt.SizeHorCursor
        Accessible.role: Accessible.Separator
        Accessible.name: qsTr("Places panel width")
        property real startX: 0
        onPressed: mouse => startX = mouse.x
        onPositionChanged: mouse => {
            if (pressed)
                places.resize(panel.width + mouse.x - startX)
        }
    }
}
