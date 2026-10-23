# Validation evidence

### 2026-10-23 — W-074h expose operation, launch and batch dialogs
- **Phase:** 6 Implement (S11)
- **Result:** named modal dialogs, risk/operation explanations, conflict facts and batch preview/errors verified with cancellation. Native text/state/cache and Orca acceptance remain open.
- **Red:** committed W-074g visually displays the two-file rename and permanent-delete prompts, but both are absent as dialogs in AT-SPI. Separate native probes fail on missing Rename 2 items and missing named delete dialog.
- **Change:** roles/names/modal flags on existing operation/launch/batch surfaces, with descriptions from their actual question/mode. Operation facts have semantic text. Batch preview is a named list with positions/count and concise current/new-name rows; collision explanations are descriptions and readable note nodes. Existing action handlers, operation policy, focus handles, layout and tokens remain. Conflict description refers to available actions without suggesting a disabled replacement choice. No dependency/toolkit/unsafe/locked-decision change.
- **Checks:** `rust-tc check` PASS, diagnostic release builds PASS. Full `rust-tc doctor` PASS, including after final description/row-name review (407 passed, zero skipped; doctests, dependencies/licences/advisories, feature checks, layers and Rust 1.95). Final `rust-tc build-diagnostics` PASS after the gate. `git diff --check` PASS; surrounding source/diff reviewed. Graph search/trace/snippet and best-effort coverage used for workspace.rs/rename_dialog.rs; native public API probes supply behavior proof instead of builder-mirroring unit tests.
- **Isolated UI:** private KWin/home/bus, 1280x800. Newly generated source holds a.txt, b.txt and harmless executable probe.sh; destination holds a different a.txt. No user files/devices used. Batch dialog is named/modal with focused fields and mode/validation description. Find a, replace b exposes two preview rows with collision explanations and no Rename action; replace c clears collision and exposes valid a-to-c preview plus a Rename action. Rename is never activated; native Cancel closes with original names/bytes. Screenshot confirms preview and explanations.
- **Confirmations:** Shift+Delete on generated a.txt exposes focused/modal Delete a.txt for good with the irreversible/no-trash warning. Ctrl+Shift+P leaves that dialog focused without opening the palette. Native Keep cancels. Focusing/activating generated probe.sh reaches the named focused Run confirmation and execution explanation; native Keep cancels and its marker remains absent. No Run/Delete confirmation accepted.
- **Conflict:** copy generated a.txt with Ctrl+C, switch to the generated destination tab, Ctrl+V. Named focused conflict dialog exposes Existing/Incoming facts and available-choice explanation. Native Stop closes/cancels while both differing byte strings remain intact. Screenshots checked for actual layout. No Replace/Merge activated and no additional destination file or temporary entry remains.
- **Final release recheck:** native preview checks pass with concise names and collision descriptions, correct position/count, valid correction and Cancel. Repeated copy/conflict check passes with the final available-action description and Existing/Incoming facts.
- **Limits/artifacts:** Failure, empty-trash, drop-choice and desktop-shortcut variants share authored semantics but were not newly replayed; failure-path/cancel/narrow acceptance remains W-076. Native disabled/expanded/text-interface and cache-notification findings are unchanged; no Orca/speech/full modal-traversal pass claimed. Initial immediate tree fetch after F2 preceded the rendered dialog; subsequent checks waited for the modal before input. Temporary clients may emit the previously documented cache-signature warnings, retained in logs. Artifacts/probes/logs/screenshots saved in ignored target/rust-toolchain/w074h. Continue public native text-range feasibility and remaining S11 work.

### 2026-10-23 — W-074g expose properties facts and permission controls
- **Phase:** 6 Implement (S11)
- **Result:** properties is a named focused modal; metadata facts and distinct permission labels/groups verified through AT-SPI. Remaining dialogs and native adapter/Orca acceptance stay open.
- **Red:** committed W-074f exposes neither a properties dialog nor its metadata facts; permission controls repeat Read/Write/Execute without class context. Native contract fails on missing Properties: a.txt dialog.
- **Change:** role/name/modal metadata on the existing focused surface; named type/size/timestamp/target facts; named Permissions/Owner/Group/Others groups with mode description and class-qualified checkbox names. Loading/error and special-bit/shortcut explanations receive semantic text/status nodes. Layout, visible labels, editing and operation policy remain. No toolkit/dependency/token/unsafe/locked-decision change.
- **Checks:** initial `rust-tc check` caught a permission metadata block placed in body rather than permissions (mode out of scope); moved it to the correct function before building. Final `rust-tc build-diagnostics` PASS and `rust-tc doctor` PASS (407 tests, zero skipped; doctests, dependency/licence/advisory checks, features, layers and Rust 1.95). `git diff --check` PASS; code/diff reviewed. Graph discovery/trace/snippet plus best-effort coverage used for properties.rs and related dialog sources. Native behavior contract supplies red/green verification; no assignment-mirroring unit tests.
- **Isolated UI:** private KWin/home/bus, 1280x800, generated a.txt only. Alt+Enter exposes focused/modal Properties: a.txt, Type: Plain text document, Size: 14 B, modified/accessed/created facts, named permission groups and all nine distinct class/bit names. Screenshot confirms unchanged visible layout. Native Owner Execute action changes the UI to 0744 and enables Apply; Apply is never activated. Escape closes; reopen restores unchecked Owner Execute and no Apply action. Native Close closes the dialog. On disk mode remains 0644 throughout; all generated file bytes unchanged.
- **Client/cache limit:** the first live toggle probe timed out while the screenshot already showed the changed permission and enabled Apply. Its retained accessible objects had stale cached state/interfaces. Repeated with the installed public clear_cache/clear_cache_single APIs and direct state queries; the expected change passes, then discard/reopen/native Close passes. This proves current queried state, not reliable incremental cache/screen-reader notification. The earlier native cache-signature warnings remain an acceptance concern; no suppression or app workaround added.
- **Limits/next:** authored loading/error/special-bit/symlink nodes are not newly replayed on screen. Small-window layout, folder calculation cancellation and unreadable-folder reporting remain W-076; no Orca acceptance. Logs, probes and screenshots saved under ignored target/rust-toolchain/w074g. Continue operation/launch/batch dialog semantics, then native text feasibility and remaining S11 queue.

### 2026-10-23 — W-074f expose context-menu focus and actions
- **Phase:** 6 Implement (S11)
- **Result:** named menu/items, main/submenu focus, native focus/activation and keyboard skipping verified. R-107 remains open; native disabled and expanded flags are unresolved adapter findings, not waived.
- **Red:** on committed W-074e, Shift+F10 visibly opens the menu but its rows are absent from AT-SPI; native probe fails on missing File actions menu and focus falls to the window.
- **Change:** named root menu with tracked menu focus, named items with stable identity across cursor changes, and active-descendant focus only for the actual current main/submenu item. The application submenu is named; its parent exposes popup/expanded metadata plus a spoken description. Native Focus chooses a row without running it; Click uses the existing menu state/Accept path, retaining command policy. Disabled items carry the AccessKit disabled flag, no actions and an Unavailable description. Existing pointer handlers, geometry and visual tokens remain. No dependency/toolkit/unsafe/locked-decision change.
- **Checks:** `rust-tc check`, diagnostic release build and `rust-tc doctor` PASS; after adding the unavailable description, final `rust-tc doctor` PASS (407 tests, none skipped; doctests, dependencies/licences/advisories, feature checks, layers and Rust 1.95) and `rust-tc build-diagnostics` PASS. `git diff --check` PASS; surrounding source/diff reviewed. Graph search/trace/snippet and best-effort coverage used for browser.rs/menu.rs; pinned native adapter/consumer source read directly. Live native red/green behavior checks provide UI proof; no role-assignment unit tests added.
- **Native-state finding:** the first semantic tree correctly exposed rows/actions/focus, but Paste still reported Enabled despite its authored disabled flag. A dedicated disabled-state assertion fails and is retained. Pinned accesskit_atspi_common checks disabled only inside its read-only-supported branch; plain MenuItem is outside the consumer's supported-role set, so it falls through to Enabled/Sensitive. The app preserves the correct flag and absent handlers, adds an accurate Unavailable description, and retains the failure. This description is not a native-flag fix. Expanded-state omission remains the earlier adapter finding.
- **Isolated UI:** private KWin/home/bus, generated a.txt, 1280x800. Native menu probe sees named rows and exactly one focused item, initially Open. Down focuses Open with; Right opens the named application submenu and focus moves to its first installed application (Zed); Left restores main-menu focus. No application was launched. Native GrabFocus on Copy changes the focused item without clipboard action; Down skips unavailable Paste to Rename. Native Click on Rename closes the menu and exposes the focused New name field; Escape cancels. Native Click on Open with opens its submenu and focuses a child, without launching it; Left/Escape closes it. Final release recheck verifies named/focused menu and Paste's Unavailable description, no Action interface and no focusability. No real device row or delete/trash item activated. Generated fixture bytes unchanged.
- **Limits/artifacts:** temporary native clients emitted AT-SPI cache signature warnings during subtree replacement; their requested state assertions completed successfully, and warnings were retained. No Orca/speech acceptance or native disabled/expanded pass claimed. Logs, screenshots and public probes saved under ignored target/rust-toolchain/w074f. Session retained for the contiguous dialog check. Continue dialog semantics and native text feasibility; keep all adapter gaps visible.

