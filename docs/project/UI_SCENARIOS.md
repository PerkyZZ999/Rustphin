# UI scenarios

Replayable checks of the running application, driven by an agent through the kwin-mcp tools. Run them after any change to `rustphin-ui`, and record the result in VALIDATION.md.

## Setup

1. Build: `cargo build --package rustphin`.
2. Create the fixture in a new scratch directory (never in the user's files): `tests/ui/make-fixture.sh <dir>`. Scenarios U1 and U2 change it, so use a fresh one afterwards.
3. Start an isolated session: `session_start` with `app_command` = `target/debug/rustphin <dir>`, `isolate_home: true`, `screen_width: 1280`, `screen_height: 800`.
4. Wait for a screenshot showing the window before sending input; debug startup can take ten seconds or more.

Notes for the agent:

- The application is not visible to `list_windows` or `find_ui_elements` (nothing is exposed over AT-SPI in these sessions), so observe with screenshots and act by keyboard or coordinates.
- With the 1280×800 screen the window is centred: toolbar at y≈115, column headings at y≈151, first row at y≈184 and 28 px per row at the default zoom; the labelled Actions button is at the right of the location bar. View controls and hidden-files toggling are in its palette or on their shortcuts. Re-read a screenshot if the layout has changed.
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
| S8 | Press the mouse button on the fifth row to the right of its name (in the Size column), move down five rows, release (use `mouse_button_down`, `mouse_move`, `mouse_button_up`) | An outlined band while dragging; the six rows it crossed stay selected after release |
| S7 | Ctrl+A | All selected: "17 of 17 items selected" |
| O1 | Click the Size heading twice | Largest first (video.mp4, report.pdf, …), folders still on top, a down arrow on Size |
| O2 | Click the Modified heading | Oldest first among files: photo.JPG (2023), notes.txt (2024), video.mp4 (2025) |
| O3 | Click the Type heading | Files without an extension first (Makefile, mystery), then by extension: gz, JPG, mp4, pdf, rs, txt |
| V1 | Ctrl+1 | Grid of large icons with names underneath; Down then Right moves to the second cell of the first row |
| V2 | Ctrl+2 | Compact grid: small icon beside each name, several per row |
| V3 | Ctrl+= then Ctrl+- twice, then Ctrl+0 | Icons grow, shrink below the default, and return to the default size |
| V4 | Ctrl+3 | Details view again, with sizes and dates |
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
| T7 | Drag the active second tab onto the first, using several waypoints paused for about 100 ms | The active folder moves to the first position; selected state, visible underline and saved session order agree |
| D1 | F3 | Two panes side by side on the same folder, a thin divider between them, an accent line above the pane that has keyboard focus; narrow panes keep Name; Size appears at 320 px per pane |
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
| F10 | Copy a generated file into a generated mode-0500 folder. Escape; repeat with S; repeat, repair fixture mode to 0700, Enter | Named Permission denied failure; Stop/Skip preserve source and leave no destination, retry publishes exact bytes without a temporary entry |
| F11 | Copy a generated 100,000-file folder from the private home to an empty disposable project-local destination; observe progress, click Stop | Count, bytes, rate and Stop visible; terminal stopped notice, staged destination cleaned, every source byte preserved. Exclude copies completed before the click |
| F12 | In a fresh-history private-home fixture, Home/F2/rename/Enter/Ctrl+Z in icons and compact; Home/F2/type/Escape | Stem edit keeps extension; exact name/bytes restored by undo and preserved by cancellation. Explicitly reselect after undo |
| X1 | After trashing two files (F3), open Actions, search `trash open`, Enter | The window title and tab read "Trash"; a banner "Trash" with Restore (disabled) and "Empty trash…"; the two files are listed |
| X2 | From a shell in the session: `kioclient move <fixture>/notes.txt trash:/`; look at the trash view | notes.txt is listed too, without a reload |
| X3 | Click notes.txt, click Restore | "Restored “notes.txt” from the trash." and the file is back in the fixture |
| X4 | Click "Empty trash…", Return | A dialog "Empty the trash? It holds 2 items."; after Return the list is empty, the line reads "Emptied the trash." with no Undo button, and `Trash/files` and `Trash/info` are empty on disk |
| C1 | Select a file whose name has spaces and non-ASCII characters, Ctrl+X | The row is drawn faded; "Cut “…”. Paste to move." |
| C2 | Start `dolphin <fixture>/Documents` in the session, click its view, Ctrl+V | The file is in Documents and gone from the fixture, name intact |
| C3 | In Dolphin, Ctrl+X on that file; click Rustphin's title bar, Ctrl+V | The file is back in the fixture: "Moved “…” to “ops-fixture”." |
| G1 | Drag a file by its name onto a folder row (waypoints, about 100 ms each) | A chip with the name follows the pointer; the folder row is highlighted; on release a dialog "Move or copy “…” into “…”?" with Cancel, Copy here (C), Move here (Enter); Return moves it |
| G2 | The same drag holding Ctrl | The file is copied without a question; the original stays |
| G3 | F3, then drag a file from the left pane onto a folder in the right pane holding Shift | The file is moved without a question |
| G4 | With Dolphin open on a folder, drag a file from Dolphin onto Rustphin's list | The same move-or-copy dialog; `C` copies it into the folder shown |
| G5 | Tile Rustphin to the left (Super+Left), drag a file by its name into Dolphin's view | Dolphin shows its own Move/Copy/Link menu; "Copy Here" copies the file, name intact |
| G6 | Press in a row's Size column and drag down across rows | A rubber band selects the rows, as before drags existed |
| K1 | In a fresh isolated home, look at the left panel | "Places" without secondary header buttons, and "No places yet. Add the folder shown with Ctrl+D, or drag a folder here." |
| K2 | Ctrl+D; enter a folder; Ctrl+D | Both folders are listed; the one being shown has an accent bar and a highlighted row; `<isolated home>/.local/share/user-places.xbel` holds two `<bookmark>` elements |
| K3 | Tab (focus the panel), Ctrl+Shift+G, type `Work`, Return, Up, G | A heading "Work" with an arrow; the place above it moves under the heading, indented; `<isolated home>/.config/rustphin/place-groups.toml` lists the group with that place's identifier |
| K4 | F2 on a place, type a new name, Return; then start `dolphin /tmp` in the session | Dolphin's Places lists both places, ungrouped, with the new name; within a second Rustphin's panel also lists the "Home" and "Trash" entries Dolphin added to the file |
| K5 | Kill and relaunch Rustphin | The group and its place are back as they were |
| K6 | Drag a place onto a group heading; drag a folder from the file list onto the panel | The place moves into the group; the folder is added to the places |
| K7 | Focus a group heading, Left, then Delete | Left folds the group (arrow points right, a count appears); Delete removes the heading and its places stay, ungrouped |
| V5 | Look at the places panel's "Devices" section. Do not click its rows or buttons: they are the machine's real drives | Each mounted volume has a drive icon, its label ("System" for the root volume), a usage bar and a line like "211 GiB free of 927 GiB"; volumes other than the system and home ones have an eject button |
| H1 | Make a folder with a wide PNG, a tall PNG, a square PNG, a file named `broken.png` that is not an image, a text file and a subfolder; open it; Ctrl+1 | The three pictures show their own content, each keeping its proportions inside the icon's square; `broken.png` keeps the image type icon; the text file and folder keep theirs. `<isolated home>/.cache/thumbnails/normal/` holds three PNGs and `fail/rustphin-<version>/` one |
| L1 | Launch the release build on a 500,000-entry folder (`rust-tc bench-listing` creates `target/bench/listing-500k`); press End | Status "500,000 items"; the last entry is selected and visible within a second |

## Command palette (S8)

Use an isolated fixture and home as above. These checks never activate a device command or touch a real volume.

| # | Steps | Expected |
|---|---|---|
| Q1 | Ctrl+Shift+P; Down, Up, Tab, Shift+Tab; Escape | Solid palette with labelled search, shortcuts and a leading selection marker; keys move the result selection without editing the query; Escape returns to the pane |
| Q2 | Open palette, type `view icons`, Enter; repeat with `view details` | Changes the active pane's view through the existing action; settings file agrees |
| Q3 | Visit Documents, return to the fixture; palette query `recent documents`, Enter | Documents opens; recent-locations.toml contains distinct local file URIs, newest first; failed path requests are absent |
| Q4 | Add Documents as a place, return to the fixture; query `place documents`, click its result | Documents opens in the active pane; the place is also searchable while its group is folded |
| Q5 | Open two tabs on different folders; query `tab documents`, Enter | Switches to the existing Documents tab without adding a tab |
| Q6 | Query `clear recent`, Enter; inspect recent-locations.toml; restart in the same isolated home | Palette stays open with "Recent folders cleared."; saved locations are empty. After restart only newly completed visits can reappear |
| Q7 | Query `zzzxnoaction`; press Ctrl+T and F3; Escape | Helpful no-match text; no tab or split opens behind the palette; Escape returns focus |
| Q8 | Select a fixture file, Shift+Delete; Ctrl+Shift+P; Escape | Delete confirmation stays on top, palette does not open, Escape keeps the file |

## Progressive disclosure (S8)

| # | Steps | Expected |
|---|---|---|
| J1 | Open the fixture in one unsplit tab | Places, navigation/location bar, file view and status line; no secondary view/create/trash buttons; a labelled Actions button is visible |
| J2 | Click Actions, query `new folder`, Enter | Existing operation engine creates a folder and opens its inline rename field; F10 still does the same |
| J3 | Actions, query `new group`, Enter; name it Work | Places panel receives focus, creates the group and opens its name field; group is saved in place-groups.toml |
| J4 | Use Actions for view modes, zoom, hidden files, sorting, split view, new file, trash and place controls | Each control is selected after opening Actions once; corresponding existing shortcuts still work; unavailable future features are absent |
| J5 | Make one pane narrower than 320 px, then narrower than 240 px | Forward and Up leave first; Back leaves under 240 px. Alt+Left, Alt+Right and Alt+Up still navigate. Actions stays. The status line stays inside the pane |
| J6 | Details view, places visible, F3; resize to the 480 px minimum window width, then enlarge through 320 px per pane | Each narrow pane keeps filename text and only Name; Size returns at 320 px with matching header/cells. Arrow selection and F2/Escape still work. Sorting hidden metadata remains reachable through Actions |
| J7 | Open Properties on a generated private-home file; resize to 480 × 320. End, Page Up/Down, Home, arrows; drag the scrollbar; toggle a permission, Escape/reopen; enlarge | Title and Close/Apply stay visible, facts/permissions scroll to both ends, unapplied permission edits are discarded, normal window shows all facts without a scrollbar. Ctrl+T and palette shortcut do not act behind the modal |

## Batch rename (S9)

Use two or more files in the fixture. Do not activate a device.

| # | Steps | Expected |
|---|---|---|
| B1 | Select two files, F2 | A solid dialog titled for both items, with Replace, Pattern and Case. Replace is filled. The preview lists both current names. A sentence says what to type, and Rename is disabled |
| B2 | Find `note`, replace with `other` | The preview shows `note` becoming `other`, a sentence that the name is already used, and Rename still disabled |
| B3 | Change the replacement to `photo`, Enter | `note` is now `photo` on disk and in the list; `other` is unchanged; the dialog is gone |
| B4 | Select the two files again, F2, Escape | The dialog closes and both names stay as they are |

## Desktop integration (S10)

Use a new scratch fixture with folders `Alpha` and `Beta Space`. In Alpha put a
29-byte `note space.txt` (mode 0644), a folder containing one 100-byte file, a
symbolic link to the note, a harmless executable that writes a marker inside the
fixture, and an application `.desktop` shortcut to that executable. The marker
must be absent before testing cancellation.

In the isolated home's data directory, register two test applications named
Fixture Editor and Fixture Viewer for `text/plain`. Each should only record its
arguments in the scratch fixture and exit. Set Fixture Editor as the default in
the isolated home's `mimeapps.list`. Register them before launching Rustphin:
application associations are loaded at startup. Pass `XDG_CONFIG_DIRS=/etc/xdg`
and `XDG_DATA_DIRS=/usr/local/share:/usr/share` to test apps so inherited paths
do not include the user's personal configuration. All desktop-entry installation
and handler changes below are confined to this isolated home.

| # | Steps | Expected |
|---|---|---|
| I1 | Launch with `<fixture>/Alpha file://<fixture>/Beta%20Space` | Two tabs, Alpha and Beta Space, in that order; Alpha lists the fixture |
| I2 | Right-click `note space.txt` | The note is selected; Open, Open with, Cut, Copy, Paste, Rename, Move to trash, Delete and Properties are listed; Paste is disabled without a clipboard payload |
| I3 | From I2, Down, Right; Down, Enter | Open with lists Fixture Editor first and Fixture Viewer second; Viewer receives the exact note path including the space; the menu closes |
| I4 | With the note selected, Enter | The default Fixture Editor receives that same path without a run confirmation |
| I5 | Alt+Enter on the note | Properties shows its name, Plain text document, 29 B, timestamps, symbolic mode and 0644; Apply is disabled |
| I6 | Toggle Group Write, Enter; inspect mode; Ctrl+Z | Before applying, the dialog shows 0664 and enables Apply. Afterwards the mode on disk is 0664 and a completion notice offers undo. Ctrl+Z restores 0644 |
| I7 | Alt+Enter on the folder, then on the link, closing each with Escape | Folder size reaches 100 B; link says Symbolic link and Points to, with permission editing disabled |
| I8 | Enter on the harmless executable; Escape; inspect marker; Enter, inspect prompt, Enter | The first prompt names the program and offers Keep, Open and Run. Cancellation leaves no marker; confirming Run creates it |
| I9 | Enter on the application shortcut; Escape | A confirmation names the application, explains that it starts an application, and offers Keep and Run; Escape returns to the list |
| I10 | Switch to Beta Space. From a separate process launched in the isolated session, call `org.freedesktop.FileManager1.ShowItems` at `/org/freedesktop/FileManager1`, with the note's encoded file URI and an empty startup id | The method returns successfully; an Alpha tab opens and selects `note space.txt` |
| I11 | Call `ShowItemProperties` on that same interface with the note URI | Properties opens for the note with size, times and permissions; this route currently labels the type generically as File |
| I12 | Close Rustphin. Copy `assets/rustphin.desktop` into the isolated data directory's applications folder, refresh its desktop database, and set the isolated `inode/directory` default to `rustphin.desktop`. Launch `xdg-open '<fixture>/Beta Space'` through the session with the release binary directory on PATH | Rustphin starts on Beta Space through the desktop entry; the process arguments contain that folder, with the space intact |
| I13 | In Beta Space, right-click empty space, click New file; type `created.txt`, Enter. Shift+F10, click Properties | The empty-space menu offers Paste (disabled without a payload), New folder and New file. New file creates a zero-byte file and opens its inline name. The renamed file is on disk; the keyboard menu opens its properties |

## Interrupted batch recovery (S11)

Use a new scratch folder containing `a` with content `A` and `b` with content `B`.
To create a genuine interrupted batch, run the compiled core test child
`ops::tests::batch_kill_child` with `RUSTPHIN_TEST_KILL_DIR=<fixture>`,
`RUSTPHIN_TEST_BATCH_MODE=rename` and `RUSTPHIN_TEST_KILL_STEP=4`. It must exit by
SIGKILL and leave the journal and two staged entries. Use only generated fixtures.
The test binary comes from `rust-tc test`; its filename contains a build hash.

| # | Steps | Expected |
|---|---|---|
| BR1 | Open that fixture in an isolated session/home; Ctrl+Shift+P, query `recover original`, inspect the result, Enter | Browsing initially leaves the journal/staged entries alone. The named recovery command restores `a` = `A`, `b` = `B`; the list updates and journal/staging files are gone |
| BR2 | Add a generated `.rustphin-rename-journal` containing invalid text, repeat the command | Visible error explaining that recovery could not finish and the record/temporary entries must be kept; the invalid record and both files are unchanged. Remove only this generated invalid record before BR3 |
| BR3 | Select both files, F2; find `a`, replace with `b`; then change replacement to `c`, Enter; Ctrl+Z | Conflict text blocks Rename for `b`; `c` enables it. Applying leaves `b` = `B`, `c` = `A`, with no journal. Undo restores `a` = `A`, `b` = `B`, with no journal |
| BR4 | Restart in the same fixture with no journal; run the recovery command again | Neutral "Checked rename recovery" completion notice; both files stay unchanged and no journal is created |

## Not covered by these scenarios

- Blur and the user's own icon theme: an isolated session cannot show them. They need the live desktop.
- Strict frame pacing and wider R-101 acceptance remain open. W-076a fixes the earlier minimum split-pane filename clipping (J6).
- Dropping entries on a tab. W-074c verified mouse tab reordering (T7).
- Dragging to an application that is not a KDE one, and a session without a data-control protocol: neither is available in the test session.
- The trashes of other volumes: the banner lists the real ones of the machine, which belong to the user and are not opened in tests.
- Mounting, unmounting and safely removing a volume: the session shares the machine's real device service, so these are only tested against a mock service in unit tests.
- Full dialog-button pointer coverage. W-076b verified the failure dialog through keyboard recovery and the progress Stop button by pointer (F10–F12).

## Performance and memory (S11 / W-073)

Prepare the generated 500,000-entry fixture with `rust-tc bench-listing` and
build with `rust-tc build-diagnostics`. Use an isolated KWin session/home and
system-only XDG search paths. Never activate a device. Read normal screenshots
to confirm the expected view; AT-SPI absence is not proof of a missing window.

| # | Steps | Expected / evidence to retain |
|---|---|---|
| P1 | Launch the diagnostic binary on the generated large directory with `RUSTPHIN_FRAME_LOG=1` and `RUSTPHIN_SCROLL_BENCH=1`; wait for all ten summaries | Exactly 1,200 driven frames; retain per-window p50/p95/max, longest gap and exclusion count. A gap above 250 ms during active scrolling must remain counted. Excluded idle/startup gaps must still be reported |
| P2 | Repeat P1 separately in details (Ctrl+3), icons (Ctrl+1) and compact (Ctrl+2); persist the mode before relaunch or seed only that isolated config | Confirm the mode and 500,000-item count on screen. Keep toolkit phase/foreground-task timings as well as view/row timings. An over-16 ms draw remains an open R-101 failure even if the model callback is fast |
| P3 | In a fresh default build, open a small generated directory; record process RSS. Open the large fixture in a second tab, wait for details, record RSS; Ctrl+W closes that tab, confirm the small folder returns, record RSS again | Large-folder RSS below 400 MB; after actual tab closure within 50 MB of the small-folder baseline. Record units, elapsed time and which build; navigating away alone is not a tab-close measurement |
| P4 | Navigate into the large fixture and send Home or sort input while its first listing/details are loading; inspect completion and logs | Input changes the selection and the requested order appears while status still says loading. Manual F5 refresh retains the previous view until a complete replacement arrives. A screenshot after completion alone cannot prove initial-loading responsiveness. Physical-display wheel/touchpad pacing remains separate |

### Navigation during loading (W-073d)

| # | Steps | Expected / evidence to retain |
|---|---|---|
| P5 | From a selected file in a small generated folder, Ctrl+L/Ctrl+A/type the large fixture path; verify the field; send Enter with timed compositor captures | New path shows Loading until matching rows arrive. Previous-folder rows/selection actions never appear under the new path. Anchor before sending Enter and retain actual capture bounds; nominal delay alone is not elapsed proof |
| P6 | From the small folder request a missing child; then Home, Shift+F10, F5, Ctrl+L | Failed navigation restores the folder/selection and explains the error. Refresh closes the menu and Ctrl+L still opens the path editor |
| P7 | F2 on a generated entry, Escape; navigate to another folder | Editor cancels without changing the fixture; old-folder rename/menu state is absent after arrival. ShowItems/reveal selection still uses its explicit arrival request |
| P8 | On the completed 500,000-entry fixture, Ctrl+A, F5; wait for the replacement event and inspect status/logs | All 500,000 selections remain; name matching is off the UI thread. Retain model/selection-schedule/selection-apply maxima |
| P9 | F5 then Home before completion; wait for the replacement; repeat while navigating away | The later single selection survives refresh; a superseded result never changes the new folder's selection. A scripted ordering check does not prove every concurrent interleaving |

## Accessibility (S11 / W-074)

The isolated AT-SPI bus can run while its `org.a11y.Status.IsEnabled` is false.
Check it through the isolated session's `dbus_call` and, for these checks only,
set it true using `org.freedesktop.DBus.Properties.Set` on `org.a11y.Bus`,
`/org/a11y/bus`, interface `org.a11y.Status`, property `IsEnabled`.
Never change the host desktop's accessibility setting. Wait for the window and
fixture rows before sending keys. Missing AT-SPI exposure with this flag false
is not proof of a missing toolkit adapter.

| # | Steps | Expected / evidence to retain |
|---|---|---|
| A1 | Open a generated folder with sub, a.txt and b.txt; inspect the accessibility tree | Named file browser/list box, named navigation and sort controls, three named selectable file entries, readable status count |
| A2 | Home, Down; switch Ctrl+1/Ctrl+2/Ctrl+3; query focused and selected entries | The selected entry retains both states in every view; screenshot confirms the visible selection marker |
| A3 | Request Component.GrabFocus on generated b.txt through the isolated AT-SPI bus | b.txt becomes the focused selected entry and the status says one of three selected; normal keyboard navigation still works |
| A4 | Inspect first entry attributes, then the generated 500,000-entry view after scrolling | First small-folder item has posinset=1 and setsize=3. Large-list items carry setsize=500000 and their actual positions; only visible/overscan rows are exposed |
| A5 | F2, inspect, Escape; Ctrl+L, inspect, Escape | Named New name and Folder location fields; cancellation preserves fixtures. Assert exactly one named editable entry with actual focused state; record native Text/EditableText support and Orca separately |
| A6 | Through the file list's AT-SPI Selection interface, select_child(0), select_child(1), then deselect_child(0); repeat in details/icons/compact | The first collection child is a file item, not a sort heading; collection is multiselectable. The first two files are selected together, then only the second remains. Re-check the isolated enable flag after temporary event-listener probes end |
| A7 | Ctrl+A, plain-click one generated file, Ctrl-click another | Plain pointer selection replaces the selection; Ctrl-click adds another. Accessibility selection toggling must not change ordinary pointer behavior |
| A8 | Create two generated bookmarks and a Work group containing one; open two distinct folder tabs; inspect names/roles/parents | Named places tree; grouped bookmark's parent is Work; named tabs/selected state and distinct new/close controls. Observe the native expanded flag separately: the pinned adapter currently omits it |
| A9 | GrabFocus on Work; Left/Right and an AT-SPI click; then focus/activate a generated bookmark | Group focus is exposed, descriptions and children follow collapse; bookmark focus alone does not navigate, activation does. Never send these actions to real device rows |
| A10 | Click a tab and the named new/close controls through AT-SPI; cycle with Ctrl+Tab; perform T7 | Exactly one correct tab stays selected, position/count metadata follows creation/closure/reorder. Closing an inactive tab does not select its neighbour accidentally |
| A11 | F2/Escape on a generated bookmark, G to move groups, then drag onto Work with paused waypoints | Named rename field; Escape retains its title. Group membership, accessible parent and private config agree after keyboard and drag changes |

| A12 | Ctrl+L; select all, type a path and Enter; F2 on a generated file in each view; replace with one key, Ctrl+Z, Escape | Actual editor is focused through AT-SPI; navigation works, undo restores the filename, cancellation preserves fixture bytes |
| A13 | Open palette and type a query; open two-file batch rename, focus Find/Replace with and Pattern/Start/Step through AT-SPI; F2 on generated bookmark/group | Each field has one named focused editable node, native focus reaches its actual editor, normal input works and Escape cancels. Do not infer Text/EditableText interfaces or Orca acceptance from focused state |

| A14 | Open palette; inspect modal, list, positions/count and status. Select a different result through AT-SPI Selection; Down/Up/Tab | Exactly one named selected result changes while the input stays focused. Selection alone never runs a command; keep the native client alive until the resulting state is observed |
| A15 | Query a no-match string; then view compact, Enter. Reopen, query view details and click its row | Empty list/status explain no matches; explicit keyboard and pointer activation change mode and close the palette. Escape returns keyboard control to the browser |

| A16 | Shift+F10 on a generated file; inspect menu/items; Down, Right on Open with, Left, Escape | Named File actions menu and one focused named item; native focus follows main/submenu keyboard cursor. Application submenu names do not imply launching any application |
| A17 | Native focus Copy, Down; native activate Rename, Escape. Native activate Open with, Left/Escape | Focus alone never runs the item. Down skips disabled Paste; Rename opens the focused New name editor. Disabled Paste has no action and says Unavailable. Retain the pinned adapter enabled/expanded-flag findings separately |

| A18 | Alt+Enter on generated a.txt; inspect facts, modal/focused dialog and permission names. Native toggle Owner Execute, Escape, reopen; native Close | All nine permission names distinguish Owner/Group/Others. UI changes enable Apply, Escape discards them; native Close closes. File mode and bytes stay unchanged without Apply. Query fresh native state if the client cache misses updates; retain cache warnings as a limit |

| A19 | In a generated two-file fixture, F2; inspect modal and preview. Replace a with b, then c; native Cancel | Preview describes current/new names and collisions; conflicting plan has no Rename action, valid correction clears errors/enables it. Cancel leaves names/bytes unchanged |
| A20 | Shift+Delete on a generated file; Ctrl+Shift+P; native Keep. Activate a generated harmless executable and native Keep | Named focused modal includes the risk explanation; palette does not open behind it; Keep cancels. No file deleted and the executable marker remains absent |
| A21 | Copy generated a.txt into a second fixture folder already holding a different a.txt; inspect conflict then native Stop | Named focused modal with Existing/Incoming facts and available choices; Stop leaves source/destination bytes unchanged. Do not activate Replace/Merge/Delete/Run for this semantics-only check |

| A22 | Read Folder location, New name in all three views and empty palette search through AT-SPI Text. On a generated `aé🦀é.txt`, set caret 2 and selection 1..3; type x, undo, Escape | Text count uses Unicode characters, selection covers exactly é🦀, typing replaces those characters, undo restores text, cancel preserves filename/bytes. Check EditableText separately; the pinned Unix adapter does not implement it. Native selection writes normalize direction through the public editor setter |

## Untrusted input (S11 / W-075)

Use generated scratch data and the isolated home only. A harmless executable and
shortcut may write a marker inside the fixture only when explicitly run; never
confirm execution during these checks.

| # | Steps | Expected / evidence to retain |
|---|---|---|
| H2 | Browse generated invalid-UTF-8/newline and 255-byte filenames, malformed desktop/PNG files and harmless executable/shortcut; switch icons/compact/details | Names stay inside rows/cells; broken image keeps a type icon and gets a failure marker. No executable marker appears from browsing or thumbnailing. Opening the malformed shortcut reports an error, without a crash |
| H3 | Select the generated invalid-byte name, copy it and paste into another generated folder; repeat with the 255-byte name | Destination names and contents exactly match source bytes. Keep source bytes and no-execution marker checks, plus a screenshot of the selected/copied entry |
| H4 | Add a generated bookmark, corrupt only the isolated home's user-places.xbel, observe, then restore it | Last good bookmark/group remains; malformed bytes stay untouched; named error explains repair/restore. Dismiss, touch an unrelated sibling: no repeated alert. Change the parse error: new warning. Restore valid data: only its own warning clears, preserving a newer unrelated operation error. Repeat with private file mode 000, then restore 0600: Permission denied warning clears automatically |
