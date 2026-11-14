# AGENTS.md

## Project overview

Rustphin is a file manager for KDE Plasma on Wayland, written in Rust. It is a re-architecture of KDE's Dolphin (not a line-by-line port) with a dark frosted-glass UI, place groups, a command palette and advanced batch rename.

- Language: Rust, edition 2024, minimum version 1.100 (D-035).
- UI: Qt 6 Quick and QML through cxx-qt 0.10 (D-034), with KDE Frameworks' `KWindowEffects` for blur. GPUI was replaced at W-114i (2026-11-14); the last GPUI version is commit `c062d63`.
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
| `DESIGN_BRIEF.md`, `INFORMATION_ARCHITECTURE.md` | The W-079 design pass: brief and screen structure. The screens are drawn in `docs/mockup/` (HTML, one page per screen); `docs/DESIGN.md` holds the adopted design and says what is implemented. |

`docs/DESIGN.md` is the reference for the visual design (decision D-017). Its front-matter tokens are authoritative and `crates/rustphin-view/src/colors.rs` (colours, with the contrast tests) and `metrics.rs` (spacing, sizes, type) implement them; `crates/rustphin/src/tokens.rs` hands them to QML as the `Tokens` singleton. Change the document first, then the tokens, and keep its "Implementation status" section true. After editing it, run `npx @google/design.md lint docs/DESIGN.md` and fix any errors.

Rules that follow from it:

- **Locked decisions are binding.** D-002, D-003, D-006, D-008 to D-011 and D-017 may not be changed or worked around (D-001, the GPUI toolkit, was superseded by D-034). If one blocks you, stop and raise it with the user; changing it needs their approval and a superseding record.
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
| Writing or reviewing QML, Qt or the C++ shell | the Qt skills in `~/.agents/skills/desktop`: `qt-qml`, `qt6-qml-development`, `qt-qml-docs`, `qt-ui-design`, `qt-qml-review` (and `qt-qml-test`, `qt-qml-profiler`, `qt-cpp-review`, `qt-project` where they fit) |

Notes on applying them here:

- Testing: every new test must pass the two litmus tests (fails when the behaviour breaks; survives a refactor). Prove new tests by breaking the code under test and seeing them fail. A bug fix starts with a failing regression test.
- The two design skills are written for web CSS. Apply their rules, tables and quality gates to Qt Quick: all visual values come from `docs/DESIGN.md` through the tokens in `crates/rustphin-view/src/colors.rs` and `metrics.rs` (4-pixel spacing grid, semantic colour names, one 2-pixel corner radius); text meets WCAG AA and is covered by the contrast tests there; state is never shown by colour alone; every empty, loading and error state says what happened and what to do; interface icons are the bundled Lucide SVGs (`crates/rustphin/icons`, drawn through `image://lucide`), file-type icons come from the icon theme (`image://theme`), all sized on the 4/8-pixel grid, and icon-only buttons carry a tooltip.

## Architecture rules

Cargo workspace, dependencies point one way: `rustphin` → `rustphin-view` → `rustphin-platform` → `rustphin-core`.

| Crate | Contains | Must not |
|---|---|---|
| `rustphin-core` | Directory model, listing, sorting, selection, history, file operation engine, undo, batch rename, fuzzy matching | Depend on Qt, cxx, Wayland or D-Bus crates |
| `rustphin-platform` | File clipboard, devices, trash, thumbnails, places, MIME, icons, FileManager1 and instance services, administrator helper client, remote folders | Depend on Qt or cxx |
| `rustphin-view` | Interface logic free of any toolkit: pane wording, settings and Settings pages, session, tabs, toolbar items, menus, commands, questions, dialogs, the design tokens (`colors`, `metrics`) | Depend on Qt or cxx |
| `rustphin` | The binary and its Qt interface: QML (`qml/`), Rust QObjects through cxx-qt (`src/`), the C++ shell (`cpp/`) | Touch the filesystem directly for changes; hold logic that `rustphin-view` can hold |

The layering is enforced by `rust-tc layers` (part of `rust-tc doctor`); `deny.toml` bans `gpui-kit` and `gpui-pre`.

