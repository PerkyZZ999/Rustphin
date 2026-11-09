---
version: alpha
name: Rustphin Obsidian (W-079 target)
description: Target design for the W-079 pass. Navy obsidian glass with royal-blue accents for the Rustphin file manager on KDE Plasma Wayland, aligned with docs/mockup.png at a roomier density.
colors:
  # Surfaces over the compositor-blurred desktop (glass mode). Stacked: window, then chrome or content.
  window-glass: "rgba(13, 21, 36, 0.80)"
  chrome-glass: "rgba(18, 27, 45, 0.50)"
  chrome-raised-glass: "rgba(26, 34, 54, 0.55)"
  content-glass: "rgba(14, 23, 39, 0.62)"
  # Solid replacements when blur is unavailable or switched off (opaque mode).
  window-solid: "#0D1524"
  chrome-solid: "#121B2D"
  chrome-raised-solid: "#1A2236"
  content-solid: "#0E1727"
  # Menus, palette, dialogs, tooltips: always solid.
  surface-raised: "#111A2C"
  scrim: "rgba(4, 7, 14, 0.60)"
  # Text.
  text: "#E6E9F2"
  text-muted: "#AEB8CC"
  text-disabled: "#7C869A"
  # Royal blue.
  primary: "#2D4CC8"
  primary-hover: "#2843B4"
  on-primary: "#F5F7FC"
  primary-subtle: "#1D2E6A"
  primary-bright: "#8FA8FF"
  # Status signals.
  secondary: "#3BCFF0"
  tertiary: "#10B981"
  error: "#FF9B9B"
  error-surface: "#37202B"
  error-border: "rgba(255, 155, 155, 0.45)"
  # Lines, tracks and states.
  border: "rgba(255, 255, 255, 0.07)"
  border-strong: "rgba(255, 255, 255, 0.14)"
  border-accent: "rgba(80, 115, 235, 0.55)"
  track: "#333D52"
  hover: "rgba(255, 255, 255, 0.07)"
typography:
  headline-md:
    fontFamily: Space Grotesk
    fontSize: 20px
    fontWeight: 600
    lineHeight: 28px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Space Grotesk
    fontSize: 17px
    fontWeight: 600
    lineHeight: 24px
  body-lg:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: 400
    lineHeight: 20px
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 400
    lineHeight: 20px
  body-strong:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 600
    lineHeight: 20px
  data-md:
    fontFamily: JetBrains Mono
    fontSize: 14px
    fontWeight: 400
    lineHeight: 20px
  data-sm:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: 400
    lineHeight: 16px
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: 500
    lineHeight: 16px
    letterSpacing: 0.06em
rounded:
  sm: 2px
spacing:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 24px
  2xl: 32px
