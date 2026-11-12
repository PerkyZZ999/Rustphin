---
version: alpha
name: Rustphin Obsidian
description: Navy obsidian glass with royal-blue accents for the Rustphin file manager on KDE Plasma Wayland (GPUI). Adopted by the W-079 design pass from the HTML mockups in docs/mockup/.
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
    fontSize: 15px
    fontWeight: 600
    lineHeight: 20px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Space Grotesk
    fontSize: 15px
    fontWeight: 600
    lineHeight: 20px
  body-lg:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 400
    lineHeight: 16px
  body-md:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 400
    lineHeight: 16px
  body-strong:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 600
    lineHeight: 16px
  data-md:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: 400
    lineHeight: 16px
  data-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: 400
    lineHeight: 16px
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: 500
    lineHeight: 14px
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
  title-bar:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    height: 36px
  window-button:
    textColor: "{colors.text-muted}"
    width: 44px
    height: 36px
  tab-strip:
    backgroundColor: "{colors.chrome-raised-solid}"
    height: 36px
  tab:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    padding: "{spacing.md}"
    width: 160px
  tab-narrowest:
    width: 96px
  tab-active:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text}"
  tab-close:
    textColor: "{colors.text-muted}"
    rounded: "{rounded.sm}"
    size: 24px
  tab-add:
    textColor: "{colors.text-muted}"
    width: 36px
  toolbar:
    backgroundColor: "{colors.chrome-solid}"
    height: 44px
    padding: "{spacing.sm}"
  toolbar-button:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    size: 32px
  toolbar-toggle-on:
    backgroundColor: "{colors.primary-subtle}"
    textColor: "{colors.primary-bright}"
  toolbar-button-hover:
    backgroundColor: "{colors.hover}"
  toolbar-button-disabled:
    textColor: "{colors.text-disabled}"
  toolbar-shortcut-hint-min:
    width: 1000px
  toolbar-tools-min:
    width: 760px
  toolbar-full-navigation-min:
    width: 560px
  toolbar-back-navigation-min:
    width: 300px
  location-field:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.md}"
  path-bar:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.data-md}"
    height: 28px
    padding: "{spacing.md}"
  actions-button:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
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
    height: 36px
    padding: "{spacing.md}"
  place-row:
    textColor: "{colors.text}"
    typography: "{typography.body-lg}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.md}"
  place-row-current:
    backgroundColor: "{colors.primary-subtle}"
    textColor: "{colors.text}"
  section-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-strong}"
    height: 28px
  device-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 48px
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
    height: 28px
  column-heading-active:
    textColor: "{colors.text}"
  column-size:
    width: 96px
  column-modified:
    width: 168px
  column-type:
    width: 160px
  details-type-column-min:
    width: 820px
  details-modified-column-min:
    width: 600px
  details-size-column-min:
    width: 360px
  entry-row:
    textColor: "{colors.text}"
    typography: "{typography.body-lg}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.sm}"
  entry-icon:
    size: 24px
  entry-thumbnail:
    width: 24px
    height: 16px
  icon-cell:
    textColor: "{colors.text}"
    typography: "{typography.body-lg}"
    rounded: "{rounded.sm}"
    width: 128px
    height: 152px
    padding: "{spacing.sm}"
  icon-cell-thumbnail:
    width: 96px
    height: 64px
  compact-column:
    width: 240px
  rename-field:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.xs}"
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
  note-neutral:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    padding: "{spacing.sm}"
  checkbox:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.sm}"
    size: 16px
  checkbox-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 28px
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
  result-line:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 44px
    padding: "{spacing.md}"
  result-line-success:
    textColor: "{colors.tertiary}"
  result-line-problem:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.error}"
  blur-status:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    height: 32px
    padding: "{spacing.md}"
  filter-bar:
    backgroundColor: "{colors.chrome-solid}"
    height: 40px
  filter-field:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.sm}"
  status-line:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
    height: 28px
  status-free-space-min:
    width: 600px
  status-selection-only-max:
    width: 440px
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

# Rustphin design

This is the design reference for Rustphin (D-017). The tokens in the front
matter are authoritative and `crates/rustphin-ui/src/theme.rs` implements them;
the prose explains how to apply them. Change this document first, then the
theme, and keep "Implementation status" true.

