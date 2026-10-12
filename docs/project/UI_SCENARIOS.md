# UI scenarios

Replayable checks of the running application, driven by an agent through the kwin-mcp tools. Run them after any change to `rustphin-ui`, and record the result in VALIDATION.md.

## Setup

1. Build: `cargo build --package rustphin`.
2. Create the fixture in a new scratch directory (never in the user's files): `tests/ui/make-fixture.sh <dir>`.
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
| L1 | Launch the release build on a 500,000-entry folder (`rust-tc bench-listing` creates `target/bench/listing-500k`); press End | Status "500,000 items"; the last entry is selected and visible within a second |

## Not covered by these scenarios

- Blur and the user's own icon theme: an isolated session cannot show them. They need the live desktop.
- Narrow-window column dropping, frame rate, and anything that needs a second application.