components:
  window-surface:
    backgroundColor: "{colors.window-solid}"
  window-surface-glass:
    backgroundColor: "{colors.window-glass}"
  content-pane:
    backgroundColor: "{colors.content-solid}"
  content-pane-glass:
    backgroundColor: "{colors.content-glass}"
  chrome-glass-surface:
    backgroundColor: "{colors.chrome-glass}"
  tab-strip-glass:
    backgroundColor: "{colors.chrome-raised-glass}"
  modal-scrim:
    backgroundColor: "{colors.scrim}"
  tab-strip:
    backgroundColor: "{colors.chrome-raised-solid}"
    height: 40px
  tab:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    padding: "{spacing.md}"
    width: 168px
  tab-active:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text}"
  toolbar:
    backgroundColor: "{colors.chrome-solid}"
    height: 52px
    padding: "{spacing.sm}"
  toolbar-button:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    size: 36px
  toolbar-button-hover:
    backgroundColor: "{colors.hover}"
  toolbar-button-disabled:
    textColor: "{colors.text-disabled}"
  location-field:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
    rounded: "{rounded.sm}"
    height: 36px
    padding: "{spacing.md}"
  actions-button:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 36px
    padding: "{spacing.md}"
  banner-error:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.error}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 44px
    padding: "{spacing.md}"
  places-panel:
    backgroundColor: "{colors.chrome-solid}"
    width: 240px
  places-title:
    textColor: "{colors.text}"
    typography: "{typography.headline-sm}"
    height: 44px
    padding: "{spacing.lg}"
  place-row:
    textColor: "{colors.text}"
    typography: "{typography.body-lg}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.md}"
  place-row-current:
    backgroundColor: "{colors.primary-subtle}"
    textColor: "{colors.text}"
  section-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-strong}"
    height: 32px
  device-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 56px
  device-figures:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  usage-bar:
    backgroundColor: "{colors.secondary}"
    height: 4px
  usage-track:
    backgroundColor: "{colors.track}"
    height: 4px
  column-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    height: 32px
  column-heading-active:
    textColor: "{colors.text}"
  column-size:
    width: 96px
  column-modified:
    width: 168px
  column-type:
    width: 160px
  entry-row:
    textColor: "{colors.text}"
    typography: "{typography.body-lg}"
    rounded: "{rounded.sm}"
    height: 36px
    padding: "{spacing.sm}"
  entry-icon:
    size: 24px
  entry-row-hover:
    backgroundColor: "{colors.hover}"
  entry-row-selected:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
  entry-data-cell:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-md}"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.lg}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
  button-outlined:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.lg}"
  button-destructive:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.error}"
    rounded: "{rounded.sm}"
    height: 32px
  input:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 36px
    padding: "{spacing.md}"
  checkbox:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.sm}"
    size: 16px
  floating-surface:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
  context-menu:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    width: 240px
    padding: "{spacing.xs}"
  menu-item:
    typography: "{typography.body-md}"
    height: 32px
    padding: "{spacing.sm}"
  menu-item-selected:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
  menu-shortcut:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  menu-item-disabled:
    textColor: "{colors.text-disabled}"
  command-palette:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 640px
    padding: "{spacing.md}"
  palette-result:
    typography: "{typography.body-lg}"
    height: 52px
    padding: "{spacing.md}"
  palette-trail:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  dialog:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 480px
    padding: "{spacing.lg}"
  dialog-title:
    textColor: "{colors.text}"
    typography: "{typography.headline-sm}"
    height: 48px
  properties-thumbnail:
    width: 96px
    height: 64px
  fact-label:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    width: 128px
  fact-value:
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
  permission-group-label:
    textColor: "{colors.text-muted}"
    typography: "{typography.label-sm}"
  rename-dialog:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 640px
    padding: "{spacing.lg}"
  activity-line:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 44px
    padding: "{spacing.md}"
  progress-bar:
    backgroundColor: "{colors.secondary}"
    rounded: "{rounded.sm}"
    height: 6px
  progress-track:
    backgroundColor: "{colors.track}"
    rounded: "{rounded.sm}"
    height: 6px
  activity-figures:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  filter-bar:
    backgroundColor: "{colors.chrome-solid}"
    height: 40px
  status-line:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
    height: 32px
  tooltip:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    padding: "{spacing.sm}"
  drag-chip:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.sm}"
---

# Rustphin design (W-079 target)

This is the **target** design for the W-079 pass, written on 2026-11-08 with
[`DESIGN_BRIEF.md`](DESIGN_BRIEF.md) and
[`INFORMATION_ARCHITECTURE.md`](INFORMATION_ARCHITECTURE.md). The visual
reference is [`../mockup.png`](../mockup.png). Until the pass starts,
[`../DESIGN.md`](../DESIGN.md) remains the record of what the current build
implements (D-017) and `crates/rustphin-ui/src/theme.rs` follows that file. The
pass's first step makes this file the new `docs/DESIGN.md`, then changes the
theme and views to match. Token values here are authoritative for the HTML
mockup; the prose explains how to apply them. Every colour and size marked
"measured" was taken from `docs/mockup.png`; everything else carries over from
the current design or is a decision recorded in the brief.

## Overview

Rustphin is a file manager for KDE Plasma, used every day by people who move
through large folders quickly. The interface is a precise instrument:
geometric, legible and calm, with nothing decorative competing with the files.

Three ideas hold it together:

