# Design Brief: Rustphin main window and its surfaces (W-079)

> **Status (2026-11-14): historical.** This brief was the input for the
> W-079 design pass, which adopted [`../DESIGN.md`](../DESIGN.md) on
> 2026-11-08; it is kept as written. Since then D-026 and W-096 made the
> type and rows smaller, and the interface moved from GPUI to Qt 6 and QML
> (D-034, W-114): `theme.rs` and the GPUI toolkit it mentions are gone, the
> colours now live in `crates/rustphin-view/src/colors.rs` and the sizes in
> `metrics.rs`. What the build draws today is in `docs/DESIGN.md` (its
> "Implementation status") and
> [`INFORMATION_ARCHITECTURE.md`](INFORMATION_ARCHITECTURE.md).

Written 2026-11-08 for the mockup-aligned design pass (W-079). It is the input
for an HTML mockup that the maintainer will produce, and then for the pass itself.
It is read together with:

- [`INFORMATION_ARCHITECTURE.md`](INFORMATION_ARCHITECTURE.md): what each
  screen contains, in what order, and how the user moves between them;
- [`../DESIGN.md`](../DESIGN.md): the visual system (tokens and component
  rules), adopted by W-079 on 2026-11-08;
- [`../mockup/`](../mockup/): the maintainer's HTML mockups of the 34 screens, with
  the changes they make in `00-what-changed.html`;
- [`../mockup.png`](../mockup.png): the original visual reference, 1672 × 941.

`docs/DESIGN.md` stays the record of what the current build implements
(D-017) until W-079 adopts the target. W-079's first step copies the target
`DESIGN.md` over `docs/DESIGN.md`, then changes `theme.rs` to match.

## Decisions taken for this brief

Asked on 2026-11-08; answers by the maintainer.

| Question | Answer |
|---|---|
| Where the documents live | `docs/project/`, with the other project documents |
| Mockup density versus today's tokens | The mockup's roomier density, snapped to the 4-pixel grid |
| Mockup features the app lacks | All four: free space in the status line, a thumbnail in Properties, "Details…" on the error banner, icons and shortcuts in menus |
| Screens the HTML mockup covers | All four groups: main window in the three views; split view and narrow sizes; every dialog; states and opaque mode |

Locked decisions still apply: the theme is dark only (L-009 is later), Wayland
and KDE Plasma only, no blur inside the window (D-017), the toolkit is pinned
(D-001/D-002), place groups live in Rustphin's own configuration (D-008), and
nothing in the design may need a network feature beyond kio-fuse paths.

## Problem

People who live in their file manager move through large folders all day:
they open, compare, copy, rename and clean up. Dolphin does all of it, but
its window is busy with toolbars and panels, its look is generic, and the
quickest way to an action is often a menu hunt. Rustphin already does the
work (it opens 500,000 entries in well under 200 ms), but its window still
reads as a dense prototype: 28-pixel rows, flat controls, text crammed into
the chrome, and states that are correct but plain. The user should feel at
ease in a window they look at for hours, and should never wonder where a
command went.

## Solution

One calm, dark glass window where the files are the brightest thing on
screen. The places panel on the left says where you can go; tabs and an
outlined toolbar say where you are; the list fills the rest. Everything else
appears only when it has something to say: an error banner with its details,
a progress row while something is copying, a status line that counts what is
selected and how much room is left. Every command lives in one searchable
Actions palette and in the right-click menu, each showing its shortcut, so a
newcomer finds it by reading and an expert by typing.

## Experience Principles

1. **The files lead, the chrome follows.** File names are the highest-contrast
   text; chrome is quieter and recessed. If a control competes with the list,
   it moves into Actions.
2. **Room to read over raw density.** 36-pixel rows and 15-pixel names by
   default, because the window is read for hours. Zooming out still reaches
   today's 28-pixel rows for people who want density.
3. **Say it in words, show it once.** Every state, error and question is a
   plain sentence with one clear way forward. Colour always comes with an
   icon, a shape or text. Each command has one label everywhere: in the menu,
   the palette and the tooltip, with the same shortcut.

## Aesthetic Direction

