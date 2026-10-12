# Work items (current slice)

## Current slice
S2 — Views, sorting, selection

**Done when:** every S2 acceptance point in ROADMAP.md (R-003, R-004, the icon part of R-014, the sort and scroll parts of R-101, R-102) is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-014 — Core metadata: size and modified time columns in the entry store, fetched lazily off the UI thread (visible rows first, everything when the sort key needs it), through the `Fs` trait (R-003, R-101)
- [ ] W-015 — Core sorting by name, size, modified and type with direction; re-sort without re-listing; benchmark each key on 500,000 entries against the 500 ms limit (R-003, R-101)
- [ ] W-016 — Core selection model: single, toggle, range, select all, invert; kept by entry identity across snapshots (R-004)
- [ ] W-017 — Platform MIME detection and icon-theme lookup (system theme, by MIME type) with a cache; fallbacks when a theme lacks an icon (R-014)
- [ ] W-018 — UI details view: name, size, modified, type columns; sortable headers showing direction by shape as well as colour (R-003)
- [ ] W-019 — UI icons and compact views, with zoom in all three modes (R-003)
- [ ] W-020 — UI selection: click, Ctrl+click, Shift+click, rubber band, select all, invert; type-ahead to jump by name (R-004)
- [ ] W-021 — Persist view mode, sort and zoom between sessions: TOML under the XDG config directory, written atomically, read through the platform layer (R-003)
- [ ] W-013 — Frame-time measurement while scrolling and while a 500,000-entry listing streams in; memory with that folder open and after closing it (R-101, R-102)
- [ ] W-022 — Replayable UI scenario script for navigation and views in an isolated KWin session (R-001, R-003, R-004)

## Done (recent)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
