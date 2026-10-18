# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S7 done; S8 in progress
- **Status:** active — S8 in progress
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-10-18

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-10-18 — W-057 command palette and persistent history

## Open loops
- Task: an 80–90 ms frame gap at start-up while a 500,000-entry folder loads
- Product: nothing is exposed over AT-SPI in test sessions; affects R-107, to resolve in S11
- Task: reordering tabs by mouse drag is implemented but was not exercised in a test session
- Product: view settings are one global file; a pane's own view settings last for the session only
- Task: file operations not yet seen on screen: the failure dialog, stopping with the stop button, rename in the icons and compact views, new file, the speed display, another volume's trash
- Task: copied data is not synced to disk, so power loss during a copy is not covered (killing the process is)
- Product: an operation that replaced or deleted something cannot be undone at all, even its reversible parts
- Manual: drag a file from Rustphin into an application that is not a KDE one (R-010); needs the user's desktop
- Task: in isolated test sessions the window was see-through without blur (other windows showed behind it unblurred). To check: whether the compositor accepts the blur request while its blur effect is off, and whether the opaque theme should be used then
- Task: not seen on screen in S5: dropping on a tab, drags in the icons and compact views, copy (not cut) between Rustphin and Dolphin, the clipboard fallback notice
- Manual: mount, unmount and safely remove a real USB drive from the places panel, and unmount one in use to see the reason (R-013); needs the user
- Task: places panel not yet seen on screen: hiding a place, reordering by keyboard, an edit made in Dolphin other than adding; its width is fixed and its visibility is not remembered
- Task: thumbnails not yet observed: reuse of a thumbnail Dolphin made and the reverse on screen (the cache format is unit-tested against the specification's example); JPEG, GIF and WebP files; scrolling a large picture folder
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- None. The maintainer approved the three individually documented maintenance-notice exceptions on 2026-10-18; the complete baseline gate passes.

## Next action
- Implement W-058 (progressive disclosure) and W-059 (complete S8 reachability and 10,000-location UI latency checks). Verify UI changes through kwin-mcp in an isolated home; use `rust-tc` for every check and commit after every task.

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
