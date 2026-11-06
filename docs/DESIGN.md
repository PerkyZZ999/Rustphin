---
version: alpha
name: Rustphin Obsidian
description: Dark obsidian-glass theme with royal-blue accents for the Rustphin file manager (GPUI, KDE Plasma on Wayland).
colors:
  # Surfaces. The three "glass" values are layered over the compositor-blurred desktop.
  window-glass: "rgba(11, 15, 23, 0.72)"
  chrome-glass: "rgba(11, 15, 23, 0.20)"
  content-glass: "rgba(11, 15, 23, 0.60)"
  # Their solid replacements when blur is unavailable or switched off.
  window-solid: "#0B0F17"
  chrome-solid: "#101521"
  content-solid: "#0B0F17"
  # Menus, palette, dialogs, tooltips: always solid.
  surface-raised: "#0F172A"
  # Dims the window behind a dialog.
  scrim: "rgba(4, 6, 10, 0.60)"
  # Text.
  text: "#DFE2EF"
  text-muted: "#C3CAD9"
  # Royal blue.
  primary: "#3457D5"
  primary-hover: "#2F4FC4"
  on-primary: "#F4F6FB"
  primary-bright: "#8FA8FF"
  # Supporting signals.
  secondary: "#38BDF8"
  tertiary: "#10B981"
  error: "#FF9B9B"
  error-surface: "#3A1A1F"
  # Lines and states.
  border: "rgba(255, 255, 255, 0.08)"
  border-strong: "rgba(255, 255, 255, 0.12)"
  border-accent: "rgba(65, 105, 225, 0.45)"
  hover: "rgba(255, 255, 255, 0.08)"
typography:
  headline-md:
    fontFamily: Space Grotesk
    fontSize: 20px
    fontWeight: 600
    lineHeight: 28px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Space Grotesk
    fontSize: 16px
    fontWeight: 600
    lineHeight: 24px
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 400
    lineHeight: 20px
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 400
    lineHeight: 16px
  data-md:
    fontFamily: JetBrains Mono
    fontSize: 13px
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
  toolbar:
    backgroundColor: "{colors.chrome-glass}"
    height: 44px
    padding: "{spacing.sm}"
  toolbar-full-navigation:
    width: 320px
  toolbar-back-navigation:
    width: 240px
  details-size-column-min:
    width: 320px
  rename-dialog:
    width: 640px
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "{spacing.lg}"
  toolbar-button:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    size: 28px
  toolbar-button-hover:
    backgroundColor: "{colors.hover}"
  toolbar-button-selected:
    backgroundColor: "{colors.border-strong}"
  column-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-sm}"
    height: 28px
  column-heading-active:
    textColor: "{colors.text}"
  entry-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.sm}"
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
    padding: "{spacing.sm}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
  input:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
    rounded: "{rounded.sm}"
    height: 28px
  banner-error:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.error}"
    rounded: "{rounded.sm}"
    padding: "{spacing.md}"
  floating-surface:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
  command-palette:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 640px
    padding: "{spacing.md}"
  palette-result:
    height: 48px
    typography: "{typography.body-md}"
  status-line:
    backgroundColor: "{colors.chrome-glass}"
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
    height: 28px
---

# Rustphin design

The main reference for how Rustphin looks. Token values in the front matter are authoritative; the prose explains how to apply them. The implementation of these tokens is `crates/rustphin-ui/src/theme.rs`, whose tests check the contrast rules below.

## Overview

Rustphin is a file manager for KDE Plasma, used every day by people who move through large folders quickly. The interface is a precise instrument: dense, geometric and legible, with nothing decorative competing with the files.

Three ideas hold it together:

- **Obsidian glass.** The window is dark, translucent glass over the desktop, which the compositor blurs. The toolbar and status line are the clearest glass; the file list sits on a more solid pane so columns of text stay readable over anything behind.
- **Royal blue as the only energy.** One accent colour marks selection, focus and the primary action. Cyan and emerald appear only as status signals.
- **Progressive disclosure.** The default window shows the location, the files and a status line. Secondary controls appear on hover, on selection, by shortcut or through the command palette, and everything reachable by hover is also reachable by keyboard.

Density is high but never cramped: a 4-pixel grid, 28-pixel rows at the default zoom, and hairline dividers instead of boxes.

## Colors

