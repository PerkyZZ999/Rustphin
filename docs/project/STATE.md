# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S10 done; S11 in progress
- **Status:** active — S11 safety hardening
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-10-22

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-10-22 — W-072c journal namespace ordering; 401 tests

## Open loops
- Task: an 80–90 ms frame gap at start-up while a 500,000-entry folder loads
- Product: nothing is exposed over AT-SPI in test sessions; affects R-107, to resolve in S11
- Task: reordering tabs by mouse drag is implemented but was not exercised in a test session
- Product: view settings are one global file; a pane's own view settings last for the session only
- Task: file operations not yet seen on screen: the failure dialog, stopping with the stop button, rename in the icons and compact views, the speed display, another volume's trash
- Task: properties opened through D-Bus label the type generically as File; folder measurement cancellation, unreadable-folder reporting and small-window dialog layout are not yet observed on screen
- Task: copied file data/attributes (W-070), copied directories and cross-filesystem destination publication before source removal (W-072b) are synced. Whole-source removal sync errors are reported; partial source-tree deletion, restart cleanup and physical power loss remain unproved
- Task: new batch renames and undo have explicit, journal-validated recovery (W-071). A kill can still leave staged names until recovery is requested; older unjournaled temporary files are not guessed at. R-103's literal immediate-after-kill criterion and physical power-loss validation remain open; journal namespace barriers are now in place (W-072c)
- Product: an operation that replaced or deleted something cannot be undone at all, even its reversible parts
- Manual: drag a file from Rustphin into an application that is not a KDE one (R-010); needs the user's desktop
- Task: in isolated test sessions the window was see-through without blur (other windows showed behind it unblurred). To check: whether the compositor accepts the blur request while its blur effect is off, and whether the opaque theme should be used then
- Task: not seen on screen in S5: dropping on a tab, drags in the icons and compact views, copy (not cut) between Rustphin and Dolphin, the clipboard fallback notice
- Manual: mount, unmount and safely remove a real USB drive from the places panel, and unmount one in use to see the reason (R-013); needs the user
- Task: places panel not yet seen on screen: hiding a place, reordering by keyboard, an edit made in Dolphin other than adding; its width is fixed and its visibility is not remembered
- Task: in a very narrow split the 88 px size column remains, so entry names can clip away
- Task: thumbnails not yet observed: reuse of a thumbnail Dolphin made and the reverse on screen (the cache format is unit-tested against the specification's example); JPEG, GIF and WebP files; scrolling a large picture folder
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- None. The maintainer approved the three individually documented maintenance-notice exceptions on 2026-10-18; the complete baseline gate passes.

## Next action
- Continue W-073: re-measure listing/sorting, startup frame gaps, scroll percentiles and RSS/after-tab-close on 500,000 entries. Fix measured UI-thread stalls, then proceed through the remaining S11 queue with `rust-tc doctor`, evidence updates and a local commit after every coherent task.

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
