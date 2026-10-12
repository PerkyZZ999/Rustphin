# Work items (current slice)

## Current slice
S2 — Views, sorting, selection

**Done when:** every S2 acceptance point in ROADMAP.md (R-003, R-004, the icon part of R-014, the sort and scroll parts of R-101, R-102) is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-019 — UI icons and compact views, with zoom in all three modes (R-003)
- [ ] W-020 — UI selection: click, Ctrl+click, Shift+click, rubber band, select all, invert; type-ahead to jump by name (R-004)
- [ ] W-021 — Persist view mode, sort and zoom between sessions: TOML under the XDG config directory, written atomically, read through the platform layer (R-003)
- [ ] W-013 — Frame-time measurement while scrolling and while a 500,000-entry listing streams in; memory with that folder open and after closing it (R-101, R-102)
- [ ] W-023 — Core: load details progressively, visible rows first, and publish as they arrive, so the first sort by size or date in a large folder with a cold inode cache stays within R-101 (measured 1.7 s for 500,000 files; see VALIDATION.md) (R-101)
- [ ] W-022 — Replayable UI scenario script for navigation and views in an isolated KWin session (R-001, R-003, R-004)

## Done (recent)
- [x] W-018 — UI details view: name, size, modified and type columns; clickable headings that sort and show direction with an arrow; system-theme icons and type descriptions resolved in the background; narrow windows drop the type then the date column; the list sits on a more solid surface than the glass chrome (R-003, R-014 icons)
- [x] W-017 — Platform: file types by name from the shared MIME database (suffix and whole-name patterns, type descriptions), and icon lookup per the Icon Theme specification (inheritance, size fit, `hicolor` fallback, cached); `icon_lookup` diagnostic example. Types are judged by name only, not content (R-014)
- [x] W-016 — Core selection model: select one, toggle, range from an anchor, select all, invert, clear; tracked by entry id so it survives re-sorting and filtering; emptied when the directory is read again (R-004)
- [x] W-015 — Core sorting by name, size, modified and type (by extension) with direction; folders stay first in both directions; re-sort reuses the listing; `rust-tc bench-listing` times each column against the 500 ms limit (R-003, R-101)
- [x] W-014 — Core details: size and modified time through the `Fs` trait, loaded off the UI thread only when the options need them, all entries in parallel after the listing. Deviation from the plan: not yet prioritised by visible rows (W-023) (R-003, R-101)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
