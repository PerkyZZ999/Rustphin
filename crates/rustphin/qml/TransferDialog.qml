// Every running operation on a card of its own (W-149; the activity line's
// Details): what it does, how far it is, how fast, what it is working on
// and its speed over the last minute, each with its own Stop. The figures
// and the wording come from Rust (`rustphin_view::transfer`) as JSON.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Shapes
import Rustphin

Popup {
    id: transferDialog

    // Rust's cards, read while the dialog is open.
    property list<var> cards: []
    // Whether an operation was shown since the dialog opened: it closes
    // itself when the last one ends, as KDE's copy window does.
    property bool seen: false

    function readCards() {
        cards = Operations.transfers.length > 0 ? JSON.parse(Operations.transfers) : []
        if (cards.length > 0)
            seen = true
        else if (seen)
            close()
    }

    Connections {
        target: Operations
        enabled: transferDialog.visible
        function onTransfersChanged() {
            transferDialog.readCards()
        }
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.dialog, parent ? parent.width - 2 * Tokens.metrics.space_lg : Tokens.metrics.dialog)
    modal: true
    focus: true
    padding: Tokens.metrics.space_lg
    onAboutToShow: {
        seen = false
        readCards()
    }
    onOpened: closeButton.forceActiveFocus()
    onClosed: cards = []
    background: FloatingSurface {}
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }

    contentItem: ColumnLayout {
        spacing: Tokens.metrics.space_md
        Accessible.role: Accessible.Dialog
        Accessible.name: heading.text

        Text {
            id: heading

            textFormat: Text.PlainText
            Layout.fillWidth: true
            color: Tokens.colors.text
            font.family: Tokens.metrics.family_heading
            font.pixelSize: Tokens.metrics.title_text
            text: qsTr("Running operations")
        }
        Text {
            textFormat: Text.PlainText
            Layout.fillWidth: true
            visible: transferDialog.cards.length === 0
            wrapMode: Text.Wrap
            color: Tokens.colors.text_muted
            font.family: Tokens.metrics.family_body
            font.pixelSize: Tokens.metrics.body_text
            text: qsTr("No operation is running. A copy, move or other operation that takes a while is shown here.")
        }
        ListView {
            id: cardList

            // What the window leaves for the cards, under the title and
            // over the button.
            readonly property real room: (transferDialog.parent ? transferDialog.parent.height : contentHeight) - Tokens.metrics.dialog_title - Tokens.metrics.button - 6 * Tokens.metrics.space_lg

            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(contentHeight, Math.max(Tokens.metrics.control_line, room))
            visible: count > 0
            clip: true
            spacing: Tokens.metrics.space_sm
            model: transferDialog.cards.length
            delegate: Rectangle {
                id: card

                required property int index
                // Its card's parts; a card that just left has none.
                readonly property string operation: transferDialog.cards[index]?.id ?? ""
                readonly property string label: transferDialog.cards[index]?.label ?? ""
                readonly property string iconName: transferDialog.cards[index]?.icon ?? "loader"
                readonly property real fraction: transferDialog.cards[index]?.fraction ?? -1
                readonly property string done: transferDialog.cards[index]?.done ?? ""
                readonly property string pace: transferDialog.cards[index]?.pace ?? ""
                readonly property string current: transferDialog.cards[index]?.current ?? ""
                readonly property list<real> points: transferDialog.cards[index]?.graph.points ?? []
                readonly property int span: transferDialog.cards[index]?.graph.span ?? 2
                readonly property string peak: transferDialog.cards[index]?.graph.peak ?? ""

                width: ListView.view.width
                implicitHeight: parts.implicitHeight + 2 * Tokens.metrics.space_md
                radius: Tokens.metrics.radius
                color: "transparent"
                border.color: Tokens.colors.border_strong
                Accessible.role: Accessible.Grouping
                Accessible.name: label

                ColumnLayout {
                    id: parts

                    anchors.fill: parent
                    anchors.margins: Tokens.metrics.space_md
                    spacing: Tokens.metrics.space_sm

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Tokens.metrics.space_sm

                        Lucide {
                            name: card.iconName
                            colour: Tokens.colors.text_muted
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: card.label
                        }
                        IconButton {
                            // The operation under the pointer when the press
                            // began: the cards may move up before it ends.
                            property string pressedId: ""

                            iconName: "circle-stop"
                            text: qsTr("Stop")
                            tip: qsTr("Stop this operation")
                            Accessible.name: qsTr("Stop: %1").arg(card.label)
                            onPressed: pressedId = card.operation
                            onClicked: Operations.cancelOperation(pressedId)
                        }
                    }
                    ActivityBar {
                        Layout.fillWidth: true
                        fraction: card.fraction
                        running: transferDialog.visible
                        Accessible.role: Accessible.ProgressBar
                        Accessible.name: card.label
                    }
                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.small_text
                        text: card.done
                    }
                    Text {
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                        color: Tokens.colors.text_muted
                        font.family: Tokens.metrics.family_data
                        font.pixelSize: Tokens.metrics.small_text
                        text: card.current
                    }
                    // The speed now and the time remaining head the graph;
                    // its highest second is named beside them.
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.topMargin: Tokens.metrics.space_xs
                        spacing: Tokens.metrics.space_md

                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: card.pace.length > 0 ? card.pace : qsTr("Measuring the speed…")
                        }
                        Text {
                            textFormat: Text.PlainText
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.small_text
                            text: card.peak
                        }
                    }
                    // The speed in each of the last seconds, newest on the
                    // right, as a part of the highest one.
                    Item {
                        id: graph

                        readonly property real step: width / Math.max(1, card.span - 1)
                        readonly property list<point> line: card.points.map((part, at) => Qt.point(width - (card.points.length - 1 - at) * step, height - 1 - part * (height - 2)))

                        Layout.fillWidth: true
                        Layout.preferredHeight: Tokens.metrics.speed_graph
                        Accessible.role: Accessible.Graphic
                        Accessible.name: qsTr("Speed over the last minute. %1").arg(card.peak)

                        Rectangle {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            height: Tokens.metrics.hairline
                            color: Tokens.colors.track
                        }
                        Shape {
                            anchors.fill: parent
                            visible: graph.line.length > 1
                            preferredRendererType: Shape.CurveRenderer

                            ShapePath {
                                strokeColor: Tokens.colors.activity
                                strokeWidth: 1
                                fillColor: "transparent"
                                joinStyle: ShapePath.RoundJoin

                                PathPolyline {
                                    path: graph.line
                                }
                            }
                        }
                    }
                }
            }
        }
        OutlinedButton {
            id: closeButton

            Layout.alignment: Qt.AlignRight
            text: qsTr("Close")
            onClicked: transferDialog.close()
        }
    }
}
