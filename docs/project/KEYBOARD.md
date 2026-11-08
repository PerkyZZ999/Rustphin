# Keyboard checklist (R-004, R-107)

The keyboard-only checklist R-004 asks for. Each row compares Dolphin's
default shortcut, read from its source (`dolphinmainwindow.cpp`,
`views/dolphinviewactionhandler.cpp`, KDE standard shortcuts), with
Rustphin's binding. Rustphin follows Dolphin wherever it has the same
action. "Replayed" names the scenario in UI_SCENARIOS.md that exercised it
by keyboard in an isolated session; "—" means bound but not replayed yet.

Last reviewed: 2026-11-08 (W-076o to W-076r), Dolphin source at `master`.

## Navigation

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| Back | Backspace, Alt+Left, Back key | Backspace, Alt+Left, Back key | K9 (Back key not replayed) |
| Forward | Alt+Right, Forward key | Alt+Right, Forward key | — |
| Up one folder | Alt+Up | Alt+Up | K9 |
| Home folder | Alt+Home | Alt+Home | K9 |
| Reload | F5 | F5 | — |
| Edit location | Ctrl+L, Alt+D, F6 | Ctrl+L, Alt+D, F6 (also from the places panel) | K8, K9 |
| Open selected | Enter | Enter | N2 |
| Move selection, first/last, page | arrows, Home/End, Page Up/Down | same, with Shift to extend | S6, F12, L1 |
| Type to jump to a name | letters | letters | K10 |

## Selection and view

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| Select all | Ctrl+A | Ctrl+A | S7 |
| Invert selection | Ctrl+Shift+A | Ctrl+Shift+A | S3 |
| Icons / Compact / Details | Ctrl+1 / Ctrl+2 / Ctrl+3 | same | V1 |
| Columns view | Ctrl+4 | not offered (no columns view) | — |
| Zoom in / out / reset | Ctrl++ / Ctrl+- / Ctrl+0 | same (Ctrl+= too) | V3 |
| Show hidden files | Ctrl+H, Alt+. | Ctrl+H, Alt+. | — |
| Filter by name | Ctrl+I, / | Ctrl+I (toggles), / | K17 |
| Context menu | Menu key, Shift+F10 | Menu key, Shift+F10 | J16, K12 |

## Files

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| New folder | F10 | F10, Ctrl+Shift+N | F1 |
| New file | (menu) | Ctrl+Alt+N | — |
| Rename | F2 | F2 (several files: batch rename) | B1 |
| Move to trash | Delete | Delete | F3 |
| Delete permanently | Shift+Delete | Shift+Delete (confirmed) | Q8 |
| Cut / Copy / Paste | Ctrl+X / Ctrl+C / Ctrl+V | same | C1 |
| Undo | Ctrl+Z | Ctrl+Z | — |
| Redo | Ctrl+Shift+Z | not offered (no redo) | — |
| Properties | Alt+Enter | Alt+Enter | I5 |
| Duplicate here | Ctrl+D | not offered; Ctrl+D adds the folder to Places instead (Dolphin has no default for that) | — |
| Copy location | Ctrl+Alt+C | Ctrl+Alt+C, context menu (remote folders give their address) | K12 |

## Tabs, split view and panels

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| New tab | Ctrl+T | Ctrl+T | K9 |
| Close tab | Ctrl+W | Ctrl+W | T4 |
| Next / previous tab | Ctrl+Tab, Ctrl+Page Down / Ctrl+Shift+Tab, Ctrl+Page Up | same | T2, J8 |
| Activate tab 1–9 / last | Alt+1…Alt+9 / Alt+0 | same | K9 |
| Move tab left / right | (none) | Ctrl+Shift+Page Up / Down | — |
| Reopen closed tab | Ctrl+Shift+T (last 6) | Ctrl+Shift+T (last 6, split panes included) | K11 |
| Split view | F3 | F3 | D1, K9, K10 |
| Focus other view | Ctrl+F3 | Ctrl+F3 (toggles the two views), Tab (also visits the places panel) | Tab: K10. Ctrl+F3 is taken by KWin's default desktop switch, so it never reaches Dolphin or Rustphin on a default Plasma desktop |
| Copy / move to other view | Shift+F5 / Shift+F6 | Shift+F5 / Shift+F6 | K10 |
| Places panel shown | F9 | F9 | — |
| Focus places panel | Ctrl+P | Ctrl+P | K9 |
| Quit | Ctrl+Q | Ctrl+Q (same as closing the window) | K9 |
| New window | Ctrl+N | not offered (one window) | — |
| Command palette | KDE command bar (from KXmlGui, not Dolphin source) | Ctrl+Shift+P | Q1 |

## Places panel (focused)

Up/Down move, Enter opens, F2 renames, Delete removes, H hides, Alt+Up/Down
reorder, Left/Right fold groups, G next group, Ctrl+Shift+G new group,
E ejects a device. These single-letter keys apply only while the panel has
focus and no text field is being edited (K7, K8).

## Not offered yet

Duplicate (Dolphin's Ctrl+D; Rustphin uses Ctrl+D for Add to Places, which
Dolphin leaves unbound), redo and Dolphin's search (Ctrl+F) have no Rustphin
equivalent. Search is L-001; the others are
candidates for later work and are not part of R-001–R-016 as written.
