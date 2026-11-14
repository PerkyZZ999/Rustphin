# Keyboard checklist (R-004, R-107)

The keyboard-only checklist R-004 asks for. Each row compares Dolphin's
default shortcut, read from its source (`dolphinmainwindow.cpp`,
`views/dolphinviewactionhandler.cpp`, KDE standard shortcuts), with
Rustphin's binding. Rustphin follows Dolphin wherever it has the same
action. "Replayed" names the scenario in UI_SCENARIOS.md that exercised it
by keyboard in an isolated session; "—" means bound but not replayed yet.

Last reviewed: 2026-11-08 (W-076o to W-076r), Dolphin source at `master`;
Rustphin's column checked against `commands.rs` on 2026-11-14.
Since W-123 (2026-11-14) every key Rustphin binds is written once, in
`crates/rustphin-view/src/commands.rs`; the first key of a command is the
one menus, tooltips and the palette show, and Settings, Shortcuts lists
them all. The test `keyboard_md_lists_the_table_s_keys` fails when a row
below and the table disagree (the selection-movement, type-to-jump and
context-menu rows and the places panel's prose are not checked).

Each key is caught by one part of the window (`Catcher` in `commands.rs`).
Window shortcuts work wherever the focus is. The file commands caught by
the list (copy, cut, paste, copy location, duplicate, move to trash, delete,
properties, copy or move to the other view) work only while the file list
has the keyboard and no text field does. Keys that also edit text (arrows,
Enter, Backspace, Home, End, Tab, Escape), and F2, F10, Ctrl+Shift+N,
Ctrl+A, Ctrl+Shift+A and /, belong to the pane's or the places panel's own
key handler, so they never fire while a name is being typed.

## Navigation

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| Back | Alt+Left, Backspace, Back key | Alt+Left, Backspace, Back key | K9 (Back key not replayed) |
| Forward | Alt+Right, Forward key | Alt+Right, Forward key | — |
| Up one folder | Alt+Up | Alt+Up | K9 |
| Home folder | Alt+Home | Alt+Home | K9 |
| Reload | F5, Refresh key | F5, Refresh key | — |
| Edit location | Ctrl+L, Alt+D, F6 (selects the location; again returns to the view) | same (also from the places panel) | K8, K9, K19 |
| Open selected | Enter | Enter | N2 |
| Move selection, first/last, page | arrows, Home/End, Page Up/Down | same, with Shift to extend | S6, F12, L1 |
| Type to jump to a name | letters | letters | K10 |

## Selection and view

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| Select all | Ctrl+A | Ctrl+A | S7 |
| Invert selection | Ctrl+Shift+A | Ctrl+Shift+A | S3 |
| Clear selection | Escape | Escape (closes the filter first) | — |
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
| Open terminal here | Shift+F4 | Shift+F4 | NT6 |
| New file | (menu) | Ctrl+Alt+N | — |
| Rename | F2 | F2 (several files: batch rename) | B1 |
| Move to trash | Delete | Delete | F3 |
| Delete permanently | Shift+Delete | Shift+Delete (confirmed) | Q8 |
| Cut / Copy / Paste | Ctrl+X / Ctrl+C / Ctrl+V | same | C1 |
| Undo | Ctrl+Z | Ctrl+Z | — |
| Redo | Ctrl+Shift+Z | not offered (no redo) | — |
| Properties | Alt+Enter | Alt+Enter | I5 |
| Duplicate here | Ctrl+D | Ctrl+D, context menu, palette: "name copy.ext", then "name copy (1).ext"; the copies are selected (D-021) | K18 |
| Copy location | Ctrl+Alt+C | Ctrl+Alt+C, context menu (remote folders give their address) | K12 |

## Tabs, split view and panels

| Action | Dolphin | Rustphin | Replayed |
|---|---|---|---|
| New tab | Ctrl+T | Ctrl+T | K9 |
| Close tab | Ctrl+W | Ctrl+W | T4 |
| Next / previous tab | Ctrl+Tab, Ctrl+Page Down / Ctrl+Shift+Tab, Ctrl+Page Up | same | T2, J8 |
| Activate tab 1–9 / last | Alt+1…Alt+9 / Alt+0 | Alt+1, Alt+2, Alt+3, Alt+4, Alt+5, Alt+6, Alt+7, Alt+8, Alt+9 / Alt+0 | K9 |
| Move tab left / right | (none) | Ctrl+Shift+Page Up / Ctrl+Shift+Page Down | — |
| Reopen closed tab | Ctrl+Shift+T (last 6) | Ctrl+Shift+T (last 6, split panes included) | K11 |
| Split view | F3 | F3 | D1, K9, K10 |
| Focus other view | Ctrl+F3 | Ctrl+F3 (toggles the two views), Tab (also visits the places panel) | Tab: K10. Ctrl+F3 is taken by KWin's default desktop switch, so it never reaches Dolphin or Rustphin on a default Plasma desktop |
| Copy / move to other view | Shift+F5 / Shift+F6 | Shift+F5 / Shift+F6 | K10 |
| Places panel shown | F9 | F9 | — |
| Add folder to Places | (none) | Ctrl+Shift+D (Ctrl+D until D-021), button, drag | K2, K18 |
| Focus places panel | Ctrl+P | Ctrl+P | K9 |
| Quit | Ctrl+Q | Ctrl+Q: ends Rustphin and closes all its windows, also when it keeps running in the background (D-031) | K9, BG5 |
| New window | Ctrl+N | not offered (one window) | — |
| Command palette | KDE command bar (from KXmlGui, not Dolphin source) | Ctrl+Shift+P | Q1 |
| Settings | Ctrl+Shift+, (Configure Dolphin) | Ctrl+Shift+, | ST1 |

## Places panel (focused)

Up/Down move, Enter opens (or mounts a device), F2 renames, Delete removes,
H hides or shows, Alt+Up/Down reorder, Left/Right fold and unfold a section,
G moves the place to the next group, E unmounts or safely removes a device,
the Menu key or Shift+F10 opens the places menu, Tab moves on to the view,
Escape returns to the view. H, G and E act only without a modifier, and all
these keys apply only while the panel has focus and no text field is being
edited (K7, K8). The panel takes Alt+Up and Alt+Down ahead of the window's
Alt+Up (up one folder) while it has the focus (W-123). Ctrl+Shift+G (new
group) is a window shortcut: it works from anywhere and takes the keyboard
to the panel to name the group. "List or hide hidden places" has no key; it
is in the panel's menu and the palette.

## Not offered yet

- Redo (Ctrl+Shift+Z): an undone operation cannot be redone. A candidate
  for later work; not part of R-001–R-016 as written.
- Dolphin's search (Ctrl+F): search is L-001.
- The columns view (Ctrl+4) and a new window (Ctrl+N): Rustphin has no
  columns view, and new windows open only for launches and other
  applications (Settings, General).
- Changing keys: Settings, Shortcuts lists them read-only.
- Commands with no key, reached through menus, the toolbar or the palette:
  folders first, sorting by a column, Extract and Compress, Recover original
  names, the trash's actions (open, restore, empty), stopping operations and
  dismissing the notice.