### 2026-10-23 — W-074e expose palette results and safe selection
- **Phase:** 6 Implement (S11)
- **Result:** named modal palette, virtual result collection, selected result and result status exposed; native selection chooses without execution. Other dialogs/menus, native text/expanded-state gaps and Orca remain R-107 work.
- **Red:** committed W-074d exposes the search field only. Public native probe fails its missing Command palette dialog assertion; result roles/selection are absent from the tree.
- **Change:** named modal Dialog, named ListBox with actual total and named selectable result rows with positions, details and shortcuts. Search field retains real focus. A polite Status node describes pending/empty results or the selected result and count; existing visible instructions/layout remain. Native Focus/Click select the requested result and keep the editor focused; Enter is the explicit activation step. Ordinary pointer click still runs a result. Delayed selection/pointer callbacks carry their query generation and reject a stale or missing row, preventing an old result position from selecting a new query's command. No toolkit/dependency/token/unsafe/locked-decision change.
- **Checks/effectiveness:** `rust-tc quick` PASS, 407 tests. Two new behavior tests cover stale-query selection and pending/out-of-bounds selection. Allowing older generations fails the stale-query regression alone (406 passed/1 failed); accepting row=len fails the missing-row regression alone. Both mutations restored before final `rust-tc doctor` PASS (407 passed, zero skipped; doctests, dependencies/licences/advisories, feature combinations, layers and Rust 1.95). Final local render-chain cleanup was followed by another passing full gate and `rust-tc build-diagnostics`. `git diff --check` PASS. Graph search/trace/snippet and best-effort coverage used for palette.rs; surrounding code/pinned native selection dispatcher read directly.
- **Isolated UI:** same private KWin/home/bus, 1280x800, generated folder only. On the new release, the native contract observes the modal Command palette, Search results, exactly one selected named list item, and position/count metadata (first of 82 with seven visible results). Native Selection.select_child(1) with a bounded state wait selects Navigation: Forward while the palette stays open; Down selects Navigation: Up one folder at 3 of 82. Native GrabFocus on Navigation: Reload folder selects it at 5 of 82 without running it. Search input remains focused. A no-match query leaves an empty collection and a named status explaining what to try. Query view compact gives one result at 1 of 1; Enter closes the palette and shows compact mode. Reopening, querying view details and a pointer click on the visible result switches back to details. Screenshots checked for actual layout/mode; underlying generated bytes unchanged.
- **Probe limits:** an initial short-lived Selection probe reported request acceptance without observing a changed selection; it is excluded as behavior proof. Replaced with a GLib event-loop probe that waits for the requested selected state and asserts the dialog is still present; it passes. This is native API/state evidence, not speech/Orca acceptance. No deterministic forced delivery of a stale native event is claimed beyond the state regression/mutation tests. Loading text is authored but no timed screenshot of an in-progress palette query is claimed. Background surfaces remain in the native tree with the dialog marked modal; full screen-reader traversal/focus containment remains open.
- **Artifacts/next:** ignored target/rust-toolchain/w074e holds probes, failing baseline, mutation/full-gate/build logs and screenshots. No user files or device controls used. Continue dialog/menu semantics and native text-range feasibility; R-107 remains unchecked.

### 2026-10-23 — W-074d attach input semantics to actual editor focus
- **Phase:** 6 Implement (S11)
- **Result:** focused editable nodes verified for location, single-file rename in all three views, palette search, bookmark/group rename and all five batch-rule inputs. R-107 remains open for native text interfaces, other widget semantics and Orca.
- **Red evidence:** a public AT-SPI probe against committed W-074c found one named Folder location entry with editable/focusable states but no focused state after Ctrl+L; its focused-state assertion failed. Pinned component source shows semantics on a separate frame handle while keyboard focus belongs to the editor. External source is outside the project graph and was read directly.
- **Change:** a shared app-owned wrapper puts the role/name/value/focus and editing actions on the editor's public focus handle. The existing styled Input remains the editor/presentation, with its frame role disabled to avoid duplicate named entries. SetValue delegates to the public text-input handler using UTF-16 bounds rather than resetting the state/history. No component fork, dependency, toolkit version, visual token, unsafe or locked-decision change.
- **Checks:** baseline W-074c gate PASS; `rust-tc check`, `rust-tc build-diagnostics` for the location prototype and final all-field implementation, then `rust-tc doctor` PASS (405 tests, zero skipped; doctests, dependencies/licences/advisories, features, layers and Rust 1.95). `git diff --check` PASS; surrounding source/diff reviewed. Graph discovery/trace/snippets and best-effort coverage used for affected UI files, no recorded issues; new input.rs covered after indexing. Native behavior probes provide the regression proof; no fluent-builder mirror tests added.
- **Final isolated UI:** private KWin/home/bus, 1280x800, generated sub/a.txt/b.txt fixture. Ctrl+L now exposes exactly one focused editable Folder location; typing the generated sub path and Enter navigates to inner.txt. F2 exposes exactly one focused New name in details/icons/compact. A selected-name replacement with a single x key followed by Ctrl+Z visibly restores a.txt; Escape preserves names. Palette search is focused and typing new folder displays the corresponding result. Find is initially focused in batch rename; native GrabFocus moves to Replace with and typing c reaches that field. Pattern mode exposes distinct named Pattern/Start/Step nodes; sequential native focus requests each reach the real editor. Native focus on the generated bookmark, F2, and creation/rename of a private New Group expose the correctly focused place/group field. Screenshots confirm field geometry and visible focus rings. Escape cancels without file mutations.
- **Native text limitation:** an exploratory EditableText SetTextContents probe failed because the field has no EditableText interface; a Text GetText probe likewise found no Text interface. Source inspection confirms the pinned adapter does not advertise EditableText, and Text requires text-range children rather than only the string value property. No SetValue-through-AT-SPI or full text/speech acceptance is claimed. The app preserves its component-equivalent SetValue action, whose end-to-end native dispatch remains unverified. The pinned inner editor still emits an informational no-role log, but native focused state on the wrapper is verified; no logs/findings suppressed.
- **Evidence integrity:** final focus probes all passed. An initial sidebar attempt used an unbound Alt+P and targeted the browser; excluded and repeated after native focus on the generated bookmark. A multi-character keyboard_type/one-undo check left the first character because it spans toolkit edit transactions; repeated with one key and observed exact restoration. One icons screenshot was overwritten by a same-second compact screenshot, so icons was recaptured separately. Generated fixture bytes are unchanged. Artifacts/probes/logs and screenshots copied to ignored target/rust-toolchain/w074d. No device control activated. Session retained briefly for contiguous W-074 checks; no Orca or real-desktop acceptance claimed.
- **Next:** palette/dialog/menu semantics and native text-range feasibility, retaining native expanded/text and screen-reader gaps explicitly.