- No work on the UI thread: listing, metadata, sorting, thumbnails and file operations run on the core's own worker pools and reach the UI as immutable snapshots over channels. No Tokio or other general async runtime in the core (D-009).
- All file mutations go through the core's operation engine (D-011): write under a temporary name then rename; verify before deleting a move's source; never overwrite or permanently delete without an explicit user choice; delete means trash.
- File names are bytes. Never assume UTF-8; never `unwrap` a name conversion.
- The file clipboard lives in `rustphin-platform` behind a trait with runtime detection and a fallback (D-010). Blur is KDE's `KWindowEffects`, asked for by the C++ shell, with the opaque theme when KWin offers none.
- Place groups are stored in Rustphin's own config, never inside KDE's `user-places.xbel` (D-008).

## Setup

Requirements: Rust through rustup (stable, plus the `1.100.0` toolchain for the minimum-version check), a Wayland session, a C++ compiler, the Qt 6 development packages (Base, Declarative, Wayland; `qmake6` on the path) and KDE Frameworks 6 WindowSystem (`KWindowEffects`), and the Rust-Toolchain tools: `rust-tc`, `just`, `cargo-deny`, `cargo-shear`, `cargo-nextest`, `cargo-hack` (`cargo-llvm-cov` for optional coverage). Install instructions are in `../Rust-Toolchain/RUST-TOOLCHAIN.md`.

```bash
rust-tc run                # build and run, opening the home folder
rust-tc run ~/Documents    # open a specific folder
```

Environment variables: `RUSTPHIN_ADMIN_BUS=session` (UI tests only) makes administrator mode look for KDE's kio-admin helper on the session bus, where `cargo run -p rustphin-platform --example fake_admin_helper -- <sandbox>` stands in for it; never test administrator mode against the real helper, which would ask for the maintainer's password on his desktop (UI_SCENARIOS.md, AD1–AD4). `RUSTPHIN_OPAQUE=1` forces the opaque theme; `RUSTPHIN_LOG=debug` sets the log level (logs go to stderr). Qt and QML warnings go to the journal unless `QT_FORCE_STDERR_LOGGING=1` is set; `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` exposes the window over AT-SPI in test sessions. `RUSTPHIN_FRAME_LOG=1` logs frame timing every 120 frames, counting dropped frames (intervals over 1.5 × the refresh period, D-019); `RUSTPHIN_SCROLL_BENCH=1` also makes the list scroll itself for 1,200 frames and ends with a "scroll benchmark result" line saying whether at most 5% were dropped, which is how frame rate is measured (see W-013 and W-085 in `docs/project/VALIDATION.md`). With either set, each opened folder logs a "first entries" line: time until a frame described its entries and until the next frame began (W-073g).

Dependencies are optimised even in debug builds, so a debug window opens about as fast as a release one. `crates/rustphin/build.rs` compiles the QML with qmlcachegen and finds every `*_bridge.rs` and `qml/*.qml` itself: a QML error fails the build, and a new bridge or QML file needs no list edited.

## Quality gate: Rust-Toolchain

`rust-tc` is the only check entry point. It wraps the repository `justfile`; do not tell anyone to run `just` directly, and do not add parallel homemade check scripts.

| Command | Runs | When |
|---|---|---|
| `rust-tc check` | compiler only | fastest feedback while editing |
| `rust-tc quick` | format, check, clippy, tests, doctests | after meaningful edits |
| `rust-tc doctor` | format, clippy, tests, doctests, `cargo-deny`, `cargo-shear`, feature checks, `layers`, `msrv`, fuzz harness checks | after every task, before committing |
| `rust-tc fuzz <target> [seconds]` | bounded URI, places, desktop, image, admin-entry or archive-entry parser campaign; defaults to 30 seconds | after relevant parser changes; requires nightly and cargo-fuzz; see `fuzz/README.md` |
| `rust-tc bench-listing` | 500,000-entry listing benchmark; fails if R-101's thresholds are missed | after changing listing, sorting or the model |
| `rust-tc deep`, `mutants`, `miri` | deeper checks | on demand, not after every edit |

Hard rules:

- Do not add `cargo-audit` (`cargo-deny` owns advisories). Do not invoke the separate `rust-doctor` binary. Do not add SonarQube.
- Do not blanket-allow lints. An `allow` needs an understood, intentional reason, the narrowest scope, and a comment when not obvious.
- Do not invent fuzz targets to satisfy the toolchain.
- `deny.toml` is reviewed for this project. Its three advisory ignores record reviewed "unmaintained" notices for crates retained by the zbus 5 integrations; re-check them at every zbus upgrade. New licences or ignores need a comment saying why.
- `rust-tc miri` is expected for changes to the `unsafe` code in `rustphin-platform` where it can run.

## Development workflow

- Keep `cxx-qt` on its 0.10 line; an upgrade is its own planned work item. Never change toolkit versions as a side effect of another task.
- Add dependencies sparingly and check the licence first.
- `spikes/` is throwaway reference code from Discovery, outside the workspace. Port ideas from it; do not depend on it or extend it.
- When unsure of a cxx-qt API, read the source under `~/.cargo/registry/src/*/cxx-qt-0.10.*/` and `cxx-qt-lib-0.10.*/`; for Qt, the documentation of the installed version (Qt 6.12 on the development machine).

## Testing

Follow the testing-suite skill. Layout and tools:

- Unit tests live in `#[cfg(test)] mod tests` beside the code; runner is `cargo-nextest` through `rust-tc`.
- Core: behaviour through the public model API where possible; fakes of the `Fs` trait instead of mocks; fault injection through `Fs` for every file operation; no sleeps (wait on model events).
- Platform: fixture and contract tests; D-Bus services against a mocked bus.
- UI: verify it yourself. After any change to the interface (`crates/rustphin`), replay the relevant scenarios in `docs/project/UI_SCENARIOS.md` through the kwin-mcp tools (load the `anthropic-skills:kwin-mcp` skill) and record what you saw. Do not ask the user to launch the app and report back; ask them only about matters of taste, or what the tools cannot show.
- UI: pure logic (selection movement, path expansion, formatting, token contrast) is unit-tested. Interaction is checked by scripted scenarios in an **isolated KWin session with an isolated home directory** (kwin-mcp `session_start` with `isolate_home: true`). Never run test instances of Rustphin or Dolphin against the user's real home, and never use the user's real files as test data. Wait for the window before sending input: the app takes a moment to start.
- File-operation UI must not be enabled before the operation engine's fault-injection suite passes.

Performance targets on a 500,000-entry directory (R-101): first entries within 200 ms, re-sort within 500 ms, 60 fps scrolling, no UI-thread task over 16 ms.

## Code style

- `cargo fmt` and `cargo clippy` clean, with warnings denied (both run by `rust-tc`). Workspace lints deny `unwrap`, `expect`, `dbg!` and `todo!` outside tests.
- Errors: typed errors in library crates (`thiserror`-style enums); no `unwrap`/`expect` on fallible I/O outside tests.
- `unsafe` only in `rustphin-platform`, each block with a `// SAFETY:` comment. The one exception is `rustphin` (D-034): cxx-qt generates unsafe FFI glue, so each bridge sits alone in a `*_bridge.rs` file of declarations, included with `#[allow(unsafe_code)]`; logic stays in the safe files beside it.
- Public items in `rustphin-core` and `rustphin-platform` carry doc comments.
- No telemetry and no network access anywhere in the application.

## Commits

- Local commits on `main`; small and focused, one work item or coherent step each.
- Message: imperative summary line (≤ 72 characters), blank line, then why. Reference the work item or requirement, for example `W-004` or `R-101`.
- Commit after every task, once `rust-tc doctor` passes and `docs/project/` reflects what changed. Never commit with failing checks.

## Reference material

- Qt 6 documentation: https://doc.qt.io/qt-6/ (QML types, Qt Quick, Qt Quick Controls)
- cxx-qt book: https://kdab.github.io/cxx-qt/book/
- KWindowEffects (KF6 WindowSystem): https://api.kde.org/frameworks/kwindowsystem/html/
- Dolphin source, for behaviour reference only: https://github.com/kde/dolphin

## Known gotchas

Qt and QML:

