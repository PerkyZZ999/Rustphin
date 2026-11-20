# Requirements

Status: **MVP boundary confirmed by the user, 2026-10-12.**

Conventions:
- Priority: **must** (MVP cannot ship without it), **should** (in the MVP, first to cut if needed).
- Validation: **test** (automated, headless), **bench** (repeatable measurement), **ui-test** (scripted in an isolated KWin session), **manual** (checklist run by the author).
- "Reference machine" means the author's desktop: Plasma 6.7 on Wayland, 16 threads.

## MVP — functional

### R-001 — Browse directories
- **Priority:** must
- **Description:** Open a local directory, show its entries, navigate into folders, back, forward, up, and to a typed path.
- **Acceptance:**
  - [x] Opening a directory lists every entry Dolphin lists for it, with hidden files off by default and a toggle to show them. (W-078: K15 side-by-side with Dolphin, same 10 entries and order; S1 checks for Ctrl+H.)
  - [x] Back, forward and up work with mouse and keyboard; history is kept per tab. (W-078: K14 toolbar clicks, Alt+Left/Right, Alt+Up, Backspace; separate history per tab.)
  - [x] A breadcrumb location bar switches to an editable path field; an invalid path shows an inline error and keeps the current view. (W-078: Slice S1 checks, N5/K8.)
  - [x] Activating a file opens it with the system default application; executables and `.desktop` files ask before running.
  - [x] An unreadable directory shows an inline error state, not a crash or an empty view. (W-078: Slice S1 checks, N4.)
- **Validation:** test (model), ui-test (navigation), manual (open-with)

### R-002 — Live updates
- **Priority:** must
- **Description:** The view reflects changes made by other programs.
- **Acceptance:**
  - [x] Files created, renamed, changed or deleted outside Rustphin appear in the open view within 1 second without a manual refresh. (W-078: W-025 U1/U2; K14 content change shown within 1 s.)
  - [x] Selection and scroll position survive an update unless the selected entry was removed. (W-078: W-025; K14 in an 81-entry scrolled list. A renamed entry counts as removed.)
- **Validation:** test

### R-003 — View modes, sorting and zoom
- **Priority:** must
- **Description:** Icons, compact and details views, with sorting and zoom.
- **Acceptance:**
  - [x] Three view modes are selectable per tab: icons, compact, details. (W-078: K14 icons and details tabs side by side; W-019.)
  - [x] Details view has name, size, modified and type columns; clicking a header sorts by it and toggles direction. (W-078: W-018, W-022 O1–O3.)
  - [x] Sorting is natural and case-insensitive by default ("file2" before "file10"), with folders first as an option, on by default. (W-078: K14 incl. the new View: Folders first toggle.)
  - [x] Zoom changes icon size in all three modes. (W-078: W-019, V3; K14 details zoom in/out/reset.)
  - [x] View mode, sort and zoom are remembered between sessions. (W-078: W-021, W-022 P1/P2.)
- **Validation:** test (sort order), ui-test (modes)

### R-004 — Selection and keyboard operation
- **Priority:** must
- **Description:** Every MVP function is reachable without a mouse.
- **Acceptance:**
  - [x] Click, Ctrl+click, Shift+click, rubber-band, select all and invert selection work. (W-078: W-020, W-022 S2/S3, W-024, W-073e.)
  - [x] Arrow keys, Home/End, Page Up/Down move focus; typing letters jumps to the next matching name. (W-078: W-020, K10, K14 Page Down/Shift+Page Down.)
  - [ ] Every action in R-001 to R-016 has a keyboard path, and the default shortcuts match Dolphin's where an equivalent exists. (Shortcuts: one command table in `rustphin_view::commands`, tested against KEYBOARD.md and for clashes (W-123); every KEYBOARD.md key replayed on Qt in W-114h and W-123. Keyboard-only pass on Qt, Shift+Tab included, W-076t (K20, K21); W-076u gave the problems dialog a keyboard path (the palette's "Files: Show what the last operation left out"); open: the transfer dialog not seen keyboard-only.)