### 2026-10-23 — W-074c expose places, groups and folder tabs
- **Phase:** 6 Implement (S11)
- **Result:** names/roles, group ancestry, spoken group state, focus and activation paths verified for the places panel and tab strip. R-107 remains open; the pinned adapter's native expanded-state gap is retained as an unresolved finding.
- **Red evidence:** with two generated bookmarks, Work containing sub, and two folder tabs, the live tree had neither places/group roles nor tab roles; close/new/eject icon buttons were unnamed. The public AT-SPI widget-contract probe failed on the absent places tree. No user home, bookmark file or device was used as a mutable fixture.
- **Change:** places and devices are a named tree with named rows and focus/activation actions; grouped bookmarks are actual children of their group rather than semantic siblings. Rendering keeps the existing header and member geometry, drop target, colours and spacing. Group descriptions state expanded/collapsed and the activation effect. The tab list contains only tabs, with labels, selected state and positions/count; new-tab is a sibling control, and new/close controls have names. Device names, mounted state and eject labels are exposed without changing device behavior. Sidebar rename fields are labelled. No dependency, unsafe, toolkit, token or locked-decision change.
- **Checks:** baseline W-074b doctor PASS, 405 tests. An intermediate focus-guard signature edit missed one call-site argument; compiler caught it, corrected without suppression. Final `rust-tc doctor` PASS (405 tests, none skipped, doctests, licences/advisories, features, layers and Rust 1.95). Final `rust-tc build-diagnostics` PASS after that gate completed; `git diff --check` PASS. Graph searches, traces, snippets and best-effort coverage covered sidebar.rs/workspace.rs; surrounding source/diff reviewed. Pinned external adapter source read directly. No assignment-mirroring unit tests; live public-interface assertions provide the UI behavior proof.
- **Native adapter finding, not waived:** the first candidate exposed names and correct group ancestry, but its separate EXPANDED-state assertion failed. The pinned accesskit_atspi_common 0.19.1 state-set conversion does not map the AccessKit expanded property, and its Action interface forwards Click only. The app still sets the correct expanded metadata; removed speculative separate Expand/Collapse handlers, retained the existing click/keyboard toggle and added an accurate spoken description. Preserved the failing native-expanded contract/log in ignored target/rust-toolchain/w074c. A separate description/hierarchy check passes; this does not turn the native-flag failure into a pass or accept R-107.
- **Final isolated UI:** installed KWin CLI, private home/bus, 1280×800; normal screenshots and AT-SPI. The semantic check observed the named places tree, Work with sub as its actual child, two named/selected tabs and named close/new controls. Component.GrabFocus on Work focused the group. Left/Right and an AT-SPI click collapsed/expanded it; descriptions changed accurately and child nodes disappeared/reappeared. Focus on the generated parent-folder bookmark did not navigate (inner.txt stayed visible); click opened the generated parent folder. F2 exposed New place or group name, Escape cancelled without changing the title. G moved the bookmark into/out of Work. A waypoint drag onto the group header moved it back in; configuration and live tree showed both bookmarks under Work. No device row, mount, unmount or eject control was activated.
- **Tabs:** AT-SPI click selected sub at position 2 of 2; named New tab created/selected position 3 of 3. Clicking the inactive parent folder's named close button left the correct sub tab active at position 2 of 2. Ctrl+Shift+Tab/Ctrl+Tab switched positions 1/2 while exactly one tab remained selected. Finally a mouse drag with 100–150 ms waypoint pauses moved the active parent-folder tab from second to first; screenshot, native positions/selection and session.toml all agreed on parent then sub, active=0. This closes the previously unobserved mouse tab-reordering loop; it does not cover file drops onto tabs.
- **Integrity/cleanup:** generated a.txt, b.txt and sub/inner.txt retained their original bytes. Only private fixture bookmarks/groups/session changed. Logs, screenshots, probes and private config copies saved under ignored target/rust-toolchain/w074c before session_stop/quit. Temporary GI probes emitted a get_action_name deprecation warning; the actual requests and assertions passed. No host settings, real files, toolkit sources or devices changed.
- **Limits/next:** native expanded flag remains absent; speech descriptions are a fallback, not a verified Orca pass. Text-input focus still falls back to the window; the runtime log explicitly reported a focused element with no accessibility node during rename. Palette/menu/dialog semantics and full keyboard/focus/Orca acceptance remain W-074. R-101 strict pacing is unchanged and unaccepted. Continue with the input-focus/public-component APIs and remaining S11 queue.

### 2026-10-23 — W-074b make the accessible file collection support multiple selection
- **Phase:** 6 Implement (S11)
- **Result:** file-list multiple-selection contract verified through AT-SPI in all three layouts; W-074/R-107 remain open for other widgets/input focus and Orca.
- **Red behavior:** a live Selection-interface probe against W-074a's release binary found the first details collection child was the Sort by Name button and multiselectable was false. A separate icons-view probe selected the first two generated files through AT-SPI and observed only a.txt, failing the expected additive selection. These are public accessibility behavior probes, independent of the fluent UI construction.
- **Change:** one shared list container now owns the virtual file rows, full count and multiselectable flag; sort headings remain siblings outside that collection. Uses the pinned public A11ySubtreeBuilder parent_node hook. Explicit accessibility Click toggles the requested entry, matching the adapter's select/deselect dispatch; Focus retains its existing single-entry focus behavior. Ordinary pointer handlers and modifier behavior are unchanged. Empty collections use the same semantics. No dependencies, toolkit sources, unsafe code, design tokens or locked decisions changed.
- **Checks:** baseline W-074a full gate PASS, 405 tests. Final `rust-tc doctor` PASS (405 tests, none skipped; doctests, dependencies/licences/advisories, features, layering and Rust 1.95); `rust-tc build-diagnostics` PASS after the gate completed; `git diff --check` PASS. Graph search/trace/snippet and best-effort coverage used for browser.rs. External pinned adapter/tree-builder source was outside the project index and read directly: the adapter dispatches select/deselect as Click and does not implement select_all/clear_selection; keyboard Ctrl+A/Escape remain available. No new unit test mirroring role assignments.
- **Final UI/probes:** isolated KWin/home/bus, generated sub/a.txt/b.txt only, no devices activated. The direct file-collection contract passed (first child list item sub, multiselectable true, select_child(0) accepted). In separate sequential details, icons and compact runs, select_child(0)/select_child(1) produced sub+a.txt; deselect_child(0) left only a.txt. The probes wait on AT-SPI selection events with a bounded timeout, not fixed sleeps. A normal screenshot confirmed unchanged row geometry/selection markers. Ctrl+A followed by a plain click on a.txt selected only a.txt; Ctrl-click on b.txt then selected exactly a.txt+b.txt, confirmed semantically. Initial overlapping probe processes were excluded and rerun sequentially. The isolated IsEnabled flag was observed false after temporary event-listener probes ended; re-enabled it only on the private bus and rechecked exposure before each subsequent sequence. No host accessibility setting was changed.
- **Large-list/layout check:** final diagnostic build, generated 500,000-entry details fixture, accessibility activated at startup and flag still true after the run. Ten 120-frame summaries, interval p95 16.736–17.034 ms / max 33.932 ms; max draw/layout/prepaint/paint 4.519/0.749/2.707/2.835 ms. Bounded AT-SPI traversal again exposed only 20 file rows, 37 total nodes, with setsize=500000 and positions 3591–3610. Normal screenshot confirmed the list/column layout and full count. Probe logs/scripts and diagnostics copied to ignored target/rust-toolchain/w074b; no general check script or new check entry point added.
- **Limits/next:** no strict 60 fps or Orca pass claimed. The adapter's bulk selection methods and off-screen discovery are limited; this verifies exposed rows and the normal keyboard paths. Named text inputs still need focus investigation; places, tabs, menus and dialogs need their semantic/focus pass. Continue W-074's remaining widget work with the pinned toolkit and existing decisions.