- **Glass surfaces** (`window-glass`, `chrome-glass`, `content-glass`) are obsidian at partial opacity. They are stacked: window first, then chrome or content on top of it. Never place text directly on `window-glass` without one of the other two.
- **Solid surfaces** (`window-solid`, `chrome-solid`, `content-solid`) replace the glass when the compositor offers no blur or the user turns transparency off. The background-effect manager must explicitly advertise the blur capability before glass is enabled; accepting an effect request is insufficient. Monitor this capability throughout the window lifetime. Loss or monitor failure restores solid surfaces; confirmed recovery restores glass. While unavailable, show a wrapping, read-only status line below the panes: “Background blur unavailable; using the opaque theme.” Keep it separate from file-operation messages. Explicit opaque mode does not produce an unavailable-service warning. Every rule in this document must hold in both modes.
- **Raised surface** (`surface-raised`) is for anything that floats inside the window: menus, the command palette, dialogs, tooltips. It is always solid, because the toolkit cannot blur content inside the window.
- **Text** uses `text` for names and primary content and `text-muted` for secondary data, headings of inactive columns and the status line. Both meet WCAG AA (4.5:1) on every surface, including glass over a white window behind.
- **Primary** (`primary`, with `on-primary` text) fills the selected entry and the one primary button in a view. `primary-hover` is its pointer-over state.
- **Primary bright** (`primary-bright`) is the accent on dark surfaces where a fill would be too heavy: the selection bar, the keyboard-focus bar, focus rings, folder glyphs and links. It meets 3:1 against every surface.
- **Secondary** (cyan) signals live activity: transfers in progress, loading. **Tertiary** (emerald) signals a healthy or completed state. Neither is used for decoration or as a second accent.
- **Error** text sits on `error-surface` with an `error`-coloured outline and an icon; colour alone never carries the message.
- **Borders** are hairlines: `border` between regions, `border-strong` around controls, `border-accent` around floating surfaces.

## Typography

Three families, each with one job:

- **Inter** (`body-md`, `body-sm`) for file names and interface text. Names are proportional so long ones fit.
- **JetBrains Mono** (`data-md`, `data-sm`, `label-sm`) for data that is compared down a column or read character by character: sizes, dates, paths, counts, permissions. Its fixed width keeps columns aligned while sorting.
- **Space Grotesk** (`headline-md`, `headline-sm`) for the few headings: empty and error states, dialog titles, section labels.

`label-sm` is set in capitals for short tags only (for example a `SYMLINK` badge). Nothing is smaller than 11 pixels. Body text uses a 20-pixel line, which is also the icon size beside it in rows.

All three families are installed on the reference system. They are not bundled yet; when one is missing the system interface font is used.

## Layout

Everything sits on a 4-pixel grid; use only the `spacing` tokens.

- **Window, top to bottom:** toolbar (44 px), an inline message when something failed, the file area (fills the rest), status line (28 px). Hairline `border` lines separate them.
- **Toolbar:** navigation buttons on the left, a flexible location field, and labelled Actions on the right. Below 320 px per pane Forward and Up leave the bar; below 240 px Back also leaves. These commands remain in Actions and on their shortcuts.
- **File area:** one of three views. Details has a 28 px heading row and columns for name (fills), size (88 px, right-aligned), modified (144 px) and type (184 px). Icons and compact are grids whose cells grow with the five zoom levels. Rows and cells have an 8 px side gutter.
- **Narrow windows:** below 760 px per pane the type column goes, below 560 px the date column, and below 320 px the size column. The name keeps the remaining width, including at the minimum split-window size. Sorting by hidden columns stays in Actions. The location collapses leading folders into an ellipsis and shows the last four.
- **Places panel:** 220 px on the left, hidden or shown with F9 or Actions. Split panes share the remaining width. Entry content and the truncated status line stay inside their pane at narrow widths.

## Elevation & Depth

Depth comes from stacked planes and edge light, not soft shadows.

- **Level 0, the desktop:** whatever is behind the window, blurred by the compositor.
- **Level 1, window glass:** `window-glass`, with `chrome-glass` for the toolbar and status line.
- **Level 2, content pane:** `content-glass` under the file views.
- **Level 3, floating surfaces:** `surface-raised`, solid, outlined with a 1 px `border-accent` line and a tight glow (`0 0 16px -2px` of the accent at about 35%) to lift them off the content.

