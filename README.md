# Rustphin

A file manager for KDE Plasma on Wayland, written in Rust: a re-architecture of
KDE's Dolphin with a dark frosted-glass interface.

Status: early development. Slice S1 (open and browse a folder) is implemented;
file operations are not. See `docs/project/` for the plan and its evidence.

## Build and run

Requires Rust (stable, 1.95 or later), a Wayland session and a Vulkan driver.

```bash
cargo run --release --package rustphin -- ~/Documents
```

Set `RUSTPHIN_OPAQUE=1` to turn off window translucency.

## Development

Checks run through the Rust-Toolchain command `rust-tc`:

```bash
rust-tc quick     # format, compile, lint, tests
rust-tc doctor    # the full gate, run before every commit
```

Contributor and agent instructions are in `AGENTS.md`.

## Licence

GPL-3.0-or-later. See `LICENSE`.
