# Information Architecture: Rustphin (W-079)

Written 2026-11-08 from [`DESIGN_BRIEF.md`](DESIGN_BRIEF.md) and the current
build (`crates/rustphin-ui`). It describes the structure the design pass keeps
and the few places where it changes. The visual rules are in the target
[`DESIGN.md`](DESIGN.md); keys are in [`KEYBOARD.md`](KEYBOARD.md).

Rustphin is a desktop application, not a web site: its "pages" are one window
made of regions, plus the surfaces that float over it. Where the skill's
template asks for URLs, this document uses locations (the paths and addresses
the location bar shows) and command names.

## Site Map

One window per process; each window holds tabs, each tab one or two panes.

- **Window** `Rustphin`, titled "<folder name> — Rustphin"
  - **Tab strip** (only with two or more tabs)
    - Tab (one per open location; reorderable; "+" adds one)
  - **Toolbar**
    - Back · Forward · Up
    - Location field (path segments; Ctrl+L edits it as text)
    - Actions button (opens the palette)
  - **Notices under the toolbar** (only while there is something to say)
    - Error banner, with "Details…" → *Error details dialog*
    - Trash banner (while a trash is shown) → *Trash actions menu*
    - "Connecting to <address>…" for remote folders
  - **Body**
    - **Places panel** (F9 shows or hides it)
      - Places: Home, Desktop, Documents, Downloads, Pictures, Music, Videos,
        Trash, and any added folder (shared with KDE in `user-places.xbel`)
      - Place groups (Rustphin's own, D-008), each collapsible, with a count
        when folded; a group may hold network bookmarks (`sftp://`, `smb://`…)
      - Devices: mounted and unmounted volumes, with usage and eject
      - Hidden places (only when listed)
    - **Pane** (one, or two side by side in split view)
      - Column headings (details view only): Name, Size, Modified, Type
      - Entry list in one of three views: Details, Icons, Compact
      - States in place of the list: Loading…, This folder is empty,
        This folder has only hidden items, nothing matches the filter,
        could not open this folder
  - **Filter bar** (Ctrl+I or /; above the lines below)
  - **Activity line** (while an operation runs; then the result line with Undo)
  - **Blur status** ("Background blur unavailable; using the opaque theme.")
  - **Status line**: selection and folder totals · free space
- **Floating surfaces** (solid, over a dimmed window when modal)
  - Actions palette (Ctrl+Shift+P)
  - Context menu → "Open with" submenu
  - Trash actions menu
  - Tooltips
  - Dialogs
    - Properties (Alt+Enter)
    - Batch rename (F2 with two or more selected)
    - Name conflict ("… already exists in …")
    - Failed entry (Skip, Try again, Stop)
    - Move or copy? (after a drop without a modifier)
    - Run this file? (executables and `.desktop` files)
    - Empty the trash?
    - Delete for good?
    - Error details (new: what failed and why)

## Navigation Model

- **Primary navigation:** the places panel (where you can go) and the location
  field (where you are). Back, Forward and Up move through the tab's history.
  Double-click or Enter opens a folder in the same pane.
- **Secondary navigation:** tabs (several locations in one window) and split
  view (two locations side by side, F3). A tab remembers its own history, view
  mode and split.
- **Utility navigation:** the Actions palette is the index of everything: every
  command, every place, every open tab and recent folders, searchable by name.
  The context menu offers what applies to the selection, or paste and new
  entries on empty space. There is no menu bar and no settings window: the few
  settings (view mode, hidden files, folders first, zoom, transparency) are
  commands.
- **Mobile navigation:** not applicable (desktop only). The narrow-window
  equivalents are in "Responsive behaviour" below.

Depth: a location is one step away from the places panel or the palette, and
every command is at most two steps away (palette → command, or right-click →
item). R-106 requires this.

## Content Hierarchy

### Main window (the view where people spend almost all their time)

1. **Entry names**: the reason the window exists; highest contrast, largest
   body text.
2. **Selection and focus**: which entries an action will apply to (fill and
   bar).
3. **Where am I**: the location field, the active tab, the current place in
   the panel, and the window title, all saying the same thing.
4. **Entry data**: size, modified date, type, in muted mono, aligned in
   columns.
5. **What is happening**: error banner (above the list, because it changes what
   the user does next), activity line (below, because it runs on its own).
6. **Status line**: counts, sizes, free space; read only when wanted.
7. **Chrome**: navigation buttons and Actions; quiet until used.

### Places panel

1. The current place (subtle fill and bright bar).
2. Places and groups, in the user's order.
3. Devices, with usage numbers.
4. Section titles, quieter than the rows.

### Actions palette

1. The search field (focused on open).
2. The selected result: name, trail ("View › Show hidden files"), shortcut.
3. Other results, at most seven visible.
4. The title row ("Actions", Ctrl+Shift+P), the quietest part.
5. Footer messages (history problems, clear-history confirmation) when present.

### Context menu

Groups, separated by hairlines, in this order (Dolphin's order):

1. Open, Open with ›
2. Cut, Copy, Paste, Copy location, Duplicate here
3. Rename, Move to trash (Restore in the trash), Delete…
4. Properties

On empty space: Paste, New folder, New file. In the trash: Restore and Empty
trash….

### Properties dialog

1. Title row: type icon, "<name> — Properties", close.
2. Header: thumbnail (or type icon), name, type.
3. Facts: location, size (with exact bytes), modified, accessed, created.
4. Permissions: the mode in words and octal, then Owner, Group, Others, each
   Read, Write, Execute.
5. Action row: Close (Esc), Apply (Enter).

### Dialogs that ask a question

1. A short title that is the question ("Delete for good?").
2. The full sentence and facts, scrolling if long.
3. The action row: the safe choice is the default; each button shows its key.

### Error details dialog (new)

1. Title: what the operation was ("Copying 12 items to “Backup”").
2. A summary sentence: how many entries failed and that the rest was done.
3. One row per failed entry: its name, its folder, and the reason in words.
4. Action row: Copy the list, Close (Esc).

## User Flows

### Open a folder and find a file

1. User opens Rustphin; the window shows the last session or the home folder.
2. User clicks a place, types a path after Ctrl+L, or picks a recent folder in
   the palette.
   - If the location does not exist or cannot be read → the list area says so
     in a sentence and the previous folder stays in history.
   - If it is a remote address → "Connecting to <address>…", then the folder,
     or an error banner saying what to do (for example install kio-fuse).
3. The entries appear (Loading… shows until the first entries arrive).
4. User types to jump to a name, or Ctrl+I to filter.
5. User opens the file with Enter or a double-click.
   - If it is an executable or a `.desktop` file → *Run this file?*

### Act on files

1. User selects one or more entries (click, Ctrl/Shift-click, rubber band,
   keyboard).
2. The status line shows "N items selected (size)".
3. User right-clicks, presses Shift+F10 or the Menu key, uses a shortcut, or
   searches the palette.
4. User picks the action.
   - Rename one → the name becomes a field with the stem selected.
   - Rename several → *Batch rename* with a live preview.
   - Move to trash → done at once; the result line offers Undo.
   - Delete… → *Delete for good?*
   - Properties → *Properties*.

### Copy or move between two folders

1. User presses F3 for split view and opens the destination in the other pane.
2. User drags the selection across, or uses Copy/Cut and Paste, or Shift+F5 /
   Shift+F6 (copy or move to the other view).
   - Drop without a modifier → *Move or copy?*
3. After 300 ms the activity line shows progress, counts, speed and Stop.
4. A name already exists → *Name conflict* (Skip, Replace or Merge, Keep both,
   "Do the same for all other conflicts").
5. An entry fails → *Failed entry* (Skip, Try again, Stop).
6. The result line says what was done, with Undo.
   - Some entries failed → error banner with "Details…" → *Error details*.

### Clean up the trash

1. User opens Trash from the places panel or the palette.
2. The trash banner explains how long items stay and offers "Trash actions".
3. User restores a selection (context menu or banner menu), or chooses
   Empty trash… → *Empty the trash?*

### Use a removable drive

1. A drive appears under Devices when plugged in, with its usage.
2. User clicks it: it mounts and opens.
3. User clicks the eject button: it unmounts and the row updates.
   - If it is busy → an error banner names what is using it.

### Find any command

1. User presses Ctrl+Shift+P or clicks Actions.
2. User types part of a name ("hidden", "split", "trash").
3. The best result is selected; Enter runs it; the shortcut is shown so the
   user learns it.

## Naming Conventions

Pick one word and use it everywhere: menu, palette, tooltip, notices.

| Concept | Label in UI | Notes |
|---|---|---|
| A file or folder in the list | entry; "item" in counts | "3 items", never "3 files" when folders may be included |
| Folder | folder | Never "directory" |
| The panel on the left | Places | Dolphin's word; the title of the panel |
| A user-made section in Places | group | "New group", "Move selected place to next group" |
| A volume | device; the section is "Devices" | Its name and mount point: "Data (/data)" |
| Removing a device | Unmount, or Safely remove for removable drives | The eject button's tooltip says which |
| Sending to the trash | Move to trash | Undoable |
| Deleting without the trash | Delete… / "Delete for good?" | Always asks; cannot be undone |
| The trash | Trash | Capital T only as a place name and title |
| The command palette | Actions | The button and the palette title |
| Searching names in the current folder | Filter | "Filter by name"; not "Search" (search is L-001) |
| Files starting with a dot | hidden files | "Show hidden files" |
| Two panes | split view | "Toggle split view" |
| A copy, move, rename, trash or delete in progress | operation | Activity and result lines |
| Undoing the last operation | Undo | "Undid: …" in the result line |
| Several renames at once | batch rename | Dialog title "Rename N items" |
| Network folder | remote folder; network bookmark in Places | Addresses shown without passwords |
| Command names in the palette | "Category: Verb object" internally; shown as name and trail | "Show hidden files", trail "View › Show hidden files" |
| Ellipsis | "…" only when the command asks something before acting | "Delete…", "Empty trash…"; not "Properties…" |
| Capitalisation | Sentence case everywhere | The reference's "Move to Trash" and "Rename…" become "Move to trash" and "Rename" |

## Component Reuse Map

| Component | Used on | Behaviour differences |
|---|---|---|
| Solid floating surface (raised, accent outline, glow) | Palette, context menu, trash actions menu, all dialogs, tooltips | Menus open at the pointer; palette near the top; dialogs centred over the scrim |
| Selection look (primary fill, light text, bright bar) | Entry rows and cells, menu items, palette results, batch-rename mode buttons | Places use the subtle fill instead of the primary fill |
| Focus bar or ring | Rows (bar), fields and buttons (ring) | Rows show the bar without fill when focused but not selected |
| Outlined button | Toolbar buttons, Actions, Stop, Details…, Trash actions, dialog secondary buttons | Error-coloured outline on the error banner |
| Primary button | Default action of a dialog; at most one per view (e.g. "Show hidden" in a state) | Destructive defaults use the error colours |
| Notice row (icon, sentence, actions, dismiss) | Error banner, trash banner, connecting notice, blur status, result line | Error uses the error surface; others use chrome |
| Progress or usage bar | Activity line, device rows | Activity: cyan, 6 px; device: cyan, 4 px |
| Scrolling dialog body | Every dialog | Arrow keys, Page Up/Down and Home/End scroll it |
| State block (icon, heading, reason, one action) | Empty, loading, hidden-only, filter-empty, unreadable folder | Only one action at most |
| Text field | Location (editing), filter, palette search, rename field, batch-rename rules, group names | The rename field sits inside a row |
| Section heading with chevron and count | Places groups, Devices | Folded groups show "(n)" |

## Content Growth Plan

- **Entries:** folders can hold 500,000 entries or more. The list is
  virtualised, never paginated; sorting, filtering and type-to-jump scale with
  it; the status line gives totals. Counts use grouped digits ("500,000 items").
- **Places and groups:** grow with the user; groups fold, and a folded group
  shows its count. The panel scrolls.
- **Devices:** a handful; listed in the order the system reports.
- **Tabs:** grow to the window width; closed tabs can be reopened
  (Ctrl+Shift+T). Proposed for W-079: past the width, tabs shrink to a
  minimum (icon and a few characters) and the strip scrolls.
- **Recent folders and palette history:** bounded, local, clearable (R-108).
- **Operations:** several may run at once; each has its own activity line,
  stacked above the status line.
- **Errors:** an operation with many failures lists them all in the details
  dialog, which scrolls; the banner stays one line.

## URL Strategy

Locations are what the location field shows and accepts:

- **Local paths:** absolute (`/home/alex/Pictures`), with `~` expanded. Names
  are bytes: a name that is not valid UTF-8 is shown with replacement
  characters but opened by its real bytes.
- **Trash:** each volume's trash is a location; the home trash is the
  "Trash" place.
- **Remote:** `sftp://`, `smb://`, `fish://`, `ftp://`, `webdav://`, `nfs://`
  addresses open through kio-fuse; the field and tabs keep the address, never
  the password.
- **Commands:** the palette names commands as "Category: Verb object" with
  categories Files, Navigation, Places, Selection, Trash, View and Workspace;
  the trail shows the same category.
- **Session:** tabs, their locations, view modes and splits are restored at the
  next start.

## Responsive behaviour

Breakpoints are per pane width (see the brief for the table):

- Columns leave in the order Type, Modified, Size; the name always stays.
- The toolbar drops Forward and Up, then Back; they stay in Actions and on
  their keys. Actions becomes an icon button with a tooltip.
- The location collapses its leading folders into "…".
- Dialogs keep their title and action row visible and scroll the body.
- The status line drops the free-space zone first, then truncates its sentence.

## Screens for the mockup

Each screen at the size given, in the glass theme over a photograph unless it
says opaque. Data is generated: user `alex`, folders and files like the
reference's.

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
5. **Single tab:** no tab strip; the result line "Moved 3 items to “Archive”."
   with Undo.
6. **Filter bar open** with "img", status "5 of 12 items match “img”".
7. **Renaming in place:** the rename field on a row, stem selected.
8. **Dragging:** the drag chip "3 items" over a folder that shows the hover fill.

### Split view and narrow sizes

9. **Split view (1280 × 800):** Pictures left, `/data/Backup` right, right pane
   active (bright top line), copying from left to right.
10. **Default window (960 × 640),** details view.
11. **Narrow pane (split at 960 × 640):** each pane about 360 px: Type and
    Modified columns gone, toolbar without Forward and Up.
12. **Minimum window (480 × 320):** places hidden, one pane, Size column only,
    Back and an Actions icon button; then the same with a dialog open.

### Every dialog (1280 × 800, over the dimmed window)

13. Properties of a picture (the reference); Apply enabled after ticking a box.
14. Properties of a folder while "Calculating size…".
15. Batch rename: Replace mode with a preview of five names, one problem line.
16. Batch rename: Pattern mode ("Holiday ###"), Rename disabled because nothing
    would change.
17. Name conflict for a file (Skip, Replace, Keep both, "Do the same for all").
18. Failed entry (Skip, Try again, Stop).
19. Move or copy? after a drop.
20. Run this file? for an executable (Keep, Open, Run).
21. Empty the trash? with its count.
22. Delete for good? for three items.
23. Error details: 2 of 12 entries failed, each with its reason.

### States and opaque mode

24. Loading… (a large folder still listing).
25. This folder is empty.
26. This folder has only hidden items, with "Show hidden".
27. Nothing matches the filter, with "Clear filter".
28. Could not open this folder (permission denied).
29. Trash shown: trash banner with "Trash actions" open.
30. Devices could not be listed (status in the places panel).
31. Places empty: "No places yet…" hint.
32. Connecting to `sftp://alex@nas.local/photos`…
33. **Opaque theme:** screen 1 again with solid surfaces and the status
    "Background blur unavailable; using the opaque theme."
34. **Opaque theme at the minimum window** with a dialog.
