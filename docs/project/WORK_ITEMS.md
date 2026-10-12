# Work items (current slice)

## Current slice
S1 — Open and browse a folder

**Done when:** every S1 acceptance point in ROADMAP.md is demonstrated and recorded in VALIDATION.md.

## Queue
- [ ] W-001 — Create the Cargo workspace: `rustphin-core`, `rustphin-platform`, `rustphin-ui`, `rustphin` binary, `xtask`; edition 2024, `rust-version = "1.95"`, GPL-3.0-or-later, shared lints, `LICENSE`, `README` with the build command (R-109 / S1)
- [ ] W-002 — `xtask check`: format, clippy, tests, and the dependency-direction check (core and platform must not depend on `gpui-kit`) (R-109 / S1)
- [ ] W-003 — Core entry store: byte-arena names, parallel columns for kind; lossless handling of non-UTF-8 names with a safe display form (R-104 / S1)
- [ ] W-004 — Core lister: batched `read_dir` on an I/O worker, first batch published immediately, cancellation by generation (R-001, R-101 / S1)
- [ ] W-005 — Core sorter: natural case-insensitive comparison, folders first, hidden filter, on the CPU pool; index-based snapshot type (R-001, R-101 / S1)
- [ ] W-006 — Core directory model: open, up, back, forward, history; error states for unreadable and missing paths; snapshot and event channel (R-001 / S1)
- [ ] W-007 — Headless benchmark (`xtask bench listing`): generate 500,000 entries, measure time to first batch and to full sorted snapshot, assert the S1 thresholds (R-101 / S1)
- [ ] W-008 — Platform blur trait: `ext-background-effect-v1` implementation ported from the spike with runtime detection and a "none" fallback (R-105 / S1)
- [ ] W-009 — UI shell on `gpui-kit` `=0.7.3`: application start, window, theme tokens (translucent and opaque modes), bridge from core channels to an entity (R-105, R-109 / S1)
- [ ] W-010 — UI item list (names only, virtualised) with keyboard and mouse navigation, breadcrumb/location field with inline errors, hidden-files toggle (R-001 / S1)
- [ ] W-011 — UI-test scenario in an isolated KWin session with isolated home: open, navigate, error state, large-directory input responsiveness (R-001, R-101 / S1)
- [ ] W-012 — Record S1 evidence in VALIDATION.md; manual visual review on the real desktop (blur and opaque modes) (R-105 / S1)

## Done (recent)
- None yet