- **Philosophy:** "obsidian glass": a dark, translucent window over the
  compositor-blurred desktop, layered as stacked planes with hairline edges.
  It feels like an instrument (precise, geometric, quiet), not a toy.
- **Tone:** calm, confident, focused. Urgent only when something failed.
- **Reference points:** `docs/mockup.png` above everything else; KDE Plasma's
  Breeze Dark for platform fit; the clarity of Zed and Linear for typography
  and spacing; Dolphin for behaviour.
- **Anti-references:** gradients and soft drop shadows; neon or a rainbow of
  accents; pill buttons and big rounded cards; glassmorphism that blurs
  content inside the window; Windows-style ribbons; dense IDE toolbars full
  of icons.

## Existing Patterns

From `docs/DESIGN.md` and `crates/rustphin-ui/src/theme.rs` (the current
build). The target `DESIGN.md` keeps the system and changes values.

- **Typography:** Inter for names and interface text, JetBrains Mono for
  sizes, dates, paths and counts, Space Grotesk for state headings. Today:
  names 14 px, data 13 px, status 12 px, smallest 11 px.
- **Colours:** obsidian surfaces (`#0B0F17` window), royal-blue selection
  (`#3457D5`), bright accent `#8FA8FF` for focus bars and rings, cyan for live
  activity, emerald for success, salmon `#FF9B9B` on `#3A1A1F` for errors.
  Glass and solid variants of each surface; solid raised surfaces for
  anything floating. Text pairs are contrast-tested in both modes.
- **Spacing:** 4-pixel grid (4, 8, 12, 16, 24, 32).
- **Shape:** one 2-pixel radius everywhere; hairline borders, no shadows
  except the tight accent glow on floating surfaces.
- **Components:** toolbar with Back/Forward/Up, breadcrumb location and the
  labelled Actions button; tab strip; split panes; three views (details,
  icons, compact); places panel with groups and devices; context menu with an
  "Open with" submenu; command palette; dialogs for properties, batch rename,
  conflicts, failures, running programs, empty trash and permanent delete;
  activity lines with progress; trash banner; filter bar; empty, loading and
  hidden-only states; rename field; drag chip; rubber band.
- **Icons:** Lucide through GPUI-Kit for interface icons; the system icon
  theme for file types; thumbnails from the shared freedesktop cache.

## What changes in the target

Measured on `docs/mockup.png` (rows about 39 px apart, name cap height
12 px), then scaled so a row is 36 px:

| Aspect | Today | Target |
|---|---|---|
| Entry rows (details, compact, default zoom) | 28 px, 20 px icons | 36 px, 24 px icons; zoom levels give 28, 32, 36, 44, 60 px |
| Thumbnails | from 32 px icons (icons view) | also in the details and compact views at 24 px, as the reference shows |
| File names and menu items | Inter 14 px | Inter 15 px |
| Data (sizes, dates, type column) | Mono 13 px; type in Inter | Mono 14 px, type column in mono too |
| Toolbar | 44 px, flat 28 px icon buttons | 52 px, outlined 36 px buttons, outlined location field with a folder icon |
| Tab strip | 32 px, only with two or more tabs | 40 px, tabs with icon, label and close button; still only with two or more tabs |
| Places panel | 220 px, 28 px rows | 240 px, 32 px rows, a "Places" title, collapsible Devices and network groups |
| Status line | 28 px, item counts | 32 px, counts and sizes on the left, free space of the volume on the right |
| Activity line | progress 4 px, 160 px wide | progress 6 px, flexible width, counts and speed in mono, outlined Stop |
| Error banner | sentence and dismiss | sentence, outlined "Details…" button, dismiss |
| Context menu | labels only | leading icon, label, shortcut on the right |
| Palette | results with category and shortcut | adds a title row ("Actions" and its shortcut), leading icons, category as a "View › Show hidden files" trail |
| Properties | 440 px, facts and permissions | 480 px, title row with close button, thumbnail header with name and type, location and exact byte size |
| Surfaces | neutral obsidian `#0B0F17` | navy obsidian `#0D1524`, matching the mockup's tint |
| Selection | `#3457D5` | `#2D4CC8`; the current place in the panel uses a subtle blue `#1D2E6A` |

## Component Inventory

