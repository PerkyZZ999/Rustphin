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
- Spell punctuation keys literally: `ctrl+=`, `ctrl+-`; use lower-case function keys such as `shift+f10` in kwin-mcp input.
- Detect blur in the isolated compositor instead of assuming it is absent: current KWin may expose the background-effect global even with its blur effect unloaded. J12/J14 toggle only the private compositor. Explicit opaque mode is available for deterministic solid-theme checks. Icons use the default `breeze-dark` theme.
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
| T5 | Ctrl+W until one tab is left; then Ctrl+W once more | The tab strip disappears at one tab. Closing the only tab closes the window (the existing Dolphin-style behavior in Workspace::close_tab); saved state still names the last folder |
| T6 | With two tabs, click the other tab; click its × | The clicked tab becomes active; the × closes that tab |
| T7 | Drag the active second tab onto the first, using several waypoints paused for about 100 ms | The active folder moves to the first position; selected state, visible underline and saved session order agree |
| D1 | F3 | Two panes side by side on the same folder, a thin divider between them, an accent line above the pane that has keyboard focus; narrow panes keep Name; Size appears at 320 px per pane |
| D2 | Tab, then Down and Return on a folder | The accent line moves to the other pane; only that pane changes folder |
| D3 | F3 again | The tab shows one pane, the one that had focus |
| R1 | With two tabs, one of them split, read `<isolated home>/.local/state/rustphin/session.toml`; kill the app and `launch_app` it with no argument | The file lists both tabs and the split tab's two panes; the app reopens with the same tabs, split and locations |
| R2 | Private generated data: seed a version-1 session with Unicode/literal-percent paths; launch and inspect upgrade. Then open a generated non-UTF-8/control-byte folder via a local file URI, split it, navigate the other pane, close/restart without arguments | Legacy paths still open and upgrade to version-2 URI panes without decoding literal percent signs. Decode the saved URI to bytes and compare with the original filename; both panes, active state, source file bytes and saved TOML survive restart |
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
| K8 | Click a place row, Ctrl+L, Ctrl+A, type a path containing `h`, Return; click a place, F6; then F2 on a place and Ctrl+L; Escape | Ctrl+L and F6 open the location field from the places panel, the typed text goes into the field (no place is hidden), Return opens the path. In a place's rename field Ctrl+L leaves the field focused. user-places.xbel is unchanged |
| K9 | Keyboard only after giving the window focus: Ctrl+P, Down, Enter; Alt+D, Escape, Backspace; Alt+Home; Ctrl+T, Alt+Up, Ctrl+T; Alt+1, Alt+0, Alt+5; Alt+1, F3, Alt+Up; Ctrl+Q | Places panel takes focus and opens the second place; Backspace goes back (Forward enabled), not up; Alt+Home opens HOME; three tabs; Alt+1 first, Alt+0 last, Alt+5 no change; split, the new right pane has focus and alone goes up; Ctrl+Q closes the window and session.toml holds all three tabs and the split |
| K10 | Generated Left (alpha.txt, beta.txt) and empty Right. Unsplit Shift+F5; F3, type `a`, Shift+F5; point the other pane at Right (Tab, Tab reaches the left pane; Ctrl+F3 is taken by KWin); `a`, Shift+F5; `b`, Shift+F6; Ctrl+Z | Split notice; same-folder notice; alpha.txt copied and beta.txt moved into Right with identical bytes, move notice offers Undo; Ctrl+Z returns beta.txt |
| K11 | Generated Start and Split/Deeper. Ctrl+T; Ctrl+L to Split; F3; `D`, Enter in the right pane; Ctrl+W; Ctrl+Shift+T; Ctrl+Shift+T | The second tab closes; it comes back split (Split, Split/Deeper) with the right pane active; the second press says there are no closed tabs to reopen |
| K12 | Session with clipboard enabled; generated `Docs dir` with `report final.txt` and `zeta.txt`. Ctrl+Alt+C with nothing selected; `r`, Ctrl+Alt+C, read the clipboard; Menu key; Escape, `z`, Menu key, Down ×4, Enter, read the clipboard | Notice asks to select an item; clipboard holds `<home>/Docs dir/report final.txt` and a notice names it; the Menu key opens the context menu with Copy location after Paste (disabled Paste skipped by Down); clipboard then holds the zeta.txt path |
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
| Q9 | Private state only: record a visit, remove write permission from its state directory, select Clear recent folders after results settle; restore permission and retry | Failure message reports save failure, old file bytes remain; retry writes an empty history and reports success. Restore directory permissions before cleanup |
| Q10 | Quit all private instances. Seed damaged, then newer-version history in separate launches; open palette and select Clear. Finally quit and remove private recent-locations.toml/session.toml, then relaunch with one explicit fixture folder | Damaged/newer reason is visible and original bytes stay intact after Clear. Offline removal forgets both saved stores; only the new explicit folder is saved. Never operate on the real home |
| Q11 | Private state only. Seed session.toml as a newer format (version 3) and launch with one explicit fixture folder; then as damaged TOML launched without arguments; then mode 000 with an explicit folder. Each time open a tab, navigate, close the window and compare the file. Finally remove the file, launch with a folder and open a tab | Explicit folder or fallback tab opens; persistent error notice names the reason and the next step; the stderr warning names it too. File bytes (and mode) unchanged after each run. With no file, session.toml is written as format 2 with the open tabs |

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
| J8 | Select generated files, F2; resize to 480 × 320. Pattern, Tab through counter fields; invalid Step then repair; Case/Upper; Replace collision then valid correction. Ctrl+Page Up/Down and scrollbar through a 22-row preview; Escape | Title/Cancel/Rename stay inside the dialog; focused fields are revealed; errors disable Rename, valid changes enable it; keys and scrollbar reach first/last preview rows; cancellation preserves every name/byte. Record reverse traversal separately |
| J9 | Two-file batch: Find alpha, Tab/type beta, Shift+Tab, Ctrl+A/type alpha2; native focused-field/Text query on Find; Escape | Desired: Find becomes alpha2 and replacement stays beta, with native Find focus/Text intact. W-076e baseline fails reverse traversal; removing wrapper tracking fails native focus, so neither is an acceptance pass |
| J10 | Generated long-name conflict and program/long-Name shortcut; resize to 480 × 320, End/Home, Page Up/Down, arrows and scrollbar; toggle conflict A, Escape/reopen, Skip; test a private unreadable source and Stop, and Shift+Delete/Escape; enlarge | Short heading/actions stay inset and visible; actions wrap. Full question/facts/explanation scroll to both ends; a new question starts at the top. Modal keys do not open a tab/palette. Cancellation/Skip preserve names/bytes and run markers remain absent; failed-copy Stop removes staging. Do not confirm permanent deletion or run the fixture |
| J11 | Start with an app-only unreachable system-bus address, then a private bus with no UDisks2. On that private bus, run the generated-only service through empty/device/error/device inventories; resize to minimum. Navigate/copy generated files by path | Starts with readable Devices status on connection/list failure; successful empty clears it; generated row appears, is removed on failure and returns on recovery; advice stays inside minimum window. Browsing/copy still work and exact bytes match. Never use the real system bus or activate a device row |
| J12 | In the isolated compositor only, unload blur and verify its background-effect global remains; open a generated white background window, then Rustphin. Replay with RUSTPHIN_OPAQUE=1, then re-enable the isolated effect and launch normally; capture startup bursts | Without usable blur, solid token colours cover white and a persistent opaque-theme status appears. Explicit opaque uses the same colours without an unavailable notice. Enabled blur permits glass without that notice. Compare colours using regular screenshots, not channel-swapped burst frames. J14 covers live capability loss after opening; real-desktop review remains separate |
| J13 | In an isolated KWin/home, use a test security-context connection with both data-control interfaces absent; confirm normal windowing globals. Launch unchanged Rustphin on that socket. Copy Unicode fixture, cut/move space-named fixture, undo move, paste again; restart and first-copy at minimum size | Starts normally. First copy explains in-app-only paste; the next cut does not repeat it and appears dimmed. Exact names/bytes survive copy/move/undo. Consumed cut clipboard reports no files on the next paste. Minimum-window first-use notice wraps completely with Dismiss visible. An ordinary client sees no shared file clipboard content. No OS sandbox claim; no real user files/devices |

