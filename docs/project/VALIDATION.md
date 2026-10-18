# Validation evidence

### 2026-10-18 — W-067 properties
- **Phase:** 6 Implement (S10)
- **Result:** pass for the facts, the folder-size walk and changing permissions. Not yet seen on screen; that is W-068.
- **Change:** Alt+Enter or Properties opens a dialog for the one selected entry: name, type, size, modified, accessed and created when the filesystem has it. A folder's size is added up off the UI thread, without following symbolic links, and says when some items could not be read. Closing the dialog stops that walk. Permissions are nine checkboxes; special bits are kept. Apply writes them through the operation engine, so the change can be undone, and a failed write leaves the old mode. A shortcut shows where it points and does not change permissions. `ShowItemProperties` now delivers the local paths.
- **Evidence:** `rust-tc doctor` PASS, 368 tests. A hand edit that counted a symbolic link's length failed `a_folder_size_adds_files_skips_links_and_counts_what_it_cannot_read` (153 bytes instead of 105); the skip was restored. `npx @google/design.md lint docs/DESIGN.md` reported 0 errors. The dialog was not opened in a session.
- **Follow-up:** W-068 replays the desktop-integration scenarios, including this dialog and the context menu.

### 2026-10-18 — W-066 context menu and run confirmation
- **Phase:** 6 Implement (S10)
- **Result:** pass for the menu contents, application lookup and the decision to ask before running. Not yet seen on screen; that is W-068.
- **Change:** Right-click, or Shift+F10, opens a menu for the selection (open, open with, cut, copy, paste, rename, trash, delete) or, on empty space, paste and new entries. Trash offers restore instead of a second trash. Open with lists applications registered for the file's type, default first, including a parent type, and leaves out removed ones. A normal file opens with the default application. An executable or a `.desktop` file asks first: Keep does nothing, Run starts it, and an executable can instead be opened. Commands are started directly, not through a shell. A desktop file that is not valid UTF-8 or is not an application is skipped.
- **Evidence:** `rust-tc doctor` PASS, 362 tests. A hand edit that disabled Move to trash failed `several_files_cannot_be_opened_or_renamed_as_one`; it was restored. A hand edit that ignored removed associations failed `the_default_application_leads_and_a_removed_one_is_absent`; it was restored. `npx @google/design.md lint docs/DESIGN.md` reported 0 errors. The menu was not opened in a session; W-068 does that.
- **Follow-up:** W-067 is the properties view. Seeing the menu in an isolated session is part of W-068.

### 2026-10-18 — W-065 ShowItems
- **Phase:** 6 Implement (S10)
- **Result:** pass on a private bus and in the headless grouping; not yet called from another application on the real session bus
- **Change:** Rustphin serves `org.freedesktop.FileManager1`. `ShowItems` for a file opens its folder and selects its name; several files in one folder are one request; a directory opens itself; a missing file is still named in its parent. The window applies those requests on the next frame, reusing the current tab when it is already there. If another file manager owns the bus name, Rustphin logs that and continues. `ShowItemProperties` is accepted and does nothing until the properties view exists.
- **Evidence:** `rust-tc doctor` PASS, 346 tests. A private peer-to-peer connection, with no session bus, called `ShowItems` and the service reported the folder and the file name. A hand mutation that treated a directory as a file to select failed `a_directory_opens_itself_and_a_missing_file_is_still_named`; the check was restored.
- **Follow-up:** W-066 is the context menu. Seeing ShowItems from another application on the desktop remains for the S10 scenario pass.

### 2026-10-18 — W-064 desktop entry and command-line tabs
- **Phase:** 6 Implement (S10)
- **Result:** pass for the desktop entry and argument parsing; ShowItems is W-065
- **Change:** `assets/rustphin.desktop` names Rustphin as a handler for `inode/directory` and passes URLs to `rustphin %U`. Command-line paths and `file:` URLs become one tab each, in order, instead of restoring the session. A path that names an existing file opens the folder that contains it. A URL for another host is ignored.
- **Evidence:** `rust-tc doctor` PASS, 341 tests. A hand mutation that kept the program name as an argument failed `no_arguments_opens_nothing_explicit`; the skip was restored. The desktop file is checked by a test for its type, folder MIME type, `%U` and window class. Not yet tried from the Plasma menu or as the system default.
- **Follow-up:** W-065 is the FileManager1 ShowItems interface.

### 2026-10-18 — W-063 batch-rename scenarios; S9 closed
- **Phase:** 6 Implement (S9)
- **Result:** pass for the dialog on screen. S9 closed. Killing the process during a rename remains open (R-103).
- **Change:** scenarios B1–B4 added to UI_SCENARIOS.md. No application code.
- **UI evidence:** isolated KWin, isolated home, generated files only. No device was activated. Release build.
  - B1: Ctrl+A was not used; Down and Shift+Down selected `note.txt` and `other.txt` (status "2 of 2 items selected"). F2 opened a solid dialog, "Rename 2 items", Replace filled and Pattern and Case outlined. Both current names were listed. The sentence "There is nothing to find. Enter the text that should be replaced." was shown and Rename was disabled.
  - B2: Find `note`, replace `other`. The preview showed both files becoming `other.txt`, with the sentences for a duplicate target and for a name a staying file already has. Rename stayed disabled.
  - B3: The replacement was changed to `photo`. The preview showed `note.txt` → `photo.txt` and `other.txt` unchanged, with no problem sentence, and Rename filled. Enter closed the dialog. The list and the directory then held `other.txt` (6 bytes) and `photo.txt` (5 bytes, the original note).
  - B4: Ctrl+A, F2, Escape. The directory listing immediately afterwards was still only those two names. The capture after Escape was the same size as the closed list view; the session cleanup then removed the image, so that frame was not re-read.
  - Pattern, on a second fixture (`aa.txt`, `bb.txt`): the mode showed `@-#`, start 1, step 1, the sentence explaining `#` and `@`, and the preview `aa.txt-1`, `bb.txt-2`.
  - Case: Lower case, Upper case and Capitalise words were shown. Lower case was selected. The sentence "Nothing would change. Edit the rule to give the files new names." was shown and Rename was disabled, because the names were already lower case.
- **Not claimed:** a regular expression typed on screen (the checkbox was visible; the expression itself is covered by the W-060 test), a counter padded with more than one `#` typed on screen (the sentence states that rule; the unit test checks it), undo of a batch rename on screen (the engine test exchanges two names and undoes that one history entry), and a process killed mid-rename.
- **Follow-up:** S10 is desktop integration. The kill gap stays open.

### 2026-10-18 — W-062 batch-rename dialog
- **Phase:** 6 Implement (S9)
- **Result:** pass for the dialog's construction; it has not yet been replayed on screen
- **Change:** F2 with two or more selected files opens a solid dialog. Replace, Pattern and Case are labelled modes, the chosen one filled. The preview lists every current and new name and writes each problem as a sentence. Rename stays disabled while a problem remains or nothing would change. Enter runs `Request::RenameBatch`; Escape closes without a change. DESIGN.md was edited before the dialog; the linter reports zero errors and the same eight orphan-token warnings.
- **Evidence:** `rust-tc doctor` PASS, 335 tests. No new pure function: the preview rules are the W-060 tests, and the apply path is the W-061 tests. The dialog calls those APIs.
- **Follow-up:** W-063 replays the dialog in an isolated session, including a conflict that blocks Rename and a successful apply.

### 2026-10-18 — W-061 apply a batch rename
- **Phase:** 6 Implement (S9)
- **Result:** pass for applying and undoing one plan; a killed process is not yet covered
- **Change:** `Request::RenameBatch` renames every pair in one directory or, when a rename fails and the repairs succeed, changes nothing. Swaps and chains go through temporary names and are stored as one `Change::BatchRenamed`, so one undo puts every file back. A name already used by a file that is staying is refused before any change.
- **Evidence:** `rust-tc doctor` PASS, 335 tests. Five new tests: three renames undone as one history entry, an exchange of two names undone, a chain onto a name another selected file is leaving, a refusal that leaves both files in place, and a fault on each of the four renames of a two-file plan with every file back at its original name and content. A hand mutation that skipped the repair failed `a_failed_batch_rename_puts_every_file_back` because an original file was missing; the repair was restored.
- **Not claimed:** killing the process mid-rename. A crash can leave a file under a `.rustphin-part-` name, which is neither its old name nor its new one. That part of R-103 stays open.
- **Follow-up:** W-062 is the dialog. The kill gap stays until a batch rename can be finished or reversed after the process dies.

### 2026-10-18 — W-060 batch-rename plan
- **Phase:** 6 Implement (S9)
- **Result:** pass for the plan and preview; nothing is renamed yet
- **Change:** `RenameRules::preview` rewrites selected names by plain or regular-expression replacement, by a counter pattern (`#`, start, step, padding, `@` for the current name), or by Unicode case. It reports duplicate targets, names that a staying file already has, and names that cannot be used. It does not read or write the filesystem. The `regex` crate was added to the core; its licence is MIT OR Apache-2.0.
- **Evidence:** `rust-tc doctor` PASS, 330 tests. A hand mutation that stopped flagging duplicate targets failed `two_files_given_the_same_name_are_both_flagged`; the check was restored.
- **Follow-up:** W-061 applies one accepted plan through the operation engine as a single undo step.

