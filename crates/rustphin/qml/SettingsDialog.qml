// Settings (W-106, mockups 30–36): the pages on the left, the chosen
// page's groups on the right. Rows come from Rust (`settings_form`) as
// JSON and are drawn here by kind; the Toolbar, Places groups, Shortcuts
// and About pages have their own layouts.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    required property string shownFolder
    required property PlacesModel places
    // Rust hands the pages over as JSON (`settings_form`): typed here.
    readonly property list<var> pageList: settings.pages.length > 0 ? JSON.parse(settings.pages) : []
    readonly property list<var> groups: settings.groups.length > 0 ? JSON.parse(settings.groups) : []
    readonly property bool pageShown: pageList.length > settings.page
    readonly property string pageTitle: pageShown ? pageList[settings.page].title : ""
    readonly property string pageDescription: pageShown ? pageList[settings.page].description : ""
    readonly property bool pageResettable: pageShown && pageList[settings.page].resettable === true
    property bool showingLicence: false
    property int toolbarCursor: -1
    property int availableCursor: -1

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.settings_width, parent.width - 2 * Tokens.metrics.space_lg)
    height: Math.min(Tokens.metrics.settings_height, parent.height - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
    Component.onCompleted: {
        settings.start(shownFolder)
        open()
    }

    SettingsModel {
        id: settings
    }

    // The text style of the dialog.
    component Words: Text {
        color: Tokens.colors.text
        font.family: Tokens.metrics.family_body
        font.pixelSize: Tokens.metrics.body_text
        wrapMode: Text.Wrap
    }
    component Quiet: Words {
        color: Tokens.colors.text_muted
        font.pixelSize: Tokens.metrics.small_text
    }
    component Field: TextField {
        id: input

        implicitHeight: Tokens.metrics.setting_control
        color: Tokens.colors.text
        placeholderTextColor: Tokens.colors.text_muted
        font.family: Tokens.metrics.family_body
        font.pixelSize: Tokens.metrics.body_text
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: Tokens.colors.content
            border.width: input.activeFocus ? 2 : 1
            border.color: input.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
        }
    }

    contentItem: ColumnLayout {
        spacing: 0
        Accessible.role: Accessible.Dialog
        Accessible.name: qsTr("Settings")

        // Title row.
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: "settings"
                colour: Tokens.colors.text
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: qsTr("Settings")
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close")
                tip: qsTr("Close (Esc)")
                onClicked: dialog.close()
            }
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Tokens.colors.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // The pages; Up and Down move between them.
            ListView {
                id: pages

                Layout.preferredWidth: Tokens.metrics.settings_nav
                Layout.fillHeight: true
                Layout.topMargin: Tokens.metrics.space_sm
                model: dialog.pageList
                currentIndex: settings.page
                keyNavigationEnabled: false
                focus: true
                Accessible.role: Accessible.List
                Accessible.name: qsTr("Settings pages")
                Keys.onUpPressed: settings.choosePage(Math.max(0, settings.page - 1))
                Keys.onDownPressed: settings.choosePage(Math.min(dialog.pageList.length - 1, settings.page + 1))

                delegate: ItemDelegate {
                    id: pageRow

                    required property int index
                    readonly property string pageTitle: dialog.pageList[index]?.title ?? ""
                    readonly property string pageIcon: dialog.pageList[index]?.icon ?? ""
                    readonly property bool chosen: settings.page === index

                    width: ListView.view.width - Tokens.metrics.space_sm
                    x: Tokens.metrics.space_xs
                    implicitHeight: Tokens.metrics.setting_row
                    Accessible.role: Accessible.ListItem
                    Accessible.name: pageTitle
                    Accessible.selected: chosen
                    background: Rectangle {
                        radius: Tokens.metrics.radius
                        color: pageRow.chosen ? Tokens.colors.selection : pageRow.hovered ? Tokens.colors.hover : "transparent"
                    }
                    contentItem: RowLayout {
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            name: pageRow.pageIcon
                            colour: pageRow.chosen ? Tokens.colors.on_selection : Tokens.colors.text_muted
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            color: pageRow.chosen ? Tokens.colors.on_selection : Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: pageRow.pageTitle
                        }
                    }
                    onClicked: {
                        dialog.showingLicence = false
                        settings.choosePage(index)
                    }
                }
            }
            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Tokens.colors.border
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Flickable {
                    id: body

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentHeight: column.implicitHeight + 2 * Tokens.metrics.space_lg
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar {
                        policy: ScrollBar.AsNeeded
                    }

                    ColumnLayout {
                        id: column

                        x: Tokens.metrics.space_lg
                        y: Tokens.metrics.space_lg
                        width: body.width - 2 * Tokens.metrics.space_lg
                        spacing: Tokens.metrics.space_md

                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_heading
                            font.pixelSize: Tokens.metrics.heading_text
                            text: dialog.pageTitle
                            Accessible.role: Accessible.Heading
                        }
                        Quiet {
                            Layout.fillWidth: true
                            text: dialog.pageDescription
                        }

                        // Pages of rows.
                        Repeater {
                            model: dialog.groups

                            delegate: ColumnLayout {
                                id: group

                                required property int index
                                readonly property string title: dialog.groups[index]?.title ?? ""
                                readonly property list<var> rows: dialog.groups[index]?.rows ?? []

                                Layout.fillWidth: true
                                Layout.topMargin: Tokens.metrics.space_sm
                                spacing: Tokens.metrics.space_xs

                                Text {
                                    textFormat: Text.PlainText
                                    color: Tokens.colors.text_muted
                                    font.family: Tokens.metrics.family_body
                                    font.pixelSize: Tokens.metrics.label_text
                                    font.capitalization: Font.AllUppercase
                                    text: group.title
                                    Accessible.role: Accessible.Heading
                                }
                                Repeater {
                                    model: group.rows

                                    delegate: SettingRow {
                                        rows: group.rows
                                    }
                                }
                            }
                        }

                        Loader {
                            Layout.fillWidth: true
                            active: settings.page === 3
                            visible: active
                            sourceComponent: toolbarPage
                        }
                        Loader {
                            Layout.fillWidth: true
                            active: settings.page === 4
                            visible: active
                            sourceComponent: groupsPage
                        }
                        Loader {
                            Layout.fillWidth: true
                            active: settings.page === 6
                            visible: active
                            sourceComponent: shortcutsPage
                        }
                        Loader {
                            Layout.fillWidth: true
                            active: settings.page === 7
                            visible: active
                            sourceComponent: aboutPage
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Tokens.metrics.control_line
                    visible: dialog.pageResettable
                    color: Tokens.colors.chrome

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Tokens.metrics.space_lg
                        anchors.rightMargin: Tokens.metrics.space_md

                        Quiet {
                            Layout.fillWidth: true
                            text: qsTr("Changes apply at once.")
                        }
                        OutlinedButton {
                            text: qsTr("Reset to defaults")
                            iconName: "rotate-ccw"
                            onClicked: settings.reset()
                        }
                    }
                }
            }
        }
    }

    // One row: its label, its control, the words under it.
    component SettingRow: RowLayout {
        id: line

        // The row `index` of `rows`, each field typed.
        required property list<var> rows
        required property int index
        readonly property string key: rows[index]?.key ?? ""
        readonly property string label: rows[index]?.label ?? ""
        readonly property string hint: rows[index]?.hint ?? ""
        readonly property bool warning: rows[index]?.warning === true
        readonly property string kind: rows[index]?.control?.kind ?? ""
        readonly property list<var> options: kind === "choice" ? (rows[index]?.control?.options ?? []) : []
        readonly property string valueText: kind === "choice" || kind === "text" ? (rows[index]?.control?.value ?? "") : ""
        readonly property real valueNumber: kind === "stepper" ? (rows[index]?.control?.value ?? 0) : 0
        readonly property real min: kind === "stepper" ? (rows[index]?.control?.min ?? 0) : 0
        readonly property real max: kind === "stepper" ? (rows[index]?.control?.max ?? 0) : 0
        readonly property real step: kind === "stepper" ? (rows[index]?.control?.step ?? 0) : 0
        readonly property string shownValue: kind === "stepper" ? (rows[index]?.control?.text || "") : ""
        readonly property bool isOn: kind === "toggle" && rows[index]?.control?.on === true
        readonly property bool controlEnabled: kind === "toggle" ? rows[index]?.control?.enabled === true : true
        readonly property string placeholder: kind === "text" ? (rows[index]?.control?.placeholder || "") : ""
        readonly property string buttonLabel: kind === "button" ? (rows[index]?.control?.label ?? "") : ""

        Layout.fillWidth: true
        spacing: Tokens.metrics.space_md

        Words {
            Layout.preferredWidth: Tokens.metrics.settings_label
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: Tokens.metrics.space_xs
            text: line.label
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            spacing: Tokens.metrics.space_xs

            ComboBox {
                id: combo

                visible: line.kind === "choice"
                Layout.preferredWidth: Tokens.metrics.setting_select + Tokens.metrics.space_lg
                implicitHeight: Tokens.metrics.setting_control
                model: line.options.map(option => option[1])
                currentIndex: line.options.findIndex(option => option[0] === line.valueText)
                Accessible.name: line.label
                onActivated: index => settings.set(line.key, line.options[index][0])
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: combo.hovered ? Tokens.colors.hover : Tokens.colors.content
                    border.width: combo.visualFocus ? 2 : 1
                    border.color: combo.visualFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                }
                contentItem: Text {
                    textFormat: Text.PlainText
                    leftPadding: Tokens.metrics.space_sm
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                    color: Tokens.colors.text
                    font: combo.font
                    text: combo.displayText
                }
            }
            CheckBox {
                id: check

                Layout.fillWidth: true
                visible: line.kind === "toggle"
                checked: line.isOn
                enabled: line.controlEnabled
                text: line.hint
                Accessible.name: line.hint.length > 0 ? line.hint : line.label
                onToggled: settings.set(line.key, checked ? "true" : "false")
                contentItem: Text {
                    textFormat: Text.PlainText
                    leftPadding: check.indicator.width + Tokens.metrics.space_sm
                    verticalAlignment: Text.AlignVCenter
                    wrapMode: Text.Wrap
                    color: check.enabled ? Tokens.colors.text : Tokens.colors.text_disabled
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: check.text
                }
            }
            RowLayout {
                visible: line.kind === "stepper"
                spacing: Tokens.metrics.space_xs

                IconButton {
                    iconName: "minus"
                    text: qsTr("Less")
                    tip: qsTr("Less")
                    enabled: line.valueNumber > line.min
                    side: Tokens.metrics.setting_control
                    onClicked: settings.set(line.key, String(line.valueNumber - line.step))
                }
                Text {
                    textFormat: Text.PlainText
                    Layout.preferredWidth: Tokens.metrics.setting_select_short / 2
                    horizontalAlignment: Text.AlignHCenter
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_data
                    font.pixelSize: Tokens.metrics.data_text
                    text: line.shownValue
                    Accessible.role: Accessible.StaticText
                    Accessible.name: line.label + ": " + text
                }
                IconButton {
                    iconName: "plus"
                    text: qsTr("More")
                    tip: qsTr("More")
                    enabled: line.valueNumber < line.max
                    side: Tokens.metrics.setting_control
                    onClicked: settings.set(line.key, String(line.valueNumber + line.step))
                }
            }
            Field {
                visible: line.kind === "text"
                Layout.preferredWidth: Tokens.metrics.setting_field
                text: line.valueText
                placeholderText: line.placeholder
                Accessible.name: qsTr("Terminal command")
                onAccepted: settings.set(line.key, text)
                onEditingFinished: settings.set(line.key, text)
            }
            OutlinedButton {
                visible: line.kind === "button"
                text: line.buttonLabel
                onClicked: settings.set(line.key, "")
            }
            Quiet {
                Layout.fillWidth: true
                visible: line.hint.length > 0 && line.kind !== "toggle"
                color: line.warning ? Tokens.colors.danger : Tokens.colors.text_muted
                text: line.hint
            }
        }
    }

    // Toolbar: what is on it, what can be added, and the order.
    Component {
        id: toolbarPage

        ColumnLayout {
            id: page

            readonly property list<var> onBar: settings.toolbar.length > 0 ? JSON.parse(settings.toolbar) : []
            property list<var> offered: JSON.parse(settings.available(search.text))

            spacing: Tokens.metrics.space_sm

            Connections {
                target: settings
                function onRevisionChanged() {
                    page.offered = JSON.parse(settings.available(search.text))
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.metrics.space_md

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1

                    Quiet {
                        text: qsTr("Available actions")
                    }
                    Field {
                        id: search

                        Layout.fillWidth: true
                        placeholderText: qsTr("Search actions…")
                        Accessible.name: qsTr("Search the available actions")
                        onTextChanged: page.offered = JSON.parse(settings.available(text))
                    }
                    ItemList {
                        items: page.offered
                        cursor: dialog.availableCursor
                        name: qsTr("Available actions")
                        empty: qsTr("Nothing matches; everything else is on the toolbar.")
                        onPicked: index => dialog.availableCursor = index
                    }
                }
                ColumnLayout {
                    spacing: Tokens.metrics.space_xs

                    IconButton {
                        iconName: "arrow-right"
                        text: qsTr("Add to the toolbar")
                        enabled: dialog.availableCursor >= 0
                        onClicked: {
                            dialog.toolbarCursor = settings.addItem(search.text, dialog.availableCursor, dialog.toolbarCursor)
                            dialog.availableCursor = -1
                        }
                    }
                    IconButton {
                        iconName: "arrow-left"
                        text: qsTr("Take off the toolbar")
                        enabled: dialog.toolbarCursor >= 0 && !page.onBar[dialog.toolbarCursor].fixed
                        onClicked: {
                            settings.removeItem(dialog.toolbarCursor)
                            dialog.toolbarCursor = -1
                        }
                    }
                    IconButton {
                        iconName: "arrow-up"
                        text: qsTr("Move left on the toolbar")
                        enabled: dialog.toolbarCursor > 0
                        onClicked: dialog.toolbarCursor = settings.shiftItem(dialog.toolbarCursor, -1)
                    }
                    IconButton {
                        iconName: "arrow-down"
                        text: qsTr("Move right on the toolbar")
                        enabled: dialog.toolbarCursor >= 0 && dialog.toolbarCursor < page.onBar.length - 1
                        onClicked: dialog.toolbarCursor = settings.shiftItem(dialog.toolbarCursor, 1)
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1

                    Quiet {
                        text: qsTr("On the toolbar")
                    }
                    ItemList {
                        items: page.onBar
                        cursor: dialog.toolbarCursor
                        name: qsTr("On the toolbar")
                        empty: ""
                        onPicked: index => dialog.toolbarCursor = index
                    }
                    CheckBox {
                        id: wordsBox

                        enabled: dialog.toolbarCursor >= 0 && page.onBar[dialog.toolbarCursor].canShowWords
                        checked: dialog.toolbarCursor >= 0 && page.onBar[dialog.toolbarCursor].words
                        text: qsTr("Show its words")
                        onToggled: settings.setWords(dialog.toolbarCursor, checked)
                        contentItem: Words {
                            leftPadding: wordsBox.indicator.width + Tokens.metrics.space_sm
                            verticalAlignment: Text.AlignVCenter
                            text: wordsBox.text
                        }
                    }
                }
            }
        }
    }

    // A list of toolbar items to pick from.
    component ItemList: Rectangle {
        id: box

        required property list<var> items
        required property int cursor
        required property string name
        required property string empty

        signal picked(int index)

        Layout.fillWidth: true
        Layout.preferredHeight: 256
        radius: Tokens.metrics.radius
        color: Tokens.colors.content
        border.color: Tokens.colors.border

        ListView {
            anchors.fill: parent
            anchors.margins: Tokens.metrics.space_xs
            clip: true
            model: box.items
            Accessible.role: Accessible.List
            Accessible.name: box.name
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }
            delegate: ItemDelegate {
                id: item

                required property int index
                readonly property string name: box.items[index]?.name ?? ""
                readonly property string iconName: box.items[index]?.icon ?? ""
                readonly property bool fixed: box.items[index]?.fixed === true

                width: ListView.view.width
                implicitHeight: Tokens.metrics.setting_row
                Accessible.name: name
                Accessible.selected: box.cursor === index
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: box.cursor === item.index ? Tokens.colors.selection : item.hovered ? Tokens.colors.hover : "transparent"
                }
                contentItem: RowLayout {
                    spacing: Tokens.metrics.space_sm

                    Lucide {
                        name: item.iconName
                        colour: box.cursor === item.index ? Tokens.colors.on_selection : Tokens.colors.text_muted
                    }
                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: box.cursor === item.index ? Tokens.colors.on_selection : Tokens.colors.text
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.body_text
                        text: item.name
                    }
                    Text {
                        textFormat: Text.PlainText
                        visible: item.fixed
                        color: box.cursor === item.index ? Tokens.colors.on_selection : Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: qsTr("always there")
                    }
                }
                onClicked: box.picked(index)
            }
        }
        Quiet {
            anchors.centerIn: parent
            visible: box.items.length === 0 && box.empty.length > 0
            text: box.empty
        }
    }

    // Places: the panel's groups, moved, renamed or deleted.
    Component {
        id: groupsPage

        ColumnLayout {
            id: groupsColumn

            property list<var> list: JSON.parse(dialog.places.groupsJson())

            spacing: Tokens.metrics.space_xs

            Connections {
                target: dialog.places
                function onCountChanged() {
                    groupsColumn.list = JSON.parse(dialog.places.groupsJson())
                }
            }

            Text {
                textFormat: Text.PlainText
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.label_text
                font.capitalization: Font.AllUppercase
                text: qsTr("Groups")
            }
            Quiet {
                Layout.fillWidth: true
                text: groupsColumn.list.length === 0 ? qsTr("No groups yet. Make one with New group in the places panel.") : qsTr("Deleting a group keeps its places.")
            }
            Repeater {
                model: groupsColumn.list

                delegate: RowLayout {
                    id: groupRow

                    required property int index
                    readonly property string name: groupsColumn.list[index]?.name ?? ""
                    readonly property int count: groupsColumn.list[index]?.count ?? 0

                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_xs

                    Field {
                        Layout.fillWidth: true
                        text: groupRow.name
                        placeholderText: qsTr("Group name")
                        Accessible.name: qsTr("Group name")
                        onEditingFinished: {
                            dialog.places.renameGroup(groupRow.index, text)
                            groupsColumn.list = JSON.parse(dialog.places.groupsJson())
                        }
                    }
                    Quiet {
                        text: groupRow.count === 1 ? qsTr("1 place") : qsTr("%1 places").arg(groupRow.count)
                    }
                    IconButton {
                        iconName: "arrow-up"
                        text: qsTr("Move %1 up").arg(groupRow.name)
                        enabled: groupRow.index > 0
                        onClicked: {
                            dialog.places.shiftGroup(groupRow.index, -1)
                            groupsColumn.list = JSON.parse(dialog.places.groupsJson())
                        }
                    }
                    IconButton {
                        iconName: "arrow-down"
                        text: qsTr("Move %1 down").arg(groupRow.name)
                        enabled: groupRow.index < groupsColumn.list.length - 1
                        onClicked: {
                            dialog.places.shiftGroup(groupRow.index, 1)
                            groupsColumn.list = JSON.parse(dialog.places.groupsJson())
                        }
                    }
                    IconButton {
                        iconName: "trash"
                        text: qsTr("Delete %1; its places stay").arg(groupRow.name)
                        onClicked: {
                            dialog.places.deleteGroup(groupRow.index)
                            groupsColumn.list = JSON.parse(dialog.places.groupsJson())
                        }
                    }
                }
            }
        }
    }

    // Shortcuts: every key, by category, searchable.
    Component {
        id: shortcutsPage

        ColumnLayout {
            id: shortcutsColumn

            readonly property list<var> found: JSON.parse(settings.shortcuts(keySearch.text))

            spacing: Tokens.metrics.space_sm

            Field {
                id: keySearch

                Layout.fillWidth: true
                placeholderText: qsTr("Search shortcuts…")
                Accessible.name: qsTr("Search shortcuts")
            }
            Quiet {
                visible: keyGroups.count === 0
                text: qsTr("No shortcut matches. Try another word.")
            }
            Repeater {
                id: keyGroups

                model: shortcutsColumn.found

                delegate: ColumnLayout {
                    id: keyGroup

                    required property int index
                    readonly property string category: shortcutsColumn.found[index]?.category ?? ""
                    readonly property list<var> shortcuts: shortcutsColumn.found[index]?.shortcuts ?? []

                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        textFormat: Text.PlainText
                        Layout.topMargin: Tokens.metrics.space_sm
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.label_text
                        font.capitalization: Font.AllUppercase
                        text: keyGroup.category
                    }
                    Repeater {
                        model: keyGroup.shortcuts

                        delegate: RowLayout {
                            id: shortcutRow

                            required property int index
                            readonly property string name: keyGroup.shortcuts[index]?.name ?? ""
                            readonly property list<string> keys: keyGroup.shortcuts[index]?.keys ?? []

                            Layout.fillWidth: true

                            Words {
                                Layout.fillWidth: true
                                text: shortcutRow.name
                            }
                            Repeater {
                                model: shortcutRow.keys

                                delegate: Rectangle {
                                    id: keyCap

                                    required property string modelData

                                    implicitWidth: keyText.implicitWidth + 2 * Tokens.metrics.space_xs
                                    implicitHeight: Tokens.metrics.key_cap
                                    radius: Tokens.metrics.radius
                                    color: Tokens.colors.content
                                    border.color: Tokens.colors.border_strong

                                    Text {
                                        id: keyText

                                        textFormat: Text.PlainText
                                        anchors.centerIn: parent
                                        color: Tokens.colors.text
                                        font.family: Tokens.metrics.family_data
                                        font.pixelSize: Tokens.metrics.small_text
                                        text: keyCap.modelData
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // About: Rustphin, its licence and credits, privacy.
    Component {
        id: aboutPage

        ColumnLayout {
            id: aboutColumn

            readonly property string version: settings.about.length > 0 ? JSON.parse(settings.about).version : ""
            readonly property string settingsPath: settings.about.length > 0 ? JSON.parse(settings.about).settings : ""

            spacing: Tokens.metrics.space_sm

            RowLayout {
                visible: !dialog.showingLicence
                spacing: Tokens.metrics.space_md

                Image {
                    source: "qrc:/qt/qml/Rustphin/logo.svg"
                    sourceSize.width: Tokens.metrics.about_logo
                    sourceSize.height: Tokens.metrics.about_logo
                    Accessible.ignored: true
                }
                ColumnLayout {
                    Text {
                        textFormat: Text.PlainText
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_heading
                        font.pixelSize: Tokens.metrics.heading_text
                        text: "Rustphin"
                    }
                    Quiet {
                        text: qsTr("Version %1").arg(aboutColumn.version)
                    }
                    Words {
                        text: qsTr("A file manager for KDE Plasma, written in Rust.")
                    }
                }
            }
            Words {
                visible: !dialog.showingLicence
                Layout.fillWidth: true
                text: qsTr("Licence: GPL-3.0-or-later. Follows the behaviour of Dolphin and KDE. Built with Qt 6, QML and cxx-qt. Icons: Lucide (ISC). Fonts: Inter, Space Grotesk and JetBrains Mono (SIL Open Font License).")
            }
            OutlinedButton {
                visible: !dialog.showingLicence
                text: qsTr("Show the licence")
                onClicked: dialog.showingLicence = true
            }
            Words {
                visible: !dialog.showingLicence
                Layout.fillWidth: true
                text: qsTr("No telemetry. Rustphin never uses the network.")
            }
            Quiet {
                visible: !dialog.showingLicence
                Layout.fillWidth: true
                text: qsTr("Settings: %1").arg(aboutColumn.settingsPath)
            }
            OutlinedButton {
                visible: !dialog.showingLicence
                text: qsTr("Open the settings folder")
                iconName: "folder-open"
                onClicked: settings.openFolder()
            }
            OutlinedButton {
                visible: dialog.showingLicence
                text: qsTr("Back")
                iconName: "arrow-left"
                onClicked: dialog.showingLicence = false
            }
            Text {
                textFormat: Text.PlainText
                visible: dialog.showingLicence
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.small_text
                text: dialog.showingLicence ? settings.licence() : ""
            }
        }
    }
}
