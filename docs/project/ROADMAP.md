# Roadmap (vertical slices)

Each slice ends with something the author can run and use, and with its acceptance demonstrated. Order reflects dependencies and risk: scale and data safety are proven early.

## Slice S1 — Open and browse a folder
- **Outcome:** Launch Rustphin, see a folder's entries in the frosted-glass window, and move around with mouse and keyboard, including in a 500,000-entry folder.
- **Acceptance:**
  - Workspace builds with one command on Rust 1.95 and stable; format, lint and test checks pass.
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
- **Acceptance:** Contrast checks pass in both surface modes; keyboard-only checklist passes; parser fuzzing has run without findings; dependency audit shows no application network use (toolkit exception under D-018) and compatible licences.
- **Requirements:** R-104 (parsers), R-105 (contrast), R-107, R-108 (no Orca pass: D-023)
- **Validation:** test, manual
- **Status:** in progress, 2026-10-20; W-069 assessment and W-070–W-078 queue

## After S11 — Mockup-aligned UI/UX and design (W-079)

- **Requested:** The maintainer, 2026-11-06; start after Slice 11 is complete.
- **Visual target:** [`../mockup.png`](../mockup.png), the user-provided 1672 × 941 reference. Match its appearance as closely as feasible while adapting to Rustphin's implemented features and supported platform behavior.
- **Scope:** tabs/navigation/location toolbar, places and device presentation, file rows/selection, actions palette, context menus, properties and operation status/progress; carry the same design through icons/compact views, split panes, dialogs and states absent from the picture.
- **Workflow:** after the S11 gate, break W-079 into focused steps. Update DESIGN.md first, implement its tokens through theme.rs, then update views. Preserve current features, operation safety, keyboard/accessibility behavior and locked platform/toolkit decisions; the image's Network row does not add network features to scope.
- **Validation:** compare real application screenshots with the reference, including normal/minimum sizes, split panes, empty/loading/error/disabled/operation states. Re-run affected contrast, native/keyboard, responsiveness and fallback checks; rust-tc doctor, project evidence and one focused commit after every task.
- **Inputs:** [`DESIGN_BRIEF.md`](DESIGN_BRIEF.md), [`INFORMATION_ARCHITECTURE.md`](INFORMATION_ARCHITECTURE.md), the maintainer's HTML mockups in [`../mockup/`](../mockup/) (34 screens, 2026-11-08) and [`../DESIGN.md`](../DESIGN.md), which W-079 adopted from the target on 2026-11-08 with the mockup's changes.
- **Status:** in progress since 2026-11-08 (D-024: started before the remaining S11 criteria close). Steps W-079a–g in WORK_ITEMS.md.

## Later — Settings window with toolbar customization (W-106)

- **Requested:** The maintainer, 2026-11-11, for later; not started.
- **What:** a Settings window (Dolphin's "Configure Rustphin…") whose first page is **Toolbar**: the user rebuilds the toolbar as they want it, as Dolphin's "Configure Toolbars…" allows. A list of available actions beside the toolbar's current actions; add, remove and reorder them (drag or buttons); separators; per action icon only or icon and text; restore the defaults. The result is saved in Rustphin's own config and applied at once.
- **Before building:** a requirement with acceptance criteria and a design for the maintainer to approve (pages, mockup, how the responsive toolbar rules of W-096 apply to a customized toolbar, where the settings are stored). It adds scope beyond the MVP, so it waits until the maintainer schedules it.
- **2026-11-12:** The maintainer asked for the whole Settings window (General, Appearance, Customizations, About, …) to be planned first. The proposal is [`SETTINGS_PLAN.md`](SETTINGS_PLAN.md): eight pages, storage, order of work W-106a–g and five decisions for the maintainer.
- **Status:** plan and mockups approved 2026-11-12 (D-029, R-019); built the same day as W-106b–g (all eight pages). R-019 is open on Tab through the controls, a second window following changes, and three differences from the plan for the maintainer to accept or change.

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
| R-101 Responsiveness at scale | S1, S2 | bench, ui-test |
| R-102 Memory | S2 | bench |
| R-103 Data safety | S4 | test |
| R-104 Untrusted input | S1, S7, S11 | test |
| R-105 Visual design | S1, S11 | test, manual |
| R-106 Progressive disclosure | S8 | manual, ui-test |
| R-107 Accessibility baseline | S11 | manual |
| R-108 Privacy | S11 | test, manual |
| R-109 Platform and build | S1 | test |
