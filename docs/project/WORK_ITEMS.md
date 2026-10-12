# Work items (current slice)

## Current slice
S4 — File operations, undo, trash

**Done when:** the operation engine passes its fault-injection and kill tests (gate, before any operation UI), then every R-007, R-008, R-009 and R-103 acceptance point is demonstrated and recorded in VALIDATION.md.

Order matters: W-031 to W-036 are the engine and its gate; W-037 onwards (UI) may not start before W-036 is recorded as passed.

## Queue
- [ ] W-033 — Trash: freedesktop trash specification in the platform layer behind a core trait, with per-volume trash directories, restore and empty; engine operations trash and permanent delete (R-007, R-009)
- [ ] W-034 — Undo journal: rename, move, copy, create and trash undone in reverse order; refusal with an explanation when files changed (R-008)
- [ ] W-035 — Engine on a worker thread: commands in, progress, questions and results out over a channel (R-007, D-009)
- [ ] W-036 — Engine gate: fault injected at every filesystem step of copy and move, process-kill tests, property tests over generated trees; hand mutation checks (R-103)
- [ ] W-037 — UI: new folder, new file, rename in place, trash, permanent delete with confirmation, undo (R-007, R-008)
- [ ] W-038 — UI: copy and move within the application (copy/cut/paste and to the other pane), progress with speed, remaining count and cancel, conflict dialog (R-006, R-007)
- [ ] W-039 — UI: trash view with restore and empty trash (R-009)
- [ ] W-040 — UI scenarios for file operations; trash interoperability with Dolphin checked in an isolated session (R-009)

## Done (recent)
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
