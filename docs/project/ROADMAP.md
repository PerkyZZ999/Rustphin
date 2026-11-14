# Roadmap (vertical slices)

Each slice ends with something the author can run and use, and with its acceptance demonstrated. Order reflects dependencies and risk: scale and data safety are proven early.

**Toolkit note.** Slices S1–S10 were built with a GPUI interface. On 2026-11-14 the interface moved to Qt 6 and QML through cxx-qt (W-114, D-033/D-034): `crates/rustphin` is now the Qt binary, the toolkit-free interface logic lives in `crates/rustphin-view`, and the core and platform crates were kept. Each slice's behaviour was replayed on the Qt interface (W-114h, W-116b); what was not repeated on Qt is said per criterion in REQUIREMENTS.md and in ACCEPTANCE.md.

## Slice S1 — Open and browse a folder
- **Outcome:** Launch Rustphin, see a folder's entries in the frosted-glass window, and move around with mouse and keyboard, including in a 500,000-entry folder.
- **Acceptance:**
  - Workspace builds with one command on the minimum Rust (1.95 then; 1.100 since D-035) and stable; format, lint and test checks pass.
  - Window uses the dark theme tokens with blur, and the opaque mode when blur is unavailable.
  - A directory's entries are listed by name, natural order, folders first; hidden files toggle.
  - Enter a folder, go up, back, forward; type a path; unreadable or invalid paths show an inline error.
  - Headless benchmark on 500,000 entries: first batch within 200 ms, full sorted snapshot within 500 ms (warm cache); the UI accepts input throughout loading.
  - Names with invalid UTF-8 and control characters display safely.
- **Requirements:** R-001 (navigation part), R-101 (first entries, input responsiveness), R-104 (names), R-105 (window and fallback), R-109
- **Validation:** test, bench, ui-test, manual visual review
- **Status:** done (2026-10-12; evidence in VALIDATION.md)

## Slice S2 — Views, sorting, selection
- **Outcome:** Icons, compact and details views with sortable columns, zoom, file-type icons, and full selection and keyboard navigation.
- **Acceptance:** R-003 and R-004 criteria; lazy metadata keeps R-101 sort and scroll targets; memory target of R-102; icons from the system theme by MIME type; view settings persist.
- **Requirements:** R-003, R-004, R-014 (icons part), R-101 (sort, scroll), R-102
- **Validation:** test, bench, ui-test with frame timing
- **Status:** done (2026-10-12; evidence in VALIDATION.md)

## Slice S3 — Live updates, tabs, split view
- **Outcome:** The view follows outside changes; work in several tabs and two panes; session restored on start.
- **Acceptance:** R-002, R-005, R-006 criteria (pane-to-pane copy arrives with S4/S5).
- **Requirements:** R-002, R-005, R-006
- **Validation:** test, ui-test
- **Status:** done 2026-10-12 (see VALIDATION.md). Copy and move between panes is left to S4/S5.

## Slice S4 — File operations, undo, trash
- **Outcome:** Create, rename, copy, move, trash and delete with progress, conflict handling and undo.
- **Acceptance:** The operation engine passes its fault-injection and kill-test suite **before** any operation UI is enabled; then R-007, R-008, R-009 criteria; trash interoperates with Dolphin.
- **Requirements:** R-007, R-008, R-009, R-103
- **Validation:** test (property, fault injection, kill tests), manual (Dolphin interop)
- **Status:** done 2026-10-12 (see VALIDATION.md). Copy and paste are internal to Rustphin until S5.

## Slice S5 — Clipboard and drag-and-drop
- **Outcome:** Copy, cut, paste and drag files between Rustphin and other applications.
- **Acceptance:** R-010 and R-011 criteria, including names with spaces and non-ASCII characters and the no-data-control fallback.
- **Requirements:** R-010, R-011
- **Validation:** ui-test (with Dolphin), test (URI encoding), manual (non-KDE app)
- **Status:** done 2026-10-12 (see VALIDATION.md). Open: a drop target that is not a KDE application, and a compositor without data-control, could not be tried.

