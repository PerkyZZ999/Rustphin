# Validation evidence

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
