# Work items (current slice)

## Current slice
S11 — Hardening for daily use (in progress)

**Done when:** the S11 roadmap checks pass with evidence: contrast, keyboard operation and Orca, parser robustness, dependency licences and privacy. Outstanding safety and performance requirements remain visible before the two-week daily-driver trial.

## Queue
- [ ] W-070 — Sync a copied regular file before publishing it. Expose the sync through `Fs`, propagate failures through the existing operation error path, and prove that failed syncs preserve sources and existing destinations with regression and mutation checks. This is a durability prerequisite, not proof of power-loss safety (R-007, R-103, D-011).
- [ ] W-071 — Make interrupted batch renames recoverable without guessing which temporary files belong to them. Specify recovery and collision behaviour, then cover each mutation boundary with faults and process-kill checks before closing R-103's batch-rename gap.
- [ ] W-072 — Audit namespace durability for cross-filesystem moves and atomic configuration writes: directory sync/order, delayed errors, source removal and restart recovery. Verify the supported contract; keep physical power-loss claims distinct from kill/fault tests (R-103).
- [ ] W-073 — Re-measure R-101/R-102 with 500,000 entries, isolate the startup frame gap, and fix measured UI-thread stalls. Record headless timings, frame percentiles, resident memory and memory after tab closure.
- [ ] W-074 — Resolve missing AccessKit exposure in the pinned toolkit, then run the Orca and visible-focus checks. Keep toolkit locks and runtime fallbacks; raise a blocked locked decision instead of working around it (R-004, R-107).
- [ ] W-075 — Exercise untrusted names and existing places, desktop-entry, URI and image parsers with malformed fixtures and bounded parser fuzzing. Demonstrate no execution on browse/preview. Add only meaningful parser targets through `rust-tc` (R-104).
- [ ] W-076 — Replay keyboard-only operation, narrow layouts, operation failure/cancel paths and missing optional services; verify opaque contrast and complete the real-desktop blur/white-background review when authorised (R-004, R-105, R-106, R-109).
- [ ] W-077 — Audit application-owned network use, telemetry, local state/clear controls and dependency licences; verify a clean build on minimum/current Rust and document the build/privacy contracts (R-108, R-109).
- [ ] W-078 — Reconcile every remaining acceptance checkbox with evidence, replay outstanding interop/device checks where safe, and record unresolved manual checks before proposing the daily-driver trial. No scope cuts or release-readiness claims without the required evidence.

