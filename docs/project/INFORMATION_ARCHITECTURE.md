# Information Architecture: Rustphin (W-079)

Written 2026-11-08 from [`DESIGN_BRIEF.md`](DESIGN_BRIEF.md) for the W-079
design pass; brought in line with the Qt 6 and QML interface as built
(`crates/rustphin/qml/`, W-114 onwards) on 2026-11-14. The visual rules are
in [`../DESIGN.md`](../DESIGN.md); the screens are drawn in
[`../mockup/`](../mockup/); keys are in [`KEYBOARD.md`](KEYBOARD.md) and,
for the build, in `crates/rustphin-view/src/commands.rs`.

Rustphin is a desktop application, not a web site: its "pages" are windows
made of regions, plus the surfaces that float over them. Where the skill's
template asks for URLs, this document uses locations (the paths and addresses
the location field shows) and command names.

## Site Map

One Rustphin per session (W-110, D-031): it owns `org.rustphin.Rustphin` and
can keep running with no window. Each window holds tabs, each tab one pane or
two side by side. A later launch, or another application asking for a folder,
opens a tab or a window as Settings, General says.

- **Window** (`Main.qml`), titled "<folder name> — Rustphin"
  - **Title bar** (`TitleBar.qml`): logo, title, Minimise, Maximise or
    Restore, Close window
  - **Toolbar** (`Toolbar.qml`; items and order from Settings, Toolbar)
    - Act as administrator
    - Back · Forward · Up one folder · Reload
    - Location field (crumbs; Ctrl+L, Alt+D or F6 edits it as text)
    - New › *New menu* · Open terminal here
    - View › *View menu* · Split view · Show hidden files · Filter by name
    - any palette command added in Settings
    - Settings (opens Settings) and Command palette (opens the palette),
      two icon buttons (W-154)
  - **Administrator bar** (only while acting as administrator), with "Stop
    acting as administrator"
  - **Body**
    - **Places panel** (`PlacesPanel.qml`; F9 shows or hides it)
      - Title "Places" with New group
      - Places: Home, Desktop, Documents, Downloads, Pictures, Music, Videos,
        Trash, and any added folder (shared with KDE in `user-places.xbel`)
      - Place groups (Rustphin's own, D-008), each foldable, with a count when
        folded; a group may hold network bookmarks (`sftp://`, `smb://`…)
      - Remote: network bookmarks outside any group, foldable
      - Devices: a card per mounted or unmounted volume (name, used and
        total, usage bar, filesystem and traits, percent used), foldable
        → *Drive menu*
      - Hidden places (only when listed)
      - → *Places menu*
    - **Tab strip** (`TabStrip.qml`, always shown): one tab per location,
      reorderable by dragging; "+" adds one
    - **Pane** (`Pane.qml`; one, or two side by side in split view)
      - Re-read banner (a folder that failed to re-read keeps its listing)
      - Trash banner (while a trash is shown) → *Trash actions menu*
      - Path bar (split view only)
      - Column headings (details view only): Name, Size, Modified, Type
      - Entry list in one of three views: Icons, Compact, Details
      - States in place of the list: Loading…, This folder is empty, This
        folder has only hidden items, Nothing matches "…", a folder that
        cannot be opened
      - Filter bar (Ctrl+I or /)
      - Status line: selection · folder totals · free space
      - → *Context menu*
  - **Window lines** (`NoticeBar.qml`, only while there is something to say)
    - Activity line while an operation runs, with Details (→ *Transfer
      dialog*) and Stop
    - Notice line after one ends (Undo, Details → *Problems dialog*) or while
      "Connecting to <address>…"
- **Floating surfaces** (solid; the palette and dialogs over a dimmed window)
  - Actions palette (Ctrl+Shift+P)
  - Context menu (`JsonMenu.qml`) → Open with ›, Extract ›, Compress ›
  - New menu, Trash actions menu, Places menu (→ Move to group ›), Drive menu
  - View menu (`ViewMenu.qml`): modes and size
  - Tooltips
  - Dialogs
    - Settings (Ctrl+Shift+,): General, Appearance, Views, Toolbar, Places,
      Confirmations, Shortcuts, About
    - Properties (Alt+Enter)
    - Batch rename (F2 with two or more selected)
    - Compress to…
    - Extract to… (the desktop's folder dialog, starting in the folder shown)
    - Questions (`QuestionDialog.qml`): name conflict, failed entry, archive
      password (and wrong password), Move or copy?, Run "…"?, Move to the
      trash? (when Settings asks), Delete for good?, Empty the trash?, Close
      the window? (several tabs, when Settings asks)
    - Problems (what an operation left out, and why)
    - Transfers (every running operation: progress, speed, time remaining,
      the current entry, a speed graph, Stop)

## Navigation Model

- **Primary navigation:** the places panel (where you can go) and the location
  field (where you are). Back, Forward and Up move through the pane's history.
  Double-click or Enter opens a folder in the same pane (a single click when
  Settings or KDE says so); the middle button opens it in a new tab.
- **Secondary navigation:** tabs (several locations in one window) and split
  view (two locations side by side, F3). A tab remembers each pane's history
  and its split; a folder's view can be remembered per folder (Settings,
  Views). Tab moves the keyboard between the views and the places panel.
- **Utility navigation:** the Actions palette is the index of everything:
  every command, every place, every open tab and recent folders, searchable
  by name. The context menu offers what applies to the selection, or paste
  and new entries on empty space. There is no menu bar. Settings is a dialog
  (Ctrl+Shift+, or "Workspace: Settings…"); the view mode, hidden files,
  folders first and zoom are also commands, the View menu and toolbar
  toggles.
- **Mobile navigation:** not applicable (desktop only). The narrow-window
  equivalents are in "Responsive behaviour" below.

Depth: a location is one step away from the places panel or the palette, and
every command is at most two steps away (palette → command, or right-click →
item). R-106 requires this.

## Content Hierarchy

### Main window (the view where people spend almost all their time)

1. **Entry names**: the reason the window exists; highest contrast.
2. **Selection and focus**: which entries an action will apply to (fill,
   bar and outline).
3. **Where am I**: the location field, the active tab, the current place in
   the panel, and the window title, all saying the same thing.
4. **Entry data**: size, modified date, type, muted, aligned in columns.
5. **What is happening**: the re-read banner (above the list, because it
   changes what the user does next), the activity and notice lines (at the
   bottom, because they run on their own).
6. **Status line**: counts, sizes, free space; read only when wanted.
7. **Chrome**: navigation buttons and Actions; quiet until used.

### Places panel

1. The current place (selection fill and royal-blue edge).
2. Places and groups, in the user's order.
3. Remote places and devices, with usage figures.
4. Section titles, in small capitals, quieter than the rows.

### Actions palette

1. The search field (focused on open).
2. The selected result: icon, name, trail ("View › Show hidden files"), key.
3. Other results, at most seven visible.
4. A status line when nothing matches or history needs attention.

### Context menu

Groups, separated by hairlines, in this order (Dolphin's order):

1. Open, Open with ›
2. Cut, Copy, Paste, Copy location, Duplicate here
3. Rename, Move to trash (Restore in the trash), Delete…
4. Extract › (archives) and Compress › (anything but one archive), not in the
   trash or as administrator
5. Properties

On empty space: Paste, New folder, New file. The trash's own actions (Restore,
Empty trash…, other volumes' trashes) are on the trash banner's menu.

### Drive menu (W-148)

On a drive card in the places panel (right-click, Menu key, Shift+F10), in
groups separated by hairlines:

1. Open, Open in new tab (an unmounted drive is mounted first)
2. Mount (not mounted) or Unmount (mounted); Safely remove (removable or
   power-off capable drives). Neither Unmount nor Safely remove for the
   volumes the system and the home folder run from.
3. Copy location (mounted)
4. Properties (W-153)

### Drive Properties dialog (W-153)

1. Title row: "<name> — Properties", close.
2. Header: drive icon, name, filesystem and traits.
3. A ring of used and free space with its legend (Used, Free, Total), or the
   sentence saying why there is none.
4. Facts: mounted at, size when not mounted, device, filesystem, UUID,
   access, encrypted, drive, kind, connection, removable (only those known).
5. Action row: Close (Esc).

### Properties dialog

1. Title row: "<name> — Properties", close.
2. Header: thumbnail (or type icon), name, type.
3. Facts: location, size (with exact bytes), modified, accessed, created, a
   link's target.
4. Permissions: the mode in words and octal, then Owner, Group, Others, each
   Read, Write, Execute.
5. Action row: Close (Esc), Apply (Enter).

### Dialogs that ask a question

1. A short title that is the question or the fact ("Delete for good?").
2. The sentence, the facts and the names it is about.
3. The action row: the safe choice is the default; each button shows its key.

### Problems dialog

1. Title: what the operation was, then "what was left out".
2. One row per entry left out: its path and the reason in words.
3. Close.

### Transfer dialog

1. Title: "Running operations".
2. One card per operation, oldest first: what it is doing and its Stop; its
   progress bar; items and bytes done, speed and time remaining; the entry
   being worked on now; the speed over the last minute.
3. Close. The dialog also closes by itself when the last operation ends.

### Settings

1. The page list on the left, the current page filled.
2. The page's title and one sentence.
3. Groups of settings, each a row with its label and control; hints under
   them.
4. "Changes apply at once." and Reset to defaults, on pages with settings.

## User Flows

### Open a folder and find a file

1. User opens Rustphin; the window shows the last tabs, the home folder or a
   chosen folder (Settings, General).
2. User clicks a place, types a path after Ctrl+L, or picks a recent folder in
   the palette.
   - If the location does not exist or cannot be read → the list area says so
     in place, with Back or Up and Home folder.
   - If it is a remote address → "Connecting to <address>…", then the folder,
     or a notice saying what to do (for example install kio-fuse).
3. The entries appear (Loading… shows until the first entries arrive).
4. User types to jump to a name, or Ctrl+I to filter.
5. User opens the file with Enter or a double-click.
   - If it is an executable or a `.desktop` file → *Run "…"?* (unless
     Settings, Confirmations says otherwise).

### Act on files

1. User selects one or more entries (click, Ctrl/Shift-click, rubber band,
   keyboard).
2. The status line shows "N items selected (size)".
3. User right-clicks, presses Shift+F10 or the Menu key, uses a key, or
   searches the palette.
4. User picks the action.
   - Rename one → the name becomes a field with the stem selected.
   - Rename several → *Batch rename* with a live preview.
   - Move to trash → done at once (or after *Move to the trash?*); the
     notice line offers Undo.
   - Delete… → *Delete for good?*
   - Extract or Compress → done with the name shown, or *Compress to…* /
     *Extract to…*.
   - Properties → *Properties*.

### Copy or move between two folders

1. User presses F3 for split view and opens the destination in the other pane.
2. User drags the selection across, or uses Copy/Cut and Paste, or Shift+F5 /
   Shift+F6 (copy or move to the other view).
   - Drop without a modifier → *Move or copy?*
3. After 300 ms the activity line shows progress, counts, speed, the time
   remaining, Details and Stop.
   - Details, or "Files: Show running operations" in the palette →
     *Transfer dialog*: every running operation on its own card, each with
     its own Stop.
4. A name already exists → *Name conflict* (Skip, Replace or Merge, Keep both,
   "Do the same for all other conflicts").
5. An entry fails → *Failed entry* (Skip, Try again, Stop).
6. The notice line says what was done, with Undo.
   - Some entries were left out → the notice offers Details, or "Files: Show
     what the last operation left out" in the palette → *Problems*.

### Clean up the trash

1. User opens Trash from the places panel or the palette.
2. The trash banner says items stay until the trash is emptied and offers
   "Trash actions".
3. User restores a selection (context menu or banner menu), or chooses
   Empty trash… → *Empty the trash?*

### Use a removable drive

1. A drive appears under Devices when plugged in, with its usage.
2. User clicks it: it mounts and opens.
3. User right-clicks it (or presses the Menu key or Shift+F10 on it) and
   chooses Unmount or Safely remove: it unmounts
   (and a removable drive powers off) and the card updates.
   - If it is busy → a notice names the problem.

### Find any command

1. User presses Ctrl+Shift+P or clicks the Command palette button.
2. User types part of a name ("hidden", "split", "trash").
3. The best result is selected; Enter runs it; the key is shown so the user
   learns it.

### Change a setting

1. User presses Ctrl+Shift+, or runs "Workspace: Settings…".
2. User picks a page (or Up and Down in the list) and changes a control; the
   change applies at once.
3. Escape or Close returns to the files.

## Naming Conventions

Pick one word and use it everywhere: menu, palette, tooltip, notices.

| Concept | Label in UI | Notes |
|---|---|---|
| A file or folder in the list | entry; "item" in counts | "3 items", never "3 files" when folders may be included |
| Folder | folder | Never "directory" |
| The panel on the left | Places | Dolphin's word; the title of the panel |
| A user-made section in Places | group | "New group", "Move selected place to next group" |
| A volume | device or drive; the section is "Devices" | Its name, "used / total", "NN% used", its filesystem |
| Removing a device | Unmount, or Safely remove for removable or power-off capable drives | Drive menu only (no key) |
| Sending to the trash | Move to trash | Undoable |
| Deleting without the trash | Delete… / "Delete for good?" | Always asks; cannot be undone |
| The trash | Trash | Capital T only as a place name and title |
| The command palette | Command palette | The toolbar button (W-154, before it "Actions"); the palette itself has no title row |
| Searching names in the current folder | Filter | "Filter by name"; not "Search" (search is L-001) |
| Files starting with a dot | hidden files | "Show hidden files" |
| Two panes | split view | "Toggle split view" |
| A copy, move, rename, trash, delete, extract or compress in progress | operation | Activity and notice lines |
| Undoing the last operation | Undo | "Undo (Ctrl+Z)" on the notice line |
| Several renames at once | batch rename | |
| Network folder | remote folder; network bookmark in Places | Addresses shown without passwords |
| Command names | "Category: Verb object" in `commands.rs`; the palette shows the name and the category as a trail | Categories: Navigation, View, Files, Trash, Selection, Workspace, Places |
| Ellipsis | "…" only when the command asks something before acting | "Delete…", "Empty trash…", "Compress to…"; not "Properties…" |
| Capitalisation | Sentence case everywhere | The reference's "Move to Trash" and "Rename…" become "Move to trash" and "Rename" |

## Component Reuse Map

| Component | Used on | Behaviour differences |
|---|---|---|
| Solid floating surface (raised, `border-strong` outline) | Palette, context menu and its submenus, New, trash and places menus, View menu, all dialogs | Menus open at the pointer or under their button; the palette under the toolbar; dialogs centred over the scrim |
| Selection look (neutral `selection` fill, light text, 2 px royal-blue edge) | Entry rows and cells, the current place, palette results, the current Settings page, dialog segments | The edge is on the leading side (segments: along the bottom); menus use the hover fill instead (D-038) |
| Focus outline | Rows (1 px `primary-bright` outline, with the `current` fill when not selected), places rows (outline), fields and buttons (2 px ring) | |
| Icon button | Title bar, toolbar, tab close and "+", New group, dismiss, Stop, Settings steppers and lists | Toggles show the subtle fill when on |
| Outlined button | Trash actions, Undo, Details, state actions, Stop acting as administrator, dialog secondary buttons | |
| Default button | Default action of a dialog (Apply, Rename, Compress, a question's default) | Destructive defaults use the error colours |
| Window line (icon, sentence, actions, dismiss) | Activity line, notice line | A problem uses the error surface |
| Progress or usage bar | Activity line, transfer cards, drive cards | Both `progress` (lightened royal blue) on `track`: activity 6 px; drive card 4 px, `error` from 90 % used (D-039) |
| Scrolling dialog body | Every dialog | |
| State block (icon, heading, reason, actions) | Empty, loading, hidden-only, filter-empty, unreadable folder | |
| Text field | Location (editing), filter, palette search, rename field, batch-rename rules, Compress name, group names, Settings fields | The rename field sits inside a row |
| Section heading with chevron and count | Places groups, Remote, Devices | Folded headings show "(n)" |

## Content Growth Plan

- **Entries:** folders can hold 500,000 entries or more. The list is
  virtualised, never paginated; sorting, filtering and type-to-jump scale with
  it; the status line gives totals. Counts use grouped digits ("500,000 items").
- **Places and groups:** grow with the user; groups fold, and a folded group
  shows its count. The panel scrolls.
- **Devices:** a handful; listed in the order the system reports.
- **Tabs:** shrink evenly from 160 to 96 px, then the strip scrolls and keeps
  the active tab in view; the last six closed tabs can be reopened
  (Ctrl+Shift+T).
- **Recent folders and palette history:** bounded, local, clearable from the
  palette (R-108).
- **Operations:** several may run at once; the activity line shows the
  oldest with "(and N more)", and the transfer dialog shows them all.
- **Problems:** an operation that left many entries out lists them all in
  the problems dialog, which scrolls; the notice stays one line.

## URL Strategy

Locations are what the location field shows and accepts:

- **Local paths:** absolute (`/home/alex/Pictures`), with `~` expanded. Names
  are bytes: a name that is not valid UTF-8 is shown with replacement
  characters but opened by its real bytes (the interface passes folders to
  QML as `file:` URIs, W-132).
- **Trash:** each volume's trash is a location; the home trash is the
  "Trash" place.
- **Remote:** `sftp://`, `smb://`, `fish://`, `ftp://`, `webdav://`, `nfs://`
  and other KIO addresses open through kio-fuse; the field and tabs keep the
  address, never the password. A malformed `scheme://` address is refused.
- **Commands:** `commands.rs` names each command by a stable id
  (`toggle-hidden`) and a label "Category: Verb object"; the palette shows the
  same category in the trail, and the toolbar saves commands by id.
- **Session:** tabs, their panes, locations and splits are restored at the
  next start when Settings, General says so.

## Responsive behaviour

Breakpoints are per pane or window width:

- Columns leave in the order Type (below 820 px), Modified (600 px), Size
  (360 px); the name always stays.
- Items leave the toolbar from the right end inwards; they stay in the
  palette and on their keys. The location field and the Command palette
  button always stay.
- The location shows at most four crumbs (fewer in narrower windows) and
  collapses the leading ones into "…".
- Tabs shrink, then scroll; below 128 px only the active tab and the hovered
  one show a close button.
- Dialogs keep their title and action row visible and scroll the body.
- The status line drops the free space below 600 px, and below 440 px shows
  only the selection while there is one.
- The window cannot be made smaller than 480 × 320 px.

## Screens for the mockup

The W-079 mockup list. Each screen at the size given, in the glass theme over
a photograph unless it says opaque. Data is generated: user `alex`, folders
and files like the reference's. Screens 1–34 were drawn for W-079; 35 (the
top bar redesign, W-096), 36 (Settings, W-106) and 37 (Compress to…, W-111)
were added later. Several screens show what the build has since changed:
the tab strip is always shown (5), the error banner has no Details… and
problems are listed from the notice line (1, 23), and no blur status line is
drawn (33); [`../DESIGN.md`](../DESIGN.md) says what is built.

### Main window, all three views (1280 × 800)

1. **Details view, the reference scene:** three tabs (Home, Pictures,
   Downloads), Pictures active; error banner "Couldn't copy “IMG_4821.jpg”:
   Permission denied." with Details…; places with Pictures current, Devices
   open (System, Data with eject), a folded "Network (2)" group; twelve entries
   with thumbnails, IMG_4821.jpg selected; context menu open on it; activity
   line copying 12 items; status line with selection, totals and free space.
2. **Icons view:** same folder, no menu, two entries selected, one hovered,
   a rubber band in progress.
3. **Compact view:** a folder of mixed documents, one entry focused but not
   selected, one cut (half opacity), one symbolic link (italic).
4. **Actions palette open** over the details view, query "hid", the first
   result selected.
5. **Single tab:** the result line "Moved 3 items to “Archive”." with Undo.
6. **Filter bar open** with "img", status "5 of 12 items match “img”".
7. **Renaming in place:** the rename field on a row, stem selected.
8. **Dragging:** three items over a folder that shows the hover fill.

### Split view and narrow sizes

9. **Split view (1280 × 800):** Pictures left, `/data/Backup` right, right pane
   active, copying from left to right.
10. **Default window (960 × 640),** details view.
11. **Narrow pane (split at 960 × 640):** each pane about 360 px: Type and
    Modified columns gone, a shorter toolbar.
12. **Minimum window (480 × 320):** places hidden, one pane, Size column only,
    Actions as an icon button; then the same with a dialog open.

### Every dialog (1280 × 800, over the dimmed window)

13. Properties of a picture (the reference); Apply enabled after ticking a box.
14. Properties of a folder while "Calculating size…".
15. Batch rename: Replace mode with a preview of five names, one problem line.
16. Batch rename: Pattern mode ("Holiday ###"), Rename disabled because nothing
    would change.
17. Name conflict for a file (Skip, Replace, Keep both, "Do the same for all").
18. Failed entry (Skip, Try again, Stop).
19. Move or copy? after a drop.
20. Run this file? for an executable (Cancel, Open, Run).
21. Empty the trash? with its count.
22. Delete for good? for three items.
23. Error details: 2 of 12 entries failed, each with its reason (built as the
    problems dialog).

### States and opaque mode

24. Loading… (a large folder still listing).
25. This folder is empty.
26. This folder has only hidden items, with "Show hidden".
27. Nothing matches the filter, with "Clear filter".
28. Could not open this folder (permission denied).
29. Trash shown: trash banner with "Trash actions" open.
30. Devices could not be listed (note in the places panel).
31. Places empty: "No places yet…" hint.
32. Connecting to `sftp://alex@nas.local/photos`…
33. **Opaque theme:** screen 1 again with solid surfaces.
34. **Opaque theme at the minimum window** with a dialog.

### Later screens

35. **Top bar redesign (W-096):** the window's own title bar, one toolbar
    across the window, the tab strip over the panes, path bars in split view,
    the administrator bar.
36. **Settings (W-106):** one scene per page and a narrow window.
37. **Compress to… (W-111).**
