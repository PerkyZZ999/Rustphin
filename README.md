# Rustphin

A file manager for KDE Plasma on Wayland, written in Rust: a re-architecture of
KDE's Dolphin with a dark frosted-glass interface.

Status: slices S1–S10 are implemented and checked, including file operations,
tabs, places, the command palette, batch rename and desktop integration.
S11 hardening remains before the daily-driver trial. See
[`docs/project/STATE.md`](docs/project/STATE.md) for current limits and next steps.

## Build and run

Requires Rust through rustup (minimum 1.95.0), `rust-tc` and `just`, a C/C++
toolchain, pkg-config, and the Wayland, xkbcommon and fontconfig development
libraries. A Wayland session and working Vulkan driver are needed to run the
application; a display is not needed to build it. Rust-Toolchain setup is
documented in `../Rust-Toolchain/RUST-TOOLCHAIN.md`.

From the checkout root, build the locked release binary without launching it:

```bash
rust-tc build
```

The binary is `target/release/rustphin` (or under `CARGO_TARGET_DIR` if set).
For a specific installed compiler, the same build command is:

```bash
RUSTUP_TOOLCHAIN=1.95.0 rust-tc build
```

Use `RUSTUP_TOOLCHAIN=stable` for the installed stable toolchain. Check
`rustup check` first: the stable alias can lag the published stable release.
A build downloads locked crates if they are not cached; `CARGO_NET_OFFLINE=true`
uses an existing cache. Application runtime connections are documented below.

To build and launch in an actual desktop session:

```bash
rust-tc run ~/Documents
```

Several folders, or `file://` URLs, open as separate tabs. A path that names a file opens the folder that contains it. `assets/rustphin.desktop` registers Rustphin as a handler for folders.

Set `RUSTPHIN_OPAQUE=1` to turn off window translucency.

After an interrupted batch rename, open the affected folder, press Ctrl+Shift+P
and search `recover original`. **Recover original names after interruption**
validates the saved record and restores the original names. Keep recovery records
and temporary entries when recovery reports a problem: staged entries may hold
the only copy of an original file. Older batches without a record need manual
inspection; Rustphin never guesses from temporary names.

## Development

Checks run through the Rust-Toolchain command `rust-tc`:

```bash
rust-tc quick     # format, compile, lint, tests
rust-tc doctor    # the full gate, run before every commit
rust-tc validate  # full gate, then both headless performance benchmarks
```

`validate` runs the 500,000-entry listing/sort benchmark and the 10,000-location
palette benchmark sequentially after `doctor`; any failure makes the command
fail. The listing fixture is generated under `target/bench/listing-500k` and is
reused on later runs. Run on an otherwise idle machine for useful timings.
This headless check does not measure UI frame pacing or replace UI acceptance.

Contributor and agent instructions are in `AGENTS.md`.

For performance investigation, `rust-tc build-diagnostics` builds the optional
`diagnostics` feature using the pinned toolkit's profiler. In an isolated UI
session, set `RUSTPHIN_FRAME_LOG=1` for view/model/row timing and toolkit
layout, prepaint, paint, presentation and foreground-task timing.
`RUSTPHIN_SCROLL_BENCH=1` drives 1,200 frames. Active benchmark stalls are
counted even above 250 ms; idle exclusions and their longest gap are reported.
Intervals over 1.5 × the display's refresh period count as dropped frames, and
the run ends with a "scroll benchmark result" verdict against D-019's 5%.
See `docs/project/UI_SCENARIOS.md` for the fixture and memory protocol. Logs
contain local paths and source locations; this is opt-in local diagnosis.


## Privacy

[Privacy and local data](docs/project/PRIVACY.md) documents saved paths,
shared caches, logs and clearing instructions, including the audit limits.

## Licence

GPL-3.0-or-later. See `LICENSE`. Dependency licences and sources are checked
against the reviewed `deny.toml` policy by `rust-tc doctor`. The dated build
and dependency audit is recorded in `docs/project/VALIDATION.md`; distribution
notices and packaging remain release work.