## Done (recent)
- [x] W-069 — Assessed the remaining MVP gates and ordered S11 around data preservation, performance, accessibility, parser robustness, visual/keyboard checks and privacy. Earlier slice closure is not daily-driver acceptance; see the 2026-10-20 evidence entry.
- [x] W-068 — Desktop-integration scenarios I1–I13 replayed in an isolated KWin session/home; default folder dispatch, external ShowItems, menus, Open with, run confirmation and editable properties observed. S10 closed; limits in VALIDATION.md.
- [x] W-067 — Properties: name, type, size, times and permissions, with folder size computed off the UI thread, and a way to change permissions (R-018, D-017).
- [x] W-066 — Context menu for the selection, including Open with the applications registered for the type (R-018, R-001). Activating a file still confirms before running an executable.
- [x] W-065 — `org.freedesktop.FileManager1` ShowItems opens the folder and selects the item, tested on a private bus (R-017).
- [x] W-064 — Desktop entry and command-line paths and `file://` URLs open as tabs (R-017).
- [x] W-063 — Record and replay the rename scenario in an isolated KWin session/home, and close S9 only against the observed evidence (R-016).
- [x] W-062 — Rename dialog: live preview, a written conflict, apply and cancel. Update DESIGN.md before the dialog (R-016, D-017).
- [x] W-061 — Apply one plan through the operation engine as a single all-or-nothing undo step, with fault injection (R-016, R-008, D-011).
- [x] W-060 — Core batch-rename plan: find/replace, a counter pattern and case changes, with a preview and conflict flags, and no filesystem writes (R-016).
- [x] W-059 — Record palette and reachability scenarios, replay them with the relevant existing scenarios in an isolated KWin session/home, measure palette latency, and close S8 only against the observed evidence (R-015, R-106).
- [x] W-058 — Progressive disclosure: simplify the default window, keep secondary controls reachable within two steps, and provide a keyboard or palette path for every hover control. Update DESIGN.md before its implementation (R-106, D-017).
- [x] W-057 — Palette UI: one shortcut, search actions/places/tabs/recent folders, visible shortcuts, keyboard and mouse activation, safe focus handling and empty state. Persist and clear recent folders through platform config I/O off the UI thread (R-015).
- [x] W-056 — Core fuzzy matching and bounded, deduplicated recent-location history, preserving raw path bytes. Behaviour tests, mutation checks and a `rust-tc` benchmark on 10,000 locations (R-015, R-104).
- [x] W-055 — Restored the baseline quality gate with three individually documented maintenance-notice exceptions approved by the maintainer on 2026-10-18; `rust-tc doctor` PASS, 291 tests. Toolkit and dependency versions unchanged.
- [x] S7 — Thumbnails: W-052 to W-054, closed 2026-10-12 (see VALIDATION.md)
- [x] W-054 — Scenario H1 for thumbnails added to UI_SCENARIOS.md and run (R-014)
- [x] W-053 — Views show thumbnails in place of the type icon for PNG, JPEG, GIF, WebP and BMP files when icons are 32 px or larger; only entries being drawn are asked for; leaving a folder drops what it was waiting for; a folder that is read again has its pictures checked again (R-014)
- [x] W-052 — `rustphin_platform::thumbnails`: the freedesktop thumbnail cache (names, size classes, text chunks, private permissions, failure markers) and a background thumbnailer that serves the newest request first and drops the oldest beyond 64; decoding bounded in file size, dimensions and memory, with decoder panics contained (R-014, R-104)
- [x] S6 — Places, place groups, devices: W-046 to W-051, closed 2026-10-12; mounting and unmounting were not exercised on real hardware (see VALIDATION.md)
- [x] W-051 — Scenario V5 for the devices section added to UI_SCENARIOS.md and run; nothing was mounted or unmounted on the machine's own drives (R-013)
- [x] W-050 — Devices in the places panel: name, a usage bar with free and total space for mounted volumes, mount on activation, unmount or safely remove (button or E), failures reported with their reason; the system and home volumes cannot be unmounted from the panel (R-013)
- [x] W-049 — `rustphin_platform::devices`: volumes over UDisks2 on the system bus (list, mount, unmount, power off, change signals), tested against a mock service over a private connection (R-013, D-013)
- [x] W-048 — Places panel: shows the shared places and the groups; open, open in a new tab, add (Ctrl+D, button, or drag a folder in), rename (F2), remove or hide (Delete, H), reorder (Alt+Up/Down), collapse (Left/Right), move between groups (G, or drag), new group (Ctrl+Shift+G); follows edits other applications make; F9 shows and hides it; Tab moves between it and the panes. Scenarios K1–K7 run with Dolphin (R-012)
- [x] W-047 — Place groups in `place-groups.toml`: name, order, collapsed state, member identifiers; a place is in at most one group; deleting a group keeps its places; vanished places are forgotten only after a successful read of the shared file (R-012, D-008)
- [x] W-046 — `rustphin_platform::places`: reads KDE's `user-places.xbel` and edits it in place (add, rename, hide, remove, reorder), changing only the bytes concerned and refusing a result that does not parse (R-012, D-008)
- [x] S5 — Clipboard and drag-and-drop: W-041 to W-045, closed 2026-10-12 with two points not verifiable in tests (see VALIDATION.md)
- [x] W-045 — Clipboard and drag scenarios C1–C3 and G1–G6 added to UI_SCENARIOS.md and run with Dolphin in an isolated session (R-010, R-011)
- [x] W-044 — Entries can be dragged out to other applications, and files dropped from other applications are accepted onto a view or a folder (R-010)
- [x] W-043 — Drag within and between panes and onto tabs: Ctrl copies, Shift moves, otherwise a question; icon and name are the drag handle in the details view, the whole cell in the grids; the rest of a row still starts a rubber band (R-010)
- [x] W-042 — Copy, cut and paste go through the system clipboard; cut entries are drawn faded until pasted or the clipboard changes; a desktop without a shared clipboard is mentioned once (R-011)
- [x] W-041 — Platform file clipboard behind a trait: data-control through `wl-clipboard-rs` with change notifications, an in-application fallback, `text/uri-list` plus KDE's and GNOME's formats; `file://` URI encoding for any byte sequence (R-011, D-010, D-015)
- [x] S4 — File operations, undo, trash: W-031 to W-040, closed 2026-10-12 (see VALIDATION.md)
- [x] W-040 — File-operation scenarios F1–F9 and trash scenarios X1–X4 added to UI_SCENARIOS.md and run; trash interoperability with Dolphin and KIO seen in both directions in an isolated session (R-009)
- [x] W-039 — Trash view: a toolbar button opens the home trash; a banner offers Restore for the selection, Empty trash with confirmation, and the trashes of other volumes; Delete inside the trash deletes for good after confirmation; the engine gained a restore request (R-009)
- [x] W-038 — UI: copy, cut and paste within the application (Ctrl+C, Ctrl+X, Ctrl+V; works across panes and tabs), a progress line with counts, speed and a stop button, the conflict dialog (skip, replace or merge, keep both, apply to all) and the failure dialog (skip, try again, stop). The system clipboard and drag-and-drop are S5 (R-006, R-007)
- [x] W-037 — UI: new folder (F10, Ctrl+Shift+N, toolbar button) and new file (Ctrl+Alt+N) created with a free name and renamed in place; rename in place (F2); trash (Delete); permanent delete with confirmation (Shift+Delete); undo (Ctrl+Z, and a button on the message of the last operation) (R-007, R-008)
- [x] W-036 — Engine gate passed 2026-10-12: faults at every filesystem step of copy and move, process-kill test, property test over random operation sequences, hand mutation checks (see VALIDATION.md). Operation UI may now be built (R-103)
- [x] W-035 — `ops::Operations`: each operation on its own thread; start, cancel, answer conflicts and failures; progress at most every 50 ms; events over a channel; owns the undo history (R-007, D-009)
- [x] W-034 — Undo: `ops::undo` and `ops::History` revert create, rename, move, copy (by trashing the copies) and trash, newest first, after checking every affected entry; refusals name the entry and the reason (R-008)
- [x] W-033 — Trash: freedesktop.org trash in `rustphin-platform` behind the core's `Trash` trait (home trash, per-volume `.Trash/$uid` and `.Trash-$uid`, restore, listing); engine requests `Trash` and `Delete` (permanent, recursive, never through links). Emptying the trash and finding mounted volumes' trashes come with the trash view, W-039 (R-007, R-009)
- [x] W-032 — Core operation engine (`rustphin_core::ops`): create folder and file, rename, copy and move through a temporary name; a move between filesystems removes a source only when its copy is in place and the source unchanged; conflicts (skip, overwrite, keep both, apply to all; folders merge), cancel, per-entry skip or retry, progress (R-007, D-011)
- [x] W-031 — Mutation primitives on the `Fs` trait (exclusive create, no-replace rename via `renameat2`, copy by reflink / `copy_file_range` / read-write, remove, attributes) for the real filesystem, and a fault-injecting wrapper for tests (R-007, R-103)
- [x] S3 — Live updates, tabs, split view: W-025 to W-030, closed 2026-10-12 (see VALIDATION.md)
- [x] W-030 — Tab, split and session scenarios (T1–T6, D1–D3, R1) added to UI_SCENARIOS.md and run (R-005, R-006)
- [x] W-029 — Session restore: tabs, split state, locations and the active tab saved to `$XDG_STATE_HOME/rustphin/session.toml` and restored at start (R-005)
- [x] W-028 — Split view: F3 splits a tab into two panes with their own locations and view settings; Tab moves keyboard focus between panes; an accent line marks the active pane (R-006)
- [x] W-027 — Tabs: open, close, switch and reorder by keyboard and mouse; middle-click a folder opens it in a background tab; the tab strip is hidden while there is one tab (R-005)
- [x] W-026 — Live updates applied by re-reading the directory quietly after a change and publishing only the finished result; the selection is carried over by name and the view does not scroll. Deviation: a full re-read, not an incremental patch of the store (R-002)
- [x] W-025 — Core filesystem watching through the `Fs` trait (inotify via `notify`): bursts coalesced, the watch follows navigation, a vanished directory gives way to its nearest existing ancestor (R-002)
- [x] S2 — Views, sorting, selection: W-013 to W-024, closed 2026-10-12 (see VALIDATION.md)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
