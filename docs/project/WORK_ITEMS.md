# Work items (current slice)

## Current slice
S1 — Open and browse a folder

**Done when:** every S1 acceptance point in ROADMAP.md is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-012 — Manual visual review on the real desktop by the user: blur behind the window, and the opaque mode with `RUSTPHIN_OPAQUE=1` (R-105 / S1)
- [ ] W-013 — Frame-time measurement while scrolling and while a 500,000-entry listing streams in (R-101; carried into S2 where the 60 fps target is validated)

## Done (recent)
- [x] W-001 — Cargo workspace: `rustphin-core`, `rustphin-platform`, `rustphin-ui`, `rustphin` binary; edition 2024, `rust-version = "1.95"`, GPL-3.0-or-later, shared lints, `LICENSE`, `README.md` (R-109)
- [x] W-002 — Quality gate. Changed from the planned `xtask check` to the user's Rust-Toolchain: `rust-tc quick` / `rust-tc doctor`, with project recipes `layers` (dependency direction), `msrv` (Rust 1.95 build), `bench-listing` and `run`; `deny.toml` reviewed (R-109)
- [x] W-003 — Core entry store: chunked byte-arena names, kinds, container flags; lossless non-UTF-8 names with a safe display form (R-104)
- [x] W-004 — Core lister: streamed `read_dir` on a worker thread, snapshots at doubling sizes, cancellation by generation, behind the `Fs` trait (R-001, R-101)
- [x] W-005 — Core sorter: natural case-insensitive comparison, folders first, hidden filter, on the core's CPU pool; index-based snapshots (R-001, R-101)
- [x] W-006 — Core directory model: open, up, back, forward, reload, history, options; failed opens restore the previous location; events over a channel (R-001)
- [x] W-007 — Headless benchmark `rust-tc bench-listing` through the real model on 500,000 files, asserting the S1 thresholds (R-101)
- [x] W-008 — Platform blur trait: `WaylandBlur` (`ext-background-effect-v1`) with runtime detection and the `NoBlur` fallback (R-105)
- [x] W-009 — UI shell on `gpui-kit` `=0.7.3`: window, design tokens with contrast tests, translucent and opaque modes, bridge from core events to the view (R-105, R-109)
- [x] W-010 — UI entry list (virtualised, names and folder/file icons), keyboard and mouse navigation, breadcrumb location bar with editable path field and inline errors, hidden-files toggle, empty and loading states, status line (R-001)
- [x] W-011 — UI scenarios run by hand through kwin-mcp in an isolated KWin session with an isolated home; three defects found and fixed. Not yet a replayable script (R-001, R-101)