Blur exists only between level 0 and the window. Do not design anything that needs to blur the window's own content.

When blur is unavailable every glass token is replaced by its solid counterpart and the levels are told apart by their borders alone.

## Shapes

One radius: `rounded.sm`, 2 pixels, on rows, cells, buttons, inputs, banners, menus and the rubber band. It only takes the hardness off the corner; the look stays orthogonal. There are no pills and no circles, and nothing uses a larger radius.

## Components

- **Toolbar:** Back, Forward and Up are familiar 28 px icon buttons with tooltips and dimmed unavailable states. The location bar fills the remaining space before a labelled ghost button, "Actions", whose tooltip gives Ctrl+Shift+P. View modes, zoom, hidden files, sorting, creation and trash are reached through the palette or their shortcuts. The places header is a label; adding places, groups and listing hidden places use the same palette or shortcuts. No secondary control relies on hover. Hover shows `hover`.
- **Column headings:** `text-muted`; the column being sorted by uses `text` and shows an up or down arrow, so direction is never shown by colour alone.
- **Entries (rows and cells):** icon from the system icon theme, then the name. Hover shows `hover`. A selected entry has the `primary` fill, `on-primary` text and a 2 px `primary-bright` bar on its left edge. An entry that has the keyboard focus without being selected keeps the bar only.
- **Data cells:** size, date and type in `text-muted`; sizes right-aligned; folders show no size.
- **Primary button:** `primary` fill, one per view at most (for example "Show hidden files" in an empty state). Other actions are plain text buttons.
- **Inputs:** the path field and, later, search: `surface-raised` with a `border-strong` outline that becomes `primary-bright` on focus. Each single-line field is one tab stop: Tab advances and Shift+Tab returns to the previous control without editing its value.
- **Messages:** failures appear inline above the files, in an `error-surface` banner with an icon, a sentence saying what happened, and a way to dismiss it. The file list stays as it was.
- **Empty and loading states:** a large icon, a `headline-sm` line saying what is the case, a `text-muted` line saying why, and one action when there is something useful to do.
- **Tab strip:** 32 px high on `surface-chrome`, shown only when a window has more than one tab. Tabs are `body-sm`; inactive tabs use `text-muted`, the active tab uses `text` with a 2 px `primary-bright` underline. Each tab has a close button; a `+` button after the last tab opens a new one.
- **Split panes:** two equal panes separated by a 1 px `border` line. A 2 px `primary-bright` line above a pane marks the one that receives keyboard input; it is only drawn while a tab is split.
- **Rename field:** shown in place of the entry's name. `surface-raised` with `text`, so it reads the same on a selected and an unselected row. It opens with the name selected up to the extension.
- **Dialogs:** a floating surface up to 440 px wide, centred over the window dimmed with `scrim` and inset by at least 16 px on every edge. A short `headline-sm` heading and the action row stay visible at the minimum window size. The complete question, including long names, explanations and facts in `data-md`, scrolls between them with the Properties scrollbar style and a 16 px gutter. Arrows, Page Up/Down and Home/End scroll that body; each new question starts at the top. Buttons wrap when needed and remain right-aligned: the safe default is the `button-primary`, a destructive default uses the error colours, the others are outlined. Every button shows its key. Dialogs appear only when a decision is needed: a name conflict, a failed entry, a permanent delete, or running a program.
- **Context menu:** right-click, or Shift+F10 with a selection, opens a 240 px solid raised menu at the pointer (at the focused entry when opened from the keyboard). Items are 28 px, in `body` type, with the 2 px radius. The item under the pointer or the keyboard cursor uses the entry selection: `primary` fill, `on-primary` text and a 2 px `primary-bright` bar. A disabled item is `text-muted` and is skipped by the keyboard. A separator is a hairline. "Open with" opens a second menu listing the applications registered for that file's type, or a sentence when there are none or the list is still being read. Arrows move, Right opens the application list, Enter runs the item, Escape or a click elsewhere closes the menu and returns focus. The menu offers only what applies to the selection or, on empty space, paste and new entries.
- **Run confirmation:** activating an executable or a `.desktop` file asks before anything runs. The short heading says Run this file?; the complete question in its scrolling body names the file (or the application, for a shortcut). Keep (Esc) is the primary button and runs nothing. Run (Enter) uses the error colours. An executable also offers Open, which hands the file to the default application instead of executing it. A normal file opens with the default application and does not ask.
- **Properties:** Alt+Enter, or Properties in the context menu, opens a solid raised dialog up to 440 px wide for the one selected entry, inset by at least 16 px on every edge. The title and Close/Apply row stay visible on short windows; facts and permissions scroll between them. Arrow keys scroll one text line, Page Up/Down one visible page, and Home/End reach the first/last facts. A scrollbar uses text-muted on a transparent track, with primary-bright for hover/active. The title is its name. Facts are a label in `text-muted` and a value in `data` type: type, size, modified, accessed, and created when the filesystem has a creation time. A folder's size says "Calculating size…" until a background walk finishes, then the size; if some items could not be read, a sentence says so. A symbolic link says where it points, and its permissions are shown but not edited. Permissions are the mode in words (`rwxr-xr-x`) and octal, then nine checkboxes in three labelled groups — owner, group and others — each read, write and execute. Special bits, when set, are named and kept. Nothing is written until Apply, which stays disabled until a box changes. Close (Esc) discards an unapplied change. Apply (Enter) uses the operation engine, so it can be undone. A failure to read the item is a sentence saying what happened, with Close.
- **Batch rename:** F2 with two or more selected files opens a solid raised dialog up to 640 px wide, inset by at least 16 px on every edge. The title and Cancel/Rename row stay visible on short windows; modes, rule fields, explanations and preview scroll between them. A field gaining focus is brought into view. Ctrl+Page Up/Down scroll the body without changing text or switching tabs, with a scrollbar styled like Properties. Three labelled modes — Replace, Pattern and Case — and the chosen one uses the primary fill. Replace has Find, Replace with, and a Regular expression checkbox. Pattern has the pattern, a start and a step, and a sentence: `#` is the counter, the number of `#` signs is the padding, `@` is the current name. Case offers lower case, upper case and capitalised words. The preview lists every old name and its new name in the same scrolling body as the rule, without a nested scroll region. The body leaves a 16 px gutter for its scrollbar. A problem is a full sentence under that row, in the error colour and in words. A problem with the rule is a sentence above the list. Rename stays disabled while a problem remains or nothing would change. Rename (Enter) uses the operation engine; Cancel (Esc) closes and changes nothing.
- **Activity lines:** under the panes, on `chrome-glass`, only while there is something to say. A running operation that has lasted 300 ms shows what it is doing, a 4 px progress bar (`secondary` on a `hover` track), counts and speed in `data-sm`, and a stop button. The result of the last operation shows as one line with an Undo button; it leaves by itself after 6 seconds, unless it reports a problem, which uses the error colours and stays until dismissed.
- **Trash banner:** while a trash is shown, a line under the toolbar on `chrome-glass` names it, says how long items stay, and holds its actions: the trashes of other volumes as plain buttons, Restore (disabled without a selection) and "Empty trash…" as outlined buttons. The trash banner and window title identify the current location; opening trash is available through Actions.
- **Cut entries:** drawn at half opacity until they are pasted or the clipboard changes.
- **Dragging:** a chip in `primary` with `on-primary` text follows the pointer, naming the entry or counting them. A folder under the pointer takes the `hover` fill. Icon and name are the handle in the details view, the whole cell in the grids.
- **Places panel:** if the device service cannot connect or list volumes, keep a Devices heading and a readable status explaining that devices could not be listed, with a path-based browsing fallback and restart/reconnect advice; detailed service errors stay in the log. Remove stale device rows while unavailable; successful listing clears the status. This status lives beside the affected feature and does not replace operation notices. 220 px wide on `chrome-glass`, left of the panes, with a 44 px header holding its label. Rows are 28 px with a 16 px icon; the place of the folder being shown has the `hover` fill, a `primary-bright` bar and icon. A group heading is `headline` type in `body-sm` size and `text-muted`, with an arrow that points down when open and right when folded; a folded group shows how many places it holds. Places in a group are indented one step. Hidden places, when listed, are at half opacity.
- **Device rows:** in the places panel under a "Devices" label, 48 px tall: a drive icon, the label, a 2 px usage bar (`secondary` on a `hover` track) and, in `data` type at 11 px, the free and total space in words. An unmounted volume is `text-muted`. An eject button sits at the right of volumes that may be unmounted.
- **Thumbnails:** a picture's own content replaces its type icon when icons are 32 px or larger. It is fitted inside the icon's square without cropping or stretching. Until it is ready, and when it cannot be made, the type icon stays; nothing flashes or shows an error.
- **Rubber band:** a 1 px `primary-bright` outline with a `hover` fill.
- **Floating surfaces:** as level 3 above. Menu items follow the entry hover and selection rules.

