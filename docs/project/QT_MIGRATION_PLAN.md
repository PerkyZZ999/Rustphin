# Plan: moving the UI from GPUI to Qt 6 and QML (W-114)

Requested by the maintainer on 2026-11-13. This plan proposes how, and on what
evidence, Rustphin's interface would move from GPUI (through GPUI-Kit) to
Qt 6 Quick with QML. Nothing here is decided: switching toolkits reverses
a locked decision (D-001) and needs the maintainer's approval and a superseding
record. The plan asks for that approval in two steps: first for a
measured spike, then, on its results, for the switch itself.

**Approved 2026-11-13:** decisions A–D as recommended (D-033). E follows the spike's numbers.

## Spike results (W-114a, 2026-11-13)

Two throwaway spikes in `spikes/qt-cxx` (cxx-qt 0.10) and `spikes/qt-bridge`
(QtBridge 0.4.1) show a details list over `rustphin-core`. Both were
measured in an isolated KWin session at 60 Hz, release builds, on the
500,000-entry benchmark folder (`rust-tc bench-listing`: core alone 48 ms
to first entries, re-sorts 56–91 ms). Rustphin on GPUI was measured in
the same session for reference.

| # | Criterion | cxx-qt | QtBridge | GPUI today | Verdict |
|---|---|---|---|---|---|
| 1 | First entries ≤ 200 ms (folder opened in a running window) | 7–13 ms (4 reopens); 148–203 ms when it is the window's first frame | 52–90 ms at start | 59–78 ms (W-073g) | pass |
| 1 | Re-sort shown ≤ 500 ms, details loaded | 75–115 ms | 156–248 ms | 92.7 ms (W-078) | pass |
| 2 | Scrolling, ≤ 5% dropped frames | 2 of 1,200 (0.17%) | 2 of 1,200 (0.17%) | 0–0.25% (W-085) | pass |
| 3 | Launch to first frame < 1 s | 0.31–0.44 s | 0.65 s | about 9.9 s to first entries in the same session (3.2 s reading 9,018 font files, 5.9 s warming font caches) | pass |
| 4 | Own memory (RssAnon) | 59 MB with a small folder; 59 MB after the window closed with the QML engine kept; 7.5 MB as a windowless process; 78 MB with 500,000 entries | 116–187 MB with 500,000 entries | 41 MB after closing (W-110e), 11.6 MB windowless, 73 MB with 500,000 entries | pass if the window and QML engine are released when the last window closes |
| 5 | Blur on the real desktop, opaque without it | translucency works; KWin reports no blur effect in an isolated session; **on the maintainer's desktop blur works** (the maintainer, 2026-11-13) | not reachable (C++ API) | — | pass |
| 6 | Drag to Dolphin; clipboard both ways | drag: Dolphin offered Copy Here and copied the file; copy → Dolphin's "Paste One File"; Dolphin's copy read back as the path | not reachable (C++ API) | Rustphin's own protocol code (D-010) | pass with Dolphin; a non-KDE application not tried |
| 7 | Rows named and selected over AT-SPI | list items by name, "selected" state | — | W-074 | pass |
| 8 | Licences and Rust 1.95 | deny.toml licences ok; builds with 1.95.0 | — | — | pass (inside `rust-tc doctor` from W-114c) |
| 9 | Which bridge | everything above, model rows made on demand, C++ helpers for KDE | every row stored as a Rust value (memory 1.5–2.4×, re-sorts 2×); no C++ reach; QML errors go to the journal and the program exits 0 silently; needs `qmake` on the PATH (Qt 5's here) | — | **cxx-qt** |

**Also found:** reloading the 500,000-entry folder grows memory by about
25 MB each time, in today's Rustphin (73 → 211 MB after six reloads) as in
the spike. The growth is in the core or the allocator, not in Qt: W-115.

**Not measured:** a true cold start with emptied disk caches (the numbers
above are warm); KWin's focus from a launcher token; icons and thumbnails
in the delegates (the spike draws a coloured square).

**Recommendation for E:** switch, with cxx-qt. Criteria 1–4 and 6–9 pass,
startup is more than ten times faster, and the file exchange with Dolphin
works with no protocol code of ours. Blur (5) was confirmed on the maintainer's desktop afterwards ("Blur is fine
with KWin").

## Short answer

Yes, the switch would improve Rustphin, but mainly through maturity and
KDE integration, not through raw speed. The speed targets are already met
on GPUI. What Qt would fix are the problems that keep coming back because
GPUI is a young toolkit that was never built for the KDE desktop.

## What would improve

Each point names the open item or workaround it would remove.

- **Starting from cold.** Plasma already keeps Qt in memory, and Qt finds
  fonts through fontconfig's cache. GPUI scans every font file itself;
  even read in parallel (W-109) a cold start takes about 3 seconds. Qt
  should remove most of that, and background mode (W-110) would matter
  less.
- **Blur.** KDE's own `KWindowEffects` blurs a Qt window through KWin's
  blur protocol. Today `rustphin-platform` borrows GPUI's Wayland
  connection with `unsafe` code to do the same (D-010).
- **Clipboard and drag-and-drop with other applications.** Qt exchanges
  files through `text/uri-list` the way Dolphin does. GPUI's clipboard
  cannot carry files, so Rustphin implements its own (D-010). Dragging
  out to applications other than KDE's is still unproved (R-010).
- **Window focus on launch.** Qt handles `xdg_activation_v1` tokens
  itself. W-110d had to add this by hand.
- **Text fields, focus and accessibility.** Qt's AT-SPI support and text
  input are mature. Reverse focus traversal is blocked by a defect in the
  pinned toolkit (W-076e). Single-letter dialog keys typed inside fields
  (W-111e) and the offset focus ring are other GPUI-specific problems.
- **Fonts.** Qt applies a variable font's weight axis. With GPUI the app
  has to bundle static faces (W-088).
- **A stable, documented toolkit.** Qt 6 has long-term releases and a
  documented API. GPUI-Kit is pre-1.0 and pinned exactly, and its API
  differs from Zed's own GPUI. Many entries in AGENTS.md "Known gotchas"
  only exist because of GPUI (re-entrant entity panics, opaque root
  background, theme tokens not reaching components, input sizing).
- **KDE's own pieces when wanted.** A file dialog that opens in a chosen
  folder (W-113), the Breeze icon theme through `QIcon`, KDE's colour and
  scaling settings, and later KIO for remote folders, instead of
  kio-fuse.

## What would not improve, and what costs more

- **Scrolling and sorting speed.** These are not a reason to switch. On
  GPUI, first entries of a 500,000-entry folder appear in 59–78 ms,
  scrolling drops 0–0.25% of frames, and a re-sort takes 92.7 ms
  (R-101, W-073g, W-085, W-078). The slow work is in `rustphin-core`,
  which stays. Qt Quick's virtualised views should match these numbers,
  but the spike has to show it.
- **Memory.** A QML engine and scene graph usually use more memory than
  GPUI. Idle Rustphin uses 41 MB today, against R-020's limit of 60 MB.
  Qt's shared libraries are already loaded for Plasma, but Rustphin's own
  memory has to be measured.
- **The work.** `rustphin-ui` is 30,700 lines in 49 files. All of it would
  be rewritten: the item views (details, icons, compact, with rubber-band
  selection, type-ahead and drag), places, tabs, split view, the palette,
  every dialog, Settings, and every mockup screen re-compared. The
  core (13,800 lines) and most of the platform layer (12,600 lines) stay.
- **The build.** It needs a C++ compiler, Qt 6 development packages and
  `qmake` on the path. `rust-tc doctor`, `cargo-deny` (LGPL-3.0 licences)
  and the minimum-version check all have to work with them.
- **Logic drifting into QML.** D-001 rejected Qt in October partly
  because the interface logic would sit outside Rust. The rule below
  keeps it in Rust.

## Which bridge: QtBridge or cxx-qt

| | QtBridge for Rust | cxx-qt (KDAB) |
|---|---|---|
| Version | 0.4.1, labelled "Technical Preview" (tag 0.4.0, 2026-10-27) | 0.10.0 on crates.io; used by KDE projects |
| Exposes Rust to QML | `#[qobject]`, `#[qslot]`, `#[qsignal]`, `qproperty!`; `QListModel` and `QTableModel` | `#[cxx_qt::bridge]` modules; any `QAbstractItemModel` subclass |
| Calls C++-only Qt and KDE APIs | No. Its README says to use cxx-qt for that | Yes, through `cxx` |
| From worker threads | `QThreadHandle` (since 0.4.0) | `CxxQtThread` queue |
| Requires | Qt 6.10 or later, Rust 1.88 | Qt 6, Rust stable |
| Licence | LGPL-3.0-only or Qt commercial | MIT or Apache-2.0 |

Both licences are compatible with GPL-3.0-or-later (D-003). Qt itself is
LGPL-3.0. QtBridge itself is built on cxx-qt.

**Recommendation:** cxx-qt as the foundation, with QtBridge tried in the
spike. Rustphin needs `KWindowEffects` for blur, a custom
`QAbstractItemModel` for 500,000 rows, and later KIO. QtBridge cannot
reach the first and third, and its API is still changing (0.4.0 removed
calling QML from Rust as unsound). If the spike shows QtBridge covering
all three, through its planned interoperability with cxx-qt, it can be
used for the simpler objects. Decision B below.

## Rules for the Qt interface

These carry the existing architecture rules over to Qt.

- **QML only declares.** QML holds layout, bindings and animation. State,
  selection, key handling decisions, menus' contents and text all come
  from Rust objects. Short JavaScript expressions in bindings are fine;
  functions with logic are not.
- **Models in Rust.** Each pane's entries are one Rust
  `QAbstractItemModel` over the core's snapshot. Rows are produced on
  demand, so QML only ever creates delegates for visible rows.
- **The UI thread contract stays (D-009).** Core workers send snapshots
  over channels. A cxx-qt thread queue hands them to the Qt thread, which
  swaps the model's snapshot and emits a reset or range change. No
  listing, sorting or file I/O happens on the Qt thread.
- **File names stay bytes.** QML receives display strings. Rust keeps
  the bytes and refers to entries by row or id, never by display name.
- **One look, from DESIGN.md (D-017 unchanged).** `theme.rs` keeps the
  tokens and the contrast tests and exports them to a QML `Theme`
  singleton, so DESIGN.md remains the authority. QML uses the Basic
  style, not Breeze or Kirigami controls, so the custom design is drawn
  as specified.
- **Layers (D-002 unchanged).** `rustphin-core` and `rustphin-platform`
  never depend on Qt. Qt and cxx-qt appear only in the UI crate and the
  binary. The cargo-deny ban that keeps `gpui-kit` out of the lower
  layers becomes a ban on `cxx-qt` there.

## Crates after the switch

| Crate | Change |
|---|---|
| `rustphin-core` | Unchanged |
| `rustphin-platform` | Keeps trash, thumbnails, places, MIME, devices, admin, instance, autostart, archive tools. Loses GPUI-only pieces once Qt covers them: blur through GPUI's surface, data-control clipboard, activation, font pre-read (each removed only after the Qt version is verified) |
| `rustphin-view` (new) | Toolkit-free interface logic moved out of `rustphin-ui` first: menu facts and entries, archive naming, selection movement, palette commands and matching, formatting, settings model, session save. Unit-tested without any toolkit |
| `rustphin-ui` | Rewritten: cxx-qt objects and models over `rustphin-view`, plus QML files in `crates/rustphin-ui/qml/` compiled ahead of time with `qmlcachegen` |
| `rustphin` | Wiring only |

Moving the logic to `rustphin-view` is worth doing whatever is decided. It
shrinks what has to be rewritten, and the GPUI interface keeps working
on top of it in the meantime.

## How the move is done

The Qt interface is built beside the GPUI one rather than in place of it.
Until the switch, `rustphin` stays the GPUI app and the maintainer keeps using
it. The Qt one is a second binary, `rustphin-qt`, over the same core.
When it reaches parity and passes the UI scenarios, the binaries swap and
GPUI is removed in one commit. No features are added to either interface
during the move, except fixes.

## Work items

- **W-114a — Spike (Discovery, in `spikes/qt/`, throwaway).** A Qt Quick
  window with a 500,000-row details list over a Rust model fed by the
  core's listing, built once with cxx-qt and once with QtBridge. It
  measures what the go/no-go criteria below ask, then reports to the maintainer.
  *Decision point: D-033.*
- **W-114b — `rustphin-view`.** Move the toolkit-free logic out of
  `rustphin-ui`, with its tests. The GPUI app is unchanged on screen.
  This is useful even if the switch is not approved.
- **W-114c — The Qt shell.** `rustphin-qt` binary, toolchain in
  `rust-tc doctor` (layers, deny, msrv), `Theme` singleton from
  `theme.rs`, a window with title bar and toolbar, blur through
  `KWindowEffects` with the opaque fallback, fonts, single instance and
  hand-over (W-110) and background mode.
- **W-114d — Item views.** Details, icons and compact over the Rust
  model: selection, rubber band, keyboard movement and type-ahead,
  inline rename, thumbnails, sorting and columns, hidden files. Checked
  against R-101 on 500,000 entries.
- **W-114e — Around the views.** Places panel and groups, devices, tabs,
  split view, location bar and breadcrumbs, status bar, the palette, the
  context menu (including Extract and Compress), filter.
- **W-114f — Operations and dialogs.** Progress and the notice bar,
  conflicts, errors, passwords, Undo, Properties, batch rename, Compress
  to…, Settings (every page), administrator mode, remote folders.
- **W-114g — Exchange with the desktop.** Clipboard and drag-and-drop
  with Dolphin and with a non-KDE application, opening with other
  applications, the default file manager over D-Bus, the portal file
  dialog starting in the folder shown (W-113).
- **W-114h — Parity check.** Every scenario in UI_SCENARIOS.md replayed
  on `rustphin-qt` in an isolated session, KEYBOARD.md re-checked, the
  mockup comparison redone, accessibility (W-074) and performance (R-101,
  R-020) measured again.
- **W-114i — The switch.** `rustphin-qt` becomes `rustphin`, GPUI and
  GPUI-Kit are removed with their workarounds and gotchas, and AGENTS.md,
  ARCHITECTURE.md, DECISIONS.md and deny.toml are updated. The maintainer uses
  it for a week first; the commit before the switch keeps the GPUI app
  recoverable.

## Go/no-go criteria for the spike (W-114a)

Measured on the maintainer's machine in an isolated session, with release
builds and the existing benchmark folder:

1. First rows of the 500,000-entry folder within 200 ms of opening, and
   a re-sort shown within 500 ms (R-101).
2. Sustained scrolling drops at most 5% of frames (D-019).
3. Cold start to first frame under 1 second (today about 3 seconds).
4. Idle memory of its own (RssAnon) after the window closes: at most
   60 MB (R-020).
5. Blur behind the window on the real desktop, and the opaque fallback
   without it.
6. A file dragged to Dolphin and to a non-KDE application, and copied
   and pasted both ways with Dolphin.
7. Rows announced by name and selection over AT-SPI.
8. `rust-tc doctor` passes with Qt in the build, including licences and
   the 1.95 minimum.
9. Which bridge meets 1–8 with less code, and whether QtBridge can reach
   `KWindowEffects`.

If 1, 2, 4 or 8 fails and cannot be fixed within the spike, the
recommendation becomes "stay on GPUI" and the plan stops after W-114b.

## Decisions for the maintainer

- **A. Run the spike?** *Recommended: yes.* Approving A does not approve
  the switch.
- **B. Bridge.** *Recommended: cxx-qt, with QtBridge for simple objects
  if the spike shows it can work alongside.* Alternative: QtBridge only,
  which depends on a technical preview and cannot reach C++-only KDE
  APIs.
- **C. Look.** *Recommended: keep DESIGN.md's own look (D-017), drawn
  with plain Qt Quick.* Alternative: Kirigami and Breeze controls, which
  look native but would mean redesigning against DESIGN.md.
- **D. File operations.** *Recommended: keep the core's engine (D-011).*
  KIO would be available, but it would replace tested behaviour
  (conflicts, Undo, fault-injection suite). It can be reconsidered later
  for remote folders only.
- **E. After the spike: switch (D-033, superseding D-001).** Decided on
  the spike's numbers.

## Size and risk

This is the largest change since the MVP: about as much interface code
as was written in slices S1–S11, though with the core, the decisions and
the designs already done. It is gated: the maintainer sees measurements after
W-114a, and the GPUI app stays usable until W-114i. The main risks are
QtBridge's preview status (avoided by recommendation B), QML delegates
for the icons view on very large folders (measured in W-114d), and the
memory limit (measured in W-114a).
