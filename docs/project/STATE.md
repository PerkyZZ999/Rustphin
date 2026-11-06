# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S10 done; S11 in progress
- **Status:** active — S11 safety hardening
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-11-06

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-11-06 — W-079 mockup intake and post-S11 queue; full gate 435 tests

## Open loops
- Queued after S11: the maintainer requests a UI/UX/design pass matching docs/mockup.png as closely as feasible, adapted to current features (2026-11-06). The reference is preserved; W-079/ROADMAP.md record the deferred work. Begin implementation only after S11 is complete; update DESIGN.md before tokens/views.
- Task: W-077a documents local paths/clearing in PRIVACY.md. Locked toolkit retains async-net/http; required IPC sockets and environment-selected bus transports mean R-108 literal dependency criterion is unsatisfied. Source shows no application TCP/UDP/telemetry setup and default GPUI NullHttpClient, and W-077d traces five complete private app lifetimes with only Unix sockets. Clear/restart, permission failure/retry, damaged/newer preservation and offline history/session removal satisfy the stored-history point. W-077e fixes per-workspace session ordering, with red/mutation-proved regressions and isolated split/single-tab restoration. Byte-safe session encoding and newer-format read/write protection remain W-077. W-077c reviews the existing licence/exception policy and passes clean linked builds on Rust 1.95.0 and published stable 1.99.1; that R-109 point is satisfied. W-077b provides rust-tc build and rust-tc validate; the combined gate/benchmark command passes and closes that R-109 point. Published stable is 1.99.1; installed stable alias is older, so select 1.99.1 explicitly.
- Task: W-073b–e fixed measured font/layout and selection-transfer stalls and stale navigation rows. Details first entries observed within 185.7 ms; all 500,000 selections survive refresh. Final three-view max draw 6.67 ms, but p95 intervals 16.72–16.98 ms do not pass strict 60 fps. Wider input/thumbnail task acceptance and physical-display pacing remain open under R-101
- Task: W-074a verified the pinned Wayland adapter after enabling IsEnabled on the isolated AT-SPI bus. File entries and navigation controls now expose names/selection/focus. W-074b verifies additive selection/deselection through AT-SPI in all three views. W-074c verifies places/group/tab semantics, focus/actions and mouse tab reordering. Native expanded state is not mapped by the pinned adapter; spoken group state is present. W-074d verifies text-input focus in all plain-text fields. W-074e verifies palette dialog/results/selection semantics. W-074f verifies context-menu focus/actions. W-074g verifies properties semantics and permission labels. W-074h verifies operation/launch/batch dialogs and preview/error text. W-074i verifies native Text reads, caret/selection and multibyte positions in all views. EditableText, native expanded/disabled flags, incremental client cache and an Orca pass remain R-107 work; orca is not on PATH.
- Task: W-076g verifies visible device-service connection/list failure, stale-row removal and private generated-service recovery; final path navigation/copy works with no device bus. W-076h/j verify startup and live blur fallback/status, same-process recovery, minimum size, independent operation notices and explicit opaque controls in an isolated compositor. W-076i verifies missing data-control copy/cut/paste and a one-time readable notice; R-011 fallback is accepted. Broader R-109 reconciliation remains W-078 work
- Product: view settings are one global file; a pane's own view settings last for the session only
- Task: W-076b verifies failure Stop/Skip/Try again, running counts/rate, progress Stop with staged-copy cleanup, and rename/undo in icons and compact views. Another volume's trash remains unobserved; real volume trashes are not test data
- Task: properties opened through D-Bus label the type generically as File; folder measurement cancellation and unreadable-folder reporting are not yet observed on screen. W-076c fixes and verifies Properties at the minimum window size; batch rename also stays contained with focus-revealed fields (W-076d). Operation/launch questions are contained with fixed headings/actions and scrolling facts (W-076f). W-076k verifies minimum-window Drop/Empty Trash dialogs, drop cancel/copy and private-trash cancel/restore; W-076l wraps the trash banner and keeps actions in a bounded, window-level menu, with native names/focus and safe restore/cancel. W-076m bounds empty/loading/hidden-only states with scrolling, native status and focus-revealed action. Minimum split reasons/actions, a 56 px notice case, real Loading in 85 px panes and normal input/menu/selection routes are verified
- Task: copied file data/attributes (W-070), copied directories and cross-filesystem destination publication before source removal (W-072b) are synced. Whole-source removal sync errors are reported; partial source-tree deletion, restart cleanup and physical power loss remain unproved
- Task: new batch renames and undo have explicit, journal-validated recovery (W-071). A kill can still leave staged names until recovery is requested; older unjournaled temporary files are not guessed at. R-103's literal immediate-after-kill criterion and physical power-loss validation remain open; journal namespace barriers are now in place (W-072c)
- Product: an operation that replaced or deleted something cannot be undone at all, even its reversible parts
- Manual: drag a file from Rustphin into an application that is not a KDE one (R-010); needs the user's desktop
- Manual: W-076h/j verify startup and live blur loss/recovery in an isolated compositor. Real-desktop blur over a white window still needs review. Unnamed/unmonitorable display connections stay opaque; a failed capability monitor requires an app restart to reconnect
- Task: not seen on screen in S5: dropping on a tab, drags in the icons and compact views, copy (not cut) between Rustphin and Dolphin
- Manual: mount, unmount and safely remove a real USB drive from the places panel, and unmount one in use to see the reason (R-013); needs the user
- Task: places panel not yet seen on screen: hiding a place, reordering by keyboard, an edit made in Dolphin other than adding; its width is fixed and its visibility is not remembered
- Task: thumbnails not yet observed: reuse of a thumbnail Dolphin made and the reverse on screen (the cache format is unit-tested against the specification's example); JPEG, GIF and WebP files; scrolling a large picture folder
- Open items carried forward from Discovery are listed in DISCOVERY.md

## Blockers
- W-076e: complete reverse field traversal and native input focus conflict through duplicate registration in the pinned toolkit. Removing the wrapper tracking restores traversal but loses native focus; original code restored. DISCOVERY.md records a concrete deduplication candidate and required checks. The maintainer approved a bounded isolated experiment on 2026-11-05; its corrected latest-registration candidate passes seven focused tests and native reverse-field/Unicode checks. Production pins remain unchanged. Full published toolkit tests are unavailable; broader modal/performance assessment and adoption still require review. This blocks that acceptance item, not unrelated S11 work. The baseline gate passes.

## Next action
- Continue S11 W-077 byte-safe session encoding and newer-format protection (W-077a–e document retention/connection limits, complete build/benchmark commands, clean linked builds, dependency policy review, bounded live clear/restart/connection checks and per-workspace session ordering); keep remaining keyboard/focus coverage (W-076e isolated experiment complete; adoption not approved) in an isolated home (W-076g verifies device fallback; W-076h/j verify startup/live blur fallback; W-076i verifies data-control fallback). W-076a fixes split-window filename clipping; W-076b verifies operation failure/recovery/Stop and grid rename; W-076c/d fix minimum-height Properties and batch-rename clipping; W-076f contains operation/launch questions. The broader keyboard checklist remains open. W-075a/b/c complete the bounded parser checks and visible failure handling; reconcile R-104 against this evidence in W-078. Keep W-074 Orca/native-adapter findings, W-073 strict pacing/wider R-101 and real-desktop/hardware acceptance open. No toolkit lock, scope or requirement has changed. Full gate, evidence and commit after every coherent task.

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
