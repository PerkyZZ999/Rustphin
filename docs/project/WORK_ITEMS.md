# Work items (current slice)

## Current slice
S3 — Live updates, tabs, split view

**Done when:** every S3 acceptance point in ROADMAP.md (R-002, R-005, R-006) is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-025 — Core filesystem watching: changes made by other programs reach the open listing within 1 second without re-reading the whole directory; events coalesced; behind a trait so tests can inject events (R-002)
- [ ] W-026 — Core incremental update: apply created, removed, renamed and changed entries to the store and re-sort; selection and scroll position survive unless the selected entry was removed. Entry ids must stay stable across an update, which the current chunked store does not yet allow for removals (R-002)
- [ ] W-027 — UI tabs: open, close, reorder and switch by mouse and keyboard; middle-click a folder to open it in a new tab; one browser state per tab (R-005)
- [ ] W-028 — UI split view: two panes in a tab with independent locations and view settings; keyboard focus moves between panes (R-006)
- [ ] W-029 — Session restore: open tabs, split state and locations saved to the XDG state directory and restored at start (R-005)
- [ ] W-030 — Extend `docs/project/UI_SCENARIOS.md` with live-update, tab and split scenarios and run them (R-002, R-005, R-006)

## Done (recent)
- [x] S2 — Views, sorting, selection: W-013 to W-024, closed 2026-10-12 (see VALIDATION.md)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
