// Rustphin's Qt window (W-114c): its own title bar, the toolbar, the
// details list and the status line, drawn with the tokens of
// docs/DESIGN.md. State and decisions stay in Rust (FolderModel,
// ToolbarModel, Tokens); this file lays out and binds.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Window {
    id: root

    required property string startFolder

    readonly property var colors: Tokens.colors
    readonly property var metrics: Tokens.metrics
    readonly property bool maximised: visibility === Window.Maximized || visibility === Window.FullScreen

    // A bundled Lucide icon in one of the theme's colours.
    function lucide(name: string, colour: string): string {
        return "image://lucide/" + name + "/" + colour.slice(1)
    }

    function toggleMaximised() {
        if (maximised)
            showNormal()
        else
            showMaximized()
    }

    width: 1000
    height: 700
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint
    color: Tokens.translucent ? "transparent" : colors.window
    title: qsTr("%1 — Rustphin").arg(listing.location.split("/").pop() || "/")

    Component.onCompleted: {
        Tokens.blurWindows()
        listing.open(startFolder)
    }

    FolderModel {
        id: listing
    }

    ToolbarModel {
        id: toolbarItems
    }

    // A plain icon button: a 16-pixel Lucide icon on its square, its name
    // read out and shown as a tooltip.
    component IconButton: ToolButton {
        id: button

        required property string iconName
        property string tip: text
        property int side: 32
        property color iconColour: enabled ? root.colors.text : root.colors.text_disabled
        property color hoverFill: root.colors.hover

        display: AbstractButton.IconOnly
        focusPolicy: Qt.TabFocus
        implicitWidth: side
        implicitHeight: side
        ToolTip.visible: hovered && tip.length > 0
        ToolTip.delay: 600
        ToolTip.text: tip
        Accessible.checked: checked

        contentItem: Image {
            source: root.lucide(button.iconName, button.iconColour.toString())
            sourceSize.width: 16
            sourceSize.height: 16
            fillMode: Image.Pad
        }
        background: Rectangle {
            radius: root.metrics.radius
            color: button.checked || button.down ? root.colors.current
                 : button.hovered ? button.hoverFill : "transparent"
            border.width: button.visualFocus ? 2 : 0
            border.color: root.colors.accent
        }
    }

    // A plain text button with the design's outline.
    component OutlinedButton: Button {
        id: plain

        implicitHeight: 32
        leftPadding: root.metrics.space_md
        rightPadding: root.metrics.space_md

        contentItem: Text {
            text: plain.text
            color: plain.enabled ? root.colors.text : root.colors.text_disabled
            font.family: root.metrics.family_body
            font.pixelSize: root.metrics.body_text
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: root.metrics.radius
            color: plain.down ? root.colors.current : plain.hovered ? root.colors.hover : "transparent"
            border.width: plain.visualFocus ? 2 : 1
            border.color: plain.visualFocus ? root.colors.accent : root.colors.border_strong
        }
    }

    Rectangle {
        anchors.fill: parent
        color: root.colors.window
        // Without the compositor's frame, a hairline marks the window's edge.
        border.width: root.maximised ? 0 : 1
        border.color: root.colors.border_strong
    }

    Rectangle {
        id: titleBar

        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
            margins: root.maximised ? 0 : 1
        }
        height: 36
        color: root.colors.chrome

        Image {
            anchors {
                left: parent.left
                leftMargin: root.metrics.space_md
                verticalCenter: parent.verticalCenter
            }
            source: "qrc:/qt/qml/Rustphin/logo.svg"
            sourceSize.width: 16
            sourceSize.height: 16
            Accessible.ignored: true
        }
        Text {
            anchors.centerIn: parent
            width: parent.width - 2 * windowButtons.width
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideMiddle
            color: root.colors.text_muted
            font.family: root.metrics.family_body
            font.pixelSize: root.metrics.body_text
            text: root.title
        }

        // Dragging moves the window; a double click maximises or restores.
        DragHandler {
            target: null
            grabPermissions: PointerHandler.TakeOverForbidden
            onActiveChanged: {
                if (active)
                    root.startSystemMove()
            }
        }
        TapHandler {
            onDoubleTapped: root.toggleMaximised()
        }

        Row {
            id: windowButtons

            anchors {
                right: parent.right
                top: parent.top
                bottom: parent.bottom
            }

            IconButton {
                iconName: "minus"
                text: qsTr("Minimise")
                side: 36
                implicitWidth: 44
                onClicked: root.showMinimized()
            }
            IconButton {
                iconName: root.maximised ? "copy" : "square"
                text: root.maximised ? qsTr("Restore") : qsTr("Maximise")
                side: 36
                implicitWidth: 44
                onClicked: root.toggleMaximised()
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close window")
                side: 36
                implicitWidth: 44
                hoverFill: root.colors.danger_surface
                onClicked: root.close()
            }
        }
    }

    Rectangle {
        id: toolbar

        anchors {
            top: titleBar.bottom
            left: parent.left
            right: parent.right
            leftMargin: root.maximised ? 0 : 1
            rightMargin: root.maximised ? 0 : 1
        }
        height: root.metrics.toolbar
        color: root.colors.chrome

        onWidthChanged: toolbarItems.fit(width)
        Component.onCompleted: toolbarItems.fit(width)

        RowLayout {
            anchors {
                fill: parent
                leftMargin: root.metrics.space_sm
                rightMargin: root.metrics.space_sm
            }
            spacing: root.metrics.space_xs

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
                    Layout.minimumWidth: kind === "location" ? 280 : -1
                    Layout.alignment: Qt.AlignVCenter
                    sourceComponent: kind === "separator" ? separator
                                   : kind === "location" ? location
                                   : kind === "actions" ? actions
                                   : button

                    Component {
                        id: separator

                        Item {
                            implicitWidth: 1 + 2 * root.metrics.space_xs
                            implicitHeight: 20
                            Accessible.ignored: true

                            Rectangle {
                                anchors.centerIn: parent
                                width: 1
                                height: 20
                                color: root.colors.border
                            }
                        }
                    }

                    Component {
                        id: button

                        IconButton {
                            iconName: entry.icon
                            text: entry.name
                            tip: entry.tip
                            enabled: entry.available && (entry.itemId === "back" ? listing.canGoBack
                                                        : entry.itemId === "forward" ? listing.canGoForward
                                                        : entry.itemId === "up" ? listing.canGoUp
                                                        : true)
                            checkable: entry.itemId === "hidden"
                            checked: entry.itemId === "hidden" && listing.showHidden
                            implicitWidth: entry.kind === "menu" ? 48 : 32
                            onClicked: listing.run(entry.itemId)

                            // A menu button shows a 12-pixel chevron after its icon.
                            Image {
                                visible: entry.kind === "menu"
                                anchors {
                                    right: parent.right
                                    rightMargin: root.metrics.space_xs
                                    verticalCenter: parent.verticalCenter
                                }
                                source: root.lucide("chevron-down", root.colors.text_disabled)
                                sourceSize.width: 12
                                sourceSize.height: 12
                            }
                        }
                    }

                    Component {
                        id: location

                        Rectangle {
                            implicitHeight: 32
                            radius: root.metrics.radius
                            color: root.colors.content
                            border.width: 1
                            border.color: root.colors.border_strong

                            Image {
                                id: folderIcon

                                anchors {
                                    left: parent.left
                                    leftMargin: root.metrics.space_sm
                                    verticalCenter: parent.verticalCenter
                                }
                                source: root.lucide("folder", root.colors.text_muted)
                                sourceSize.width: 16
                                sourceSize.height: 16
                                Accessible.ignored: true
                            }
                            Text {
                                anchors {
                                    left: folderIcon.right
                                    right: parent.right
                                    top: parent.top
                                    bottom: parent.bottom
                                    leftMargin: root.metrics.space_sm
                                    rightMargin: root.metrics.space_md
                                }
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideMiddle
                                color: root.colors.text
                                font.family: root.metrics.family_data
                                font.pixelSize: root.metrics.data_text
                                text: listing.location
                                Accessible.role: Accessible.StaticText
                                Accessible.name: qsTr("Location: %1").arg(listing.location)
                            }
                        }
                    }

                    Component {
                        id: actions

                        OutlinedButton {
                            enabled: entry.available
                            text: entry.name
                            Accessible.name: entry.name
                            ToolTip.visible: hovered
                            ToolTip.delay: 600
                            ToolTip.text: entry.tip

                            contentItem: Row {
                                spacing: root.metrics.space_sm

                                Image {
                                    visible: !toolbarItems.actionsWords
                                    anchors.verticalCenter: parent.verticalCenter
                                    source: root.lucide("menu", root.colors.text_disabled)
                                    sourceSize.width: 16
                                    sourceSize.height: 16
                                }
                                Text {
                                    visible: toolbarItems.actionsWords
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: root.colors.text_disabled
                                    font.family: root.metrics.family_body
                                    font.pixelSize: root.metrics.body_text
                                    text: entry.name
                                }
                                Text {
                                    visible: toolbarItems.actionsWords && toolbarItems.actionsHint
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: root.colors.text_disabled
                                    font.family: root.metrics.family_data
                                    font.pixelSize: root.metrics.small_text
                                    text: qsTr("Ctrl+Shift+P")
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Column headings; clicking one sorts by it.
    Row {
        id: headings

        anchors {
            top: toolbar.bottom
            left: parent.left
            right: parent.right
            leftMargin: 1
            rightMargin: 1
        }
        height: root.metrics.heading_row

        Repeater {
            model: [
                { label: qsTr("Name"), column: 0 },
                { label: qsTr("Size"), column: 1 },
                { label: qsTr("Modified"), column: 2 }
            ]
            delegate: Item {
                id: heading

                required property var modelData

                width: modelData.column === 0 ? headings.width - 360 : 180
                height: headings.height
                Accessible.role: Accessible.ColumnHeader
                Accessible.name: qsTr("Sort by %1").arg(modelData.label)

                Text {
                    // Sizes are right-aligned, so their heading is too.
                    anchors {
                        verticalCenter: parent.verticalCenter
                        left: heading.modelData.column === 1 ? undefined : parent.left
                        right: heading.modelData.column === 1 ? parent.right : undefined
                        leftMargin: root.metrics.space_lg
                        rightMargin: root.metrics.space_lg
                    }
                    color: root.colors.text_muted
                    font.family: root.metrics.family_body
                    font.pixelSize: root.metrics.small_text
                    text: heading.modelData.label
                }
                TapHandler {
                    onTapped: listing.sortBy(heading.modelData.column, false)
                }
            }
        }
    }

    Rectangle {
        anchors {
            top: headings.bottom
            left: parent.left
            right: parent.right
            bottom: status.top
            leftMargin: 1
            rightMargin: 1
        }
        color: root.colors.content

        ListView {
            id: list

            anchors.fill: parent
            model: listing
            clip: true
            focus: true
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            keyNavigationEnabled: true
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Entries")

            // A newly shown folder starts with nothing highlighted, as in Dolphin.
            Connections {
                target: listing
                function onLocationChanged() {
                    list.currentIndex = -1
                }
            }

            Keys.onReturnPressed: listing.activate(currentIndex)
            Keys.onEnterPressed: listing.activate(currentIndex)

            delegate: Rectangle {
                id: row

                required property int index
                required property string name
                required property string size
                required property string modified
                required property bool folder
                required property bool hidden
                required property string icon

                readonly property bool current: ListView.isCurrentItem
                // Hidden entries are faded, as in Dolphin; their dot says so too.
                readonly property real fade: hidden ? root.metrics.hidden_opacity : 1

                width: ListView.view.width
                height: root.metrics.row
                color: current ? root.colors.selection
                               : hover.hovered ? root.colors.hover : "transparent"
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: current

                Image {
                    x: root.metrics.space_lg
                    anchors.verticalCenter: parent.verticalCenter
                    source: "image://theme/" + row.icon
                    sourceSize.width: root.metrics.icon
                    sourceSize.height: root.metrics.icon
                    opacity: row.fade
                    Accessible.ignored: true
                }
                Text {
                    x: root.metrics.space_lg + root.metrics.icon + root.metrics.space_sm
                    width: parent.width - x - 360
                    anchors.verticalCenter: parent.verticalCenter
                    elide: Text.ElideRight
                    opacity: row.fade
                    color: row.current ? root.colors.on_selection : root.colors.text
                    font.family: root.metrics.family_body
                    font.pixelSize: root.metrics.name_text
                    text: row.name
                }
                Text {
                    x: parent.width - 360
                    width: 180 - root.metrics.space_lg
                    horizontalAlignment: Text.AlignRight
                    anchors.verticalCenter: parent.verticalCenter
                    opacity: row.fade
                    color: row.current ? root.colors.on_selection : root.colors.text_muted
                    font.family: root.metrics.family_data
                    font.pixelSize: root.metrics.data_text
                    text: row.size
                }
                Text {
                    x: parent.width - 180 + root.metrics.space_lg
                    anchors.verticalCenter: parent.verticalCenter
                    opacity: row.fade
                    color: row.current ? root.colors.on_selection : root.colors.text_muted
                    font.family: root.metrics.family_data
                    font.pixelSize: root.metrics.data_text
                    text: row.modified
                }
                HoverHandler {
                    id: hover
                }
                TapHandler {
                    onTapped: {
                        list.currentIndex = row.index
                        list.forceActiveFocus()
                    }
                    onDoubleTapped: listing.activate(row.index)
                }
            }
        }

        // Said in place when the folder cannot be shown (mockup 28), with
        // the ways on.
        Column {
            anchors.centerIn: parent
            width: parent.width - 2 * root.metrics.space_lg
            spacing: root.metrics.space_md
            visible: listing.problem.length > 0

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                color: root.colors.danger
                font.family: root.metrics.family_body
                font.pixelSize: root.metrics.body_text
                text: listing.problem
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: root.metrics.space_sm

                OutlinedButton {
                    visible: listing.canGoUp
                    text: qsTr("Go up")
                    onClicked: listing.up()
                }
                OutlinedButton {
                    text: qsTr("Home folder")
                    onClicked: listing.home()
                }
            }
        }
    }

    Rectangle {
        id: status

        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
            margins: root.maximised ? 0 : 1
        }
        height: root.metrics.heading_row
        color: root.colors.chrome

        Text {
            anchors {
                verticalCenter: parent.verticalCenter
                left: parent.left
                right: parent.right
                leftMargin: root.metrics.space_md
                rightMargin: root.metrics.space_md
            }
            elide: Text.ElideRight
            color: listing.notice.length > 0 ? root.colors.danger : root.colors.text_muted
            font.family: root.metrics.family_data
            font.pixelSize: root.metrics.small_text
            text: listing.notice.length > 0 ? listing.notice : listing.summary
        }
    }

    // Bands along the edges resize the window (4 pixels; 16-pixel corners).
    Repeater {
        model: [
            { edges: Qt.LeftEdge, shape: Qt.SizeHorCursor, l: 0, t: 16, w: 4, h: -32 },
            { edges: Qt.RightEdge, shape: Qt.SizeHorCursor, l: -4, t: 16, w: 4, h: -32 },
            { edges: Qt.TopEdge, shape: Qt.SizeVerCursor, l: 16, t: 0, w: -32, h: 4 },
            { edges: Qt.BottomEdge, shape: Qt.SizeVerCursor, l: 16, t: -4, w: -32, h: 4 },
            { edges: Qt.TopEdge | Qt.LeftEdge, shape: Qt.SizeFDiagCursor, l: 0, t: 0, w: 16, h: 16 },
            { edges: Qt.TopEdge | Qt.RightEdge, shape: Qt.SizeBDiagCursor, l: -16, t: 0, w: 16, h: 16 },
            { edges: Qt.BottomEdge | Qt.LeftEdge, shape: Qt.SizeBDiagCursor, l: 0, t: -16, w: 16, h: 16 },
            { edges: Qt.BottomEdge | Qt.RightEdge, shape: Qt.SizeFDiagCursor, l: -16, t: -16, w: 16, h: 16 }
        ]
        delegate: MouseArea {
            required property var modelData

            visible: !root.maximised
            x: modelData.l < 0 ? root.width + modelData.l : modelData.l
            y: modelData.t < 0 ? root.height + modelData.t : modelData.t
            width: modelData.w <= 0 ? root.width + modelData.w : modelData.w
            height: modelData.h <= 0 ? root.height + modelData.h : modelData.h
            cursorShape: modelData.shape
            onPressed: root.startSystemResize(modelData.edges)
        }
    }

    Shortcut {
        sequences: ["Backspace", "Alt+Left", StandardKey.Back]
        onActivated: listing.back()
    }
    Shortcut {
        sequences: ["Alt+Right", StandardKey.Forward]
        onActivated: listing.forward()
    }
    Shortcut {
        sequence: "Alt+Up"
        onActivated: listing.up()
    }
    Shortcut {
        sequence: "Alt+Home"
        onActivated: listing.home()
    }
    Shortcut {
        sequences: [StandardKey.Refresh]
        onActivated: listing.reload()
    }
    Shortcut {
        sequences: ["Ctrl+H", "Alt+."]
        onActivated: listing.run("hidden")
    }
    Shortcut {
        sequence: "Shift+F4"
        onActivated: listing.run("terminal")
    }
}