- **Command palette:** Ctrl+Shift+P opens a 640 px solid raised surface, inset by at least 16 px on narrow windows, over the scrim. A visible label names its search field. Up to seven 48 px results are shown at once in a scrolling list; each has a name, a category/path and its shortcut when available. Search covers actions, places, open tabs and recent folders. Selection uses the primary fill plus a leading marker. Up/Down or Tab/Shift+Tab changes the selection, Enter runs it, Escape or clicking the scrim closes it and returns focus. Background navigation is blocked. A pending query cannot activate a stale result; the empty state asks for another search. History failures and clear-history confirmation are written out in the palette footer.

## Do's and Don'ts

- Do take every colour, size, spacing and radius from the tokens; add a token before adding a literal.
- Do check both glass and solid modes for any new surface or text colour, and add the pair to the contrast tests.
- Do pair colour with a shape, icon or text for every state: selection, focus, sort direction, errors.
- Do give every icon-only control a tooltip, and every hover-revealed control a keyboard or palette path.
- Do use JetBrains Mono only where alignment or exact reading matters.
- Don't blur, or rely on blurring, anything inside the window.
- Don't introduce a second accent, gradients, or soft drop shadows.
- Don't put text on bare `window-glass`, and don't go below 11 px.
- Don't add a radius other than 2 px.
- Don't follow the system colour scheme; do use the system icon theme for file types.

