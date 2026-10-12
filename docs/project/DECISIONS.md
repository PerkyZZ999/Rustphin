# Decisions

Owner of every record: the maintainer. Evidence references point to VALIDATION.md and DISCOVERY.md.

Status on 2026-10-12: D-001 to D-003, D-006, D-008 to D-011 and D-017 are **locked** (confirmed by the maintainer); D-007 is superseded by D-017. Changing one requires the change-impact loop. D-004, D-005 and D-012 to D-016 are **assumptions**: cheap to reverse, changeable without a change-impact loop.

---

## D-001 — UI toolkit: GPUI through GPUI-Kit, pinned
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** The frontend framework dictates project structure. Candidates were GPUI and Qt6/QML via cxx-qt. Requirements R-003, R-010, R-101, R-105 depend on it.
- **Decision:** Build the UI on `gpui-kit`, pinned exactly (`=0.7.3`, which pins `gpui-pre` `=0.3.9`). Upgrades are planned work items, never incidental.
- **Rationale:** Whole stack in Rust including the item view; virtualised lists suited to very large directories; design freedom for a custom theme; GPUI-Kit supplies components (sidebar, tabs, menus, command list, resizable panels) and a crates.io release compatible with the 1.95 minimum.
- **Evidence:** Spikes of 2026-10-12: 500k-row list renders; Breeze SVG icons load; drag-out to and drop-in from Dolphin work; same sources build and pass drag-out on GPUI-Kit 0.7.3. GPUI integrates AccessKit (source read).
- **Consequences:** Pre-1.0 API churn; no in-window backdrop blur; file clipboard and compositor blur are not provided and are ours to build (D-010); GPUI-Kit components themselves are untried; KDE look-and-feel integration is manual.
- **Alternatives:** Qt6/QML via cxx-qt (rejected: item view and UI logic would sit outside Rust; retained as fallback). Tracking Zed HEAD GPUI directly (rejected: pins Rust 1.98.1, no component library).
- **Reopen criteria:** A GPUI-Kit release removes drag-out or raw window handles; a blocking rendering or input defect with no fix or workaround within a slice; GPUI-Kit is abandoned.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-002 — Workspace with a headless, UI-agnostic core
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** Toolkit risk (D-001) and data-safety requirements (R-007, R-103) both call for logic that runs and is tested without a display.
- **Decision:** A Cargo workspace of three layers with one-way dependencies: `rustphin-core` (directory model, sorting, selection, operation engine, undo, batch rename, fuzzy matching; no UI or desktop dependencies) ← `rustphin-platform` (Wayland, D-Bus, UDisks2, trash, thumbnails, places, MIME, icons; behind traits) ← `rustphin-ui` (GPUI views only). Core and platform never depend on GPUI.
- **Rationale:** Keeps the toolkit replaceable; lets file operations be tested exhaustively without a display; makes performance measurable headlessly.
- **Evidence:** Listing benchmark ran headless (VALIDATION.md).
- **Consequences:** Some state is mirrored between core snapshots and views; trait boundaries add indirection; discipline is needed to keep GPUI types out of the lower layers (enforced by `rust-tc layers` and `deny.toml`).
- **Alternatives:** Single crate (rejected: couples logic to a pre-1.0 toolkit).
- **Reopen criteria:** The boundary causes measurable overhead in the view (R-101 misses) or persistent duplicated state that tests cannot keep consistent.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-003 — Licence: GPL-3.0-or-later
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** Rustphin is written with reference to Dolphin's source (GPL-2.0-or-later) and depends on Apache-2.0 crates.
- **Decision:** GPL-3.0-or-later for all Rustphin code.
- **Rationale:** Compatible with deriving from Dolphin and with Apache-2.0 dependencies (Apache-2.0 is not compatible with GPL-2.0-only).
- **Evidence:** Licences read from crate manifests: gpui-pre Apache-2.0, gpui-kit Apache-2.0. Not legal advice.
- **Consequences:** Every dependency must be GPLv3-compatible; a licence audit is a Release gate item; the project cannot later be relicensed permissively without removing Dolphin-derived parts.
- **Alternatives:** GPL-2.0-or-later (rejected: Apache-2.0 conflict); permissive licence with clean-room discipline (rejected: forfeits Dolphin's source as a reference).
- **Reopen criteria:** A required dependency has a GPLv3-incompatible licence.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-004 — Network locations via kio-fuse
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** Remote locations are reached through kio-fuse mount paths; no native protocol code in the MVP.
- **Reopen criteria:** kio-fuse is unavailable or too slow for daily use.

## D-005 — Minimum Rust version 1.95
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** MSRV 1.95, edition 2024; development on current stable (1.98.1 installed). The local check command builds both.
- **Reopen criteria:** A needed dependency or language feature requires a newer compiler.

## D-006 — Platform scope: Linux, Wayland only, Plasma first
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** The author uses Plasma on Wayland. Blur, clipboard and drag-and-drop all rely on Wayland protocols.
- **Decision:** Support Linux on Wayland only, tested on KDE Plasma 6. No X11, macOS or Windows code paths. Other Wayland compositors are expected to run with features degrading by protocol availability, but are not tested for the MVP.
- **Rationale:** Removes whole classes of work (X11 drag-and-drop, per-platform clipboard) and matches the user.
- **Evidence:** User decision 2026-10-12; protocol list observed on the reference machine (DISCOVERY.md).
- **Consequences:** The X11 backend of GPUI is not compiled in; users on X11 cannot run Rustphin; platform code may assume Wayland types.
- **Alternatives:** Wayland plus degraded X11 (rejected by the user).
- **Reopen criteria:** The project gains users or contributors who need another platform.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-007 — Visual direction: one dark frosted-glass theme
- **Status:** superseded
- **Date:** 2026-10-12
- **Context:** User's direction: modern, classy, professional "obsidian frosted glass" with royal-blue accents and progressive disclosure. GPUI cannot blur in-window content.
- **Decision:** A single dark theme defined by design tokens. The window background is translucent with compositor blur; surfaces floating inside the window (menus, palette, dialogs) are tinted opaque. An opaque mode applies automatically when blur is unavailable and on user request. File-type icons come from the system icon theme; the system colour scheme is not followed. Progressive disclosure governs layout (R-106).
- **Rationale:** Delivers the requested look within what the toolkit and compositor can do, and keeps text readable everywhere.
- **Evidence:** Blur observed on the real desktop via `ext-background-effect-v1` (VALIDATION.md).
- **Consequences:** Rustphin does not match the user's Plasma colour scheme; two surface treatments must be designed and contrast-checked; a light theme is later work (L-009).
- **Alternatives:** Follow system colours (declined by the user); Breeze imitation (declined).
- **Reopen criteria:** Blur proves unreliable in daily use; the user wants a light variant or system-colour following.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** superseded by D-017 (2026-10-12)

## D-008 — Place groups stored beside, not inside, the shared places file
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** R-012. KDE's `user-places.xbel` is a flat list shared by Dolphin and file dialogs, and Dolphin rewrites it routinely.
- **Decision:** Ordinary places are read from and written to the shared file. Groups (name, order, collapsed state, member place IDs) live in Rustphin's own file under its XDG config directory, keyed by each bookmark's KDE `<ID>`. Rustphin never adds elements to the shared file that KDE does not define.
- **Rationale:** Dolphin and file dialogs keep working unchanged; no risk of other apps dropping or corrupting unknown structure.
- **Evidence:** Structure of the shared file inspected on the reference machine (DISCOVERY.md). Whether KDE preserves unknown elements was not tested.
- **Consequences:** Groups are invisible to other KDE apps; membership must be reconciled when IDs disappear or the shared file changes underneath; two files to keep consistent.
- **Alternatives:** Custom metadata or `<folder>` elements inside the shared file (rejected: untested interop risk to a file other apps depend on).
- **Reopen criteria:** KDE adds native groups, or testing shows a namespaced metadata element round-trips safely through Dolphin.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-009 — Concurrency model: runtime-agnostic core with its own workers
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** R-101 forbids listing, metadata, sorting or thumbnail work on the UI thread. GPUI has its own executors; the core must not depend on GPUI (D-002).
- **Decision:** `rustphin-core` exposes synchronous, cancellable operations and runs them on worker pools it owns (a CPU pool for sort and metadata, bounded I/O workers for file operations and thumbnails). Results reach consumers as immutable snapshots and events over channels. No general async runtime (such as Tokio) in the core. The UI layer bridges channels onto GPUI's executor.
- **Rationale:** Filesystem calls are blocking; the measured wins come from thread parallelism (about tenfold on stat and sort). Keeping the core free of an async runtime keeps it usable from any frontend and simple to test.
- **Evidence:** Listing benchmark: stat 579 ms → 53–60 ms, sort 181 ms → 26–29 ms with a thread pool (VALIDATION.md).
- **Consequences:** Cancellation and back-pressure are designed by hand; snapshot hand-off needs care to avoid copying large lists; D-Bus code in the platform layer may run its own small executor.
- **Alternatives:** Tokio throughout (rejected: second runtime beside GPUI's, little benefit for blocking filesystem work); io_uring (deferred: optimisation, not architecture).
- **Reopen criteria:** Benchmarks show the model cannot meet R-101, or network/FUSE latency demands true async I/O.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-010 — Own the Wayland integrations in the platform layer
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** GPUI provides neither a file clipboard nor blur on current KWin (R-011, R-105).
- **Decision:** `rustphin-platform` implements file clipboard (data-control protocol) and background blur (`ext-background-effect-v1`, using GPUI's Wayland connection and surface through raw window handles), each behind a trait with runtime protocol detection and a defined fallback: in-app clipboard only, and the opaque theme. No fork of GPUI. Improvements are offered upstream where practical.
- **Rationale:** Proven to work without forking; isolates compositor-specific code; fallbacks keep the app usable elsewhere.
- **Evidence:** Clipboard copy and cut both ways with Dolphin; blur confirmed by the user (VALIDATION.md).
- **Consequences:** We maintain unsafe FFI-adjacent code that borrows GPUI's connection; a toolkit upgrade can break it, so the spike scenarios become regression tests; behaviour on non-KDE compositors is unverified.
- **Alternatives:** Fork GPUI (rejected: maintenance burden against a fast-moving upstream); wait for upstream (rejected: blocks the MVP).
- **Reopen criteria:** GPUI gains equivalent features; the borrowed-connection technique stops working.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-011 — File operation safety model
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** R-007, R-008, R-103. Losing user data is the project's most severe risk.
- **Decision:** All mutations go through one operation engine in the core. Rules: copies are written under a temporary name in the destination directory and renamed into place on success; a cross-filesystem move deletes the source only after the copy completes and is verified; nothing is overwritten or permanently deleted without an explicit choice made in that operation; delete means trash by default; every operation records what is needed to undo it; batch operations are validated in full before the first change. The UI never touches the filesystem directly.
- **Rationale:** One audited path for destructive actions, testable with fault injection, with crash behaviour defined in advance.
- **Evidence:** None beyond design reasoning; to be proven by the engine's test gate before any use on real data.
- **Consequences:** Temporary files are visible briefly in destination folders; verification costs time on large moves; undo history is session-scoped; the engine must be finished and gated before file-operation UI ships.
- **Alternatives:** Direct calls from UI handlers (rejected: unauditable); delegating to KIO (rejected: C++ dependency, contradicts the project's aim).
- **Reopen criteria:** The rules prove incompatible with a required filesystem (for example FUSE mounts without atomic rename).
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** —

## D-012 — Configuration format
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** TOML files under `$XDG_CONFIG_HOME/rustphin/`, state under `$XDG_STATE_HOME/rustphin/`, written atomically.

## D-013 — D-Bus library
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** `zbus` for UDisks2 and `org.freedesktop.FileManager1`.

## D-014 — Filesystem watching
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** inotify through the `notify` crate, with coalescing in the core.

## D-015 — Clipboard implementation
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** Start with `wl-clipboard-rs` as proven in the spike; replace with a direct data-control implementation if it limits us.

## D-016 — Repository and hosting
- **Status:** assumption
- **Date:** 2026-10-12
- **Decision:** Local git repository on `main` only (user, 2026-10-12): no remote and no hosted CI. The checks a CI would run are `rust-tc doctor` (the user's Rust-Toolchain), run locally after every task and before each commit.

## D-017 — Visual design follows docs/DESIGN.md
- **Status:** locked
- **Date:** 2026-10-12
- **Context:** The user added `docs/DESIGN.md`, generated by another agent, as the intended main design document, and asked for it to be reviewed and refined. The draft disagreed with D-007 and with the toolkit in places: an indigo accent, square corners, monospace for all text, backdrop blur on in-window surfaces, and features Rustphin does not have.
- **Decision:** `docs/DESIGN.md`, as refined on 2026-10-12, is the reference for Rustphin's visual design; its front-matter tokens are authoritative and `crates/rustphin-ui/src/theme.rs` implements them. Chosen by the user during the review: royal-blue accent (not indigo); 2-pixel corners (not square, not 6 px); JetBrains Mono for data only, Inter for file names and interface text, Space Grotesk for headings. Carried over from D-007: one dark theme; window glass blurred by the compositor; surfaces floating inside the window are solid; an opaque mode when blur is unavailable or switched off; file-type icons from the system icon theme; the system colour scheme is not followed; progressive disclosure.
- **Rationale:** One written source for design decisions that agents and people can follow, kept honest about what the toolkit can do.
- **Evidence:** The refined document passes the official linter with no errors (seven "token not referenced by a component" warnings for colours used only in prose). The theme's contrast tests pass with the document's values. The applied radius and fonts were seen in an isolated session (VALIDATION.md).
- **Consequences:** Design changes are made in `docs/DESIGN.md` first, then in the theme. The three font families are assumed installed and fall back to the system font otherwise; bundling them is undecided. Components the document describes but the app lacks (floating surfaces, places rail, information panel) follow it when built.
- **Alternatives:** Adopt the draft unchanged (rejected by the user's choices above); keep D-007 without a design document (rejected: the user wants the document to lead).
- **Reopen criteria:** The user wants a different accent, radius or type system; a light theme is added; a toolkit upgrade makes in-window blur possible.
- **Confirmed by / date:** The maintainer, 2026-10-12
- **Supersedes / superseded by:** supersedes D-007
