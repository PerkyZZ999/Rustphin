// A question the window waits on (mockups 19–22): a file operation's, or
// whether to run a program. What happened, the facts, the names it is
// about and the choices, each with its keys; the wording and the choices
// come from Rust (`questions::show`, `questions::launch`) as JSON.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Rustphin

Popup {
    id: dialog

    property string json: ""
    readonly property var shown: json.length > 0 ? JSON.parse(json) : null
    property bool applyToAll: false

    // The button pressed, with the box and the password field.
    signal answered(int answer, bool applyToAll, string password)

    function answer(value: int) {
        const typed = password.text
        const all = applyToAll
        password.text = ""
        applyToAll = false
        answered(value, all, typed)
    }

    // The answer of the button `key` presses; -1 for none. K keeps both
    // names in a conflict, as Enter does.
    function answerFor(key: string): int {
        if (key === "k" && shown.applyToAll)
            key = "enter"
        for (const button of shown.buttons) {
            if (button.enabled && button.keys.indexOf(key) >= 0)
                return button.answer
        }
        return -1
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.dialog, parent ? parent.width - 2 * Tokens.metrics.space_lg : Tokens.metrics.dialog)
    modal: true
    focus: true
    visible: shown !== null
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
        if (shown && shown.password)
            password.forceActiveFocus()
        else
            keyCatcher.forceActiveFocus()
    }

    contentItem: Item {
        id: keyCatcher

        implicitHeight: content.implicitHeight
        focus: true
        Accessible.role: Accessible.Dialog
        Accessible.name: dialog.shown ? dialog.shown.title : ""
        Accessible.description: dialog.shown ? dialog.shown.description : ""

        // Single letters answer, as in Dolphin; not while a field is typed in.
        Keys.onPressed: event => {
            if (!dialog.shown)
                return
            if (event.key === Qt.Key_A && dialog.shown.applyToAll) {
                dialog.applyToAll = !dialog.applyToAll
                event.accepted = true
                return
            }
            const key = event.key === Qt.Key_Escape ? "escape"
                      : (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) ? "enter"
                      : event.text.toLowerCase()
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
                    name: dialog.shown ? dialog.shown.icon : ""
                    colour: Tokens.colors.text
                    side: Tokens.metrics.icon_large / 1.5
                }
                Text {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Tokens.colors.text
                    font.family: Tokens.metrics.family_heading
                    font.pixelSize: Tokens.metrics.title_text
                    text: dialog.shown ? dialog.shown.title : ""
                    Accessible.role: Accessible.Heading
                }
            }
            Text {
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                text: dialog.shown ? dialog.shown.sentence : ""
            }
            Repeater {
                model: dialog.shown ? dialog.shown.facts : []

                delegate: RowLayout {
                    required property var modelData

                    Layout.fillWidth: true
                    Layout.leftMargin: Tokens.metrics.space_lg
                    Layout.rightMargin: Tokens.metrics.space_lg
                    spacing: Tokens.metrics.space_sm

                    Text {
                        Layout.preferredWidth: Tokens.metrics.fact_label
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_body
                        font.pixelSize: Tokens.metrics.small_text
                        text: modelData[0]
                    }
                    Text {
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                        color: Tokens.colors.text
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.data_text
                        text: modelData[1]
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                Layout.preferredHeight: Math.min(names.contentHeight, Tokens.metrics.menu_visible / 2)
                visible: dialog.shown !== null && dialog.shown.names.length > 0
                radius: Tokens.metrics.radius
                color: "transparent"
                border.color: Tokens.colors.border

                ListView {
                    id: names

                    anchors.fill: parent
                    clip: true
                    model: dialog.shown ? dialog.shown.names : []
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: RowLayout {
                        required property var modelData
                        required property int index

                        width: ListView.view.width
                        height: Tokens.metrics.control
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            Layout.leftMargin: Tokens.metrics.space_md
                            visible: modelData.icon.length > 0
                            name: modelData.icon
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.leftMargin: modelData.icon.length > 0 ? 0 : Tokens.metrics.space_md
                            elide: Text.ElideMiddle
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.data_text
                            text: modelData.name
                        }
                        Text {
                            Layout.rightMargin: Tokens.metrics.space_md
                            visible: modelData.detail.length > 0
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: modelData.detail
                        }
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                visible: text.length > 0
                wrapMode: Text.Wrap
                color: Tokens.colors.text_muted
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.small_text
                text: dialog.shown ? dialog.shown.note : ""
            }
            TextField {
                id: password

                Layout.fillWidth: true
                Layout.leftMargin: Tokens.metrics.space_lg
                Layout.rightMargin: Tokens.metrics.space_lg
                visible: dialog.shown !== null && dialog.shown.password
                implicitHeight: Tokens.metrics.button
                echoMode: TextInput.Password
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_body
                font.pixelSize: Tokens.metrics.body_text
                Accessible.name: qsTr("Archive password")
                background: Rectangle {
                    radius: Tokens.metrics.radius
                    color: Tokens.colors.content
                    border.width: password.activeFocus ? 2 : 1
                    border.color: password.activeFocus ? Tokens.colors.accent : Tokens.colors.border_strong
                }
                Keys.onReturnPressed: dialog.answer(3)
                Keys.onEnterPressed: dialog.answer(3)
                Keys.onEscapePressed: dialog.answer(0)
            }
            CheckBox {
                id: all

                Layout.leftMargin: Tokens.metrics.space_lg
                visible: dialog.shown !== null && dialog.shown.applyToAll
                checked: dialog.applyToAll
                text: qsTr("Do the same for all other conflicts (A)")
                onToggled: dialog.applyToAll = checked
                contentItem: Text {
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

                RowLayout {
                    id: buttons

                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.space_md
                    spacing: Tokens.metrics.space_sm

                    Item {
                        Layout.fillWidth: true
                    }
                    Repeater {
                        model: dialog.shown ? dialog.shown.buttons : []

                        delegate: OutlinedButton {
                            required property var modelData

                            text: modelData.label
                            iconName: modelData.icon
                            enabled: modelData.enabled
                            primary: modelData.style === "primary"
                            destructive: modelData.style === "destructive"
                            onClicked: dialog.answer(modelData.answer)
                        }
                    }
                }
            }
        }
    }
}