### 2026-10-18 — W-059 palette acceptance; S8 closed
- **Phase:** 6 Implement (S8)
- **Result:** pass. S8 closed. Two limits are recorded below and are not treated as S8 failures.
- **Change:** a pane under 320 px drops Forward and Up; under 240 px it also drops Back. The location and Actions stay. The status line truncates inside the pane. `Navigation::for_width` is the tested rule. DESIGN.md gained the two widths before the behaviour, and the design linter reports zero errors and the same eight orphan-token warnings as before.
- **Evidence:** `rust-tc doctor` PASS, 311 tests. A hand mutation changing the 320 px comparison from `>=` to `>` failed `narrow_panes_drop_forward_and_up_then_back` on the boundary; the comparison was restored. The core `rust-tc bench-palette` result from W-056 is unchanged and was not re-run.
- **UI evidence:** isolated KWin, isolated home, generated fixture only. No device row was activated. Release build for the latency and restart checks.
  - Q1: Tab moved the selection from Back to Forward; Shift+Tab moved it back. The query stayed empty. Escape returned to the file pane. Selection fill was the royal-blue primary on a normal screenshot.
  - Q4: docs was added as a place, moved into a group named Work, and the group was folded (right arrow and a count of 1). Query `place docs` still selected `Place: docs`, and Enter opened that folder. The group stayed folded.
  - Q6: `clear recent` left the palette open with "Recent folders cleared." `recent-locations.toml` became `version = 1` and `locations = []`. After restart in the same home the 10,000 seeded paths were gone. The launched fixture was then the only saved visit. Opening `other` wrote that folder first and the fixture second.
  - J1: at 960×640 the toolbar shows Back, Forward, Up, the location and labelled Actions. Places and the status line are present. One tab, so no tab strip.
  - J5: a pane about 260 px wide kept Back and dropped Forward and Up; the date column was also gone. Splitting that window put each pane under 240 px: Back, Forward and Up were all gone, Actions stayed, and the status read "1 of 3 items…" with the rest cut off. Alt+Left returned from docs to the fixture in that state. Alt+Right and Alt+Up were not pressed there.
  - Latency: 10,000 saved locations plus 75 commands, one tab and Clear, which is 10,077 candidates. The debug log `palette results rendered` measures from the start of the search (the input change) to the render that shows the new results, not the compositor's present. Worst keystroke in the session was 10,536 µs. The empty-query render after opening was 3,927 µs. Both are under R-015's 50 ms.
  - Every application action was in that open set: 10,000 locations + 75 `command!` entries + the current tab + Clear = 10,077, matching the logged result count. Running a chosen row was observed for the folded place and for clear.
- **Not claimed:** screen-reader output, blur on the real desktop, Alt+Right and Alt+Up while the buttons are hidden, and a status string long enough to need the ellipsis outside the split. At the split width the 88 px size column remains, so entry names can disappear; that is the existing column rule, not a new one. Properties are still S10. There is no separate filter control.
- **Follow-up:** S9 is batch rename (R-016). The clipped names in a very narrow split stay an open loop.

### 2026-10-18 — W-058 progressive disclosure
- **Phase:** 6 Implement (S8)
- **Result:** pass for the simpler default chrome; final acceptance checklist remains W-059
- **Change:** the browser toolbar keeps Back/Forward/Up, breadcrumbs or location input, and one labelled Actions button. Secondary creation, trash, view and hidden-file buttons move to the palette; existing keyboard shortcuts remain. The Places header becomes a label; its add/group/hidden-place actions remain in the palette and shortcuts. Removed unused imports and the obsolete hidden-state toolbar accessor. Palette and dialogs now share the design's floating-surface shadow from Theme. DESIGN.md was edited before implementation; old UI-scenario button references were updated and J1–J4 describe reachability checks.
- **Evidence:** `rust-tc doctor` PASS, 310 tests; DESIGN.md linter zero errors/eight orphan-token warnings; debug application built. No new pure logic warranted additional unit tests.
- **Observed:** fresh explicit fixture tab in the same isolated KWin/home has the simpler toolbar and Places header. Clicking Actions opens the palette; choosing New folder creates the fixture folder and focuses its inline name field. After Escape, choosing New group focuses the places panel and opens its name field; entering Work saves that name in place-groups.toml. Normal screenshots show the shared royal-blue glow, solid palette, readable text and full-width selection. No real device was activated.
- **Follow-up:** W-059 replays reachability more broadly, checks persistence, folded places, Tab navigation and narrow layouts, measures the 10,000-location UI path and closes S8 against evidence.

### 2026-10-18 — W-057 command palette and persistent history
- **Phase:** 6 Implement (S8)
- **Result:** pass for palette integration; progressive disclosure and final latency acceptance remain W-058/W-059
- **Change:** Ctrl+Shift+P opens a solid, token-based, virtualised palette. Named actions dispatch to the active pane or places panel through existing handlers, preserving operation confirmations; actions, places, tabs and recent folders are fuzzy-searchable. Background ranking rejects stale results and invalidates activation while searching. Successful completed folder visits feed one background history worker; loading, byte-safe URI/TOML encoding and serial atomic writes stay off the UI thread. Clear stays in the palette with confirmation. Damaged/newer history files are reported and protected from overwrite. Debug-level latency logs contain only elapsed time/result counts, not queries or paths.
- **Evidence:** `rust-tc doctor` PASS, 310 tests (eight new); eight manual mutations through `rust-tc test` were caught by their intended tests (raw path round trip, rejected location types, future version, damaged history, empty saved history, stale results, resetting selection and wrapping navigation). Added palette-hover pairs to both existing contrast tests; a deliberate unreadable background mutation failed both with the intended contrast message. All mutations restored. DESIGN.md linter: zero errors, eight orphan-token warnings.
- **UI evidence:** isolated KWin/home, generated fixture only. Observed opening the palette, full-width selected result with marker and shortcuts, Down selecting the next result, fuzzy action search and Enter changing to icons (also checked settings.toml), recent-folder Enter navigation, named-tab activation without adding a tab (checked session.toml), mouse activation of a place result, helpful no-match text, blocked Ctrl+T/F3 while searching, clear confirmation and an empty saved locations list, Escape returning to the browser, and palette shortcut blocked behind the permanent-delete confirmation (cancelled; file kept). An unsuccessful location request did not enter history. Q1–Q8 describe replayable checks; Tab/Shift+Tab, folded places and history after restart are still to be replayed in W-059.
- **Corrections found by UI replay:** initial Up/Down bindings used a conjunction for two different key-context nodes and did not select a result; `Palette > Input` fixes their descendant relationship. Result rows initially measured to their content; explicit full width fixes alignment and selection width. Replayed Down and action activation after rebuilding.
- **Environment:** the configured KWin-MCP server lacks XDG_RUNTIME_DIR, so its isolated-session wrapper could not create the socket and waited indefinitely. Cleaned up only those failed test sessions, then used the installed `kwin-mcp-cli` with XDG_RUNTIME_DIR=/run/user/1000 and the same session_start/isolate_home API. No installed tool or desktop configuration changes. AT-SPI still does not expose Rustphin; screenshots and on-disk isolated state were used. Delayed frame captures swap red/blue channels in this environment; normal screenshot captures were used for colour review. Real devices were not activated.
- **Follow-up:** W-058 simplifies default chrome. W-059 finishes reachability, persistence, focus, narrow-window and UI latency checks before closing S8. No claim of complete keyboard, screen-reader or real-desktop blur acceptance.

### 2026-10-18 — W-056 fuzzy matching and recent-location history
- **Phase:** 6 Implement (S8)
- **Result:** pass for the headless foundation; no palette UI yet
- **Change:** core `SearchQuery` ranks exact names, prefixes, word substrings, other substrings and subsequences, with Unicode lowercasing, stable ties and bounded result count. `RecentLocations` keeps at most 10,000 distinct absolute paths, newest first, preserves raw bytes, rejects NUL/relative paths and can be cleared. Neither API performs I/O. No dependencies added.
- **Evidence:** `rust-tc quick` and `rust-tc doctor` PASS, 302 tests (11 new). Eleven hand mutations run through `rust-tc test` were detected by their intended new tests: Unicode/query normalisation, match-class ranking, missing/repeated characters, gap preference, tie order, result limit, revisit deduplication, restore deduplication, history bound, byte preservation and clearing. The initial gap mutation survived because candidate length also determined the same order; the case was strengthened and that mutation then failed. All source mutations were restored.
- **Performance:** `rust-tc bench-palette` PASS: 10,000 generated paths, six queries including Unicode, subsequences and no match, 20 runs each; worst per-query times 0.298–2.091 ms including query creation, matching and sorting, below R-015's 50 ms limit. This measures the matcher, not input-to-frame latency in the future UI.
- **Follow-up:** W-057 wires these APIs to the palette and persistent history; W-059 verifies the complete interaction. Existing S1–S7 limitations remain.

### 2026-10-18 — W-055 baseline quality gate restored
- **Phase:** 6 Implement (S8 preparation)
- **Result:** pass
- **Approval:** The maintainer explicitly approved the three proposed exceptions and continuing S8 in this chat on 2026-10-18.
- **Change:** `deny.toml` individually documents RUSTSEC-2026-0370, RUSTSEC-2026-0371 and RUSTSEC-2026-0372, their retained dependency paths and review at the next toolkit upgrade. These are informational lifecycle notices; no vulnerability exception, dependency update, toolkit upgrade or lock change was made.
- **Evidence:** `rust-tc doctor` PASS: formatting, clippy, 291 tests, doctests, dependency advisories/licences/bans/sources, unused dependencies, feature checks, layering and Rust 1.95 check. Existing duplicate-version warnings remain warnings under the reviewed policy.
- **Follow-up:** W-056 starts the S8 implementation; W-057 to W-059 cover the palette, progressive disclosure and UI acceptance.

### 2026-10-18 — S8 baseline gate blocked by new maintenance advisories
- **Phase:** 6 Implement (S8 preparation, W-055)
- **Result:** blocked; no application changes made
- **Evidence:** `rust-tc doctor` ran before edits. Formatting, compilation through clippy, 291 tests and doctests passed. Dependency checking failed on three informational maintenance notices: RUSTSEC-2026-0370 (`zvariant`), RUSTSEC-2026-0371 (`zbus_names`) and RUSTSEC-2026-0372 (`zvariant_derive`). Licences, bans and sources passed. The command stopped at `deps`; subsequent gates were not run by that invocation.
- **Investigation:** the dependency paths printed by cargo-deny include the pinned GPUI toolkit's AT-SPI, portal and keyring dependencies as well as Rustphin's zbus 5 device integration. Upstream confirms that the three standalone crates were merged into zbus 6 and deprecated: https://github.com/z-galaxy/zbus/issues/2043. Updating only the direct zbus dependency cannot eliminate the toolkit's copies.
- **Proposal:** individually document these three lifecycle exceptions in `deny.toml`, retaining the toolkit pin and requiring review at its next upgrade. Exact proposed patch: `/tmp/rustphin-proposed-advisory-policy.patch`. Not applied; owner approval requested because AGENTS.md prohibits suppressing gate failures.
- **Remaining baseline checks:** `rust-tc features`, `rust-tc layers` and `rust-tc msrv` each passed when run separately after the doctor command stopped. `git diff --check` passed for the planning and evidence edits. These do not turn the failed doctor result into a pass.
- **Follow-up:** resolve W-055 before application work. No commit while `rust-tc doctor` fails. S8 work items W-056 to W-059 are planned in WORK_ITEMS.md.

