# Work items (current slice)

## Current slice
S5 — Clipboard and drag-and-drop

**Done when:** every R-010 and R-011 acceptance point is demonstrated and recorded in VALIDATION.md, or recorded as not verifiable with the reason.

## Queue
- None. S5 is closed; S6 is planned in ROADMAP.md and its work items are written when it starts.

## Done (recent)
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
