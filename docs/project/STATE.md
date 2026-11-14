# Project state

- **Name:** Rustphin
- **Version:** 0.1.0 (workspace `Cargo.toml`)
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S10 done; S11 (hardening) in progress; the interface is Qt 6 and QML through cxx-qt since W-114i (D-033, D-034)
- **Status:** active — W-114 (the switch from GPUI), W-116 (Qt quality pass) and W-118–W-144 (refactoring into `rustphin-view`, libarchive, byte-safe paths, test audits and their fixes, the GitHub repository) W-147 (charcoal colours, S12), W-148 (drive cards and the drive menu, S12) and W-149 (transfer details, S12) done 2026-11-14; open: W-117, W-115, the Qt keyboard pass, the blur notice, and the maintainer's real-desktop checks
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-11-14

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (pushing to the GitHub repository, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-11-14 — W-149 Transfer details
- Acceptance: ACCEPTANCE.md lists every open criterion and who can close it (re-mapped 2026-11-14 for the Qt interface).

## Open loops
- Task: W-117, the Qt leftovers W-116b saw (one-tab strip vs T5, Ctrl+0 zoom default, Trash place highlight, notice icons, recovery wording, Run prompt default styling, unnamed AT-SPI buttons, a link's Properties icon, J7), plus Settings › Appearance's Look and Title bar choices, which the Qt interface stores but ignores (R-019 unticked on that).
- Task: W-115, memory grows by about 25 MB on each reload of a 500,000-entry folder (below the interface).
- Task: keyboard-only pass of the Qt interface with Shift+Tab and visible focus on every surface, Settings controls included (R-004, R-107, R-019; W-076). The GPUI focus blocker W-076e no longer applies.
- Task: without blur the Qt interface is opaque from the start but says nothing, and does not follow blur appearing or going while it runs; DESIGN.md's "Background blur unavailable" line is not built (R-109 unticked on that; W-076).
- Task: R-101 on Qt: first entries, re-sort and details-view scrolling re-measured (W-114d, W-114h); icons and compact scrolling, the closing-tab memory figure of R-102 and per-task times on the Qt thread were not; nothing measured on a physical display.
- Task: administrator mode in the Qt interface is one state for the whole process (every window), the title is not marked, and batch rename is offered there although `AdminFs` has no directory lock (ADMIN_MODE.md "Differences from the design"); D-028 says per window.
- Task: `rust-tc fuzz` could not run after W-129 and W-133: the installed nightly (1.97) is below the 1.100 minimum. The fuzz harness checks in `rust-tc doctor` pass.
- Task: remote folders (W-080–W-080c) not tried against a real sftp or smb server, nor replayed on Qt (no kio-fuse mount in the test session).
- Task: compact view on Qt not seen sideways-scrolling with many entries or with a real wheel; drops on a tab and onto a place from another application not seen.
- Task: thumbnails not yet observed: reuse of a thumbnail Dolphin made and the reverse on screen (the cache format is unit-tested against the specification's example); JPEG, GIF and WebP files; scrolling a large picture folder.
- Task: Properties' folder measurement with unreadable entries, and its cancellation, not seen on screen; another volume's trash never exercised (real volumes are not test data).
- Task: data safety beyond the tests: physical power loss, partial source-tree deletion after a cross-filesystem move, and restart cleanup remain unproved (W-070, W-072).
- Product: an operation that replaced or deleted something cannot be undone at all, even its reversible parts (`UndoError::Replaced`).
- Product: Ctrl+F3 is bound but KWin takes it by default, as for Dolphin; redo is not offered (KEYBOARD.md).
- Manual (the maintainer): blur look of the current build and contrast over a white window (R-105); a real USB drive mounted, unmounted, safely removed and kept busy (R-013); focus from a real activation token, from the panel and from Chrome's "Show in folder" (R-020, W-120); "Also start it when you log in" after a real login; KDE's portal folder dialog for Extract to…; administrator mode with the real helper (W-098e).
- Open items carried forward from Discovery are listed in DISCOVERY.md.

## Blockers
- None for the agent. The real-desktop and hardware checks above wait on the maintainer.

## Next action
- Slice S12 (ROADMAP.md): the charcoal colour scheme (W-147, D-038), the drive cards (W-148, D-039) and the transfer details (W-149) are done; next the UI completion items (W-146) and the Qt leftovers (W-117). Change `docs/DESIGN.md` first, then the tokens in `crates/rustphin-view/src/colors.rs` and `metrics.rs`, with their contrast tests.
- Also open: W-115 (memory on reloading a very large folder) and the Qt keyboard pass (W-076).
- For the maintainer on the real desktop: after installing a build, run `rustphin --quit` once so the new build answers launches; then the checks under "Manual" above.
- Then: the two-week daily-driver trial, and phase 7 (release).

## Phase checklist
- [x] 0 Intake
- [x] 1 Idea
- [x] 2 Discovery
- [x] 3 Spec
- [x] 4 Decision lock
- [x] 5 Architecture
- [ ] 6 Implement
- [ ] 7 Release
- [ ] 8 Observe
