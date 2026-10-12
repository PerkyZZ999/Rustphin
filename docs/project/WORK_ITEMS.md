# Work items (current slice)

## Current slice
S3 — Live updates, tabs, split view

**Done when:** every S3 acceptance point in ROADMAP.md (R-002, R-005, R-006) is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-027 — UI tabs: open, close, reorder and switch by mouse and keyboard; middle-click a folder to open it in a new tab; one browser state per tab (R-005)
- [ ] W-028 — UI split view: two panes in a tab with independent locations and view settings; keyboard focus moves between panes (R-006)
- [ ] W-029 — Session restore: open tabs, split state and locations saved to the XDG state directory and restored at start (R-005)
- [ ] W-030 — Extend `docs/project/UI_SCENARIOS.md` with live-update, tab and split scenarios and run them (R-002, R-005, R-006)

## Done (recent)
- [x] W-026 — Live updates applied by re-reading the directory quietly after a change and publishing only the finished result; the selection is carried over by name and the view does not scroll. Deviation: a full re-read, not an incremental patch of the store (R-002)
- [x] W-025 — Core filesystem watching through the `Fs` trait (inotify via `notify`): bursts coalesced, the watch follows navigation, a vanished directory gives way to its nearest existing ancestor (R-002)
- [x] S2 — Views, sorting, selection: W-013 to W-024, closed 2026-10-12 (see VALIDATION.md)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