- **Obsidian glass.** The window is dark navy glass over the desktop, which the
  compositor blurs. Chrome (tabs, toolbar, places, status) is recessed glass;
  the file list sits on a more solid pane so columns of text stay readable over
  anything behind.
- **Royal blue as the only energy.** One accent marks selection, focus, the
  current place and the primary action. Cyan and emerald appear only as status
  signals; salmon only for errors.
- **Progressive disclosure.** The default window shows places, the location,
  the files and a status line. Banners, progress, the filter and the tab strip
  appear when they have something to say; every command is in the Actions
  palette with its shortcut.

The density is comfortable rather than packed: 36-pixel rows and 15-pixel
names at the default zoom (measured: the reference's rows are about 39 px
apart), on a 4-pixel grid, with hairline dividers instead of boxes. Zooming
out reaches 28-pixel rows.

## Colors

- **Glass surfaces** (`window-glass`, `chrome-glass`, `chrome-raised-glass`,
  `content-glass`) are navy obsidian at partial opacity, stacked: window first,
  then chrome or content on top of it. Never put text directly on bare
  `window-glass`; it is the backdrop for the others. The navy tint is measured
  from the reference (`#0D1726` list, `#11192A` places, `#1B2035` tab strip).
- **Solid surfaces** (`window-solid`, `chrome-solid`, `chrome-raised-solid`,
  `content-solid`) replace the glass in opaque mode: when the compositor does
  not advertise blur, loses it, or the user turns transparency off. Every rule
  in this document holds in both modes. When blur is unavailable (not when it
  is switched off), a read-only status line says "Background blur unavailable;
  using the opaque theme."
- **Chrome raised** (`chrome-raised-*`) is the tab strip: one step lighter than
  the toolbar, so the active tab, drawn in the toolbar's own colour, reads as
  joined to it.
- **Raised surface** (`surface-raised`) is for anything that floats inside the
  window: menus, the palette, dialogs, tooltips, text fields. Always solid; the
  toolkit cannot blur inside the window.
- **Text:** `text` for names and primary content; `text-muted` for data
  columns, inactive tabs and headings, shortcuts and the status line;
  `text-disabled` only for disabled controls, which also lose their hover and
  are skipped by the keyboard. `text` and `text-muted` meet 4.5:1 on every
  surface, including glass over a white window (lowest pair: `text-muted` on
  window glass over white, 4.92:1, checked 2026-11-08). `text-disabled` meets
  3:1 on `surface-raised`.
- **Primary** (`primary`, with `on-primary` text; measured selection fill
  `#2744B5`–`#2E4FCC`) fills the selected entry, the selected menu item and
  palette result, checked checkboxes and the one primary button in a view.
  `primary-hover` is its pointer-over state.
- **Primary subtle** (`primary-subtle`, measured `#1D2E6A`) marks the current
  place in the places panel: a location indicator, quieter than a selection.
- **Primary bright** (`primary-bright`) is the accent line on dark surfaces:
  selection and focus bars, focus rings, the active-tab underline, the active
  pane's top line, the current place's bar and the rubber band. It meets 3:1
  against every surface.
- **Secondary** (cyan, measured `#39D2F3`) is live activity and capacity: the
  progress bar and device usage bars, both on `track`. **Tertiary** (emerald)
  marks a completed state. Neither is decoration.
- **Error** text on `error-surface` with an `error-border` outline and a
  warning icon; colour never carries the message alone. Destructive buttons use
  the same three.
- **Borders** are hairlines: `border` between regions, `border-strong` around
  outlined controls (toolbar buttons, location field, Actions, Stop, secondary
  dialog buttons), `border-accent` around floating surfaces.