### 2026-10-12 — Intake gate
- **Phase:** 0 Intake
- **Result:** pass
- **Evidence:** Observed: working directory empty, not a git repository, no prior workflow artifacts; `rustc 1.98.1`, `cargo 1.98.1` installed. STATE.md records profile (product), goal, current phase and next action.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — GPUI desk check (source reading, nothing executed)
- **Phase:** 2 Discovery (early, ahead of the Idea gate; informs the spike, does not replace it)
- **Result:** partial — no show-stopper found; two gaps identified
- **Evidence:** Read from `zed-industries/zed` at commit `7eccdcec` (2026-10-12), crates `gpui` (0.2.2, `publish = true`) and `gpui_linux`. All items are *observed in source*, not *observed working*:
  - Drag files out, Wayland: present. `start_external_drag` in `gpui_linux/src/linux/wayland/client.rs` offers `text/uri-list` with Copy|Move actions.
  - Drag files out, X11: no implementation found; only the receiving side of XDND appears in `x11/client.rs`. The platform trait default returns false.
  - Clipboard: `ClipboardEntry` has `String`, `Image`, `ExternalPaths`. The Wayland clipboard module references `text/uri-list` but no code writing `ExternalPaths` was found there, and no KDE cut marker (`application/x-kde-cutselection`) appears anywhere. Copy/cut of files to other apps is likely a gap.
  - Blur behind window: present on Wayland through KWin's `org_kde_kwin_blur` protocol (`WindowBackgroundAppearance::Blurred`).
  - Accessibility: AccessKit is integrated in `gpui` (`window/a11y.rs`), with `accesskit_unix` in `gpui_linux`. Earlier assumption that accessibility is absent was wrong.
- **Waiver:** none
- **Follow-up:** Discovery spike must run each item on Plasma Wayland; decide how to cover file clipboard (upstream patch, or own Wayland data-control/clipboard handling) and whether X11 drag-out matters.

### 2026-10-12 — Idea gate
- **Phase:** 1 Idea
- **Result:** pass
- **Evidence:** User agreed in conversation to the brief, the success signal, all three defining features in the MVP, dark-only, and Wayland-only. BRIEF.md updated accordingly.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — GPUI toolkit spike (runtime)
- **Phase:** 2 Discovery
- **Result:** pass with one confirmed gap and one item unverified
- **Setup:** `spikes/gpui-toolkit` (disposable), GPUI from `zed-industries/zed` @ `7eccdcec`, release build (2m17s cold) with rustc 1.98.1. Run in an isolated KWin Wayland session (kwin-mcp) next to the installed Dolphin; input driven through KWin EIS. Plasma 6.7.5.
- **Observed working:**
  - Drag a file out to Dolphin: Dolphin showed its Move/Copy/Link menu; "Copy Here" produced `spike-file-02.txt` in the target folder. Screenshot: `spikes/gpui-toolkit/evidence-drag-out.png`.
  - Drop a file from Dolphin into the spike: `on_drop::<ExternalPaths>` received the path.
  - 500,000-row `uniform_list` renders and scrolls; process RSS about 88 MB. No frame timing was measured.
  - Breeze SVG icons load from `/usr/share/icons/breeze-dark` through `img(PathBuf)` at 16 px and 64 px.
  - Translucent window: content behind shows through. Screenshot: `spikes/gpui-toolkit/evidence-translucency.png`.
- **Observed failing (confirmed gap):**
  - Clipboard write of `ClipboardEntry::ExternalPaths`: Dolphin's paste treated it as text ("Paste Clipboard Content" filename prompt), not as files.
  - Clipboard read after copying a file in Dolphin: `read_from_clipboard()` returned `None`.
- **Not verified:**
  - Blur behind the window: the isolated session showed transparency but no blur. Unknown whether that is the virtual session (blur effect not loaded) or GPUI. Needs a run on the real desktop.
  - Scroll smoothness / frame times at 500k rows.
- **Other observations:**
  - The 16 px row icons were missing in two screenshots taken while the spike window was not focused and present when focused. Unexplained; to recheck.
  - GPUI HEAD pins toolchain 1.98.1; an MSRV of 1.95 is not guaranteed against it.
  - GPUI-Kit (`gpui-kit` 0.7.3 on crates.io, Apache-2.0, needs Rust 1.92+) builds on `gpui-pre` 0.3.9, a crates.io snapshot of zed@93d7456, not on Zed HEAD. The `external_drag_payload` API is present in `gpui-pre` 0.3.9 source; its Wayland backend (`gpui-pre-linux`) was not checked, and the spike did not use GPUI-Kit.
- **Waiver:** none
- **Follow-up:** (1) user runs the spike on the real desktop to check blur; (2) rebuild the spike on `gpui-kit` 0.7.3 to confirm drag-out there; (3) design our own file clipboard (text/uri-list plus KDE cut marker) on the Wayland data-device, outside GPUI's clipboard API.

### 2026-10-12 — Blur behind window (real desktop)
- **Phase:** 2 Discovery
- **Result:** pass, with a workaround outside GPUI
- **Evidence:**
  - Observed by the user on the real Plasma 6.7.5 desktop: with stock GPUI (`WindowBackgroundAppearance::Blurred`) the window was transparent but not blurred (user screenshot).
  - Cause, observed: `wayland-info` on this session lists `ext_background_effect_manager_v1` and no `org_kde_kwin_blur_manager`; KWin's blur effect is loaded (`isEffectLoaded blur` = true). GPUI at zed@7eccdcec binds only `org_kde_kwin_blur_manager` and contains no reference to `background_effect`.
  - Workaround, `spikes/gpui-toolkit/src/blur.rs`: borrow GPUI's `wl_display` and `wl_surface` through raw-window-handle, bind `ext_background_effect_manager_v1` on a private queue and set a blur region. The app logged "BLUR requested via ext-background-effect-v1" and the user confirmed the window is now blurred.
- **Not verified:** behaviour on resize, on compositors without the protocol (expected: falls back to plain transparency, so the theme needs a more opaque fallback), and in-window backdrop blur for panels and popovers, which GPUI does not offer.
- **Waiver:** none
- **Follow-up:** production version belongs in `rustphin-platform`; consider upstreaming to GPUI. The same borrowed-connection technique is the candidate route for the file clipboard.

### 2026-10-12 — File clipboard prototype
- **Phase:** 2 Discovery
- **Result:** pass
- **Setup:** `spikes/gpui-toolkit/src/clipboard.rs`, using `wl-clipboard-rs` 0.9.4 over KWin's `ext_data_control_manager_v1` (advertised on this session), independent of GPUI's clipboard. Offers `text/uri-list`, `application/x-kde-cutselection`, `x-special/gnome-copied-files` and plain text. Isolated KWin session with the installed Dolphin.
- **Observed working:**
  - Copy 2 files in the spike → Dolphin's context menu offered "Paste 2 Items"; both files appeared in the target folder and the sources remained.
  - Cut 1 file in the spike → Dolphin offered "Paste One File"; the file appeared in the target and was removed from the source folder.
  - Copy a file in Dolphin → spike read its path with cut = false.
  - Cut a file in Dolphin → spike read its path with cut = true.
  - GPUI's own `read_from_clipboard()` returned `None` in both cases, as before.
- **Not verified:** paths needing percent-encoding (spaces, non-ASCII), very large selections, non-KDE targets, and compositors without a data-control protocol.
- **Note:** Ctrl+V in Dolphin raised "Ambiguous shortcut detected" in the user's Dolphin configuration; pastes were done through Dolphin's context menu. Unrelated to the spike.
- **Waiver:** none
- **Follow-up:** production implementation in `rustphin-platform` with proper URI encoding; decide between `wl-clipboard-rs` and a direct implementation.

### 2026-10-12 — Spike rebuilt on GPUI-Kit 0.7.3
- **Phase:** 2 Discovery
- **Result:** pass
- **Setup:** `spikes/gpui-kit-check` compiles the same source files against `gpui-kit` 0.7.3 (`gpui-pre` / `gpui-pre-linux` 0.3.9, `gpui-component` 0.7.3) from crates.io. No source changes were needed.
- **Observed working:** app starts and renders the 500k-row list and Breeze icons; drag-out to Dolphin showed the Move/Copy/Link menu and "Copy Here" produced `spike-file-03.txt`; the blur request was accepted ("BLUR requested via ext-background-effect-v1").
- **Not verified:** visible blur on the real desktop with this build (only the Zed-HEAD build was seen blurred by the user); GPUI-Kit's own components (the spike uses plain GPUI elements only).
- **Waiver:** none
- **Follow-up:** try GPUI-Kit components (sidebar, virtual list, command, menu) during Architecture; they are not a Discovery risk.

### 2026-10-12 — Directory listing measurement
- **Phase:** 2 Discovery
- **Result:** pass (informational)
- **Evidence:** `spikes/listing-bench`, 500,000 empty files on XFS, warm cache, 16 threads, two runs: `read_dir` 63 ms; stat all 579 ms single-threaded, 53–60 ms with rayon; natural sort 181 ms single-threaded, 26–29 ms with rayon. Test files deleted afterwards.
- **Waiver:** none
- **Follow-up:** cold cache, Btrfs and UI frame times in the first implementation slice.

### 2026-10-12 — Discovery gate
- **Phase:** 2 Discovery
- **Result:** pass — stance: go, with conditions (DISCOVERY.md)
- **Evidence:**
  - Constraints inventoried: DISCOVERY.md "Constraints".
  - Security, privacy, accessibility, data lifecycle, operations: DISCOVERY.md "Proportionate checks".
  - Top risks with mitigation or accept: DISCOVERY.md "Top risks" (8 risks; 6 and 8 accepted for MVP).
  - No known show-stopper: three candidates closed by spikes recorded above.
  - Feasibility stance recorded.
  - Claims labelled observed, sourced or assumed.
- **Waiver:** none. Risks 6 and 8 are accepted on the agent's recommendation and need the user's confirmation at the MVP checkpoint.
- **Follow-up:** open items listed in DISCOVERY.md carry into Spec and the first slice.