The visual reference is the set of HTML mockups in [`mockup/`](mockup/), one
page per screen, numbered as in
[`project/INFORMATION_ARCHITECTURE.md`](project/INFORMATION_ARCHITECTURE.md),
with the notes in `mockup/00-what-changed.html`. The decisions behind them are
in [`project/DESIGN_BRIEF.md`](project/DESIGN_BRIEF.md). The original
reference picture is [`mockup.png`](mockup.png); colours marked "measured"
were taken from it. This version was adopted on 2026-11-08 by the W-079 design
pass, which replaces the earlier design (neutral obsidian, 28 px rows) step by
step; "Implementation status" says which parts the build already follows.

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

- **Inter** for names and interface text. `body-lg` (12 px since D-026, which
  made the whole scale smaller after the maintainer's test run) for file names, place
  names, menu items and palette result names; `body-md` (12 px) for tabs,
  buttons, banners, column headings and dialog text; `body-strong` for section
  headings in the places panel ("Devices", group names).
- **JetBrains Mono** for data compared down a column or read character by
  character: `data-md` for sizes, dates, the type column, the location path and
  dialog facts; `data-sm` for the status line, shortcuts, palette trails,
  activity figures and device figures; `label-sm` in capitals for short labels
  (OWNER, GROUP, OTHERS; a SYMLINK tag).
- **Space Grotesk** for the few headings, all 15 px: `headline-sm` for panel
  and dialog titles ("Places", "Actions", "Delete for good?", the name in
  Properties), `headline-md` for empty and error states.
- Sizes (D-026): 12 px text and names, 11 px data, 10 px capital labels, 15 px
  headings; 16 px lines for text and data, 14 px for labels, 20 px for
  headings. Row and control heights are unchanged until the W-096 top-bar
  mockup reviews the window's density.

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

- **Window, top to bottom (W-096, mockup 35):** the title bar (36 px) when
  the compositor leaves decorations to Rustphin; the toolbar (44 px) across
  the whole window; the body, which is the places panel beside a column made
  of the tab strip (36 px, always shown) over one or two panes; then the
  window lines across the full width: activity lines (44 px each) while
  operations run, the result line (44 px) after one ends, and the blur status
  (32 px) when blur is unavailable. The places panel runs from under the
  toolbar to the window lines; its title row lines up with the tab strip.
  Hairline `border` lines separate them. Without the compositor's frame a
  hairline `border-strong` frame marks the window's edge.
- **Title bar:** the small logo (16 px) on the left, the window title
  centred in `body-md` `text-muted`, then Minimise, Maximise (Restore when
  maximised) and Close as 44 × 36 px plain buttons. Dragging it moves the
  window, a double click maximises or restores, a right click opens the
  compositor's window menu; 4 px bands (16 px corners) along the edges
  resize the window.
- **Each pane, top to bottom:** in split view, its path bar (28 px); the
  error or trash banner when present (44 px, inset 8 px from the pane's
  edges), the column headings and the list, the filter bar (40 px) when open,
  and the status line. So at the bottom of the window come the panes' status
  lines first, then the window lines.
- **Tab strip:** tabs with a 16 px type icon, the label and a 24 px close
  button (12 px icon); "+" after the last tab; the rest of the strip is
  `chrome-raised`. When the tabs do not fit, they shrink evenly down to
  96 px; past that the tabs scroll sideways, the "+" stays in view after
  them, and the active tab is scrolled into view (W-103). Tabs narrower
  than 128 px show the close button on the active tab only, so the names
  keep their room; Ctrl+W and the middle button still close any tab.
- **Toolbar:** plain 32 px icon buttons (`toolbar-button`, no outline, the
  hover fill under the pointer), in groups separated by 20 px hairlines: Act
  as administrator · Back, Forward, Up, Reload · the location field (fills
  the middle) · New (Lucide `plus` and a 12 px chevron, opening the New
  menu), Open terminal here (`square-terminal`, Shift+F4) · the View button
  (the current mode's icon and a 12 px chevron, opening the view menu) ·
  Split, Show hidden, Filter · "Actions ⌄"
  with the shortcut "Ctrl+Shift+P" in `data-sm` beside it, in one outlined
  group. Toggles that are on take `toolbar-toggle-on` and are announced as
  pressed; their tooltips name the key. In split view the toolbar acts on
  the active pane. By window width: below 1000 px the shortcut hint leaves;
  below 760 px Reload, New, Open terminal here, the View button and the three
  toggles leave;
  below 560 px Forward and Up leave and Actions becomes an icon button
  (Lucide `menu`, tooltip "Actions (Ctrl+Shift+P)"); below 300 px Back
  leaves. They remain in Actions and on their keys.
- **Path bar (split view):** the side's location as clickable segments in
  `data-md`, `text` on the active side and `text-muted` on the other, under
  the 2 px `primary-bright` line that marks the active side.
- **Location field:** a 16 px folder icon, then the path in `data-md` as
  clickable segments; leading segments collapse into "…" when space runs out.
  Trash shows as "Trash", the way Places names it, not `trash:/`; Ctrl+L
  still edits the address as text.
- **Places panel:** 240 px on the left (F9 hides it); a 36 px title row
  "Places" with a 28 px ghost "New group" button on its right (Lucide
  `list-plus`, tooltip "New group (Ctrl+Shift+G)"), so its hairline lines up
  with the tab strip's; rows 28 px with
  16 px icons and a 12 px inset; section headings 28 px; device rows 48 px. The panel scrolls on its own.
- **Details view:** 28 px heading row; columns name (fills), size (96 px,
  right-aligned), modified (168 px), type (160 px); 8 px side gutter; rows
  32 px with 24 px icons or 24 × 16 px thumbnails. Below 820 px of pane width
  the type column goes, below 600 px the modified column, below 360 px the
  size column. The scrollbar's track starts below the headings.
- **New menu (W-108):** as Dolphin's "Create New": the context menu opened
  under the New button: Folder… (`folder-plus`, F10); a hairline; Text File…
  (`file-text`), Empty File… (`file`), HTML File…
  (`file-code`), then the user's templates (the files in `~/Templates`, and
  KDE template entries) by name with `file`. Choosing one creates it with a
  free name ("Text File.txt", "Text File 2.txt") and opens the name for
  editing, as New folder does. KDE's three "Link to…" entries are not
  offered yet. **Open terminal here** opens the terminal chosen in KDE's
  settings (Konsole when none) in the folder shown; it is unavailable in
  the trash and while nothing is listed.
- **Settings (W-106, D-029, mockup 36):** a dialog over the window, opened by
  Ctrl+Shift+, and the palette's "Workspace: Settings…". 760 × 560 px at most,
  16 px from every window edge; the dialog surface, accent outline and glow;
  a 48 px title row (`settings`, "Settings", Close). On the left a 200 px
  list of pages, 32 px rows with a 16 px icon and the name: General
  (`settings`), Appearance (`palette`), Views (`layout-list`), Toolbar
  (`sliders-horizontal`), Places (`bookmark`), Confirmations
  (`circle-check`), Shortcuts (`keyboard`), About (`info`); the current page
  in `primary-subtle` with the 2 px `primary-bright` bar and icon, others
  `hover` under the pointer; Up and Down move through it. Below 720 px of
  window width the list becomes a 36 px row of tabs above the page, the
  current one underlined in `primary-bright`. A page: its name in
  `headline-sm` and one sentence in `text-muted`, then groups under
  `label-sm` capitals with a hairline; each setting a 32 px row with its
  label in a 184 px column and the control after it (segmented controls for
  two or three choices, 28 px drop-downs for longer lists, checkboxes, and
  sliders with their value in `data-sm`). The page scrolls on its own. The
  action row: "Reset to defaults" (outlined, `rotate-ccw`) on the left for
  the page shown, where it has settings; "Close (Esc)" primary on the right.
  Changes apply at once. **About:** the 64 px logo, "Rustphin" in
  `headline-md`, the version in `data-sm`, one line on what it is; the
  licence (GPL-3.0-or-later) with "Show the licence", which shows the full
  text in the page with "Back"; credits; "No telemetry. Rustphin never uses
  the network." with `circle-check`; where the settings file is, with "Open
  the settings folder", which opens it in a new tab. Before the first change
  there is no file: the path adds "(made when you first change a setting)"
  and the button is unavailable.
- **View menu (W-107):** the View button opens a 240 px panel with the
  context menu's surface, under the button's right edge: the three modes as
  rows (icon, name, key: Icons Ctrl+1, Compact Ctrl+2, Details Ctrl+3), the
  current one in `primary-subtle` with the 2 px `primary-bright` bar and a
  check; a hairline; then "Size" in `label-sm` capitals with the level's
  icon size in `data-sm` ("24 px"), and a row of a 28 px "−" button, a
  slider over the seven levels (filled in `primary-bright`, its thumb in
  `text`) and a 28 px "+" button (Ctrl+− and Ctrl+=;
  Ctrl+0 resets). Choosing a mode or a size applies at once; Escape or a
  click elsewhere closes the panel.
- **Zoom:** seven levels with 16, 20, 24, 28, 32, 40 and 48 px icons; rows are
  the icon plus 8 px, at least 24 px (24, 28, 32, 36, 40, 48, 56; W-096d,
  W-107). The default is the 24 px level. In the details and compact views
  the text grows with the rows: names 11, 12, 12, 13, 14, 15 and 16 px on
  16, 16, 16, 18, 18, 20 and 20 px lines; sizes, dates and types 10, 11, 11,
  12, 12, 13 and 14 px (the default level keeps the 12 and 11 px of the
  type scale). Column headings keep their size. Icons view uses 48, 64, 96,
  112, 128, 160 and 192 px icons; at the
  96 px level a cell is 128 × 152 px with 8 px gaps (cells scale with the
  level), the name wraps to two lines, centred. Compact view fills 240 px
  columns, 8 px apart, top to bottom, with 32 px rows; as many rows as fit
  the pane's height, and the view scrolls sideways (the wheel too). Up and
  Down step through the entries, Left and Right change columns, Page Up and
  Page Down go to the ends of the column, as in Dolphin.
- **Split view:** two equal panes with a 1 px divider; each applies the column
  rules to its own width.
- **Status line:** left zone: "1 item selected (6.3 MiB)", a `border` divider,
  "12 items (150.4 MiB)", hidden counts and filter matches as today; right zone:
  "Free: 1.8 TiB of 3.6 TiB (Data)" for the volume holding the folder. By pane
  width: below 600 px the right zone leaves; below 440 px, while entries are
  selected, the left zone shows only the selection.
- **Dialogs:** centred, at least 16 px from every window edge; title row
  48 px; body scrolls; action row right-aligned, wrapping when needed. A
  question with four buttons (the name conflict) wraps its primary button onto
  a second row at 480 px rather than growing wider.

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
  (Ctrl+W)"). Tabs can be dragged to reorder. The strip is shown with one tab too (W-096).
- **Toolbar buttons:** plain 32 px buttons with 16 px Lucide icons
  (shield, arrow-left, arrow-right, arrow-up, refresh-cw, folder-plus,
  layout-grid, columns-3, list, square-split-horizontal, eye, funnel); hover
  fills `hover`; unavailable buttons use `text-disabled`. Each has a tooltip
  with its label and key.
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
  `primary-subtle`, a 2 px `primary-bright` bar on its left edge and a
  `primary-bright` icon. Section headings ("Devices", group names) in
  `body-strong` `text-muted` with a chevron that points down when open and right
  when folded; a folded section shows its count ("Network (2)"). Places in a
  group are indented one step. A group heading can be dragged: dropped on
  another heading, or on a place in that group, it takes that group's place
  (W-105); while dragged its name follows the pointer as a chip in the
  selection colours and the heading under it takes the `hover` fill. Hidden
  places, when listed, are at half opacity.
  Network bookmarks use the `server` icon.
- **Places menu (W-102):** a right-click in the panel opens a menu with the
  context menu's look and keys. On a place: Open (Enter), Open in new tab;
  "Move to group ›" (each group, the place's own disabled, then "New group…"
  and, for a grouped place, "Out of groups"), Move up (Alt+Up), Move down
  (Alt+Down); Rename (F2), Hide or Show (H), Remove from places (Del; not for
  KDE's own places). On a group heading: Collapse or Expand (Left/Right); Move
  up, Move down; Rename group (F2), Delete group (Del; its places stay). On
  empty space or the title: New group (Ctrl+Shift+G), Add to places (the
  folder shown, Ctrl+Shift+D); Show or Hide hidden places. Volumes have no menu.
- **Device rows:** 48 px: a drive icon, the name and mount point ("Data
  (/data)") in `body-md`, a 4 px usage bar (`secondary` on `track`), and
  "1.8 of 3.6 TiB free" in `data-sm` `text-muted`; when the two units differ
  both are written ("512.0 MiB of 1.2 GiB free"), since the longer sentence
  does not fit the 240 px panel. Removable volumes have an outlined 28 px eject
  button (Lucide `arrow-up-from-line`, tooltip "Safely remove"); unmounted
  volumes are
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
  fitted in the icon's square without cropping: a 3:2 picture is 24 × 16 px in
  a row and 96 × 64 px in an icon cell and in Properties. Until ready, or if it cannot be
  made, the type icon stays; nothing flashes.
- **Context menu:** 240 px, 4 px inner padding. Items 32 px: a 16 px Lucide
  icon, the label in `body-md`, the shortcut right-aligned in `data-sm`
  `text-muted`: Open (Enter), Open with ›; Cut (Ctrl+X), Copy (Ctrl+C), Paste
  (Ctrl+V), Copy location (Ctrl+Alt+C), Duplicate here (Ctrl+D); Rename (F2),
  Move to trash (Del), Delete… (Shift+Del); Properties (Alt+Enter). The item
  under the pointer or keyboard takes the selection look: the `primary` fill
  and the 2 px `primary-bright` bar. Disabled items use `text-disabled` and are
  skipped. Separators are `border` hairlines with 4 px above and below. "Open with ›" opens a second menu of applications with their icons.
- **Actions palette:** 640 px, near the top of the window, over the scrim. A
  title row with "Actions" in `headline-sm` and "Ctrl+Shift+P" in `data-sm`. A
  search field with a search icon and the placeholder "Search actions, places,
  tabs and recent folders…", ringed in `primary-bright` while focused. Results
  52 px: a 20 px icon, the name in `body-lg`, a trail in `data-sm` ("View ›
  Show hidden files"), the shortcut right-aligned. At most seven results show;
  the list scrolls. The selected result takes the selection look, fill and
  bar. Empty: "No
  matches. Try another action name or folder path."
- **Dialogs:** 480 px wide (batch rename 640 px), `surface-raised`, accent
  outline and glow, over the scrim. A 48 px title row with an icon, the title in
  `headline-sm` and a close button; the body scrolls between title and actions;
  the action row is right-aligned and wraps. The safe default is the
  `button-primary`; a destructive default uses `button-destructive`; the
  others are `button-outlined`. Every button shows its key ("Close (Esc)",
  "Apply (Enter)"). A disabled primary button keeps its fill at half opacity
  with `text-disabled` text. A button whose action has a clear picture leads
  with a 16 px Lucide icon 8 px before its words, the same icon the action
  has in menus and the toolbar (W-097): Rename `pencil`, Apply `check`, Open
  `square-arrow-out-up-right`, Run `play`, Skip `skip-forward`, Replace
  `replace`, Merge `merge`, Keep both `copy-plus`, Try again `rotate-cw`,
  Copy here and Copy list `copy`, Move here `folder-input`, Delete for good
  `circle-x`, Empty the trash `trash`, Undo `undo-2`, Back `arrow-left`, Up
  `arrow-up`, Show hidden `eye`, Clear filter `funnel-x`, Stop acting as
  administrator `shield-off`. Cancel, Close and Stop stay words only: their
  picture would be the close cross, which already means closing the dialog.
- **Properties:** title row "<name> — Properties". Header: a 96 × 64 thumbnail
  (or the 48 px type icon), the name in `headline-sm` and the type in
  `data-md`. Facts as label (`text-muted`, 128 px column) and value
  (`data-md`): Location, Size (with exact bytes), Modified, Accessed, Created
  when known. A folder's size reads "Calculating size…" until known. A
  hairline, then "Permissions" with the mode in words and octal
  ("rw-r--r-- (0644)"), then three columns in `label-sm` naming the owner and
  group ("OWNER · alex", "GROUP · alex", "OTHERS"), each with Read, Write and
  Execute checkboxes, separated by hairlines. While a folder's size is being
  calculated the line reads "Calculating… 1,204 items so far". Apply stays
  disabled until a box changes.
- **Batch rename:** three labelled modes (Replace, Pattern, Case) as a
  segmented row, the chosen one in `primary`; the rule fields; the preview as
  two `data-md` columns, old and new name, with an arrow between them, in the
  same scrolling body; new names that cannot be used are in `error`. A problem
  is a sentence in the error colours with an alert icon above the preview.
  "Nothing would change" is not a problem but a neutral note (`note-neutral`:
  `text-muted`, an info icon, a `border` outline). Rename stays disabled while
  there is a problem or nothing would change.
- **Question dialogs** (name conflict, failed entry, move or copy, run, empty
  trash, delete for good): the title states the question or the fact
  ("“IMG_4821.jpg” already exists", "Couldn't copy “IMG_4821.jpg”", "Run this
  file?"); the full sentence and facts (`data-md`, labels in a 72 px
  `text-muted` column) scroll in the body. The name conflict lists the existing
  and incoming entries and a "Keep both" fact with the name the copy would get;
  its buttons are Stop, Skip, Replace and Keep both. "Run this file?" makes
  Cancel the default (Enter), as Dolphin focuses Cancel; Run has its own key.
  Cancel buttons say "Cancel" (W-093; they read "Keep" before).
- **Error details dialog:** title says what the operation was; a summary
  sentence; one row per failed entry with the name (`body-md`), its folder
  (`data-sm` `text-muted`) and the reason in words; "Copy list" (outlined) and
  "Close (Esc)" (primary).
- **Checkboxes:** 16 px, `border-strong` outline; checked: `primary` fill with
  an `on-primary` check; focus: `primary-bright` ring; the label is clickable.
- **Focus ring:** buttons and checkboxes show keyboard focus with a 2 px
  `primary-bright` ring outside a 1–2 px gap in `surface-raised`; fields show
  a 1 px `primary-bright` ring around their `primary-bright` outline.
- **Activity line:** an operation icon (Lucide `copy` while copying, `move`
  while moving, `loader` while connecting or loading), the sentence ("Copying 12 items to
  “/data/Backup/Pictures”…") in `body-md`, a flexible 6 px progress bar
  (`secondary` on `track`), the figures as today ("6 of 12 · 742.0 MiB of 1.2 GiB ·
  48.6 MiB/s")
  in `data-sm` `text-muted`, and an outlined "Stop". It appears after 300 ms.
- **Result line:** 44 px after an operation ends: an emerald `circle-check`,
  the sentence ("Copied 12 items to “Pictures”."), an outlined "Undo Ctrl+Z"
  when it can be undone, and a dismiss button (tooltip "Dismiss"). It leaves
  after 6 seconds, unless it reports a problem: then it uses the error colours
  and an alert icon and stays until dismissed.
- **Blur status:** a 32 px chrome line with an info icon and "Background blur
  unavailable; using the opaque theme." in `text-muted`.
- **Filter bar:** a 16 px filter icon, a 32 px field labelled "Filter by name"
  (so its 2 px ring fits the 40 px bar), and a close button (tooltip "Close
  the filter (Esc)").
- **Status line:** two zones as in Layout, in `data-sm` `text-muted`.
- **Empty and loading states:** a 32 px icon, a `headline-md` line saying what
  is the case, a `text-muted` line saying why, and at most one action. Centred
  when it fits; otherwise a bounded scroller with a 16 px gutter.
- **Rename field:** 28 px inside the 32 px row with 2 px padding,
  `surface-raised` with `text` in place of the name, the stem selected, ringed
  in `primary-bright`. Dialog inputs stay 36 px.
- **Tooltips:** `surface-raised`, `body-md`, the label then the key in
  `data-sm` `text-muted`, after 500 ms of hover or on keyboard focus.
- **Dragging:** a `drag-chip` with the first entry's thumbnail or type icon,
  then its name or the count ("3 items"); no copy or move icon, because the
  drop asks "Move or copy?" unless Shift or Ctrl is held. A folder under the
  pointer takes `hover`.
- **Rubber band:** a 1 px `primary-bright` outline with a `hover` fill.
- **Split panes:** the pane taking keyboard input has a 2 px `primary-bright`
  line along its top, drawn only while split.
- **Scrollbars:** 8 px, 2 px in from the edge, `text-muted` thumb at half
  opacity on a transparent track; `primary-bright` while hovered or dragged.
  In details view the track starts below the column headings.
- **Interface icons:** Lucide outlines from the toolkit's bundled set. Fixed
  names for the less obvious ones: `menu` (Actions as an icon),
  `arrow-up-from-line` (eject; Lucide has no eject glyph), `copy` (copying),
  `loader` (connecting, loading), `circle-check` (done), `triangle-alert`
  (problems), `info` (neutral notes).

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
- Don't put text on bare `window-glass`, and don't go below 10 px (capital labels only; other text at least 11 px).
- Don't add a radius other than 2 px.
- Don't use `primary` fills for location (that is `primary-subtle`) or for
  decoration.
- Don't follow the system colour scheme; do use the system icon theme for file
  types.

## Changes from the earlier design

What this version changes from the design the build followed before W-079:

- **Density:** rows 28 → 36 px at the default zoom (24 px icons), names 14 →
  15 px, data 13 → 14 px, headline-sm 16 → 17 px; toolbar 44 → 52 px; tab
  strip 32 → 40 px; status line 28 → 32 px; filter bar 36 → 40 px; places
  panel 220 → 240 px with 32 px rows; device rows 48 → 56 px; dialogs 440 →
  480 px; column widths size 88 → 96, modified 144 → 168, type 184 → 160 px;
  column breakpoints 760/560/320 → 820/600/360 px; toolbar breakpoints
  320/240 → 640/560/300 px.
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
  progress bar with mono figures and an outlined Stop; the result line with
  Undo; question titles that state the question; "Run this file?" defaults to
  Keep.
- **Unchanged:** the three font families, the 2 px radius, the 4-pixel grid,
  opaque fallback behaviour, every wording rule, the keyboard model, and the
  rule that menus, palette and dialogs are solid.

## Implementation status

Adopted on 2026-11-08 (W-079). The build still follows the earlier design
except where the list below says otherwise; each W-079 step adds its line.

- W-079a (2026-11-08): every colour token, `data-md` 14 px and `headline-sm`
  17 px, the floating glow; the tab strip on `chrome-raised` with the active
  tab on `chrome`, progress and usage bars on `track`, the error banner's
  `error-border`, the current place in `primary-subtle`, disabled menu items
  in `text-disabled`, the success icon on results that can be undone.
- W-079b (2026-11-08): the tab strip (40 px, icons, 24 px close, 40 px "+"),
  the 52 px toolbar with outlined 36 px buttons, the outlined location field
  with its icon and mono segments ("Trash" in the trash), Actions with its
  chevron and shortcut, the 640/560/300 px rules, the 52 px places title, and
  the window lines' order. Disabled toolbar buttons use the component
  library's dimmed text, close to but not exactly `text-disabled`.
- W-079c (2026-11-08): the places panel as described, with Devices folding
  for the run (not saved). With an eject button, long figures can still be
  cut short.
- W-079d (2026-11-08): the zoom table and 36 px default rows, icon cells,
  240 px compact columns, column widths and pane-width rules, the mono type
  column, "—" for folder sizes, thumbnails from 24 px, the 28 px rename
  field, the drag chip with its icon, 20 px state headings with reason
  lines. List views have their scrollbar (W-090): vertical in details
  (below the headings) and icons, horizontal along the bottom in compact.
  The places panel has a Remote section (W-094) between the groups and
  Devices, with a foldable heading like Devices ("Remote (n)" when folded)
  and the `server` icon for each network place.
  Hidden entries (W-091), shown with Ctrl+H, are drawn at 60% opacity
  except while selected, as in Dolphin; their text stays at 4.5:1 or more.
- W-086 (2026-11-09): compact view fills columns top to bottom and scrolls
  sideways, with Dolphin's keys in that view.
- W-079e (2026-11-08): the two-zone status line with free space and its
  600/440 px rules (the volume name came with W-103), the filter bar's 32 px field, the
  activity line, the result line with Undo, Details… and dismiss, the error
  details dialog (opened from the result line), the blur status line.
- W-079f (2026-11-08): menu icons, shortcuts and the selection bar; the
  palette's title row, search icon, 52 px results with category icons and
  trails, placed near the top. Results use one icon per category.
- W-079g (2026-11-08): every dialog on the shared frame, title row and
  action row; question titles that state the question; the Keep both fact;
  Cancel as the run default; Properties' header, facts and permission columns;
  batch rename's segments, preview and notes.
- Mockup comparison (2026-11-09): each screen was laid over its mockup
  render at the mockup's size. Fixed to match: outlined buttons clear until
  hovered and destructive buttons with their error outline; text fields on
  `surface-raised` (the palette's on `content`) with the 1 px focus ring and
  their real heights (a single-line field's `h` was ignored, so every field
  was 32 px); disabled toolbar icons in `text-disabled`; the path as one
  string; table columns with 8 px padding and no gaps, so headings line up
  with cells, and the "↑"/"↓" sort mark; the selection bar laid over the
  entry instead of pushing it; icons-view padding 16/12 px and compact cells
  starting at their column; the film icon for Videos; heading line heights
  (24 and 28 px); the palette without a visible key-hint footer; the funnel
  filter icon and 8 px field padding; renaming on the hover fill with a field
  as wide as the name, `primary` text selection and the status hint; equal
  permission columns, `label-sm` capitals and 28 px checkbox rows; dialog
  scrollbars in the body padding; mono name lists; state icons (`eye-off`,
  `search-x`), spacing and 32 px buttons; the 44 px folder error banner with
  a dismiss icon; batch rename segments with icons, Find and Replace side by
  side in mono, and one problem note above the preview.
- Top bar redesign (W-096, mockup 35): the window's own title bar with
  resize edges, one toolbar across the window with the new buttons, the tab strip always shown
  over the files, a path bar per side in split view, and the denser rows,
  places, headings and status line.
- Acting as administrator (W-098, mockup 35c): the shield toggles the
  window's mode (`error` on `error-surface` while on, dimmed with a reason
  when kio-admin is missing), the 36 px bar under the toolbar with "Stop
  acting as administrator", and "(administrator)" in the title. Seen only
  in the isolated session with the stand-in helper; KDE's real password
  dialog is the maintainer's check.
- Font weights (W-088): the app bundles static faces of Inter (400, 500,
  600), Space Grotesk (600) and JetBrains Mono (400, 500), because the pinned
  GPUI ignores a variable font's weight axis. Headings render semibold as
  drawn. Weights not listed in the typography tokens fall back to the
  nearest bundled or installed face.
- Places menu and the "New group" button (W-102), and second menus that
  line up with the row that opens them.
- W-103 (2026-11-10): the volume's name after the free space (only for a
  listed volume mounted exactly where the folder's filesystem is: tmpfs
  and network mounts have none); an icon per command in the palette (the
  same as its toolbar button or menu row; other results keep their
  category's); icons, and sizes for files, in the delete question's list
  of names; tab overflow; Devices and Remote stay folded between runs
  (kept with the place groups).
- Not yet implemented: the design's offset focus ring on buttons (the
  library draws its own); icons in the move-or-copy question's list (a drop
  does not say which entries are folders). "Details…" sits on the result line, since
  operation results are window-level here.
- A folder that cannot be opened (W-089): the view goes into it and says so
  in place, as mockup 28 draws it: a lock icon (`folder-x` when the reason is
  not permission), "Could not open this folder", why and what to do, and a
  primary "Back (Alt+Left)" button, or "Up (Alt+Up)" when there is no history.
  The status line names the reason ("Not readable", "Does not exist"). A
  shown folder that fails to re-read keeps its listing under the error
  banner, as does a folder that vanished.
- Settings (W-106b): the dialog, its page list and the tabs below 720 px,
  the action row and the About page are built; the other pages arrive with
  W-106c–g, each with its "Reset to defaults".

Carried over unchanged from the earlier design and implemented: the Actions
palette's **Recover original names after interruption** for the current folder
(browsing never starts recovery; an invalid record, changed file or occupied
original name produces an actionable error); the opaque fallback when blur is
not advertised, lost or switched off, with its persistent status; bounded
scrolling bodies for every dialog and folder state at the minimum window; the
trash banner and its Trash actions menu; floating surfaces that escape pane
clipping. Reverse traversal between named text fields is blocked by duplicate
focus registration in the pinned toolkit (W-076e, not approved).

Lint with `npx @google/design.md lint docs/DESIGN.md`. Contrast is checked by
the tests in `theme.rs` (glass over white and opaque); before W-079 adopted the
tokens a script mirroring them gave the lowest text pair 4.92:1 and the lowest
indicator pair (`primary-bright` on window glass over white) 4.31:1. The
linter's remaining warnings are `border`, `border-strong`, `border-accent` and
`error-border`, used as outline colours, which the format's component
properties cannot express; the prose says where.