### 2026-10-23 — W-074a enable isolated AT-SPI checks and expose file-list semantics
- **Phase:** 6 Implement (S11)
- **Result:** pinned Wayland accessibility adapter verified without upgrade/fork; missing file-list and browser-control semantics implemented. W-074/R-107 remain open for other widgets, text-input focus, complete keyboard/focus review and Orca.
- **Diagnosis/regression:** source inspection found the existing AccessKit Unix adapter and its subscription to org.a11y.Status.IsEnabled. On the isolated session bus that property was false. Setting it true through that session's dbus_call immediately exposed the existing app: only six buttons, five unnamed, with no entries. The running AT-SPI service alone had not enabled accessibility. No application change forces a desktop accessibility setting. This corrects the interpretation of earlier missing-tree observations; they were not evidence that the pinned toolkit lacked a Wayland adapter.
- **Change:** named browser/list containers; named navigation, breadcrumb, sort and text-input controls; named selectable entries with active-descendant focus and an AT-SPI focus action shared by all three virtualised layouts; readable status. Stable per-listing entry IDs replace row-based element IDs. Item positions use AccessKit's zero-based index, and the total count belongs on the list container. The adapter converts positions to AT-SPI's one-based values. No visual values, icons, dependency, runtime, unsafe code, toolkit sources or locked decisions changed.
- **Checks:** baseline W-073e doctor PASS, 405 tests. Initial compiler/full-gate attempt caught applying a stateful role to a plain Div; added its explicit ID, without a suppression. Subsequent gates passed. Final `rust-tc doctor` PASS (405 tests, none skipped, doctests, dependencies/licences/advisories, features, layers and Rust 1.95); final `rust-tc build-diagnostics` PASS only after the gate exited; `git diff --check` PASS. No construction-mirroring unit tests: the before/after live AT-SPI tree is the behavior regression. Graph search/trace/snippet and coverage verified the browser path with best-effort coverage; external pinned toolkit/adapter sources were outside the graph and read directly.
- **Behavior/effectiveness:** normal and semantic checks through the installed KWin CLI, isolated home/bus, generated files only; no real device activated or host setting changed. After confirming the final window/list, Home selected/focused sub, Down selected/focused a.txt; queries retained both states through icons, compact and details. A temporary isolated Atspi Component.GrabFocus probe selected/focused b.txt; the normal status reported one of three selected. F2 exposed New name, Escape cancelled, Ctrl+L exposed Folder location, Escape returned to the list. A screenshot confirmed the visible selection marker. An initial command batch sent before the final window was confirmed is excluded; the checked sequence was repeated after observing the ready list.
- **Adapter-contract regressions:** the initial status node had an empty accessible name despite aria_label; the adapter reads a Label's value, so the final node uses aria_value and reports the actual count. A temporary position probe observed posinset=2 and missing setsize on the first item; corrected the AccessKit index/count placement, then observed posinset=1 and setsize=3. The probe's first assertion mistakenly treated GI's attributes dictionary as a string list; corrected that probe and repeated the failing old-binary check before the final build, then it passed. The focused-entry action probe also passed on the final binary.
- **Large-list evidence:** final diagnostic build, accessibility still enabled, generated 500,000-entry details fixture. Ten 120-frame summaries: interval p95 16.730–17.296 ms, maximum 31.476 ms; max draw/layout/prepaint/paint 5.287/0.893/3.302/1.404 ms. An isolated bounded AT-SPI traversal observed 37 nodes and 20 file entries; first/last entries reported positions 3591/3610 and setsize=500000. The normal screenshot confirmed 500,000 entries. Small-fixture focus/position logs and screenshot copied to ignored target/rust-toolchain/w074a; the temporary session was stopped after verification.
- **Limits/next:** no Orca executable found on PATH (nor package in the queried system package database), so no screen-reader pass is claimed. Named text inputs did not report focused on the entry node during the observed Ctrl+L flow; investigate this separately. Places, tabs, menus and dialogs still need semantic/focus work; the list's multi-selection accessibility capability also needs review. All R-107 criteria remain unchecked. Strict frame pacing remains W-073; this run does not waive it. Continue W-074 with those widget/focus gaps, then the remaining S11 queue.

### 2026-10-23 — W-073e preserve large selections without UI-thread name matching
- **Phase:** 6 Implement (S11)
- **Result:** measured selection-transfer stall and manual-refresh selection loss fixed. Final three-view measurement checkpoint complete; strict 60 fps and wider R-101 acceptance remain open.
- **Regression:** on the generated completed 500,000-entry details fixture, Ctrl+A/F5 published a replacement beginning at 8,192 entries; the eventual status retained only 8,192 selections. Name carry-over took 63.492 ms and the model callback 63.562 ms on the UI thread. The new public-model regression initially failed (402 existing passed): manual refresh emitted three partial/final snapshots rather than one complete replacement. An intermediate implementation's explicit refresh error was silently treated like a watcher failure; a new injected permission-denied test timed out, then passed after separating background failure handling from quiet publication. A third regression rejected transferring same-name selections into another folder; initially failed, then passed with the worker API's folder guard.
- **Change:** manual reload retains the existing view until the complete replacement arrives, including when the previous read is unfinished. Explicit refresh errors still reach the UI. Selection name matching runs on the core's existing Rayon pool and returns over a bounded channel. The UI retains usable rows, checks its selection revision before applying a worker answer, and rematches after intervening selection input; ordinary snapshot generations still reject superseded navigation. A disconnected worker produces a visible refresh instruction. Same-listing updates keep the existing lightweight sync. No new dependency/runtime/unsafe code, toolkit change or visual token change.
- **Checks/effectiveness:** testing-suite detector confirmed Cargo/nextest. Restoring progressive manual reload made its new regression fail alone; bypassing worker name matching failed the full-selection refresh and existing live-update selection tests. Restored all mutations before final `rust-tc doctor` PASS (405 tests, none skipped, plus doctests, licences/advisories, features, layers and Rust 1.95). `rust-tc build-diagnostics` PASS after the gate; `git diff --check` PASS. Graph search/trace/snippets and direct surrounding reads covered selection/model/browser; coverage tool reports no recorded issues for model.rs, selection.rs, browser.rs, diagnostics.rs and settings.rs, with its best-effort caveat.
- **Headless:** `rust-tc bench-listing` PASS: worst of five first batches 5.4 ms, complete name ordering 215.7 ms; details at top 56.4 ms, all details 201.9 ms; re-sort Size 54.9 / Modified 37.7 / Type 66.0 / descending Name 46.6 ms. This run overlapped startup of the UI check app, so it is a bounded passing result under that load, not a contention-free comparison. Subsequent selection checks and three-view runs began after the headless process exited.
- **Final UI:** same isolated KWin/home, 1280×800 and generated XFS fixture; no devices or user files touched. Normal screenshot after completed refresh explicitly showed 500,000 of 500,000 selected. F5/Home showed one selected entry both before and after completion. A fresh small-to-large navigation followed by Home in the timed action sequence visibly selected alpha_57.txt while 262,144 entries were still loading. Burst frames assess text/state only because of the tool's colour-channel discrepancy. Over the exercised final interactive run, model callback max 0.818 ms, selection application/sync 0.452 ms, scheduling 0.020 ms, versus the original 63.562/63.492 ms callback/carry. Scripted input ordering was exercised; no deterministic forced input-at-the-exact-worker-completion race is claimed. Fresh error-reporting behavior is covered by the injected core test, not a new permission-dialog UI replay.
- **Three-view P1/P2:** final release diagnostic build, each mode seeded in a separate config inside the isolated home, one app at a time, ten 120-frame summaries per mode. Screenshots confirmed each mode and 500,000 entries. Details: interval p50 16.659–16.667 ms, p95 16.730–16.979 ms, max/longest 32.902 ms; max draw/layout/prepaint/paint 4.087/0.611/3.149/0.918 ms. Icons: p50 16.659–16.665, p95 16.735–16.832, max/longest 35.057; max draw/layout/prepaint/paint 4.631/0.746/2.510/2.800 ms. Compact: p50 16.662–16.666, p95 16.720–16.830, max/longest 33.780; max draw/layout/prepaint/paint 6.666/0.656/4.097/2.338 ms. All summaries reported zero excluded gaps. Max model callback across these runs 0.687 ms, visible-row construction 0.451 ms. Logs and selected screenshots copied to ignored target/rust-toolchain/w073e before session cleanup.
- **Limits/next:** these intervals do not pass a strict 16.667 ms p95, despite drawing below 7 ms. No threshold relaxation, physical-display claim or exhaustive thumbnail/input/task proof. R-101 stays unchecked; R-102 retains its earlier default-build evidence. W-073 remains open for strict pacing/wider acceptance reconciliation. Continue independent S11 W-074 accessibility assessment with the same toolkit locks. Isolated compositor retained briefly for the contiguous accessibility check; benchmark apps stopped.

