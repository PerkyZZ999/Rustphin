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
  - [ ] Opening a directory lists every entry Dolphin lists for it, with hidden files off by default and a toggle to show them.
  - [ ] Back, forward and up work with mouse and keyboard; history is kept per tab.
  - [ ] A breadcrumb location bar switches to an editable path field; an invalid path shows an inline error and keeps the current view.
  - [x] Activating a file opens it with the system default application; executables and `.desktop` files ask before running.
  - [ ] An unreadable directory shows an inline error state, not a crash or an empty view.
- **Validation:** test (model), ui-test (navigation), manual (open-with)

### R-002 — Live updates
- **Priority:** must
- **Description:** The view reflects changes made by other programs.
- **Acceptance:**
  - [ ] Files created, renamed, changed or deleted outside Rustphin appear in the open view within 1 second without a manual refresh.
  - [ ] Selection and scroll position survive an update unless the selected entry was removed.
- **Validation:** test

### R-003 — View modes, sorting and zoom
- **Priority:** must
- **Description:** Icons, compact and details views, with sorting and zoom.
- **Acceptance:**
  - [ ] Three view modes are selectable per tab: icons, compact, details.
  - [ ] Details view has name, size, modified and type columns; clicking a header sorts by it and toggles direction.
  - [ ] Sorting is natural and case-insensitive by default ("file2" before "file10"), with folders first as an option, on by default.
  - [ ] Zoom changes icon size in all three modes.
  - [ ] View mode, sort and zoom are remembered between sessions.
- **Validation:** test (sort order), ui-test (modes)

### R-004 — Selection and keyboard operation
- **Priority:** must
- **Description:** Every MVP function is reachable without a mouse.
- **Acceptance:**
  - [ ] Click, Ctrl+click, Shift+click, rubber-band, select all and invert selection work.
  - [ ] Arrow keys, Home/End, Page Up/Down move focus; typing letters jumps to the next matching name.
  - [ ] Every action in R-001 to R-016 has a keyboard path, and the default shortcuts match Dolphin's where an equivalent exists.
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
  - [ ] Dragging files out to Dolphin and to at least one non-KDE application delivers them. (Dolphin: done. A non-KDE application: not yet tried.)
  - [x] Files dropped from other applications are accepted onto a view or a folder.
  - [x] Names with spaces and non-ASCII characters survive the round trip.
- **Validation:** ui-test (with Dolphin), manual (non-KDE app)

### R-011 — File clipboard
- **Priority:** must
- **Acceptance:**
  - [x] Copy and cut in Rustphin paste correctly in Dolphin; copy and cut in Dolphin paste correctly in Rustphin, with cut resulting in a move.
  - [x] Cut items are shown dimmed until pasted or the clipboard changes.
  - [x] Names with spaces and non-ASCII characters survive the round trip.
  - [ ] Where the compositor lacks a data-control protocol, copy and paste still work inside Rustphin and the limitation is reported once. (Implemented and unit-tested; no such compositor was available to try it on.)
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
- **Validation:** test (D-Bus), manual

### R-018 — Context menu and properties
- **Priority:** should
- **Acceptance:**
  - [x] Right-click offers the operations of R-007 to R-011 relevant to the selection, plus "Open with" listing applications registered for the MIME type.
  - [x] A properties view shows name, type, size (computed in the background for folders), times and permissions, and can change permissions.
- **Validation:** ui-test, manual

## MVP — non-functional