| Component | Status | Notes |
|---|---|---|
| Window frame and title | Exists | Drawn by KWin (server-side decoration). The mockup may show it for context; the title reads "<folder> — Rustphin" |
| Tab strip and tab | Modify | 40 px; type-icon, label, close; active tab joins the toolbar surface with a 2 px accent underline; "+" after the last tab |
| Toolbar | Modify | Outlined icon buttons, outlined location field, "Actions ⌄" button with the shortcut beside it |
| Location field | Modify | Folder icon, the path in mono as clickable segments; Ctrl+L turns it into a text field with everything selected |
| Error banner | Modify | Adds "Details…" (outlined, error colour) |
| Error details dialog | New | Lists each entry that failed and why; Copy and Close |
| Column headings | Modify | 32 px, sort arrow beside the active column |
| Entry row (details) | Modify | 36 px, 24 px icon or thumbnail, mono data, "—" for folder sizes |
| Entry cell (icons, compact) | Modify | Same rules at the new sizes |
| Places panel | Modify | Title, current-place fill, collapsible sections with counts |
| Device row | Modify | Name and mount point, 4 px usage bar, free/total in mono, outlined eject button |
| Context menu | Modify | Icons and shortcuts; disabled items dimmed |
| "Open with" submenu | Modify | Application icons |
| Command palette | Modify | Title row, icons, trail-style categories |
| Properties dialog | Modify | Title row, thumbnail header, location and bytes, three permission columns |
| Batch-rename dialog | Modify | New sizes; layout kept |
| Conflict, failure, run, empty-trash and delete dialogs | Modify | New sizes; title, scrolling body, wrapping action row kept |
| Activity line | Modify | Larger progress, mono figures, outlined Stop |
| Result line (with Undo) | Modify | New sizes |
| Status line | Modify | Two zones: selection and folder totals; free space |
| Filter bar | Modify | New sizes |
| Trash banner | Modify | New sizes |
| Empty, loading, hidden-only, unreadable states | Modify | New type sizes; same wording rules |
| Blur-unavailable status | Exists | Unchanged wording |
| Rename field, drag chip, rubber band | Exists | New sizes only |
| Tooltip | Exists | Shows the command's label and shortcut |
| Checkbox, button (primary, outlined, destructive, ghost) | Modify | Outlined buttons gain a visible `border-strong` outline; heights 32 px |

## Key Interactions

- **Selecting:** click selects one; Ctrl and Shift extend; a drag on empty
  space draws the rubber band. The selected row gets the primary fill, light
  text and a 2 px bright bar on its left edge; a focused, unselected row keeps
  only the bar. The status line updates at once: "1 item selected (6.3 MiB)".
- **Right-click:** a solid menu opens at the pointer (at the focused entry from
  the keyboard). The row under the pointer or keyboard cursor takes the
  selection look. Disabled items stay visible, dimmed, and are skipped.
- **Actions palette (Ctrl+Shift+P, or the Actions button):** the window dims;
  the palette appears near the top with its field focused. Typing filters
  actions, places, tabs and recent folders. Up/Down moves the selection, Enter
  runs, Escape closes and returns focus where it was.
- **Copying or moving:** after 300 ms an activity line appears above the
  status line with a sentence, the progress bar, "6 of 12", bytes and speed,
  and Stop. When it ends the line becomes a one-line result with Undo, which
  leaves after 6 seconds unless it reports a problem.
- **Something failed:** a banner appears under the toolbar with the sentence,
  "Details…" and a close button. The list stays as it was. Details opens a
  dialog listing every entry and its reason.
- **A decision is needed** (name conflict, a failed entry, running a program,
  emptying the trash, deleting for good): a dialog with a short title, the
  full question in a scrolling body and a right-aligned action row; every
  button shows its key; the safe choice is the default.
- **Properties (Alt+Enter):** a dialog with the thumbnail, facts and
  permissions. Ticking a box enables Apply; Apply can be undone.
- **Filtering (Ctrl+I or /):** a bar opens above the status line; the list
  narrows as you type; Escape clears and closes.
- **Hover:** rows, places, tabs, menu items and buttons show the hover fill.
  Nothing is reachable only by hover.
- **Glass and opaque:** when the compositor offers blur, the window is glass;
  when it does not, or the user turns it off, surfaces are solid and a status
  line says why (only when blur is unavailable, not when switched off).

