# Acceptance reconciliation (W-078)

Every open acceptance criterion in REQUIREMENTS.md, with the evidence in
VALIDATION.md, what is missing, and who can close it. First mapped on
2026-11-08 from the evidence log; brought up to date on 2026-11-14 after the
switch to the Qt interface (W-114i) and the work that followed (W-115–W-144).
Nothing here is ticked without a recorded check.

**Closable by the agent** means an isolated KWin session with generated data
can show it. **Needs the maintainer** means a decision, the real desktop or real
hardware.

## Closed by the agent

On 2026-11-08: W-085 closed R-101's 60 fps criterion under D-019 (details
0.25%, icons 0.08%, compact 0% dropped), and D-022 (startup outside R-101) let
W-073f/g close its first-entries and 16 ms criteria. Replays K14–K17 and J4,
the token audit and the floating-surface test closed the rest: the Dolphin
listing comparison, back/forward/up with per-tab history, content changes with
selection and scroll kept, per-tab view modes, case-insensitive sort with the
folders-first toggle, zoom in all modes, the Page keys, design tokens, opaque
floating surfaces and secondary controls (with the filter bar, W-082). R-105's
opaque-mode contrast was accepted by the maintainer on token-level tests,
without a per-screen audit. The maintainer dropped the Orca pass (D-023), and
R-107's names-and-roles criterion was ticked on W-074a–i. K15 found two
symlink display differences from Dolphin, fixed in W-081.

On 2026-11-14: R-010's drag out to a non-KDE application is ticked on W-114g
(a file dragged from Rustphin into Nautilus arrived, original kept, in an
isolated session). R-101, R-107 names and roles and R-020's memory criterion
were re-measured or re-checked on the Qt interface (W-114c3, W-114d, W-114h);
REQUIREMENTS.md says per criterion what was and was not repeated on Qt.

## Closable by the agent: the Qt interface

| Requirement · criterion | State | What is needed |
|---|---|---|
| R-004 · every action has a keyboard path; Dolphin defaults | One command table, tested against KEYBOARD.md (W-123); keys replayed on Qt (W-114h, W-123) | A keyboard-only pass of every surface on Qt. The GPUI blocker W-076e (a toolkit focus patch the maintainer did not approve) went with GPUI in W-114i |
| R-107 · keyboard-complete with visible focus everywhere | A focus ring on Properties' permission boxes (W-116b); not checked surface by surface | The same keyboard-only pass, Shift+Tab included |
| R-019 · Settings reachable by keyboard | Pages and opening keys seen on Qt | Tab through each page's controls |
| R-019 · every change applies in every window | Applies at once, survives restarts, damaged files kept (W-140) | A second window seen following a change |

## Code and acceptance disagree

| Requirement · criterion | What the code does | Needed |
|---|---|---|
| R-019 · each page's settings do what SETTINGS_PLAN.md describes | Appearance › Look and Title bar are saved but the Qt interface ignores them (only `RUSTPHIN_OPAQUE` turns the glass off; the window is always frameless) | Wire them into the Qt interface, or a decision to drop them (W-117) |
| R-109 · a missing optional service degrades with a visible notice | UDisks2 and data-control say so; without blur the Qt interface is opaque and says so on a status line, and follows blur appearing or going (W-146c; the decision is unit-tested, the line was seen only when forced) | A look on a desktop with KWin\'s Blur effect switched off (UI_SCENARIOS AP6) |

## Needs the maintainer: real desktop or hardware

| Requirement · criterion | State | What is needed |
|---|---|---|
| R-013 · mount, unmount, safely remove; hot-plug updates | Mock service and generated private service only | A real USB drive on the real desktop |
| R-013 · busy unmount shows the reason | Mock service only | A real drive kept busy, then unmount |
| R-105 · the DESIGN.md theme with compositor blur | Blur confirmed with the Qt spike (W-114b2); tokens applied | A look at the current build on the real desktop |
| R-105 · AA over blur with a white window behind | Token tests against white | A look on the real desktop over a white window |
| R-020 · a new window takes the focus with an activation token | Token handed to Qt as `XDG_ACTIVATION_TOKEN` (W-114c3, W-120); delivery tested on a private bus | A launch from the panel and Chrome's "Show in folder" on the real desktop |

Also for the maintainer, outside any criterion: administrator mode with the
real helper (W-098e, the checklist at the end of ADMIN_MODE.md).

## Not part of any criterion yet

Remote folders (W-080–W-080c) have no requirement of their own; their
evidence is in VALIDATION.md and UI_SCENARIOS W1–W11. They have not been
tried against a real sftp or smb server, nor replayed on the Qt interface
(no kio-fuse mount in the test session).
