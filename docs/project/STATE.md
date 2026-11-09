# Project state

- **Name:** Rustphin
- **Artifact root:** docs/project/
- **Workflow profile:** product — intended as a real, daily-usable file manager; file operations touch user data, so full gates and automated validation apply. Not regulated, so not high-risk.
- **Current phase:** 6 (Vertical-slice implementation) — S1–S10 done; S11 in progress; W-079 design pass started (D-024)
- **Status:** active — W-079 design pass done (a–g); remaining S11 acceptance needs the maintainer
- **MVP success signal:** author uses Rustphin instead of Dolphin as default for two consecutive weeks; a 500,000-entry directory opens, sorts and scrolls without visible stalls (agreed 2026-10-12)
- **Last updated:** 2026-11-08

## Authority boundaries
- Agent may decide: stack-local, reversible implementation details consistent with locked decisions; document structure; disposable spike code.
- Human confirmation required: MVP boundary, locking toolkit/architecture/licence decisions, destructive actions, publishing (remote repo, releases), anything involving credentials or paid services.

## Current evidence
- Latest passed gate: 5 Architecture (2026-10-12)
- Latest validation entry: VALIDATION.md, 2026-11-08 — W-079g dialogs, W-079 pass complete; full gate 492 tests

## Open loops
- Task: W-079 design pass done 2026-11-08 (D-024, steps a–g) from the maintainer's HTML mockups; docs/DESIGN.md "Implementation status" lists what is left (compact columns W-086, list scrollbars, volume name, per-command palette icons, app icons, tab overflow, disabled toolbar colour, saved Devices fold). Glass look needs the real desktop.
- Task: W-078: 27 criteria ticked on evidence; 7 open in docs/project/ACCEPTANCE.md. Decided by the maintainer on 2026-11-08: D-019 (60 fps = ≤5% dropped frames, met in W-085), D-020 (startup batch recovery, done W-083), D-021 (Ctrl+D duplicate, done W-084); W-076e not approved. D-022 (startup outside R-101) closed R-101; R-105 opaque-mode contrast accepted on token tests; D-023 dropped the Orca pass and R-107 names/roles is ticked on W-074a–i. Still open for the maintainer: the 5 real-desktop/hardware checks (non-KDE drop, USB mount/busy, blur look and white-window contrast).
- Task: W-076o–r add docs/project/KEYBOARD.md (Rustphin vs Dolphin defaults) and fix Backspace=Back, Alt+Home, Alt+D, Alt+1…0, Ctrl+P, Ctrl+Q, Shift+F5/F6 copy/move to the other view, Ctrl+Shift+T reopen closed tab, Ctrl+Alt+C copy location, Menu key. Ctrl+F3 is bound but KWin takes it by default (Dolphin too); Tab works. Not offered yet: redo (Duplicate added in W-084).
- Task: W-080/W-080b/W-080c (requested addition) open typed remote addresses and network bookmarks through kio-fuse, save remote tabs by address and reconnect them on restart, and show remote breadcrumbs; verified with a private-bus stand-in and the real kio-fuse on a generated tar archive, no real host. Recent folders and the palette also keep remote folders by address. Not yet tried against a real sftp/smb server. The Ctrl+L gap W-080 found is fixed in W-076n.
- Task: W-077 complete. W-077a–d inventory local data/connections, add rust-tc build/validate, verify clean 1.95.0/1.99.1 builds and live clear/restart with only Unix sockets traced. W-077e/f order and byte-safely encode session saves. W-077g keeps unreadable, damaged and newer session files unchanged for the run (explicit launches included) with a visible reason. W-077h: the maintainer accepted the locked toolkit's async-net/http crates as a documented exception (D-018, 2026-11-08); deny.toml bans HTTP clients, TLS and telemetry crates and confines the retained ones, and R-108 is accepted. Remote folders remain mounted paths under D-004. Cross-process session merging and immediate-quit flushing remain unproved. Published stable is 1.99.1; select it explicitly.
- Task: W-073b–e fixed measured font/layout and selection-transfer stalls and stale navigation rows. Details first entries observed within 185.7 ms; all 500,000 selections survive refresh. Final three-view max draw 6.67 ms; p95 intervals 16.72–16.98 ms failed the old strict p95, and W-085 measured 0–0.25% dropped frames, passing D-019. Wider input/thumbnail task acceptance and physical-display pacing remain open under R-101
- Task: W-074a verified the pinned Wayland adapter after enabling IsEnabled on the isolated AT-SPI bus. File entries and navigation controls now expose names/selection/focus. W-074b verifies additive selection/deselection through AT-SPI in all three views. W-074c verifies places/group/tab semantics, focus/actions and mouse tab reordering. Native expanded state is not mapped by the pinned adapter; spoken group state is present. W-074d verifies text-input focus in all plain-text fields. W-074e verifies palette dialog/results/selection semantics. W-074f verifies context-menu focus/actions. W-074g verifies properties semantics and permission labels. W-074h verifies operation/launch/batch dialogs and preview/error text. W-074i verifies native Text reads, caret/selection and multibyte positions in all views. EditableText, native expanded/disabled flags and the incremental client cache are listed gaps; the Orca pass was dropped (D-023).
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
- The maintainer to review the W-079 result on the real desktop (glass look) and decide W-086 (compact view in columns). The open S11 criteria still need the maintainer's desktop or hardware (5) or W-076e (2). Otherwise continue with the smaller W-079 leftovers listed in docs/DESIGN.md.

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