## Implementation status

The Actions palette offers **Recover original names after interruption** for the
current folder. It explicitly restores original names from a batch-rename
recovery record. Browsing never starts recovery. A missing record is harmless;
an invalid record, changed file or occupied original name produces an actionable
error through the existing operation notice, without replacing that entry.

As of 2026-10-18 the application implements the surfaces, text colours, selection, hover, error banner, toolbar (including the 320 px and 240 px navigation breakpoints), tab strip, split panes, dialogs, the batch-rename dialog, activity lines, the rename field, the trash banner, the places panel, the command palette and labelled Actions entry, the three views, the 4-pixel spacing scale, the 2 px radius, and the three font roles (Inter for names and interface text, JetBrains Mono for sizes, dates and the status line, Space Grotesk for state headings). The status line truncates inside its pane.

Floating surfaces (`surface-raised`, `border-accent`, the glow) are used by dialogs, the properties dialog, the palette and the context menu. The cyan signal is used by the progress bar. Focused inputs use `primary-bright` through the component ring token (observed 2026-11-04). Not yet applied in code: the emerald signal and the type column, which still uses Inter. The size-column breakpoint is implemented and verified at the minimum split-window size (S11 W-076a, 2026-11-05). Properties and batch rename keep their title/actions visible with scrolling bodies at the minimum window size (S11 W-076c/d, 2026-11-05). Operation and run questions also use bounded scrolling bodies with fixed short headings and wrapping actions (S11 W-076f, 2026-11-05). Device-service failures show a readable feature status and remove stale rows; successful listing restores the normal section (S11 W-076g, 2026-11-05). An owned capability monitor confirms blur before choosing glass and follows loss/recovery while the window is open. Unavailable blur uses solid tokens and a persistent status separate from operation notices; explicit opaque mode stays quiet (S11 W-076h/j, 2026-11-06). Normal and minimum-window transitions are observed over generated white content in an isolated compositor; real-desktop review remains open. Reverse traversal between named text fields is blocked by duplicate focus registration in the pinned toolkit (S11 W-076e); the desired one-stop input behavior above is not yet fully implemented.

Changed from the first draft of this document, by the user's decision on 2026-10-12: the accent is royal blue, not indigo; corners are 2 px, not square; monospace is for data only, with Inter for file names. Removed because the toolkit cannot do it: backdrop blur on popovers and panels inside the window. Removed because Rustphin has no such feature planned for its first release: terminal docks, checksum and hex inspectors, tree connectors.
