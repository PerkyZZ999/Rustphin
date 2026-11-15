# Handoff — 2026-11-14

For the next agent picking up Rustphin. Read `AGENTS.md` first: its rules
are binding. The documents in `docs/` are the source of truth, and
`docs/project/STATE.md`, `ROADMAP.md` and `WORK_ITEMS.md` say where the
project stands. This file covers what those documents cannot yet: the work
that was in flight, and what the maintainer asked for next.

## Where things stand

- **Version:** 0.1.0. Qt 6 and QML interface through cxx-qt; Rust 1.100.
- **Repository:** pushed to `https://github.com/PerkyZZ999/Rustphin.git`,
  branch `main`, public (D-037). The last pushed commit is `e8c0e92`
  (W-148b). Push only when the maintainer asks, and only commits that passed
  `rust-tc doctor`. No hosted CI.
- **Gate:** `rust-tc doctor` passes with 988 tests and 3 skipped (W-117).
- **Installed build:** `~/.local/bin/rustphin` is the build from before W-147.
  After installing a new one, the maintainer runs `rustphin --quit` once.
- **Public repository:** never write the maintainer's personal name, home
  paths or drive labels into tracked files or commit messages. Write "the
  maintainer", `~`, or sibling-relative paths. The whole history was
  rewritten once to remove them.

## Done but not pushed: W-149, W-146, W-117 (Slice S12)

Committed locally after the last push (`e8c0e92`, W-148b), each with `rust-tc
doctor` passing; VALIDATION.md has the evidence of each:

- W-149 transfer details: time remaining, Details, the transfer dialog.
- W-146a–e: tooltips and scrollbars from the tokens; Settings that did
  nothing (toolbar words, narrow Settings, live text size); the blur status
  line, Look and Title bar; administrator mode per window; the volume name,
  read-only Extract, a panicking operation reported.
- W-117: the Qt leftovers.

Found on the way and worth knowing: Rustphin always started opaque (it asked
KWin about blur too early); isolated KWin sessions do offer blur.

## Waiting on the maintainer

- KWin's window menu on a right-click of the title bar (the last W-146 item):
  Qt Wayland's private API, or KWin's shortcut over D-Bus (STATE.md).
- The v0.2.0 roadmap discussion.

## Practical notes

- **Checks:** `rust-tc doctor` before every commit, and `rust-tc qml-format`
  and `rust-tc qml-lint` for QML. The lint plugin is found in
  `../QtQmlBestPractices` (its own `build/`). There is no system `cmake`.
- **UI verification:** use kwin-mcp in an isolated session
  (`isolate_home: true`). Kill test instances by PID only: never run
  `pkill -x rustphin`, which also ends the maintainer's own Rustphin.
- **Screenshots.** Frames captured with `screenshot_after_ms` can come back
  with red and blue swapped; the plain `screenshot` tool's PNGs are right.
  Judge colours by measuring pixels against the tokens (`magick file.png
  -crop 1x1+X+Y -format "%[pixel:p{0,0}]" info:`).
- **Transfers on screen** need a copy between two filesystems (UI_SCENARIOS
  "Transfer details"): inside one XFS or Btrfs filesystem a copy shares
  blocks and ends at once.
- **Stopping a test instance:** `kill <pid>`. `pkill -f target/debug/rustphin`
  from a shell command also matches that shell's own command line.
- **Fuzzing** cannot run here: the installed nightly is 1.97, below 1.100.
- **Original history:** a backup of the history before the rewrite is at
  `../Rustphin-history-backup.git`. It is never to be pushed.
