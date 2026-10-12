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

Rules that follow from it:

- **Locked decisions are binding.** D-001, D-002, D-003, D-006 to D-011 may not be changed or worked around. If one blocks you, stop and raise it with the user; changing it needs their approval and a superseding record.
- **Evidence over claims.** Never record a check as passed unless you ran it. Say what was observed, inferred or not verified.
- **One slice at a time.** Implement the current slice's work items; do not start later slices' features.
- Ask the user before: changing MVP scope, locking or changing a decision, anything destructive, publishing anything.

## Architecture rules

Cargo workspace, dependencies point one way: `rustphin-ui` → `rustphin-platform` → `rustphin-core`.

| Crate | Contains | Must not |
|---|---|---|
| `rustphin-core` | Directory model, listing, sorting, selection, history, file operation engine, undo, batch rename, fuzzy matching | Depend on GPUI, Wayland or D-Bus crates |
| `rustphin-platform` | Blur, file clipboard, devices, trash, thumbnails, places, MIME, icons | Depend on `gpui-kit` |
| `rustphin-ui` | Theme, views, key bindings, bridge to the core | Touch the filesystem directly |
| `rustphin` | Binary; wiring only | Hold logic |
| `xtask` | Developer commands | Ship |

- No work on the UI thread: listing, metadata, sorting, thumbnails and file operations run on the core's own worker pools and reach the UI as immutable snapshots over channels. No Tokio or other general async runtime in the core (D-009).
- All file mutations go through the core's operation engine (D-011): write under a temporary name then rename; verify before deleting a move's source; never overwrite or permanently delete without an explicit user choice; delete means trash.
- File names are bytes. Never assume UTF-8; never `unwrap` a name conversion.
- Wayland integrations (blur, clipboard) live in `rustphin-platform` behind traits with runtime detection and a fallback (D-010). Do not fork GPUI.
- Place groups are stored in Rustphin's own config, never inside KDE's `user-places.xbel` (D-008).

## Setup

Requirements: Rust (rustup, stable ≥ 1.95), a Wayland session, a working Vulkan driver, and the Wayland, xkbcommon and fontconfig development packages.

The Cargo workspace is created by work item W-001. Until it lands, the only buildable code is the disposable spikes:

```bash
cargo build --release --manifest-path spikes/gpui-toolkit/Cargo.toml
cargo build --release --manifest-path spikes/gpui-kit-check/Cargo.toml
cargo build --release --manifest-path spikes/listing-bench/Cargo.toml
```

A cold GPUI build takes about two to three minutes.

When W-001 and W-002 land, replace this section and the next two with the real workspace commands, and verify each one runs.

## Development workflow

- Keep `gpui-kit` pinned exactly. Never run `cargo update` for it or change its version as a side effect; an upgrade is its own planned work item.
- Add dependencies sparingly and check the licence first.
- `spikes/` is throwaway reference code from Discovery. Port ideas from it; do not depend on it or extend it.

## Testing

Planned strategy (see `ARCHITECTURE.md`); commands are added as each piece exists:

- Core: unit and property tests; fault-injection tests through the `Fs` trait for every file operation; headless benchmarks on generated 500,000-entry directories.
- Platform: fixture and contract tests; D-Bus services against a mocked bus.
- UI: scripted scenarios in an **isolated KWin session with an isolated home directory** (kwin-mcp `session_start` with `isolate_home: true`). Never run test instances of Rustphin or Dolphin against the user's real home, and never use the user's real files as test data.
- Add or update tests with every change. File-operation UI must not be enabled before the operation engine's fault-injection suite passes.

Performance targets on a 500,000-entry directory (R-101): first entries within 200 ms, re-sort within 500 ms, 60 fps scrolling, no UI-thread task over 16 ms.

## Code style

- `cargo fmt` and `cargo clippy` clean, with warnings denied.
- Errors: typed errors in library crates (`thiserror`-style enums); no `unwrap`/`expect` on fallible I/O outside tests.
- `unsafe` only in `rustphin-platform`, each block with a `// SAFETY:` comment.
- Public items in `rustphin-core` and `rustphin-platform` carry doc comments.
- No telemetry and no network access anywhere in the application.

## Commits

- Local commits on `main`; small and focused, one work item or coherent step each.
- Message: imperative summary line (≤ 72 characters), blank line, then why. Reference the work item or requirement, for example `W-004` or `R-101`.
- Before committing: format, lint and tests pass, and `docs/project/` reflects what changed (`WORK_ITEMS.md`, `VALIDATION.md`, `STATE.md`).

## Reference material

- GPUI source: https://github.com/zed-industries/zed/tree/main/crates/gpui (the pinned snapshot is older than `main`; check the `gpui-pre` 0.3.9 source when an API differs)
- GPUI-Kit documentation index: https://gpui-kit.com/llms.txt
- Dolphin source, for behaviour reference only: https://github.com/kde/dolphin

## Known gotchas

- GPUI's own clipboard cannot exchange files with other applications, and its blur uses a protocol current KWin does not offer. Use the platform layer's implementations.
- GPUI cannot blur content inside the window. Menus, the palette and dialogs are tinted opaque surfaces by design (D-007).
- The user's Dolphin reports Ctrl+V as an ambiguous shortcut; in UI tests paste through Dolphin's context menu.
