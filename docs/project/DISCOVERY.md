# Discovery

Phase 2 output, 2026-10-12. Each claim is marked **observed** (run or read on this machine today; evidence in VALIDATION.md), **sourced** (read in upstream source or docs), or **assumed**.

## Feasibility stance

**Go, with conditions.**

1. Blur and the file clipboard are owned by Rustphin's platform layer, not by GPUI, and each has a fallback when its Wayland protocol is missing.
2. The visual design does not depend on blurring in-window content behind panels.
3. Directory listing, stat and sort run off the UI thread from the first slice, with the 500k-entry target measured in CI or a repeatable benchmark.
4. The toolkit version is pinned exactly and upgraded deliberately.

## Constraints

| Area | Constraint | Basis |
|---|---|---|
| Platform | Linux, Wayland only; KDE Plasma first (target machine: Plasma 6.7.5) | user decision; observed |
| Language | Rust, MSRV 1.95, edition 2024; installed 1.98.1 | user decision; observed |
| Toolkit | `gpui-kit` 0.7.3 on `gpui-pre` 0.3.9 (states Rust 1.92+). Zed HEAD GPUI pins 1.98.1, so tracking HEAD would break the MSRV | sourced; observed |
| Licence | GPL-3.0-or-later. Dependencies seen so far are Apache-2.0 or MIT/Apache (gpui-kit, wl-clipboard-rs, rayon, wayland-rs), all GPLv3-compatible | sourced; a full licence audit belongs to Release |
| Dolphin source | Reference for behaviour only; reading it makes Rustphin a derived work, which the licence choice already covers | assumed (not legal advice) |
| Team and time | One author plus an AI agent; no deadline. Scope must stay sliceable | assumed |
| Budget | None; no paid services | assumed |
| System integrations present | udisks2 2.11.2 (active), kio-fuse 5.1.1, Baloo 6.30, Breeze icon themes, `ext_data_control_manager_v1`, `ext_background_effect_manager_v1`, `wl_data_device_manager` v3 | observed |
| System integrations absent | `org_kde_kwin_blur_manager` is not advertised by this KWin | observed |

## Proportionate checks

- **Security.** A file manager runs with the user's full file permissions and handles untrusted names and content. Constraints: never execute a file on activation without the same confirmation Dolphin gives for executables and `.desktop` files; treat names as bytes that may not be UTF-8 and may contain control characters; thumbnailers parse untrusted data, so they run out of process or through memory-safe decoders; no elevation in the MVP (no "open as administrator").
- **Data safety.** File operations are the highest-consequence feature. Constraints: delete goes to trash by default; permanent delete, overwrite and cross-device move need explicit confirmation; copy/move never leaves a silently truncated target; undo covers rename, move, copy and trash.
- **Privacy.** No telemetry, no network access by the application itself. Recent locations and palette history stay in local config and can be cleared.
- **Accessibility.** GPUI integrates AccessKit (sourced), but screen-reader behaviour was not tested. Full keyboard operation is a requirement; the command palette supports it. Translucent surfaces need a contrast check and an opaque mode.
- **Data lifecycle.** Rustphin owns only its config, place-group data and palette history under XDG directories. It shares, and must not corrupt, `user-places.xbel` and the freedesktop thumbnail cache.
- **Operations.** A desktop application, no services. Needs structured logs, a crash-safe config write (write-then-rename), and distribution as a source build first (packaging later).

## Place groups and Dolphin's shared places file

Observed in this machine's `~/.local/share/user-places.xbel` (structure only): a flat list of `<bookmark>` elements, each with a KDE `<ID>` and optional `isSystemItem`, `IsHidden`, `OnlyInApp`; devices appear as `<separator>` elements carrying a UDisks `UDI` and `uuid`; there are no `<folder>` elements. Dolphin rewrites this file during normal use (observed: modified during test runs).

Conclusion: the shared file has no place for user-defined groups, and adding unknown structure risks other KDE apps dropping or mangling it. Rustphin therefore reads and writes ordinary places in the shared file and stores group membership and order in its own file, keyed by the bookmark `<ID>`. A place created inside a Rustphin group still appears, ungrouped, in Dolphin and file dialogs. Places whose ID vanishes from the shared file drop out of their group. This confirms D-008 as an assumption to lock in phase 4; whether other apps preserve unknown elements was not tested.

## Directory listing performance

Observed with `spikes/listing-bench` on a 500,000-file directory (XFS, warm cache, 16 threads, release build):

| Step | 1 thread | Parallel (rayon) |
|---|---|---|
| List names and types (`read_dir`) | 63 ms | — |
| Stat every entry | 579 ms | 53–60 ms |
| Natural sort by name | 181 ms | 26–29 ms |

