# Acceptance reconciliation (W-078)

Every open acceptance criterion in REQUIREMENTS.md, with the evidence in
VALIDATION.md, what is missing, and who can close it. Mapped on 2026-11-08
from the evidence log; the ten criteria ticked that day were re-read against
their cited entries first. Nothing here is ticked without a recorded check.

**Closable by the agent** means an isolated KWin session with generated data
can show it. **Needs the maintainer** means a decision, the real desktop or real
hardware.

## Closable by the agent (replays still to run)

| Requirement · criterion | Evidence so far | Missing |
|---|---|---|
| R-001 · lists every entry Dolphin lists; hidden off by default, toggle | S1 checks: Ctrl+H shows `.config` and `.hidden-file` ("11 items, 2 hidden" → "13 items") | A side-by-side listing of one generated folder in Rustphin and in Dolphin (isolated home) |
| R-001 · back, forward, up with mouse and keyboard; history per tab | K9 Backspace/Alt+Up; J5 Alt+Left | Toolbar clicks for back/forward/up; Alt+Right; separate history in two tabs |
| R-002 · outside changes appear within 1 s | W-025: created, deleted and renamed files shown within a second (U1/U2) | A file whose content changes (size/date update) |
| R-002 · selection and scroll survive an update | W-025: selection kept, no scroll on a 17-entry fixture | Scroll kept in a long list; a renamed selected file loses its selection (Dolphin keeps it; decide whether to match) |
| R-003 · three view modes selectable per tab | W-019, V4 | Two tabs (or panes) in different modes at once; settings are global with per-pane overrides for the session |
| R-003 · natural, case-insensitive sort, folders first on by default | S1: folders first, `file2` before `file10` | Case-insensitive order on screen; turning folders-first off |
| R-003 · zoom in all three modes | W-019: compact zoom in; V3 icons | Zoom in details; zoom out and reset |
| R-004 · arrows, Home/End, Page Up/Down; type-ahead | W-020 type-ahead; L1 End; K10 type-to-jump | Page Up/Down and Shift+Page keys in the file list |
| R-105 · menus, palette, dialogs are opaque | W-057 palette, B1 dialog | Context menus: K12/K13 screenshots show solid menus; record them against this criterion |
| R-105 · colours, spacing, radii, sizes from one token set | Design statement in W-018 | A code audit for literal colours/sizes outside `theme.rs` |
| R-106 · secondary controls within two steps | W-059: every action in the open Actions set | Run J4; operation details reachability |

## Needs the maintainer: decisions

| Requirement · criterion | State | Decision needed |
|---|---|---|
| R-101 · 60 fps at the 95th percentile | p50 = 16.66 ms (the refresh period), p95 16.72–16.98 ms, draw under 7 ms (W-073e) | Is "60 fps" "no dropped frames" (then measure dropped frames) or a strict p95 interval ≤ 16.667 ms (then presentation jitter in a virtual compositor fails it)? A real-display measurement also needs the desktop |
| R-101 · no UI-thread task over 16 ms | Measured paths well under (model callback 0.8 ms) | Window/GPU bootstrap produced a 104 ms foreground poll in one diagnostic run (W-073b); thumbnail paths were not measured. Accept startup as outside "listing, sorting, metadata, thumbnails" and measure thumbnails, or treat startup as a defect to fix |
| R-101 · first entries within 200 ms, input during loading | 185.6 ms warm-cache details navigation; input observed while loading | Whether one measured mode/warm cache is enough, or every mode and a cold start must be measured |
| R-103 · kill during batch rename leaves files intact or complete | Copy/move pass; a killed batch rename keeps staged names until recovery is chosen (W-071) | Reword to "recoverable with Recover original names", or require automatic recovery at the next start |
| R-004 · every action has a keyboard path; Dolphin defaults | KEYBOARD.md; K8–K13 | Ctrl+D: Dolphin duplicates, Rustphin adds to Places. Implement Duplicate on Ctrl+D (new engine operation) and move Add to Places? Also W-076e (reverse Tab between text fields needs the approved toolkit change) |
| R-107 · keyboard-complete with visible focus everywhere | Focus surfaces observed (W-076a); reverse traversal blocked | W-076e toolkit adoption (D-001/D-010 approval) |

## Needs the maintainer: real desktop or hardware

| Requirement · criterion | State | What is needed |
|---|---|---|
| R-010 · drag out to a non-KDE application | Dolphin target works (G5) | Drag a file from Rustphin into, e.g., Firefox or a GTK app on the real desktop |
| R-013 · mount, unmount, safely remove; hot-plug updates | Mock service and generated private service only | A real USB drive on the real desktop |
| R-013 · busy unmount shows the reason | Mock service only | A real drive kept busy, then unmount |
| R-105 · the DESIGN.md theme with compositor blur | User saw blur in W-012; tokens applied | A look at the current build on the real desktop |
| R-105 · opaque mode: all text meets AA | Token contrast tests; isolated opaque replays | Accept token-level tests, or ask for a per-screen audit |
| R-105 · AA over blur with a white window behind | Token tests against white | A look on the real desktop over a white window |
| R-107 · Orca pass with gaps listed | AT-SPI names/roles checked (W-074a–i) | Orca installed (`orca` is not on PATH) and a session on the real desktop |

## Not part of any criterion yet

Remote folders (W-080–W-080c) have no requirement of their own; their
evidence is in VALIDATION.md and UI_SCENARIOS W1–W11. They have not been
tried against a real sftp or smb server.
