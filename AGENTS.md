# AGENTS.md

## Project overview

Rustphin is a file manager for KDE Plasma on Wayland, written in Rust. It is a re-architecture of KDE's Dolphin (not a line-by-line port) with a dark frosted-glass UI, place groups, a command palette and advanced batch rename.

- Language: Rust, edition 2024, minimum version 1.100 (D-035).
- UI: Qt 6 Quick and QML through cxx-qt 0.10 (D-034), with KDE Frameworks' `KWindowEffects` for blur. GPUI was replaced at W-114i (2026-11-14); the last GPUI version is commit `72ccb06`.
- Platform: Linux on Wayland only. No X11, macOS or Windows code.
- Licence: GPL-3.0-or-later. Every dependency must be GPLv3-compatible.
- Repository: branch `main`, pushed to GitHub at `https://github.com/PerkyZZ999/Rustphin.git` (remote `origin`, D-037). There is no hosted CI: `rust-tc doctor` run locally is the gate. Push only when the maintainer asks, and only commits that passed doctor.

## Read this first

The project follows a gated workflow. Its documents are the source of truth and live in `docs/project/`:

| File | Use it for |
|---|---|
| `STATE.md` | Current phase, blockers and the next action. Read it before doing anything; update it when you finish. |
| `WORK_ITEMS.md` | The queue for the current slice. Work from here. |
| `ROADMAP.md` | Slices S1–S11 and which requirement each covers. |
| `REQUIREMENTS.md` | Requirements R-001… with acceptance criteria. `ACCEPTANCE.md` maps the open criteria to their evidence. |
| `DECISIONS.md` | Locked decisions and assumptions. |
| `ARCHITECTURE.md` | Crates and modules, data flow, trust boundaries, failure modes, test strategy. |
| `VALIDATION.md` | Evidence log. Append an entry for every gate or check you actually ran. |
| `DISCOVERY.md` | Constraints, risks and measurements from the feasibility phase. |
| `UI_SCENARIOS.md` | The UI checks to replay through kwin-mcp after UI changes. |
| `KEYBOARD.md` | Every key, compared with Dolphin's. A test in `rustphin-view` (`commands`) checks the command table against it. |
| `PRIVACY.md` | Every file Rustphin reads or writes in the home folder, what it holds and how to clear it; connections. |
| `ADMIN_MODE.md` | Administrator mode: design and threat model. |
| `DESIGN_BRIEF.md`, `INFORMATION_ARCHITECTURE.md` | The W-079 design pass: brief and screen structure. The screens are drawn in `docs/mockup/` (HTML, one page per screen); `docs/DESIGN.md` holds the adopted design and says what is implemented. |

