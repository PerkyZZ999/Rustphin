# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — slice S1 implemented; awaiting the user's visual review
- **Status:** active — S1 code complete and gated; W-012 (real-desktop review) open
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-10-12

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-10-12 — Slice S1 checks

## Open loops
- Task: W-012 — blur and opaque mode not yet seen on the real desktop with the S1 build
- Task: W-013 — frame times unmeasured (R-101's 60 fps target is validated in S2)
- Task: UI scenarios are hand-run, not a replayable script
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- None

## Next action
- User runs `rust-tc run` on the real desktop and reviews blur and the opaque mode (W-012). On approval, mark S1 done in ROADMAP.md and seed WORK_ITEMS.md for S2 (views, sorting, selection). The quality gate is `rust-tc doctor`; run it and commit after every task (AGENTS.md).

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
