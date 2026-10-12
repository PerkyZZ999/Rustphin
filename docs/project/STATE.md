# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S3 done; S4 next
- **Status:** active — between slices
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-10-12

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-10-12 — Implement gate, slice S3

## Open loops
- Task: an 80–90 ms frame gap at start-up while a 500,000-entry folder loads
- Product: nothing is exposed over AT-SPI in test sessions; affects R-107, to resolve in S11
- Task: reordering tabs by mouse drag is implemented but was not exercised in a test session
- Product: view settings are one global file; a pane's own view settings last for the session only
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- None

## Next action
- Start S4 (file operations, undo, trash): write its work items, build the operation engine in `rustphin-core` and pass fault-injection tests before any operation UI. Verify UI changes yourself through kwin-mcp, always in an isolated home. Run `rust-tc doctor` and commit after every task.

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