**The documents in `docs/` are the source of truth for behaviour and design.** A change to behaviour, a file written, a key, a command, a crate's responsibility or the look updates the relevant document first, or in the same commit. A document that disagrees with the code is a bug: fix whichever is wrong, and never leave the two apart. Dated records (VALIDATION.md entries, DECISIONS.md records, DISCOVERY.md findings) are history and are not rewritten; add a new entry instead.

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
| `rustphin-core` | The directory model and its snapshots (`model`, `entry`, `details`), sorting, the name filter, selection, recent folders, fuzzy matching (`search`), permission bits (`mode`), the batch-rename plan (`rename`), archive formats with the entry-name and link rules (`archive`), and the operation engine (`ops`: requests, undo, the batch-rename journal, compress and extract, a fault-injecting filesystem for tests). The engine reaches the filesystem, the trash and the archive formats it does not read itself only through the `Fs`, `Trash` and `ForeignReader` traits it is given | Depend on Qt, cxx, Wayland or D-Bus crates, or on libarchive (which reaches the engine through `ForeignReader`) |
| `rustphin-platform` | XDG locations and atomic writes of Rustphin's files (`xdg`, `config`), the file clipboard, UDisks2 devices, the freedesktop trash, thumbnails, KDE's places file, MIME types, icon themes, applications and `mimeapps.list`, `kdeglobals`, templates, the terminal, properties, the FileManager1 and single-instance services, autostart, `/proc` facts, the administrator helper client (`AdminFs`), kio-fuse remote folders, the byte-preserving `file://` URI codec, and the reader of other archive formats (`libarchive`, `archive_tool::ForeignArchives` with its 7-Zip/unrar fallback). The only crate allowed `unsafe` | Depend on Qt or cxx |
| `rustphin-view` | Interface logic free of any toolkit, as plain data and functions: the command table and keys (`commands`); one pane's state between snapshots (`pane_state`) and its wording (`pane`); what an action on the selection means (`selection_actions`); the operations' sequencing, wording, questions and speed (`operation_board`, `operations`, `questions`, `transfer`); tabs and split views (`workspace`, `tabs`, `session`); the places panel's state, rows and groups (`places_panel`, `groups`, `place_menu`); context menus (`menu`, with `OpenMenu`); palette results and recent folders (`palette`, `history`); settings, Settings pages, each folder's view and the policy for damaged saved files (`settings`, `settings_form`, `preferences`, `folder_views`, `saved_file`); the toolbar's items, dialogs, Properties, archive menus, remote-folder wording, view geometry, formats, icon names; the design tokens (`colors`, `metrics`) | Depend on Qt or cxx |
| `rustphin` | The binary and its Qt interface: QML (`qml/`); Rust QObjects through cxx-qt (`src/`), which turn `rustphin-view`'s decisions into Qt models and signals; the hub (`hub.rs`), which owns the engine's runners, the clipboard and the timers; paths across the QML boundary (`qml_path.rs`); the C++ shell (`cpp/`: windows, the QML engine, fonts, `KWindowEffects` blur) | Touch the filesystem directly for changes; hold logic that `rustphin-view` can hold; hand QML a path as plain text |

The layering is enforced by `rust-tc layers` (part of `rust-tc doctor`): the core may not depend on GPUI, Wayland, zbus or cxx crates; the platform layer and `rustphin-view` not on GPUI or cxx; nothing on GPUI. `deny.toml` bans `gpui-kit` and `gpui-pre`, and every HTTP client, TLS stack and telemetry crate (R-108, D-018).

- No work on the UI thread: listing, metadata, sorting, thumbnails and file operations run on the core's own worker pools and reach the UI as immutable snapshots over channels. No Tokio or other general async runtime in the core (D-009).
- All file mutations go through the core's operation engine (D-011): write under a temporary name then rename; verify before deleting a move's source; never overwrite or permanently delete without an explicit user choice; delete means trash.
- File names are bytes. Never assume UTF-8; never `unwrap` a name conversion. A path goes to QML as its `file://` URI and a bare name percent-encoded (`qml_path`, W-132); lossy text is for display only.
- Rustphin's own files are written through `config::write_atomic` (private temporary file, sync, rename). A saved file that cannot be read, is damaged or is from a newer format is kept as it is for the whole run and the user is told (`saved_file` for settings, place groups and folder views; `session` for the saved tabs). PRIVACY.md lists every such file.
- The file clipboard lives in `rustphin-platform` behind a trait with runtime detection and a fallback (D-010). Blur is KDE's `KWindowEffects`, asked for by the C++ shell, with the opaque theme when KWin offers none.
- Place groups are stored in Rustphin's own config, never inside KDE's `user-places.xbel` (D-008).

## Setup

