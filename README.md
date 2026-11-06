# Rustphin

A file manager for KDE Plasma on Wayland, written in Rust: a re-architecture of
KDE's Dolphin with a dark frosted-glass interface.

Status: slices S1–S10 are implemented and checked, including file operations,
tabs, places, the command palette, batch rename and desktop integration.
S11 hardening remains before the daily-driver trial. See
[`docs/project/STATE.md`](docs/project/STATE.md) for current limits and next steps.

## Build and run

Requires Rust (stable, 1.95 or later), a Wayland session and a Vulkan driver.

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
```

Contributor and agent instructions are in `AGENTS.md`.

For performance investigation, `rust-tc build-diagnostics` builds the optional
`diagnostics` feature using the pinned toolkit's profiler. In an isolated UI
session, set `RUSTPHIN_FRAME_LOG=1` for view/model/row timing and toolkit
layout, prepaint, paint, presentation and foreground-task timing.
`RUSTPHIN_SCROLL_BENCH=1` drives 1,200 frames. Active benchmark stalls are
counted even above 250 ms; idle exclusions and their longest gap are reported.
See `docs/project/UI_SCENARIOS.md` for the fixture and memory protocol. Logs
contain local paths and source locations; this is opt-in local diagnosis.


## Privacy

[Privacy and local data](docs/project/PRIVACY.md) documents saved paths,
shared caches, logs and clearing instructions, including the audit limits.

## Licence

GPL-3.0-or-later. See `LICENSE`.