### 2026-10-12 — Spec gate
- **Phase:** 3 Spec
- **Result:** pass
- **Evidence:** REQUIREMENTS.md: 27 MVP requirements, each with acceptance criteria and a validation method; 9 non-functional requirements explicit; must/should priorities and a Later list of 14 items; non-goals match the brief; traceability table present. User confirmed the MVP boundary in conversation on 2026-10-12, including accepted risks 6 and 8.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — Decision-lock gate
- **Phase:** 4 Decision lock
- **Result:** pass
- **Evidence:** DECISIONS.md holds records for all consequential choices. D-001, D-002, D-003, D-006, D-007, D-008, D-009, D-010, D-011 are locked, each with rationale, consequences, reopen criteria and owner; the user confirmed them explicitly in conversation on 2026-10-12. D-004, D-005, D-012 to D-016 remain assumptions. No undecided item blocks architecture.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — Architecture gate
- **Phase:** 5 Architecture
- **Result:** pass
- **Evidence:**
  - Matches locked decisions: ARCHITECTURE.md "Alignment to locks" covers D-001, D-002, D-003, D-006 to D-011; no contradiction found on review.
  - Trust and data boundaries: ARCHITECTURE.md "Trust & boundaries".
  - Vertical slices with acceptance: ROADMAP.md S1 to S11.
  - Every MVP requirement maps to a slice and validation method: checked by script, 27 of 27 requirements present in the ROADMAP.md traceability table.
  - First slice ready: WORK_ITEMS.md W-001 to W-012.
  - Test strategy, migrations, rollback, secrets, config, observability: ARCHITECTURE.md "Test strategy" and "Operability".
- **Waiver:** none
- **Follow-up:** CI provider and repository hosting undecided (D-016, assumption); does not block S1.