- **File-type icons** come from the system icon theme (the reference shows
  Breeze's blue folders); they are not tokens.

## Typography

Three families, each with one job:

- **Inter** for names and interface text. `body-lg` (15 px, measured from the
  reference's 12 px cap height scaled to the target) for file names, place
  names, menu items and palette result names; `body-md` (14 px) for tabs,
  buttons, banners, column headings and dialog text; `body-strong` for section
  headings in the places panel ("Devices", group names).
- **JetBrains Mono** for data compared down a column or read character by
  character: `data-md` for sizes, dates, the type column, the location path and
  dialog facts; `data-sm` for the status line, shortcuts, palette trails,
  activity figures and device figures; `label-sm` in capitals for short labels
  (OWNER, GROUP, OTHERS; a SYMLINK tag).
- **Space Grotesk** for the few headings: `headline-sm` for panel and dialog
  titles ("Places", "Actions", "Delete for good?", the name in Properties),
  `headline-md` for empty and error states.

Nothing is smaller than 11 px. Dates are `YYYY-MM-DD HH:MM` in the list and
`YYYY-MM-DD HH:MM:SS` in Properties. Sizes use one decimal and IEC units
as today ("6.3 MiB"; the reference's "MB" reads as MiB); Properties adds the
exact bytes with grouped digits ("6.3 MiB (6,611,968 bytes)"). Folder sizes in
the list show "—" in
`text-muted`.

The three families are installed on the reference system and are not bundled;
the system interface font stands in when one is missing.

## Layout

Everything sits on a 4-pixel grid; use only the `spacing` tokens.

- **Window, top to bottom:** tab strip (40 px, only with two or more tabs),
  toolbar (52 px), the error or trash banner when present (44 px, inset 8 px
  from the window edges), the body (places panel and panes), the filter bar
  (40 px) when open, activity lines (44 px each) while operations run, the blur
  status when blur is unavailable, and the status line (32 px). Hairline
  `border` lines separate them.
- **Tab strip:** tabs 168 px wide with a 16 px type icon, the label and a close
  button; "+" after the last tab. Proposed: past the window width tabs shrink
  to 96 px and the strip scrolls.
- **Toolbar:** Back, Forward and Up as outlined 36 px buttons with 8 px gaps;
  the location field fills the middle; then "Actions ⌄" with the shortcut
  "Ctrl+Shift+P" in `data-sm` beside it, inside one outlined group. Below
  400 px of pane width Forward and Up leave and Actions becomes an icon button;
  below 300 px Back leaves. They remain in Actions and on their keys.
- **Location field:** a 16 px folder icon, then the path in `data-md` as
  clickable segments; leading segments collapse into "…" when space runs out.
- **Places panel:** 240 px on the left (F9 hides it); a 44 px title row
  "Places"; rows 32 px with 20 px icons and a 12 px inset; section headings
  32 px; device rows 56 px. The panel scrolls on its own.
- **Details view:** 32 px heading row; columns name (fills), size (96 px,
  right-aligned), modified (168 px), type (160 px); 8 px side gutter; rows
  36 px with 24 px icons or thumbnails. Below 820 px of pane width the type
  column goes, below 600 px the modified column, below 360 px the size column.
- **Zoom:** five levels with 16, 20, 24, 32 and 48 px icons; rows are the icon
  plus 12 px, at least 28 px (28, 32, 36, 44, 60). The default is the 24 px
  level. Icons view uses 48, 64, 96, 128 and 192 px icons as today.
- **Split view:** two equal panes with a 1 px divider; each applies the column
  rules to its own width.
- **Status line:** left zone: "1 item selected (6.3 MiB)", a `border` divider,
  "12 items (150.4 MiB)", hidden counts and filter matches as today; right zone:
  "Free: 1.8 TiB of 3.6 TiB (Data)" for the volume holding the folder. The right
  zone leaves first at narrow widths.
- **Dialogs:** centred, at least 16 px from every window edge; title row
  48 px; body scrolls; action row right-aligned, wrapping when needed.

## Elevation & Depth

Depth comes from stacked planes and edge light, not soft shadows.

- **Level 0, the desktop:** whatever is behind the window, blurred by the
  compositor.
- **Level 1, window glass and chrome:** `window-glass` with `chrome-glass`
  (toolbar, places, status, activity) and `chrome-raised-glass` (tab strip).
- **Level 2, content pane:** `content-glass` under the file views and the
  location field.
- **Level 3, floating surfaces:** `surface-raised`, solid, outlined with a 1 px
  `border-accent` line and a tight glow (`0 0 16px -2px` of `primary` at about
  35%). Modal dialogs and the palette dim the window with `scrim`.

Blur exists only between level 0 and the window. In opaque mode every glass
token becomes its solid counterpart and levels are told apart by their
borders and tones alone.

## Shapes

One radius: `rounded.sm`, 2 pixels, on rows, cells, buttons, fields, banners,
menus, dialogs, checkboxes, progress bars and the rubber band. The reference's
corners read as 3–4 px at its scale; the 2 px decision (the maintainer, 2026-10-12)
stands unless he changes it. There are no pills and no circles.

## Components

- **Tabs:** inactive tabs `text-muted` on the strip; the hovered tab takes
  `hover`; the active tab takes the toolbar's surface, `text`, and a 2 px
  `primary-bright` underline. Each tab has a close button (tooltip "Close tab
  (Ctrl+W)"). Tabs can be dragged to reorder. With one tab there is no strip.
- **Toolbar buttons:** outlined (`border-strong`), 36 px, 16 px Lucide icons
  (arrow-left, arrow-right, arrow-up); hover fills `hover`; unavailable buttons
  use `text-disabled` and keep their outline. Each has a tooltip with its label
  and key.
- **Location field:** outlined; hover shows `hover`; clicking a segment opens
  it; clicking empty space or Ctrl+L turns it into a text field with the whole
  path selected and a `primary-bright` ring; Ctrl+L again or Escape returns to
  segments.
- **Actions button:** "Actions", a chevron, and the shortcut in `data-sm`
  `text-muted`; opens the palette.
- **Error banner:** under the toolbar, inset 8 px: a warning icon, the sentence
  ("Couldn't copy “IMG_4821.jpg”: Permission denied."), an outlined
  "Details…" button in the error colours, and a close button. The list stays as
  it was. Details opens the error details dialog.
- **Trash banner:** under the toolbar on chrome: the trash icon and name, a
  wrapping sentence on how long items stay, and an outlined "Trash actions"
  button opening a menu: Restore (disabled without a selection), "Empty
  trash…", then other volumes' trash after a separator.
- **Places panel:** the title "Places" in `headline-sm`. Place rows: 20 px icon
  and `body-lg` name; hover fills `hover`; the current place has
  `primary-subtle`, a 2 px `primary-bright` bar on its left edge and
  `primary-bright` icon. Section headings ("Devices", group names) in
  `body-strong` `text-muted` with a chevron that points down when open and right
  when folded; a folded section shows its count ("Network (2)"). Places in a
  group are indented one step. Hidden places, when listed, are at half opacity.
  Network bookmarks use the `server` icon.
- **Device rows:** 56 px: a drive icon, the name and mount point ("Data
  (/data)") in `body-md`, a 4 px usage bar (`secondary` on `track`), and
  "1.8 TiB free of 3.6 TiB" in `data-sm` `text-muted`. Removable volumes have an
  outlined 28 px eject button (tooltip "Safely remove"); unmounted volumes are
  `text-muted`. If devices cannot be listed, the section keeps its heading and
  shows a sentence saying so and what to do.
- **Column headings:** `text-muted`; the sorted column uses `text` and an arrow
  (↑ ascending, ↓ descending), so direction is never colour alone. Clicking a
  heading sorts; clicking again reverses.
- **Entries (rows and cells):** icon or thumbnail, then the name in `body-lg`.
  Hover fills `hover`. Selected: `primary` fill, `on-primary` text, and a 2 px
  `primary-bright` bar on the left edge. Focused but unselected: the bar only.
  Cut entries: half opacity. Symbolic links: italic name, the target's icon and
  type. Data cells in `data-md` `text-muted`; sizes right-aligned; folders show
  "—".
- **Thumbnails:** a picture's content replaces its icon at 24 px and above,
  fitted in the icon's square without cropping. Until ready, or if it cannot be
  made, the type icon stays; nothing flashes.
- **Context menu:** 240 px, 4 px inner padding. Items 32 px: a 16 px Lucide
  icon, the label in `body-md`, the shortcut right-aligned in `data-sm`
  `text-muted` (Enter, Ctrl+X, Ctrl+C, Ctrl+V, F2, Del, Alt+Enter). The item
  under the pointer or keyboard takes the selection look. Disabled items use
  `text-disabled` and are skipped. Separators are hairlines with 4 px above and
  below. "Open with ›" opens a second menu of applications with their icons.
- **Actions palette:** 640 px, near the top of the window, over the scrim. A
  title row with "Actions" in `headline-sm` and "Ctrl+Shift+P" in `data-sm`. A
  search field with a search icon and the placeholder "Search actions, places,
  tabs and recent folders…", ringed in `primary-bright` while focused. Results
  52 px: a 20 px icon, the name in `body-lg`, a trail in `data-sm` ("View ›
  Show hidden files"), the shortcut right-aligned. At most seven results show;
  the list scrolls. The selected result takes the selection look. Empty: "No
  matches. Try another action name or folder path."
- **Dialogs:** 480 px wide (batch rename 640 px), `surface-raised`, accent
  outline and glow, over the scrim. A 48 px title row with an icon, the title in
  `headline-sm` and a close button; the body scrolls between title and actions;
  the action row is right-aligned and wraps. The safe default is the
  `button-primary`; a destructive default uses `button-destructive`; the
  others are `button-outlined`. Every button shows its key ("Close (Esc)",
  "Apply (Enter)"). A disabled primary button keeps its fill at half opacity
  with `text-disabled` text.
- **Properties:** title row "<name> — Properties". Header: a 96 × 64 thumbnail
  (or the 48 px type icon), the name in `headline-sm` and the type in
  `data-md`. Facts as label (`text-muted`, 128 px column) and value
  (`data-md`): Location, Size (with exact bytes), Modified, Accessed, Created
  when known. A folder's size reads "Calculating size…" until known. A
  hairline, then "Permissions" with the mode in words and octal
  ("rw-r--r-- (0644)"), then three columns OWNER, GROUP, OTHERS in `label-sm`,
  each with Read, Write and Execute checkboxes, separated by hairlines. Apply
  stays disabled until a box changes.
- **Batch rename:** three labelled modes (Replace, Pattern, Case) as a
  segmented row, the chosen one in `primary`; the rule fields; the preview as
  two `data-md` columns, old and new name, in the same scrolling body; a
  problem as a sentence in the error colour above the preview. Rename stays
  disabled while there is a problem or nothing would change.
- **Question dialogs** (name conflict, failed entry, move or copy, run, empty
  trash, delete for good): the question is the title; the full sentence and
  facts (`data-md`, labels in a 72 px `text-muted` column) scroll in the body.
- **Error details dialog:** title says what the operation was; a summary
  sentence; one row per failed entry with the name (`body-md`), its folder
  (`data-sm` `text-muted`) and the reason in words; "Copy list" (outlined) and
  "Close (Esc)" (primary).
- **Checkboxes:** 16 px, `border-strong` outline; checked: `primary` fill with
  an `on-primary` check; focus: `primary-bright` ring; the label is clickable.
- **Activity line:** an operation icon, the sentence ("Copying 12 items to
  “/data/Backup/Pictures”…") in `body-md`, a flexible 6 px progress bar
  (`secondary` on `track`), the figures as today ("6 of 12 · 742.0 MiB of 1.2 GiB ·
  48.6 MiB/s")
  in `data-sm` `text-muted`, and an outlined "Stop". It appears after 300 ms.
  The result line that follows has the sentence and an Undo button and leaves
  after 6 seconds, unless it reports a problem, which uses the error colours
  and stays until dismissed.
- **Filter bar:** a 16 px filter icon, a field labelled "Filter by name", and a
  close button (tooltip "Close the filter (Esc)").
- **Status line:** two zones as in Layout, in `data-sm` `text-muted`.
- **Empty and loading states:** a 32 px icon, a `headline-md` line saying what
  is the case, a `text-muted` line saying why, and at most one action. Centred
  when it fits; otherwise a bounded scroller with a 16 px gutter.
- **Rename field:** `surface-raised` with `text` in place of the name, the stem
  selected, ringed in `primary-bright`.
- **Tooltips:** `surface-raised`, `body-md`, the label then the key in
  `data-sm` `text-muted`, after 500 ms of hover or on keyboard focus.
- **Dragging:** a `drag-chip` naming the entry or counting ("3 items"); a folder
  under the pointer takes `hover`.
- **Rubber band:** a 1 px `primary-bright` outline with a `hover` fill.
- **Split panes:** the pane taking keyboard input has a 2 px `primary-bright`
  line along its top, drawn only while split.
- **Scrollbars:** 8 px, `text-muted` thumb at half opacity on a transparent
  track; `primary-bright` while hovered or dragged.

## Do's and Don'ts

- Do take every colour, size, spacing and radius from the tokens; add a token
  before adding a literal.
- Do check every new surface or text pair in both glass (over white) and opaque
  modes, and add it to the contrast tests in `theme.rs`.
- Do pair colour with a shape, icon or text for every state: selection,
  current place, focus, sort direction, errors, disabled items, usage.
- Do give every icon-only control a tooltip with its label and key, and every
  command a keyboard or palette path.
- Do use the same label for a command in the menu, the palette and the tooltip.
- Do use JetBrains Mono only for data that is aligned or read exactly.
- Don't blur, or rely on blurring, anything inside the window.
- Don't introduce a second accent, gradients, or soft drop shadows.
- Don't put text on bare `window-glass`, and don't go below 11 px.
- Don't add a radius other than 2 px.
- Don't use `primary` fills for location (that is `primary-subtle`) or for
  decoration.
- Don't follow the system colour scheme; do use the system icon theme for file
  types.

## Changes from docs/DESIGN.md

What this target changes from the design the current build implements:

- **Density:** rows 28 → 36 px at the default zoom (24 px icons), names 14 →
  15 px, data 13 → 14 px, headline-sm 16 → 17 px; toolbar 44 → 52 px; tab
  strip 32 → 40 px; status line 28 → 32 px; filter bar 36 → 40 px; places
  panel 220 → 240 px with 32 px rows; device rows 48 → 56 px; dialogs 440 →
  480 px; column widths size 88 → 96, modified 144 → 168, type 184 → 160 px;
  column breakpoints 760/560/320 → 820/600/360 px; toolbar breakpoints
  320/240 → 400/300 px.
- **Colour:** neutral obsidian → navy obsidian (`#0B0F17` → `#0D1524`); glass
  opacities raised for chrome (0.20 → 0.50) to match the reference; selection
  `#3457D5` → `#2D4CC8`; new `primary-subtle`, `chrome-raised-*`,
  `text-disabled`, `error-border` and `track`; `text-muted` `#C3CAD9` →
  `#AEB8CC` (still AA everywhere).
- **New typography roles:** `body-lg`, `body-strong`; the type column moves to
  mono.
- **Components:** outlined toolbar buttons and location field with a folder
  icon; Actions with the shortcut beside it; the places title and collapsible
  sections with counts; the current place in `primary-subtle`; thumbnails in
  every view from 24 px; "—" for folder sizes; menu icons and shortcuts;
  palette title row, icons and trails; Properties title row, thumbnail header,
  location, exact bytes and dates with seconds; the error banner's "Details…" and the new error
  details dialog; the two-zone status line with free space; a thicker, flexible
  progress bar with mono figures and an outlined Stop.
- **Unchanged:** the three font families, the 2 px radius, the 4-pixel grid,
  opaque fallback behaviour, every wording rule, the keyboard model, and the
  rule that menus, palette and dialogs are solid.

## Status

Written for the HTML mockup on 2026-11-08; not implemented. Lint with
`npx @google/design.md lint docs/project/DESIGN.md`. The token contrast was
checked with a script that mirrors the tests in `theme.rs` (lowest text pair
4.92:1, lowest indicator pair: `primary-bright` on window glass over white,
4.31:1); the tests themselves are updated when W-079 adopts these tokens.
The linter reports no errors and four warnings: `border`, `border-strong`,
`border-accent` and `error-border` are used as outline colours, which the
format's component properties cannot express; the prose above says where.