- A window `Shortcut` listing the same key twice (`"Alt+Left"` and `StandardKey.Back` are one key on Linux), or one press matching two of its sequences (Shift+, is also "<"), is ambiguous: Qt fires neither. Main.qml's `Key` component acts on an ambiguous match; never let two shortcuts share a sequence.
- Keys that also edit text (arrows, Enter, Backspace, Home, End, Tab, Escape) belong to the pane's or the places panel's `Keys` handler, not to window shortcuts, or they fire while the user types in a field.
- A model reset destroys every delegate: a press in progress is lost and an open field loses focus (which ends a rename). Use `dataChanged` for focus, current-row and renaming changes, insert/remove/move rows for tabs, and redraw unchanged rows in place (places panel).
- A delegate's required role named like an outer id shadows it (role `folder` was renamed `isFolder`; the window's `tabs` and `places` are reached as `root.tabsModel`, `root.placesModel`).
- cxx-qt generates `setX` for each `#[qproperty]`; an invokable named like one (`setMode`) clashes, and C++ keywords (`class`, `case`) cannot be parameter names. QML must not write properties the Rust side owns.
- `QMetaObject::invokeMethod` on a QML function needs `Q_ARG` types matching its declared parameters: a `string` parameter takes `QString`, an untyped one `QVariant`.
- `destroy()` throws on objects Qt owns, such as the item that holds a submenu; take submenus out with `takeMenu` and destroy only what QML created. A window of its own (the folder dialog) opened from a menu handler leaves the menu half-open: open it with `Qt.callLater`.
- Blur is behind the window only. Menus, the palette and dialogs are solid surfaces by design (D-017, `docs/DESIGN.md`).
- The app bundles static faces of the design's fonts (`crates/rustphin/fonts/`, every `.ttf` there is registered); a new weight in `docs/DESIGN.md` needs a new face there.

Testing with kwin-mcp:

- The user's Dolphin reports Ctrl+V as an ambiguous shortcut; in UI tests paste through Dolphin's context menu.
- Drags need waypoints with pauses of about 100 ms, and a short last move before the release, or the application does not see a drag or a drop. Holding a modifier during a drag (`keyboard_key_down`) is refused in this environment.
- Stop test instances by PID or by their exact path (`pkill -f target/debug/rustphin`). Never `pkill -x rustphin`: it also ends the maintainer's own Rustphin on the real desktop. A crashed test instance may need `kill -9 <pid>`.
- KDE adds its built-in places (Home, Trash, Network, …) to `user-places.xbel` the first time one of its applications reads a file that lacks them, so the file changes under Rustphin right after Dolphin starts in a fresh home.
- A debug build can take ten seconds or more to show its window in a kwin-mcp session; a small screenshot (a few KB) means nothing is on screen yet, not that the app crashed.
- Isolated kwin-mcp sessions still share the machine's system bus: the devices listed there are the user's real drives, and the trash menu lists their trashes. Never click a device row, its eject button or another volume's trash in a test; mount and unmount are tested against the mock service in `rustphin-platform`.
- A file operation's first progress report arrives when its plan is ready, before any change; tests that cancel "after n reports" count from there.
- kwin-mcp's smooth `mouse_scroll` does not move the list and its discrete scroll sends one event; use the scroll benchmark for sustained scrolling.
- Spell punctuation keys literally (`ctrl+=`, `ctrl+-`, `ctrl+shift+,`); names like `equal`, `minus` or `comma` are accepted but do nothing. Page Up is `Page_Up`, not `Prior`.
- Isolated KWin sessions may shift every window's colours between screenshots (Nautilus too); judge colour by the tokens and their contrast tests, not by a screenshot.
- Isolated KWin sessions offer no blur, so Rustphin shows its opaque theme there. Blur can only be reviewed on the real desktop; pass `RUSTPHIN_OPAQUE=1` to compare colours with the opaque mockups.
- Rustphin keeps running after its last window closes (W-110, D-031) and later launches hand over to it. After installing a new build, run `rustphin --quit` (or Ctrl+Q in a window), or the old process keeps answering launches. Isolated kwin-mcp sessions have their own session bus, so a test Rustphin never hands over to the maintainer's.
- Compare the app with the mockups as in UI_SCENARIOS.md "Mockup comparison": `tests/ui/make-mockup-home.sh` builds the scene in an isolated home.
