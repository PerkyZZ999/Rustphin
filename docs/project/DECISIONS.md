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

## D-018 — Network crates retained by the locked toolkit
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** R-108's validation text asked for "no network crates or sockets in the dependency audit". The locked toolkit (D-001, `gpui-kit` =0.7.3 / `gpui-pre` =0.3.9) pulls in `async-net` through `smol` and the `http`/`http-body` types through `gpui-pre-http-client`. Removing them means forking or patching the toolkit, which D-001 and D-010 rule out. Local sockets (Wayland, D-Bus) are required by design. W-077a found no HTTP client, remote service, updater or telemetry in Rustphin's source, and the Linux build leaves GPUI's `NullHttpClient` installed. W-077d traced five complete application lifetimes on all threads: 60 socket calls, all AF_UNIX.
- **Decision:** Accept those toolkit crates as a documented exception. R-108 means no application-owned network connections and no telemetry, checked by (1) source review, (2) traced runs showing only local sockets and (3) `deny.toml` bans that fail the gate if an HTTP client, TLS stack or telemetry crate is added, or if `async-net`, `http`, `http-body` or `gpui-pre-http-client` become reachable other than through the toolkit crates that bring them in now. Cargo.lock also lists the toolkit's `gpui-pre-reqwest` with hyper, h2 and rustls, for non-Linux targets of `gpui-kit-assets` only; they are not built for Linux and are banned so an upgrade cannot add them to the Linux build unnoticed (2026-11-08).
- **Rationale:** The criterion's intent is privacy, not the absence of unused types; the bans keep the exception from growing silently.
- **Evidence:** VALIDATION.md entries W-077a, W-077d and W-077h; `docs/project/PRIVACY.md`.
- **Consequences:** R-108's first criterion is accepted on that evidence. A toolkit upgrade must re-check the dependency graph and the wrapper lists in `deny.toml`. The traced runs do not cover external applications Rustphin launches, alternate bus transports or already mounted remote filesystems; those stay outside the application-owned contract described in PRIVACY.md. Remote folders are unaffected: under D-004 they are reached as mounted paths (kio-fuse, sshfs, SMB/NFS mounts), where a separate process or the kernel makes the connection and Rustphin reads ordinary files. Asking kio-fuse to mount an `sftp://` or `smb://` address is a local D-Bus call and is also compatible. Native protocol code inside Rustphin (L-008) would reopen this decision.
- **Alternatives:** Keep the literal criterion open into the daily-driver trial (rejected by the user); patch the toolkit to drop the crates (rejected: D-001/D-010).
- **Reopen criteria:** A toolkit upgrade adds a network client, TLS or telemetry crate; a Rustphin feature needs network access; the traces observe a non-local socket.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("Accept documented exception")
- **Supersedes / superseded by:** amends R-108 validation; supersedes nothing

## D-019 — "60 frames per second" means no dropped frames
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** R-101 asked for 60 fps "at the 95th percentile". Measured scrolling of 500,000 entries draws each frame in under 7 ms and the median frame interval equals the 16.67 ms refresh period, but the 95th-percentile interval is 16.72–16.98 ms (W-073e): presentation jitter around the refresh period in a virtual compositor, not missed frames.
- **Decision:** scrolling passes when no more than 5% of frame intervals during sustained scrolling exceed 1.5 × the display's refresh period (a dropped frame), in each view mode. Frame intervals, draw times and the longest gap are still reported.
- **Rationale:** a strict p95 ≤ 16.667 ms fails on timing jitter of a few tenths of a millisecond that no user sees; dropped frames are what makes scrolling look uneven.
- **Consequences:** R-101's wording changes accordingly; the frame log counts dropped frames.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("No dropped frames")
- **Supersedes / superseded by:** amends R-101

## D-020 — Interrupted batch renames are recovered at the next start
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** R-103 asks that killing the process during a batch rename leaves every file intact or complete. A killed batch keeps staged temporary names until the user chooses "Recover original names after interruption" (W-071).
- **Decision:** at startup Rustphin looks for unfinished batch-rename journals in the places it can find them (folders it shows) and restores the original names automatically through the existing journal-validated recovery, then reports what it did in a notice. Recovery failures are reported and keep the journal, as today.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("Recover at next start")
- **Supersedes / superseded by:** amends R-103's validation

