// One tab: one pane, or two side by side (F3). It tells the tabs model
// where its panes are and which has the keyboard.
pragma ComponentBehavior: Bound

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
    required property list<string> select
    required property bool current
    required property TabsModel tabs
    required property PlacesModel places
    readonly property Pane activeItem: split && activePane === 1 && rightLoader.item ? rightLoader.item as Pane : leftPane

    signal fileActivated(string uri)
    signal openInNewTab(string uri)
    signal menuRequested(Pane pane, int row, real x, real y)
    signal copyText(string words)
    signal dialogRequested(string kind, var data)
    signal otherPane

    function report(side: int, pane: Pane) {
        const folder = pane.listing
        const icon = folder.trash ? "trash" : (page.places.iconFor(folder.locationUri) || "folder")
        page.tabs.setPane(page.key, side, folder.locationUri, folder.title, icon)
    }

    // Places arrive after the first folder: its icon is looked up again.
    Connections {
        target: page.places
        function onCountChanged() {
            page.report(0, leftPane)
            if (rightLoader.item)
                page.report(1, rightLoader.item as Pane)
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Pane {
            id: leftPane

            Layout.fillWidth: true
            Layout.fillHeight: true
            startFolder: page.leftStart
            startSelect: page.select
            split: page.split
            current: !page.split || page.activePane === 0
            onEntered: page.tabs.setActivePane(page.key, 0)
            onFileActivated: uri => page.fileActivated(uri)
            onOpenInNewTab: uri => page.openInNewTab(uri)
            onMenuRequested: (row, x, y) => page.menuRequested(leftPane, row, x, y)
            onReported: page.report(0, leftPane)
            onCopyText: words => page.copyText(words)
            onDialogRequested: (kind, data) => page.dialogRequested(kind, data)
            onOtherPane: page.otherPane()
        }
        Rectangle {
            visible: page.split
            Layout.fillHeight: true
            Layout.preferredWidth: 1
            color: Tokens.colors.border
        }
        Loader {
            id: rightLoader

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
                onFileActivated: uri => page.fileActivated(uri)
                onOpenInNewTab: uri => page.openInNewTab(uri)
                onMenuRequested: (row, x, y) => page.menuRequested(rightPane, row, x, y)
                onReported: page.report(1, rightPane)
                onCopyText: words => page.copyText(words)
                onDialogRequested: (kind, data) => page.dialogRequested(kind, data)
                onOtherPane: page.otherPane()
                Component.onCompleted: focusList()
            }
        }
    }
}
