# Work items (current slice)

## Current slice
S2 — Views, sorting, selection

**Done when:** every S2 acceptance point in ROADMAP.md (R-003, R-004, the icon part of R-014, the sort and scroll parts of R-101, R-102) is demonstrated and recorded in VALIDATION.md.

## Queue

## Done (recent)
- [x] W-024 — Rubber-band selection by dragging over the list or grid, with Ctrl to add to the selection; band geometry in the tested layout module (R-004)
- [x] W-023 — Core: details are read from the top of the list downward in batches and published as they arrive; rows keep name order until a size or date sort has every value. Top-first follows the display order, not the actual scroll position (R-101)
- [x] W-013 — Frame timing and memory on 500,000 entries, measured with an opt-in frame log (`RUSTPHIN_FRAME_LOG`) and a self-driven scroll (`RUSTPHIN_SCROLL_BENCH`) (R-101, R-102)
- [x] W-022 — Replayable UI scenarios in `docs/project/UI_SCENARIOS.md` with a fixture script (`tests/ui/make-fixture.sh`); run by an agent through kwin-mcp in an isolated session. They are instructions for an agent, not an unattended script (R-001, R-003, R-004)
- [x] W-021 — View mode, zoom, sort, direction, hidden-files and folders-first settings saved as TOML in `$XDG_CONFIG_HOME/rustphin/settings.toml`, written atomically on a background thread and restored at start; unreadable files fall back to defaults; files from a newer version are not overwritten (R-003)
- [x] W-019 — UI icons and compact views as virtualised grids, five zoom levels in all three modes (Ctrl+=, Ctrl+-, Ctrl+0), view-mode buttons and Ctrl+1/2/3, arrow keys moving by row and column in grids (R-003)
- [x] W-020 — UI multi-selection on the core `Selection`: click, Ctrl+click, Shift+click, Shift+arrows/Home/End/Page keys, Ctrl+A, Ctrl+Shift+A (invert), Escape to clear, type-ahead by name, selected count in the status line. Rubber band moved to W-024 (R-004)
- [x] W-018 — UI details view: name, size, modified and type columns; clickable headings that sort and show direction with an arrow; system-theme icons and type descriptions resolved in the background; narrow windows drop the type then the date column; the list sits on a more solid surface than the glass chrome (R-003, R-014 icons)
- [x] W-017 — Platform: file types by name from the shared MIME database (suffix and whole-name patterns, type descriptions), and icon lookup per the Icon Theme specification (inheritance, size fit, `hicolor` fallback, cached); `icon_lookup` diagnostic example. Types are judged by name only, not content (R-014)
- [x] W-016 — Core selection model: select one, toggle, range from an anchor, select all, invert, clear; tracked by entry id so it survives re-sorting and filtering; emptied when the directory is read again (R-004)
- [x] W-015 — Core sorting by name, size, modified and type (by extension) with direction; folders stay first in both directions; re-sort reuses the listing; `rust-tc bench-listing` times each column against the 500 ms limit (R-003, R-101)
- [x] W-014 — Core details: size and modified time through the `Fs` trait, loaded off the UI thread only when the options need them, all entries in parallel after the listing. Deviation from the plan: not yet prioritised by visible rows (W-023) (R-003, R-101)
- [x] S1 — Open and browse a folder: W-001 to W-012, closed 2026-10-12 (see VALIDATION.md)
