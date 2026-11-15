// A drop-down of Settings (DESIGN.md, Settings, W-155): the chosen words
// and a chevron in a 216 px field; its list is a solid surface with a check
// mark before the chosen option.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import Rustphin

ComboBox {
    id: combo

    implicitWidth: Tokens.metrics.setting_select
    implicitHeight: Tokens.metrics.setting_control
    font.family: Tokens.metrics.family_body
    font.pixelSize: Tokens.metrics.body_text

    background: Rectangle {
        radius: Tokens.metrics.radius
        color: combo.hovered ? Tokens.colors.hover : Tokens.colors.content
        border.width: combo.visualFocus ? 2 : 1
        border.color: combo.visualFocus || combo.popup.visible ? Tokens.colors.accent : Tokens.colors.border_strong
    }
    contentItem: Text {
        textFormat: Text.PlainText
        leftPadding: Tokens.metrics.space_sm
        rightPadding: Tokens.metrics.icon_ui + Tokens.metrics.space_sm
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        color: Tokens.colors.text
        font: combo.font
        text: combo.displayText
    }
    indicator: Lucide {
        x: combo.width - width - Tokens.metrics.space_sm
        y: (combo.height - height) / 2
        name: "chevron-down"
        colour: Tokens.colors.text_muted
    }
    delegate: ItemDelegate {
        id: option

        required property int index
        required property string modelData

        width: ListView.view.width
        implicitHeight: Tokens.metrics.setting_row
        leftPadding: Tokens.metrics.space_sm
        topPadding: 0
        bottomPadding: 0
        highlighted: combo.highlightedIndex === index
        Accessible.name: modelData
        Accessible.selected: combo.currentIndex === index

        contentItem: Row {
            spacing: Tokens.metrics.space_sm

            Lucide {
                anchors.verticalCenter: parent.verticalCenter
                name: "check"
                side: Tokens.metrics.icon_small
                colour: Tokens.colors.accent
                opacity: option.index === combo.currentIndex ? 1 : 0
            }
            Text {
                textFormat: Text.PlainText
                anchors.verticalCenter: parent.verticalCenter
                color: Tokens.colors.text
                font: combo.font
                text: option.modelData
            }
        }
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: option.highlighted || option.hovered ? Tokens.colors.hover : "transparent"
        }
    }
    popup: Popup {
        y: combo.height + Tokens.metrics.space_xs
        width: Math.max(combo.width, list.contentItem.childrenRect.width + 2 * Tokens.metrics.space_xs)
        padding: Tokens.metrics.space_xs
        implicitHeight: Math.min(list.contentHeight, 8 * Tokens.metrics.setting_row) + 2 * padding

        contentItem: ListView {
            id: list

            clip: true
            implicitHeight: contentHeight
            model: combo.popup.visible ? combo.delegateModel : null
            currentIndex: combo.highlightedIndex
            ScrollBar.vertical: Scroller {}
        }
        background: Rectangle {
            radius: Tokens.metrics.radius
            color: Tokens.colors.raised
            border.color: Tokens.colors.border_strong
        }
    }
}
