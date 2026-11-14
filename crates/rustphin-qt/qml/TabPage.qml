// One tab: one pane, or two side by side (F3). It tells the tabs model
// where its panes are and which has the keyboard.
import QtQuick
import QtQuick.Layouts
import Rustphin

Item {
    id: page

    required property int key
    required property bool split
    required property int activePane
    required property string leftStart
    required property string rightStart
    required property var select
    required property bool current
    required property TabsModel tabs
    required property var places
    readonly property var activeItem: split && activePane === 1 && right.item ? right.item : left

    signal fileActivated(string path)
    signal openInNewTab(string path)
    signal menuRequested(var pane, int row, real x, real y)
    signal copyText(string words)

    function report(side: int, pane) {
        const folder = pane.listing
        const icon = folder.trash ? "trash" : (page.places.iconFor(folder.location) || "folder")
        page.tabs.setPane(page.key, side, folder.location, folder.title, icon)
    }

    // Places arrive after the first folder: its icon is looked up again.
    Connections {
        target: page.places
        function onCountChanged() {
            page.report(0, left)
            if (right.item)
                page.report(1, right.item)
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Pane {
            id: left

            Layout.fillWidth: true
            Layout.fillHeight: true
            startFolder: page.leftStart
            startSelect: page.select
            split: page.split
            current: !page.split || page.activePane === 0
            onEntered: page.tabs.setActivePane(page.key, 0)
            onFileActivated: path => page.fileActivated(path)
            onOpenInNewTab: path => page.openInNewTab(path)
            onMenuRequested: (row, x, y) => page.menuRequested(left, row, x, y)
            onReported: page.report(0, left)
            onCopyText: words => page.copyText(words)
        }
        Rectangle {
            visible: page.split
            Layout.fillHeight: true
            Layout.preferredWidth: 1
            color: Tokens.colors.border
        }
        Loader {
            id: right

            Layout.fillWidth: true
            Layout.fillHeight: true
            active: page.split
            visible: active
            sourceComponent: Pane {
                id: rightPane

                startFolder: page.rightStart
                split: true
                current: page.activePane === 1
                onEntered: page.tabs.setActivePane(page.key, 1)
                onFileActivated: path => page.fileActivated(path)
                onOpenInNewTab: path => page.openInNewTab(path)
                onMenuRequested: (row, x, y) => page.menuRequested(rightPane, row, x, y)
                onReported: page.report(1, rightPane)
                onCopyText: words => page.copyText(words)
                Component.onCompleted: focusList()
            }
        }
    }
}
