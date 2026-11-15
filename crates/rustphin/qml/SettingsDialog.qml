// Settings (W-106, W-155): a title row with a search across the pages,
// the pages on the left under their headings with About at the foot, and
// the chosen page's sections on the right, each a panel of rows. Rows come
// from Rust (`settings_form`) as JSON and are drawn here by kind; the
// Toolbar, Places groups, Shortcuts and About pages have their own layouts.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    // The `file:` URI of the folder the window shows, or empty.
    required property string shownFolder
    required property PlacesModel places
    // Rust hands the pages over as JSON (`settings_form`): typed here.
    // In a narrow window the page list is a row of tabs over the page.
    readonly property bool narrow: parent !== null && parent.width < Tokens.metrics.settings_narrow
    readonly property list<var> pageList: settings.pages.length > 0 ? JSON.parse(settings.pages) : []
    readonly property list<var> groups: settings.groups.length > 0 ? JSON.parse(settings.groups) : []
    readonly property bool pageShown: pageList.length > settings.page
    readonly property string pageTitle: pageShown ? pageList[settings.page].title : ""
    readonly property string pageDescription: pageShown ? pageList[settings.page].description : ""
    readonly property bool pageResettable: pageShown && pageList[settings.page].resettable === true
    // The headings of the page list, in order; About has none.
    readonly property list<string> sections: pageList.map(page => page.section).filter((section, at, all) => section.length > 0 && all.indexOf(section) === at)
    // The numbers of the pages the search finds.
    property list<int> shown: [0, 1, 2, 3, 4, 5, 6, 7]
    property bool showingLicence: false
    property int toolbarCursor: -1
    property int availableCursor: -1
    // The row whose control had the keyboard when the page was built
    // again, so it gets it back.
    property string focusKey: ""

    // Shows page `number`.
    function choose(number: int) {
        showingLicence = false
        settings.choosePage(number)
    }
    // Moves to the page `delta` steps away among those the search finds.
    function step(delta: int) {
        if (shown.length === 0)
            return
        const at = shown.indexOf(settings.page)
        const next = at < 0 ? (delta > 0 ? 0 : shown.length - 1) : Math.max(0, Math.min(shown.length - 1, at + delta))
        choose(shown[next])
    }
    // Searches the pages; a page the search hides gives way to the first
    // one it finds.
    function search(query: string) {
        shown = JSON.parse(settings.matching(query))
        if (shown.length > 0 && shown.indexOf(settings.page) < 0)
            choose(shown[0])
    }
    function forgetFocus() {
        focusKey = ""
    }
    // Tells Rust of a change, keeping the keyboard on the row's control.
    function change(key: string, value: string) {
        focusKey = key
        settings.set(key, value)
    }

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
    background: FloatingSurface {}
    Component.onCompleted: {
        settings.start(shownFolder)
        open()
    }

    SettingsModel {
        id: settings
    }
    // Once the rows are built again, the row that had the keyboard has it
    // back; later rebuilds leave the keyboard where it is.
    Connections {
        target: settings
        function onRevisionChanged() {
            Qt.callLater(dialog.forgetFocus)
        }
    }

    // The text styles of the dialog.
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
    // A search field: the search icon inside on the left.
    component SearchField: Field {
        id: searchField

        leftPadding: Tokens.metrics.space_sm + Tokens.metrics.icon_ui + Tokens.metrics.space_sm

        Lucide {
            x: Tokens.metrics.space_sm
            anchors.verticalCenter: parent.verticalCenter
            name: "search"
            colour: Tokens.colors.text_muted
        }
    }
    // A key as a cap, in `data-sm`.
    component KeyCap: Rectangle {
        id: keyCap

        required property string keys

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
            text: keyCap.keys
        }
    }
    // A hairline between the rows of a panel.
    component Divider: Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Tokens.metrics.hairline
        color: Tokens.colors.border
    }

    contentItem: ColumnLayout {
        spacing: 0
        Accessible.role: Accessible.Dialog
        Accessible.name: qsTr("Settings")
        // Ctrl+F puts the keyboard in the search, wherever it is.
        Keys.onPressed: event => {
            if (event.key === Qt.Key_F && event.modifiers === Qt.ControlModifier) {
                pageSearch.forceActiveFocus()
                pageSearch.selectAll()
                event.accepted = true
            }
        }

        // Title row: the name, the search across every page, Close.
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_sm
            spacing: Tokens.metrics.space_sm

            Lucide {
                name: "settings"
                colour: Tokens.colors.text_muted
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                font.weight: Font.DemiBold
                text: qsTr("Settings")
                Accessible.role: Accessible.Heading
            }
            SearchField {
                id: pageSearch

                Layout.preferredWidth: dialog.narrow ? Tokens.metrics.settings_search / 2 + Tokens.metrics.space_lg * 2 : Tokens.metrics.settings_search
                rightPadding: dialog.narrow ? Tokens.metrics.space_sm : findKey.width + Tokens.metrics.space_sm * 2
                placeholderText: qsTr("Search settings")
                Accessible.name: qsTr("Search settings (Ctrl+F)")
                onTextChanged: dialog.search(text)
                onAccepted: navigation.forceActiveFocus()

                KeyCap {
                    id: findKey

                    visible: !dialog.narrow && pageSearch.text.length === 0
                    anchors.right: parent.right
                    anchors.rightMargin: Tokens.metrics.space_xs
                    anchors.verticalCenter: parent.verticalCenter
                    keys: "Ctrl+F"
                }
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close")
                tip: qsTr("Close (Esc)")
                onClicked: dialog.close()
            }
        }
        Divider {}

        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: dialog.narrow ? 1 : 3
            columnSpacing: 0
            rowSpacing: 0

            // The pages: under their headings on the left, or a row of
            // tabs in a narrow window; the arrows move between them.
            FocusScope {
                id: navigation

                Layout.preferredWidth: dialog.narrow ? -1 : Tokens.metrics.settings_nav
                Layout.fillWidth: dialog.narrow
                Layout.preferredHeight: dialog.narrow ? Tokens.metrics.setting_row + 2 * Tokens.metrics.space_sm : -1
                Layout.fillHeight: !dialog.narrow
                activeFocusOnTab: true
                focus: true
                Accessible.role: Accessible.List
                Accessible.name: qsTr("Settings pages")
                Keys.onUpPressed: dialog.step(-1)
                Keys.onDownPressed: dialog.step(1)
                Keys.onLeftPressed: dialog.step(-1)
                Keys.onRightPressed: dialog.step(1)

                // A narrow window: one row of tabs.
                ListView {
                    id: tabs

                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.space_sm
                    visible: dialog.narrow
                    orientation: ListView.Horizontal
                    clip: true
                    spacing: Tokens.metrics.space_xs
                    model: dialog.narrow ? dialog.shown : []
                    currentIndex: dialog.shown.indexOf(settings.page)
                    keyNavigationEnabled: false
                    interactive: contentWidth > width
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)

                    delegate: PageRow {
                        required property int modelData

                        number: modelData
                        trailing: ""
                    }
                }

                // A wide window: the headings, and About at the foot.
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.space_sm
                    anchors.topMargin: Tokens.metrics.space_md
                    visible: !dialog.narrow
                    spacing: Tokens.metrics.space_md

                    Repeater {
                        model: dialog.narrow ? [] : dialog.sections

                        delegate: ColumnLayout {
                            id: heading

                            required property string modelData
                            readonly property list<int> members: dialog.shown.filter(number => dialog.pageList[number]?.section === modelData)

                            Layout.fillWidth: true
                            visible: members.length > 0
                            spacing: 2

                            Text {
                                textFormat: Text.PlainText
                                Layout.leftMargin: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
                                Layout.bottomMargin: Tokens.metrics.space_xs
                                color: Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.small_text
                                text: heading.modelData
                                Accessible.role: Accessible.Heading
                            }
                            Repeater {
                                model: heading.members

                                delegate: PageRow {
                                    required property int modelData

                                    Layout.fillWidth: true
                                    number: modelData
                                    trailing: ""
                                }
                            }
                        }
                    }
                    Quiet {
                        Layout.fillWidth: true
                        Layout.leftMargin: Tokens.metrics.space_sm
                        visible: dialog.shown.length === 0
                        text: qsTr("No settings match “%1”. Try another word.").arg(pageSearch.text.trim())
                    }
                    Item {
                        Layout.fillHeight: true
                    }
                    Divider {
                        visible: dialog.shown.indexOf(7) >= 0
                    }
                    PageRow {
                        Layout.fillWidth: true
                        Layout.bottomMargin: Tokens.metrics.space_xs
                        visible: dialog.shown.indexOf(7) >= 0
                        number: 7
                        trailing: settings.about.length > 0 ? JSON.parse(settings.about).version : ""
                    }
                }
            }
            Rectangle {
                Layout.fillHeight: !dialog.narrow
                Layout.fillWidth: dialog.narrow
                Layout.preferredWidth: Tokens.metrics.hairline
                Layout.preferredHeight: Tokens.metrics.hairline
                color: Tokens.colors.border
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Flickable {
                    id: body

                    readonly property real margin: Tokens.metrics.space_lg + Tokens.metrics.space_sm

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentHeight: column.implicitHeight + 2 * margin
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: Scroller {}

                    ColumnLayout {
                        id: column

                        x: body.margin
                        y: body.margin - Tokens.metrics.space_xs
                        width: body.width - 2 * body.margin
                        spacing: Tokens.metrics.space_lg + Tokens.metrics.space_sm

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Tokens.metrics.space_xs

                            Text {
                                textFormat: Text.PlainText
                                Layout.fillWidth: true
                                color: Tokens.colors.text
                                font.family: Tokens.metrics.family_heading
                                font.pixelSize: Tokens.metrics.heading_text
                                font.weight: Font.DemiBold
                                text: dialog.pageTitle
                                Accessible.role: Accessible.Heading
                            }
                            Quiet {
                                Layout.fillWidth: true
                                text: dialog.pageDescription
                            }
                        }

                        // Pages of rows.
                        Repeater {
                            model: dialog.groups

                            delegate: SettingsSection {
                                id: group

                                required property int index
                                readonly property list<var> sectionRows: dialog.groups[index]?.rows ?? []

                                title: dialog.groups[index]?.title ?? ""
                                note: dialog.groups[index]?.note ?? ""

                                Repeater {
                                    model: group.sectionRows

                                    delegate: SettingRow {
                                        rows: group.sectionRows
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

                // The foot of a page with settings.
                Divider {
                    visible: dialog.pageResettable
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Tokens.metrics.setting_line
                    Layout.leftMargin: body.margin
                    Layout.rightMargin: Tokens.metrics.space_md
                    visible: dialog.pageResettable
                    spacing: Tokens.metrics.space_xs

                    Lucide {
                        name: "check"
                        side: Tokens.metrics.icon_small
                        colour: Tokens.colors.success
                    }
                    Quiet {
                        Layout.fillWidth: true
                        text: qsTr("Changes apply at once.")
                    }
                    IconButton {
                        iconName: "rotate-ccw"
                        worded: true
                        iconColour: Tokens.colors.text_muted
                        text: qsTr("Reset %1 to defaults").arg(dialog.pageTitle)
                        tip: qsTr("Puts this page's settings back as they first were")
                        onClicked: settings.reset()
                    }
                }
            }
        }
    }

    // One page in the page list: its icon and name, the chosen one in
    // `selection` with a royal-blue edge on its left (under a tab).
    component PageRow: ItemDelegate {
        id: pageRow

        required property int number
        // Words at the row's right end: About's version.
        required property string trailing
        readonly property string pageTitle: dialog.pageList[number]?.title ?? ""
        readonly property string pageIcon: dialog.pageList[number]?.icon ?? ""
        readonly property bool chosen: settings.page === number

        leftPadding: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
        rightPadding: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
        implicitHeight: Tokens.metrics.setting_row
        focusPolicy: Qt.NoFocus
        Accessible.role: Accessible.ListItem
        Accessible.name: pageTitle
        Accessible.selected: chosen
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: pageRow.chosen ? Tokens.colors.selection : pageRow.hovered ? Tokens.colors.hover : "transparent"
            // The keyboard is on the list: the chosen page's outline, as a
            // focused place's.
            border.width: pageRow.chosen && navigation.activeFocus ? 1 : 0
            border.color: Tokens.colors.accent

            Rectangle {
                visible: pageRow.chosen
                y: dialog.narrow ? parent.height - height : 0
                width: dialog.narrow ? parent.width : Tokens.metrics.selection_bar
                height: dialog.narrow ? Tokens.metrics.selection_bar : parent.height
                color: Tokens.colors.accent
            }
        }
        contentItem: RowLayout {
            spacing: Tokens.metrics.space_sm

            Lucide {
                name: pageRow.pageIcon
                colour: pageRow.chosen ? Tokens.colors.accent : Tokens.colors.text_muted
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                color: pageRow.chosen ? Tokens.colors.on_selection : Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                font.weight: pageRow.chosen ? Font.DemiBold : Font.Normal
                text: pageRow.pageTitle
            }
            Text {
                textFormat: Text.PlainText
                visible: pageRow.trailing.length > 0
                color: pageRow.chosen ? Tokens.colors.on_selection : Tokens.colors.text_muted
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.data_text
                text: pageRow.trailing
            }
        }
        onClicked: {
            dialog.choose(number)
            navigation.forceActiveFocus()
        }
    }

    // One setting: what it is and what it means on the left, its control on
    // the right, or under the words for a wide control.
    component SettingRow: ColumnLayout {
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
        readonly property list<var> segmentList: kind === "segmented" ? (rows[index]?.control?.options ?? []) : []
        readonly property list<var> chipList: kind === "chips" ? (rows[index]?.control?.chips ?? []) : []
        readonly property string valueText: kind === "choice" || kind === "text" || kind === "segmented" || kind === "looks" ? (rows[index]?.control?.value ?? "") : ""
        readonly property real valueNumber: kind === "slider" ? (rows[index]?.control?.value ?? 0) : 0
        readonly property real min: kind === "slider" ? (rows[index]?.control?.min ?? 0) : 0
        readonly property real max: kind === "slider" ? (rows[index]?.control?.max ?? 0) : 0
        readonly property real step: kind === "slider" ? (rows[index]?.control?.step ?? 1) : 1
        readonly property list<string> steps: kind === "slider" ? (rows[index]?.control?.steps ?? []) : []
        readonly property bool isOn: kind === "toggle" && rows[index]?.control?.on === true
        readonly property bool controlEnabled: kind === "toggle" ? rows[index]?.control?.enabled === true : true
        readonly property string placeholder: kind === "text" ? (rows[index]?.control?.placeholder || "") : ""
        readonly property string buttonLabel: kind === "button" || kind === "badge" ? (rows[index]?.control?.label ?? "") : ""
        // Wide controls go under the words.
        readonly property bool stacked: kind === "looks" || kind === "chips" || kind === "text"

        // Takes the keyboard back if this row's control had it before the
        // rows were built again.
        function takeFocus() {
            if (dialog.focusKey.length === 0)
                return
            const chip = chipList.findIndex(chip => chip.key === dialog.focusKey)
            if (dialog.focusKey !== key && chip < 0)
                return
            if (kind === "toggle")
                toggleControl.forceActiveFocus()
            else if (kind === "choice")
                choiceControl.forceActiveFocus()
            else if (kind === "segmented")
                segmentsControl.focusChosen()
            else if (kind === "slider")
                sliderControl.focusSlider()
            else if (kind === "looks")
                (settingLooks.itemAt(valueText === "glass" ? 0 : 1) as LookTile)?.forceActiveFocus()
            else if (kind === "chips")
                (settingChips.itemAt(chip) as Chip)?.forceActiveFocus()
        }

        Layout.fillWidth: true
        spacing: 0
        Component.onCompleted: takeFocus()

        Divider {
            visible: line.index > 0
        }
        GridLayout {
            Layout.fillWidth: true
            Layout.minimumHeight: Tokens.metrics.setting_line - Tokens.metrics.hairline - 2 * Tokens.metrics.space_md
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_lg
            Layout.topMargin: Tokens.metrics.space_md
            Layout.bottomMargin: Tokens.metrics.space_md
            columns: line.stacked ? 1 : 2
            columnSpacing: Tokens.metrics.space_lg
            rowSpacing: Tokens.metrics.space_md

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 2

                RowLayout {
                    id: labelRow

                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_xs

                    Words {
                        Layout.maximumWidth: labelRow.width - (lock.visible ? lock.width + labelRow.spacing : 0)
                        color: line.controlEnabled ? Tokens.colors.text : Tokens.colors.text_muted
                        text: line.label
                    }
                    // A setting that cannot be changed says so by a lock too.
                    Lucide {
                        id: lock

                        visible: !line.controlEnabled && line.kind === "toggle" && line.isOn
                        name: "lock"
                        side: Tokens.metrics.icon_small
                        colour: Tokens.colors.text_muted
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                }
                Quiet {
                    Layout.fillWidth: true
                    visible: line.hint.length > 0
                    color: line.warning ? Tokens.colors.danger : Tokens.colors.text_muted
                    text: line.hint
                }
            }

            // The controls; one shows.
            SettingSwitch {
                id: toggleControl

                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "toggle"
                checked: line.isOn
                enabled: line.controlEnabled
                Accessible.name: line.label
                Accessible.description: line.hint
                onToggled: dialog.change(line.key, checked ? "true" : "false")
            }
            SettingChoice {
                id: choiceControl

                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "choice"
                model: line.options.map(option => option[1])
                currentIndex: line.options.findIndex(option => option[0] === line.valueText)
                Accessible.name: line.label
                onActivated: index => dialog.change(line.key, line.options[index][0])
            }
            Segments {
                id: segmentsControl

                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "segmented"
                options: line.segmentList
                value: line.valueText
                name: line.label
                onChosen: id => dialog.change(line.key, id)
            }
            SettingSlider {
                id: sliderControl

                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "slider"
                from: line.min
                to: line.max
                stepSize: line.step
                value: line.valueNumber
                steps: line.steps
                name: line.label
                onSettled: value => dialog.change(line.key, String(value))
            }
            OutlinedButton {
                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "button"
                text: line.buttonLabel
                onClicked: dialog.change(line.key, "")
            }
            Row {
                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                visible: line.kind === "badge"
                spacing: Tokens.metrics.space_xs
                Accessible.role: Accessible.StaticText
                Accessible.name: line.label + ": " + line.buttonLabel

                Lucide {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "circle-check"
                    colour: Tokens.colors.success
                }
                Text {
                    textFormat: Text.PlainText
                    anchors.verticalCenter: parent.verticalCenter
                    color: Tokens.colors.success
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    font.weight: Font.DemiBold
                    text: line.buttonLabel
                }
            }
            GridLayout {
                Layout.fillWidth: true
                visible: line.kind === "looks"
                columns: 2
                columnSpacing: Tokens.metrics.space_md
                Accessible.role: Accessible.Grouping
                Accessible.name: line.label

                Repeater {
                    id: settingLooks

                    model: line.kind === "looks" ? ["glass", "opaque"] : []

                    delegate: LookTile {
                        required property string modelData

                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        glass: modelData === "glass"
                        checked: line.valueText === modelData
                        onClicked: dialog.change(line.key, modelData)
                    }
                }
            }
            Flow {
                Layout.fillWidth: true
                visible: line.kind === "chips"
                spacing: Tokens.metrics.space_sm

                Repeater {
                    id: settingChips

                    model: line.chipList

                    delegate: Chip {
                        required property int index
                        readonly property string chipKey: line.chipList[index]?.key ?? ""

                        text: line.chipList[index]?.words ?? ""
                        checked: line.chipList[index]?.on === true
                        onToggled: dialog.change(chipKey, checked ? "true" : "false")
                    }
                }
            }
            Field {
                Layout.fillWidth: true
                Layout.maximumWidth: Tokens.metrics.setting_field
                visible: line.kind === "text"
                text: line.valueText
                placeholderText: line.placeholder
                Accessible.name: line.label
                onAccepted: settings.set(line.key, text)
                onEditingFinished: settings.set(line.key, text)
            }
        }
    }

    // Toolbar: how it looks, what can be added, what is on it, the order.
    Component {
        id: toolbarPage

        ColumnLayout {
            id: page

            readonly property list<var> onBar: settings.toolbar.length > 0 ? JSON.parse(settings.toolbar) : []
            property list<var> offered: JSON.parse(settings.available(search.text))
            readonly property bool picked: dialog.toolbarCursor >= 0 && dialog.toolbarCursor < onBar.length

            spacing: Tokens.metrics.space_lg + Tokens.metrics.space_sm

            Connections {
                target: settings
                function onRevisionChanged() {
                    page.offered = JSON.parse(settings.available(search.text))
                }
            }

            SettingsSection {
                title: qsTr("Preview")
                note: qsTr("The toolbar as it will look. Pick a button to find it in the list.")

                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Tokens.metrics.toolbar
                    Layout.leftMargin: Tokens.metrics.space_sm
                    Layout.rightMargin: Tokens.metrics.space_sm
                    spacing: 0
                    clip: true

                    Repeater {
                        model: page.onBar

                        delegate: Item {
                            id: preview

                            required property int index
                            readonly property string itemId: page.onBar[index]?.id ?? ""
                            readonly property string itemName: page.onBar[index]?.name ?? ""
                            readonly property string itemIcon: page.onBar[index]?.icon ?? ""
                            readonly property bool worded: page.onBar[index]?.words === true

                            Layout.fillWidth: itemId === "location"
                            Layout.minimumWidth: itemId === "separator" ? Tokens.metrics.space_sm : itemId === "location" ? Tokens.metrics.slider_value + Tokens.metrics.space_lg : previewButton.implicitWidth
                            Layout.preferredWidth: Layout.minimumWidth
                            Layout.preferredHeight: Tokens.metrics.setting_control

                            Rectangle {
                                visible: preview.itemId === "separator"
                                anchors.centerIn: parent
                                width: Tokens.metrics.hairline
                                height: Tokens.metrics.icon_ui + Tokens.metrics.space_xs
                                color: Tokens.colors.border_strong
                            }
                            // The location field, drawn small.
                            Rectangle {
                                visible: preview.itemId === "location"
                                anchors.fill: parent
                                radius: Tokens.metrics.radius
                                color: Tokens.colors.content
                                border.width: dialog.toolbarCursor === preview.index ? 2 : 1
                                border.color: dialog.toolbarCursor === preview.index ? Tokens.colors.accent : Tokens.colors.border

                                Row {
                                    x: Tokens.metrics.space_sm
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: Tokens.metrics.space_xs

                                    Lucide {
                                        anchors.verticalCenter: parent.verticalCenter
                                        name: "folder"
                                        side: Tokens.metrics.icon_small
                                    }
                                    Text {
                                        textFormat: Text.PlainText
                                        anchors.verticalCenter: parent.verticalCenter
                                        color: Tokens.colors.text_muted
                                        font.family: Tokens.metrics.family_data
                                        font.pixelSize: Tokens.metrics.data_text
                                        text: qsTr("Location")
                                    }
                                }
                                TapHandler {
                                    onTapped: dialog.toolbarCursor = preview.index
                                }
                            }
                            IconButton {
                                id: previewButton

                                side: Tokens.metrics.setting_control
                                visible: preview.itemId !== "separator" && preview.itemId !== "location"
                                anchors.fill: parent
                                iconName: preview.itemIcon
                                text: preview.itemName
                                worded: preview.worded
                                checked: dialog.toolbarCursor === preview.index
                                onClicked: dialog.toolbarCursor = preview.index
                            }
                        }
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 3
                columnSpacing: Tokens.metrics.space_md
                rowSpacing: Tokens.metrics.space_sm

                Text {
                    textFormat: Text.PlainText
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    font.weight: Font.DemiBold
                    text: qsTr("Available")
                }
                Item {}
                Text {
                    textFormat: Text.PlainText
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    font.weight: Font.DemiBold
                    text: qsTr("On the toolbar")
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: Tokens.metrics.settings_list
                    radius: Tokens.metrics.radius
                    color: Tokens.colors.panel
                    border.color: Tokens.colors.border

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Tokens.metrics.hairline
                        spacing: 0

                        SearchField {
                            id: search

                            Layout.fillWidth: true
                            Layout.margins: Tokens.metrics.space_xs
                            placeholderText: qsTr("Search actions")
                            Accessible.name: qsTr("Search the available actions")
                            onTextChanged: page.offered = JSON.parse(settings.available(text))
                        }
                        ItemList {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            items: page.offered
                            cursor: dialog.availableCursor
                            name: qsTr("Available actions")
                            empty: qsTr("Nothing matches; everything else is on the toolbar.")
                            onPicked: index => dialog.availableCursor = index
                            onChosen: index => {
                                dialog.availableCursor = index
                                dialog.toolbarCursor = settings.addItem(search.text, index, dialog.toolbarCursor)
                                dialog.availableCursor = -1
                            }
                        }
                    }
                }
                ColumnLayout {
                    Layout.alignment: Qt.AlignVCenter
                    spacing: Tokens.metrics.space_xs

                    MoveButton {
                        iconName: "arrow-right"
                        text: qsTr("Add to the toolbar")
                        enabled: dialog.availableCursor >= 0
                        onClicked: {
                            dialog.toolbarCursor = settings.addItem(search.text, dialog.availableCursor, dialog.toolbarCursor)
                            dialog.availableCursor = -1
                        }
                    }
                    MoveButton {
                        iconName: "arrow-left"
                        text: qsTr("Take off the toolbar")
                        enabled: page.picked && !page.onBar[dialog.toolbarCursor].fixed
                        onClicked: {
                            settings.removeItem(dialog.toolbarCursor)
                            dialog.toolbarCursor = -1
                        }
                    }
                    Divider {
                        Layout.preferredWidth: Tokens.metrics.space_lg
                        Layout.alignment: Qt.AlignHCenter
                        Layout.fillWidth: false
                        Layout.topMargin: Tokens.metrics.space_xs
                        Layout.bottomMargin: Tokens.metrics.space_xs
                    }
                    MoveButton {
                        iconName: "arrow-up"
                        text: qsTr("Move left on the toolbar")
                        enabled: dialog.toolbarCursor > 0
                        onClicked: dialog.toolbarCursor = settings.shiftItem(dialog.toolbarCursor, -1)
                    }
                    MoveButton {
                        iconName: "arrow-down"
                        text: qsTr("Move right on the toolbar")
                        enabled: page.picked && dialog.toolbarCursor < page.onBar.length - 1
                        onClicked: dialog.toolbarCursor = settings.shiftItem(dialog.toolbarCursor, 1)
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: Tokens.metrics.settings_list
                    radius: Tokens.metrics.radius
                    color: Tokens.colors.panel
                    border.color: Tokens.colors.border

                    ItemList {
                        anchors.fill: parent
                        anchors.margins: Tokens.metrics.hairline
                        items: page.onBar
                        cursor: dialog.toolbarCursor
                        name: qsTr("On the toolbar")
                        empty: ""
                        onPicked: index => dialog.toolbarCursor = index
                        onChosen: index => {
                            if (!page.onBar[index].fixed) {
                                settings.removeItem(index)
                                dialog.toolbarCursor = -1
                            }
                        }
                    }
                }
            }

            SettingsSection {
                title: qsTr("Buttons")

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumHeight: Tokens.metrics.setting_line - 2 * Tokens.metrics.space_md
                        Layout.leftMargin: Tokens.metrics.space_lg
                        Layout.rightMargin: Tokens.metrics.space_lg
                        Layout.topMargin: Tokens.metrics.space_md
                        Layout.bottomMargin: Tokens.metrics.space_md
                        spacing: Tokens.metrics.space_lg

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Words {
                                Layout.fillWidth: true
                                color: wordsSwitch.enabled ? Tokens.colors.text : Tokens.colors.text_muted
                                text: page.picked ? qsTr("Show the words of %1").arg(page.onBar[dialog.toolbarCursor].name) : qsTr("Show a button's words")
                            }
                            Quiet {
                                Layout.fillWidth: true
                                text: !page.picked ? qsTr("Pick a button on the toolbar first.") : wordsSwitch.enabled ? qsTr("A wider button, easier to tell apart.") : qsTr("This item has no words to show.")
                            }
                        }
                        SettingSwitch {
                            id: wordsSwitch

                            enabled: page.picked && page.onBar[dialog.toolbarCursor].canShowWords
                            checked: page.picked && page.onBar[dialog.toolbarCursor].words
                            Accessible.name: qsTr("Show its words")
                            onToggled: settings.setWords(dialog.toolbarCursor, checked)
                        }
                    }
                }
            }
        }
    }

    // A bordered icon button between the Toolbar page's lists.
    component MoveButton: IconButton {
        id: move

        side: Tokens.metrics.button

        background: Rectangle {
            radius: Tokens.metrics.radius
            color: move.down ? Tokens.colors.selection : move.hovered && move.enabled ? Tokens.colors.hover : "transparent"
            border.width: move.visualFocus ? 2 : 1
            border.color: move.visualFocus ? Tokens.colors.accent : move.enabled ? Tokens.colors.border_strong : Tokens.colors.border
        }
    }

    // A list of toolbar items to pick from; a double click moves the item.
    component ItemList: ListView {
        id: box

        required property list<var> items
        required property int cursor
        required property string name
        required property string empty

        signal picked(int index)
        signal chosen(int index)

        clip: true
        topMargin: Tokens.metrics.space_xs
        bottomMargin: Tokens.metrics.space_xs
        leftMargin: Tokens.metrics.space_xs
        rightMargin: Tokens.metrics.space_xs
        model: items
        Accessible.role: Accessible.List
        Accessible.name: name
        ScrollBar.vertical: Scroller {}
        // An item picked elsewhere (in the preview) is scrolled to.
        onCursorChanged: {
            if (cursor >= 0)
                positionViewAtIndex(cursor, ListView.Contain)
        }
        delegate: ItemDelegate {
            id: item

            required property int index
            readonly property string name: box.items[index]?.name ?? ""
            readonly property string iconName: box.items[index]?.icon ?? ""
            readonly property bool fixed: box.items[index]?.fixed === true
            readonly property bool current: box.cursor === index

            width: ListView.view.width - Tokens.metrics.space_sm
            implicitHeight: Tokens.metrics.setting_row
            leftPadding: Tokens.metrics.space_sm
            rightPadding: Tokens.metrics.space_sm
            Accessible.name: name
            Accessible.selected: current
            background: Rectangle {
                radius: Tokens.metrics.radius
                color: item.current ? Tokens.colors.selection : item.hovered ? Tokens.colors.hover : "transparent"
                border.width: item.current ? 1 : 0
                border.color: Tokens.colors.accent
            }
            contentItem: RowLayout {
                spacing: Tokens.metrics.space_sm

                Lucide {
                    name: item.iconName
                    colour: item.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                }
                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: item.current ? Tokens.colors.on_selection : Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: item.name
                }
                Text {
                    textFormat: Text.PlainText
                    visible: item.fixed
                    color: item.current ? Tokens.colors.on_selection : Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.small_text
                    text: qsTr("Always there")
                }
            }
            onClicked: box.picked(index)
            onDoubleClicked: box.chosen(index)
        }

        Quiet {
            anchors.centerIn: parent
            width: parent.width - 2 * Tokens.metrics.space_lg
            horizontalAlignment: Text.AlignHCenter
            visible: box.items.length === 0 && box.empty.length > 0
            text: box.empty
        }
    }

    // Places: the panel's groups, moved, renamed or deleted.
    Component {
        id: groupsPage

        SettingsSection {
            id: groupsColumn

            property list<var> list: JSON.parse(dialog.places.groupsJson())

            function reload() {
                list = JSON.parse(dialog.places.groupsJson())
            }

            title: qsTr("Groups")
            note: list.length === 0 ? "" : qsTr("Deleting a group keeps its places.")

            Connections {
                target: dialog.places
                function onCountChanged() {
                    groupsColumn.reload()
                }
            }

            Quiet {
                Layout.fillWidth: true
                Layout.margins: Tokens.metrics.space_lg
                visible: groupsColumn.list.length === 0
                text: qsTr("No groups yet. Make one with New group in the places panel.")
            }
            Repeater {
                model: groupsColumn.list

                delegate: ColumnLayout {
                    id: groupRow

                    required property int index
                    readonly property bool separator: groupsColumn.list[index]?.separator ?? false
                    readonly property string name: separator ? qsTr("Separator") : groupsColumn.list[index]?.name ?? ""
                    readonly property int count: groupsColumn.list[index]?.count ?? 0

                    Layout.fillWidth: true
                    spacing: 0

                    Divider {
                        visible: groupRow.index > 0
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Tokens.metrics.setting_line - Tokens.metrics.hairline
                        Layout.leftMargin: Tokens.metrics.space_sm
                        Layout.rightMargin: Tokens.metrics.space_sm
                        spacing: Tokens.metrics.space_xs

                        // A separator between groups (W-152): moved and
                        // deleted here too, with nothing to name.
                        Lucide {
                            visible: groupRow.separator
                            Layout.leftMargin: Tokens.metrics.space_sm
                            name: "separator-horizontal"
                        }
                        Words {
                            visible: groupRow.separator
                            Layout.fillWidth: true
                            Layout.leftMargin: Tokens.metrics.space_xs
                            color: Tokens.colors.text_muted
                            text: qsTr("Separator")
                        }
                        // The name, edited where it stands.
                        TextField {
                            id: groupName

                            visible: !groupRow.separator
                            Layout.fillWidth: true
                            implicitHeight: Tokens.metrics.setting_control
                            leftPadding: Tokens.metrics.space_sm
                            text: groupRow.name
                            color: Tokens.colors.text
                            placeholderText: qsTr("Group name")
                            placeholderTextColor: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            Accessible.name: qsTr("Group name")
                            background: Rectangle {
                                radius: Tokens.metrics.radius
                                color: groupName.activeFocus ? Tokens.colors.content : "transparent"
                                border.width: groupName.activeFocus ? 2 : 1
                                border.color: groupName.activeFocus ? Tokens.colors.accent : groupName.hovered ? Tokens.colors.border_strong : "transparent"
                            }
                            onEditingFinished: {
                                dialog.places.renameGroup(groupRow.index, text)
                                groupsColumn.reload()
                            }
                        }
                        Quiet {
                            visible: !groupRow.separator
                            Layout.preferredWidth: Tokens.metrics.slider_value
                            horizontalAlignment: Text.AlignRight
                            text: groupRow.count === 1 ? qsTr("1 place") : qsTr("%1 places").arg(groupRow.count)
                        }
                        IconButton {
                            iconName: "arrow-up"
                            text: qsTr("Move %1 up").arg(groupRow.name)
                            enabled: groupRow.index > 0
                            onClicked: {
                                dialog.places.shiftGroup(groupRow.index, -1)
                                groupsColumn.reload()
                            }
                        }
                        IconButton {
                            iconName: "arrow-down"
                            text: qsTr("Move %1 down").arg(groupRow.name)
                            enabled: groupRow.index < groupsColumn.list.length - 1
                            onClicked: {
                                dialog.places.shiftGroup(groupRow.index, 1)
                                groupsColumn.reload()
                            }
                        }
                        IconButton {
                            iconName: "trash"
                            text: groupRow.separator ? qsTr("Delete separator") : qsTr("Delete %1; its places stay").arg(groupRow.name)
                            hoverFill: Tokens.colors.danger_surface
                            onClicked: {
                                dialog.places.deleteGroup(groupRow.index)
                                groupsColumn.reload()
                            }
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

            spacing: Tokens.metrics.space_lg + Tokens.metrics.space_sm

            SearchField {
                id: keySearch

                Layout.fillWidth: true
                implicitHeight: Tokens.metrics.button
                placeholderText: qsTr("Search by action or key, such as Ctrl+L")
                Accessible.name: qsTr("Search shortcuts")
            }
            Quiet {
                visible: keyGroups.count === 0
                text: qsTr("No shortcut matches “%1”. Try another word.").arg(keySearch.text.trim())
            }
            Repeater {
                id: keyGroups

                model: shortcutsColumn.found

                delegate: SettingsSection {
                    id: keyGroup

                    required property int index
                    readonly property list<var> shortcuts: shortcutsColumn.found[index]?.shortcuts ?? []

                    title: shortcutsColumn.found[index]?.category ?? ""

                    Repeater {
                        model: keyGroup.shortcuts

                        delegate: ColumnLayout {
                            id: shortcutRow

                            required property int index
                            readonly property string name: keyGroup.shortcuts[index]?.name ?? ""
                            readonly property list<string> keys: keyGroup.shortcuts[index]?.keys ?? []

                            Layout.fillWidth: true
                            spacing: 0

                            Divider {
                                visible: shortcutRow.index > 0
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Tokens.metrics.control_line - Tokens.metrics.space_xs - Tokens.metrics.hairline
                                Layout.leftMargin: Tokens.metrics.space_lg
                                Layout.rightMargin: Tokens.metrics.space_lg
                                spacing: Tokens.metrics.space_xs

                                Words {
                                    Layout.fillWidth: true
                                    text: shortcutRow.name
                                }
                                Repeater {
                                    model: shortcutRow.keys

                                    delegate: RowLayout {
                                        id: keyPart

                                        required property int index
                                        required property string modelData

                                        spacing: Tokens.metrics.space_xs

                                        Quiet {
                                            visible: keyPart.index > 0
                                            text: qsTr("or")
                                        }
                                        KeyCap {
                                            keys: keyPart.modelData
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // About: Rustphin, its licence and credits, privacy, the settings file.
    Component {
        id: aboutPage

        ColumnLayout {
            id: aboutColumn

            readonly property string version: settings.about.length > 0 ? JSON.parse(settings.about).version : ""
            readonly property string settingsPath: settings.about.length > 0 ? JSON.parse(settings.about).settings : ""
            readonly property list<var> credits: [[qsTr("Licence"), "GPL-3.0-or-later"], [qsTr("Behaviour"), qsTr("Follows Dolphin and KDE")], [qsTr("Built with"), "Qt 6, QML and cxx-qt"], [qsTr("Icons"), "Lucide (ISC)"], [qsTr("Fonts"), qsTr("Inter, Space Grotesk and JetBrains Mono (SIL Open Font License)")]]

            spacing: Tokens.metrics.space_lg + Tokens.metrics.space_sm

            RowLayout {
                visible: !dialog.showingLicence
                Layout.fillWidth: true
                spacing: Tokens.metrics.space_lg

                Image {
                    source: "qrc:/qt/qml/Rustphin/logo.svg"
                    sourceSize.width: Tokens.metrics.about_logo
                    sourceSize.height: Tokens.metrics.about_logo
                    Accessible.ignored: true
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_xs

                    RowLayout {
                        spacing: Tokens.metrics.space_sm

                        Text {
                            textFormat: Text.PlainText
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_heading
                            font.pixelSize: Tokens.metrics.heading_text
                            font.weight: Font.DemiBold
                            text: "Rustphin"
                        }
                        KeyCap {
                            keys: aboutColumn.version
                            Accessible.role: Accessible.StaticText
                            Accessible.name: qsTr("Version %1").arg(aboutColumn.version)
                        }
                    }
                    Quiet {
                        Layout.fillWidth: true
                        text: qsTr("A file manager for KDE Plasma, written in Rust.")
                    }
                }
                OutlinedButton {
                    text: qsTr("Show the licence")
                    iconName: "file-text"
                    onClicked: dialog.showingLicence = true
                }
            }
            // Privacy, said once and plainly.
            Rectangle {
                visible: !dialog.showingLicence
                Layout.fillWidth: true
                implicitHeight: privacy.implicitHeight + 2 * Tokens.metrics.space_md
                radius: Tokens.metrics.radius
                color: Tokens.colors.panel
                border.color: Tokens.colors.border

                RowLayout {
                    id: privacy

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: Tokens.metrics.space_lg
                    anchors.rightMargin: Tokens.metrics.space_lg
                    spacing: Tokens.metrics.space_md

                    Lucide {
                        Layout.alignment: Qt.AlignTop
                        name: "shield"
                        colour: Tokens.colors.success
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Words {
                            Layout.fillWidth: true
                            font.weight: Font.DemiBold
                            text: qsTr("No telemetry")
                        }
                        Quiet {
                            Layout.fillWidth: true
                            text: qsTr("Rustphin never uses the network.")
                        }
                    }
                }
            }
            SettingsSection {
                visible: !dialog.showingLicence
                title: qsTr("Credits")

                Repeater {
                    model: aboutColumn.credits

                    delegate: ColumnLayout {
                        id: credit

                        required property int index

                        Layout.fillWidth: true
                        spacing: 0

                        Divider {
                            visible: credit.index > 0
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.leftMargin: Tokens.metrics.space_lg
                            Layout.rightMargin: Tokens.metrics.space_lg
                            Layout.topMargin: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
                            Layout.bottomMargin: Tokens.metrics.space_sm + Tokens.metrics.hairline * 2
                            spacing: Tokens.metrics.space_lg

                            Words {
                                Layout.preferredWidth: Tokens.metrics.fact_label
                                Layout.alignment: Qt.AlignTop
                                color: Tokens.colors.text_muted
                                text: aboutColumn.credits[credit.index][0]
                            }
                            Words {
                                Layout.fillWidth: true
                                text: aboutColumn.credits[credit.index][1]
                            }
                        }
                    }
                }
            }
            SettingsSection {
                visible: !dialog.showingLicence
                title: qsTr("Settings file")

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: Tokens.metrics.space_md
                    Layout.leftMargin: Tokens.metrics.space_lg
                    spacing: Tokens.metrics.space_lg

                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: aboutColumn.settingsPath
                        Accessible.role: Accessible.StaticText
                        Accessible.name: qsTr("Settings file: %1").arg(text)
                    }
                    OutlinedButton {
                        text: qsTr("Open the settings folder")
                        iconName: "folder-open"
                        onClicked: settings.openFolder()
                    }
                }
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
                lineHeight: Tokens.metrics.text_line
                lineHeightMode: Text.FixedHeight
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_data
                font.pixelSize: Tokens.metrics.small_text
                text: dialog.showingLicence ? settings.licence() : ""
            }
        }
    }
}
