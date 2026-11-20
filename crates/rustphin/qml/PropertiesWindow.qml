// Properties (W-170, D-041, mockup 40): a window of its own, like Settings
// (W-157), one per entry. Tabs General, Permissions, Checksums and Details
// (a folder has the first two); each tab's sections are panels of rows, as
// on a Settings page. Rust hands everything over as one JSON view
// (`rustphin_view::properties_window`); permission and group changes wait
// for Apply and go through the operation engine.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Window {
    id: props

    // The window Properties was opened from: it stays above it.
    required property Window owner
    // The number of that window: the entry is read with its rights.
    required property int windowId
    // The `file:` URI of the entry shown.
    required property string path
    // Rustphin draws the title row and edges itself, as Settings,
    // Appearance chose for windows when this one was made.
    readonly property bool ownTitleBar: Shell.ownTitleBar()
    readonly property bool framed: ownTitleBar && visibility !== Window.Maximized
    // The view Rust sends as JSON, read once per change into typed parts.
    property bool loaded: false
    property list<var> tabs: []
    property bool folder: false
    property bool hasOpensWith: false
    property string opensWithName: ""
    property string opensWithIcon: ""
    property string location: ""
    property string locationUri: ""
    property string nameKey: ""
    property string pointsTo: ""
    property list<var> sizeRows: []
    property list<var> dates: []
    property list<var> classes: []
    property list<string> choices: []
    property bool runs: false
    property bool runsShown: false
    property bool editable: false
    property string words: ""
    property string accessNote: ""
    property string ownerName: ""
    property string ownerNote: ""
    property list<string> groups: []
    property int groupIndex: 0
    property bool groupEditable: false
    property bool groupChanged: false
    property string pending: ""
    property list<var> sums: []
    property bool hasVerdict: false
    property bool verdictMatches: false
    property string verdictWords: ""
    property list<var> details: []
    property int tab: 0
    readonly property string tabId: tabs.length > tab ? (tabs[tab]?.id ?? "") : ""
    // Advanced shows the Read/Write/Execute grid.
    property bool advanced: false

    // The folder holding the entry is shown with it selected (`file:` URI
    // and percent-encoded name).
    signal revealRequested(string folder, string name)

    function readView() {
        const view = info.view.length > 0 ? JSON.parse(info.view) : null
        loaded = view !== null
        if (!view)
            return
        tabs = view.tabs
        folder = view.folder === true
        hasOpensWith = view.opensWith !== null
        opensWithName = view.opensWith ? view.opensWith.name : ""
        opensWithIcon = view.opensWith ? view.opensWith.icon : ""
        location = view.location
        locationUri = view.locationUri
        nameKey = view.nameKey
        pointsTo = view.pointsTo
        sizeRows = view.size
        dates = view.dates
        // A drop-down's model is assigned only when it changes: assigning
        // one resets the chosen index.
        if (JSON.stringify(choices) !== JSON.stringify(view.access.choices))
            choices = view.access.choices
        classes = view.access.classes
        runs = view.access.runs === true
        runsShown = view.access.runsShown === true
        editable = view.access.editable === true
        words = view.access.words
        accessNote = view.access.note
        ownerName = view.owner
        ownerNote = view.ownerNote
        if (JSON.stringify(groups) !== JSON.stringify(view.groups))
            groups = view.groups
        groupIndex = view.groupIndex
        groupEditable = view.groupEditable === true
        groupChanged = view.groupChanged === true
        pending = view.pending
        sums = view.sums
        hasVerdict = view.verdict !== null
        verdictMatches = view.verdict !== null && view.verdict.matches === true
        verdictWords = view.verdict ? view.verdict.words : ""
        details = view.details
    }

    function step(delta: int) {
        if (tabs.length > 0)
            tab = (tab + delta + tabs.length) % tabs.length
    }

    transientParent: owner
    title: qsTr("%1 — Properties").arg(info.title)
    flags: ownTitleBar ? Qt.Dialog | Qt.FramelessWindowHint : Qt.Dialog
    minimumWidth: Tokens.metrics.properties_min_width
    minimumHeight: Tokens.metrics.properties_min_height
    color: Tokens.colors.raised
    Component.onCompleted: info.open(path)
    onClosing: info.close()

    PropertiesModel {
        id: info

        window: props.windowId
        onViewChanged: props.readView()
    }

    // The text styles of the window.
    component Words: Text {
        textFormat: Text.PlainText
        color: Tokens.colors.text
        font.family: Tokens.metrics.family_body
        font.pixelSize: Tokens.metrics.body_text
        wrapMode: Text.Wrap
        lineHeight: Tokens.metrics.text_line
        lineHeightMode: Text.FixedHeight
    }
    component Quiet: Words {
        color: Tokens.colors.text_muted
        font.pixelSize: Tokens.metrics.small_text
    }
    component Data: Text {
        textFormat: Text.PlainText
        color: Tokens.colors.text
        font.family: Tokens.metrics.family_data
        font.pixelSize: Tokens.metrics.data_text
        elide: Text.ElideMiddle
    }
    component Divider: Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Tokens.metrics.hairline
        color: Tokens.colors.border
    }

    // One row of a section: a label in its column, then the row's content;
    // a hairline above every row but the first.
    component Line: ColumnLayout {
        id: line

        property bool first: false
        property string label: ""
        // A second, quieter line under the label ("alex" under Owner).
        property string sublabel: ""
        default property alias content: lineContent.data

        Layout.fillWidth: true
        spacing: 0

        Divider {
            visible: !line.first
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.minimumHeight: Tokens.metrics.setting_line - Tokens.metrics.hairline - 2 * Tokens.metrics.space_sm
            Layout.leftMargin: Tokens.metrics.space_md
            Layout.rightMargin: Tokens.metrics.space_md
            Layout.topMargin: Tokens.metrics.space_sm
            Layout.bottomMargin: Tokens.metrics.space_sm
            spacing: Tokens.metrics.space_md

            ColumnLayout {
                visible: line.label.length > 0
                // A fixed column: its words fill it, never the row.
                Layout.fillWidth: false
                Layout.preferredWidth: Tokens.metrics.properties_row_label
                Layout.maximumWidth: Tokens.metrics.properties_row_label
                Layout.alignment: Qt.AlignVCenter
                spacing: 0

                Words {
                    Layout.fillWidth: true
                    color: Tokens.colors.text_muted
                    text: line.label
                }
                Quiet {
                    visible: line.sublabel.length > 0
                    Layout.fillWidth: true
                    text: line.sublabel
                }
            }
            RowLayout {
                id: lineContent

                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: Tokens.metrics.space_sm
            }
        }
    }

    // A label and a value, with an optional quieter aside.
    component Fact: Line {
        id: fact

        property string value: ""
        property string aside: ""
        property bool mono: false

        Data {
            visible: fact.mono
            Layout.fillWidth: true
            text: fact.value
        }
        Words {
            visible: !fact.mono
            Layout.fillWidth: !fact.aside
            wrapMode: Text.NoWrap
            elide: Text.ElideRight
            text: fact.value
        }
        Data {
            visible: fact.aside.length > 0
            Layout.fillWidth: true
            color: Tokens.colors.text_muted
            text: fact.aside
        }
    }

    // A section of facts from JSON rows `{label, value, aside, data}`.
    component FactSection: SettingsSection {
        id: factSection

        property list<var> facts: []

        Repeater {
            model: factSection.facts

            delegate: Fact {
                required property int index

                first: index === 0
                label: factSection.facts[index]?.label ?? ""
                value: factSection.facts[index]?.value ?? ""
                aside: factSection.facts[index]?.aside ?? ""
                mono: factSection.facts[index]?.data === true
            }
        }
    }

    // The window's surface: solid, as dialogs are, with a royal-blue edge
    // when Rustphin draws the frame itself.
    Rectangle {
        anchors.fill: parent
        color: Tokens.colors.raised
        border.width: props.framed ? Tokens.metrics.hairline : 0
        border.color: Tokens.colors.border_accent
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: props.framed ? Tokens.metrics.hairline : 0
        focus: true
        spacing: 0
        Accessible.role: Accessible.Dialog
        Accessible.name: props.title
        Keys.onEscapePressed: props.close()
        Keys.onReturnPressed: info.apply()
        Keys.onEnterPressed: info.apply()
        // Ctrl+Tab and Ctrl+Shift+Tab move between the tabs.
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Tab && event.modifiers === Qt.ControlModifier) {
                props.step(1)
                event.accepted = true
            } else if (event.key === Qt.Key_Backtab && (event.modifiers & Qt.ControlModifier)) {
                props.step(-1)
                event.accepted = true
            }
        }

        // Title row, when Rustphin draws the frame: the entry's name and
        // Close; it moves the window.
        RowLayout {
            visible: props.ownTitleBar
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_sm
            spacing: Tokens.metrics.space_sm

            DragHandler {
                target: null
                grabPermissions: PointerHandler.TakeOverForbidden
                onActiveChanged: {
                    if (active)
                        props.startSystemMove()
                }
            }
            Lucide {
                name: info.icon
                colour: Tokens.colors.text_muted
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                font.weight: Font.DemiBold
                text: props.title
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close")
                tip: qsTr("Close (Esc)")
                onClicked: props.close()
            }
        }

        // The tabs: icon and name; the current one brighter, with the
        // accent edge under it (not colour alone).
        RowLayout {
            visible: props.tabs.length > 0
            Layout.fillWidth: true
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            spacing: 0
            Accessible.role: Accessible.PageTabList

            Repeater {
                model: props.tabs

                delegate: AbstractButton {
                    id: tabButton

                    required property int index
                    readonly property string iconName: props.tabs[index]?.icon ?? ""
                    readonly property bool current: props.tab === index

                    implicitHeight: Tokens.metrics.button + Tokens.metrics.space_xs
                    leftPadding: Tokens.metrics.space_md
                    rightPadding: Tokens.metrics.space_md
                    focusPolicy: Qt.TabFocus
                    text: props.tabs[index]?.label ?? ""
                    Accessible.role: Accessible.PageTab
                    Accessible.name: text
                    Accessible.selected: current
                    onClicked: props.tab = index
                    Keys.onLeftPressed: props.step(-1)
                    Keys.onRightPressed: props.step(1)

                    contentItem: Row {
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            anchors.verticalCenter: parent.verticalCenter
                            name: tabButton.iconName
                            colour: tabButton.current ? Tokens.colors.text : Tokens.colors.text_muted
                        }
                        Text {
                            textFormat: Text.PlainText
                            anchors.verticalCenter: parent.verticalCenter
                            color: tabButton.current ? Tokens.colors.text : Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            font.weight: tabButton.current ? Font.DemiBold : Font.Normal
                            text: tabButton.text
                        }
                    }
                    background: Rectangle {
                        color: tabButton.hovered && !tabButton.current ? Tokens.colors.hover : "transparent"
                        border.width: tabButton.visualFocus ? 2 : 0
                        border.color: Tokens.colors.accent

                        Rectangle {
                            visible: tabButton.current
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: Tokens.metrics.selection_bar
                            color: Tokens.colors.accent
                        }
                    }
                }
            }
            Item {
                Layout.fillWidth: true
            }
        }
        Divider {}

        // The tab's page, scrolling when the window is shorter.
        Flickable {
            id: body

            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: page.implicitHeight + 2 * Tokens.metrics.space_lg
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: Scroller {}

            ColumnLayout {
                id: page

                x: Tokens.metrics.space_lg
                y: Tokens.metrics.space_lg
                width: body.width - 2 * Tokens.metrics.space_lg
                spacing: Tokens.metrics.space_lg

                // Reading, or why it could not be read: said in place.
                Words {
                    visible: info.loading || info.error.length > 0
                    Layout.fillWidth: true
                    color: info.error.length > 0 ? Tokens.colors.danger : Tokens.colors.text_muted
                    text: info.error.length > 0 ? info.error : qsTr("Reading properties…")
                    Accessible.role: Accessible.StaticText
                }

                // ---- General ---------------------------------------------
                ColumnLayout {
                    visible: props.tabId === "General" && props.loaded
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_lg

                    RowLayout {
                        spacing: Tokens.metrics.space_lg

                        Item {
                            implicitWidth: Tokens.metrics.properties_thumb_width
                            implicitHeight: Tokens.metrics.properties_thumb_height

                            // The type's icon from the theme, as in the
                            // views; the outline icon only when there is none.
                            Image {
                                anchors.centerIn: parent
                                visible: info.thumbnail.length === 0 && info.themeIcon.length > 0
                                width: Tokens.metrics.icon_header
                                height: Tokens.metrics.icon_header
                                source: visible ? "image://theme/" + info.themeIcon : ""
                                sourceSize.width: Tokens.metrics.icon_header
                                sourceSize.height: Tokens.metrics.icon_header
                                Accessible.ignored: true
                            }
                            Lucide {
                                anchors.centerIn: parent
                                visible: info.thumbnail.length === 0 && info.themeIcon.length === 0
                                name: info.icon
                                side: Tokens.metrics.icon_header
                            }
                            Image {
                                anchors.fill: parent
                                visible: info.thumbnail.length > 0
                                source: info.thumbnail
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                                sourceSize.width: Tokens.metrics.properties_thumb_width
                                sourceSize.height: Tokens.metrics.properties_thumb_height
                            }
                        }
                        ColumnLayout {
                            spacing: 0

                            Text {
                                textFormat: Text.PlainText
                                color: Tokens.colors.text
                                font.family: Tokens.metrics.family_heading
                                font.pixelSize: Tokens.metrics.title_text
                                font.weight: Font.DemiBold
                                text: info.title
                            }
                            Words {
                                color: Tokens.colors.text_muted
                                text: info.kind
                            }
                        }
                    }
                    SettingsSection {
                        // A folder has only its location here.
                        title: props.folder ? qsTr("Location") : qsTr("Opening")

                        Line {
                            first: true
                            visible: !props.folder
                            label: qsTr("Opens with")

                            Image {
                                visible: props.opensWithIcon.length > 0
                                source: visible ? props.opensWithIcon : ""
                                sourceSize.width: Tokens.metrics.icon_ui
                                sourceSize.height: Tokens.metrics.icon_ui
                                Layout.preferredWidth: Tokens.metrics.icon_ui
                                Layout.preferredHeight: Tokens.metrics.icon_ui
                            }
                            Words {
                                Layout.fillWidth: true
                                wrapMode: Text.NoWrap
                                elide: Text.ElideRight
                                color: props.hasOpensWith ? Tokens.colors.text : Tokens.colors.text_muted
                                text: props.hasOpensWith ? props.opensWithName : qsTr("No application is set for this type")
                            }
                            OutlinedButton {
                                text: qsTr("Change…")
                                onClicked: chooserLoader.active = true
                            }
                        }
                        Line {
                            first: props.folder
                            label: qsTr("Location")

                            Data {
                                Layout.fillWidth: true
                                text: props.location
                            }
                            IconButton {
                                iconName: "folder-open"
                                text: qsTr("Show in its folder")
                                onClicked: props.revealRequested(props.locationUri, props.nameKey)
                            }
                        }
                        Fact {
                            visible: props.pointsTo.length > 0
                            label: qsTr("Points to")
                            value: props.pointsTo
                            mono: true
                        }
                    }
                    FactSection {
                        title: qsTr("Size")
                        facts: props.sizeRows
                    }
                    SettingsSection {
                        title: qsTr("Dates")

                        Repeater {
                            model: props.dates.length

                            delegate: Fact {
                                required property int index

                                first: index === 0
                                label: props.dates[index]?.label ?? ""
                                value: props.dates[index]?.value ?? ""
                                mono: true
                            }
                        }
                    }
                }

                // ---- Permissions -----------------------------------------
                ColumnLayout {
                    visible: props.tabId === "Permissions" && props.loaded
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_lg

                    SettingsSection {
                        title: qsTr("Access")
                        note: props.accessNote

                        Repeater {
                            model: props.classes.length

                            delegate: Line {
                                id: accessLine

                                required property int index
                                readonly property string className: props.classes[index]?.name ?? ""
                                readonly property int choice: props.classes[index]?.choice ?? -1
                                readonly property string custom: props.classes[index]?.custom ?? ""
                                readonly property bool changed: props.classes[index]?.changed === true

                                first: index === 0
                                label: qsTr(className)
                                sublabel: props.classes[index]?.who ?? ""

                                Item {
                                    Layout.fillWidth: true
                                }
                                SettingChoice {
                                    id: accessChoice

                                    // Set, not bound: a new model resets
                                    // the index (and breaks a binding).
                                    function sync() {
                                        currentIndex = accessLine.choice < 0 ? 0 : accessLine.choice
                                    }

                                    enabled: props.editable
                                    model: accessLine.choice < 0 ? [accessLine.custom].concat(props.choices) : props.choices
                                    onModelChanged: sync()
                                    Component.onCompleted: sync()
                                    Connections {
                                        target: accessLine
                                        function onChoiceChanged() {
                                            accessChoice.sync()
                                        }
                                    }
                                    Accessible.name: qsTr("%1 access").arg(accessLine.className)
                                    // A change not yet applied keeps the
                                    // accent outline, with the footer's note.
                                    background: Rectangle {
                                        radius: Tokens.metrics.radius
                                        color: Tokens.colors.content
                                        border.width: accessLine.changed ? 2 : 1
                                        border.color: accessLine.changed ? Tokens.colors.accent : Tokens.colors.border_strong
                                    }
                                    onActivated: index => info.chooseAccess(accessLine.index, accessLine.choice < 0 ? index - 1 : index)
                                }
                            }
                        }
                        Line {
                            visible: props.runsShown

                            Check {
                                id: runsBox

                                enabled: props.editable
                                checked: props.runs
                                text: qsTr("Allow running the file as a program")
                                onToggled: info.setRuns(checked)
                                contentItem: Words {
                                    leftPadding: runsBox.indicator.width + Tokens.metrics.space_sm
                                    verticalAlignment: Text.AlignVCenter
                                    text: runsBox.text
                                }
                            }
                        }
                        Line {
                            AbstractButton {
                                id: advancedButton

                                Layout.fillWidth: true
                                implicitHeight: Tokens.metrics.setting_control
                                focusPolicy: Qt.TabFocus
                                text: qsTr("Advanced")
                                Accessible.role: Accessible.Button
                                Accessible.name: text
                                Accessible.checked: props.advanced
                                onClicked: props.advanced = !props.advanced
                                contentItem: RowLayout {
                                    spacing: Tokens.metrics.space_sm

                                    Lucide {
                                        name: props.advanced ? "chevron-down" : "chevron-right"
                                        side: Tokens.metrics.icon_small
                                        colour: Tokens.colors.text_muted
                                    }
                                    Words {
                                        Layout.fillWidth: true
                                        text: advancedButton.text
                                    }
                                    Data {
                                        color: Tokens.colors.text_muted
                                        text: props.words
                                    }
                                }
                                background: Rectangle {
                                    color: "transparent"
                                    border.width: advancedButton.visualFocus ? 2 : 0
                                    border.color: Tokens.colors.accent
                                    radius: Tokens.metrics.radius
                                }
                            }
                        }
                        // The bits themselves, under Advanced.
                        ColumnLayout {
                            visible: props.advanced
                            Layout.fillWidth: true
                            spacing: 0

                            Divider {}
                            GridLayout {
                                Layout.fillWidth: true
                                Layout.margins: Tokens.metrics.space_md
                                columns: 4
                                rowSpacing: Tokens.metrics.space_xs
                                columnSpacing: Tokens.metrics.space_md

                                Item {
                                    Layout.preferredWidth: Tokens.metrics.properties_row_label - Tokens.metrics.space_md
                                }
                                Repeater {
                                    model: [qsTr("Read"), qsTr("Write"), props.folder ? qsTr("Enter") : qsTr("Run")]

                                    delegate: Quiet {
                                        required property string modelData

                                        Layout.fillWidth: true
                                        text: modelData
                                    }
                                }
                                Repeater {
                                    model: props.classes.length * 4

                                    delegate: Loader {
                                        id: cell

                                        required property int index
                                        readonly property int who: Math.floor(index / 4)
                                        readonly property int which: index % 4 - 1

                                        Layout.fillWidth: true
                                        sourceComponent: which < 0 ? classLabel : bitBox

                                        Component {
                                            id: classLabel

                                            Words {
                                                text: qsTr(props.classes[cell.who]?.name ?? "")
                                            }
                                        }
                                        Component {
                                            id: bitBox

                                            Check {
                                                enabled: props.editable
                                                checked: props.classes[cell.who]?.bits[cell.which] === true
                                                Accessible.name: qsTr("%1 may %2").arg(props.classes[cell.who]?.name ?? "").arg(["read", "write", "run"][cell.which])
                                                onToggled: info.setBit(cell.who, cell.which, checked)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SettingsSection {
                        title: qsTr("Ownership")

                        Line {
                            first: true
                            label: qsTr("Owner")

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0

                                Words {
                                    Layout.fillWidth: true
                                    text: props.ownerName
                                }
                                Quiet {
                                    Layout.fillWidth: true
                                    text: props.ownerNote
                                }
                            }
                        }
                        Line {
                            label: qsTr("Group")

                            Item {
                                Layout.fillWidth: true
                            }
                            SettingChoice {
                                id: groupChoice

                                enabled: props.groupEditable
                                model: props.groups
                                onModelChanged: currentIndex = props.groupIndex
                                Component.onCompleted: currentIndex = props.groupIndex
                                Connections {
                                    target: props
                                    function onGroupIndexChanged() {
                                        groupChoice.currentIndex = props.groupIndex
                                    }
                                }
                                Accessible.name: qsTr("Group")
                                background: Rectangle {
                                    radius: Tokens.metrics.radius
                                    color: Tokens.colors.content
                                    border.width: props.groupChanged ? 2 : 1
                                    border.color: props.groupChanged ? Tokens.colors.accent : Tokens.colors.border_strong
                                }
                                onActivated: index => info.chooseGroup(index)
                            }
                        }
                    }
                }

                // ---- Checksums -------------------------------------------
                ColumnLayout {
                    visible: props.tabId === "Checksums" && props.loaded
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_lg

                    SettingsSection {
                        title: qsTr("Check a download")
                        note: qsTr("Paste the checksum the website gives for this file. Rustphin finds which kind it is and calculates that one.")
                        panelled: false

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Tokens.metrics.space_sm

                            TextField {
                                id: expected

                                Layout.fillWidth: true
                                implicitHeight: Tokens.metrics.button
                                placeholderText: qsTr("Expected checksum (MD5, SHA1, SHA256 or SHA512)")
                                color: Tokens.colors.text
                                placeholderTextColor: Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_data
                                font.pixelSize: Tokens.metrics.data_text
                                Accessible.name: qsTr("Expected checksum")
                                background: Rectangle {
                                    radius: Tokens.metrics.radius
                                    color: Tokens.colors.content
                                    border.width: expected.activeFocus ? 2 : 1
                                    border.color: expected.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                                }
                                onTextChanged: info.paste(text)
                            }
                            OutlinedButton {
                                text: qsTr("Paste")
                                iconName: "clipboard-paste"
                                onClicked: {
                                    expected.clear()
                                    expected.paste()
                                }
                            }
                        }
                        RowLayout {
                            visible: props.hasVerdict
                            Layout.fillWidth: true
                            spacing: Tokens.metrics.space_sm

                            Lucide {
                                Layout.alignment: Qt.AlignTop
                                name: props.verdictMatches ? "circle-check" : "triangle-alert"
                                colour: props.verdictMatches ? Tokens.colors.success : Tokens.colors.danger
                            }
                            Words {
                                Layout.fillWidth: true
                                color: props.verdictMatches ? Tokens.colors.success : Tokens.colors.danger
                                text: props.verdictWords
                                Accessible.role: Accessible.AlertMessage
                            }
                        }
                    }
                    SettingsSection {
                        title: qsTr("Checksums")

                        Repeater {
                            model: props.sums.length

                            delegate: Line {
                                id: sumLine

                                required property int index
                                readonly property string name: props.sums[index]?.name ?? ""
                                readonly property string stage: props.sums[index]?.state ?? ""
                                readonly property string value: props.sums[index]?.value ?? ""
                                readonly property real fraction: props.sums[index]?.fraction ?? 0

                                first: index === 0
                                label: name

                                Data {
                                    visible: sumLine.stage === "done"
                                    Layout.fillWidth: true
                                    text: sumLine.value
                                }
                                ColumnLayout {
                                    visible: sumLine.stage === "running"
                                    Layout.fillWidth: true
                                    spacing: Tokens.metrics.space_xs

                                    ActivityBar {
                                        Layout.fillWidth: true
                                        fraction: sumLine.fraction
                                    }
                                    Quiet {
                                        Layout.fillWidth: true
                                        text: sumLine.value
                                    }
                                }
                                Words {
                                    visible: sumLine.stage === "failed"
                                    Layout.fillWidth: true
                                    color: Tokens.colors.danger
                                    text: sumLine.value
                                }
                                Item {
                                    visible: sumLine.stage === "idle"
                                    Layout.fillWidth: true
                                }
                                IconButton {
                                    visible: sumLine.stage === "done"
                                    iconName: "copy"
                                    text: qsTr("Copy %1").arg(sumLine.name)
                                    onClicked: textClip.put(sumLine.value)
                                }
                                OutlinedButton {
                                    visible: sumLine.stage === "running"
                                    text: qsTr("Stop")
                                    onClicked: info.stopSum(sumLine.index)
                                }
                                OutlinedButton {
                                    visible: sumLine.stage === "idle" || sumLine.stage === "failed"
                                    text: sumLine.stage === "failed" ? qsTr("Try again") : qsTr("Calculate")
                                    onClicked: info.calculate(sumLine.index)
                                }
                            }
                        }
                    }
                }

                // ---- Details ---------------------------------------------
                Repeater {
                    model: props.tabId === "Details" ? props.details.length : 0

                    delegate: FactSection {
                        required property int index

                        title: qsTr(props.details[index]?.title ?? "")
                        facts: props.details[index]?.rows ?? []
                    }
                }
            }
        }

        // The footer: what waits for Apply, Close and Apply.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.control_line
            color: Tokens.colors.chrome

            Divider {
                anchors.top: parent.top
                width: parent.width
                height: Tokens.metrics.hairline
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Tokens.metrics.space_lg
                anchors.rightMargin: Tokens.metrics.space_md
                spacing: Tokens.metrics.space_sm

                Words {
                    Layout.fillWidth: true
                    color: Tokens.colors.text_muted
                    text: props.pending
                    Accessible.role: Accessible.StaticText
                }
                OutlinedButton {
                    text: qsTr("Close (Esc)")
                    onClicked: props.close()
                }
                OutlinedButton {
                    text: qsTr("Apply (Enter)")
                    iconName: "check"
                    primary: true
                    enabled: info.dirty
                    onClicked: info.apply()
                }
            }
        }
    }

    // A checksum for the clipboard.
    TextEdit {
        id: textClip

        visible: false

        function put(words: string) {
            text = words
            selectAll()
            copy()
        }
    }

    // Change…: the application chooser, its "always" box ticked.
    Loader {
        id: chooserLoader

        active: false
        sourceComponent: AppChooserDialog {
            uri: props.path
            remember: true
            chooseOnly: true
            onClosed: {
                chooserLoader.active = false
                info.refreshApp()
            }
        }
    }

    // Bands along the edges resize the window when Rustphin draws its frame
    // (4 pixels; 16-pixel corners), as Settings'.
    ResizeBand {
        edges: Qt.LeftEdge
        shape: Qt.SizeHorCursor
        l: 0
        t: 16
        w: 4
        h: -32
    }
    ResizeBand {
        edges: Qt.RightEdge
        shape: Qt.SizeHorCursor
        l: -4
        t: 16
        w: 4
        h: -32
    }
    ResizeBand {
        edges: Qt.TopEdge
        shape: Qt.SizeVerCursor
        l: 16
        t: 0
        w: -32
        h: 4
    }
    ResizeBand {
        edges: Qt.BottomEdge
        shape: Qt.SizeVerCursor
        l: 16
        t: -4
        w: -32
        h: 4
    }
    ResizeBand {
        edges: Qt.TopEdge | Qt.LeftEdge
        shape: Qt.SizeFDiagCursor
        l: 0
        t: 0
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.TopEdge | Qt.RightEdge
        shape: Qt.SizeBDiagCursor
        l: -16
        t: 0
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.BottomEdge | Qt.LeftEdge
        shape: Qt.SizeBDiagCursor
        l: 0
        t: -16
        w: 16
        h: 16
    }
    ResizeBand {
        edges: Qt.BottomEdge | Qt.RightEdge
        shape: Qt.SizeFDiagCursor
        l: -16
        t: -16
        w: 16
        h: 16
    }

    // A band that resizes the window from `edges`. Negative `l`/`t` count
    // from the right or bottom; `w`/`h` of zero or less take the window's
    // size less that much.
    component ResizeBand: MouseArea {
        required property int edges
        required property int shape
        required property real l
        required property real t
        required property real w
        required property real h

        visible: props.framed
        x: l < 0 ? props.width + l : l
        y: t < 0 ? props.height + t : t
        width: w <= 0 ? props.width + w : w
        height: h <= 0 ? props.height + h : h
        cursorShape: shape
        onPressed: props.startSystemResize(edges)
    }
}
