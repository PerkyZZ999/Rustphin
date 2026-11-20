# Project state

- **Name:** Rustphin
- **Version:** 0.1.0 (workspace `Cargo.toml`)
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Implement) for v0.2.0 "Daily driver", Slice S13 — v0.1.0 released 2026-11-15 (tag `v0.1.0`, GitHub release); S1–S10 and S12 done, S11's remaining checks moved into v0.2.0 (D-040); the interface is Qt 6 and QML through cxx-qt since W-114i (D-033, D-034)
- **Status:** active — Slice S12 done: W-147 charcoal colours, W-148 drive cards, W-149 transfer details (2026-11-14), W-146 UI completion, W-117 Qt leftovers and W-146f KWin's window menu on the title bar (2026-11-15); W-115 memory on reload, W-076t keyboard pass, W-152 separators in the places panel, W-153 drive Properties, W-154 Settings and Command palette buttons, W-155 Settings restyled after the maintainer's mockup, W-156 toolbar order, centred rows and tooltips below, W-157 Settings window, W-158 every Settings option working, W-159 drives in three styles, W-160 the window keeps its size (2026-11-15); v0.2.0 planned (D-040); v0.1.0 released: W-161 rename works again, W-162 documents, W-163 public README, W-164 tag and GitHub release (2026-11-15)
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-11-19

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (pushing to the GitHub repository, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-11-15 — W-164 Release v0.1.0
- Acceptance: ACCEPTANCE.md lists every open criterion and who can close it (re-mapped 2026-11-14 for the Qt interface).

## Open loops
- Task: the transfer dialog not seen keyboard-only (W-076u; copies on the test volume finish at once).
- Task: the blur status line and the theme following blur (W-146c) were not seen on a compositor without blur: UI_SCENARIOS AP6 needs a desktop with KWin's Blur effect switched off (R-109 unticked on that).
- Task: R-101 on Qt: per-task times on the Qt thread are not measured separately (only through dropped frames, all views now measured, W-150); nothing measured on a physical display.
- Task: `rust-tc fuzz` could not run after W-129 and W-133: the installed nightly (1.97) is below the 1.100 minimum. The fuzz harness checks in `rust-tc doctor` pass.
- Task: remote folders (W-080–W-080c) not tried against a real sftp or smb server, nor replayed on Qt (no kio-fuse mount in the test session).
- Task: compact view on Qt not scrolled with a real wheel (the benchmark scrolled it sideways through 500,000 entries, W-150); drops on a tab and onto a place from another application not seen.
- Task: thumbnails: scrolling a large picture folder not observed (sharing the cache with Dolphin both ways and JPEG, GIF and WebP pictures seen, W-151).
- Task: Properties' folder measurement cancelled on screen not seen (a warm cache measures too fast to cancel by hand; unit-tested); another volume's trash never exercised (real volumes are not test data).
- Task: data safety beyond the tests: physical power loss, partial source-tree deletion after a cross-filesystem move, and restart cleanup remain unproved (W-070, W-072).
- Product: an operation that replaced or deleted something cannot be undone at all, even its reversible parts (`UndoError::Replaced`).
- Product: Ctrl+F3 is bound but KWin takes it by default, as for Dolphin; redo is not offered (KEYBOARD.md).
- Manual (the maintainer): blur look of the current build and contrast over a white window (R-105); a real USB drive mounted, unmounted, safely removed and kept busy (R-013); focus from a real activation token, from the panel and from Chrome's "Show in folder" (R-020, W-120); "Also start it when you log in" after a real login; KDE's portal folder dialog for Extract to…; administrator mode with the real helper (W-098e).
- Open items carried forward from Discovery are listed in DISCOVERY.md.

## Blockers
- None for the agent. The real-desktop and hardware checks above wait on the maintainer.

## Next action
- v0.2.0, Slice S13 (ROADMAP.md, WORK_ITEMS.md): W-170, Properties in tabs: decided (D-041), mockup 40 drawn, waiting for the maintainer's approval before it is built (W-165 to W-169 done 2026-11-19) (W-168–W-170 are the maintainer's first daily-driver findings, 2026-11-19: Reload's feedback, Open With missing, Properties in tabs).
- Then v0.2.0 "Daily driver": S13 fixes and hardening (the trial and the real-desktop checks move here), S14 the PKGBUILD, S15 service menus, S16 quick preview.
- For the maintainer on the real desktop: after installing a build, run `rustphin --quit` once so the new build answers launches; then the checks under "Manual" above.

## Phase checklist
- [x] 0 Intake
- [x] 1 Idea
- [x] 2 Discovery
- [x] 3 Spec
- [x] 4 Decision lock
- [x] 5 Architecture
- [x] 6 Implement (v0.1.0; v0.2.0 starts again here, D-040)
- [x] 7 Release (v0.1.0, 2026-11-15)
- [ ] 8 Observe