## D-021 — Ctrl+D duplicates, as in Dolphin
- **Status:** locked
- **Date:** 2026-11-08
- **Decision:** Ctrl+D duplicates the selection in place as "name copy.ext" (Dolphin's naming, a free name when taken) through the operation engine, undoable. Add to Places moves to Ctrl+Shift+D and stays in the palette.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("Duplicate, like Dolphin")

## D-022 — R-101 measures a running window; application startup is outside it
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** everything after the window exists meets R-101: first entries of a 500,000-entry folder on screen within 59–78 ms in every view (W-073g), thumbnail work under 1 ms (W-073f), scrolling within D-019 (W-085). Application startup is not: the toolkit's window/GPU bootstrap blocks the UI thread for 93–104 ms, after about 0.4 s of off-thread font loading.
- **Decision:** R-101's "opening" means opening a folder in a running window, and its 16 ms criterion covers the listing, sorting, metadata and thumbnail code paths it names, not application startup. Startup timing stays recorded in VALIDATION.md and is not an acceptance target.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("Exclude startup")
- **Supersedes / superseded by:** clarifies R-101

## D-023 — No Orca screen-reader pass in the MVP
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** R-107 asked for a recorded Orca pass. Orca is not installed, and the accessibility tree itself was verified through AT-SPI in W-074a–i (names, roles, selection, focus, text), with gaps listed there.
- **Decision:** the Orca pass is dropped from R-107 and from S11's acceptance. R-107 keeps the AccessKit names-and-roles criterion, judged on the AT-SPI evidence.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("the Orca screen-reader can be forgotten and removed from the list of things todo")
- **Supersedes / superseded by:** amends R-107 and the S11 acceptance

## D-024 — Start the design pass before S11 acceptance closes
- **Status:** locked
- **Date:** 2026-11-08
- **Context:** W-079 was queued "after S11 acceptance". Seven S11 criteria remain open: five need the maintainer's real desktop or hardware (non-KDE drop target, USB mount, busy unmount, blur look, contrast over a white window) and two need the toolkit focus patch W-076e, which is not approved. The maintainer delivered the HTML mockups (docs/mockup/, 34 screens).
- **Decision:** W-079 starts now from the mockups. The open S11 criteria stay open on the list and are not ticked by the design pass. docs/DESIGN.md is replaced by the adopted target design (D-017 still makes it the reference), and its "Implementation status" says which parts the build follows. Two open points in the mockup notes are settled by following Dolphin and the brief: "Run this file?" makes Keep the Enter default (Dolphin focuses Cancel); the four-button name-conflict dialog stays 480 px and wraps its primary button.
- **Confirmed by / date:** The maintainer, 2026-11-08 ("You should be good to skip remaining S11 tasks and start W-079 now using the mockups I just created")
- **Supersedes / superseded by:** amends the W-079 start condition in ROADMAP.md

W-076e (patching the locked toolkit's focus registration) was not approved on 2026-11-08 ("Not yet"); it stays open until a planned toolkit upgrade.

## D-025 — Bundle static faces of the design's fonts
- **Status:** decided
- **Date:** 2026-11-09
- **Context:** the pinned GPUI ignores a variable font's `wght` axis, and the Inter, Space Grotesk and JetBrains Mono installed on Linux systems are usually variable, so the design's semibold headings rendered at the fonts' defaults (Regular, or Light for Space Grotesk). Forking GPUI is ruled out.
- **Decision:** the UI crate bundles static instances of the three fonts at the weights docs/DESIGN.md uses (`crates/rustphin-ui/assets/fonts/`) and registers them at start. They are under the SIL OFL 1.1, which permits bundling with software of any licence; the licence texts ship beside them. They are font data, not linked code, so the GPL-compatibility rule for dependencies is not engaged.
- **Confirmed by / date:** The maintainer, 2026-11-09 ("Yes, bundle them", for Inter and Space Grotesk); JetBrains Mono added by the agent for the same defect.
- **Supersedes / superseded by:** none