- **Validation:** ui-test, manual (keyboard-only checklist)

### R-005 — Tabs
- **Priority:** must
- **Acceptance:**
  - [x] Open, close, reorder and switch tabs by mouse and keyboard; middle-click a folder opens it in a new tab.
  - [x] Open tabs and their locations are restored on next start.
- **Validation:** ui-test

### R-006 — Split view
- **Priority:** must
- **Acceptance:**
  - [x] A tab can be split into two panes with independent locations, each with its own view settings.
  - [x] Copy and move between panes work by drag-and-drop and by keyboard. (Keyboard: done in S4 with Ctrl+C/X/V; drag-and-drop: S5.)
- **Validation:** ui-test

### R-007 — File operations
- **Priority:** must
- **Description:** Create folder, create empty file, rename, copy, move, trash, permanently delete.
- **Acceptance:**
  - [x] Each operation works on single and multiple selections, including nested folders, symlinks (not followed on copy of the link itself) and names that are not valid UTF-8.
  - [x] Operations run in the background with progress, speed, remaining count and a cancel control; the UI stays responsive.
  - [x] A name conflict offers skip, overwrite, rename and "apply to all"; nothing is overwritten without that choice.
  - [x] A cancelled or failed copy leaves no partial file under the final name.
  - [x] A move across filesystems removes the source only after the copy is complete and verified by size.
  - [x] Delete sends to trash by default; permanent delete requires explicit confirmation.
  - [x] Copies use reflink or `copy_file_range` where the filesystem supports it.
- **Validation:** test (property and fault-injection tests in temporary directories), manual

### R-008 — Undo
- **Priority:** must
- **Acceptance:**
  - [x] Rename, move, copy, create and trash can be undone in reverse order within the session.
  - [x] Undo refuses, with an explanation, when the affected files have changed since the operation.
- **Validation:** test

### R-009 — Trash
- **Priority:** must
- **Acceptance:**
  - [x] Trash follows the freedesktop trash specification, including per-volume trash directories, so items trashed in Rustphin appear in Dolphin's trash and the reverse.
  - [x] The trash view supports restore and empty trash (with confirmation).
- **Validation:** test, manual (interop with Dolphin)