| J14 | Over a generated white window in an isolated compositor/private home, start with blur disabled; enable, disable and re-enable it without restarting Rustphin. Repeat at 480 × 320, including a generated file copy during capability loss. Close normally; replay with explicit opaque mode through the same toggles | Same process switches solid/glass tokens with capability changes; the persistent readable blur status appears only while unavailable and clears on recovery. Copy notice/Undo/Dismiss remain independent and source/destination bytes match. No duplicate-effect protocol errors. Window close removes its monitor worker. Explicit opaque remains solid and quiet with no monitor thread. Protocol fixtures separately cover manager removal, disconnection and silent-compositor cancellation. No real desktop or devices |

| J15 | At 480 × 320 in a private home, drag a long Unicode generated source onto a long destination name with paused waypoints; End, Escape, repeat and C. Trash a separate generated file, use the palette to open the private home trash and request Empty this trash; cancel with Escape and Keep, then restore the generated item | Drop keeps heading/all actions visible and scrolls complete names/instructions. Cancel changes nothing; Copy preserves exact names/bytes. Empty dialog shows count, irreversible warning and both actions completely. Both cancellation paths preserve private content/record; Restore returns matching bytes. Never accept permanent deletion or activate other-volume links. Separately inspect the trash banner for clipping (W-076l) |