## Slice S6 — Places, place groups, devices
- **Outcome:** Places panel shared with KDE, user-defined collapsible groups, and mountable devices.
- **Acceptance:** R-012 and R-013 criteria; the shared places file stays valid for Dolphin after every edit.
- **Requirements:** R-012, R-013
- **Validation:** test (fixtures, mocked UDisks2), ui-test, manual (Dolphin, USB drive)
- **Status:** done 2026-10-12 (see VALIDATION.md). Open: mounting, unmounting and safe removal on a real USB drive.

## Slice S7 — Thumbnails
- **Outcome:** Image thumbnails that load as you scroll and are shared with Dolphin's cache.
- **Acceptance:** R-014 thumbnail criteria; malformed and oversized images are harmless.
- **Requirements:** R-014 (thumbnails), R-104 (images)
- **Validation:** test, manual
- **Status:** done 2026-10-12 (see VALIDATION.md)

## Slice S8 — Command palette and progressive disclosure
- **Outcome:** One shortcut reaches every action and any recent folder; the default window is pared back to the essentials.
- **Acceptance:** R-015 and R-106 criteria.
- **Requirements:** R-015, R-106
- **Validation:** test, bench, ui-test, manual design checklist
- **Status:** done 2026-10-18 (see VALIDATION.md). Narrow split-pane filename clipping was fixed in S11 W-076a (2026-11-05), with a design breakpoint, regression/mutation and isolated keyboard/layout evidence.

## Slice S9 — Advanced batch rename
- **Outcome:** Rename many files with patterns, regular expressions and counters, with a live preview and one-step undo.
- **Acceptance:** R-016 criteria.
- **Requirements:** R-016
- **Validation:** test, ui-test
- **Status:** done 2026-10-18 (see VALIDATION.md). Open: killing the process during a rename can leave a file under a `.rustphin-part-` name.

## Slice S10 — Desktop integration
- **Outcome:** Rustphin can be the default file manager: opens from other apps, offers a context menu, "Open with" and properties.
- **Acceptance:** R-017 and R-018 criteria; activating files opens the default application, with confirmation for executables.
- **Requirements:** R-001 (open-with part), R-017, R-018
- **Validation:** test (D-Bus), ui-test, manual
- **Status:** done, 2026-10-19 (W-068; isolated desktop acceptance, with limits in VALIDATION.md)

