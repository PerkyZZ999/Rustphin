// A drive's Properties (W-153): its name and filesystem, a ring of used and
// free space with its legend, then the facts the device service told.
// Everything shown comes decided from Rust (`rustphin_view::drive_properties`).
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Shapes
import Rustphin

Popup {
    id: dialog

    // The sheet as JSON (`DriveSheet`), read into typed parts.
    required property string json
    property string title: ""
    property string icon: "hard-drive"
    property string kind: ""
    property string note: ""
    property list<var> facts: []
    property bool hasUsage: false
    property real fraction: 0
    property string percent: ""
    property string used: ""
    property string free: ""
    property string total: ""
    property bool nearlyFull: false
    property string description: ""
    readonly property color usedColour: nearlyFull ? Tokens.colors.danger : Tokens.colors.activity

    function read() {
        const sheet = JSON.parse(json)
        const usage = sheet.usage ?? null
        title = sheet.title
        icon = sheet.icon
        kind = sheet.kind
        note = sheet.note
        facts = sheet.facts
        hasUsage = usage !== null
        fraction = usage ? usage.fraction : 0
        percent = usage ? usage.percent : ""
        used = usage ? usage.used : ""
        free = usage ? usage.free : ""
        total = usage ? usage.total : ""
        nearlyFull = usage ? usage.nearly_full : false
        description = usage ? usage.description : ""
    }

    onJsonChanged: read()

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Tokens.metrics.dialog, parent.width - 2 * Tokens.metrics.space_lg)
    height: Math.min(implicitHeight, parent.height - 2 * Tokens.metrics.space_lg)
    modal: true
    focus: true
    padding: 0
    Overlay.modal: Rectangle {
        color: Tokens.colors.scrim
    }
    background: FloatingSurface {}
    Component.onCompleted: {
        read()
        open()
    }

    contentItem: ColumnLayout {
        focus: true
        spacing: 0
        Accessible.role: Accessible.Dialog
        Accessible.name: qsTr("Properties: %1").arg(dialog.title)
        Accessible.description: dialog.hasUsage ? dialog.description : dialog.note
        Keys.onReturnPressed: dialog.close()
        Keys.onEnterPressed: dialog.close()

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.dialog_title
            Layout.leftMargin: Tokens.metrics.space_lg
            Layout.rightMargin: Tokens.metrics.space_xs

            Lucide {
                name: dialog.icon
                colour: Tokens.colors.text
            }
            Text {
                textFormat: Text.PlainText
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: Tokens.colors.text
                font.family: Tokens.metrics.family_heading
                font.pixelSize: Tokens.metrics.title_text
                text: qsTr("%1 — Properties").arg(dialog.title)
                Accessible.role: Accessible.Heading
            }
            IconButton {
                iconName: "x"
                text: qsTr("Close")
                tip: qsTr("Close (Esc)")
                onClicked: dialog.close()
            }
        }
        Flickable {
            id: factsView

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: body.implicitHeight + 2 * Tokens.metrics.space_lg
            contentHeight: body.implicitHeight + 2 * Tokens.metrics.space_lg
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: Scroller {}

            ColumnLayout {
                id: body

                x: Tokens.metrics.space_lg
                y: Tokens.metrics.space_lg
                width: parent.width - 2 * Tokens.metrics.space_lg
                spacing: Tokens.metrics.space_md

                RowLayout {
                    spacing: Tokens.metrics.space_md

                    Lucide {
                        name: dialog.icon
                        side: Tokens.metrics.icon_header
                    }
                    ColumnLayout {
                        spacing: 0

                        Text {
                            textFormat: Text.PlainText
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            font.weight: Font.DemiBold
                            text: dialog.title
                        }
                        Text {
                            textFormat: Text.PlainText
                            visible: text.length > 0
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.small_text
                            text: dialog.kind
                        }
                    }
                }
                // The ring: used space as an arc over the free track, the
                // percentage in its middle; the legend says it in words, so
                // nothing rests on colour alone.
                RowLayout {
                    visible: dialog.hasUsage
                    Layout.fillWidth: true
                    spacing: Tokens.metrics.space_lg

                    Item {
                        id: ring

                        readonly property real side: Tokens.metrics.drive_ring
                        readonly property real stroke: Tokens.metrics.drive_ring_stroke
                        readonly property real radius: (side - stroke) / 2

                        implicitWidth: side
                        implicitHeight: side
                        Accessible.role: Accessible.Graphic
                        Accessible.name: dialog.description

                        Shape {
                            anchors.fill: parent
                            preferredRendererType: Shape.CurveRenderer

                            ShapePath {
                                strokeColor: Tokens.colors.track
                                strokeWidth: ring.stroke
                                fillColor: "transparent"
                                capStyle: ShapePath.FlatCap

                                PathAngleArc {
                                    centerX: ring.side / 2
                                    centerY: ring.side / 2
                                    radiusX: ring.radius
                                    radiusY: ring.radius
                                    startAngle: -90
                                    sweepAngle: 360
                                }
                            }
                            ShapePath {
                                strokeColor: dialog.usedColour
                                strokeWidth: ring.stroke
                                fillColor: "transparent"
                                capStyle: ShapePath.FlatCap

                                PathAngleArc {
                                    centerX: ring.side / 2
                                    centerY: ring.side / 2
                                    radiusX: ring.radius
                                    radiusY: ring.radius
                                    startAngle: -90
                                    sweepAngle: 360 * Math.max(0, Math.min(1, dialog.fraction))
                                }
                            }
                        }
                        Column {
                            anchors.centerIn: parent

                            Text {
                                textFormat: Text.PlainText
                                anchors.horizontalCenter: parent.horizontalCenter
                                color: Tokens.colors.text
                                font.family: Tokens.metrics.family_heading
                                font.pixelSize: Tokens.metrics.title_text
                                text: dialog.percent
                            }
                            Text {
                                textFormat: Text.PlainText
                                anchors.horizontalCenter: parent.horizontalCenter
                                color: Tokens.colors.text_muted
                                font.family: Tokens.metrics.family_body
                                font.pixelSize: Tokens.metrics.small_text
                                text: qsTr("used")
                            }
                        }
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 3
                        columnSpacing: Tokens.metrics.space_sm
                        rowSpacing: Tokens.metrics.space_sm

                        Repeater {
                            model: dialog.hasUsage ? [[qsTr("Used"), dialog.used, dialog.usedColour], [qsTr("Free"), dialog.free, Tokens.colors.track], [qsTr("Total"), dialog.total, "transparent"]] : []

                            delegate: Item {
                                id: legend

                                required property list<var> modelData

                                Layout.columnSpan: 3
                                Layout.fillWidth: true
                                implicitHeight: legendRow.implicitHeight
                                Accessible.role: Accessible.StaticText
                                Accessible.name: modelData[0] + ": " + modelData[1]

                                RowLayout {
                                    id: legendRow

                                    width: parent.width
                                    spacing: Tokens.metrics.space_sm

                                    Rectangle {
                                        Layout.alignment: Qt.AlignTop
                                        Layout.topMargin: 2
                                        implicitWidth: Tokens.metrics.swatch
                                        implicitHeight: Tokens.metrics.swatch
                                        radius: Tokens.metrics.radius
                                        color: legend.modelData[2]
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 0

                                        Text {
                                            textFormat: Text.PlainText
                                            color: Tokens.colors.text_muted
                                            font.family: Tokens.metrics.family_body
                                            font.pixelSize: Tokens.metrics.small_text
                                            text: legend.modelData[0]
                                        }
                                        Text {
                                            textFormat: Text.PlainText
                                            Layout.fillWidth: true
                                            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                                            color: Tokens.colors.text
                                            font.family: Tokens.metrics.family_data
                                            font.pixelSize: Tokens.metrics.data_text
                                            text: legend.modelData[1]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Text {
                    textFormat: Text.PlainText
                    visible: text.length > 0
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    lineHeight: Tokens.metrics.text_line
                    lineHeightMode: Text.FixedHeight
                    color: Tokens.colors.text_muted
                    font.family: Tokens.metrics.family_body
                    font.pixelSize: Tokens.metrics.body_text
                    text: dialog.note
                    Accessible.role: Accessible.StaticText
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Tokens.metrics.hairline
                    color: Tokens.colors.border
                }
                Repeater {
                    model: dialog.facts

                    delegate: RowLayout {
                        id: fact

                        required property list<string> modelData

                        Layout.fillWidth: true
                        spacing: Tokens.metrics.space_sm
                        Accessible.role: Accessible.StaticText
                        Accessible.name: modelData[0] + ": " + modelData[1]

                        Text {
                            textFormat: Text.PlainText
                            Layout.preferredWidth: Tokens.metrics.properties_label
                            Layout.alignment: Qt.AlignTop
                            color: Tokens.colors.text_muted
                            font.family: Tokens.metrics.family_body
                            font.pixelSize: Tokens.metrics.body_text
                            text: fact.modelData[0]
                        }
                        Text {
                            textFormat: Text.PlainText
                            Layout.fillWidth: true
                            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                            color: Tokens.colors.text
                            font.family: Tokens.metrics.family_data
                            font.pixelSize: Tokens.metrics.data_text
                            text: fact.modelData[1]
                        }
                    }
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Tokens.metrics.control_line
            color: Tokens.colors.chrome

            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Tokens.metrics.space_md
                spacing: Tokens.metrics.space_sm

                Item {
                    Layout.fillWidth: true
                }
                OutlinedButton {
                    text: qsTr("Close (Esc)")
                    primary: true
                    onClicked: dialog.close()
                }
            }
        }
    }
}
