# Work items (current slice)

## Current slice
S3 — Live updates, tabs, split view

**Done when:** every S3 acceptance point in ROADMAP.md (R-002, R-005, R-006) is demonstrated and recorded in VALIDATION.md.

## Queue
- None. S3 is closed; S4 is planned in ROADMAP.md and its work items are written when it starts.

## Done (recent)
- [x] S3 — Live updates, tabs, split view: W-025 to W-030, closed 2026-10-12 (see VALIDATION.md)
- [x] W-030 — Tab, split and session scenarios (T1–T6, D1–D3, R1) added to UI_SCENARIOS.md and run (R-005, R-006)
- [x] W-029 — Session restore: tabs, split state, locations and the active tab saved to `$XDG_STATE_HOME/rustphin/session.toml` and restored at start (R-005)
- [x] W-028 — Split view: F3 splits a tab into two panes with their own locations and view settings; Tab moves keyboard focus between panes; an accent line marks the active pane (R-006)
- [x] W-027 — Tabs: open, close, switch and reorder by keyboard and mouse; middle-click a folder opens it in a background tab; the tab strip is hidden while there is one tab (R-005)
- [x] W-026 — Live updates applied by re-reading the directory quietly after a change and publishing only the finished result; the selection is carried over by name and the view does not scroll. Deviation: a full re-read, not an incremental patch of the store (R-002)
- [x] W-025 — Core filesystem watching through the `Fs` trait (inotify via `notify`): bursts coalesced, the watch follows navigation, a vanished directory gives way to its nearest existing ancestor (R-002)
- [x] S2 — Views, sorting, selection: W-013 to W-024, closed 2026-10-12 (see VALIDATION.md)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
