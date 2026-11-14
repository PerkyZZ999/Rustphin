// A question the window waits on (mockups 19–22): a file operation's, or
// whether to run a program. What happened, the facts, the names it is
// about and the choices, each with its keys; the wording and the choices
// come from Rust (`questions::show`, `questions::launch`) as JSON.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    // The question as JSON from Rust (`rustphin_view::questions`), read once
    // per change into typed parts.
    property string json: ""
    property bool shown: false
    property string title: ""
    property string description: ""
    property string iconName: ""
    property string sentence: ""
    property list<var> facts: []
    property list<var> nameList: []
    property string note: ""
    property bool asksPassword: false
    property bool offersApplyToAll: false
    property list<var> buttonList: []
    property bool applyToAll: false

    function readQuestion() {
        const read = json.length > 0 ? JSON.parse(json) : null
        title = read ? read.title : ""
        description = read ? read.description : ""
        iconName = read ? read.icon : ""
        sentence = read ? read.sentence : ""
        facts = read ? read.facts : []
        nameList = read ? read.names : []
        note = read ? read.note : ""
        asksPassword = read !== null && read.password === true
        offersApplyToAll = read !== null && read.applyToAll === true
        buttonList = read ? read.buttons : []
        shown = read !== null
    }

    onJsonChanged: readQuestion()

    // The button pressed, with the box and the password field.
    signal answered(int answer, bool applyToAll, string password)

    function answer(value: int) {
        const typed = passwordField.text
        const all = applyToAll
        passwordField.text = ""
        applyToAll = false
        answered(value, all, typed)
    }

    // The answer of the button `key` presses; -1 for none. K keeps both
    // names in a conflict, as Enter does.
    function answerFor(key: string): int {
        if (key === "k" && offersApplyToAll)
            key = "enter"
        for (const button of buttonList) {
            if (button.enabled && button.keys.indexOf(key) >= 0)
                return button.answer
        }
        return -1
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    // Wide enough for its buttons on one line when the window allows;
    // narrower windows wrap them.
    width: Math.min(Math.max(Tokens.metrics.dialog, buttonsWidth + 2 * Tokens.metrics.space_md), parent ? parent.width - 2 * Tokens.metrics.space_lg : Tokens.metrics.dialog)
    property real buttonsWidth: 0

    // The buttons' width on one line.
    function measure() {
        let width = 0
        let count = 0
        for (let i = 0; i < answers.count; ++i) {
            const button = answers.itemAt(i)
            if (button) {
                width += button.implicitWidth
                count += 1
            }
        }
        buttonsWidth = width + Math.max(0, count - 1) * Tokens.metrics.space_sm
    }
    modal: true
    focus: true
    visible: shown
    closePolicy: Popup.NoAutoClose
    padding: 0

    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: Rectangle {
        radius: Tokens.metrics.radius
        color: Tokens.colors.raised
        border.color: Tokens.colors.border_strong
    }
    onOpened: {
        if (asksPassword)
            passwordField.forceActiveFocus()
        else
            keyCatcher.forceActiveFocus()
    }

    contentItem: Item {
        id: keyCatcher

        implicitHeight: content.implicitHeight
        focus: true
        Accessible.role: Accessible.Dialog
        Accessible.name: dialog.title
        Accessible.description: dialog.description

        // Single letters answer, as in Dolphin; not while a field is typed in.
        Keys.onPressed: event => {
            if (!dialog.shown)
                return
            if (event.key === Qt.Key_A && dialog.offersApplyToAll) {
                dialog.applyToAll = !dialog.applyToAll
                event.accepted = true
                return
            }
            const key = event.key === Qt.Key_Escape ? "escape" : (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) ? "enter" : event.text.toLowerCase()
            const value = dialog.answerFor(key)
            if (value >= 0) {
                dialog.answer(value)
                event.accepted = true
            }
        }

        ColumnLayout {
            id: content

            anchors.left: parent.left
            anchors.right: parent.right
            spacing: Tokens.metrics.space_md

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: Tokens.metrics.space_lg
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                spacing: Tokens.metrics.space_sm

                Lucide {
                    name: dialog.iconName
                    colour: Tokens.colors.text
                    side: Tokens.metrics.icon_large / 1.5
                }
                Text {
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_heading
                    font.pixelSize: Tokens.metrics.title_text
                    text: dialog.title
                    Accessible.role: Accessible.Heading
                }
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: dialog.sentence
            }
            Repeater {
                model: dialog.facts

                delegate: RowLayout {
                    id: fact

                    required property list<string> modelData

                    Layout.fillWidth: true
                    Layout.leftMargin: Tokens.metrics.space_lg
                    Layout.rightMargin: Tokens.metrics.space_lg
                    spacing: Tokens.metrics.space_sm

                    Text {
                        textFormat: Text.PlainText
                        Layout.preferredWidth: Tokens.metrics.fact_label
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: fact.modelData[0]
                    }
                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: fact.modelData[1]
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                Layout.preferredHeight: Math.min(names.contentHeight, Tokens.metrics.menu_visible / 2)
                visible: dialog.nameList.length > 0
                radius: Tokens.metrics.radius
                color: "transparent"
                border.color: Tokens.colors.border

                ListView {
                    id: names

                    anchors.fill: parent
                    clip: true
                    model: dialog.nameList
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: RowLayout {
                        id: listed

                        required property int index
                        readonly property string iconName: dialog.nameList[index]?.icon ?? ""
                        readonly property string name: dialog.nameList[index]?.name ?? ""
                        readonly property string detail: dialog.nameList[index]?.detail ?? ""

                        width: ListView.view.width
                        height: Tokens.metrics.control
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            Layout.leftMargin: Tokens.metrics.space_md
                            visible: listed.iconName.length > 0
                            name: listed.iconName
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            Layout.leftMargin: listed.iconName.length > 0 ? 0 : Tokens.metrics.space_md
                            elide: Text.ElideMiddle
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.data_text
                            text: listed.name
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.rightMargin: Tokens.metrics.space_md
                            visible: listed.detail.length > 0
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: listed.detail
                        }
                    }
                }
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                visible: text.length > 0
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.small_text
                text: dialog.note
            }
            TextField {
                id: passwordField

                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                visible: dialog.asksPassword
                implicitHeight: Tokens.metrics.button
                echoMode: TextInput.Password
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                Accessible.name: qsTr("Archive password")
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: Tokens.colors.content
                    border.width: passwordField.activeFocus ? 2 : 1
                    border.color: passwordField.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                }
                Keys.onReturnPressed: dialog.answer(3)
                Keys.onEnterPressed: dialog.answer(3)
                Keys.onEscapePressed: dialog.answer(0)
            }
            CheckBox {
                id: all

                Layout.leftMargin: Tokens.metrics.space_lg
                visible: dialog.offersApplyToAll
                checked: dialog.applyToAll
                text: qsTr("Do the same for all other conflicts (A)")
                onToggled: dialog.applyToAll = checked
                contentItem: Text {
                    textFormat: Text.PlainText
                    leftPadding: all.indicator.width + Tokens.metrics.space_sm
                    verticalAlignment: Text.AlignVCenter
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: all.text
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: buttons.implicitHeight + 2 * Tokens.metrics.space_md
                color: Tokens.colors.chrome
                radius: Tokens.metrics.radius

                Flow {
                    id: buttons

                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.space_md
                    spacing: Tokens.metrics.space_sm
                    // Right-aligned, so the order is reversed: the primary
                    // answer ends on the right, as in the mockups.
                    layoutDirection: Qt.RightToLeft

                    Repeater {
                        id: answers

                        onItemAdded: Qt.callLater(dialog.measure)
                        onItemRemoved: Qt.callLater(dialog.measure)
                        // Right to left: the order is reversed, so the
                        // primary answer ends on the right.
                        model: dialog.buttonList.slice().reverse()

                        delegate: OutlinedButton {
                            id: answerButton

                            required property int index
                            readonly property int value: answers.model[index]?.answer ?? 0
                            readonly property string style: answers.model[index]?.style ?? ""

                            text: answers.model[index]?.label ?? ""
                            iconName: answers.model[index]?.icon ?? ""
                            enabled: answers.model[index]?.enabled === true
                            primary: style === "primary"
                            destructive: style === "destructive"
                            onClicked: dialog.answer(answerButton.value)
                        }
                    }
                }
            }
        }
    }
}
