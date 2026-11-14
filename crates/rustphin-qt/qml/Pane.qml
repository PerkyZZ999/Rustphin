// One browsing pane (W-114d): the folder's entries as icons, compact
// columns or details, with selection, rubber band, keyboard movement,
// type-ahead and renaming in place; its in-place states and its status
// line. FolderModel decides everything; this file draws and forwards input.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

FocusScope {
    id: pane

    required property string startFolder
    // Names to select once the start folder is listed.
    property var startSelect: []
    // This pane receives the window's commands (the active one of a split).
    property bool current: true
    // Shown beside another pane: its location is drawn on its own bar.
    property bool split: false
    readonly property FolderModel listing: folder
    readonly property alias view: viewLoader.item
    property int hoverIndex: -1
    property bool filterOpen: false

    // A file was activated; the window opens it.
    signal fileActivated(string path)
    // A folder asks for a new tab (middle click).
    signal openInNewTab(string path)
    // The context menu: on `row` (-1 for the folder) at a point in the pane.
    signal menuRequested(int row, real x, real y)
    // The pane took the keyboard.
    signal entered()
    // Text for the clipboard.
    signal copyText(string words)
    // The pane shows another folder, or its title or trash state changed.
    signal reported()

    // Entries per line (icons) or per column (compact); one in details.
    function lanes(): int {
        if (!view)
            return 1
        if (folder.mode === 0)
            return Math.max(1, Math.floor(view.width / folder.cellWidth))
        if (folder.mode === 1)
            return Math.max(1, Math.floor(view.height / folder.rowHeight))
        return 1
    }

    function showFilter() {
        filterOpen = true
        filterField.forceActiveFocus()
        filterField.selectAll()
    }

    function closeFilter() {
        filterOpen = false
        filterField.text = ""
        folder.setFilter("")
        keys.forceActiveFocus()
    }

    // Letters typed over the list jump to a name.
    function typeAhead(event) {
        if (event.text.length > 0 && event.text.charCodeAt(0) >= 32) {
            folder.typeAhead(event.text)
            event.accepted = true
        }
    }

    function focusList() {
        keys.forceActiveFocus()
    }

    // The point of the menu for the keyboard (Shift+F10): under the focused
    // entry, or the pane's corner.
    function menuForKeyboard() {
        const row = folder.focusRow
        const item = row >= 0 && view ? view.itemAtIndex(row) : null
        if (item && folder.isSelected(row)) {
            const point = item.mapToItem(pane, 0, item.height)
            pane.menuRequested(row, point.x, point.y)
        } else {
            pane.menuRequested(-1, body.x + Tokens.metrics.space_lg, body.y + Tokens.metrics.space_lg)
        }
    }

    onWidthChanged: folder.setWidth(width)
    Component.onCompleted: {
        folder.setWidth(width)
        if (startSelect && startSelect.length > 0)
            folder.reveal(startFolder, startSelect)
        else
            folder.openTyped(startFolder)
    }
    // The tab joined its views and this one goes on with the other's folder.
    onStartFolderChanged: {
        if (startFolder.length > 0 && startFolder !== folder.location)
            folder.openTyped(startFolder)
    }
    onActiveFocusChanged: {
        if (activeFocus)
            pane.entered()
    }

    FolderModel {
        id: folder

        onFileActivated: path => pane.fileActivated(path)
        onOpenInNewTab: path => pane.openInNewTab(path)
        onCopyText: words => pane.copyText(words)
        onRevealRow: row => {
            if (pane.view)
                pane.view.positionViewAtIndex(row, ListView.Contain)
        }
        onTitleChanged: pane.reported()
        onTrashChanged: pane.reported()
        onLocationChanged: {
            pane.reported()
            pane.hoverIndex = -1
            if (pane.filterOpen && folder.filterText.length === 0)
                pane.filterOpen = false
        }
    }

    Connections {
        target: Preferences
        function onRevisionChanged() {
            folder.adoptSettings()
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Tokens.colors.content
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // A folder that failed to re-read keeps its listing under this banner.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.control
            visible: folder.banner.length > 0
            color: Tokens.colors.danger_surface
            border.color: Tokens.colors.danger_border
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_xs
                spacing: Tokens.metrics.space_sm

                Lucide {
                    name: "triangle-alert"
                    colour: Tokens.colors.danger
                }
                Text {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: folder.banner
                }
            }
        }

        // The trash explains itself and offers its actions.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: trashColumn.implicitHeight + 2 * Tokens.metrics.space_sm
            visible: folder.trash
            color: Tokens.colors.chrome

            ColumnLayout {
                id: trashColumn

                anchors.fill: parent
                anchors.margins: Tokens.metrics.space_sm
                spacing: Tokens.metrics.space_xs
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Trash")
                Accessible.description: qsTr("Items stay here until the trash is emptied.")

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_sm

                    Lucide {
                        name: "trash"
                        colour: Tokens.colors.text
                    }
                    Text {
                        Layout.fillWidth: true
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: folder.title
                    }
                    OutlinedButton {
                        id: trashActions

                        text: qsTr("Trash actions")
                        ToolTip.visible: hovered
                        ToolTip.delay: 600
                        ToolTip.text: qsTr("Restore, empty or browse another trash; also available in Actions")
                        onClicked: {
                            if (folder.openMenu("trash"))
                                trashMenu.openAt(trashActions, 0, trashActions.height)
                        }
                    }
                }
                Text {
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: qsTr("Items stay here until the trash is emptied.")
                }
            }
        }

        // The location of a pane in a split view, so each says where it is.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.path_bar
            visible: pane.split
            color: pane.current ? Tokens.colors.chrome_raised : Tokens.colors.chrome

            Text {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_md
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideMiddle
                color: pane.current ? Tokens.colors.text : Tokens.colors.text_muted
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.data_text
                text: folder.location
            }
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: Tokens.metrics.selection_bar
                color: Tokens.colors.accent
                visible: pane.current
            }
        }

        // Column headings of the details view; a click sorts by the column,
        // again reverses it.
        Rectangle {
            id: headings

            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.heading_row
            visible: folder.mode === 2
            color: Tokens.colors.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_lg
                anchors.rightMargin: Tokens.metrics.space_lg
                spacing: 0

                Repeater {
                    model: [
                        { label: qsTr("Name"), column: 0, shown: true, width: -1 },
                        { label: qsTr("Size"), column: 1, shown: folder.showSize, width: Tokens.metrics.column_size },
                        { label: qsTr("Modified"), column: 2, shown: folder.showModified, width: Tokens.metrics.column_modified },
                        { label: qsTr("Type"), column: 3, shown: folder.showType, width: Tokens.metrics.column_type }
                    ]
                    delegate: Item {
                        id: heading

                        required property var modelData
                        readonly property bool sorted: folder.sortColumn === modelData.column

                        visible: modelData.shown
                        Layout.fillWidth: modelData.width < 0
                        Layout.preferredWidth: modelData.width < 0 ? 100 : modelData.width
                        Layout.fillHeight: true
                        Accessible.role: Accessible.ColumnHeader
                        Accessible.name: sorted ? qsTr("%1, sorted %2").arg(modelData.label).arg(folder.descending ? qsTr("descending") : qsTr("ascending"))
                                                : qsTr("Sort by %1").arg(modelData.label)

                        Row {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.right: heading.modelData.column === 1 ? parent.right : undefined
                            anchors.left: heading.modelData.column === 1 ? undefined : parent.left
                            anchors.leftMargin: heading.modelData.column === 0 ? 0 : Tokens.metrics.space_md
                            spacing: Tokens.metrics.space_xs

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                color: heading.sorted ? Tokens.colors.text : Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.small_text
                                font.weight: heading.sorted ? Font.DemiBold : Font.Normal
                                text: heading.modelData.label
                            }
                            Lucide {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: heading.sorted
                                name: folder.descending ? "arrow-down" : "arrow-up"
                                side: Tokens.metrics.icon_small
                                colour: Tokens.colors.text
                            }
                        }
                        HoverHandler {
                            id: headingHover
                        }
                        Rectangle {
                            anchors.fill: parent
                            color: headingHover.hovered ? Tokens.colors.hover : "transparent"
                            z: -1
                        }
                        TapHandler {
                            onTapped: folder.sortBy(heading.modelData.column)
                        }
                    }
                }
            }
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Tokens.colors.border
            }
        }

        Item {
            id: body

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            // Keys over the list: movement, type-ahead and the list's own
            // commands. Keys that also edit text stay here, so they never
            // fire while a field is typed in.
            Item {
                id: keys

                anchors.fill: parent
                focus: true
                Accessible.ignored: true

                Keys.onPressed: event => {
                    const shift = (event.modifiers & Qt.ShiftModifier) !== 0
                    const ctrl = (event.modifiers & Qt.ControlModifier) !== 0
                    const alt = (event.modifiers & Qt.AltModifier) !== 0
                    const steps = {
                        [Qt.Key_Up]: 0, [Qt.Key_Down]: 1, [Qt.Key_Left]: 2, [Qt.Key_Right]: 3,
                        [Qt.Key_PageUp]: 4, [Qt.Key_PageDown]: 5, [Qt.Key_Home]: 6, [Qt.Key_End]: 7
                    }
                    if (!ctrl && !alt && steps[event.key] !== undefined) {
                        folder.moveFocus(steps[event.key], shift, pane.lanes())
                        event.accepted = true
                        return
                    }
                    if (alt || (ctrl && event.key !== Qt.Key_A && event.key !== Qt.Key_N))
                        return
                    switch (event.key) {
                    case Qt.Key_Return:
                    case Qt.Key_Enter:
                        folder.activateSelected()
                        break
                    case Qt.Key_Backspace:
                        folder.back()
                        break
                    case Qt.Key_A:
                        if (!ctrl)
                            return pane.typeAhead(event)
                        if (shift)
                            folder.invertSelection()
                        else
                            folder.selectAll()
                        break
                    case Qt.Key_N:
                        if (!(ctrl && shift))
                            return pane.typeAhead(event)
                        folder.createEntry(true)
                        break
                    case Qt.Key_F2:
                        folder.beginRename()
                        break
                    case Qt.Key_F10:
                        if (shift)
                            pane.menuForKeyboard()
                        else
                            folder.createEntry(true)
                        break
                    case Qt.Key_Menu:
                        pane.menuForKeyboard()
                        break
                    case Qt.Key_Slash:
                        pane.showFilter()
                        break
                    case Qt.Key_Escape:
                        if (pane.filterOpen)
                            pane.closeFilter()
                        else
                            return
                        break
                    default:
                        return pane.typeAhead(event)
                    }
                    event.accepted = true
                }
            }

            Loader {
                id: viewLoader

                anchors.fill: parent
                anchors.topMargin: folder.mode === 2 ? 0 : Tokens.metrics.space_sm
                anchors.leftMargin: folder.mode === 2 ? 0 : Tokens.metrics.space_sm
                anchors.rightMargin: folder.mode === 2 ? 0 : Tokens.metrics.space_sm
                sourceComponent: folder.mode === 0 ? iconsView : folder.mode === 1 ? compactView : detailsView
                visible: folder.problem.length === 0
            }

            // Pointer input over the view: selection, rubber band, menus.
            MouseArea {
                id: pointer

                property int pressRow: -1
                property point bandStart
                property point bandEnd
                property bool banding: false
                property bool bandAdd: false
                property point pressedAt
                property bool moved: false

                function contentPoint(x: real, y: real): point {
                    return pane.view.contentItem.mapFromItem(pointer, x, y)
                }

                function rowAt(x: real, y: real): int {
                    if (!pane.view)
                        return -1
                    const p = contentPoint(x, y)
                    return pane.view.indexAt(p.x, p.y)
                }

                function modifiersOf(mouse): int {
                    return ((mouse.modifiers & Qt.ControlModifier) ? 1 : 0) | ((mouse.modifiers & Qt.ShiftModifier) ? 2 : 0)
                }

                anchors.fill: viewLoader
                visible: viewLoader.visible
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton

                onPressed: mouse => {
                    keys.forceActiveFocus()
                    const row = rowAt(mouse.x, mouse.y)
                    pressRow = row
                    pressedAt = Qt.point(mouse.x, mouse.y)
                    moved = false
                    if (mouse.button === Qt.MiddleButton) {
                        if (row >= 0)
                            folder.middleClick(row)
                        return
                    }
                    if (mouse.button === Qt.RightButton) {
                        if (row >= 0 && !folder.isSelected(row))
                            folder.click(row, 0)
                        else if (row < 0)
                            folder.clearSelection()
                        const p = pointer.mapToItem(pane, mouse.x, mouse.y)
                        pane.menuRequested(row, p.x, p.y)
                        return
                    }
                    if (row < 0) {
                        bandAdd = (mouse.modifiers & Qt.ControlModifier) !== 0
                        folder.beginBand()
                        if (!bandAdd)
                            folder.clearSelection()
                        bandStart = contentPoint(mouse.x, mouse.y)
                        bandEnd = bandStart
                        banding = true
                    } else {
                        folder.press(row, modifiersOf(mouse))
                    }
                }
                onPositionChanged: mouse => {
                    if (!pressed) {
                        pane.hoverIndex = rowAt(mouse.x, mouse.y)
                        return
                    }
                    if (Math.abs(mouse.x - pressedAt.x) + Math.abs(mouse.y - pressedAt.y) > 4)
                        moved = true
                    if (banding) {
                        bandEnd = contentPoint(Math.max(0, Math.min(width, mouse.x)), Math.max(0, Math.min(height, mouse.y)))
                        folder.band(bandStart.x, bandStart.y, bandEnd.x, bandEnd.y, pane.lanes(), bandAdd)
                        bandScroll.direction = mouse.y < 0 ? -1 : mouse.y > height ? 1 : 0
                    }
                }
                onReleased: mouse => {
                    // A press on a selected entry kept the selection for a
                    // drag; without one, the click narrows it to the entry.
                    if (mouse.button === Qt.LeftButton && !banding && !moved && pressRow >= 0
                            && modifiersOf(mouse) === 0)
                        folder.click(pressRow, 0)
                    banding = false
                    bandScroll.direction = 0
                }
                onClicked: mouse => {
                    if (mouse.button === Qt.LeftButton && folder.oneClick && pressRow >= 0 && !moved
                            && modifiersOf(mouse) === 0)
                        folder.activate(pressRow)
                }
                onDoubleClicked: mouse => {
                    if (mouse.button === Qt.LeftButton && !folder.oneClick && pressRow >= 0)
                        folder.activate(pressRow)
                }
                onExited: pane.hoverIndex = -1
                onCanceled: {
                    banding = false
                    bandScroll.direction = 0
                }

                // The band scrolls the view while held past its edge.
                Timer {
                    id: bandScroll

                    property int direction: 0

                    interval: 30
                    repeat: true
                    running: direction !== 0 && pointer.banding
                    onTriggered: {
                        const view = pane.view
                        if (folder.mode === 1)
                            view.contentX = Math.max(view.originX, Math.min(view.contentX + direction * 24, view.originX + view.contentWidth - view.width))
                        else
                            view.contentY = Math.max(view.originY, Math.min(view.contentY + direction * 24, view.originY + view.contentHeight - view.height))
                    }
                }
            }

            // The rubber band, drawn in view coordinates.
            Rectangle {
                readonly property point a: pane.view && pointer.banding ? pane.view.contentItem.mapToItem(body, pointer.bandStart.x, pointer.bandStart.y) : Qt.point(0, 0)
                readonly property point b: pane.view && pointer.banding ? pane.view.contentItem.mapToItem(body, pointer.bandEnd.x, pointer.bandEnd.y) : Qt.point(0, 0)

                visible: pointer.banding
                x: Math.min(a.x, b.x)
                y: Math.min(a.y, b.y)
                width: Math.abs(a.x - b.x)
                height: Math.abs(a.y - b.y)
                color: Qt.alpha(Tokens.colors.accent, 0.15)
                border.color: Tokens.colors.accent
                border.width: 1
            }

            // Said in place: an empty folder, nothing matching the filter,
            // or a folder that cannot be shown (mockup 28), with ways on.
            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 2 * Tokens.metrics.space_lg, Tokens.metrics.state_text_max)
                spacing: Tokens.metrics.space_md
                visible: folder.problem.length > 0 || (folder.empty && !folder.loading) || (folder.empty && folder.count === 0 && folder.loading)

                Lucide {
                    Layout.alignment: Qt.AlignHCenter
                    name: folder.problem.length > 0 ? "folder-x"
                        : folder.loading ? "loader"
                        : folder.filterText.length > 0 ? "search-x"
                        : folder.emptyTitle.indexOf("hidden") >= 0 ? "eye-off" : "folder-open"
                    side: Tokens.metrics.icon_large
                    colour: folder.problem.length > 0 ? Tokens.colors.danger : Tokens.colors.text_muted
                }
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_heading
                    font.pixelSize: Tokens.metrics.heading_text
                    text: folder.problem.length > 0 ? folder.problem : folder.emptyTitle
                    Accessible.role: Accessible.Heading
                }
                Text {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: folder.problem.length > 0 ? folder.problemText : folder.emptyText
                }
                Row {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: Tokens.metrics.space_sm

                    OutlinedButton {
                        visible: folder.problem.length > 0 && folder.wayLabel.length > 0
                        text: folder.wayLabel
                        iconName: folder.canGoBack ? "arrow-left" : "arrow-up"
                        onClicked: folder.canGoBack ? folder.back() : folder.up()
                    }
                    OutlinedButton {
                        visible: folder.problem.length > 0
                        text: qsTr("Home folder")
                        iconName: "house"
                        onClicked: folder.home()
                    }
                    OutlinedButton {
                        visible: folder.problem.length === 0 && folder.emptyTitle.indexOf("hidden") >= 0
                        text: qsTr("Show hidden")
                        iconName: "eye"
                        ToolTip.visible: hovered
                        ToolTip.delay: 600
                        ToolTip.text: qsTr("Show hidden files (Ctrl+H)")
                        onClicked: {
                            folder.setHidden(true)
                            keys.forceActiveFocus()
                        }
                    }
                    OutlinedButton {
                        visible: folder.problem.length === 0 && folder.filterText.length > 0 && folder.empty
                        text: qsTr("Clear filter")
                        iconName: "funnel-x"
                        ToolTip.visible: hovered
                        ToolTip.delay: 600
                        ToolTip.text: qsTr("Clear the filter (Esc in the filter bar)")
                        onClicked: pane.closeFilter()
                    }
                }
            }
        }

        // Dolphin's filter bar: shows only names containing the text.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.filter_bar
            visible: pane.filterOpen
            color: Tokens.colors.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_xs
                spacing: Tokens.metrics.space_sm

                Lucide {
                    name: "funnel"
                }
                TextField {
                    id: filterField

                    Layout.fillWidth: true
                    Layout.maximumWidth: Tokens.metrics.filter_field_max
                    implicitHeight: Tokens.metrics.filter_field
                    placeholderText: qsTr("Filter by name")
                    placeholderTextColor: Tokens.colors.text_muted
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    Accessible.name: qsTr("Filter")
                    background: Rectangle {
                        radius: Tokens.metrics.radius
                        color: Tokens.colors.content
                        border.width: filterField.activeFocus ? 2 : 1
                        border.color: filterField.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                    }
                    onTextChanged: folder.setFilter(text)
                    Keys.onEscapePressed: pane.closeFilter()
                    Keys.onDownPressed: keys.forceActiveFocus()
                    Keys.onReturnPressed: keys.forceActiveFocus()
                }
                Item {
                    Layout.fillWidth: true
                }
                IconButton {
                    iconName: "x"
                    text: qsTr("Close the filter")
                    tip: qsTr("Close the filter (Esc)")
                    onClicked: pane.closeFilter()
                }
            }
        }

        // The status line: the selection, the totals and the free space,
        // dropping the free space and then the totals as the pane narrows.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.status
            color: Tokens.colors.chrome

            Rectangle {
                width: parent.width
                height: 1
                color: Tokens.colors.border
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_md
                anchors.rightMargin: Tokens.metrics.space_md
                spacing: Tokens.metrics.space_md

                Text {
                    visible: folder.selectionText.length > 0 && folder.problem.length === 0
                    Layout.fillWidth: !folder.showsTotals
                    elide: Text.ElideRight
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.small_text
                    text: folder.selectionText
                }
                Text {
                    visible: folder.showsTotals || folder.problem.length > 0 || folder.notice.length > 0
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: folder.notice.length > 0 ? Tokens.colors.danger : Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.small_text
                    text: folder.notice.length > 0 ? folder.notice
                        : folder.problem.length > 0 ? folder.problemStatus : folder.total
                }
                Text {
                    visible: folder.showsFreeSpace && folder.freeSpace.length > 0
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.small_text
                    text: folder.freeSpace
                }
            }
        }
    }

    JsonMenu {
        id: trashMenu

        json: folder.menu
        onChosen: path => folder.runMenu(path)
    }

    // The name field over an entry being renamed; Enter keeps the name,
    // Esc cancels, leaving the field keeps it too (as Dolphin).
    component RenameField: TextField {
        id: field

        property bool done: false

        function finish(apply: bool) {
            if (done)
                return
            done = true
            folder.finishRename(text, apply)
            keys.forceActiveFocus()
        }

        text: folder.renameText
        color: Tokens.colors.text
        selectionColor: Tokens.colors.selection
        selectedTextColor: Tokens.colors.on_selection
        font.family: Tokens.metrics.family_body
        font.pixelSize: Math.round(folder.nameText * Tokens.metrics.text_scale)
        leftPadding: Tokens.metrics.space_xs
        rightPadding: Tokens.metrics.space_xs
        topPadding: 0
        bottomPadding: 0
        implicitHeight: Tokens.metrics.rename_field
        Accessible.name: qsTr("New name")
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: Tokens.colors.content
            border.width: 2
            border.color: Tokens.colors.accent
        }
        Component.onCompleted: {
            forceActiveFocus()
            select(0, folder.renameStem)
        }
        Keys.onReturnPressed: finish(true)
        Keys.onEnterPressed: finish(true)
        Keys.onEscapePressed: finish(false)
        onActiveFocusChanged: {
            if (!activeFocus)
                finish(true)
        }
    }

    // The entry's picture: its thumbnail when there is one, else its
    // type's icon from the desktop theme; faded when hidden or cut.
    component EntryIcon: Item {
        id: entryIcon

        required property string icon
        required property string thumbnail
        required property bool link
        property real side: folder.iconSize

        implicitWidth: side
        implicitHeight: side

        Image {
            anchors.centerIn: parent
            width: entryIcon.side
            height: entryIcon.side
            visible: entryIcon.thumbnail.length === 0 || status !== Image.Ready
            source: "image://theme/" + entryIcon.icon
            sourceSize.width: entryIcon.side
            sourceSize.height: entryIcon.side
            Accessible.ignored: true
        }
        Image {
            anchors.centerIn: parent
            width: entryIcon.side
            height: entryIcon.side
            visible: entryIcon.thumbnail.length > 0
            source: entryIcon.thumbnail
            asynchronous: true
            fillMode: Image.PreserveAspectFit
            sourceSize.width: entryIcon.side * 2
            sourceSize.height: entryIcon.side * 2
            Accessible.ignored: true
        }
        // A link is marked in its corner, as in Dolphin.
        Rectangle {
            visible: entryIcon.link
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            width: Tokens.metrics.icon_small
            height: Tokens.metrics.icon_small
            radius: Tokens.metrics.radius
            color: Tokens.colors.raised

            Lucide {
                anchors.centerIn: parent
                name: "link"
                side: Tokens.metrics.icon_small - 2
                colour: Tokens.colors.text
            }
        }
    }

    // A row's or cell's fill: the selection, the hover, the focus outline.
    component EntryFill: Rectangle {
        required property int index
        required property bool selected
        required property bool current
        required property bool renaming

        radius: Tokens.metrics.radius
        color: selected ? (pane.hoverIndex === index ? Tokens.colors.selection_hover : Tokens.colors.selection)
             : (pane.hoverIndex === index || renaming) ? Tokens.colors.hover : "transparent"
        border.width: current && pane.activeFocus && !selected ? 1 : 0
        border.color: Tokens.colors.border_accent
    }

    Component {
        id: detailsView

        ListView {
            id: list

            model: folder
            clip: true
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            keyNavigationEnabled: false
            currentIndex: folder.focusRow
            highlightFollowsCurrentItem: false
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Entries")
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            delegate: Item {
                id: row

                required property int index
                required property string name
                required property string size
                required property string modified
                required property string kind
                required property bool isFolder
                required property string icon
                required property string thumbnail
                required property bool selected
                required property bool current
                required property real fade
                required property bool link
                required property bool renaming

                readonly property color ink: selected ? Tokens.colors.on_selection : Tokens.colors.text
                readonly property color quiet: selected ? Tokens.colors.on_selection : Tokens.colors.text_muted

                width: ListView.view.width
                height: folder.rowHeight
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: selected
                Accessible.focused: current

                EntryFill {
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_xs
                    anchors.rightMargin: Tokens.metrics.space_xs
                    index: row.index
                    selected: row.selected
                    current: row.current
                    renaming: row.renaming
                }
                // The focused entry's bar, as in the mockups.
                Rectangle {
                    visible: row.current && row.selected
                    x: Tokens.metrics.space_xs
                    width: Tokens.metrics.selection_bar
                    height: parent.height
                    color: Tokens.colors.accent
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_lg
                    anchors.rightMargin: Tokens.metrics.space_lg
                    spacing: 0
                    opacity: row.fade

                    EntryIcon {
                        icon: row.icon
                        thumbnail: row.thumbnail
                        link: row.link
                    }
                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.leftMargin: Tokens.metrics.space_sm

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width
                            visible: !row.renaming
                            elide: Text.ElideRight
                            color: row.ink
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Math.round(folder.nameText * Tokens.metrics.text_scale)
                            text: row.name
                        }
                        Loader {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width
                            active: row.renaming
                            sourceComponent: RenameField {}
                        }
                    }
                    Text {
                        visible: folder.showSize
                        Layout.preferredWidth: Tokens.metrics.column_size
                        horizontalAlignment: Text.AlignRight
                        color: row.quiet
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Math.round(folder.dataText * Tokens.metrics.text_scale)
                        text: row.size
                    }
                    Text {
                        visible: folder.showModified
                        Layout.preferredWidth: Tokens.metrics.column_modified
                        leftPadding: Tokens.metrics.space_md
                        elide: Text.ElideRight
                        color: row.quiet
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Math.round(folder.dataText * Tokens.metrics.text_scale)
                        text: row.modified
                    }
                    Text {
                        visible: folder.showType
                        Layout.preferredWidth: Tokens.metrics.column_type
                        leftPadding: Tokens.metrics.space_md
                        elide: Text.ElideRight
                        color: row.quiet
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Math.round(folder.dataText * Tokens.metrics.text_scale)
                        text: row.kind
                    }
                }
            }
        }
    }

    Component {
        id: iconsView

        GridView {
            id: grid

            model: folder
            clip: true
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            keyNavigationEnabled: false
            cellWidth: folder.cellWidth
            cellHeight: folder.rowHeight
            currentIndex: folder.focusRow
            highlightFollowsCurrentItem: false
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Entries")
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            delegate: Item {
                id: cell

                required property int index
                required property string name
                required property bool isFolder
                required property string icon
                required property string thumbnail
                required property bool selected
                required property bool current
                required property real fade
                required property bool link
                required property bool renaming

                width: GridView.view.cellWidth
                height: GridView.view.cellHeight
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: selected
                Accessible.focused: current

                EntryFill {
                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.cell_gap / 2
                    index: cell.index
                    selected: cell.selected
                    current: cell.current
                    renaming: cell.renaming
                }
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.cell_gap / 2 + Tokens.metrics.space_xs
                    spacing: Tokens.metrics.space_xs
                    opacity: cell.fade

                    EntryIcon {
                        Layout.alignment: Qt.AlignHCenter
                        icon: cell.icon
                        thumbnail: cell.thumbnail
                        link: cell.link
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        visible: !cell.renaming
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignTop
                        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        color: cell.selected ? Tokens.colors.on_selection : Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Math.round(folder.nameText * Tokens.metrics.text_scale)
                        text: cell.name
                    }
                    Loader {
                        Layout.fillWidth: true
                        visible: cell.renaming
                        active: cell.renaming
                        sourceComponent: RenameField {}
                    }
                }
            }
        }
    }

    Component {
        id: compactView

        GridView {
            id: columns

            model: folder
            clip: true
            reuseItems: true
            flow: GridView.FlowTopToBottom
            boundsBehavior: Flickable.StopAtBounds
            keyNavigationEnabled: false
            cellWidth: folder.cellWidth
            cellHeight: folder.rowHeight
            currentIndex: folder.focusRow
            highlightFollowsCurrentItem: false
            Accessible.role: Accessible.List
            Accessible.name: qsTr("Entries")
            ScrollBar.horizontal: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            delegate: Item {
                id: entry

                required property int index
                required property string name
                required property bool isFolder
                required property string icon
                required property string thumbnail
                required property bool selected
                required property bool current
                required property real fade
                required property bool link
                required property bool renaming

                width: GridView.view.cellWidth
                height: GridView.view.cellHeight
                Accessible.role: Accessible.ListItem
                Accessible.name: name
                Accessible.selected: selected
                Accessible.focused: current

                EntryFill {
                    anchors.fill: parent
                    anchors.rightMargin: Tokens.metrics.cell_gap
                    index: entry.index
                    selected: entry.selected
                    current: entry.current
                    renaming: entry.renaming
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Tokens.metrics.space_sm
                    anchors.rightMargin: Tokens.metrics.cell_gap + Tokens.metrics.space_xs
                    spacing: Tokens.metrics.space_sm
                    opacity: entry.fade

                    EntryIcon {
                        icon: entry.icon
                        thumbnail: entry.thumbnail
                        link: entry.link
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: !entry.renaming
                        elide: Text.ElideRight
                        color: entry.selected ? Tokens.colors.on_selection : Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Math.round(folder.nameText * Tokens.metrics.text_scale)
                        text: entry.name
                    }
                    Loader {
                        Layout.fillWidth: true
                        visible: entry.renaming
                        active: entry.renaming
                        sourceComponent: RenameField {}
                    }
                }
            }
        }
    }
}
