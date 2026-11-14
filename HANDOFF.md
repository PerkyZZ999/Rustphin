# Handoff — 2026-11-14

For the next agent picking up Rustphin. Read `AGENTS.md` first: its rules
are binding. The documents in `docs/` are the source of truth, and
`docs/project/STATE.md`, `ROADMAP.md` and `WORK_ITEMS.md` say where the
project stands. This file covers what those documents cannot yet: the work
that was in flight, and what the maintainer asked for next.

## Where things stand

- **Version:** 0.1.0. Qt 6 and QML interface through cxx-qt; Rust 1.100.
- **Repository:** pushed to `https://github.com/PerkyZZ999/Rustphin.git`,
  branch `main`, public (D-037). The last pushed commit is `ca6c6e3` (W-147,
  the charcoal colour scheme). W-148 (`ed9a765`) and this file are committed
  locally, not pushed. Push only when the maintainer asks, and only
  commits that passed `rust-tc doctor`. No hosted CI.
- **Gate:** `rust-tc doctor` passes at `ed9a765`, with 950 tests and 3 skipped.
- **Installed build:** `~/.local/bin/rustphin` is the build from before W-147.
  After installing a new one, the maintainer runs `rustphin --quit` once.
- **Public repository:** never write the maintainer's personal name, home
  paths or drive labels into tracked files or commit messages. Write "the
  maintainer", `~`, or sibling-relative paths. The whole history was
  rewritten once to remove them.

## Done but not pushed: W-148, drive cards and a drive context menu

Committed locally as `ed9a765`, after the last push (`ca6c6e3`). `rust-tc
doctor` passed at that commit with 950 tests and 3 skipped. The on-screen
checks ran against the stand-in UDisks2 service
(`crates/rustphin-platform/examples/fake_udisks2.rs`, UI_SCENARIOS DV1–DV6),
never against real drives.

**Waiting on the maintainer:**
- **Confirm D-039.** D-038 said drive usage bars are neutral grey. The
  maintainer's request makes them the accent blue, so the agent recorded
  D-039 citing that request as the approval.
- **The E key.** On a fixed drive that can power off, E now powers it off
  (Safely remove) instead of only unmounting it.
- **Push** `ed9a765` once confirmed.

Not done: `docs/screenshots/rustphin.png` does not show the drive cards yet,
and real drives were not tried (R-013).

## Next: W-149, transfer details (agreed, not started)

The maintainer chose to keep the activity band above the status line (the
"Option 1" mockup), not to move it into the status bar.

1. **The band** (`NoticeBar.qml`, as in mockup 09) stays compact and gains:
   - Time remaining in its figures, for example "4 of 12 · 51.2 MiB of
     150.4 MiB · 48.6 MiB/s · about 2 s left". Base it on a smoothed rate,
     and show it only once the rate is steady and the sizes are known.
   - A **Details** button before Stop, reachable by keyboard and with a
     tooltip.
2. **A transfer dialog**, styled like the other floating surfaces, with one
   card per running operation. Each card shows:
   - its sentence and its own progress bar;
   - items and bytes done;
   - speed and time remaining;
   - **the file being worked on now** (`Progress::current`, elided in the
     middle);
   - a **small speed graph** of about the last 60 s, drawn in QML (Canvas or
     Shape) with no chart library;
   - its own Stop button.

   Decide, following KDE, what happens when the last operation ends.
3. **Where the logic lives.** The rate smoothing, the time-remaining wording
   and the graph samples go in rustphin-view, beside `OperationBoard`. Test
   them with injected time and never read the wall clock. The hub only
   forwards progress, and `Operations` hands the list to QML.
4. **Docs first.** Update WORK_ITEMS.md and ROADMAP.md (S12), DESIGN.md,
   INFORMATION_ARCHITECTURE.md, and add a UI scenario with two concurrent
   transfers, stopping one from the dialog.
5. **Testing note.** `/tmp` is fast. Use many files or several GiB so a
   transfer lasts long enough to see.

## Then

- **Slice S12:**
  - W-146: each item is a defect against the documents. Examples are the
    Appearance › Look and Title bar settings, which do nothing; the blur
    notice; administrator mode per window as D-028 says; and a notice when
    UDisks2 is missing.
  - W-117: the Qt leftovers.
- **Also open:** W-115 (memory on reloading a huge folder), the keyboard
  pass, and the maintainer's real-desktop checks (see STATE.md).
- **Before release work:** after W-149, push, reinstall
  (`rust-tc build`, `install -m 755 target/release/rustphin
  ~/.local/bin/rustphin`, tell the maintainer to run `rustphin --quit`), then
  **plan the v0.2.0 roadmap with the maintainer**. They asked for that
  discussion. Come with:
  - what is still open;
  - the "Later" list in REQUIREMENTS.md: search, information panel, quick
    preview, service menus, packaging, translations;
  - a suggested grouping for 0.2.0, for them to reshape.

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
- **Fuzzing** cannot run here: the installed nightly is 1.97, below 1.100.
- **Original history:** a backup of the history before the rewrite is at
  `../Rustphin-history-backup.git`. It is never to be pushed.