Requirements: Rust through rustup (stable, plus the `1.100.0` toolchain for the minimum-version check), a Wayland session, a C++ compiler, the Qt 6 development packages (Base, Declarative, Svg, Wayland; `qmake6` on the path; `qmlformat` and `qmllint` in `/usr/lib/qt6/bin`), KDE Frameworks 6 WindowSystem (`KWindowEffects`), libarchive (linked by `rustphin-platform`), `python3` and the maintainer's qmllint BestPractices plugin for `rust-tc qml-lint` (from `../QtQmlBestPractices` beside this repository, or the folder `QML_BEST_PRACTICES` names; built there, or built into `target/` when `cmake` is installed), and the Rust-Toolchain tools: `rust-tc`, `just`, `cargo-deny`, `cargo-shear`, `cargo-nextest`, `cargo-hack` (`cargo-llvm-cov` for optional coverage, `cargo-mutants` for mutation testing, nightly Rust with `cargo-fuzz` and Miri for fuzzing and `rust-tc miri`). Install instructions are in `../Rust-Toolchain/RUST-TOOLCHAIN.md`. At run time Rustphin also uses, when installed: kio-fuse (remote folders), kio-admin (administrator mode), 7-Zip or unrar (archives libarchive cannot read).

```bash
rust-tc run                # build (release) and run, opening the home folder
rust-tc run ~/Documents    # open a specific folder
```

Command line: `rustphin [folder…]` opens the folders (a file opens the folder that holds it), else the saved tabs; `--background` starts without a window; `--quit` ends the running Rustphin.

Environment variables: `RUSTPHIN_ADMIN_BUS=session` (UI tests only) makes administrator mode look for KDE's kio-admin helper on the session bus, where `cargo run -p rustphin-platform --example fake_admin_helper -- <sandbox>` stands in for it; never test administrator mode against the real helper, which would ask for the maintainer's password on the real desktop (UI_SCENARIOS.md, AD1–AD4). `RUSTPHIN_OPAQUE=1` forces the opaque theme; `RUSTPHIN_LOG=debug` sets the log level (default `info`; logs go to stderr). Qt and QML warnings go to the journal unless `QT_FORCE_STDERR_LOGGING=1` is set; `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` exposes the window over AT-SPI in test sessions. `RUSTPHIN_FRAME_LOG=1` makes each opened folder log a "first entries" line on stderr: the time from asking for the listing to the frame that showed its rows, and Rustphin's own memory (W-073g). `RUSTPHIN_SCROLL_BENCH=1` does the same and also makes the active list scroll itself, once a folder of more than 1,000 entries has finished loading; every 1,200 frames it logs a "scroll benchmark result" line, counting as dropped each interval over 1.5 × the median and passing at most 5% dropped (D-019, `pane::frame_report`). That is how frame rate is measured (see W-013 and W-085 in `docs/project/VALIDATION.md`). Tests of the core also use `RUSTPHIN_TEST_KILL_*` and `RUSTPHIN_TEST_BATCH_MODE` for their child processes; never set them yourself.

Dependencies are optimised even in debug builds, so a debug window opens about as fast as a release one. `crates/rustphin/build.rs` compiles the QML with qmlcachegen and finds every `*_bridge.rs` and `qml/*.qml` itself: a QML error fails the build, and a new bridge or QML file needs no list edited.

## Quality gate: Rust-Toolchain

`rust-tc` is the only check entry point. It wraps the repository `justfile`; do not tell anyone to run `just` directly, and do not add parallel homemade check scripts.

