# AGENTS.md

## Project overview

Rustphin is a file manager for KDE Plasma on Wayland, written in Rust. It is a re-architecture of KDE's Dolphin (not a line-by-line port) with a dark frosted-glass UI, place groups, a command palette and advanced batch rename.

- Language: Rust, edition 2024, minimum version 1.95.
- UI: `gpui-kit` pinned to `=0.7.3` (GPUI snapshot `gpui-pre` `=0.3.9`).
- Platform: Linux on Wayland only. No X11, macOS or Windows code.
- Licence: GPL-3.0-or-later. Every dependency must be GPLv3-compatible.
- Repository: local git only, branch `main`. There is no remote and no hosted CI; do not add one or push anywhere.

## Read this first

The project follows a gated workflow. Its documents are the source of truth and live in `docs/project/`:

| File | Use it for |
|---|---|
| `STATE.md` | Current phase, blockers and the next action. Read it before doing anything; update it when you finish. |
| `WORK_ITEMS.md` | The queue for the current slice. Work from here. |
| `ROADMAP.md` | Slices S1–S11 and which requirement each covers. |
| `REQUIREMENTS.md` | Requirements R-001… with acceptance criteria. |
| `DECISIONS.md` | Locked decisions and assumptions. |
| `ARCHITECTURE.md` | Crates, data flow, trust boundaries, failure modes, test strategy. |
| `VALIDATION.md` | Evidence log. Append an entry for every gate or check you actually ran. |
| `DISCOVERY.md` | Constraints, risks and measurements from the feasibility phase. |
| `UI_SCENARIOS.md` | The UI checks to replay through kwin-mcp after UI changes. |

`docs/DESIGN.md` is the reference for the visual design (decision D-017). Its front-matter tokens are authoritative and `crates/rustphin-ui/src/theme.rs` implements them: change the document first, then the theme, and keep its "Implementation status" section true. After editing it, run `npx @google/design.md lint docs/DESIGN.md` and fix any errors.

Rules that follow from it:

- **Locked decisions are binding.** D-001, D-002, D-003, D-006, D-008 to D-011 and D-017 may not be changed or worked around. If one blocks you, stop and raise it with the user; changing it needs their approval and a superseding record.
- **Evidence over claims.** Never record a check as passed unless you ran it. Say what was observed, inferred or not verified.
- **One slice at a time.** Implement the current slice's work items; do not start later slices' features.
- Ask the user before: changing MVP scope, locking or changing a decision, anything destructive, publishing anything.

## After every task

A task is one work item or one coherent step. When it is done, in this order:

