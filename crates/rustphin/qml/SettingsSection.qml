// A section of a Settings page (DESIGN.md, Settings, W-155): its title in
// `body-md-strong`, an optional line under it and an optional action on the
// right, then its rows in one `panel` with hairlines between them.
import QtQuick
import QtQuick.Layouts
import Rustphin

ColumnLayout {
    id: section

    property string title: ""
    property string note: ""
    // The rows, put in the panel.
    default property alias rows: panelColumn.data
    // Whether the rows sit in a panel, or straight on the page.
    property bool panelled: true

    Layout.fillWidth: true
    spacing: Tokens.metrics.space_sm

    ColumnLayout {
        Layout.fillWidth: true
        visible: section.title.length > 0 || section.note.length > 0
        spacing: 0

        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            visible: section.title.length > 0
            color: Tokens.colors.text
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            font.weight: Font.DemiBold
            text: section.title
            Accessible.role: Accessible.Heading
        }
        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            visible: section.note.length > 0
            wrapMode: Text.Wrap
            color: Tokens.colors.text_muted
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.small_text
            text: section.note
        }
    }
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: panelColumn.implicitHeight
        radius: Tokens.metrics.radius
        color: section.panelled ? Tokens.colors.panel : "transparent"
        border.color: section.panelled ? Tokens.colors.border : "transparent"

        ColumnLayout {
            id: panelColumn

            anchors.left: parent.left
            anchors.right: parent.right
            spacing: 0
        }
    }
}