### 2026-10-23 — W-073d keep navigation listings and actions bound to their folder
- **Phase:** 6 Implement (S11)
- **Result:** navigation transition defect fixed; fresh warm-cache first-visible-content measurement below 200 ms in the exercised details case. W-073/R-101 remain open for final three-view pacing and wider input/task coverage.
- **Regression/change:** the previous timed UI run exposed old-folder rows under the newly requested location before its first snapshot arrived. Rendering, selection actions, properties, rename/create and row/drop targets now use only a snapshot whose path matches the model's requested location. The retained old snapshot still supports failed-navigation rollback and selection recovery. Successful arrival clears the previous folder's rename/menu state; closing a focused menu explicitly returns keyboard focus. No filesystem work moved into the UI, no dependencies/tokens/locked decisions changed, and no fluent-construction unit test was added in place of the observed UI regression.
- **Fresh checks after resume:** previous temporary artifacts were unavailable, so repeated `rust-tc doctor` (PASS: 402 tests, none skipped; doctests, dependencies/licences/advisories, feature combinations, layers and Rust 1.95 included), `rust-tc build-diagnostics` (PASS, after the gate completed), and `git diff --check` (PASS). The indexed graph reports no recorded coverage issue for browser.rs; searched current_snapshot/dismiss_menu/on_model_event, traced the snapshot reader's callers, read the event-handler snippet and checked the actual diff/source. Coverage remains best-effort, not exhaustive proof.
- **UI evidence:** installed KWin CLI with scoped XDG_RUNTIME_DIR, isolated KWin/home, 1280×800, generated small fixture and existing generated 500,000-entry XFS fixture only. Normal screenshots confirmed Home selection, Shift+F10 menu, F5 dismissal and working Ctrl+L focus; opening a missing child restored the small folder, selected a.txt and an explicit location-does-not-exist banner. F2 on a large-folder entry displayed the inline editor; Escape cancelled. No device activated. AT-SPI again omitted Rustphin, so screenshots supplied visual evidence. The direct MCP session startup hung; its owned failed wrapper was stopped and the functioning installed CLI used instead.
- **Timed capture:** location field/path verified before Enter. A same-host time_ns anchor was written before sending the key. The nominal 0 ms capture showed the new path, no old a.txt row and Loading; its PNG was saved within 73.877 ms of the anchor. A separate genuine small-to-large navigation's nominal 100 ms capture showed alpha entries and 131,072 items — loading; PNG mtime was 185.611 ms after the anchor. Since KWin captures raw frames before PNG encoding, this is a conservative upper bound including control/key/capture/encoding overhead, not an exact presentation latency. The earlier two-frame burst showed 65,536/262,144 new entries but encoding finished after 200 ms; it is not used for the threshold claim. Burst screenshots have a tool colour-channel discrepancy, so only text/geometry/state are assessed. This measures a warm-cache details navigation, not cold launch, every mode or physical-display pacing.
- **Limits/next:** strict 60 fps and exhaustive sub-16 ms work/input throughout loading are not accepted or waived. The previous candidate/focus-probe evidence was not silently promoted after temporary files disappeared; the final checks above were repeated. Continue W-073's three-view assessment, then W-074. Isolated session remains temporarily active for those contiguous checks.

### 2026-10-22 — W-073c remove empty-metadata layout stalls
- **Phase:** 6 Implement (S11)
- **Result:** remaining measured 80 ms browser layout spike fixed in the exercised details path; W-073 remains open for timed first-visible entries and final frame pacing evidence.
- **Diagnosis:** 200 bounded GDB main-thread samples in the isolated fixture captured font file mapping/parsing inside COSMIC get_font_matches during shaping. An initial one-sample debugger launch ended before UI work and is not evidence. A separate normal run with cosmic_text::font::system debug logging identified a cold generic SansSerif/metadata-0 lookup taking 81.62 ms after the named font warmup had completed. The pinned shaper uses default attributes for empty text. The details view added empty strings for missing size/date/type values, causing that unnecessary work. Debugger runs were excluded from acceptance timings; no GDB security/global configuration changed and no toolkit source was edited.
- **Change:** only add size/date/type text children when nonempty; keep the same column divs, widths, spacing, styles and eventual values. No new dependency, abstraction, token or locked decision. No unit test mirroring fluent UI construction; the existing P1/P2 runtime benchmark supplies a meaningful failing/passing regression contrast.
- **Checks:** baseline W-073b doctor PASS with 402 tests. Candidate `rust-tc build-diagnostics` PASS. Final `rust-tc doctor` PASS (402 passed, none skipped), including dependency/licence/advisory, feature, layer and minimum-Rust checks. `git diff --check` PASS. Pinned source read for shaper, font matching, element layout, asset loading and toolkit journal; graph APIs unavailable.
- **Runtime evidence:** final diagnostic candidate on the same generated 500,000-entry XFS fixture, isolated KWin/home and Intel Arc B580/Vulkan. Ten 120-frame summaries; no late generic SansSerif match appeared in the debug log. Max draw 5.777 ms, request-layout initially 0.930 ms and 1.132 ms after subsequent input/reload/sort, prepaint 4.400 ms, paint 0.820 ms; presentation max 1.896 ms. Across subsequent reload/selection/sort, model/selection transfer max about 1.113/1.102 ms; visible row construction max 0.244 ms. These are observed wall times for these exercised paths, not a proof that every possible UI task is below 16 ms.
- **P4/UI:** final screenshot showed 500,000 entries and populated size/date columns. F5 followed by Home visibly selected alpha_57.txt while status still said loading. F5 followed by clicking the observed Size heading showed its ascending arrow while loading; later screenshot retained the selection, loaded values and arrow after loading ended. An earlier Ctrl+R attempt is not counted as refresh: this app binds F5. No device touched. App stopped; isolated compositor retained temporarily for the next bounded measurement.
- **Limits/next:** logged p95 frame intervals still include virtual-compositor jitter (up to 17.40 ms in a window); one initial interval reached 31.41 ms despite all measured drawing under 6 ms. Strict 60 fps is not claimed or waived. First-visible-entry time has not yet been measured by a timed compositor capture. No total cold-launch or physical-display acceptance. Continue W-073 with those measurements and carry any external/manual limit into S11 reconciliation.

### 2026-10-22 — W-073b prepare fonts off the UI thread
- **Phase:** 6 Implement (S11)
- **Result:** measured initial prepaint stall fixed; separate browser layout stall remains W-073. R-102 memory criterion now checked from a final default-build actual-tab-close measurement.
- **Change:** background task calls the pinned text system's public prewarm_fonts API for the system default plus Inter, JetBrains Mono and Space Grotesk from theme constants, in normal/medium/semibold styles. Startup asynchronously awaits completion before opening the same window/root/theme/session. Extracted window wiring for readability. No token, font-installation, dependency, toolkit, unsafe or locked-decision change. System font matching confirms the three configured families are installed.
- **Regression evidence:** W-073a's isolated P1/P2 trace was the failing UI benchmark: initial prepaint 269–305 ms. With font resolution warmed in the background, a diagnostic details run on the same generated 500,000-entry fixture measured font preparation 284.1 ms off-thread, initial prepaint 2.86 ms maximum over that run, but layout 80.71 ms / draw 83.99 ms remained. A probe also warming printable-ASCII shaping did not remove layout (78.32 ms / draw 81.46 ms, prepaint under 3.74 ms); that extra code was removed. This avoids retaining an ineffective workaround. A launch after a failed probe build ran the preceding binary; that run is excluded from comparison. This is an observed GUI regression contrast, not a deterministic unit performance test.
- **Checks:** baseline W-073a doctor PASS, 402 tests. Early compiler checks caught two pinned-API assumptions (AsyncApp update returns the closure value; shaping belongs to WindowTextSystem); corrected, with no suppressions. Final `rust-tc doctor` PASS (402 tests, none skipped), including licences/advisories, feature checks, layers and Rust 1.95. Diagnostic candidate builds and final default release build PASS; `git diff --check` PASS. Final sources retain font resolution warming only. No new unit test for framework wiring; the runtime benchmark and normal startup/tab/mode flows validate it.
- **Final default UI/R-102:** same isolated KWin/home and generated small/large fixtures, Intel Arc B580/Vulkan. Final default build starts normally after a 288.6 ms background font warm. Small-folder ps RSS 130332 KiB (127.3 MiB); open the 500,000-entry directory in a second tab, confirm all entries/details on screen, RSS 162124 KiB (158.3 MiB). Ctrl+W closes the large tab, small folder visibly returns, RSS 150112 KiB (146.6 MiB), delta 19780 KiB (19.3 MiB / 20.3 MB), under 50 MB. Subsequent icons/compact/details switches on the small fixture displayed correctly and RSS was 150124 KiB. No device activated or host files used. App stopped after verification; isolated compositor remains temporarily for the next investigation.
- **Limits/next:** the standalone warm runs before a window is shown; this measures removal of blocking draw work, not a faster total cold application launch. Initial window/GPU bootstrap still produced a 104 ms foreground poll in one diagnostic run, and the separate browser layout spike remains. Strict p95 60 fps, visible first-content within 200 ms and input throughout initial listing remain unverified/unmet; R-101 stays open. No physical-display or real wheel/touchpad claim. Continue the same W-073 task with a debugger trace and remaining loading/input measurements.

