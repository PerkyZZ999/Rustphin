# UI scenarios

Replayable checks of the running application, driven by an agent through the kwin-mcp tools. Run them after any change to `rustphin-ui`, and record the result in VALIDATION.md.

## Setup

1. Build: `cargo build --package rustphin`.
2. Create the fixture in a new scratch directory (never in the user's files): `tests/ui/make-fixture.sh <dir>`. Scenarios U1 and U2 change it, so use a fresh one afterwards.
3. Start an isolated session: `session_start` with `app_command` = `target/debug/rustphin <dir>`, `isolate_home: true`, `screen_width: 1280`, `screen_height: 800`.
4. Wait about three seconds for the window before sending input.

Notes for the agent:

- The application is not visible to `list_windows` or `find_ui_elements` (nothing is exposed over AT-SPI in these sessions), so observe with screenshots and act by keyboard or coordinates.
- With the 1280×800 screen the window is centred: toolbar at y≈115, column headings at y≈151, first row at y≈184 and 28 px per row at the default zoom; the view buttons are at x≈1015, 1043 and 1071 and the hidden-files toggle at x≈1099. Re-read a screenshot if the layout has changed.
- Spell punctuation keys literally: `ctrl+=`, `ctrl+-`.
- The isolated session has no blur, so the opaque theme is shown. It uses the default `breeze-dark` icons.
- Stop the session when done. Before deleting the fixture run `chmod 755 <dir>/locked`.

## Expected fixture listing

Details view, name order, hidden files off: folders Documents, empty, locked, only-hidden, Pictures, Projects; then backup.tar.gz, the bad name (shown as `bad�␊name.txt`), file2.txt, file10.txt, main.rs, Makefile, mystery, notes.txt, photo.JPG, report.pdf, video.mp4. Status line: "17 items, 1 hidden".

## Scenarios

| # | Steps | Expected |
|---|---|---|
| N1 | Launch | Listing as above; `file2.txt` before `file10.txt`; sizes `42 B`, `1.5 KiB`, `120.6 KiB`, `5.0 MiB`; folders show no size; a type name and icon on every row |
| N2 | Press Down, Down, Enter | Enters "empty": "This folder is empty" |
| N3 | Alt+Up | Back in the fixture with "empty" selected |
| N4 | Double-click "locked" | Inline error "you do not have permission…"; listing unchanged; location bar still names the fixture |
| N5 | Ctrl+L, Ctrl+A, type `/no/such/folderX`, Backspace, Enter | The field held `/no/such/folder` (Backspace edited text); inline error "does not exist"; listing and location unchanged |
| N6 | Enter "only-hidden" | "This folder has only hidden items", "1 hidden item is not shown.", a "Show hidden files" button |
| N7 | Ctrl+H in the fixture | `.hidden` appears; status "18 items"; press again to hide |
| S1 | Click row 1, Ctrl+click rows 4 and 7 | Three separate rows selected; status "3 of 17 items selected" |
| S2 | Click row 1, Shift+click row 4 | Rows 1–4 selected; "4 of 17 items selected" |
| S3 | Ctrl+Shift+A | The other 13 selected; the focused, unselected row keeps an accent bar |
| S4 | Escape | Nothing selected; status "17 items, 1 hidden" |
| S5 | Click the last row, type `p` | Selection wraps to "Pictures" |
| S6 | Shift+Down twice | Three consecutive rows selected |
| S8 | Press the mouse button on the fifth row, move down five rows, release (use `mouse_button_down`, `mouse_move`, `mouse_button_up`) | An outlined band while dragging; the six rows it crossed stay selected after release |
| S7 | Ctrl+A | All selected: "17 of 17 items selected" |
| O1 | Click the Size heading twice | Largest first (video.mp4, report.pdf, …), folders still on top, a down arrow on Size |
| O2 | Click the Modified heading | Oldest first among files: photo.JPG (2023), notes.txt (2024), video.mp4 (2025) |
| O3 | Click the Type heading | Files without an extension first (Makefile, mystery), then by extension: gz, JPG, mp4, pdf, rs, txt |
| V1 | Ctrl+1 (or the first view button) | Grid of large icons with names underneath; Down then Right moves to the second cell of the first row |
| V2 | Ctrl+2 | Compact grid: small icon beside each name, several per row |
| V3 | Ctrl+= then Ctrl+- twice, then Ctrl+0 | Icons grow, shrink below the default, and return to the default size |
| V4 | Ctrl+3 (or the third view button) | Details view again, with sizes and dates |
| P1 | Set icons view, zoom in once, sort by Type, show hidden; read `<isolated home>/.config/rustphin/settings.toml` | `mode = "icons"`, `zoom = 2`, `sort = "type"`, `show_hidden = true` |
| P2 | Kill the app and `launch_app` it again in the same session | Opens in the icons view at that zoom, sorted by type, hidden files shown |
| U1 | Select two files; from a shell create a file, delete one of the selected files and rename another file in the fixture; wait one second | All three changes are shown without a reload; the surviving selected file is still selected; the list has not scrolled |
| U2 | Enter "empty"; from a shell `rmdir` it; wait one second | The fixture is shown with a message that the folder no longer exists; the location bar names the fixture |
| T1 | Middle-click the "Documents" row | A tab strip appears with "ui-fixture" (active, accent underline) and "Documents"; the view still shows the fixture |
| T2 | Ctrl+Tab, then Ctrl+Shift+Tab | The "Documents" tab is shown (one item, window title "Documents — Rustphin"), then the first tab again |
| T3 | On the second tab, Ctrl+Shift+PageUp | "Documents" moves to the first position and stays active |
| T4 | Ctrl+T, then Ctrl+W | A third tab opens on the same folder and is closed again |
| T5 | Ctrl+W until one tab is left | The tab strip disappears; Ctrl+W on the last tab does not close the window |
| T6 | With two tabs, click the other tab; click its × | The clicked tab becomes active; the × closes that tab |
| D1 | F3 | Two panes side by side on the same folder, a thin divider between them, an accent line above the pane that has keyboard focus; narrow panes keep the Name and Size columns |
| D2 | Tab, then Down and Return on a folder | The accent line moves to the other pane; only that pane changes folder |
| D3 | F3 again | The tab shows one pane, the one that had focus |
| R1 | With two tabs, one of them split, read `<isolated home>/.local/state/rustphin/session.toml`; kill the app and `launch_app` it with no argument | The file lists both tabs and the split tab's two panes; the app reopens with the same tabs, split and locations |
| F1 | F10, type `Work`, Return | A folder "New Folder" appears selected with its name in an edit field, all of it selected; after Return it is called "Work" and stays selected |
| F2 | Select a file, F2 | The name becomes an edit field with the part before the extension selected; Escape leaves the name unchanged |
| F3 | Select `video.mp4`, Delete | The file leaves the list; a line says "Moved “video.mp4” to the trash." with an Undo button; `<isolated home>/.local/share/Trash/files/video.mp4` and `info/video.mp4.trashinfo` exist |
| F4 | Ctrl+Z | "Undid: moved “video.mp4” to the trash."; the file is back |
| F5 | Select two files, Shift+Delete | A dialog "Delete 2 items for good?" over a dimmed window, with "Keep (Esc)" and "Delete for good (Enter)"; Escape closes it and both files remain |
| F6 | Select a file, Ctrl+C, enter a folder, Ctrl+V, Ctrl+V | The first paste copies it. The second opens "“name” already exists in “folder”" with both sizes and dates; `K` or Enter keeps both and a "name (1)" appears |
| F7 | Select a file, Ctrl+X, enter a folder, Ctrl+V, then Ctrl+Z | The file moves into the folder, and back |
| F8 | Create a folder of 30,000 empty files from a shell, select it, Ctrl+C, Ctrl+V | A progress line "Copying “many” to “…”" with a moving bar, a count "n of 30,001" and a stop button |
| F9 | After F3, launch `dolphin trash:/` in the same session | Dolphin's trash lists the file with its original path |
| L1 | Launch the release build on a 500,000-entry folder (`rust-tc bench-listing` creates `target/bench/listing-500k`); press End | Status "500,000 items"; the last entry is selected and visible within a second |

## Not covered by these scenarios

- Blur and the user's own icon theme: an isolated session cannot show them. They need the live desktop.
- Narrow-window column dropping, frame rate, and anything that needs a second application.
- Reordering tabs by dragging with the mouse.
- The failure dialog (it needs an entry that fails mid-operation) and the dialogs' buttons by mouse.