| J16 | Private home/generated trash item only: show trash normally, at minimum frame size and in a minimum split. Open Trash actions by pointer and palette, check initial disabled Restore, select the fixture and restore it. Reopen menu then palette; test Empty confirmation and cancel. In a generated ordinary folder, Shift+F10 then Up reaches Properties | Header/button and complete explanation remain visible in each pane. One bounded single-column popup paints across pane edges; native menu name/focused choice agree. Empty opens confirmation, cancellation preserves bytes/record, Restore returns exact bytes. Palette/dialog focus closes the menu. The final context-menu row scrolls fully into view. No other-volume link or permanent-delete confirmation is activated. Record native client bounds alongside frame size; short empty-state clipping is W-076m |

| J17 | In a private home/minimum split, show a hidden-only generated folder/trash and use Home/End, Page Up/Down and line keys; focus the hidden-files action, then activate with Enter, Space and pointer. Test location Home/End/literal Space and menu arrows. Restore only a generated fixture to leave an operation notice, then End in the now-empty pane. Finally use a generated large hidden listing to observe real Loading | Each state is bounded, visibly scrollable and named natively. Full heading/count/action are reachable; focused action scrolls into view and showing hidden returns list focus. Inputs/menu keep their own keys, ordinary list End still selects a file. The full empty reason can be read in a 56 px body with notice/Undo/Dismiss. Loading is visible in 85 px split panes and changes to the correct completed count. Never activate another volume or permanently delete anything; loading-only links need not have trash records and are never restored/emptied |

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

## Remote folders (W-080)

Private session bus only. Use a generated stand-in for `org.kde.KIOFuse` (target/rust-toolchain/w080/mock-kiofuse.py maps `sftp://fixture-host/<p>` and a 5-second `slow-host` to generated folders, refuses `http`, fails `fail-host`) for the sftp rows, and the real kio-fuse with a generated `tar:` archive for the last rows. Never type a real host: kio-fuse would connect to it. Focus the file view before Ctrl+L.

| # | Steps | Expected / evidence to retain |
|---|---|---|
| W1 | Places file with a `file://` bookmark, an `sftp://me@fixture-host/projects` bookmark and `remote:/`; launch | The sftp bookmark is listed with the server icon; `remote:/` is not |
| W2 | Click the sftp bookmark; open `sub dir`; Ctrl+L | Fixture rows appear; the service log shows the bookmark address; the location field reads `sftp://me@fixture-host/projects/sub dir` |
| W3 | Ctrl+L, type `sftp://me:hunter2@slow-host/data`, Return; screenshot at once and after 6 s | "Connecting to sftp://me@slow-host/data…" without the password, then the folder opens and the notice is gone; `hunter2` does not appear in Rustphin's log |
| W4 | Type `sftp://slow-host/data`, Return, then click another place within 5 s | The view stays on the other place after the reply arrives |
| W5 | Type `sftp://fail-host/x`, then `http://example.org/` | Error notices name the address, the reason and what to do; the view does not change |
| W6 | Type a `file://` URI of a generated folder with `%20` | Opens that local folder |
| W7 | Stop the stand-in; type `tar:<generated archive>/`; open its folder; Ctrl+L; hash a file through the mount | kio-fuse starts on the private bus and mounts it; rows listed; the field shows the `tar:` address; bytes match the source |
| W8 | Type `tar:<missing archive>/`; close the window and stop the session | Error "… could not be opened. No such file or directory. …"; afterwards no kio-fuse mount or process remains |
| W9 | With the stand-in: open the sftp bookmark, enter `sub dir`; read session.toml; click the first crumb; re-enter, close, relaunch without arguments | Crumbs read `sftp://me@fixture-host/projects / sub dir`; session.toml stores `sftp://me@fixture-host/projects/sub dir`; the first crumb opens the mount root; on relaunch the tab shows the fallback briefly, then the remote subfolder |
| W10 | Restart the stand-in with `fail`, relaunch; then open a local folder in that tab; then type `sftp://me:hunter2@fixture-host/projects` | Fallback folder with an error notice naming the address; session.toml is unchanged until the tab is moved, then holds the local folder; the failed typed address is not saved and `hunter2` appears in no state file or log |
| W11 | With the stand-in: open the sftp bookmark and `sub dir`, then Local; read recent-locations.toml; close, relaunch, Ctrl+Shift+P, `recent sub`, Return | The file lists `sftp://me@fixture-host/projects/sub dir` and `…/projects` by address; the palette offers "Recent: sub dir" with the address as detail; Return mounts it and shows its rows |