### R-101 — Responsiveness at scale
- **Priority:** must
- **Acceptance:** on the reference machine, in a directory of 500,000 entries on a local filesystem:
  - [ ] First entries are visible within 200 ms of opening (warm cache) and the UI accepts input throughout loading.
  - [ ] Re-sorting by any column completes within 500 ms once the sizes and dates it needs have been read. The first read has no fixed limit, but the list is shown in name order with a loading indicator while it runs, and details appear from the top of the list first. (Reworded 2026-10-12 with the user's agreement: on a cold disk cache the read is bounded by the disk, 5.3 s for 500,000 files.)
  - [ ] Scrolling holds 60 frames per second at the 95th percentile.
  - [ ] No UI-thread task exceeds 16 ms in listing, sorting, metadata or thumbnail code paths.
- **Validation:** bench (headless core), ui-test with frame timing

### R-102 — Memory
- **Priority:** should
- **Acceptance:**
  - [ ] Resident memory with one 500,000-entry directory open stays under 400 MB; closing the tab returns to within 50 MB of the starting figure.
- **Validation:** bench

### R-103 — Data safety
- **Priority:** must
- **Acceptance:**
  - [x] No code path overwrites or permanently deletes user data without an explicit user choice in that operation.
  - [ ] Killing the process at any point during copy, move or batch rename leaves every source file either intact or fully present at its destination.
  - [ ] Configuration and place-group files are written atomically (write, sync, rename).
- **Validation:** test (fault injection, kill tests)

### R-104 — Robustness with untrusted input
- **Priority:** must
- **Acceptance:**
  - [ ] Names with control characters, invalid UTF-8, or 255-byte length are displayed safely and are operable.
  - [ ] Malformed images, places files and `.desktop` files produce an error state, never a crash.
  - [ ] Nothing is executed by browsing, previewing or thumbnailing.
- **Validation:** test (fixtures and fuzzing of parsers)

### R-105 — Visual design
- **Priority:** must
- **Acceptance:**
  - [ ] A single dark theme as specified in `docs/DESIGN.md`: obsidian translucent surfaces with compositor blur behind the window, royal-blue accent, 2-pixel corners, Inter / JetBrains Mono / Space Grotesk.
  - [ ] Floating surfaces inside the window (menus, palette, dialogs) are tinted opaque, since in-window blur is unavailable.
  - [ ] Where the compositor offers no blur protocol, or the user turns transparency off, the window is opaque and every text element still meets WCAG AA contrast (4.5:1).
  - [ ] Text meets WCAG AA contrast over the blurred background in the worst case tested (a white window behind).
  - [ ] Colours, spacing, radii and type sizes come from one set of design tokens.
- **Validation:** test (token contrast), manual (visual review on the real desktop)

### R-106 — Progressive disclosure
- **Priority:** must
- **Acceptance:**
  - [x] The default window shows only: places panel, tab strip, location bar, the view, and a minimal status line.
  - [ ] Secondary controls (view options, filters, split, operation details, properties) appear on hover, selection, shortcut or through the palette, and each is reachable in at most two steps.
  - [x] Nothing available only through hover lacks a keyboard or palette path.
- **Validation:** manual (design checklist), ui-test (reachability)

### R-107 — Accessibility baseline
- **Priority:** should
- **Acceptance:**
  - [ ] Keyboard-complete operation (R-004) with a visible focus indicator everywhere.
  - [ ] The entry list, places panel, tabs and dialogs expose names and roles through AccessKit; a pass with the Orca screen reader is recorded, with gaps listed.
- **Validation:** manual

### R-108 — Privacy
- **Priority:** must
- **Acceptance:**
  - [ ] The application makes no network connections of its own and collects no telemetry.
  - [ ] All stored history is local, documented, and clearable.
- **Validation:** test (no network crates or sockets in the dependency audit), manual

### R-109 — Platform and build
- **Priority:** must
- **Acceptance:**
  - [ ] Builds with Rust 1.95 and current stable from a clean checkout with one documented command.
  - [ ] Runs on Plasma 6 Wayland; missing optional services (UDisks2, data-control, blur) degrade the related feature with a visible notice rather than failing to start.
  - [ ] One local command runs formatting, lints, the test suite and the headless benchmarks (the repository is local only; there is no hosted CI).
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
- L-011 — Archive browsing and compress/extract actions
- L-012 — Packaging (Arch package, Flatpak)
- L-013 — Translations
- L-014 — Running operations with elevated privileges

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