1. Run all checks: `rust-tc doctor`. Fix every failure; do not suppress findings to get a pass.
2. Update `docs/project/` (`WORK_ITEMS.md`, `VALIDATION.md`, `STATE.md`) to match what was done and what was actually verified.
3. Commit (see [Commits](#commits)).

Do not start the next task with failing checks or uncommitted work.

## Required skills

Load and follow these skills whenever the work matches. They are mandatory for this project.

| Work | Skill |
|---|---|
| Writing, reviewing, fixing or auditing any test | `~/.agents/skills/devops-quality/testing-suite` (read its `references/rust.md`) |
| Any frontend, visual design or UI/UX work | `~/.agents/skills/web-design/craft-beautiful-frontend` |
| Adding, choosing or reviewing icons | `~/.agents/skills/web-design/iconography-frontend-ui` |

Notes on applying them here:

- Testing: every new test must pass the two litmus tests (fails when the behaviour breaks; survives a refactor). Prove new tests by breaking the code under test and seeing them fail. A bug fix starts with a failing regression test.
- The two design skills are written for web CSS. Apply their rules, tables and quality gates to GPUI: all visual values come from `docs/DESIGN.md` through the tokens in `crates/rustphin-ui/src/theme.rs` (4-pixel spacing grid, semantic colour names, one 2-pixel corner radius); text meets WCAG AA and is covered by the contrast tests there; state is never shown by colour alone; every empty, loading and error state says what happened and what to do; icons come from GPUI-Kit's bundled Lucide set, sized on the 4/8-pixel grid, and icon-only buttons carry a tooltip.

## Architecture rules

Cargo workspace, dependencies point one way: `rustphin-ui` → `rustphin-platform` → `rustphin-core`.

| Crate | Contains | Must not |
|---|---|---|
| `rustphin-core` | Directory model, listing, sorting, selection, history, file operation engine, undo, batch rename, fuzzy matching | Depend on GPUI, Wayland or D-Bus crates |
| `rustphin-platform` | Blur, file clipboard, devices, trash, thumbnails, places, MIME, icons | Depend on `gpui-kit` |
| `rustphin-ui` | Theme, views, key bindings, bridge to the core | Touch the filesystem directly |
| `rustphin` | Binary; wiring only | Hold logic |

The layering is enforced by `rust-tc layers` (part of `rust-tc doctor`) and by the `gpui-kit` ban in `deny.toml`.

- No work on the UI thread: listing, metadata, sorting, thumbnails and file operations run on the core's own worker pools and reach the UI as immutable snapshots over channels. No Tokio or other general async runtime in the core (D-009).
- All file mutations go through the core's operation engine (D-011): write under a temporary name then rename; verify before deleting a move's source; never overwrite or permanently delete without an explicit user choice; delete means trash.
- File names are bytes. Never assume UTF-8; never `unwrap` a name conversion.
- Wayland integrations (blur, clipboard) live in `rustphin-platform` behind traits with runtime detection and a fallback (D-010). Do not fork GPUI.
- Place groups are stored in Rustphin's own config, never inside KDE's `user-places.xbel` (D-008).

## Setup

Requirements: Rust through rustup (stable, plus the `1.95.0` toolchain for the minimum-version check), a Wayland session, a working Vulkan driver, the Wayland, xkbcommon and fontconfig development packages, and the Rust-Toolchain tools: `rust-tc`, `just`, `cargo-deny`, `cargo-shear`, `cargo-nextest`, `cargo-hack` (`cargo-llvm-cov` for optional coverage). Install instructions are in `../Rust-Toolchain/RUST-TOOLCHAIN.md`.

```bash
rust-tc run                # build and run, opening the home folder
rust-tc run ~/Documents    # open a specific folder
```

Environment variables: `RUSTPHIN_OPAQUE=1` forces the opaque theme; `RUSTPHIN_LOG=debug` sets the log level (logs go to stderr). `RUSTPHIN_FRAME_LOG=1` logs frame timing every 120 frames; `RUSTPHIN_SCROLL_BENCH=1` also makes the list scroll itself for 1,200 frames, which is how frame rate is measured (see W-013 in `docs/project/VALIDATION.md`).

A cold build of the UI toolkit takes two to three minutes.

## Quality gate: Rust-Toolchain

`rust-tc` is the only check entry point. It wraps the repository `justfile`; do not tell anyone to run `just` directly, and do not add parallel homemade check scripts.

| Command | Runs | When |
|---|---|---|
| `rust-tc check` | compiler only | fastest feedback while editing |
| `rust-tc quick` | format, check, clippy, tests, doctests | after meaningful edits |
| `rust-tc doctor` | format, clippy, tests, doctests, `cargo-deny`, `cargo-shear`, feature checks, `layers`, `msrv`, fuzz harness checks | after every task, before committing |
| `rust-tc fuzz <target> [seconds]` | bounded URI, places, desktop or image parser campaign; defaults to 30 seconds | after relevant parser changes; requires nightly and cargo-fuzz; see `fuzz/README.md` |
| `rust-tc bench-listing` | 500,000-entry listing benchmark; fails if R-101's thresholds are missed | after changing listing, sorting or the model |
| `rust-tc deep`, `mutants`, `miri` | deeper checks | on demand, not after every edit |

Hard rules:

- Do not add `cargo-audit` (`cargo-deny` owns advisories). Do not invoke the separate `rust-doctor` binary. Do not add SonarQube.
- Do not blanket-allow lints. An `allow` needs an understood, intentional reason, the narrowest scope, and a comment when not obvious.
- Do not invent fuzz targets to satisfy the toolchain.
- `deny.toml` is reviewed for this project. Its four advisory ignores are "unmaintained" notices for crates that arrive only through the pinned UI toolkit; re-check them at every toolkit upgrade. New licences or ignores need a comment saying why.
- `rust-tc miri` is expected for changes to the `unsafe` code in `rustphin-platform` where it can run.

## Development workflow

- Keep `gpui-kit` pinned exactly. Never run `cargo update` for it or change its version as a side effect; an upgrade is its own planned work item.
- Add dependencies sparingly and check the licence first.
- `spikes/` is throwaway reference code from Discovery, outside the workspace. Port ideas from it; do not depend on it or extend it.
- GPUI APIs differ between the pinned snapshot and Zed's `main`. When unsure, read the pinned source under `~/.cargo/registry/src/*/gpui-pre-0.3.9/` and `gpui-component-0.7.3/`.

## Testing

Follow the testing-suite skill. Layout and tools:

- Unit tests live in `#[cfg(test)] mod tests` beside the code; runner is `cargo-nextest` through `rust-tc`.
- Core: behaviour through the public model API where possible; fakes of the `Fs` trait instead of mocks; fault injection through `Fs` for every file operation; no sleeps (wait on model events).
- Platform: fixture and contract tests; D-Bus services against a mocked bus.
- UI: verify it yourself. After any change to `rustphin-ui`, replay the relevant scenarios in `docs/project/UI_SCENARIOS.md` through the kwin-mcp tools (load the `anthropic-skills:kwin-mcp` skill) and record what you saw. Do not ask the user to launch the app and report back; ask them only about matters of taste, or what the tools cannot show.
- UI: pure logic (selection movement, path expansion, formatting, token contrast) is unit-tested. Interaction is checked by scripted scenarios in an **isolated KWin session with an isolated home directory** (kwin-mcp `session_start` with `isolate_home: true`). Never run test instances of Rustphin or Dolphin against the user's real home, and never use the user's real files as test data. Wait for the window before sending input: the app takes a moment to start.
- File-operation UI must not be enabled before the operation engine's fault-injection suite passes.

Performance targets on a 500,000-entry directory (R-101): first entries within 200 ms, re-sort within 500 ms, 60 fps scrolling, no UI-thread task over 16 ms.

## Code style

- `cargo fmt` and `cargo clippy` clean, with warnings denied (both run by `rust-tc`). Workspace lints deny `unwrap`, `expect`, `dbg!` and `todo!` outside tests.
- Errors: typed errors in library crates (`thiserror`-style enums); no `unwrap`/`expect` on fallible I/O outside tests.
- `unsafe` only in `rustphin-platform`, each block with a `// SAFETY:` comment.
- Public items in `rustphin-core` and `rustphin-platform` carry doc comments.
- No telemetry and no network access anywhere in the application.

## Commits

- Local commits on `main`; small and focused, one work item or coherent step each.
- Message: imperative summary line (≤ 72 characters), blank line, then why. Reference the work item or requirement, for example `W-004` or `R-101`.
- Commit after every task, once `rust-tc doctor` passes and `docs/project/` reflects what changed. Never commit with failing checks.

## Reference material

- GPUI source: https://github.com/zed-industries/zed/tree/main/crates/gpui (the pinned snapshot is older than `main`; check the `gpui-pre` 0.3.9 source when an API differs)
- GPUI-Kit documentation index: https://gpui-kit.com/llms.txt
- Dolphin source, for behaviour reference only: https://github.com/kde/dolphin

## Known gotchas

- GPUI's own clipboard cannot exchange files with other applications, and its blur uses a protocol current KWin does not offer. Use the platform layer's implementations.
- GPUI cannot blur content inside the window. Menus, the palette and dialogs are solid surfaces by design (D-017, `docs/DESIGN.md`).
- The user's Dolphin reports Ctrl+V as an ambiguous shortcut; in UI tests paste through Dolphin's context menu.
- Key bindings that also edit text (arrows, Enter, Backspace, Home, End) must use the `Browser && !Input` context, or they fire while the user types in a text field.
- Never read an entity inside its own `cx.listener` or `update`: GPUI panics ("cannot read … while it is already being updated"). Drop handlers read the dragged entries from their source view, which may be the same view, so they are plain closures that read first and update afterwards.
- Setting colours on GPUI-Kit's theme does not reach its components until `tokens` is rebuilt from `colors` (see `Theme::apply_to_components`).
- In kwin-mcp, drags need waypoints with pauses of about 100 ms, or the application does not see a drag. A crashed test instance may ignore `pkill`; use `pkill -9 -x rustphin`.
- KDE adds its built-in places (Home, Trash, Network, …) to `user-places.xbel` the first time one of its applications reads a file that lacks them, so the file changes under Rustphin right after Dolphin starts in a fresh home.
- A debug build can take ten seconds or more to show its window in a kwin-mcp session; a 4 KB screenshot means nothing is on screen yet, not that the app crashed.
- Isolated kwin-mcp sessions still share the machine's system bus: the devices listed there are the user's real drives. Never click a device row or its eject button in a test; mount and unmount are tested against the mock service in `rustphin-platform`.
- Dialogs use the `Dialog` key context and single-letter keys; workspace shortcuts that must not fire behind a dialog need `&& !Dialog` in their context.
- A file operation's first progress report arrives when its plan is ready, before any change; tests that cancel "after n reports" count from there.
- GPUI-Kit's root element paints an opaque background under the app view. The window is opened with a root whose background is cleared (`crates/rustphin-ui/src/lib.rs`); keep that, or translucency silently disappears.
- GPUI-Kit's component `IconName` is a small subset. Other Lucide icons are `gpui_kit::assets::IconName::…` and only render because the app registers `AllAssets`.
- kwin-mcp's smooth `mouse_scroll` does not move the list and its discrete scroll sends one event; use the scroll benchmark for sustained scrolling.
- In kwin-mcp, spell punctuation keys literally (`ctrl+=`, `ctrl+-`); names like `equal` or `minus` are accepted but do nothing.
- Isolated KWin sessions offer no blur protocol, so Rustphin shows its opaque theme there. Blur can only be reviewed on the real desktop.
