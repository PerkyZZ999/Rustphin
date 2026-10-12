# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S2 in progress (W-014 to W-018 done)
- **Status:** active — details view built; awaiting the user's look at it on the real desktop
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-10-12

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-10-12 — W-018

## Open loops
- Task: W-023 — first details load on a cold inode cache took 1.7 s for 500,000 files, over R-101's 500 ms
- Task: W-013 — frame times and memory unmeasured (in the S2 queue)
- Task: UI scenarios are hand-run, not a replayable script (W-022)
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- None

## Next action
- After the user's review of the details view: W-020 (multi-selection in the UI using the core `Selection`), then W-019 (icons and compact views, zoom), W-021 (persist view settings), W-023, W-013, W-022. Run `rust-tc doctor` and commit after every task.

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