### 2026-10-22 — W-073a scale baselines and trustworthy frame diagnostics
- **Phase:** 6 Implement (S11)
- **Result:** measurement checkpoint complete; W-073 remains open for measured startup drawing stalls, visible first-content/input-during-loading proof and strict frame pacing. Memory passes the measured default-build case; no R-101 checkbox is advanced.
- **Change:** diagnostic timing no longer excludes an active scroll stall over 250 ms; excluded idle/startup gaps are explicitly counted and their maximum logged. Added opt-in model/selection/visible-row task timing. Optional `diagnostics` feature and `rust-tc build-diagnostics` expose the pinned toolkit's foreground journal (draw phases, presentation, retained view work and foreground polls) and report lost entries. Uses the existing public pinned API, with no fork or version upgrade. GPL-compatible hdrhistogram 7.6.0 has MIT/Apache-2.0 licensing confirmed in its fetched manifest; existing dependency/licence policy passes unchanged. Lockfile records the toolkit's optional gpui-fast branch/taffy as a resolution side effect; those are not activated/compiled. No visual tokens, icons or product layout changed.
- **Regression/checks:** previous W-072c full gate passed 401 tests. New deterministic injected-Instant regression reproduced a one-second active scroll stall being dropped (401 passed, one failed), then passed. Restoring the old exclusion condition made that regression alone fail again; restored. Earlier `rust-tc quick` passed before optional profiler wiring; final `rust-tc doctor` PASS with 402 tests, none skipped, including all feature combinations checked by the gate, licence/advisory/layer checks and Rust 1.95. Optional release builds and `rust-tc build-diagnostics` PASS. `git diff --check` PASS. Required test/design/KWin skills read; graph tools unavailable, pinned API and project sources read directly.
- **Headless R-101 baseline:** `rust-tc bench-listing` PASS on the existing generated XFS fixture. Five runs: worst first batch 4.3 ms, complete name order 171.0 ms, details at top 136.2 ms, all details 2578.8 ms (not thresholded), re-sort size 60.9 ms / modified 41.5 ms / type 69.0 ms / reverse name 49.0 ms. A release build overlapped the latter part of this baseline; these are bounded observed timings, not an isolated throughput comparison. Core code unchanged by W-073a.
- **Default-build R-102 measurement:** isolated KWin/home, release built from W-072c, generated small folder, ps RSS in KiB. Small baseline 136412 KiB (133.2 MiB); second tab with 500,000 entries and details present 168428 KiB (164.5 MiB); after Ctrl+W and confirming the small folder returned, 156432 KiB (152.8 MiB), still the same on a later reading. Delta 20020 KiB (19.6 MiB), below 50 MB; large-folder RSS below 400 MB. This tests actual tab closure, not only navigation.
- **UI/phase evidence:** installed KWin-MCP CLI, scoped runtime, isolated home, system-only XDG search directories, generated fixtures, Intel Arc B580/Vulkan, 1280x800. No device activated. Confirmed 500,000 entries in details, icons and compact by normal screenshots; End selected a row, view shortcuts changed modes. Ten full 120-frame summaries collected separately per view using driven scrolling. Final-journal details run: p95 intervals 16.78–16.90 ms, initial draw 283.3 ms / prepaint 282.0 ms. Another details run recorded initial draw 305.1 ms / prepaint 304.2 ms plus layout 75.6 ms (draw 77.8 ms). Icons: p95 16.77–16.96 ms, initial draw/prepaint 269.3/268.4 ms. Compact: p95 16.77–16.97 ms, initial draw/prepaint 274.6/273.7 ms. Presentation max about 2.2 ms. Across these runs model callback max 0.284 ms, selection transfer max 0.002 ms, visible-row construction max 0.461 ms. No journal discontinuities observed in final icon/compact runs. This locates work in drawing phases; it does not prove font-cache causality yet.
- **Limits/follow-up:** virtual output and opt-in profiler overhead; strict 60 fps p95 is not passed or waived. Initial gaps above 250 ms were classified before scrolling began but are now visible in longest-gap/exclusion fields and toolkit phases. Render-start timing is not a presented-frame timestamp; screenshot-after-load does not prove visible-first-entries within 200 ms or input throughout the initial listing. Other callbacks and late full-selection live rereads are not exhaustively timed. P1–P4 protocol added; P4 loading-phase proof remains open. Continue W-073 with background font warming, then repeat the affected measurements. The isolated session is retained temporarily for that next step, without a running app or access to real user files.

### 2026-10-22 — W-072c journal namespace audit and ordering
- **Phase:** 6 Implement (S11)
- **Result:** W-072 audit complete across configuration, cross-filesystem moves and batch recovery; supported guarantees and remaining gaps documented. Physical power loss and R-103 immediate-after-kill batch semantics remain unverified.
- **Change:** journal creation syncs the directory before staging originals. Cleanup syncs all final/restored names before unlinking the record, then syncs that removal. Distinguishes failure before unlink (rollback/recovery available) from sync failure after unlink (retain visible committed names/change history, report uncertainty, never begin rollback without a record). Recovery with no record syncs the directory so a late cleanup sync can be retried. Rollback messages no longer claim a record certainly remains after any failure. API docs updated. No dependency, UI, unsafe, toolkit or decision changes.
- **Checks:** baseline W-072b full gate passed 397 tests. Four new behaviour regressions initially failed (397 passed), then `rust-tc quick` PASS with 401 tests. Disabling all journal namespace syncs caused those four plus the expanded ordinary-failure sweep to fail; restored. Final `rust-tc doctor` PASS with fresh XFS TMPDIR (401 passed, none skipped), including all existing real-SIGKILL batch/apply/undo/recovery tests, doctests, licences/advisories, features, layering and Rust 1.95. `git diff --check` PASS. Graph tools unavailable; exact source paths inspected.
- **Coverage:** no staging when the initial journal-directory sync fails; rollback when final names cannot sync before journal removal; committed names and batch undo record retained when only the post-removal sync fails; explicit recovery reporting its late sync failure after whole original names have already been restored. Ordinary-error sweep now includes all nine boundaries and distinguishes pre-staging failure, recoverable rollback and committed late failure. Earlier staged fixtures now interrupt at step 3 because the new pre-stage sync is a boundary. BR1 fixture command updated to step 4; actually ran the current compiled child at that boundary: exit SIGKILL, journal plus exactly two staged entries. No on-screen replay claimed; UI sources unchanged.
- **Audit limits:** file/directory sync calls and injected delayed errors test application ordering, not a physical power cut or storage-device honesty. Same-filesystem moves and regular-copy publication lack a complete namespace-durability contract; partial source removal can reappear as duplicate data after restart. File and directory identity checks are not atomic protection against unrelated processes racing names/content. No prefix-based restart cleanup is authorised: unknown temporary files are retained, and batch repair is explicit and journal-bound. Invalid pre-stage records remain for inspection. R-103's batch originals may still be staged immediately after kill, until explicit recovery; that acceptance gap stays open. Atomic-configuration criterion alone is checked from W-072a evidence.
- **Follow-up:** W-073 performance/memory, then remaining S11 work; final reconciliation must carry the immediate-after-kill/manual-hardware/physical-power-loss limits forward.

