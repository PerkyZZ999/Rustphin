// The tab strip (W-114e): each tab with its folder's icon and name, a
// close button on the wider ones, and "+" for a new tab. Tabs keep 160
// pixels while they fit, shrink evenly to 96, then the strip scrolls.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Rustphin

Rectangle {
    id: strip

    required property TabsModel tabs
    readonly property real tabWidth: Math.max(Tokens.metrics.tab_min, Math.min(Tokens.metrics.tab, (width - Tokens.metrics.tab_add) / Math.max(1, tabs.count)))
    property int dragFrom: -1

    signal newTab
    signal closeTab(int index)

    height: Tokens.metrics.tabs
    color: Tokens.colors.chrome
    Accessible.role: Accessible.PageTabList
    Accessible.name: qsTr("Tabs")

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Tokens.colors.border
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ListView {
            id: row

            Layout.fillHeight: true
            Layout.preferredWidth: Math.min(contentWidth, strip.width - Tokens.metrics.tab_add)
            orientation: ListView.Horizontal
            model: strip.tabs
            clip: true
            interactive: false
            boundsBehavior: Flickable.StopAtBounds
            currentIndex: strip.tabs.active
            highlightFollowsCurrentItem: false
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)

            delegate: Item {
                id: tab

                required property int index
                required property string title
                required property string icon
                required property bool current

                width: strip.tabWidth
                height: strip.height
                Accessible.role: Accessible.PageTab
                Accessible.name: title
                Accessible.selected: current

                // The whole tab, its close button included (W-171): the
                // press area below stops short of the button.
                HoverHandler {
                    id: tabHover
                }
                Rectangle {
                    anchors.fill: parent
                    color: tab.current ? Tokens.colors.content : tabHover.hovered ? Tokens.colors.hover : "transparent"
                }
                Rectangle {
                    visible: tab.current
                    width: parent.width
                    height: Tokens.metrics.selection_bar
                    color: Tokens.colors.accent
                }
                Rectangle {
                    anchors.right: parent.right
                    width: 1
                    height: parent.height
                    color: Tokens.colors.border
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_md
                    anchors.rightMargin: Tokens.metrics.space_sm
                    spacing: Tokens.metrics.space_xs

                    Lucide {
                        name: tab.icon
                        colour: tab.current ? Tokens.colors.text : Tokens.colors.text_muted
                    }
                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: tab.current ? Tokens.colors.text : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: tab.title
                    }
                    IconButton {
                        visible: strip.tabWidth >= Tokens.metrics.tab_close_shown && (tab.current || tabHover.hovered)
                        iconName: "x"
                        text: qsTr("Close tab")
                        tip: qsTr("Close tab (Ctrl+W)")
                        side: Tokens.metrics.tab_close
                        iconSide: Tokens.metrics.icon_small
                        onClicked: strip.closeTab(tab.index)
                    }
                }
                MouseArea {
                    id: tabArea

                    anchors.fill: parent
                    anchors.rightMargin: Tokens.metrics.tab_close + Tokens.metrics.space_sm
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onPressed: mouse => {
                        if (mouse.button === Qt.MiddleButton)
                            strip.closeTab(tab.index)
                        else {
                            strip.tabs.activate(tab.index)
                            strip.dragFrom = tab.index
                        }
                    }
                    // Dragging a tab sideways moves it.
                    onPositionChanged: mouse => {
                        if (!pressed || strip.dragFrom < 0)
                            return
                        const p = mapToItem(row.contentItem, mouse.x, mouse.y)
                        const to = Math.max(0, Math.min(strip.tabs.count - 1, Math.floor(p.x / strip.tabWidth)))
                        if (to !== strip.dragFrom) {
                            strip.tabs.moveTab(strip.dragFrom, to)
                            strip.dragFrom = to
                        }
                    }
                    onReleased: strip.dragFrom = -1
                }
            }
        }
        IconButton {
            iconName: "plus"
            text: qsTr("New tab")
            tip: qsTr("New tab (Ctrl+T)")
            side: Tokens.metrics.tab_add
            onClicked: strip.newTab()
        }
        Item {
            Layout.fillWidth: true
        }
    }
}
