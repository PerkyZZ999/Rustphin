// A checkbox in the design's colours (DESIGN.md, Checkboxes): Qt Quick's
// Basic indicator, which otherwise takes the desktop's colour scheme, with
// a 2 px `primary-bright` ring on keyboard focus. Each use gives its words.
import QtQuick
import QtQuick.Controls.Basic
import Rustphin

CheckBox {
    palette.base: Tokens.colors.content
    palette.text: Tokens.colors.text
    palette.windowText: Tokens.colors.text
    palette.mid: Tokens.colors.border_strong
    palette.dark: Tokens.colors.border
    palette.light: Tokens.colors.hover
    palette.highlight: Tokens.colors.accent
}