### R-010 — Drag-and-drop
- **Priority:** must
- **Acceptance:**
  - [x] Dragging within and between Rustphin panes, tabs and windows copies or moves, with a modifier or menu to choose.
  - [x] Dragging files out to Dolphin and to at least one non-KDE application delivers them. (W-114g, isolated session: a file dragged into Dolphin (Copy Here) and into Nautilus, a GTK application, arrived with the original kept. Not yet tried on the maintainer's desktop.)
  - [x] Files dropped from other applications are accepted onto a view or a folder.
  - [x] Names with spaces and non-ASCII characters survive the round trip.
- **Validation:** ui-test (with Dolphin), manual (non-KDE app)

### R-011 — File clipboard
- **Priority:** must
- **Acceptance:**
  - [x] Copy and cut in Rustphin paste correctly in Dolphin; copy and cut in Dolphin paste correctly in Rustphin, with cut resulting in a move.
  - [x] Cut items are shown dimmed until pasted or the clipboard changes.
  - [x] Names with spaces and non-ASCII characters survive the round trip.
  - [x] Where the compositor lacks a data-control protocol, copy and paste still work inside Rustphin and the limitation is reported once. (W-076i on the GPUI interface: isolated KWin connection with both data-control interfaces hidden; copy/cut/paste and the one-time notice verified. The fallback is in the platform (`clipboard::for_session`, `LocalClipboard`) and the notice in `OperationBoard::picked_up`, both unchanged by the Qt switch; not replayed on Qt.)
- **Validation:** ui-test (with Dolphin), test (URI encoding)

### R-012 — Places and place groups
- **Priority:** must
- **Description:** A places panel shared with KDE, plus user-defined collapsible groups.
- **Acceptance:**
  - [x] Places from the shared KDE places file are shown; adding, renaming, removing and hiding a place in Rustphin is visible in Dolphin, and the reverse.
  - [x] The user can create, rename, reorder, collapse and delete groups, and move places into and out of them by drag-and-drop and by keyboard.
  - [x] A group's collapsed state and order persist between sessions.
  - [x] A grouped place still appears, ungrouped, in Dolphin and KDE file dialogs; the shared file remains valid for them after any Rustphin edit.
  - [x] Deleting a group does not delete its places.
- **Validation:** test (shared-file round trip against fixtures), ui-test, manual (Dolphin interop)

### R-013 — Devices
- **Priority:** must
- **Acceptance:**
  - [x] Removable and internal volumes appear in the places panel with name, and with capacity usage when mounted.
  - [ ] Mount on activation, unmount and safely remove work; plugging and unplugging updates the list without restart. (Implemented and tested against a mock service; not yet tried on a real drive.)
  - [ ] A failed unmount (device busy) shows the reason. (Tested against the mock service; not yet seen on screen.)
- **Validation:** test (against a mocked UDisks2 service), manual (real USB drive)

### R-014 — Thumbnails and icons
- **Priority:** must
- **Acceptance:**
  - [x] File-type icons come from the system icon theme by MIME type.
  - [x] Images show thumbnails, generated off the UI thread and only for visible or nearly visible entries; scrolling away cancels pending work.
  - [x] Thumbnails are read from and written to the shared freedesktop thumbnail cache, so they are reused by and from Dolphin.
  - [x] A corrupt or oversized image never crashes or stalls the application.
- **Validation:** test (cache format, malformed inputs), manual

### R-015 — Command palette and fuzzy jump
- **Priority:** must
- **Acceptance:**
  - [x] One shortcut opens a palette that lists every application action with its shortcut and runs the chosen one.
  - [x] Typing fuzzy-matches actions, places, open tabs and recently visited folders; Enter navigates or runs.
  - [x] Results update within 50 ms of a keystroke with 10,000 recent locations.
  - [x] Recent-location history is stored locally and can be cleared.
- **Validation:** test (matching and ranking), bench, ui-test

### R-016 — Advanced batch rename
- **Priority:** must
- **Acceptance:**
  - [x] With several files selected, rename opens a dialog offering find/replace (plain and regular expression), a pattern with counters (start, step, padding), and case changes.
  - [x] A live preview shows old and new names for the whole selection before anything is changed.
  - [x] Conflicts (duplicate targets, existing files, invalid names) are flagged in the preview and block applying.
  - [x] Applying is all-or-nothing and is a single undo step.
- **Validation:** test, ui-test

### R-017 — Default file manager integration
- **Priority:** should
- **Acceptance:**
  - [x] A desktop entry lets Rustphin be set as the default handler for folders.
  - [x] "Show in folder" requests from other applications (the `org.freedesktop.FileManager1` D-Bus interface) open the folder and select the item.
  - [x] Paths and `file://` URLs given on the command line open as tabs.
- **Validation:** test (D-Bus), isolated desktop scenarios I1, I10 and I12; observed in W-068 (2026-10-19); on the Qt interface, FileManager1 ShowItems and ShowItemProperties and a handed-over launch in W-114g, W-114i and W-116b (I1, I10). Real-user default registration is not changed by tests.

### R-018 — Context menu and properties
- **Priority:** should
- **Acceptance:**
  - [x] Right-click offers the operations of R-007 to R-011 relevant to the selection, plus "Open with" listing applications registered for the MIME type.
  - [x] A properties view shows name, type, size (computed in the background for folders), times and permissions, and can change permissions.
- **Validation:** isolated desktop scenarios I2–I9, I11 and I13; observed in W-068 (2026-10-19), with limits in VALIDATION.md; replayed on the Qt interface in W-114h and W-116b (I2–I9, I13); Properties as a window with tabs, the group and checksums, W-170 (PR1–PR9).

### R-019 — Settings window (after the MVP, D-029)
- **Priority:** could (requested by the maintainer on 2026-11-12, beyond the MVP)
- **Acceptance:**
  - [ ] "Settings…" in the Actions menu, the palette command and Ctrl+Shift+, open one Settings dialog over the window with General, Appearance, Views, Toolbar, Places, Confirmations, Shortcuts and About pages, reachable by mouse and keyboard. (W-106b–g on GPUI: all eight pages, opened from the palette and Ctrl+Shift+,; pages by Up/Down, a page by Page Up/Down, drop-downs by arrows. Qt: opened by Ctrl+Shift+, and the palette, General, Views, Toolbar, Shortcuts, Places and About pages seen (W-114f, W-114h, W-116a). Open: Tab through the controls was not seen moving.)
  - [ ] Every change applies at once in every window and survives a restart; each page can be reset to its defaults; a damaged or newer settings file never stops Rustphin starting and is never overwritten. (Applies at once and survives restarts on screen, Reset seen on every page with settings (GPUI); a damaged, unreadable or newer settings file is kept, saving is turned off and the user told, unit-tested and seen on screen on Qt (W-140, `saved_file`). Open: a second window following a change was not seen.)
  - [x] The Toolbar page adds, removes and reorders toolbar actions and separators, chooses icon only or icon and words per action, and restores the default toolbar; the responsive rules still apply. (W-106f: TB10–TB15 on screen; the narrowing rule unit-tested.)
  - [ ] Each page's settings do what SETTINGS_PLAN.md describes, with today's behaviour as the default. (Built W-106c–g on GPUI with today's defaults. Three differences from the plan accepted by the maintainer, D-030: no "system format" for dates; hidden files fade from 60 %, not 30 % (AA); toolbar items move by buttons, not by dragging. Unticked 2026-11-14: the Qt interface stores but ignores two Appearance choices: Look (glass or opaque; only `RUSTPHIN_OPAQUE` turns the glass off) and Title bar (Rustphin's or KDE's; the window is always frameless with Rustphin's title bar).)
  - [x] About shows the version, the licence text, credits and where Rustphin keeps its files; nothing is fetched from the network. (W-106b, ST2–ST4.)