## Responsive Behavior

A desktop window, not a web page: the breakpoints are pane widths and window
sizes. The minimum window is 480 × 320; the default is 960 × 640; the mockup's
window is about 1060 × 770 at the target sizes.

| Pane width | What changes |
|---|---|
| ≥ 820 px | All columns: name, size (96 px), modified (168 px), type (160 px) |
| 600–819 px | Type column hidden |
| 360–599 px | Modified column hidden |
| < 360 px | Size column hidden; the name keeps the width |
| < 400 px | Forward and Up leave the toolbar (still in Actions and on their keys); "Actions" becomes an icon button with a tooltip |
| < 300 px | Back leaves the toolbar too |

- **Split view:** two equal panes with a 1 px divider; each pane applies the
  table above on its own width. The pane that takes keyboard input has a 2 px
  bright line along its top.
- **Places panel:** 240 px; F9 hides it. The window does not hide it by itself.
- **Location:** at narrow widths the leading folders collapse into "…" and the
  last folders stay visible.
- **Dialogs** keep at least 16 px from every window edge, keep their title and
  action row visible at 480 × 320, and scroll the body between them. Action
  rows wrap and stay right-aligned.
- **Status and activity lines** truncate their sentence before their figures
  and buttons; the free-space zone leaves first when space runs out.

## Accessibility Requirements

- **Contrast:** all text at least 4.5:1 (WCAG AA) on every surface it can sit
  on, in the opaque theme and in the glass theme over a white window; icons,
  focus indicators, the usage bar and the progress bar at least 3:1. The target
  token set was checked with a script on 2026-11-08: the lowest text pair is
  muted text on bare window glass over white, 4.92:1. W-079 adds every new pair
  to the tests in `theme.rs`.
- **Never colour alone:** selection has a bar, sorting an arrow, errors an icon
  and words, disabled items are skipped and say so to assistive technology,
  usage bars have numbers beside them.
- **Keyboard:** every command has a key or a palette entry (R-004, `KEYBOARD.md`);
  focus is always visible (bright bar on rows, ring on fields and buttons); a
  dialog traps focus until closed and returns it where it was.
- **Assistive technology:** names and roles through AccessKit as today
  (W-074): rows, places, tabs, menus, palette, dialogs and fields. No Orca
  pass is required (D-023).
- **Sizes:** nothing below 11 px; click targets at least 28 px tall, 32 px for
  buttons. Every icon-only button has a tooltip with its label and key.
- **Motion:** no animation is needed to understand anything; transitions, if
  any, are under 150 ms.

## The HTML mockup

What the maintainer's mockup should contain so W-079 can follow it:

- One page per screen listed in `INFORMATION_ARCHITECTURE.md` (section
  "Screens for the mockup"), or one page with a switcher; each at the sizes
  given there.
- Colours, sizes, spacing and type taken from the target `DESIGN.md` tokens,
  ideally as CSS custom properties named after them, so values can be compared
  one to one with `theme.rs`.
- Generated, realistic data (folder and file names, sizes, dates in
  `YYYY-MM-DD HH:MM`), never real personal files; the user name in paths is
  `alex` as in the reference.
- The glass version over a photograph (as in the reference) and the opaque
  version on its own.
- Fonts: Inter, JetBrains Mono, Space Grotesk; icons: Lucide.
- Anything it adds or changes from the target `DESIGN.md` is noted, so W-079
  can update the document before the code.

## Out of Scope

- A light theme or following the system colour scheme (L-009).
- Features not in the MVP: search, terminal panel, information panel,
  service menus, version-control overlays, tags and ratings, archives, quick
  preview, Miller columns (L-001 to L-014).
- The window title bar, which belongs to KWin.
- Network browsing: the reference's "Network (2)" is a place group holding
  network bookmarks that open through kio-fuse; there is no network
  neighbourhood view.
- Blur of anything inside the window, and custom shadows other than the
  floating-surface glow.
- Changing keyboard shortcuts, operation behaviour or wording rules, except
  where this brief says so (free space, Details, menu icons and shortcuts,
  thumbnails in Properties and in the list).
- Animations and transitions beyond simple state changes.