### 2026-10-12 — Rust-Toolchain integration
- **Phase:** 6 Implement (S1, W-002)
- **Result:** pass
- **Evidence:** Integrated per `../Rust-Toolchain/RUST-TOOLCHAIN.md`: `justfile`, `deny.toml`, `.config/nextest.toml`, `rust-toolchain.toml` copied from the templates (none existed), project recipes added (`layers`, `msrv`, `bench-listing`, `run`), agent policy merged into AGENTS.md. `rust-tc check`, `rust-tc quick` and `rust-tc doctor` were run and pass; the last run printed "Rust-Toolchain: PASS" with 25 tests passed, `advisories ok, bans ok, licenses ok, sources ok`, cargo-shear "no issues found", feature checks on four crates, the layering check, and `cargo +1.95.0 check` of the workspace.
- **Project-specific exceptions (in `deny.toml`, each commented):** licences 0BSD, CC0-1.0, MPL-2.0 (in the UI toolkit's tree, GPLv3-compatible) and GPL-3.0-or-later (our crates) allowed; four "unmaintained" advisories ignored for crates that arrive only through the pinned toolkit (`instant`, `paste`, `rustybuzz`, `ttf-parser`); `allow-wildcard-paths` for unpublished workspace path dependencies; `gpui-kit` banned outside `rustphin-ui`.
- **Waiver:** none
- **Follow-up:** re-check the advisory ignores at every toolkit upgrade.

### 2026-10-12 — Slice S1 checks
- **Phase:** 6 Implement (S1)
- **Result:** pass for everything automated; two items open (below)
- **Evidence by S1 acceptance point:**
  - *Builds with one command on Rust 1.95 and stable; checks pass.* `rust-tc doctor` passes, including `cargo +1.95.0 check --workspace --all-targets --all-features`. Release build with stable 1.98.1 succeeded.
  - *Theme with blur, and opaque mode without.* The blur request and fallback are implemented and unit-tested for the non-Wayland and disabled cases. In the isolated KWin session (no blur protocol) the app logged the fallback and drew the opaque theme (observed in screenshots). **Blur on the real desktop with this build is not yet observed.** Token contrast tests pass for the opaque theme and for the translucent theme over a white window (WCAG AA 4.5:1 for text, 3:1 for the accent).
  - *Entries listed by name, natural order, folders first; hidden toggle.* Unit tests in `rustphin-core`; observed in the UI: folders before files, `file2.txt` before `file10.txt`, Ctrl+H showing `.config` and `.hidden-file`, status line "11 items, 2 hidden" → "13 items".
  - *Enter a folder, up, back, forward; type a path; inline errors.* Unit tests for history and failed opens. Observed in the UI: Enter into a folder; Alt+Up returning with the folder just left selected; double-click on an unreadable folder and a typed non-existent path each showing an inline error while the previous folder stays shown and the location bar still names it; empty folder and only-hidden-items states. Back and forward were exercised by unit tests and the toolbar state, not by a UI click.
  - *500,000 entries: first batch within 200 ms, sorted snapshot within 500 ms; UI accepts input throughout.* `rust-tc bench-listing` through the real model on XFS, warm cache, five runs: first entries 2.3 ms worst, complete sorted listing 144.9 ms worst — PASS. In the UI (release build): the folder showed "500,000 items" and End jumped to the last entry within the 600 ms screenshot delay. Input responsiveness *during* loading and frame times were not measured (W-013).
  - *Non-UTF-8 and control characters display safely.* Unit tests; observed in the UI as `bad�␊name.txt` on one row.
- **Test quality (testing-suite skill):** 25 tests. Five mutations were applied by hand to the core (folders-first reversed, digit runs compared as text, history not restored after a failed open, superseded results still published, control characters shown raw). Four were caught at once; the fourth survived, so a deterministic test was added and then failed under that mutation as intended. The same exercise exposed a real ordering race (the completed listing was recorded after its final snapshot was published), fixed by recording first; the suite then passed 25 consecutive runs.
- **Defects found by UI testing and fixed:** (1) an error banner was cleared by the next re-sort and a failed open dropped the selection; (2) list shortcuts fired while typing in the location field — Enter also activated the selected row, which made a failed typed path revert to the wrong folder, and Backspace would navigate up; fixed with a `Browser && !Input` key context; (3) End with nothing selected chose the first row; fixed, with selection movement extracted into a unit-tested function. Fix (3) is covered by unit tests but was not re-run in the UI.
- **Not verified:** blur on the real desktop with this build; frame times; back/forward by mouse; behaviour on Btrfs or with a cold cache; the UI scenarios exist as a hand-run sequence, not a replayable script.
- **Waiver:** none
- **Follow-up:** W-012 (user's visual review), W-013 (frame timing, in S2).

### 2026-10-12 — S1 visual review on the real desktop (W-012)
- **Phase:** 6 Implement (S1)
- **Result:** pass after one fix
- **Evidence:**
  - First review, by the user (screenshots): the default and `RUSTPHIN_OPAQUE=1` runs looked almost identical; no transparency. Cause, found in the pinned source: `gpui-component`'s root plugin paints `theme.tokens.background` (opaque) under the application view.
  - Fix: the window is opened with `gpui_kit::base::Root` whose background is cleared, and the window colour went from 84% to 72% opacity. Secondary text was brightened (`#aab2c5` → `#c3cad9`) so the contrast tests still pass over a white window. `rust-tc doctor` passed (25 tests).
  - Second review, by the user (screenshot): the window behind is visible and blurred through the list area; toolbar and status line read slightly darker. The opaque mode was seen solid in the first review.
- **Not verified:** blur while resizing; appearance over a bright window (covered by the contrast tests only).
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — Implement gate, slice S1
- **Phase:** 6 Implement
- **Result:** pass
- **Evidence:** every S1 acceptance point in ROADMAP.md is demonstrated in the two entries above and "Slice S1 checks". Requirements advanced: R-001 (navigation part), R-101 (first entries and sorted listing; input during loading and frame rate move to S2 as W-013), R-104 (names), R-105 (window, blur, fallback, contrast tests), R-109.
- **Waiver:** none
- **Follow-up:** W-013 in S2; the UI scenarios should become a replayable script.

### 2026-10-12 — W-014, W-015: details and sorting in the core
- **Phase:** 6 Implement (S2)
- **Result:** pass, with one measured shortfall recorded below
- **Evidence:**
  - `rust-tc doctor`: PASS, 36 tests.
  - `rust-tc bench-listing`, 500,000 files, XFS, inode cache warm: load sizes and dates 153 ms; re-sort by size 52 ms, by modified 37 ms, by type 60 ms, by name descending 47 ms; first entries 3.3 ms; complete sorted listing 173 ms. All within R-101's limits.
  - Same benchmark immediately after the folder had only been listed, never stat-ed (inode cache cold): the first load of sizes and dates took **1,748 ms**, over R-101's 500 ms for a re-sort. A second run took 162 ms. During the load the model has already published the entries in name order with the loading flag set, so the view is populated and responsive; only the sorted result is late.
  - Mutation checks on the new tests: six breakages applied by hand (descending ignored; folders ordered by their own size; hidden files given an extension; size sort not loading details; unreadable details reported as zero; listing id changing on every re-sort). Five were caught at once. The second survived because both sample folders had the same size; the sample was changed and the mutation then failed two tests.
- **Deviations from the plan:** details load for all entries at once, not visible rows first (ARCHITECTURE.md); sorting by type uses the file extension, not the MIME description Dolphin uses.
- **Not verified:** Btrfs, network or FUSE mounts; behaviour when files change during the load.
- **Waiver:** none
- **Follow-up:** W-023 (progressive, visible-first details). The benchmark now reports the details load without a limit, because the program cannot control the inode cache; the re-sorts after it are gated.

### 2026-10-12 — W-016: selection model in the core
- **Phase:** 6 Implement (S2)
- **Result:** pass
- **Evidence:** `rust-tc doctor` PASS, 45 tests. Nine behaviour tests through the public `Selection` and model API. Five hand-applied mutations (filtered-out entries staying selected; invert doing nothing; a range missing its last row; toggle never deselecting; the selection surviving a re-read) each failed at least one test.
- **Not verified:** use from the UI (W-020); rubber-band selection is a UI concern not covered here.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — W-017: file types and icon lookup in the platform layer
- **Phase:** 6 Implement (S2)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 61 tests (18 in `rustphin-platform`), all against fixture databases and themes in temporary directories.
  - Run against the real system with the `icon_lookup` example: theme `DevTuxIcon-Devicons` read from the KDE settings; a folder, `.txt`, `.JPG`, `.tar.gz`, `.rs`, `.pdf`, `Makefile`, `.mp3` and an unknown name each resolved to the expected type, description ("Plain text document", "Rust source code", ...) and an icon file found through the theme's inheritance chain.
  - Mutation checks: six breakages by hand (lighter glob winning; only the longest suffix tried; hi-dpi directories used at scale 1; size fit ignored; unsafe theme names accepted; directory sections leaving the theme). Five caught at once; the second survived, a test was added (`report.final.v2.txt`), and it then failed under the mutation.
- **Security notes:** theme names from configuration and directory names from `index.theme` are rejected if they could leave the icons folder; MIME types are validated before being used as a path for descriptions. Each has a test.
- **Not verified / limits:** types come from file names only (no content sniffing, so extensionless files are "Unknown"); glob patterns other than `*.suffix` and whole names are skipped (53 of 1,621 on this system); hi-dpi (scale 2) icon directories are not used; rendering of the found icons in the UI (W-018).
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — W-018: details view
- **Phase:** 6 Implement (S2)
- **Result:** pass in the isolated session; real-desktop review pending
- **Evidence:**
  - `rust-tc doctor` PASS, 68 tests. New unit tests cover size and date formatting, counts, the column breakpoints, and token contrast on the new list surface (text and secondary text at WCAG AA over a white window).
  - Observed in an isolated KWin session (isolated home, so the default `breeze-dark` theme): columns Name, Size, Modified, Type; folders without a size; sizes as `42 B`, `1.5 KiB`, `120.6 KiB`, `5.0 MiB`; dates as set on the test files; type names from the MIME database ("Rust source code", "Tar archive (gzip-compress…" truncated with an ellipsis); a distinct theme icon per type; hover highlight on a row.
  - Clicking the Size heading twice gave descending order by size with folders still first and a down arrow on the heading.
- **Design decisions applied (design skills):** all values from tokens; row icons 20 px to match the 20 px text line; headings and secondary cells in the secondary text colour, the active heading in primary plus an arrow (never colour alone); numeric column right-aligned.
- **Not verified:** the user's own icon theme and blur in this view on the real desktop; the column-dropping breakpoints visually (unit-tested only); sorting by Modified and Type by click (covered in core tests); dates use the system time zone, not exercised beyond UTC and a fixed offset in tests; the icon-missing-when-unfocused oddity seen in the Discovery spike was not looked for.
- **Deviation:** date and size formats are fixed (ISO-style date, binary units), not locale-aware.
- **Waiver:** none
- **Follow-up:** user review of the details view on the real desktop before the icons and compact views (W-019) are built.

### 2026-10-12 — User review of the details view (W-018)
- **Phase:** 6 Implement (S2)
- **Result:** pass
- **Evidence:** the user ran the release build on the real desktop and reported "everything works great". No specifics were given on the icon theme, blur or column dropping, so those are covered only by that general statement.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — W-020: multi-selection and type-ahead in the UI
- **Phase:** 6 Implement (S2)
- **Result:** pass for what was exercised
- **Evidence:**
  - `rust-tc doctor` PASS, 71 tests. New tests: type-ahead prefix search in the core (match, case, wrap-around, absent and empty prefixes) and the type-ahead query rule in the UI (one repeated letter cycles, a longer prefix narrows).
  - Observed in an isolated KWin session: a click then two Ctrl+clicks left three separate rows selected with "3 of 11 items selected" in the status line; typing `p` with the focus on the last row wrapped to "Pictures"; Shift+Down twice then selected Pictures, Projects and backup.tar.gz.
- **Not verified:** Shift+click was performed but its screenshot was overwritten before being viewed; Ctrl+A, invert, Escape, and Shift+Home/End/Page keys are wired but were not exercised in the UI (their logic is covered by the core `Selection` tests); behaviour with 500,000 rows selected.
- **Deviation:** rubber-band selection is not built; split into W-024.
- **Waiver:** none
- **Follow-up:** W-024; W-022 should script these interactions.

### 2026-10-12 — W-019: icons and compact views, zoom
- **Phase:** 6 Implement (S2)
- **Result:** pass for what was exercised
- **Evidence:**
  - `rust-tc doctor` PASS, 78 tests. New unit tests cover the geometry of each mode at each zoom level (sizes on the 4-pixel grid, zooming never shrinking anything, whole cells per row, at least one column, zoom steps clamped) and the icon size requested from the theme.
  - Observed in an isolated KWin session: Ctrl+1 showed a grid of large theme icons with names underneath, and Down then Right moved the selection to the second cell; Ctrl+2 showed the compact grid with small icons beside names; Ctrl+= enlarged the compact icons and widened the cells from four to three per row; the three view-mode buttons show their icons with the active one highlighted.
- **Defect found and fixed:** the view-mode buttons were blank because the default icon bundle lacks those icons; the application now embeds the complete icon set.
- **Deviations:** the compact view fills rows left to right and scrolls vertically, where Dolphin's fills columns and scrolls horizontally; embedding the complete icon set makes the binary larger (not measured).
- **Not verified:** zoom out and reset; zoom in the icons and details modes; clicking the view-mode buttons (keyboard shortcuts were used); very large folders in the grid modes; names longer than two lines in the icons view; view mode and zoom are not yet remembered (W-021).
- **Waiver:** none
- **Follow-up:** W-021, W-013 (frame times should include the grid modes).

### 2026-10-12 — W-021: remembered view settings
- **Phase:** 6 Implement (S2)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 91 tests. New tests: atomic write in the platform layer (creates folders, replaces the file, leaves no temporary file, a failed write keeps the old file) and the settings format in the UI (round trip, readable key names, defaults for an empty file, missing and unknown keys, damaged files and wrong values, zoom clamping, newer-version detection).
  - Observed in an isolated KWin session with an isolated home: after Ctrl+1, Ctrl+= and Ctrl+H the file `.config/rustphin/settings.toml` held `mode = "icons"`, `zoom = 2`, `show_hidden = true`; after closing and relaunching, the window opened in the icons view at that zoom with `.hidden` shown.
- **Not verified:** restoring the sort column and direction in the UI (covered by the format tests only); the "do not overwrite a newer file" path in the running app (its detection is unit-tested); behaviour when the config folder is not writable (a warning is logged; not exercised).
- **Deviation:** settings are global, not per folder as Dolphin can do.
- **Waiver:** none
- **Follow-up:** none

### 2026-10-12 — W-022: UI scenarios written and run
- **Phase:** 6 Implement (S2)
- **Result:** pass for the scenarios run
- **Evidence:** in an isolated KWin session with an isolated home, against the fixture from `tests/ui/make-fixture.sh`, observed by screenshot:
  - S2: click then Shift+click selected rows 1–4, "4 of 17 items selected".
  - S3: Ctrl+Shift+A selected the other 13; the focused unselected row kept its accent bar.
  - S4: Escape cleared the selection.
  - O2: clicking Modified ordered files 2023, 2024, 2025, then today's, with an up arrow on the heading.
  - O3 and V1/V3: clicking Type, then the icons view button, then Ctrl+- showed the smallest icon grid ordered Makefile, mystery, backup.tar.gz, photo.JPG, video.mp4, report.pdf, main.rs, then the text files.
  - P1/P2: after Ctrl+0 the settings file held `mode = "icons"`, `zoom = 1`, `sort = "type"`; after a restart, clicking the details button showed the list still sorted by Type with its arrow.
  - Earlier today, recorded under their work items: N1–N7, S1, S5, S6, O1, V2, L1.
- **Not run this time:** S7 (Ctrl+A) on its own; V4 by keyboard.
- **Findings:**
  - The application does not appear in `list_windows` and exposes nothing to `find_ui_elements` in these sessions, so no accessibility tree is reachable over AT-SPI. This matters for R-107 (accessibility baseline) and is carried to S11.
  - On selected rows, the theme's small monochrome icons are hard to see against the selection fill. Cosmetic; depends on the icon theme.
- **Limits:** the scenarios are instructions an agent replays with screenshots and coordinates, not an unattended test; coordinates depend on the window layout.
- **Waiver:** none
- **Follow-up:** accessibility exposure (S11).

### 2026-10-12 — W-013: frame timing and memory on 500,000 entries
- **Phase:** 6 Implement (S2)
- **Result:** pass in the test environment (see limits)
- **Method:** release build with `RUSTPHIN_SCROLL_BENCH=1`, which scrolls the list three rows every frame for 1,200 frames and logs, per 120 frames, the interval between frames (which includes building rows, layout and painting). Run in an isolated KWin session at 1280×800 on the reference machine (Intel Arc B580, Vulkan). Memory is resident set size from `ps`.
- **Evidence:**
  - Details view: median frame interval 16.6–16.7 ms in all ten windows; 95th percentile 17.2–17.6 ms; maximum 17.5–19.5 ms in nine windows. The first window had one 89.7 ms gap while the folder was still loading. No frame took two display periods after loading.
  - Icons view: median 16.6–16.7 ms, 95th percentile 17.3–17.5 ms, maximum 17.4–17.6 ms after one 80.5 ms gap in the first window.
  - The view's own description took at most 0.5 ms per frame.
  - Memory: 95 MB with a 17-entry folder open; 137 MB with the 500,000-entry folder in the details view (sizes and dates loaded); 108 MB in the icons view; 126 MB after leaving the large folder.
- **Against the requirements:** R-101 "scrolling holds 60 frames per second at the 95th percentile": the display runs at 60 Hz (16.7 ms) and every frame after loading arrived within one period plus about 1 ms of jitter, so no frames were dropped; a strict reading of "95th-percentile interval ≤ 16.7 ms" is missed by under a millisecond of timer jitter. R-102: 137 MB is under 400 MB, and leaving the folder returned to 31 MB above the small-folder figure, within the 50 MB allowed.
- **Limits:** a virtual compositor output, not the real display, and no blur; the compact view was not measured; real wheel or touchpad scrolling was not measured (the test tool's smooth scrolling is not picked up by the app and its wheel scroll sends a single event); R-101's "no UI-thread task over 16 ms" is supported by the absence of long frames after loading but not measured per task; the 80–90 ms gap at start-up is unexplained beyond coinciding with loading.
- **Other observation:** the release binary is 309 MB with line tables and 43 MB stripped.
- **Waiver:** none
- **Follow-up:** look at the start-up gap when working on W-023; measure on the live desktop before the release gate.

### 2026-10-12 — W-023: progressive details loading
- **Phase:** 6 Implement (S2)
- **Result:** pass; the cold-cache sort itself is still over the R-101 limit
- **Evidence:**
  - `rust-tc doctor` PASS, 100 tests. New tests: details arrive for the top of the list before the rest; rows keep name order until a size sort has every size; hidden entries stay hidden while details load.
  - Mutation checks: four breakages by hand. "Partial results sorted by half-known sizes" and "no partial results at all" were caught. "Hidden entries shown while loading" survived, a test was added, and it then failed under the mutation. "Only one partial result published" still survives: it changes how often partial results appear, not what the user ends up seeing, and no test pins the cadence.
  - `rust-tc bench-listing`, 500,000 files. With a cold inode cache (first run after the machine had been idle): sizes and dates for the top of the list after **207 ms**, for all entries after **5,314 ms**. Before this change nothing was shown until the full load finished (1,748 ms in the earlier cold run). With a warm cache on an otherwise idle machine: top after 74 ms, all after 252–322 ms (153 ms before this change: publishing partial results costs about 100–170 ms in total).
  - Observed in the application (release build, isolated session, 500,000-entry folder): sizes, dates and types shown, and clicking Size put its arrow on the heading with the list still populated.
- **Measurement conditions:** the cold run and the three runs after it were taken while a game was using the machine (load average 19); under that load the benchmark still passed its limits (sorted listing 305–394 ms, slowest re-sort 141–194 ms).
- **Still open against R-101:** the first sort by size or date in a very large folder with a cold cache takes as long as reading every entry's details (seconds), over the 500 ms limit. The list stays populated in name order meanwhile. This is bounded by disk reads, not by sorting.
- **Deviation:** "visible rows first" is approximated by reading in display order from the top; the view does not tell the core where the user has scrolled to.
- **Not verified:** how the list looks while details arrive on a slow filesystem (rows fill in top-down); network and FUSE mounts.
- **Waiver:** none
- **Follow-up:** decide with the user whether R-101's re-sort limit should apply only once details are loaded.

### 2026-10-12 — W-024: rubber-band selection
- **Phase:** 6 Implement (S2)
- **Result:** pass for what was exercised
- **Evidence:**
  - `rust-tc doctor` PASS, 105 tests. New tests: which rows a band touches in a list and in a grid, clipping to the entries that exist, bands above or left of the entries, and `Selection::select_rows` replacing or adding.
  - Observed in an isolated KWin session against the UI fixture: in the details view, pressing at one row and dragging down five rows drew an outlined band and selected the six rows it crossed ("6 of 17 items selected"); the selection was still there after releasing the button and after switching to the icons view. In the icons view a band across the second to fourth cells of the first line selected exactly those three.
- **Known interim behaviour:** a drag anywhere in the list starts a band, including from on top of an entry. When drag-and-drop arrives in S5, dragging from an entry will have to start a drag instead.
- **Not verified:** Ctrl+drag adding to a selection (unit-tested in the core only); bands in the compact view; a band while the list is scrolled; the list does not scroll when the pointer reaches its edge.
- **Waiver:** none
- **Follow-up:** S5 must split "drag from an entry" from "drag from empty space"; edge auto-scroll.

### 2026-10-12 — Design document reviewed, refined and applied
- **Phase:** 6 Implement (S2); change-impact loop on D-007
- **Result:** pass
- **Evidence:**
  - `docs/DESIGN.md` as received (commit `e4db833`) was reviewed against the brief, D-007, the toolkit and the running app. Findings: its token block and its prose gave different values for the same colours; it was titled for a different product and described features Rustphin does not have; its accent was indigo where the user had asked for royal blue; it required square corners, monospace for all text, and backdrop blur on in-window surfaces, which GPUI cannot do.
  - The user decided: royal-blue accent, 2-pixel corners, monospace for data only. D-007 is superseded by D-017, confirmed by the user.
  - The document was rewritten in the Google Labs DESIGN.md format. `npx @google/design.md lint docs/DESIGN.md`: 0 errors, 7 warnings, all "token not referenced by a component" for colours that components use through borders or states the format has no property for.
  - Applied to the app: radius 6 → 2 px, base colour `#0B0D12` → `#0B0F17`, primary text `#E6E9F0` → `#DFE2EF`, Inter for names and interface text, JetBrains Mono for sizes, dates and the status line, Space Grotesk for state headings. Token contrast tests pass with the new values.
  - Observed in an isolated session: proportional names, monospace sizes, dates and status line, and near-square corners on the selected row.
- **Not verified:** the new look with blur on the live desktop; Space Grotesk in an empty-state heading (not opened in this run); behaviour when one of the three fonts is missing.
- **Waiver:** none
- **Follow-up:** decide whether to bundle the fonts; apply floating-surface styling when menus and the palette exist.

### 2026-10-12 — Implement gate, slice S2
- **Phase:** 6 Implement
- **Result:** pass
- **Evidence:** `rust-tc doctor` PASS with 105 tests at the last run before this entry. Per requirement:
  - R-003 (view modes, sorting, zoom): three modes, sortable columns with natural folders-first order, zoom in all modes, settings remembered — W-015, W-018, W-019, W-021 and scenarios O1–O3, V1–V4, P1–P2.
  - R-004 (selection and keyboard): click, Ctrl+click, Shift+click, rubber band, select all, invert, arrows, Home/End, Page keys, type-ahead — W-016, W-020, W-024 and scenarios S1–S8. "Every action has a keyboard path" holds for what exists so far; "shortcuts match Dolphin's" was not audited key by key.
  - R-014 (icons part): icons from the system theme by MIME type — W-017, W-018.
  - R-101: first entries 3 ms, sorted listing 155–175 ms, re-sorts 37–110 ms on an idle machine; one frame per 60 Hz period while scrolling after loading; reworded today for the first details read (see REQUIREMENTS.md).
  - R-102: 137 MB with 500,000 entries; 31 MB above the small-folder figure after leaving.
- **Carried forward, not blocking:** the 80–90 ms frame gap at start-up; nothing exposed over AT-SPI (R-107, S11); frame timing only measured in a virtual session; per-folder view settings and Dolphin's column-flow compact view are not implemented; the shortcut audit against Dolphin.
- **Waiver:** none
- **Follow-up:** S3.

### 2026-10-12 — W-025, W-026: live updates
- **Phase:** 6 Implement (S3)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 115 tests. New core tests, against real directories with real inotify unless noted: a created file appears without a reload; removed and renamed entries follow; re-reads keep the view options and do not blank sizes and dates; a vanished directory gives way to its parent with a `Moved` event; a change in a directory already left is ignored; the directory entered later is the one watched; re-reading a large directory publishes only the finished result (fake filesystem); the selection survives a live update by name, and a re-sort unchanged.
  - Mutation checks: eight breakages by hand. Five were caught at once. Three survived (partial results during a large re-read; details blanking during a re-read; the first directory staying watched), tests were added, and all three then failed.
  - Observed in the application (isolated session): with `file10.txt` and `notes.txt` selected, creating `aaa-new.txt`, deleting `notes.txt` and renaming `main.rs` from a shell showed all three changes within the one-second wait, kept `file10.txt` selected ("1 of 17 items selected") and did not scroll. Deleting the open folder showed its parent with the message "… no longer exists. Showing the nearest folder that does."
- **Against R-002:** changes appear within 1 second (100 ms coalescing delay plus the re-read); selection and scroll survive unless the selected entry was removed.
- **Deviations and limits:** a change triggers a full, quiet re-read of the directory rather than patching the listing, so a 500,000-entry folder under constant change costs a re-read each time; consecutive re-reads are spaced by three times the last one's duration. Entries are matched across a re-read by name, so a renamed selected file loses its selection. Not measured: the delay on a very large folder; inotify watch limits; network and FUSE mounts, where inotify does not report remote changes.
- **Waiver:** none
- **Follow-up:** incremental updates if large busy folders prove slow; W-030 adds these checks to the UI scenarios.

### 2026-10-12 — W-027 to W-030: tabs, split view, session restore
- **Phase:** 6 Implement (S3)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 131 tests. New unit tests: the tab list (open beside the active tab, close with neighbour activation, the only tab cannot be closed, cycling wraps, moving keeps the same tab active, out-of-range indices) and the session file (round trip, missing or damaged file, unusable entries dropped and indices clamped, a file from a newer version recognised).
  - Observed in the application (isolated session with its own home, scenarios T1–T5, D1–D3, R1): middle-click on a folder opened a background tab and showed the tab strip; Ctrl+Tab switched tabs and the window title followed; Ctrl+Shift+PageUp moved the active tab to the first position; Ctrl+T opened and Ctrl+W closed tabs, and the strip disappeared at one tab; F3 split the tab; Tab moved the accent line and keyboard input to the other pane; `session.toml` listed two tabs with the split tab's two panes; after killing and relaunching with no argument the same tabs, split and locations came back.
  - Found and fixed while testing: new panes stole keyboard focus on creation; narrow panes hid the Name column; the pane divider and the gap above the inactive pane showed the desktop through the window, because each pane painted its own background (the workspace now paints it once).
- **Not verified:** T6 (clicking a tab and its close button) after the last fixes; reordering by mouse drag; two panes holding different view modes (each pane has its own settings in code, not observed); restoring a session whose folder no longer exists, beyond the core's fallback to the nearest ancestor; the look with blur on the live desktop.
- **Waiver:** none
- **Follow-up:** none blocking.

### 2026-10-12 — Implement gate, slice S3
- **Phase:** 6 Implement
- **Result:** pass
- **Evidence:** `rust-tc doctor` PASS with 131 tests. Per requirement:
  - R-002 (live updates): W-025, W-026 and scenarios U1, U2.
  - R-005 (tabs): W-027, W-029 and scenarios T1–T5, R1.
  - R-006 (split view): W-028 and scenarios D1–D3. Its second point, copy and move between panes, is not part of S3 and stays open until S4/S5.
- **Carried forward, not blocking:** live updates re-read the whole directory; tab reordering by mouse drag and T6 unobserved; view settings are saved globally, not per pane or folder; nothing exposed over AT-SPI (R-107, S11); the 80–90 ms start-up frame gap.
- **Waiver:** none
- **Follow-up:** S4.

### 2026-10-12 — W-031, W-032: filesystem primitives and the operation engine
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 170 tests (39 new, all against real temporary directories; no fake filesystem).
  - Behaviour: create, rename, copy of nested folders with links (kept as links, dangling included) and names that are not UTF-8, permissions and modification time kept, numbered name for a copy beside its source, every conflict choice including apply to all and folder merge, refusal of impossible requests before any change, cancel between entries and in the middle of a large file, skip and retry of a failing file.
  - Fault sweeps: the same copy, same-filesystem move and cross-filesystem move are run once per changing filesystem call, with that call (a) failing, (b) failing together with every later call, as when the process dies, (c) for copies, writing half the file and reporting success. After each run: sources of a copy unchanged; nothing partial under a final name; a file being replaced is wholly old or wholly new; for a move every source entry is intact at the source or whole at the destination; after a plain error no temporary entry is left.
  - Kill test: a child process runs the cross-filesystem move and is sent SIGKILL at each step in turn (13 or more steps); the same invariants hold after each.
  - Mutation checks: 14 breakages by hand (copy straight to the final name, always replace, no clean-up, no size check, partial file kept, source removed although changed or not copied, silent replace on rename, conflicts not asked, file over folder, cancel ignored, into-itself check dropped, skip not honoured). 11 were caught at once; 3 survived, tests were added, and all 3 then failed.
- **Not verified:** the reflink path (the tests run on tmpfs, which has none; the project disk is XFS); the fallback for filesystems without no-replace rename; a real second filesystem (the cross-filesystem case is simulated by the test wrapper refusing renames between two folders); behaviour on power loss (files are not synced to disk; killing the process is covered, losing power is not); very deep trees (the scan recurses).
- **Deviations:** a folder is copied into a temporary folder and renamed as a whole, so a folder appears only when complete; if the user skips a failing file inside it, the folder is put in place without that file and the skip is reported. Devices, pipes and sockets are not copied.
- **Waiver:** none
- **Follow-up:** W-033 trash, W-034 undo, W-036 property tests and the gate record.

### 2026-10-12 — W-033: trash and permanent delete
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 189 tests. New tests, all on real temporary directories: the `.trashinfo` record is byte-for-byte what the specification asks for (absolute path in the home trash, path relative to the volume's top in a volume trash, percent-encoding including bytes outside UTF-8, local-time date); trash directories are created private (0700); equal names get distinct trash names and restore to their own places; restore refuses to replace what took the place; orphaned content in the trash is not replaced; the shared `.Trash` folder is used only with the sticky bit; an entry stays when its volume cannot hold a trash; the trash, its parents and its contents cannot be trashed; records in the styles Dolphin and GLib write are read; trashing with the process dying at each step leaves the entry either where it was or whole in the trash.
  - Engine: trash and delete requests, with skip, retry and cancel; delete removes a tree but never what a link points to; a file that cannot be deleted keeps its folders and is reported once.
  - Mutation checks: 16 breakages by hand across the trash and the engine's delete path; 13 caught at once, 3 survived (trash replacing orphaned content, a record left after a failed move, the trash-contains-itself check), tests were added, and all 3 then failed.
- **Not verified:** interoperability against a running Dolphin (W-040); a real second filesystem (volumes are simulated: the test filesystem reports a different device number for a folder); the `directorysizes` cache, which is optional in the specification and not written.
- **Decision within D-011:** when no trash can be used on an entry's own volume, trashing fails and the entry stays; it is never copied to the home trash across filesystems and never deleted instead.
- **Waiver:** none
- **Follow-up:** W-034 undo.

### 2026-10-12 — W-034, W-035: undo and the background runner
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 217 tests.
  - Undo: each kind of operation (create folder and file, rename, copy, move within and across filesystems, merge move, trash) is run and undone, and the source and destination trees are compared with the state before. Refusals are tested for a replaced file, a permanent delete, an edited copy, a taken old place, a missing entry, a created file that was written to, and a created folder that was filled. A refused undo is shown to touch none of the operation's entries; an undo that stops half-way reports exactly what is still in effect.
  - Property test: random sequences of up to 13 operations (create, rename, copy, move, trash, with colliding names, one not UTF-8, half the runs with a folder acting as another filesystem) are applied and then undone one by one; the tree must equal the starting tree. 200 cases per run in the suite; one run of 4,000 cases passed.
  - The property test found two real defects, both fixed before this entry: undo refused a created folder after a later operation inside it had been undone (its modification time had moved), and refused entries that an earlier undo had brought back across filesystems (new inode). Entries are now compared by kind, size, modification time and permissions for files, and by kind for folders.
  - Runner: start/progress/finish, a conflict that waits for its answer, cancel while a question is open, skip and retry of a failed entry, a refused request, undo twice in order, a refused undo that stays on offer. One test found that an operation waiting for an answer lived on after the runner was dropped; dropping the runner now cancels running operations.
  - Mutation checks: 17 breakages of undo by hand; 13 caught at once, 4 survived (redundant checks that only matter for multi-entry operations or races), tests were added, all 4 then failed.
- **Limits, by design:** a folder is not checked for changes before its undo, because undoing a folder never deletes (a created folder is removed only if empty, a moved one goes back with its contents, a copied one goes to the trash with whatever it holds). Undo of a copy needs a trash; without one it is refused. History is per session, 100 operations.
- **Waiver:** none
- **Follow-up:** none.

### 2026-10-12 — W-036: operation engine gate
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **What the gate requires (ROADMAP S4, ARCHITECTURE risk rule):** the engine passes its fault-injection and kill-test suite before any operation UI is enabled.
- **Evidence:** the entries of 2026-10-12 for W-031/W-032, W-033 and W-034/W-035 above. In short: a fault at every changing filesystem call of a copy, a move and a cross-filesystem move (failing once, dying there, or producing a short copy); a child process killed with SIGKILL at each step of a cross-filesystem move; trashing with the process dying at each step; the property test over operation sequences; 47 hand mutations in total, every one of which now fails at least one test.
- **Against R-103:** "no code path overwrites or permanently deletes without an explicit choice": replacing happens only on `ConflictChoice::Overwrite`, deleting only through `Request::Delete`; undo never deletes non-empty data. "Killing the process at any point during copy or move leaves every source file intact or fully present at its destination": shown by the kill test and the crash sweeps. Batch rename is not built yet (S9). Atomic configuration writes were shown in S2.
- **Not covered:** power loss (no fsync of copied data); real second filesystems and reflink (simulated or not reachable on the test machine's tmpfs); FUSE and network mounts; trees deep enough to exhaust the stack.
- **Waiver:** none
- **Follow-up:** W-037 onwards may start.

### 2026-10-12 — W-037, W-038: file operations in the interface
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 227 tests. New UI unit tests: the wording of progress and result lines (one entry named, several counted; quiet success for create and rename; problems name the first entry left out and why; a refused undo says what it was undoing), progress fraction and speed text, the free default name for a new entry, the part of a name preselected for renaming; contrast of text on dialogs and of the progress bar on its track.
  - Observed in an isolated session with its own home, on a scratch fixture (scenarios F1–F9): F10 created "New Folder" selected with its name in an edit field, and typing a name plus Return renamed it; F2 opened the field with the stem selected; Delete moved a file to the isolated home's trash with a message and an Undo button, and Ctrl+Z brought it back; Shift+Delete opened the confirmation dialog and Escape kept the files; copy and two pastes produced the conflict dialog showing both sizes and dates, and `K` kept both; cut, paste and undo moved a file into a folder and back; copying a folder of 30,000 files showed the progress line with bar, count and stop button; Dolphin, started in the same session, listed the trashed file with its original path.
  - Found and fixed while testing: after a rename the keyboard could stay in the hidden name field; the default button was white instead of the accent colour; the progress line read "0 of 0" while a question was open (the engine now reports once when its plan is ready); the rename field was unreadable on a selected row.
- **Not verified:** the failure dialog on screen (its logic is covered by the runner's tests); dialog buttons by mouse; rename in the icons and compact views; new file (Ctrl+Alt+N); stopping an operation with the stop button; speed display (the test files were empty); copy between two panes of a split (same code path as between folders); the look with blur on the live desktop.
- **Limits:** copy and paste are internal to Rustphin until S5 brings the system clipboard; cut entries are not shown dimmed; a name typed in the rename field replaces a name that was not valid UTF-8 with what is typed.
- **Waiver:** none
- **Follow-up:** W-039 trash view, W-040.

### 2026-10-12 — W-039, W-040: trash view and Dolphin interoperability
- **Phase:** 6 Implement (S4)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 232 tests. New tests: restore through the engine, with an occupied place skipped and reported, and its undo (trash again); mount points read from `mountinfo` with escapes undone; selected trash contents mapped to their records, orphans counted; everything an emptying removes; which folder counts as a trash view; naming of volume trashes; an irreversible operation does not block undoing the one before it.
  - Observed in an isolated session with its own home (scenarios X1–X4, F5 completed): two files trashed in Rustphin and one trashed with `kioclient move … trash:/` (KDE's own trash implementation) were all listed in Rustphin's trash view; Restore put the KIO-trashed file back in the fixture; "Empty trash…" asked, and after Return `Trash/files` and `Trash/info` were empty on disk; Shift+Delete, Return deleted a file and the message offered no Undo. Earlier the same day Dolphin, started in the session, listed a file trashed by Rustphin with its original path. So trash entries are exchanged in both directions.
  - Found and fixed while testing: "Emptied the trash." offered Undo, and an irreversible operation stayed in the history and blocked every earlier undo; such operations are no longer remembered.
- **Not verified:** opening another volume's trash (the banner listed the machine's two real volume trashes; they belong to the user and were not opened); restoring several entries at once on screen; the trash view when the home trash does not exist yet (it is created on opening, covered by `home_files_dir` only indirectly).
- **Limits:** the trash view shows stored names, not original paths or deletion dates; one trash directory at a time rather than a merged view; restore of an entry whose record is missing is refused with a message.
- **Waiver:** none
- **Follow-up:** none blocking.

### 2026-10-12 — Implement gate, slice S4
- **Phase:** 6 Implement
- **Result:** pass
- **Evidence:** `rust-tc doctor` PASS with 232 tests; the engine gate (W-036) was passed before any operation UI was built. Per requirement:
  - R-007 (file operations): create, rename, copy, move, trash, delete on single and multiple entries, nested folders, links and names that are not UTF-8 (engine tests); background execution with progress, count, speed and stop (W-035, W-038, scenario F8); conflict choices with apply to all (tests, scenario F6); no partial file under a final name and verified cross-filesystem moves (fault sweeps, kill test); trash by default and confirmed permanent delete (F3, F5); reflink then `copy_file_range` then read/write (implemented; the reflink path could not run on the test filesystem).
  - R-008 (undo): rename, move, copy, create, trash and restore undone newest first; refusal with the entry and the reason when files changed (tests, scenarios F4, F7).
  - R-009 (trash): freedesktop specification with per-volume directories (tests, volumes simulated); exchange with Dolphin/KIO in both directions (F9, X2); trash view with restore and confirmed emptying (X1–X4).
  - R-103 (data safety): first two points shown by the engine gate for copy and move; batch rename comes in S9; atomic configuration writes were shown in S2.
- **Carried forward, not blocking:** see the open loops in STATE.md: screens not yet observed, no sync to disk, no partial undo of an operation that replaced something, internal clipboard until S5, cross-filesystem and reflink behaviour only simulated or unreachable in tests.
- **Waiver:** none
- **Follow-up:** S5.

### 2026-10-12 — W-041 to W-045: file clipboard and drag-and-drop
- **Phase:** 6 Implement (S5)
- **Result:** pass, with two points not verifiable in the test session
- **Evidence:**
  - `rust-tc doctor` PASS, 242 tests. New tests: `file://` URIs for names with spaces, non-ASCII text, `%`, `#`, a line break and bytes outside UTF-8, round trip; lists in the forms other applications send (host, short form, comments, bare line ends, lower-case escapes); URIs that are not local files skipped; the clipboard formats byte for byte (`text/uri-list`, KDE cut marker, GNOME copied-files, plain text); what is offered reads back the same through a KDE-style, a GNOME-style and a plain reader; a clipboard without files; the in-application fallback; the once-only notice; which dropped entries are left out.
  - Observed in an isolated session with Dolphin (scenarios C1–C3, G1–G6), using a file named `aaa été 日本.txt`: cut in Rustphin faded the row, and Ctrl+V in Dolphin moved the file; cut in Dolphin and Ctrl+V in Rustphin moved it back, content intact. A plain drag onto a folder asked "Move or copy", and Return moved; a Ctrl-drag copied without asking; a Shift-drag between the two panes of a split moved. A file dragged from Dolphin onto Rustphin was accepted and copied. A file dragged from Rustphin into Dolphin opened Dolphin's own drop menu, and "Copy Here" copied it with its name intact. A press in the Size column still drew a rubber band.
  - Found and fixed while testing: dropping crashed the application (a view was read while being updated); the default dialog button had no visible label once its colour was set, because the toolkit's components read a second set of tokens.
- **Not verifiable here:** dragging to an application that is not a KDE one (R-010, needs the live desktop); a compositor without a data-control protocol (R-011: the fallback and its notice are unit-tested only).
- **Not verified:** copy (as opposed to cut) across the two applications on screen, though it differs only in the marker, which is unit-tested; dropping on a tab; drags in the icons and compact views; drags between two Rustphin windows; several entries dragged at once.
- **Observation, not part of this slice:** in this session Rustphin's window was see-through without blur. Recorded as an open loop in STATE.md.
- **Waiver:** none
- **Follow-up:** the manual non-KDE drag when the user is at the desktop.

### 2026-10-12 — Implement gate, slice S5
- **Phase:** 6 Implement
- **Result:** pass with the two open points above
- **Evidence:** `rust-tc doctor` PASS with 242 tests. R-010: drag within and between panes with modifier or question (G1–G3), out to Dolphin (G5), in from Dolphin (G4), names preserved; the non-KDE target is open. R-011: cut both ways with Dolphin resulting in a move (C1–C3), cut entries faded, names preserved, fallback implemented and unit-tested.
- **Waiver:** none
- **Follow-up:** S6.

### 2026-10-12 — W-046 to W-048: places and place groups
- **Phase:** 6 Implement (S6)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 265 tests. New tests: the shared places file read with its KDE details (identifier, hidden, built-in, restricted to one application, icon, remote places, device separators ignored); each edit compared line by line with the original, showing that only the lines concerned change (add with a name and path needing escapes, rename, hide with and without an existing marker, remove, reorder up, down and to the end); unknown fields kept; unknown identifiers reported; text that is not a places file refused. Groups: round trip with order and collapsed state, damaged file, a place listed twice, assigning between groups and out, reordering, add/rename/reorder/delete without losing places, layout order, forgetting vanished places. Panel: which places are listed, row order with collapsed groups, icon mapping.
  - Mutation checks through `rust-tc quick`: 12 breakages by hand (titles not escaped, a line break left behind, a place added after the devices, a reused identifier, an unparseable result accepted, the hidden flag misread, a bookmark without identifier listed, a place in two groups, the ungrouped list inverted, vanished places kept, other applications' places shown, a collapsed group still listing its places); all 12 failed the gate. One of them (unparseable result accepted) may have failed at compile time rather than in a test.
  - Observed in an isolated session with its own home (scenarios K1–K7): the empty panel explained how to add a place; Ctrl+D twice wrote two bookmarks to a new `user-places.xbel`; a group was created, named and given a place by keyboard, and `place-groups.toml` recorded it; a place was renamed with F2; Dolphin, started afterwards, listed both places ungrouped under the new name and added its own built-in entries to the same file, which Rustphin's panel picked up without a restart; after killing and relaunching Rustphin the group was back; dragging a place onto the group heading moved it in; dragging a folder from the file list onto the panel added it; Delete on a place removed it; Left folded the group and Delete on its heading dissolved it, leaving its place listed.
- **Against R-012:** places from the shared file are shown; add, rename and remove in Rustphin were seen in Dolphin's panel or in the file Dolphin then read; Dolphin's additions appeared in Rustphin. Groups can be created, renamed, reordered, collapsed and deleted, and places moved in and out by keyboard and by drag. Collapsed state and order persist. A grouped place appears ungrouped in Dolphin and the file stayed valid for it. Deleting a group kept its places.
- **Not verified on screen:** hiding (H) and the "list hidden places" button; reordering with Alt+Up/Down; a rename or removal made in Dolphin showing up in Rustphin (only Dolphin's additions were seen); KDE's file dialogs; two groups.
- **Limits:** only local folders and the trash are listed (network and "recent" places stay in the file, untouched); place icons are bundled ones chosen from the theme icon's name, not the theme's own; the panel's width is fixed and its visibility is not remembered between sessions.
- **Waiver:** none
- **Follow-up:** W-049 devices.

### 2026-10-12 — W-049 to W-051: devices
- **Phase:** 6 Implement (S6)
- **Result:** pass, with real-hardware checks open
- **Evidence:**
  - `rust-tc doctor` PASS, 276 tests. The device client is tested against a mock UDisks2 service reached over a private connection (no bus daemon): volumes listed fixed first, without ignored volumes and disk images, with label, size, mount point and whether the drive is removable and can be powered off; mounting returns the mount point and the list then shows it; unmounting a busy volume fails with the service's reason and leaves it mounted; a dismissed password prompt is reported as missing permission; a removable drive is powered off and a fixed one is not; a volume appearing on the service is noticed and then listed. Also tested: free and total space of a mounted filesystem; which volumes the panel offers to unmount (never the system or home volume); naming of unlabelled volumes; the compact size format.
  - Mutation checks through `rust-tc quick`: 8 breakages of the device client by hand (ignored volumes listed, disk images listed, busy not recognised, trailing NUL kept in a mount point, removable flag lost, fixed drives offered for power-off, changes not reported, sort order); all 8 failed.
  - Observed in an isolated session (scenario V5): the panel's "Devices" section listed the machine's three mounted volumes with usage bars and "n GiB free of m GiB"; the root volume was named "System" and had no eject button; the other two had one. Nothing was clicked there.
- **Not verified:** mounting, unmounting, safe removal, the busy message and plug/unplug on a real drive (R-013): the test session shares the machine's device service and no spare removable drive is attached. These paths are covered by the mock-service tests only.
- **Limits:** the usage figures are read when the list changes, not continuously; encrypted volumes that need unlocking are not offered; optical and network volumes are not handled specially.
- **Waiver:** none
- **Follow-up:** the manual check with a USB drive (recorded in STATE.md).

### 2026-10-12 — Implement gate, slice S6
- **Phase:** 6 Implement
- **Result:** pass with the hardware checks above open
- **Evidence:** `rust-tc doctor` PASS with 276 tests. R-012: all five points demonstrated (entry of 2026-10-12 for W-046 to W-048). R-013: volumes appear with name and usage (V5); mount, unmount, safe removal, change notification and the busy reason are implemented and pass against a mock service, and are not yet tried on hardware. The shared places file stayed valid for Dolphin after Rustphin's edits (K4).
- **Waiver:** none
- **Follow-up:** S7.

### 2026-10-12 — W-052 to W-054: thumbnails
- **Phase:** 6 Implement (S7)
- **Result:** pass
- **Evidence:**
  - `rust-tc doctor` PASS, 291 tests. New tests: the cache file name equals the specification's own example (MD5 of the URI); a thumbnail fits its size class, keeps proportions and content, records the original's URI, modification time and dimensions, and is private (0600 in a 0700 folder); a small image is not scaled up; a cached thumbnail is reused, not rewritten, until the file's time changes, and each size class is separate; a thumbnail written by another application with only the required keys is used as it is; truncated, random, textual and empty files give no thumbnail, are marked as failed and are not tried again until they change; a PNG header claiming 60,000 × 60,000 pixels is refused without being decoded; the cache does not thumbnail itself; requests are answered on background threads, failures included; beyond 64 waiting requests the oldest are dropped and handed back. UI side: which types are thumbnailed, forgetting a folder's files, not re-asking for a known failure, bounded memory of answers.
  - Mutation checks through `rust-tc quick`: 10 breakages by hand; 9 failed at once, 1 survived (a known failure being asked for again), its test was strengthened, and it then failed.
  - Observed in an isolated session (scenario H1): in the icons view a wide, a tall and a square picture showed their content with proportions kept; a file named `broken.png` that is not an image kept its type icon and nothing crashed; the isolated home's cache held three thumbnails in `normal/` and one failure marker; a thumbnail's text chunks carried the file's URI, modification time and original size.
- **Against R-014 and R-104:** type icons from the theme (S2); thumbnails made off the UI thread and only for entries being drawn, with waiting work dropped on leaving a folder or when it piles up; read from and written to the shared cache in the specified format; damaged and oversized images are harmless.
- **Not verified:** a thumbnail made by Dolphin being picked up on screen, and Dolphin picking up ours (both follow from the shared name and format, which are tested); JPEG, GIF and WebP on screen; behaviour while scrolling thousands of pictures; very slow decodes (there is no time limit, only size limits).
- **Limits:** no thumbnails for video, PDF, SVG or RAW files; none in the details and compact views at their default icon sizes; a decode in progress is not interrupted when its entry scrolls away, only waiting ones are dropped.
- **Waiver:** none
- **Follow-up:** none blocking.

### 2026-10-12 — Implement gate, slice S7
- **Phase:** 6 Implement
- **Result:** pass
- **Evidence:** `rust-tc doctor` PASS with 291 tests; the entry above covers R-014's four points and the image part of R-104.
- **Waiver:** none
- **Follow-up:** S8.