- **Validation:** unit tests of the settings model (defaults, parsing, migration from version 2); isolated desktop scenarios per page; mockups approved by the maintainer first (W-106a).

### R-020 — Background mode (after the MVP, D-031)
- **Priority:** could (requested by the maintainer on 2026-11-13, beyond the MVP)
- **Acceptance:**
  - [x] With "keep running" on, closing the last window leaves Rustphin running, and the next window (launcher, a folder sent by another application, the command line) shows its folder within 200 ms of the request on a warm machine, whatever the font cache holds. (W-110b on GPUI: 41 ms after a close, 94 ms after a background start; BG3, BG4, BG7. Qt (W-114c3): later launches handed over in 16–18 ms and opened a fresh window; the time to the window's first frame was not measured on Qt.)
  - [x] One Rustphin process runs per session; a second launch hands its request to it and exits without starting the UI toolkit; if the running one does not answer within 2 s, the launch starts its own. (W-110a: 12 ms hand-over on screen; the time limit unit-tested. Qt: W-114c3, W-114i.)
  - [x] With no window open, Rustphin's own memory (RssAnon) is at most 60 MB and it uses no CPU while nothing happens. (Qt, W-114c3: 53.7–54.9 MB after closing the last window over three cycles (the engine is deleted and `malloc_trim` returns freed memory), 9.8 MB after `--background`; no CPU time over 5 s. GPUI: 41 MB and 11.6 MB, W-110e.)
  - [x] Ctrl+Q, the palette's Quit and `rustphin --quit` end the process; closing windows does not. (BG3, BG5, BG8.)
  - [x] The login switch adds or removes exactly one autostart entry and changes nothing else. (Unit-tested; BG6.)
  - [ ] New windows get the focus when the request carries an activation token. (Qt: a launch's token, and a FileManager1 caller's `startup_id` (W-120), are handed to Qt as `XDG_ACTIVATION_TOKEN` before the window asks for activation; the private-bus test sees the token delivered. KWin's focus needs a real token, so it is not yet seen: the maintainer's desktop.)
  - [x] With "keep running" off, Rustphin behaves as before W-110. (BG9.)
- **Validation:** private-bus tests of the hand-over and `--quit`; unit tests of the autostart entry; isolated-session scenarios for closing, reopening, a second launch and quitting; memory and wake-up measurements with no window.

### R-021 — Compress and extract (after the MVP, D-032)
- **Priority:** could (requested by the maintainer on 2026-11-13, beyond the MVP)
- **Acceptance:**
  - [x] Extract › offers Extract here (one top-level item lands as it is, otherwise a folder named after the archive), Extract into "name/", Extract and trash archive (trash only after everything extracted) and Extract to… for ZIP, TAR (plain, .gz, .bz2, .xz, .zst) and 7Z; other formats (RAR, ISO, CAB, LHA, CPIO, RPM, DEB, lone compressed files…) are read through libarchive inside the engine, and what libarchive cannot read (encrypted RAR or 7Z, ARJ…) through 7-Zip or unrar, still inside the engine, with a readable message when neither is installed (D-036). (W-111: every entry on screen with ZIP and 7Z, every TAR variant in engine tests; W-119: ISO, CPIO and a lone .gz on screen through libarchive; 7-Zip with a password in unit tests against the real 7-Zip; RAR not tried: no RAR fixture.)
  - [x] Compress › makes "name.zip" and "name.tar.gz" named as Dolphin does; Compress to… offers ZIP, TAR.GZ, TAR.XZ, TAR.ZST and 7Z at three levels. (W-111: ZIP and 7Z on screen, all five in round-trip tests.)
  - [x] Name clashes go through the conflict question; nothing is overwritten without a choice; a stopped or failed operation leaves no partial result. (W-111: clashes on screen; Stop and a fault at every step in engine tests.)
  - [x] Progress and Stop in the operation bar; Undo moves what was made to the trash, and restores a trashed archive. (W-111: progress, Stop button and Undo on screen; stopping part-way in engine tests, not on screen.)
  - [x] A hostile archive cannot write outside its destination, create devices or set-ID files, or fill the disk unnoticed; a damaged archive or wrong password ends with a readable reason. (W-111a: hostile-TAR, free-space and damaged-archive tests; `archive_entry` fuzzing.)
  - [x] Encrypted ZIP and 7Z ask for a password, which is never logged or kept. (W-111e: AES ZIP on screen, wrong then right, no trace in the log; 7Z through the same engine path in tests.)
- **Validation:** core tests with fake `Fs` and fault injection; hostile-archive fixtures; the `archive_entry` fuzz target; isolated-session scenarios for each entry, a clash, Stop, Undo and a password.

## MVP — non-functional

### R-101 — Responsiveness at scale
- **Priority:** must
- **Acceptance:** on the reference machine, in a directory of 500,000 entries on a local filesystem:
  - [x] First entries are visible within 200 ms of opening (warm cache) and the UI accepts input throughout loading. (D-022: opening a folder in a running window; application startup is outside this. W-073g: 59–78 ms in every view mode; input during loading W-073e. Qt interface: 91.9 ms (W-114d) and 136.2 ms (W-114h).)
  - [x] Re-sorting by any column completes within 500 ms once the sizes and dates it needs have been read. The first read has no fixed limit, but the list is shown in name order with a loading indicator while it runs, and details appear from the top of the list first. (Reworded 2026-10-12 with the user's agreement: on a cold disk cache the read is bounded by the disk, 5.3 s for 500,000 files.) (W-078: rust-tc bench-listing, slowest re-sort 92.7 ms in W-077b; name order with loading indicator and details from the top in W-023/W-073c. Qt interface: re-sort by size 129.9 ms (W-114d) and 88.0 ms (W-114h) to the frame showing all 500,000 rows.)
  - [x] Scrolling holds 60 frames per second: no more than 5% of frame intervals during sustained scrolling exceed 1.5 × the refresh period, in each view mode. (Reworded 2026-11-08 by D-019 with the maintainer's agreement; previously "at the 95th percentile".) (W-085 on GPUI: details 3 dropped (0.25%), icons 1 (0.08%), compact 0 of 1,199 intervals. Qt interface, details: 0 of 1,200 dropped (W-114d, W-114h), 2 of 1,200 (0.17%) while details loaded during a mode switch (W-114d); icons 2 (0.17%, the interval with the switch) then 0, compact 1 (0.08%) then 0 of 1,200 (W-150, 2026-11-15). Isolated KWin at 60 Hz; not measured on a physical display.)
  - [x] No UI-thread task exceeds 16 ms in listing, sorting, metadata or thumbnail code paths. (D-022: application startup is outside this. Measured on GPUI: model callback 0.8 ms (W-073e), thumbnail updates 0.56 ms and draws 9.3 ms while 2,000 were made (W-073f). Qt interface: listing, sorting and metadata still run on the core's workers; task times on the Qt thread are not measured separately (W-114d), only through the dropped-frame counts above; the theme icon provider draws icons on the Qt thread (W-116a, not re-measured).)
- **Validation:** bench (headless core), ui-test with frame timing

### R-102 — Memory
- **Priority:** should
- **Acceptance:**
  - [x] Resident memory with one 500,000-entry directory open stays under 400 MB; closing the tab returns to within 50 MB of the starting figure.
- **Validation:** bench; W-073b (2026-10-22, GPUI interface), final default release in isolated KWin: 130332 KiB small baseline, 162124 KiB large/details tab, 150112 KiB after closing that tab (19.3 MiB above baseline). Qt interface (W-114d): own memory 124–156 MB with 500,000 entries and their details; closing the tab (W-150, 2026-11-15, release build): 247,976 KiB with a small folder, 302,640 KiB with the 500,000-entry tab in details, 290,948 KiB after closing it (42.9 MiB above the start; RssAnon 79,548 → 134,084 → 122,392 KiB). Reloading such a folder grew memory by about 25 MB each time until W-115 capped the allocator's arenas (2026-11-15).

### R-103 — Data safety
- **Priority:** must
- **Acceptance:**
  - [x] No code path overwrites or permanently deletes user data without an explicit user choice in that operation.
  - [x] Killing the process at any point during copy, move or batch rename leaves every source file either intact or fully present at its destination. (W-083 under D-020: a killed batch rename keeps every file's bytes intact under journal-recorded temporary names, and the next start restores the original names automatically; copy/move per W-036/W-072.)
  - [x] Configuration and place-group files are written atomically (write, sync, rename).
- **Validation:** test (fault injection, kill tests). W-071 (2026-10-22) demonstrates batch rename/undo recovery after SIGKILL, including recovery killed in turn; W-083 recovers interrupted renames at the next start (D-020), and W-134/W-135 harden the recovery record's parser. W-072 adds file/directory ordering and late-error fault tests. Settings, place groups and folder views that cannot be read, are damaged or come from a newer Rustphin are kept and never overwritten (W-140, W-143). Physical power loss remains unverified.

### R-104 — Robustness with untrusted input
- **Priority:** must
- **Acceptance:**
  - [x] Names with control characters, invalid UTF-8, or 255-byte length are displayed safely and are operable. (W-078: W-075b H2/H3.)
  - [x] Malformed images, places files and `.desktop` files produce an error state, never a crash. (W-078: H1, W-075b, W-075c H4; bounded fuzzing, not exhaustive.)
  - [x] Nothing is executed by browsing, previewing or thumbnailing. (W-078: W-075b H2.)
- **Validation:** test (fixtures and fuzzing of parsers)

### R-105 — Visual design
- **Priority:** must
- **Acceptance:**
  - [ ] A single dark theme as specified in `docs/DESIGN.md`: charcoal translucent surfaces (D-038) with compositor blur behind the window, royal-blue accent, 2-pixel corners, Inter / JetBrains Mono / Space Grotesk. (Tokens and bundled fonts in place; blur through `KWindowEffects` was confirmed on the maintainer's desktop with the Qt spike (W-114b2), not yet with the current build.)
  - [x] Floating surfaces inside the window (menus, palette, dialogs) are tinted opaque, since in-window blur is unavailable. (`rustphin_view::colors` test `floating_surfaces_stay_solid_in_the_translucent_theme` pins the `raised` surface opaque in the glass theme (W-078d, moved with the tokens in W-114c1); the QML menus, palette and dialogs paint with it. The look on the real desktop stays under the first R-105 criterion.)
  - [x] Where the compositor offers no blur protocol, or the user turns transparency off, the window is opaque and every text element still meets WCAG AA contrast (4.5:1). (Accepted by the maintainer on 2026-11-08 on token-level evidence: `opaque_theme_meets_contrast_minimums` checks every text/surface token pair used by the views, menus, palette and dialogs at 4.5:1, and W-078c found all colours come from those tokens; the opaque fallback was observed in isolated sessions (W-076h/j). No per-screen pixel audit was made.)
  - [ ] Text meets WCAG AA contrast over the blurred background in the worst case tested (a white window behind).
  - [x] Colours, spacing, radii and type sizes come from one set of design tokens. (`rustphin_view::colors` and `metrics` implement DESIGN.md and reach QML as the `Tokens` singleton (`crates/rustphin/src/tokens.rs`). The QML has no colour or font-size literals; the window's default size, a few 2 px spacings and borders, and the documented half opacity remain literals (grep of `crates/rustphin/qml`, 2026-11-14). The GPUI audit was W-078c.)
- **Validation:** test (token contrast), manual (visual review on the real desktop)

### R-106 — Progressive disclosure
- **Priority:** must
- **Acceptance:**
  - [x] The default window shows only: places panel, tab strip, location bar, the view, and a minimal status line.
  - [x] Secondary controls (view options, filters, split, operation details, properties) appear on hover, selection, shortcut or through the palette, and each is reachable in at most two steps. (W-082: filter bar on Ctrl+I, / and the palette, replay K17; J4 for the others; operation progress shows by itself while one runs.)
  - [x] Nothing available only through hover lacks a keyboard or palette path.
- **Validation:** manual (design checklist), ui-test (reachability)

### R-107 — Accessibility baseline
- **Priority:** should
- **Acceptance:**
  - [ ] Keyboard-complete operation (R-004) with a visible focus indicator everywhere. (Keyboard-only pass on Qt, W-076t, 2026-11-15: the window and the Settings, Properties, question, batch rename, Compress dialogs and the palette, K20, K21, Q1; found and fixed a pane without a visible focus, Shift+Tab going forward, the Settings page list out of Tab's reach, checkboxes in the desktop's colours. K22 (W-076u) added a keyboard path to the problems dialog and replayed it and the context menu; the View menu's entries all have keys. Open: the transfer dialog not seen keyboard-only, since copies on the test volume finish at once.)
  - [x] The entry list, places panel, tabs and dialogs expose names and roles to assistive technology (AT-SPI), with gaps listed. (Qt interface, W-114h: entries are list items by name and report "selected", toolbar buttons are named with a Press action, the places panel is a named panel with a "Places" list and a named width separator, a question is a dialog by its title with pressable buttons. Gaps: some buttons are unnamed (W-116b, W-117); tabs and the palette were not re-checked on Qt. The GPUI interface was checked through AccessKit in W-074a–i. The Orca pass was dropped by D-023.)
- **Validation:** manual

### R-108 — Privacy
- **Priority:** must
- **Acceptance:**
  - [x] The application makes no network connections of its own and collects no telemetry.
  - [x] All stored history is local, documented, and clearable.
- **Validation:** test (`deny.toml` bans HTTP clients, TLS, telemetry crates and, since the Qt switch (W-114i), the async-net/http crates GPUI brought; `cargo tree -i http` finds none), source review and traced runs showing only local sockets (W-077, on the GPUI interface), manual. Amended 2026-11-08 by D-018: previously "no network crates or sockets in the dependency audit", which GPUI could not meet; with GPUI gone the exception is no longer used.

### R-109 — Platform and build
- **Priority:** must
- **Acceptance:**
  - [x] Builds with the minimum Rust (1.100 since D-035) and current stable, Qt 6 (Base, Declarative, Wayland) and KF6 WindowSystem, from a clean checkout with one documented command (`rust-tc run`; checks with `rust-tc doctor`, which also checks the minimum version).
  - [ ] Runs on Plasma 6 Wayland; missing optional services (UDisks2, data-control, blur) degrade the related feature with a visible notice rather than failing to start. (UDisks2: the places panel says the devices could not be listed (`PlacesPanel::devices_failed`); data-control: the one-time clipboard notice (R-011). Blur: since W-146c the Qt interface uses the opaque theme when KWin offers no blur and says "Background blur unavailable; using the opaque theme." on a status line, and chooses the theme again whenever the settings change or a window becomes active (`settings::Look::theme`, unit-tested). Not ticked: the line and the switch were not seen on a compositor without blur (UI_SCENARIOS AP6).)
  - [x] One local command runs formatting, lints, the test suite and the headless benchmarks (`rust-tc doctor`, with QML format and lint since W-116a; `rust-tc bench-listing` for R-101). There is no hosted CI; the repository is on GitHub without one (D-037).
- **Validation:** test (local check command)

## Later (not in the MVP)

- L-001 — Search (file name and content, Baloo integration)
- L-002 — Embedded terminal panel
- L-003 — KDE service menus in the context menu
- L-004 — Version-control status overlays
- L-005 — Information panel
- L-006 — Miller columns view
- L-007 — Quick preview (Space)
- L-008 — Native network protocols (beyond kio-fuse paths)
- L-009 — Light theme and following the system colour scheme
- L-010 — Tags, ratings and comments
- L-011 — Archive browsing and compress/extract actions. Compress and extract done as R-021 (W-111, W-119); browsing inside an archive is not offered.
- L-012 — Packaging (Arch package, Flatpak)
- L-013 — Translations
- L-014 — Running operations with elevated privileges. Done as Act as Administrator through KDE's kio-admin helper (D-027, D-028, W-098; ADMIN_MODE.md); the real-desktop check W-098e is open.

## Non-goals

- A line-by-line port of Dolphin or of KDE Frameworks.
- X11, macOS and Windows.
- Imitating Breeze widgets.
- Any network service, account or telemetry.

## Traceability to the brief

| Brief item | Requirements |
|---|---|
| Core local workflow | R-001 to R-011, R-013, R-014, R-017, R-018 |
| Place groups | R-012 |
| Command palette and fuzzy jump | R-015 |
| Advanced batch rename | R-016 |
| Obsidian frosted glass, progressive disclosure | R-105, R-106 |
| Success signal: daily driver for two weeks | R-001 to R-018, R-103, R-104 |
| Success signal: 500,000 entries without stalls | R-101, R-102 |