Reading: names for 500k entries arrive in well under 100 ms; the costly part is per-entry metadata, which parallelises about tenfold; a full natural sort is about 30 ms in parallel. The approach this supports: stream names first and show them immediately, fetch metadata lazily (visible rows first, the rest in the background, all of it only when the sort key needs it), sort on a worker pool, and hand the view immutable snapshots. Not measured: cold cache, Btrfs (the home filesystem here), network or FUSE mounts, and UI frame times while results stream in.

## Top risks

| # | Risk | Likelihood | Impact | Response |
|---|---|---|---|---|
| 1 | A file operation loses data (overwrite, interrupted move, bad undo) | Medium | Severe | Mitigate: operation engine in the headless core with property and fault-injection tests against temp directories; trash by default; confirmations; no production use on real data until the engine's gate passes |
| 2 | Toolkit churn: GPUI is pre-1.0 and GPUI-Kit follows its own snapshot | High | Medium | Mitigate: exact version pins; UI-agnostic core (D-002); upgrades as planned work items with the spike re-run as a regression check |
| 3 | Our Wayland code (blur, clipboard, borrowed connection) breaks on a compositor or toolkit update | Medium | Medium | Mitigate: isolate in `rustphin-platform` behind traits; detect protocols at runtime; fall back to an opaque theme and to GPUI's text clipboard; offer the blur change upstream |
| 4 | The frosted-glass design is limited: no in-window backdrop blur, contrast on translucent surfaces | High | Medium | Mitigate: design floating panels as tinted opaque surfaces; define an opaque mode; contrast-check tokens |
| 5 | Scope: Dolphin parity is large for one author | High | Medium | Mitigate: MVP limited to the agreed core workflow; vertical slices; explicit "later" list |
| 6 | Missing network locations and KIO protocols make it unusable as the only file manager | Medium | Medium | Accept for MVP: kio-fuse paths; Dolphin stays installed |
| 7 | Rendering defects in GPUI on Linux (seen: 16 px icons missing while unfocused) and unmeasured scroll frame times | Medium | Low–Medium | Mitigate: reproduce and measure in the first UI slice; report upstream |
| 8 | Accessibility falls short of Dolphin | Medium | Medium | Accept for MVP with keyboard-complete operation required; test with Orca before release |

## Show-stoppers

None known. The three candidates raised before the spike are closed:

- Dragging files out to other apps: works on GPUI-Kit 0.7.3 (observed).
- File clipboard with other apps: works through the data-control protocol outside GPUI (observed).
- Blur: works through `ext-background-effect-v1` outside GPUI (observed on the real desktop with the Zed-HEAD build; request accepted with the GPUI-Kit build).

## Open items carried forward

- Visible blur with the GPUI-Kit build on the real desktop; blur on resize.
- Percent-encoding and non-UTF-8 names in clipboard and drag payloads.
- Frame times for the 500k-row list; cold-cache and Btrfs listing numbers.
- Whether to depend on `wl-clipboard-rs` or implement data-control directly.
- GPUI-Kit components (sidebar, virtual list, command, menu) untried.

## 2026-11-05 — W-076e reverse input traversal and native focus

A current S11 input blocker supersedes the historical no-show-stoppers statement
above for complete keyboard/accessibility acceptance. Named input semantics and
the actual editor track the same FocusHandle. In pinned gpui-pre 0.3.9,
elements/div.rs registers each occurrence; tab_stop.rs keeps both ordered nodes
but maps the shared ID to its final occurrence. Reverse traversal therefore
returns to the preceding occurrence of the same field. This was observed in
batch Replace: Tab reaches Replace with, Shift+Tab leaves it there, and subsequent
typing changes that field instead of Find.

Removing only the semantic wrapper's track_focus temporarily restores reverse
traversal, but a native AT-SPI assertion then fails because Find is not focused.
The original code has been restored. Changing only a cloned handle's public
tab_stop flag has no effect because track_focus clones again; FocusHandle::clone
rebuilds from shared metadata. No per-occurrence tab-registration override was
found in the pinned public API. This is a bounded API/source assessment, not a
proof that every possible application architecture is impossible.

A concrete maintenance candidate is to deduplicate focus IDs in GPUI's
TabStopMap::insert, retaining one ordered position per FocusHandle. A minimal
candidate is to return before inserting an ID already present in by_id. It is a
proposal, not an implemented or verified fix: first-versus-last placement and
metadata updates need review for shared handles, tab groups, disabled controls
and partial frame replay. Required proof is a toolkit regression for repeated
handle registration in both directions, then Rustphin's native focus/Text/range
checks, complete modal traversal, the full gate and large-directory timing.

D-001 locks the exact toolkit and D-010 forbids a GPUI fork. A patched dependency
or toolkit upgrade needs the maintainer's approval and the change-impact loop before
implementation; no registry source, dependency pin, patch section or decision
has been changed. The current acceptance item remains open. Unrelated S11 layout,
fallback and privacy/build checks can continue while that decision is pending.