## Slice S11 — Hardening for daily use
- **Outcome:** Ready for the two-week daily-driver trial.
- **Acceptance:** Contrast checks pass in both surface modes; keyboard-only checklist passes; parser fuzzing has run without findings; dependency audit shows no application network use and compatible licences (D-018's toolkit exception is no longer used: `deny.toml` bans every network crate since W-114i).
- **Requirements:** R-104 (parsers), R-105 (contrast), R-107, R-108 (no Orca pass: D-023)
- **Validation:** test, manual
- **Status:** in progress since 2026-10-20 (W-069 assessment, W-070–W-078 queue). Done: contrast tests in both modes, parser fuzzing (W-075; later targets `admin_entry` and `archive_entry`), licences and privacy (W-077, R-108). Open (ACCEPTANCE.md): the keyboard-only pass on the Qt interface (R-004, R-107), the blur notice (R-109), and the maintainer's real-desktop and hardware checks (R-013, R-105).

## After S11 — Mockup-aligned UI/UX and design (W-079)

- **Requested:** The maintainer, 2026-11-06; start after Slice 11 is complete.
- **Visual target:** [`../mockup.png`](../mockup.png), the user-provided 1672 × 941 reference. Match its appearance as closely as feasible while adapting to Rustphin's implemented features and supported platform behavior.
- **Scope:** tabs/navigation/location toolbar, places and device presentation, file rows/selection, actions palette, context menus, properties and operation status/progress; carry the same design through icons/compact views, split panes, dialogs and states absent from the picture.
- **Workflow:** after the S11 gate, break W-079 into focused steps. Update DESIGN.md first, implement its tokens (then `theme.rs`; now `rustphin-view`'s `colors.rs` and `metrics.rs`, handed to QML as `Tokens`), then update views. Preserve current features, operation safety, keyboard/accessibility behavior and locked platform/toolkit decisions; the image's Network row does not add network features to scope.
- **Validation:** compare real application screenshots with the reference, including normal/minimum sizes, split panes, empty/loading/error/disabled/operation states. Re-run affected contrast, native/keyboard, responsiveness and fallback checks; rust-tc doctor, project evidence and one focused commit after every task.
- **Inputs:** [`DESIGN_BRIEF.md`](DESIGN_BRIEF.md), [`INFORMATION_ARCHITECTURE.md`](INFORMATION_ARCHITECTURE.md), the maintainer's HTML mockups in [`../mockup/`](../mockup/) (34 screens, 2026-11-08) and [`../DESIGN.md`](../DESIGN.md), which W-079 adopted from the target on 2026-11-08 with the mockup's changes.
- **Status:** done 2026-11-08 (D-024: started before the remaining S11 criteria closed), steps W-079a–g, followed by W-086–W-105; carried into the Qt interface in W-114 and compared with the mockups again (W-114h). The glass look on the real desktop is still to be reviewed. The maintainer's next task is a colour scheme and UI refactor.

## Done — Settings window with toolbar customization (W-106)

- **Requested:** The maintainer, 2026-11-11.
- **What:** a Settings window (Dolphin's "Configure Rustphin…") whose first page is **Toolbar**: the user rebuilds the toolbar as they want it, as Dolphin's "Configure Toolbars…" allows. A list of available actions beside the toolbar's current actions; add, remove and reorder them (drag or buttons); separators; per action icon only or icon and text; restore the defaults. The result is saved in Rustphin's own config and applied at once.
- **Before building:** a requirement with acceptance criteria and a design for the maintainer to approve (pages, mockup, how the responsive toolbar rules of W-096 apply to a customized toolbar, where the settings are stored). It adds scope beyond the MVP, so it waits until the maintainer schedules it.
- **2026-11-12:** The maintainer asked for the whole Settings window (General, Appearance, Customizations, About, …) to be planned first. The proposal is [`SETTINGS_PLAN.md`](SETTINGS_PLAN.md): eight pages, storage, order of work W-106a–g and five decisions for the maintainer.
- **Status:** plan and mockups approved 2026-11-12 (D-029, R-019); built the same day as W-106b–g (all eight pages), then in Qt (W-114f). Three differences from the plan accepted (D-030). R-019 is open on Tab through the controls, a second window following changes, and two Appearance choices (Look, Title bar) the Qt interface ignores.

## Done — Background mode and archives (W-110, W-111)

- **Requested:** The maintainer, 2026-11-13, as the next tasks.
- **W-110, background mode:** after its last window closes Rustphin keeps running without a window and can start hidden at login, as Dolphin's daemon mode, so new windows and folders sent by other applications open at once. Follows W-109: reading fonts in parallel brought a cold start to about 3 s, and only a resident process removes it. Costs memory while idle; needs a Settings switch and a way to quit for good.
- **W-111, compress and extract:** "Extract" and "Compress" in the context menu as Dolphin offers them through Ark: extract here, into a new folder, into a chosen folder, or into a new folder then trash the archive; compress here as ZIP or TAR.GZ, or to a chosen name and format.
- **Before building:** each gets a requirement with acceptance criteria and a plan for the maintainer to approve; both add scope beyond the MVP.
- **2026-11-13:** W-110's plan is [`BACKGROUND_PLAN.md`](BACKGROUND_PLAN.md): one Rustphin per session that keeps running without windows (as Nautilus does; Dolphin's `--daemon` only relays D-Bus requests to new processes), launches handed to it, start at login as a Settings switch; work W-110a–f and five decisions for the maintainer. Approved with all five recommendations (D-031, R-020).
- **2026-11-13:** W-111's plan is [`ARCHIVE_PLAN.md`](ARCHIVE_PLAN.md): Ark's Extract and Compress submenus with the maintainer's extra "into a new folder", done by Rustphin's own engine for ZIP, TAR and 7Z with Ark for other formats (recommended); work W-111a–g and five decisions. The maintainer reviews it, then both plans are built.
- **Status:** both built 2026-11-13, then in Qt (W-114c3, W-114f). W-110a–f done (R-020; KWin focus from a real launcher token and a login start need the maintainer's desktop). W-111a–g done (R-021 ticked). W-113 (Extract to… starts in the folder shown) done in W-114g. W-119 (D-036) replaced Ark: other formats are read by libarchive inside the engine, then 7-Zip or unrar; RAR not tried (no fixture).

## Done — Qt 6 and QML interface (W-114)

- **Requested:** The maintainer, 2026-11-13: move the interface from GPUI to Qt 6 and QML, with QtBridge for Rust.
- **Plan:** [`QT_MIGRATION_PLAN.md`](QT_MIGRATION_PLAN.md). Gains are maturity and KDE integration (cold start, blur, file exchange with other applications, focus, text input, accessibility); speed targets are already met on GPUI. Recommended: a measured spike first (W-114a) with go/no-go criteria, cxx-qt as the bridge with QtBridge tried alongside, the Qt interface built beside the GPUI one until parity, then the switch (D-033, superseding D-001). Five decisions for the maintainer.
- **Status:** A–D approved (D-033). Spike W-114a done 2026-11-13: cxx-qt passes 1–4 and 6–9 (startup 0.3–0.4 s against about 10 s); blur confirmed on the maintainer's desktop with the spike. Switch decided (D-034, cxx-qt) and made 2026-11-14 (W-114i): GPUI, GPUI-Kit and `rustphin-ui` removed. W-116 (Qt quality pass: `qmllint` with the BestPractices plugin and `qmlformat` in `rust-tc doctor`, the qt-qml-review findings, 36 scenarios replayed) done 2026-11-14. Then W-118–W-144: libarchive (D-036), interface logic moved into `rustphin-view` (commands, pane state, operation board, selection actions, places panel, workspace, menus), byte-safe paths to QML, test audits and the bugs they found, the GitHub repository (D-037).

## Next — Slice S12: Charcoal colour scheme and UI completion (W-147, W-148, W-149, W-146, W-117)

**Goal:** Rustphin's look moves from navy to dark charcoal, and everything the
documents describe but the interface does not yet do is built. Asked for by the
maintainer on 2026-11-14.

**Colour scheme (W-147, done 2026-11-14, D-038):**
- Base colours are dark charcoal, with black and grey tints for every surface:
  window, chrome, content, panels, menus, dialogs, rows, borders.
- Royal blue is kept only as a small brand accent: primary buttons and glows
  (the focus glow, and the accent glow on menus and dialogs). Selection, the
  current place, the current tab, hover and other states use neutral tints,
  never blue fills.
- Order of work: `docs/DESIGN.md` and a decision record first, then the tokens
  in `crates/rustphin-view/src/colors.rs` with their WCAG contrast tests (glass
  and opaque), then any QML that hard-codes a colour.
- Done when every text and state pair meets WCAG AA, including the default and
  destructive buttons (now about 2:1). State is never shown by colour alone.
  The screens are compared with the mockups in an isolated session, and the
  glass look on the maintainer's desktop.

**Drive cards (W-148, asked for by the maintainer on 2026-11-14):**
- Each volume in the places panel is a card: drive icon and name, "used /
  total", a full-width usage bar (royal blue, red from 90 % full), the
  filesystem and its traits ("Btrfs · Encrypted") and "NN% used".
- The eject button goes; a drive menu (right-click, Menu key, Shift+F10)
  offers Open, Open in new tab, Mount or Unmount, Safely remove and Copy
  location. E still ejects.
- Checked on screen only against a stand-in UDisks2 service on a private
  bus, never the machine's drives.

**Transfer details (W-149, done 2026-11-14):**
- The activity line stays above the status line. Its figures gain the time
  remaining, from a smoothed speed, and it gains a Details button.
- Details opens a transfer dialog with one card per running operation:
  progress, items and bytes, speed, time remaining, the entry being worked
  on, a small speed graph of the last minute and its own Stop.
- The dialog closes itself when the last operation ends, as KDE's copy
  window does; the notice line reports the result.
- The speed, the wording and the graph samples live in `rustphin-view`
  (`transfer`), tested with injected time.

**UI completion (W-146):** each is a defect against the documents.
- Styling not yet built: the accent outline and glow on menus and dialogs;
  tooltips and scrollbars drawn from the tokens rather than Qt's Basic style;
  the Medium (500) weights; the fixed sizes in QML moved into `metrics.rs`;
  unused tokens removed or used.
- Settings that do nothing today:
  - Appearance › Look (glass or opaque) and Title bar (Rustphin's or KDE's);
  - Toolbar › Show its words;
  - the Settings pages become tabs in a narrow window;
  - a text-size change shows at once.
- Blur: a "Background blur unavailable" status line, and following blur when it
  appears or goes while Rustphin runs (R-109).
- Administrator mode as D-028 and ADMIN_MODE.md say: per window, ended when its
  window closes, "(administrator)" in the title, a warning line in folders not
  owned by root, and batch rename refused.
- A notice when the device service (UDisks2) is missing, instead of only a log
  line. A panic in an operation thread is reported to the user.
- Smaller items:
  - the status line names the volume ("(System)");
  - right-clicking the title bar offers KWin's window menu;
  - Extract's "here" entries are disabled in read-only folders.

**Qt leftovers (W-117):**
- The one-tab strip: settle hidden or shown.
- Ctrl+0's default zoom.
- The Trash place highlighted while the trash is shown.
- Notice icons by kind (information, success, error).
- The rename-recovery wording.
- The Run prompt's default button.
- Unnamed buttons over AT-SPI.
- A link's Properties icon.
- Replay J7.

**Done when:** the items above are built and tested, `docs/DESIGN.md` and the
other documents describe exactly what is built, and the UI scenarios they touch
are replayed on screen.

## What remains

- **Next:** Slice S12 above: W-146 and W-117 (the charcoal colour scheme W-147, the drive cards W-148 and the transfer details W-149 are done).
- **Also open:** W-115 (memory grows on each reload of a very large folder).
- **S11 acceptance:** a keyboard-only pass of the Qt interface (R-004, R-107, R-019); the maintainer's real-desktop and hardware checks (R-013, R-105, R-020 focus, W-098e administrator mode).
- **Then:** the two-week daily-driver trial (the MVP success signal), and phase 7 (release).

## Traceability

| Requirement | Slice(s) | Validation |
|---|---|---|
| R-001 Browse directories | S1, S10 | test, ui-test, manual |
| R-002 Live updates | S3 | test |
| R-003 View modes, sorting, zoom | S2 | test, ui-test |
| R-004 Selection and keyboard | S2 | ui-test, manual |
| R-005 Tabs | S3 | ui-test |
| R-006 Split view | S3 | ui-test |
| R-007 File operations | S4 | test, manual |
| R-008 Undo | S4 | test |
| R-009 Trash | S4 | test, manual |
| R-010 Drag-and-drop | S5 | ui-test, manual |
| R-011 File clipboard | S5 | ui-test, test |
| R-012 Places and place groups | S6 | test, ui-test, manual |
| R-013 Devices | S6 | test, manual |
| R-014 Thumbnails and icons | S2, S7 | test, manual |
| R-015 Command palette | S8 | test, bench, ui-test |
| R-016 Batch rename | S9 | test, ui-test |
| R-017 Default file manager | S10 | test, manual |
| R-018 Context menu and properties | S10 | ui-test, manual |
| R-019 Settings window | W-106 | test, ui-test |
| R-020 Background mode | W-110 | test, ui-test, manual |
| R-021 Compress and extract | W-111 | test, fuzz, ui-test |
| R-101 Responsiveness at scale | S1, S2 | bench, ui-test |
| R-102 Memory | S2 | bench |
| R-103 Data safety | S4 | test |
| R-104 Untrusted input | S1, S7, S11 | test |
| R-105 Visual design | S1, S11 | test, manual |
| R-106 Progressive disclosure | S8 | manual, ui-test |
| R-107 Accessibility baseline | S11 | manual |
| R-108 Privacy | S11 | test, manual |
| R-109 Platform and build | S1 | test |