| Command | Runs | When |
|---|---|---|
| `rust-tc check` | compiler only | fastest feedback while editing |
| `rust-tc quick` | format, check, clippy, tests, doctests | after meaningful edits |
| `rust-tc doctor` | format, `qml-format`, clippy, tests, doctests, `qml-lint`, `cargo-deny`, `cargo-shear`, feature checks, `layers`, `msrv`, `fuzz-check` (the fuzz harness's format, clippy and dependency policy) | after every task, before committing |
| `rust-tc qml-format`, `qml-lint` | `qmlformat` check of every QML file (settings in `crates/rustphin/qml/.qmlformat.ini`); `qmllint` with the BestPractices plugin, no warning allowed (W-116) | part of doctor; alone after QML edits |
| `rust-tc build` | the locked release application, linked, not launched | when a linked build is the evidence (R-109) |
| `rust-tc validate` | doctor, then both headless benchmarks | before acceptance work |
| `rust-tc fuzz <target> [seconds]` | bounded `uri`, `places`, `desktop`, `images`, `admin_entry` or `archive_entry` parser campaign; defaults to 30 seconds | after relevant parser changes; requires nightly and cargo-fuzz; see `fuzz/README.md` |
| `rust-tc bench-listing` | 500,000-entry listing benchmark; fails if R-101's thresholds are missed | after changing listing, sorting or the model |
| `rust-tc bench-palette` | fuzzy ranking of 10,000 recent locations; fails above R-015's 50 ms | after changing `search` or the palette's candidates |
| `rust-tc deep`, `mutants`, `miri`, `coverage` | feature powerset; cargo-mutants; Miri on nightly; LCOV report | on demand, not after every edit |

Hard rules:

- Do not add `cargo-audit` (`cargo-deny` owns advisories). Do not invoke the separate `rust-doctor` binary. Do not add SonarQube.
- Do not blanket-allow lints. An `allow` needs an understood, intentional reason, the narrowest scope, and a comment when not obvious.
- Do not invent fuzz targets to satisfy the toolchain.
- `deny.toml` is reviewed for this project. Its three advisory ignores record reviewed "unmaintained" notices for crates retained by the zbus 5 integrations; re-check them at every zbus upgrade. New licences, exceptions or ignores need a comment saying why; a banned network, TLS or telemetry crate needs a new decision (D-018).
- `rust-tc miri` is expected for changes to the `unsafe` code in `rustphin-platform` where it can run.

## Development workflow

- Keep `cxx-qt` on its 0.10 line; an upgrade is its own planned work item. Never change toolkit versions as a side effect of another task.
- Add dependencies sparingly and check the licence first.
- `spikes/` is throwaway reference code from Discovery, outside the workspace. Port ideas from it; do not depend on it or extend it.
- When unsure of a cxx-qt API, read the source under `~/.cargo/registry/src/*/cxx-qt-0.10.*/` and `cxx-qt-lib-0.10.*/`; for Qt, the documentation of the installed version (Qt 6.12 on the development machine).

## Testing

Follow the testing-suite skill. Layout and tools:

- Unit tests live in `#[cfg(test)] mod tests` beside the code (or a `tests.rs` beside a large module); runner is `cargo-nextest` through `rust-tc` (`.config/nextest.toml`).
- Core: behaviour through the public model and engine APIs where possible; fakes of the `Fs` trait instead of mocks; fault injection through `ops/faults.rs` (the real filesystem failing at chosen steps; other crates' tests get it with the `test-support` feature) for every file operation; kill tests run a child process of the test binary; property tests with `proptest` (failures kept in `proptest-regressions/`); no sleeps (wait on model or engine events).
- Platform: fixture and contract tests (`tests/fixtures/`, temporary homes through injectable XDG locations); D-Bus services against a private peer-to-peer bus; stand-in scripts for 7-Zip and unrar.
- View: every decision in `rustphin-view` is tested headlessly; the command table is checked for clashing keys and against KEYBOARD.md.
- Interface: the pure Rust in `crates/rustphin/src` (paths across QML, model roles, toolbar tooltips, icon names) and a few QML contracts read from the files are unit-tested; QML itself is checked by `qml-lint` and `qml-format` and by the UI scenarios.
- Parsers of untrusted input have fuzz targets (`fuzz/`, `rust-tc fuzz`); turn a finding into a regression test beside the parser.
- Mutation testing: `rust-tc mutants` (cargo-mutants). A surviving mutant is killed by a new test or recorded as equivalent, with the reason, in VALIDATION.md (W-134, W-139, W-142).
- UI: verify it yourself. After any change to the interface (`crates/rustphin`), replay the relevant scenarios in `docs/project/UI_SCENARIOS.md` through the kwin-mcp tools (load the `anthropic-skills:kwin-mcp` skill) and record what you saw. Do not ask the user to launch the app and report back; ask them only about matters of taste, or what the tools cannot show.
- UI: pure logic (selection movement, path expansion, formatting, token contrast) is unit-tested. Interaction is checked by scripted scenarios in an **isolated KWin session with an isolated home directory** (kwin-mcp `session_start` with `isolate_home: true`). Never run test instances of Rustphin or Dolphin against the user's real home, and never use the user's real files as test data. Wait for the window before sending input: the app takes a moment to start.
- File-operation UI must not be enabled before the operation engine's fault-injection suite passes.

Performance targets on a 500,000-entry directory (R-101): first entries within 200 ms, re-sort within 500 ms, 60 fps scrolling, no UI-thread task over 16 ms.

## Code style

- `cargo fmt` and `cargo clippy` clean, with warnings denied (both run by `rust-tc`). Workspace lints deny `unwrap`, `expect`, `dbg!` and `todo!` outside tests.
- Errors: typed errors in library crates (`thiserror`-style enums); no `unwrap`/`expect` on fallible I/O outside tests.
- `unsafe` only in `rustphin-platform`, each block with a `// SAFETY:` comment. The one exception is `rustphin` (D-034): cxx-qt generates unsafe FFI glue, so each bridge sits alone in a `*_bridge.rs` file of declarations, included with `#[allow(unsafe_code)]`; logic stays in the safe files beside it. Its `build.rs` also allows it for the one `cc_builder` call that adds KWindowSystem's include path.
- Public items in `rustphin-core` and `rustphin-platform` carry doc comments.
- No telemetry and no network access anywhere in the application.

## Commits

- Commits on `main`; small and focused, one work item or coherent step each. Push to `origin` only when the maintainer asks (D-037).
- Message: imperative summary line (≤ 72 characters), blank line, then why. Reference the work item or requirement, for example `W-004` or `R-101`.
- Commit after every task, once `rust-tc doctor` passes and `docs/project/` reflects what changed. Never commit with failing checks.

## Reference material

- Qt 6 documentation: https://doc.qt.io/qt-6/ (QML types, Qt Quick, Qt Quick Controls)
- cxx-qt book: https://kdab.github.io/cxx-qt/book/
- KWindowEffects (KF6 WindowSystem): https://api.kde.org/frameworks/kwindowsystem/html/
- Dolphin source, for behaviour reference only: https://github.com/kde/dolphin

## Known gotchas

Qt and QML:

- Keys are written once, in `rustphin_view::commands` (KEYBOARD.md); Main.qml makes a window `Shortcut` for each command from that table (`Shell.shortcuts()`). Two shortcuts sharing a sequence (`"Alt+Left"` and `StandardKey.Back` are one key on Linux) are ambiguous and Qt fires neither, so the table's tests refuse a clash. One press matching two sequences of the same command (Shift+= is also "+", Shift+, also "<") is also reported as ambiguous; the shortcut's `onActivatedAmbiguously` runs the command.
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
- Isolated KWin sessions offer blur, so Rustphin shows its glass theme there, over whatever the session draws behind the window (black, or white in frames taken with a delay). Pass `RUSTPHIN_OPAQUE=1`, or choose Opaque in Settings, to compare colours with the opaque mockups. How the blur looks can only be judged on the real desktop. Until W-146c Rustphin asked KWin about blur too early and always started opaque.
- Rustphin keeps running after its last window closes (W-110, D-031) and later launches hand over to it. After installing a new build, run `rustphin --quit` (or Ctrl+Q in a window), or the old process keeps answering launches. Isolated kwin-mcp sessions have their own session bus, so a test Rustphin never hands over to the maintainer's.
- Compare the app with the mockups as in UI_SCENARIOS.md "Mockup comparison": `tests/ui/make-mockup-home.sh` builds the scene in an isolated home.