### 2026-10-22 — W-072b cross-filesystem move namespace barriers
- **Phase:** 6 Implement (S11)
- **Result:** ordering and delayed-error handling implemented for cross-filesystem destinations and complete source removal; W-072 continues with batch-journal namespace entries. R-103 atomic-configuration checkbox now has direct W-072a evidence; kill/power-loss criterion remains open.
- **Change:** copied directories apply attributes and sync bottom-up through a handle opened before restrictive final permissions. Attribute/sync failure is offered to skip/retry/cancel instead of only logging; skipping keeps that source subtree. After publishing a cross-filesystem copy, sync its parent before deleting any original. Retry repeats only that sync, preserving undo/copy accounting and avoiding another overwrite/copy. After complete source removal, sync its parent; report late failure as an already-visible move. `Fs::sync_directory` refuses final-component links; unsupported sync is an error. Removed the unsafe stat/rename no-replace fallback: unsupported kernel/filesystem support now fails closed. No dependency, UI, unsafe or locked-decision change.
- **Tests/checks:** baseline is W-072a's passing 393-test gate. Four new behaviour regressions failed first (393 passed), then passed with the fix: publication-sync skip retains both copies and a copy record; retry completes one move with correct bytes; copied-folder sync failure keeps the complete source subtree; source-removal sync failure reports a visible move without repeating it. Initial quick check found one clone-to-slice clippy finding; fixed without suppression. `rust-tc quick` PASS (397 tests). Final `rust-tc doctor` PASS using a fresh XFS TMPDIR (397 passed, none skipped), including existing error/crash/short-copy sweeps and real SIGKILL move/batch/recovery tests, doctests, dependencies, features, layers and Rust 1.95. `git diff --check` PASS.
- **Effectiveness:** deliberately bypassing destination/source sync and copied-folder fault boundaries made all four new regressions fail (393 passed); restored before the full gate. Existing sweeps now include copied-directory and destination/source-parent sync boundaries. No exhaustive cross-filesystem kernel or unsupported-filesystem test is claimed: the cross-device boundary is simulated by FaultFs, and fail-closed no-replace is source-reviewed plus existing operation tests.
- **Limits:** no physical power-cut validation. Partial-source removal is not fully synced and can leave duplicate entries after restart; data remains copied before removal. Regular-copy publication, same-filesystem rename, batch-journal entries, automatic restart cleanup and unrelated-process namespace races remain outside this bounded guarantee. Unknown temporary paths are never automatically deleted. No UI replay required: UI sources unchanged.

### 2026-10-22 — W-072a atomic configuration write hardening
- **Phase:** 6 Implement (S11)
- **Result:** configuration portion complete; W-072 operation-ordering audit continues.
- **Findings/change:** predictable `File::create` temporary paths could follow a preexisting link and truncate another file, concurrent writers shared the same path, default permissions exposed settings more widely, and directory-sync failures were discarded. Writes now use private exclusive temporary creation with collision retries, sync newly created directories/parents, file-sync before rename, and propagate directory-sync failures with an explicit already-published message. Only an owned temporary is cleaned after failure; cleanup errors are reported. No dependency, UI, unsafe or decision changes.
- **Checks:** baseline `rust-tc doctor` PASS (388 tests); testing-suite detector run, required Rust reference read. Two initial public-API regressions failed against the old implementation (388 passed): unrelated symlink target overwritten and nonprivate configuration. `rust-tc quick` PASS (392 tests before concurrent case). Final restored `rust-tc doctor` PASS (393 tests, none skipped), including licences/advisories, features, layering and Rust 1.95. `git diff --check` PASS. Graph tools were unavailable; used direct source reads including every config writer.
- **Effectiveness:** suppressing directory sync caused both new sync-error tests to fail; suppressing publication caused seven failures including concurrent whole-file publication. A fixed temporary-counter mutation survived the short concurrent run because exclusive create/retry can serialize successful writes; that test is bounded concurrency evidence, not exhaustive scheduling proof. All mutations restored before the final gate. The original red run proves the link and permissions regressions.
- **Limits:** a reported late sync failure can leave the new whole file visible. No physical power-cut test, whole read/modify/write transaction lock, or external directory-race confinement is claimed. Newly created empty directories can remain on failure. No UI replay required: UI sources unchanged. W-072 continues with cross-filesystem publication/source-removal ordering and the batch journal's directory entries.

### 2026-10-22 — W-071 recover interrupted batch renames and undo
- **Phase:** 6 Implement (S11)
- **Result:** explicit journal-validated recovery implemented; W-071 complete. R-103's immediate-after-kill criterion and power-loss safety remain open, not reinterpreted as passed.
- **Contract/change:** ARCHITECTURE.md records the versioned binary format, private per-directory record, directory binding, raw names, entry stamps, bounded/no-follow reads, advisory directory lock and no-replace recovery. Apply and batch undo write/file-sync a record before any selected entry moves; successful completion or rollback checks record identity again before cleanup. Changed records are kept. Recovery validates all locations and destinations before moving entries, can be retried after interruption, never guesses from a prefix, and only runs on explicit request. The UI palette exposes **Recover original names after interruption** through the existing operation worker; browsing never invokes it. `Fs::write_new` now creates private files (also appropriate for existing trash records). No dependency or unsafe-code additions, toolkit changes or locked-decision changes.
- **Baseline/checks:** `rust-tc doctor` PASS, 372 tests at baseline. Testing-suite detector confirmed Cargo/nextest. Final `rust-tc quick` PASS on tmpfs, 388 tests; final `rust-tc doctor` PASS with a fresh TMPDIR on XFS, 388 passed, none skipped, including doctests, dependency/licence/advisory checks, features, layers and Rust 1.95. Release build PASS from restored final sources; `git diff --check` PASS. DESIGN.md was updated before the palette action; its required linter reported zero errors/eight existing orphan-token warnings. The codebase-memory skill was loaded, but its graph/index/coverage APIs were not callable in this run; relied-on paths were read directly and no graph-coverage claim is made.
- **Regression/behaviour evidence:** 15 added behaviour tests plus one child-process harness. The first four recovery tests failed before implementation (372 existing passed). Further failing regressions reproduced an operation request incorrectly reporting a recovery I/O pause as preflight refusal, and cleanup removing a recovery record another program changed; both passed after their fixes. Coverage includes swap recovery and idempotence; unchanged refusal for occupied original names, edited entries, malformed/newer records and wrong directory binding (including matching hard-linked entries); raw non-UTF-8/control-character and 255-byte names; ignoring unjournaled staging names; no-follow/size-bounded reads; concurrent batch/recovery refusal; and retrying a paused operation. The existing ordinary-failure sweep was expanded to include record creation, all entry renames and record removal; record-write failure is correctly a preflight `Err`, not a mutated-file outcome.
- **Kill evidence:** three parent tests run the child with real SIGKILL before each changing `Fs` call in batch apply, batch undo and recovery, continuing until a child completes without reaching the kill point. After each kill a fresh `RealFs` recovery is requested; both original byte contents remain under a whole original or final name and no record/staging entry remains. Recovery killed during its own staging/restoration is retried the same way. This is post-recovery evidence, not a claim that paths never become temporary between the kill and recovery. Mid-syscall hardware/power failures are not simulated.
- **Effectiveness checks:** bypassing recovery caused 12 failures, including all three kill suites; ignoring version and directory bindings caused exactly the newer-version and copied-record tests to fail; removing directory locking and no-follow protection caused exactly the lock and bounded-read tests to fail. All mutations restored. One release build was started while a deliberate mutation was active; it was discarded for acceptance and rebuilt from restored sources before UI checks. The final release and full gate use restored code only.
- **UI evidence (BR1–BR4):** installed KWin-MCP CLI, isolated KWin/home, release build, generated fixture only, system-only XDG search paths. No device row was activated. A test child actually killed before the first publish left two staged entries plus the record. Opening the folder showed only three hidden entries and did not recover them. Query `recover original` selected the labelled action; Enter restored `a` = `A`, `b` = `B`, removed the record/staging entries, updated the list and displayed completion. A generated invalid record produced a visible recovery error; record and file contents were unchanged. After removing only that generated invalid record, F2 on both files showed the existing replace dialog: `a` → `b` blocked Rename with conflict text; changing replacement to `c` enabled it and left `b` = `B`, `c` = `A`. Ctrl+Z restored `a` = `A`, `b` = `B` with no journal. Restarting the final build and repeating recovery without a record displayed "Checked rename recovery in “src”." and changed nothing. Session stopped after verification. AT-SPI still did not expose Rustphin; normal screenshots and fixture state supplied the evidence.
- **Limits/follow-up:** recovery applies only to journaled batches introduced here; older unjournaled temporary files require manual inspection. A record write that fails before any selected entry moves can leave an invalid record, which is retained rather than guessed at. R-103 remains unchecked because a killed batch may have staged names until explicit recovery. Directory-entry sync, power-loss ordering, copy restart cleanup and unrelated applications racing namespace changes remain W-072. Normal undo history still does not survive restart. S11/daily-driver acceptance is not complete.

### 2026-10-20 — W-070 sync copied regular files before publication
- **Phase:** 6 Implement (S11)
- **Result:** copied-file sync implemented and fault-tested; R-103 remains open for batch rename, namespace durability and the other outstanding criteria.
- **Change:** `Fs::copy_file` now guarantees synced file content and attributes before success. The shared real-filesystem copy helper retains the destination handle through permissions, modification time and `sync_all`, avoiding a reopen after restrictive permissions have been applied. Sync errors retain their I/O kind, add stage context and return through the engine's existing cleanup and skip/retry/cancel path. `FaultFs` counts the sync as a distinct fault/kill boundary and can fail the first sync after a complete write. No new runtime dependency, unsafe code, engine mutation route or UI change.
- **Regression evidence:** added four behaviour tests through the operation API. A sync failure leaves no published or temporary copy; an explicitly replaceable destination retains its original content; a simulated cross-filesystem move retains its complete source; retry creates one whole copy and reports the correct final byte count. Before enabling the sync boundary, `rust-tc test` ran 372 tests: the four new tests failed and the previous 368 passed. The failures included an incorrectly published copy, replacement and removed source.
- **Checks:** `rust-tc quick` PASS with 372 tests on the default tmpfs. Its first attempt stopped on five clippy slice-clone findings in the new tests; replaced those clones without lint exceptions, then re-ran successfully. Final `rust-tc doctor` PASS with 372 tests, none skipped, using a fresh TMPDIR on the project's XFS volume. This includes the existing error/crash/short-copy sweeps and real process-kill move test, now with the additional sync boundary, plus doctests, dependencies, features, layers and Rust 1.95. `git diff --check` PASS.
- **Effectiveness checks:** deliberately removing failed-copy cleanup caused 12 test failures, including all four new tests. Deliberately restoring one excess progress byte after failure caused the retry regression alone to fail (13 of 12 bytes instead of 12 of 12). Both mutations were restored before the final full gate; `ops.rs` has no remaining change. The initial no-sync red run also proves that all four new tests detect a missing sync boundary.
- **Limits:** these are injected sync errors and normal kernel sync calls, not a physical power-cut test. The new guarantee covers regular-file content and attributes, not parent-directory entries, symlinks, directory metadata, source-removal ordering or restart cleanup. Cross-filesystem move behaviour is simulated by `FaultFs`; no real user drive was moved. File sync may add background I/O latency; no throughput benchmark or UI timing was claimed. No UI replay was needed because `rustphin-ui` was unchanged.
- **Follow-up:** W-071 interrupted batch-rename recovery; W-072 namespace durability and atomic configuration audit. Neither a complete power-loss guarantee nor daily-driver readiness is claimed.

### 2026-10-20 — W-069 hardening assessment
- **Phase:** 6 Implement (S11)
- **Result:** assessment complete; S11 remains open. No application changes or newly accepted requirements.
- **Checks:** `rust-tc doctor` PASS, 368 tests passed, none skipped. Testing-suite stack detector confirmed the Cargo/nextest setup. Read current requirements, state/open loops, Discovery carry-forwards and architecture; verified the relevant engine and filesystem paths through graph snippets and source, with no recorded index-coverage gaps in the five relied-on core files. Graph call resolution for generic filesystem methods is imprecise; source supplied the actual call order.
- **Safety findings:** `RealFs::copy_file` copies data, applies permissions/time and returns without syncing; `Engine::copy_entry` can then publish it, and a cross-filesystem move can remove its source. Batch rename stages originals under temporary names and repairs ordinary errors, but has no persistent recovery record. The current move kill test does not prove batch-rename recovery or power-loss safety. Atomic config writes, source-removal ordering and namespace durability need a separate audit. W-070 starts with syncing copied regular files; W-071/W-072 retain the wider gaps.
- **Other gates:** W-073 re-measures responsiveness/memory rather than treating old headless benchmarks as present-frame evidence. W-074 retains AccessKit/Orca as unverified. W-075 covers untrusted parsers and names. W-076 covers keyboard, contrast, fallbacks and outstanding UI paths. W-077 covers application privacy and clean builds. W-078 reconciles unchecked acceptance points with existing evidence and the remaining manual hardware/desktop checks. Unchecked boxes are not automatically missing implementations, and checked slice features do not imply complete MVP acceptance.
- **Scope:** existing requirements and locked decisions unchanged; no user files, desktop settings or device state touched. No new fuzz target, dependency or toolkit change.
- **Follow-up:** W-070, followed by the ordered queue in WORK_ITEMS.md. No daily-driver trial until the applicable gates are resolved.

### 2026-10-19 — W-068 desktop acceptance; S10 closed
- **Phase:** 6 Implement (S10)
- **Result:** pass for R-017 and R-018's desktop paths. S10 closed; this is not acceptance for the daily-driver trial.
- **Change:** added replayable scenarios I1–I13, refreshed the README's obsolete S1 status, and queued S11 hardening. No application code or dependencies changed.
- **Checks:** `rust-tc doctor` PASS at baseline and after the session, 368 tests passed, none skipped. The complete gate includes formatting, clippy, doctests, advisories/licences, unused dependencies, feature checks, layers and Rust 1.95. `cargo build --release --package rustphin` and `git diff --check` PASS. No new automated tests; existing behaviour was exercised through the running release build.
- **Environment:** used the installed `kwin-mcp-cli` with scoped `XDG_RUNTIME_DIR=/run/user/1000`, isolated KWin and `isolate_home: true`, 1280×800, generated files only. Additional apps used system-only XDG search directories plus the isolated data/config homes. No device row was activated and no host default handler was changed. AT-SPI did not list Rustphin, so normal screenshots, isolated state, process arguments and fixture contents supplied the evidence. The session was stopped after the checks.
- **Observed:**
  - I1: a filesystem path to Alpha and a `file://` URL ending `Beta%20Space` opened two tabs in that order. The session file held the two decoded paths.
  - I2–I4: right-click selected the note and offered Open, Open with, Cut, Copy, Paste (disabled), Rename, Move to trash, Delete and Properties. The submenu listed the isolated default Fixture Editor first and Fixture Viewer second. Down/Right/Down/Enter chose Viewer; its argument log contained the exact `Alpha/note space.txt` path. Enter on the note then started Fixture Editor with that same path and no run prompt. The test applications only recorded arguments and exited.
  - I5–I7: Alt+Enter showed the note name, Plain text document, 29 B, modified/accessed/created times, `rw-r--r--` and 0644. Group Write changed the display to 0664 and enabled Apply; Enter produced the completion notice and `stat` confirmed 0664. Ctrl+Z restored 0644 on disk. Folder properties reached the expected 100 B. Link properties said Symbolic link, displayed the target, and disabled permission editing with an explanation.
  - I8–I9: activating the harmless program showed Keep, Open and Run. Its fixture marker was absent before and after Escape. Opening it again and confirming Run created the marker. Activating the `.desktop` shortcut showed the application name and Keep/Run, and Escape closed it; confirming that shortcut was not replayed.
  - I10: while Beta Space was active, a separate `gdbus` process on the isolated session bus called `org.freedesktop.FileManager1.ShowItems` with the note URI. It returned `()`. Rustphin opened an Alpha tab and selected `note space.txt`, with status "1 of 5 items selected". The pre-existing Alpha tab remained; requests reuse the current matching location, not every matching tab.
  - I11: a separate `ShowItemProperties` call opened the note's properties with 29 B, times and permissions. Its type was the generic File, unlike the browser route's Plain text document.
  - I12: with Rustphin closed, its unchanged desktop entry was installed only in the isolated home and registered there for `inode/directory`. With the release directory on the child PATH, `xdg-open` on Beta Space launched Rustphin. Screenshot and process arguments both showed that exact folder. The host's handlers were untouched.
  - I13: empty-space right-click showed disabled Paste, New folder and New file. Clicking New file created a zero-byte file and focused its inline name; typing `created.txt` and Enter left that exact name on disk. Shift+F10 opened its selection menu; clicking Properties showed Plain text document and 0 B.
- **Not claimed:** all menu operations re-run (their operation and clipboard checks remain in S4/S5), a full graphical editor session, executing a desktop shortcut, properties at small window sizes, folder-measurement cancellation or unreadable-folder messages on screen, the real Plasma application menu or real-user default registration. Special permissions and operation failures retain the W-067 automated evidence. The D-Bus type-label limitation is carried in STATE.md.
- **Waiver:** none.
- **Follow-up:** W-069 assesses the remaining MVP gates and orders S11 hardening, including the existing data-preservation, accessibility and performance gaps.

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
