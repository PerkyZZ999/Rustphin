---
version: alpha
name: Rustphin Charcoal
description: Charcoal glass with a royal-blue accent on default buttons and glows for the Rustphin file manager on KDE Plasma Wayland (Qt 6 and QML). Adopted by the W-079 design pass from the HTML mockups in docs/mockup/; the tokens equal crates/rustphin-view/src/colors.rs and metrics.rs.
colors:
  # Surfaces over the compositor-blurred desktop (glass mode). Stacked: window, then chrome or content.
  window-glass: "rgba(20, 21, 22, 0.80)"
  chrome-glass: "rgba(24, 25, 27, 0.50)"
  chrome-raised-glass: "rgba(32, 33, 36, 0.55)"
  content-glass: "rgba(17, 18, 19, 0.62)"
  # Solid replacements when blur is unavailable or switched off (opaque mode).
  window-solid: "#141516"
  chrome-solid: "#18191B"
  chrome-raised-solid: "#202124"
  content-solid: "#111213"
  # Menus, palette, dialogs: always solid.
  surface-raised: "#1D1E21"
  surface-panel: "#232427"
  scrim: "rgba(0, 0, 0, 0.60)"
  # Text.
  text: "#ECECED"
  text-muted: "#AEB1B8"
  text-disabled: "#7D8087"
  # Royal blue, the one accent: default buttons, outlines and glows.
  primary: "#4169E1"
  primary-hover: "#3A5DC8"
  on-primary: "#FFFFFF"
  primary-bright: "#7B98F2"
  progress: "#7B98F2"
  border-accent: "rgba(65, 105, 225, 0.55)"
  glow: "rgba(65, 105, 225, 0.20)"
  # Neutral states: selection, the current entry, pressed controls.
  selection: "#3B3D42"
  selection-hover: "#45474D"
  on-selection: "#F5F5F6"
  current: "#2A2C30"
  hover: "rgba(255, 255, 255, 0.06)"
  # Status signals.
  tertiary: "#10B981"
  error: "#FF9B9B"
  error-surface: "#3A1F22"
  error-border: "rgba(255, 155, 155, 0.45)"
  destructive: "#C42F2F"
  destructive-hover: "#AB2929"
  # Lines and tracks.
  border: "rgba(255, 255, 255, 0.07)"
  border-strong: "rgba(255, 255, 255, 0.14)"
  track: "#34363B"
typography:
  headline-md:
    fontFamily: Space Grotesk
    fontSize: 15px
    fontWeight: 600
    lineHeight: 20px
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
  body-sm:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: 400
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
  body-md-strong:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 600
    lineHeight: 16px
  label-sm:
    fontFamily: Inter
    fontSize: 10px
    fontWeight: 400
rounded:
  sm: 2px
spacing:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
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
  window-line-glass:
    backgroundColor: "{colors.chrome-raised-glass}"
  modal-scrim:
    backgroundColor: "{colors.scrim}"
  title-bar:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    height: 36px
  window-button:
    textColor: "{colors.text}"
    width: 44px
    height: 36px
  tab-strip:
    backgroundColor: "{colors.chrome-solid}"
    height: 36px
  tab:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    padding: "{spacing.md}"
    width: 160px
  tab-narrowest:
    width: 96px
  tab-active:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
  tab-close:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    size: 24px
  tab-add:
    textColor: "{colors.text}"
    width: 36px
  toolbar:
    backgroundColor: "{colors.chrome-solid}"
    height: 44px
    padding: "{spacing.sm}"
  toolbar-button:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    size: 32px
  toolbar-menu-button:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 48px
    height: 32px
  toolbar-toggle-on:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.text}"
  toolbar-button-hover:
    backgroundColor: "{colors.hover}"
  toolbar-button-disabled:
    textColor: "{colors.text-disabled}"
  toolbar-location-min:
    width: 280px
  location-field:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.sm}"
  path-bar:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text-muted}"
    typography: "{typography.data-md}"
    height: 28px
    padding: "{spacing.md}"
  path-bar-active:
    backgroundColor: "{colors.chrome-raised-solid}"
    textColor: "{colors.text}"
  admin-bar:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.error}"
    typography: "{typography.body-md}"
    height: 36px
    padding: "{spacing.md}"
  banner-error:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 36px
    padding: "{spacing.md}"
  trash-banner:
    backgroundColor: "{colors.chrome-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    padding: "{spacing.sm}"
  places-panel:
    backgroundColor: "{colors.chrome-solid}"
    width: 240px
  places-title:
    textColor: "{colors.text-muted}"
    typography: "{typography.label-sm}"
    height: 36px
    padding: "{spacing.md}"
  place-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.md}"
  place-row-current:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
  section-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.label-sm}"
    height: 28px
  place-separator:
    backgroundColor: "{colors.border-strong}"
    height: 12px
  device-card:
    textColor: "{colors.text}"
    typography: "{typography.body-md-strong}"
    rounded: "{rounded.sm}"
    padding: "{spacing.sm}"
    height: 56px
  device-row:
    height: 60px
  drive-compact-row:
    height: 28px
  drive-compact-bar:
    backgroundColor: "{colors.track}"
    width: 48px
    height: 4px
  drive-ring-row:
    height: 48px
  place-ring:
    backgroundColor: "{colors.track}"
    size: 32px
  drive-button:
    rounded: "{rounded.sm}"
    size: 24px
  device-figures:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  device-traits:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-sm}"
  usage-bar:
    backgroundColor: "{colors.progress}"
    height: 4px
  usage-bar-full:
    backgroundColor: "{colors.error}"
    height: 4px
  usage-track:
    backgroundColor: "{colors.track}"
    height: 4px
  drive-ring:
    backgroundColor: "{colors.track}"
    size: 112px
    width: 12px
  drive-ring-used:
    backgroundColor: "{colors.progress}"
  drive-ring-full:
    backgroundColor: "{colors.error}"
  column-heading:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-sm}"
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
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.xs}"
  entry-row-hover:
    backgroundColor: "{colors.hover}"
  entry-row-selected:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
  entry-row-selected-hover:
    backgroundColor: "{colors.selection-hover}"
    textColor: "{colors.on-selection}"
  entry-row-current:
    backgroundColor: "{colors.current}"
    textColor: "{colors.text}"
  entry-data-cell:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-md}"
  entry-type-cell:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-sm}"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.md}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.on-primary}"
  button-outlined:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.md}"
  button-destructive:
    backgroundColor: "{colors.destructive}"
    textColor: "{colors.on-primary}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.md}"
  button-destructive-hover:
    backgroundColor: "{colors.destructive-hover}"
    textColor: "{colors.on-primary}"
  input:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 32px
    padding: "{spacing.sm}"
  note-neutral:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    padding: "{spacing.sm}"
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
  menu-item-highlighted:
    backgroundColor: "{colors.hover}"
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
    padding: "{spacing.sm}"
  palette-field:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    height: 40px
  palette-result:
    typography: "{typography.body-md}"
    height: 52px
    padding: "{spacing.md}"
  palette-result-selected:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
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
  dialog-actions:
    backgroundColor: "{colors.chrome-solid}"
    padding: "{spacing.md}"
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
  compress-dialog:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 560px
  settings-dialog:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 800px
    height: 576px
  settings-window-min:
    width: 480px
    height: 360px
  settings-page-list:
    width: 216px
  settings-search:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    width: 240px
    height: 28px
  settings-page-current:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
  settings-section-title:
    textColor: "{colors.text}"
    typography: "{typography.body-md-strong}"
  settings-panel:
    backgroundColor: "{colors.surface-panel}"
    rounded: "{rounded.sm}"
  setting-row:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    padding: "{spacing.lg}"
    height: 48px
  setting-hint:
    textColor: "{colors.text-muted}"
    typography: "{typography.body-sm}"
  setting-control:
    height: 28px
  setting-select:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 216px
    height: 28px
  setting-switch:
    backgroundColor: "{colors.track}"
    rounded: "{rounded.sm}"
    width: 36px
    height: 20px
  setting-switch-on:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
  setting-segment-chosen:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
  setting-slider:
    backgroundColor: "{colors.track}"
    rounded: "{rounded.sm}"
    width: 160px
  setting-slider-value:
    backgroundColor: "{colors.content-solid}"
    textColor: "{colors.text}"
    typography: "{typography.data-md}"
    rounded: "{rounded.sm}"
    width: 64px
  setting-chip:
    textColor: "{colors.text-muted}"
    rounded: "{rounded.sm}"
    height: 28px
  setting-chip-on:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
  look-tile:
    rounded: "{rounded.sm}"
    height: 64px
  settings-list:
    backgroundColor: "{colors.surface-panel}"
    rounded: "{rounded.sm}"
    height: 224px
  key-cap:
    textColor: "{colors.text}"
    typography: "{typography.data-sm}"
    rounded: "{rounded.sm}"
    height: 20px
  activity-line:
    backgroundColor: "{colors.chrome-raised-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 44px
    padding: "{spacing.md}"
  progress-bar:
    backgroundColor: "{colors.progress}"
    rounded: "{rounded.sm}"
    height: 6px
  progress-track:
    backgroundColor: "{colors.track}"
    rounded: "{rounded.sm}"
    height: 6px
  activity-figures:
    textColor: "{colors.text-muted}"
    typography: "{typography.data-sm}"
  transfer-dialog:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    width: 480px
    padding: "{spacing.lg}"
  transfer-card:
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.sm}"
    padding: "{spacing.md}"
  speed-graph:
    backgroundColor: "{colors.progress}"
    height: 48px
  result-line:
    backgroundColor: "{colors.chrome-raised-solid}"
    textColor: "{colors.text}"
    typography: "{typography.body-md}"
    height: 44px
    padding: "{spacing.md}"
  result-line-success:
    textColor: "{colors.tertiary}"
  result-line-problem:
    backgroundColor: "{colors.error-surface}"
    textColor: "{colors.text}"
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
    backgroundColor: "{colors.content-solid}"
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
  scrollbar-thumb:
    backgroundColor: "{colors.text-muted}"
    rounded: "{rounded.sm}"
    width: 8px
  drag-chip:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.on-selection}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    height: 28px
    padding: "{spacing.md}"
---

# Rustphin design

This is the design reference for Rustphin (D-017; its colours D-038). The tokens in the front
matter are authoritative. `crates/rustphin-view/src/colors.rs` implements the
colours (with the contrast tests) and `metrics.rs` the spacing, sizes and type
scale; `crates/rustphin/src/tokens.rs` hands both to QML as the `Tokens`
singleton (`Tokens.colors.<name>`, `Tokens.metrics.<name>`), and the screens in
`crates/rustphin/qml/` draw with nothing else. Change this document first,
then the tokens, and keep "Implementation status" true.

Token names in code differ from the names here; the mapping is:

| Here | `colors.rs` / QML | Here | `colors.rs` / QML |
|---|---|---|---|
| `window-*` | `window` | `primary` | `button_primary` |
| `chrome-*` | `chrome` | `primary-hover` | `button_primary_hover` |
| `chrome-raised-*` | `chrome_raised` | `on-primary` | `on_button` |
| `content-*` | `content` | `primary-bright` | `accent` |
| `surface-raised` | `raised` | `progress` | `activity` |
| `selection` | `selection` | `destructive` | `button_destructive` |
| `selection-hover` | `selection_hover` | `destructive-hover` | `button_destructive_hover` |
| `on-selection` | `on_selection` | `error` | `danger` |
| `current` | `current` | `error-surface` | `danger_surface` |
| `tertiary` | `success` | `error-border` | `danger_border` |
| `border-strong` | `border_strong` | `text-disabled` | `text_disabled` |
| `border-accent` | `border_accent` | `glow` | `glow` |

`Colors::charcoal(translucent)` picks the glass or the solid value of each
surface; every other colour is the same in both.
In `metrics.rs`, `space::XS…LG` are the spacing tokens, `size::RADIUS` is
`rounded.sm`, `text::BODY`, `NAME`, `SMALL`, `DATA`, `LABEL`, `TITLE` and
`HEADING` are the type sizes (`text::LINE`, `HEADING_LINE`, `TITLE_LINE` the
line heights) and the `size` constants are the component sizes below.

The visual reference is the set of HTML mockups in [`mockup/`](mockup/), one
page per screen, numbered as in
[`project/INFORMATION_ARCHITECTURE.md`](project/INFORMATION_ARCHITECTURE.md),
with the notes in `mockup/00-what-changed.html`. The decisions behind them are
in [`project/DESIGN_BRIEF.md`](project/DESIGN_BRIEF.md). The original
reference picture is [`mockup.png`](mockup.png); colours marked "measured"
were taken from it. The mockups were drawn at the W-079 density; D-026 and
W-096 have since made the type and the rows smaller, and they draw the navy
colours that D-038 replaced with charcoal, so where a mockup and this
document disagree on a size or a colour, this document holds.

## Overview

Rustphin is a file manager for KDE Plasma, used every day by people who move
through large folders quickly. The interface is a precise instrument:
geometric, legible and calm, with nothing decorative competing with the files.

Three ideas hold it together:

- **Charcoal glass.** The window is dark charcoal glass over the desktop, which the
  compositor blurs (KDE's `KWindowEffects`, asked for by the C++ shell). Chrome
  (title bar, toolbar, tabs, places, status) is recessed glass; the file list
  sits on a more solid pane so columns of text stay readable over anything
  behind.
- **Royal blue, used sparingly.** Everything else is charcoal, black and
  grey. Royal blue is the one accent and appears only on the default button,
  on focus rings, on the outline and glow of menus and dialogs, as the 2 px
  edge of a selection (and of the current place and the active tab), and as
  the progress bar. Emerald marks success; salmon and red mean errors and
  destructive actions.
- **Progressive disclosure.** The default window shows places, the location,
  the files and a status line. Banners, progress, notices and the filter
  appear when they have something to say; every command is in the Actions
  palette with its key.

The density is compact: 32-pixel rows with 24-pixel icons and 12-pixel names
at the default zoom, on a 4-pixel grid, with hairline dividers instead of
boxes. Zooming out reaches 24-pixel rows.

## Colors

- **Glass surfaces** (`window-glass`, `chrome-glass`, `chrome-raised-glass`,
  `content-glass`) are neutral charcoal at partial opacity, stacked: window
  first, then chrome or content on top of it. Never put text directly on bare
  `window-glass`; it is the backdrop for the others. The tints carry no hue
  (D-038): `#111213` list, `#18191B` chrome and places, `#202124` tab strip.
- **Solid surfaces** (`window-solid`, `chrome-solid`, `chrome-raised-solid`,
  `content-solid`) replace the glass in opaque mode: when Settings,
  Appearance, Look is Opaque, when `RUSTPHIN_OPAQUE` is set to anything but
  `0`, or when KWin does not offer blur (`settings::Look::theme`). The choice
  is made again whenever the settings change and whenever a window becomes
  the active one, so blur that appears or goes while Rustphin runs is
  followed. Every rule in this document holds in both modes. When glass is
  wanted but blur is unavailable (not when it is switched off), a status
  line says so (see Blur status).
- **Chrome raised** (`chrome-raised-*`) is the window lines at the bottom
  (activity and result) and the active side's path bar in split view: one
  step lighter than the chrome, so they stand out from the status lines.
- **Raised surface** (`surface-raised`) is for anything that floats inside the
  window: menus, the View menu, the palette, dialogs. Always solid; nothing is
  blurred inside the window.
- **Panel** (`surface-panel`, `colors.rs` `panel`) is one step lighter than
  the raised surface: the panels that hold a Settings page's sections, and
  the lists of its Toolbar page (W-155). Solid, like every surface inside a
  dialog.
- **Text:** `text` for names and primary content; `text-muted` for data
  columns, inactive tabs, headings, shortcuts and the status line;
  `text-disabled` only for disabled controls and the toolbar's quiet hints
  (menu chevrons). `text` and `text-muted` meet 4.5:1 on every
  surface, including glass over a white window; `text-disabled` meets 3:1.
  The tests in `colors.rs` check every pair, glass over white and opaque.
- **Royal blue** has exactly these uses and no others:
  - `primary` (`#4169E1`) fills a dialog's default button, with `on-primary`
    (white) words, 4.9:1; `primary-hover` under the pointer or pressed.
  - `primary-bright` (`#7B98F2`, royal blue lightened just enough to reach
    3:1 on every surface and on the selection fill) draws the lines: focus
    rings on buttons, fields and checkboxes (2 px), the focused entry's or
    place's 1 px outline, the 2 px selection edge, the active tab's top edge,
    the active split pane's path-bar underline, the outline of a drop target
    and of the rubber band.
  - `border-accent` (royal blue at 55 %) is the 1 px outline of menus, the
    View menu, the palette and dialogs, and `glow` (royal blue at 20 %) the
    2 px halo just outside it.
  - `progress` (the same lightened blue) is the filled part of the activity
    line's progress bar and of a drive card's usage bar (W-148).
  No text, interface icon, check mark or large fill is blue.
- **Selection is neutral** (`selection`, `#3B3D42`, with `on-selection` text,
  10:1): it fills the selected entries, the current place in the places
  panel, the selected palette result, the current Settings page, a list's
  cursor row, the chosen segment in dialogs, a checked or pressed icon
  button, a pressed outlined button and the place drag chip.
  `selection-hover` is a selected entry or palette result under the pointer.
  Selected entries, the current place, the selected palette result and the
  current Settings page also carry a 2 px `primary-bright` edge on their
  leading side, and the chosen segment one along its bottom, so the state is
  never told by a grey alone. `current` (`#2A2C30`) is the quiet fill of the
  focused entry when it is not selected.
- **Hover** is white at 6 %, under the pointer on rows, buttons and tabs.
- **Usage bars and rings** of drives are `progress` on `track`; a volume
  90 % full or more draws them in `error`, and its line says "Almost full",
  so the warning is never colour alone. **Tertiary** (emerald) marks a
  completed operation. Neither is decoration.
- **Error** text and icons on `error-surface` (a dark red-grey) with an
  `error-border` outline and a warning icon; colour never carries the message
  alone. Used by the folder's re-read banner, a notice that reports a
  problem, the administrator bar, the Close button's hover in the title bar
  and a folder that cannot be opened. **Destructive** (`#C42F2F`, white words,
  5.5:1; `destructive-hover` under the pointer) fills a destructive default
  button such as "Delete for good".
- **Borders** are hairlines: `border` between regions and as menu
  separators; `border-strong` around outlined controls, text fields and
  checkboxes, and as the window's own frame when not maximised.
- **File-type icons** come from the system icon theme (`image://theme`); they
  are not tokens.

## Typography

Three families, each with one job. They are bundled
(`crates/rustphin/fonts/`: Inter 400 and 600, JetBrains Mono 400, Space
Grotesk 600; no token uses another weight) and registered by the C++ shell at start.

- **Inter** for names and interface text. `body-lg` (12 px) for file names
  in every view; `body-md` (12 px) for place names, tabs, menu items, buttons,
  banners, palette result names, dialog and Settings text; `body-sm` (11 px)
  for column headings, the type column, the trash banner's sentence, problem
  reasons, notes in the places panel, the palette's status line and the drag
  chip; `label-sm` (10 px, written in capitals) for short labels: the places
  panel's "PLACES" title and its section headings,
  OWNER, GROUP and OTHERS in Properties, and SIZE in the View menu.
- **JetBrains Mono** for data compared down a column or read character by
  character: `data-md` for sizes and dates, the location crumbs, the split
  pane's path bar, dialog facts, Settings values and the problem list's paths;
  `data-sm` for the status line, shortcuts in menus and the palette, palette
  trails, activity figures, a drive card's figures ("211.3 / 927.0 GiB",
  "22% used").
- **Space Grotesk** for the few headings, all 15 px semibold: `headline-sm`
  for dialog titles ("Delete for good?", "<name> — Properties", "Compress
  …") and Settings page titles, `headline-md` for empty and error states in
  the pane.
- The current crumb in the location field, the sorted column heading, a
  drive card's name, Settings section titles and the current Settings page
  (`body-md-strong`) are drawn semibold.
- **Text size** (Settings, Appearance) multiplies every interface size by
  11/12 (Smaller) or 14/12 (Larger); names and data in the lists follow the
  zoom table below, scaled the same way. A change shows at once in every
  window (`Tokens.refresh`).

Nothing is smaller than 10 px, and only capital labels are 10 px. Dates are
`YYYY-MM-DD HH:MM` in the list by default (Settings offers "Today 14:32") and
`YYYY-MM-DD HH:MM:SS` in Properties. Sizes use one decimal and IEC units by
default ("6.3 MiB"; Settings offers decimal units); Properties adds the exact
bytes with grouped digits ("6.3 MiB (6,611,968 bytes)"). Folder sizes in the
list show "—" in `text-muted`.

## Layout

Everything sits on a 4-pixel grid; use only the `spacing` tokens.

- **Window, top to bottom (W-096, mockup 35):** Rustphin's own title bar
  (36 px; the window is frameless); the toolbar (44 px) across the whole
  window; the administrator bar (36 px) while acting as administrator; the
  body, which is the places panel and a 1 px divider beside a column made of
  the tab strip (36 px, always shown) over the tab's one or two panes; then
  the window lines across the full width: the activity line (44 px) while an
  operation runs and the notice line (44 px) after one ends or while
  connecting. The places panel runs from under the toolbar to the window
  lines. When not maximised a 1 px `border-strong` frame marks the window's
  edge. The window opens at 1000 × 700 px and cannot be made smaller than
  480 × 320 px.
- **Title bar:** the 16 px logo on the left, the window title ("<folder> —
  Rustphin") centred in `body-md` `text-muted`, then Minimise (`minus`),
  Maximise (`square`; Restore, `copy`, when maximised) and Close window (`x`,
  hover on `error-surface`) as 44 × 36 px icon buttons. Dragging it moves the
  window, a double click maximises or restores. 4 px bands along the edges
  (16 px at the corners) resize the window.
- **Each pane, top to bottom:** the re-read error banner when present; the
  trash banner while a trash is shown; in split view its path bar (28 px); the
  column headings (28 px, details view only) and the list; the filter bar
  (40 px) when open; and the status line (28 px). So at the bottom of the
  window come the panes' status lines first, then the window lines.
- **Tab strip:** each tab a 16 px Lucide icon (the place's icon when the
  folder is a place, `trash` for the trash, else `folder`), the title in
  `body-md`, and a 24 px close button (12 px `x`); "+" (36 px) after the last
  tab; the rest of the strip is `chrome`. Tabs are 160 px; when they do not
  fit, they shrink evenly down to 96 px, then the strip scrolls and keeps the
  active tab in view. The close button shows on the active tab and on the tab
  under the pointer, and only on tabs at least 128 px wide; Ctrl+W and the
  middle button still close any tab. Dragging a tab sideways moves it.
- **Toolbar:** 32 px icon buttons (`toolbar-button`, no outline, the hover
  fill under the pointer) with 4 px between them and 8 px at the ends, in
  groups separated by 20 px `border` hairlines. The default, from
  `toolbar_items::Toolbar::default`: Act as administrator (`shield-alert`) ·
  Back, Forward, Up one folder, Reload · the location field (fills the middle,
  at least 280 px) · New (`plus`) and Open terminal here (`square-terminal`) ·
  View (`list`), Split view, Show hidden files, Filter by name · Command
  palette (`command`) and Settings (`settings`) last, two icon buttons
  (W-154, which replaced the worded Actions button; W-156 put Settings at
  the right end).
  New and View are 48 px menu buttons with a 12 px `chevron-down` in
  `text-disabled`. Show hidden files and Filter by name are toggles: on, they
  take `toolbar-toggle-on`. Each tooltip is the item's name and its key
  ("Back (Alt+Left)"). In split view the toolbar acts on the active pane.
  The items and their order are chosen in Settings, Toolbar; any palette
  command can be added, shown by its palette icon. An item set to "Show its
  words" there draws its name in `body-md` after its icon, 8 px from it and
  from the button's end (a menu button's chevron stays last). When the window is too
  narrow (`Toolbar::fit`), items leave from the right end inwards; the
  location field and the Command palette button always stay. What leaves
  remains in the palette and on its key. Settings can be taken off the
  toolbar (Ctrl+Shift+, and the palette still open it); a toolbar saved with
  the Actions button gets Command palette and Settings in its place.
- **Location field:** 32 px, `content` with a `border-strong` outline. A
  16 px `folder` icon (`trash` in the trash), then the path as crumbs in
  `data-md` separated by 12 px `chevron-right`; the last crumb in `text` and
  semibold, the others `text-muted` with the hover fill. At most four crumbs
  show (three below a 1000 px window, two below 780 px, one below 520 px),
  fewer when they do not fit; the leading ones collapse into "…". Clicking a
  crumb opens it; clicking beside the crumbs, Ctrl+L, Alt+D or F6 turns the
  field into a text field with the whole location selected and a 2 px
  `primary-bright` focus ring. Enter opens what was typed; Escape, leaving the
  field, or Ctrl+L again returns to the crumbs and the list.
- **Path bar (split view):** the side's location in `data-md`, elided in the
  middle; the active side on `chrome-raised` in `text` with a 2 px
  `primary-bright` line along its bottom, the other on `chrome` in
  `text-muted`.
- **Places panel:** 240 px on the left by default (200–360 px; the 4 px grip
  on its right edge drags the width, also set in Settings, Places); F9 hides
  it. A 36 px title row: "PLACES" in `label-sm` `text-muted` and a 28 px
  "New group" icon button (`list-plus`, tooltip "New group (Ctrl+Shift+G)").
  Place rows and section headings 28 px, place icons 16 px, a 12 px inset
  (24 px for a place inside a group); drives 56 px tall with 4 px between
  them in the bars style (`size::DEVICE_CARD`, `DEVICE_ROW`), 28 px in the
  compact style and 48 px in the rings style (`DRIVE_RING_ROW`), with 16 px
  icons. The
  panel scrolls on its own.
- **Details view:** 28 px heading row (Name, Size, Modified, Type); columns
  name (fills), size (96 px, right-aligned), modified (168 px), type (160 px),
  each data column with 8 px on both sides. Rows follow the zoom table (32 px
  with 24 px icons by default). Below 820 px of pane width the type column
  goes, below 600 px the modified column, below 360 px the size column;
  Settings, Views can also turn each column off.
- **Zoom:** seven levels (`layout.rs`) with 16, 20, 24, 28, 32, 40 and 48 px
  icons in the details and compact views; rows are the icon plus 8 px, at
  least 24 px (24, 28, 32, 36, 40, 48, 56). The default is the 24 px level.
  Names are 11, 12, 12, 13, 14, 15 and 16 px; sizes, dates and types 10, 11,
  11, 12, 12, 13 and 14 px. Column headings keep their size. The icons view
  uses 48, 64, 96, 112, 128, 160 and 192 px icons in cells 40 px wider and
  64 px taller than the icon, gap included (at 96 px: 128 × 152 px cells 8 px
  apart); the name wraps under the icon, centred. The compact view fills
  columns top to bottom, as many rows as fit the pane's height, and scrolls
  sideways; its column pitch is 232–296 px by level (240 px columns and an
  8 px gap at the default). Up and Down step through the entries, Left and
  Right change columns, as in Dolphin.
- **View menu (W-107):** the View button opens a 240 px popup on the
  context menu's surface under the button: the three modes as 32 px rows
  (icon, name, key: Icons `layout-grid` Ctrl+1, Compact `columns-3` Ctrl+2,
  Details `layout-list` Ctrl+3), the current one with a `text` `check`; a hairline; then a row with "SIZE" in `label-sm`, the level's icon
  size in `data-sm` ("24 px") and two icon buttons, Smaller (`zoom-out`,
  Ctrl+−) and Larger (`zoom-in`, Ctrl+=). Choosing a mode closes the popup;
  the size buttons apply at once; Escape or a click elsewhere closes it.
- **New menu (W-108):** as Dolphin's "Create New", the context menu opened
  under the New button: Folder… (`folder-plus`); a hairline; Text File…
  (`file-text`), Empty File… (`file`), HTML File… (`file-code`), then the
  user's templates (the files in `XDG_TEMPLATES_DIR` or `~/Templates`, and
  KDE template entries) by name with `file`. Choosing one creates it with a
  free name ("Text File.txt", "Text File 2.txt") and opens the name for
  editing, as New folder does. KDE's "Link to…" entries are not offered.
  **Open terminal here** opens the terminal chosen in Settings (KDE's when
  none) in the folder shown.
- **Split view:** two equal panes with a 1 px `border` divider; each applies
  the column rules to its own width.
- **Status line:** on `chrome` under a hairline, in `data-sm`: the selection
  ("1 item selected (6.3 MiB)") in `text`, the folder's totals ("12 items
  (150.4 MiB)", hidden counts and filter matches) in `text-muted` (a folder
  notice replaces them in `error`), and on the right the free space ("Free:
  1.8 TiB of 3.6 TiB (Data)") for the volume holding the folder. Below 600 px
  of pane width the free space leaves; below 440 px, while entries are
  selected, only the selection shows.
- **Dialogs:** centred over the scrim, at most the window's width less 16 px
  on each side; a 48 px title row; the body scrolls; the action row on
  `chrome`, right-aligned. Widths: 480 px (questions, Properties, the
  problems list), 560 px (Compress to…), 640 px (batch rename), 760 × 560 px
  at most (Settings). A question with many buttons grows to fit them.
- **Settings (W-106, D-029, W-155; the maintainer's refined mockup of
  2026-11-15, kept to this document's 2 px corners, type scale and flat
  depth):** a window of its own (`SettingsWindow.qml`, W-157), opened by
  Ctrl+Shift+, and the palette's "Workspace: Settings…"; opening it again
  brings it forward. It belongs to the window that opened it (kept above
  it, closed with it) but does not block it, so a change is seen in the
  window at once; no scrim. It opens at 800 × 576 px and can be moved and
  resized down to 480 × 360 px; a change of text size keeps the size the
  user gave it. It follows the Title bar setting as the main window does:
  with Rustphin's own, it has no system frame, a 1 px `border-accent`
  edge, 4 px resize bands with 16 px corners, and its title row moves it;
  with KDE's, KWin draws its frame, title ("Settings — Rustphin") and Close,
  and the title row holds only the search. Escape closes it (an open
  drop-down takes Escape first) and the keyboard goes back to its window.
  A title row: `settings` in `text-muted`,
  "Settings" in Space Grotesk, a 240 px search field (`search` icon inside,
  "Search settings", a "Ctrl+F" key cap until something is typed) and a Close
  icon button, "Close (Esc)". Ctrl+F anywhere in the window puts the keyboard
  in the search; every word typed must be found in a page's name or its
  keywords (`settings_form::Page::keywords`), the page list shows only the
  pages found, a page the search hides gives way to the first one found, and
  Enter takes the keyboard to the list. Nothing found: "No settings match
  "…". Try another word."
  On the left a 216 px list of pages under `body-sm` `text-muted` headings:
  Rustphin (General `settings`, Appearance `palette`), Browsing (Views
  `layout-grid`, Toolbar `panel-top`, Places `panel-left`), Behaviour
  (Confirmations `circle-check`, Shortcuts `keyboard`); About (`info`) stands
  apart at the foot, under a hairline, with the version in `data-md` at the
  row's end. Rows are 32 px with a 16 px icon and the name; the current page
  in `selection` with `on-selection` semibold words, its icon in
  `primary-bright` and a 2 px `primary-bright` edge on its left; others
  `hover` under the pointer. Up and Down move through the pages shown. The
  list is the first stop of Tab and, while it has the keyboard, the current
  page carries the 1 px `primary-bright` outline of a focused place.
  A page: its name in `headline-sm` and one sentence in `text-muted`, then its
  sections 24 px apart, each a `body-md-strong` title, an optional
  `body-sm` line under it, and its settings in one `surface-panel` panel with
  a 1 px `border` edge and hairlines between the rows. A setting is at least
  48 px tall with 16 px sides: what it is in `body-md` and what it means under
  it in `body-sm` `text-muted` (`error` for a warning) on the left, the control
  on the right; a wide control goes under the words. The controls:
  a **switch** for on or off (36 × 20 px, 2 px corners: `primary` with the
  knob on the right in `on-primary` when on, `track` outlined in
  `border-strong` with the knob on the left in `text-muted` when off, so the
  position tells the state as well as the colour; half opacity when it cannot
  be changed, with a `lock` after the words of a setting that is always on);
  **segments** for two or three short options (28 px, `content` outlined in
  `border-strong`, the chosen one in `selection` with `on-selection` words,
  Left and Right choose); a **drop-down** for longer or more options (216 ×
  28 px, `chevron-down`, its list on `surface-raised` with a `check` in
  `primary-bright` before the chosen option); a **slider** for a number in
  steps (160 px `track` filled in `primary` to a 16 px `text` handle, the
  value in words beside it in a 64 px `data-md` box, told when the handle is
  let go or at each arrow key); **chips** for several on/off settings in one
  row (28 px, on: `selection` with a `check`; off: outlined in `border-strong`
  with `text-muted` words); the **look tiles**, a drawing of the window in
  each look (64 px tall, the chosen one with a 2 px `primary-bright` edge, the
  `current` fill and a `check`); a text field, an outlined button, or a badge
  (`circle-check` and its word in `tertiary`). After a change the page is
  built again and the keyboard goes back to the control it was on.
  Pages with settings end with a 48 px row under a hairline: a `check` in
  `tertiary`, "Changes apply at once." and the worded icon button "Reset
  <page> to defaults" (`rotate-ccw`). Changes apply at once.
  - **General:** "Starting" (When Rustphin starts: Restore the last tabs,
    Open the home folder, Open a chosen folder, with the chosen folder and
    "Use the folder shown"; Folders opened by other applications: In a new
    tab, In a new window); "Desktop" (Default file manager: the state and
    "Make Rustphin the default"; Terminal: KDE's, or a command with "Use
    KDE's" and "Use another…"; Open files and folders with: a single or a
    double click, "As KDE is set" until chosen); "Background" (Keep Rustphin
    running after its last window closes; Also start it when you log in, with
    the memory Rustphin uses now).
  - **Appearance:** "Window" (Look: Glass, Opaque, "Blur shows only where the
    desktop offers it."; Title bar: Rustphin's own, KDE's, "Applies to new
    windows."); "Text and files" (Text size: Smaller, Normal, Larger; Hidden
    files are drawn at: a stepper from 60 % to 100 %).
  - **Views:** "Default view" (View; Size, a stepper showing the icon size;
    Each folder, to remember a view per folder); "Order" (Sort by, Direction A
    to Z or Z to A, Folders first, Show hidden files); "Details view" (the
    Size, Modified and Type columns; Dates; Sizes: Binary or Decimal);
    "Previews" (Show pictures' thumbnails; Largest picture previewed).
  - **Toolbar:** "Preview", the toolbar drawn small as it will look (28 px
    buttons, the location field as a box); picking a button picks it in the
    list. Then "Available" (a search field and the built-in buttons, a
    separator and every palette command) and "On the toolbar" (the location
    field and Command palette marked "Always there"), 224 px lists on
    `surface-panel`, with bordered icon buttons between them to add, take
    off, move left and move right; a double click adds or takes off. Under
    them "Buttons": the switch that shows the chosen button's words.
  - **Places:** "Sections" (Show Devices, Remote; Hidden places: Keep them
    hidden, Show them faded); "Drives" (Style: Bars, Compact, Rings, "How
    each drive is drawn in the places panel.", W-159); "Groups" lists each group with its name as a
    field, "N places" and icon buttons Move up, Move down and Delete
    ("Deleting a group keeps its places.", or "No groups yet. Make one with
    New group in the places panel."); "Panel" (Width, a slider from 200 to
    360 px, "Or drag the panel's edge."). Group names are edited where they
    stand, outlined only under the pointer or while edited.
  - **Confirmations:** "Ask before" (Moving to the trash, off; Deleting for
    good, always on and unavailable; Emptying the trash, on; Closing a window
    with several tabs, off); "Executable files" (Opening one: Ask each time,
    Open it, Run it).
  - **Shortcuts:** "The keys Rustphin understands. Changing them comes
    later." A search field, then a panel per command category of rows: the
    command's name and its keys as 20 px key caps in `data-sm`, "or" between
    them. Only commands
    with a key are listed; no match says "No shortcut matches. Try another
    word."
  - **About:** the 64 px logo, "Rustphin" with the version as a key cap, one
    line on what it is and "Show the licence" (`file-text`; the full text,
    with "Back"); a panel "No telemetry. Rustphin never uses the network."
    with a `shield` in `tertiary`; "Credits" (Licence, Behaviour, Built with,
    Icons, Fonts); "Settings file" with its path in `data-md` and "Open the
    settings folder".

## Elevation & Depth

Depth comes from stacked planes and edge lines, not soft shadows.

- **Level 0, the desktop:** whatever is behind the window, blurred by the
  compositor in glass mode.
- **Level 1, window glass and chrome:** `window-glass` with `chrome-glass`
  (title bar, toolbar, tab strip, places, headings, status, filter bar) and
  `chrome-raised-glass` (the window lines, the active path bar).
- **Level 2, content pane:** `content-glass` under the file views, the
  location field, the active tab and text fields.
- **Level 3, floating surfaces:** `surface-raised`, solid, outlined with a
  1 px `border-accent` line and a 2 px `glow` halo outside it (the one place
  royal blue surrounds a surface; no soft shadow). Modal dialogs and the palette dim the window with
  `scrim`; menus and the View menu do not.

Blur exists only between level 0 and the window. In opaque mode every glass
token becomes its solid counterpart and levels are told apart by their
borders and tones alone.

## Shapes

One radius: `rounded.sm`, 2 pixels (`size::RADIUS`), on rows, cells, buttons,
fields, menus, dialogs, progress and usage bars and the drag chip. The
reference's corners read as 3–4 px at its scale; the 2 px decision (the
maintainer, 2026-10-12) stands. There are no pills and no circles.

## Components

- **Tabs:** inactive tabs `text-muted` on the strip, with the hover fill
  under the pointer; the active tab on `content` in `text`, with a 2 px
  `primary-bright` line along its top. A hairline after each tab. Close
  tooltip "Close tab (Ctrl+W)", "+" tooltip "New tab (Ctrl+T)".
- **Icon buttons** (`IconButton.qml`): a 16 px Lucide icon in `text`
  (`text-disabled` when unavailable) on a 32 px square by default; `hover`
  under the pointer, `selection` while pressed or checked, a 2 px
  `primary-bright` ring on keyboard focus. Every one has a tooltip.
- **Outlined buttons** (`OutlinedButton.qml`): 32 px, 12 px side padding,
  `body-md` in `text`, a 1 px `border-strong` outline; `hover` under the
  pointer, `selection` while pressed, a 2 px `primary-bright` ring on keyboard
  focus. An optional 16 px icon goes 8 px
  before the words, the same icon the action has in menus and the toolbar
  (W-097): Rename `pencil`, Undo `undo-2`, Back `arrow-left`, Up `arrow-up`,
  Home folder `house`, Show hidden `eye`, Clear filter `funnel-x`, Stop
  acting as administrator `shield-off`, Reset to defaults `rotate-ccw`,
  Compress `file-archive`. Cancel, Close and Stop stay words only.
- **Default and destructive buttons:** a dialog's safe default is
  `button-primary`; a destructive default uses `button-destructive`; the
  others are outlined. `button-primary` is `primary` with `on-primary` words
  (`primary-hover` under the pointer or pressed); `button-destructive` is
  `destructive` with `on-primary` words (`destructive-hover`). Keyboard focus
  draws a 2 px `on-primary` ring inside the fill. Every dialog button shows its
  key ("Close (Esc)", "Apply (Enter)", "Skip (S)"). A disabled default keeps
  its fill at half opacity with `text-disabled` words.
- **Command palette and Settings buttons (W-154, W-156):** toolbar icon
  buttons like the others, at its right end, Settings last: `command`,
  tooltip "Command palette (Ctrl+Shift+P)", opens the palette; `settings`,
  tooltip "Settings (Ctrl+Shift+,)", opens Settings. Either can show its words (Settings,
  Toolbar). The dialog each opens takes the keyboard, so Escape closes it.
- **Administrator bar:** while acting as administrator, 36 px across the
  window on `error-surface` with an `error-border` outline: `shield-alert`,
  "Acting as administrator. Changes you make here can affect the whole
  system; be careful." in `error`, and an outlined 28 px "Stop acting as
  administrator". The mode belongs to the window (D-028): its title reads
  "<folder> (administrator) — Rustphin", other windows are not in it, and
  closing the window ends it. A pane showing a folder that is not root's own
  has a warning line over its list (`error-surface`, `triangle-alert`): "This
  folder does not belong to root: its owner can change what is in it while
  you work here as administrator." The toolbar's shield turns `error` and its tooltip says how
  to stop; without KDE's kio-admin it is unavailable and says so.
- **Re-read banner:** a shown folder that fails to re-read keeps its listing
  under a banner across the pane: `error-surface`, `error-border` outline, a
  `triangle-alert` icon in `error` and the sentence in `text`, wrapping.
- **Trash banner:** on `chrome` while a trash is shown: `trash`, the trash's
  name, an outlined "Trash actions" button (tooltip "Restore, empty or browse
  another trash; also available in Actions") opening a menu: Restore
  (unavailable without a selection), "Empty trash…", then other volumes'
  trashes after a separator; under them "Items stay here until the trash is
  emptied." in `body-sm` `text-muted`.
- **Places panel:** place rows with a 16 px icon in `text-muted` and the name
  in `body-md` `text`; hover fills `hover`; the current place (the folder the
  active pane shows; the Trash place while the home trash is shown) is filled `selection` with `on-selection` icon and name
  and a 2 px `primary-bright` edge on its left; the focused row has a 1 px
  `primary-bright` outline while the panel has the keyboard. Section headings (groups, "Devices", "Remote") in `label-sm`
  capitals `text-muted` after a 12 px chevron that points down when open and
  right when folded; a folded heading shows its count ("PHOTOS (3)", "DEVICES
  (2)", W-127). Places in a group are indented one step. Hidden places, when
  listed, are drawn at 60 % opacity. Network bookmarks use `server`.
  Separators (W-152) are rows 12 px tall holding a 1 px `border-strong`
  hairline across the panel's text column (indented with the places of a
  group); they take the hover fill, the focus outline and the drag like any
  row, and are read as "Separator". The user puts them between groups,
  between the places of a group, or between the places outside groups.
  Rows can be dragged (the name follows the pointer as a `drag-chip`) to
  reorder or regroup them, and a separator dropped on a row goes above it; folders dropped from the list or another
  application become places. With no places: "No places yet. Add the folder
  shown with Ctrl+Shift+D, or drag a folder here." If devices cannot be
  listed, a note under the panel says so.
- **Places menu (W-102):** a right-click, the Menu key or Shift+F10 in the
  panel opens a menu with the context menu's look. On a place: Open
  (`square-arrow-out-up-right`), Open in new tab (`square-plus`); "Move to
  group ›" (`folder-input`: each group, the place's own unavailable, then
  "New group…" and, for a grouped place, "Out of groups"), Move up, Move down,
  Add separator below (`separator-horizontal`; not for a place in the Remote
  section); Rename (F2), Hide or Show (H), Remove from places (Delete; not
  offered for KDE's built-in places). On a group heading: Collapse or Expand;
  Move up, Move down, Add separator above; Rename group (F2), Delete group
  (Delete; its places stay). On a separator: Move up, Move down; Remove
  separator (Delete, `circle-x`). On empty space: New group (Ctrl+Shift+G),
  Add separator (after the last group), Add to places (the folder shown);
  Show or Hide hidden places. On a drive (W-148): Open (Enter), Open in new
  tab; Mount (`hard-drive`) when it is not mounted, or Unmount
  (`arrow-up-from-line`) when it is; Safely remove (`power`) for a
  removable or power-off capable drive; Copy location (`link`) when it is
  mounted; then Properties (`info`, W-153). The volumes the system and the
  home folder run from offer neither Unmount nor Safely remove.
- **Drives (W-148, W-159):** each volume under Devices is drawn in one of
  three styles, chosen in Settings, Places, Drives, Style (the maintainer's
  three mockups of 2026-11-15); Bars is the default. Fixed drives come
  first; removable ones follow under "REMOVABLE" (`label-sm` `text-muted`,
  28 px above the first). In every style: no card fill (the place rows'
  `hover`, the current place's `selection` fill and 2 px edge, and the focus
  outline); the name in `body-md-strong`; a 16 px `hard-drive` or `usb`
  icon; usage in `progress` on `track`, `error` from 90 % used with "Almost
  full" in the words so the warning is never colour alone; a mounted
  removable drive has a 24 px Eject icon button (`arrow-up-from-line`,
  tooltip "Eject <name>": Safely remove where it can be powered off, else
  Unmount) at its row's right, and a drive that is not mounted a 24 px
  outlined Mount button (tooltip "Mount <name> and open it") instead; while
  it is being mounted or removed it says "Working…" and offers neither.
  Clicking a drive opens it (mounting it first); the drive menu holds every
  action. Its accessible description reads its figures in one sentence.
  - **Bars** (default): 56 px tall, 4 px apart, 8 px sides. The name, and on
    the right what is free ("261.2 GiB free", `data-sm`, `text-muted`,
    `error` when almost full); a full-width 4 px bar; then in `body-sm`
    `text-muted` the filesystem and size ("Btrfs · 927.4 GiB"), "Almost full ·
    ext4", or "Not mounted · NTFS · 237.9 GiB".
  - **Compact:** one 28 px line: icon, name, a 48 px bar and "NN%" in
    `data-sm`. The drive whose folder is shown opens its figures under it,
    16 px a line in `body-sm` and `data-sm`: Free, Used ("used / total"),
    Mount (mount point and filesystem), or Size when not mounted.
  - **Rings:** 48 px tall: a 32 px ring (3 px stroke, used share in
    `progress` or `error`, dashed and empty when not mounted) around a 12 px
    icon; the name, and under it "261.2 GiB free · Btrfs", "Almost full ·
    8.1 GiB free" or "Not mounted · ext4" in `body-sm` `text-muted`.
  Sizes are in the units Settings, Views chose, as the status line, and
  follow a change at once (W-158). The filesystem is UDisks2's `IdType`
  ("Encrypted" added when unlocked from an encrypted container).
- **Column headings:** `body-sm` `text-muted`, `hover` under the pointer; the
  sorted column in `text`, semibold, with a 12 px `arrow-up` or `arrow-down`,
  so direction is never colour alone. Clicking a heading sorts; clicking
  again reverses.
- **Entries (rows and cells):** icon or thumbnail, then the name in
  `body-lg`. Hover fills `hover`. Selected: `selection` fill
  (`selection-hover` under the pointer) with `on-selection` text and data and
  a 2 px `primary-bright` edge along the fill's left side. The focused entry
  has a 1 px `primary-bright` outline while the pane has the keyboard and,
  when not selected, the `current` fill. Hidden entries are drawn at the opacity Settings chooses
  (60 % by default) unless selected; cut entries at half opacity. Symbolic
  links: italic name and a 12 px `link` badge in the icon's corner. Data cells
  in `text-muted`; sizes right-aligned; folders show "—".
- **Thumbnails:** a picture's content replaces its icon, fitted in the icon's
  square without cropping (a 3:2 picture is 24 × 16 px in a row and
  96 × 64 px in an icon cell and in Properties). Until ready, or if it cannot
  be made, the type icon stays. Settings, Views turns them off or limits the
  picture size.
- **Context menu** (`JsonMenu.qml`, built from `menu::to_json`): 240 px,
  4 px inner padding. Items 32 px: a
  16 px Lucide icon (`text-muted`) or an application's own icon, the label in
  `body-md`, the key right-aligned in `data-sm` `text-muted`. The row under
  the pointer or the keyboard takes `hover`. Disabled items use
  `text-disabled` and are skipped. Separators are `border` hairlines. On a
  selection: Open, Open with ›; Cut, Copy, Paste, Copy location, Duplicate
  here; Rename, Move to trash (Restore in the trash; neither as
  administrator), Delete…; Extract › and Compress › (W-111; not in the trash
  or as administrator); Properties. On empty space: Paste; New folder, New
  file. A row that opens a second menu has an icon like any other (W-118):
  "Open with ›" `app-window`, "Extract ›" `archive-restore`, "Compress ›"
  `archive`, the places menu's "Move to group ›" `folder-input`; its 12 px
  `chevron-right` is on the right. The menu opens below the point, or above
  the entry when it does not fit below.
- **Compress and extract (W-111, D-032, mockup 37):** Extract › holds
  `package-open` Extract here, `folder-archive` Extract into "name/" (or
  "Extract each into its own folder" for several), `archive-x` Extract and
  trash archive, `folder-input` Extract to… (the desktop's folder dialog,
  starting in the folder shown); Compress › holds `file-archive` Compress to
  "name.zip" and "name.tar.gz", each the exact name it will make, and
  `archive` Compress to…. One archive alone is not offered for compressing.
  Compress to… is a 560 px dialog titled with `file-archive`: Name (a 32 px
  field holding the keyboard), Format (ZIP, TAR.GZ, TAR.XZ, TAR.ZST, 7Z) and
  Compression (Fast, Normal, Smallest) as segmented rows, the chosen segment
  in `selection` with a 2 px `primary-bright` edge along its bottom; a note saying what will be made and where, or why it cannot
  be; Cancel (Esc) and Compress (Enter), unavailable while the note is an
  error. A password question has a masked field focused at once and
  "Extract (Enter)".
- **Actions palette** (`Palette.qml`): up to 640 px wide, 8 px padding, its
  top edge under the toolbar, over the scrim. A 40 px search field on
  `content` with a 2 px `primary-bright` focus ring, a `search` icon and the
  placeholder "Search actions, places, tabs and recent folders…". Results
  52 px: a 20 px icon, the name in `body-md`, a trail in `data-sm` ("View ›
  Show hidden files"), the key right-aligned. The selected result is filled
  `selection` (`selection-hover` under the pointer) with `on-selection` text
  and a 2 px `primary-bright` edge on its left; others take `hover` under the
  pointer. Up, Down and Tab move, Enter runs, Escape closes. With no result:
  "Nothing matches. Try fewer letters." in `body-sm` `text-muted`.
- **Dialogs:** `surface-raised` with a 1 px `border-accent` outline and the
  2 px `glow` halo outside it, over the scrim. Properties, batch rename, Compress to… and Settings open with a
  48 px title row: an icon, the title in `headline-sm` and a close icon
  button; questions and the problems list start with their title. The body
  scrolls between title and actions; the action row is on `chrome`. Text fields (`input`) are 32 px on `content` with a
  `border-strong` outline, `primary-bright` while focused.
- **Drive Properties** (`DriveDialog.qml`, W-153): title "<name> —
  Properties" with the drive's `usb` or `hard-drive` icon and a Close icon
  button. Header: the 48 px icon, the name in `body-md-strong` and the
  card's traits ("exFAT · Removable", or "Not mounted") in `body-sm`
  `text-muted`. For a mounted volume, a ring 112 px across with a 12 px
  stroke: the free part in `track`, the used part from the top, clockwise,
  in `progress` (`error` from 90 % used, as the card's bar), "43%" in
  `headline-md` and "used" in `body-sm` `text-muted` in its middle; beside
  it a legend of Used, Free and Total, each a 12 px swatch (none for Total),
  the label in `body-sm` `text-muted` and the size with exact bytes in
  `data-md`. Without a ring, one sentence says why: "Not mounted. Mount it
  to see how full it is." or "Its free space could not be read." A
  hairline, then the facts as in Properties, only those the device service
  told: Mounted at, Size (not mounted), Device, Filesystem, UUID, Access
  ("Read and write" or "Read only"), Encrypted, Drive (maker and model),
  Kind ("Solid-state drive" or "Hard disk"), Connection ("USB", "SD card
  reader"…), Removable. Close (Esc), the primary button; Enter closes too.
- **Properties** (`PropertiesDialog.qml`, mockups 13–14): title "<name> —
  Properties" with a Close icon button. Header: a 96 × 64 thumbnail (or the
  48 px type icon), the name and the type. Facts as label (`text-muted`,
  128 px column) and value (`data-md`, wrapping between words, and inside
  a long path only where it must): Location, Size (with exact bytes),
  Modified, Accessed, Created when known, and a link's target; a link's
  icon is that of what it points to, as in the list. A folder's
  size reads "Calculating size…", then "Calculating… 1,204 items so far",
  until known. "Permissions" with the mode in words and octal, then OWNER,
  GROUP and OTHERS columns in `label-sm`, each with Read, Write and Execute
  checkboxes. Close (Esc) and Apply (Enter), which stays unavailable until a
  box changes. The facts scroll when the window is small, by the scrollbar
  or by Home, End, Page Up, Page Down and the arrows wherever the keyboard
  is in the dialog; title and buttons stay in view.
- **Batch rename** (`BatchDialog.qml`, mockups 15–16): 640 px, title row with
  `pencil`. Three modes (Replace, Pattern, Case) as a segmented row, the
  chosen one in `selection` with its `primary-bright` edge; the rule fields (Text to find and Replace with,
  Regular expression; Pattern with Start and Step, "# is the counter…"; Lower
  case, Upper case, Capitalise words); the preview as two `data-md` columns,
  Current name and New name; names that cannot be used are in `error`, with
  one problem note above the preview. "Nothing would change. Edit the rule to
  give the files new names." is a neutral note. Rename (Enter) stays
  unavailable while there is a problem or nothing would change.
- **Question dialogs** (`QuestionDialog.qml`, wording from
  `rustphin_view::questions`): an icon and the title, which states the
  question or the fact ("“IMG_4821.jpg” already exists", "Couldn't copy
  “IMG_4821.jpg”", "Run “setup.sh”?", "Move or copy?", "Delete for good?",
  "Move to the trash?", "Empty the trash?", "Close the window?", "Wrong
  password for “x.zip”"); the sentence in `text-muted`; the facts (labels in
  `body-sm` `text-muted`, values in `data-md`); the names it is about; the
  buttons, each with its key. A single letter answers (as in Dolphin) unless
  a field has the keyboard. The name conflict: Stop (Esc), Skip (S), Replace
  or Merge (R), Keep both (Enter), and "Do the same for all other conflicts
  (A)". A failed entry: Stop (Esc), Skip (S), Try again (Enter). "Run …?":
  Cancel is the Enter default (Dolphin focuses Cancel), Open (O) and Run (R);
  all three are outlined, since a filled button would claim Enter.
  Move or copy: Cancel (Esc), Copy here (C), Move here (Enter).
- **Problems dialog** (`ProblemsDialog.qml`): opened by Details on the
  notice line after an operation left entries out, or from the keyboard by
  the palette's "Files: Show what the last operation left out" (which says
  "The last operation left nothing out." when that is so): "<operation>: what was
  left out" in `headline-sm`; one row per entry with its path in `data-md`
  and the reason in `body-sm` `text-muted`, scrolling past 384 px; an
  outlined Close.
- **Checkboxes:** the Qt Quick Controls Basic indicator (`Check.qml`) in
  the design's colours, never the desktop colour scheme's: `content` fill,
  `border-strong` edge, `text` check, and a 2 px `primary-bright` outline on
  keyboard focus; the label is clickable. Selected text in any field takes
  `selection` and `on-selection` the same way.
- **Activity line** (`NoticeBar.qml`): on `chrome-raised`: the operation's
  icon in `text-muted` (`copy`, `move`, `trash`, `undo-2` for restoring,
  `pencil`, `package-open`, `file-archive`, `loader` otherwise), the sentence
  ("Copying 12 items to “Pictures”…", "(and N more)" while several run) in
  `body-md` over the figures ("4 of 12 · 51.2 MiB of 150.4 MiB · 48.6 MiB/s
  · about 2 s left") in `data-sm` `text-muted`, a 6 px progress bar
  (`progress` on `track`, at least 80 px and a fifth of the window; it sweeps
  while the size is unknown), an outlined Details button (tooltip "Show every
  running operation") that opens the transfer dialog, and a Stop icon button
  (`circle-stop`, tooltip "Stop this operation"). It appears after 300 ms
  (`operation_board::QUIET_PERIOD`) and shows the oldest operation.
  - The speed is a smoothed one (`transfer::Meter`): what moved in each
    whole second, averaged with a weight of 0.3 for the newest second, so one
    slow or fast second does not make the figure jump. It is shown from the
    first whole second. An operation measured in bytes shows "48.6 MiB/s";
    one that only counts entries (deleting, trashing) shows "120 items/s".
  - The time remaining is what is left divided by that speed. It is shown
    only once three whole seconds were measured, the totals are known and
    the speed is not zero: "about 2 s left" (exact below 10 s, then in steps
    of 5 s), "about 3 min left", "about 1 h 20 min left" (steps of 5 min).
- **Transfer dialog** (`TransferDialog.qml`, W-149): opened by Details on
  the activity line, or from the keyboard by the palette's "Files: Show
  running operations" (no key of its own); a dialog (480 px) titled "Running
  operations" over the dimmed window, with one card per operation shown
  (oldest first, after the same 300 ms), scrolling when the cards are taller
  than the window allows, and an outlined Close (Esc). Tab moves between the
  cards' Stop buttons and Close. A card is outlined in `border-strong` with
  12 px padding and holds, top to bottom:
  - the operation's icon in `text-muted`, its sentence in `body-md` and its
    own Stop icon button (`circle-stop`, tooltip "Stop this operation");
  - its 6 px progress bar across the card (it sweeps while the size is
    unknown);
  - what is done, in `data-sm` `text-muted`: "4 of 12 items · 51.2 MiB of
    150.4 MiB";
  - the entry being worked on now, its path elided in the middle, in
    `data-sm` `text-muted` ("Preparing…" before the first one);
  - the speed and the time remaining in `data-sm` `text` ("48.6 MiB/s ·
    about 2 s left"; "Measuring the speed…" during the first second), with
    the highest second on the right in `text-muted` ("Peak 52.0 MiB/s");
  - under them the speed graph, 48 px high: one point per second for the
    last 60 seconds, newest on the right, drawn as a 1 px `progress` line
    over a `track` baseline, scaled to its highest second. Drawn with Qt
    Quick Shapes, with no chart library. The speed is always also written
    in figures, so the graph is never the only place it is shown.

  A card leaves when its operation ends; the dialog closes itself when the
  last one ends, as KDE's copy window does, and the notice line then reports
  the result. A question (name conflict, failed entry, password) opens over
  the dialog, which is still there once it is answered. Opened while nothing
  runs, it says "No operation is running. A copy, move or other operation
  that takes a while is shown here."
- **Notice line:** 44 px after an operation ends, or while connecting to a
  remote folder: an icon that says the kind of message (`operations::Notice::
  icon`): an emerald `circle-check` for something done as asked, `info` in
  `text-muted` for information ("The clipboard holds no files.", "Stopped
  before anything changed."), `triangle-alert` for a problem; then the
  sentence ("Copied 12 items to “Pictures”."), an outlined "Undo (Ctrl+Z)" when it can be undone, Details
  when entries were left out, and a dismiss icon button (tooltip "Dismiss
  this message"). It leaves after 6 seconds (`NOTICE_TIME`), unless it
  reports a problem or is waiting on something: a problem uses
  `error-surface` with an `error-border` outline and `triangle-alert` in
  `error`, and stays until dismissed.
- **Blur status** (Main.qml): a 32 px `chrome` line over the activity and
  notice lines, shown only when the Look is Glass, `RUSTPHIN_OPAQUE` is not
  set and KWin offers no blur: an `info` icon in `text-muted`, "Background
  blur unavailable; using the opaque theme." in `body-md` `text-muted`, and
  a dismiss icon button (tooltip "Dismiss this message") that hides it in
  that window until blur changes again. Choosing Opaque in Settings removes
  it for good.
- **Title bar setting:** with Settings, Appearance, Title bar set to KDE's, a
  new window is an ordinary framed one: KWin draws the title bar and the
  resize edges, and Rustphin's own title bar, edge hairline and resize bands
  are left out. Windows already open keep what they have.
- **Filter bar:** a 16 px `funnel` icon, a 32 px field (at most 360 px)
  labelled "Filter by name", and a close icon button (tooltip "Close the
  filter (Esc)").
- **Empty, loading and error states** (in place of the list): a 32 px icon
  (`loader`, `folder-open`, `eye-off`, `search-x`, or `folder-x` in `error`),
  a `headline-md` line saying what is the case ("Loading…", "This folder is
  empty", "This folder has only hidden items", "Nothing matches “x”", "This
  folder is not readable"), a `text-muted` line saying why or what to do (at
  most 440 px wide), and outlined actions: Show hidden; Clear filter; or for a
  folder that cannot be opened, Back or Up (whichever applies, with its key)
  and Home folder. The status line names the reason.
- **Rename field:** 28 px inside the row, on `content` with a 2 px
  `primary-bright` outline, the name in its list size, the stem selected
  (`selection` with `on-selection`). Enter or leaving the field keeps the name; Escape
  cancels.
- **Tooltips** (`Tip.qml`): after 600 ms of hover, the label then the key in
  parentheses, in `body-md` `text` on `surface-raised` with a 1 px
  `border-strong` outline and 8 px padding; a long one wraps at 440 px. It
  opens 4 px below its control, centred under it, and is kept inside the
  window (W-156). A tooltip is a note, not a menu: it has no royal-blue outline or glow.
- **Dragging:** entries leave as a system drag of their URIs, shown by the
  first entry's type icon (32 px); a folder under the pointer takes `hover`,
  and a drop on the folder itself outlines the pane in 2 px `primary-bright`.
  A drop asks "Move or copy?" unless Ctrl (copy) or Shift (move) is held.
  Places are dragged as a `drag-chip` with the place's name.
- **Rubber band:** a 1 px `primary-bright` outline over `hover`.
- **Scrollbars** (`Scroller.qml`): a thumb 8 px thick, 2 px from the edge
  it runs along, in `text-muted` at half opacity (65 % under the pointer,
  80 % while dragged), with no track; shown whenever there is more than
  fits: vertical in the details and icons views and in every list that
  scrolls, horizontal in the compact view.
- **Interface icons:** the bundled Lucide SVGs (`crates/rustphin/icons`),
  drawn through `image://lucide/<name>/<colour>` in a token colour, 16 px by
  default, 12 px for chevrons and badges, 20 px in palette results, 32 px in
  states. Fixed names for the less obvious ones: `command` (the Command
  palette button), `settings` (Settings), `arrow-up-from-line` (Unmount; Lucide has no eject
  glyph), `power` (Safely remove), `loader` (connecting, loading), `circle-check` (done),
  `triangle-alert` (problems), `shield-alert` (administrator).

## Do's and Don'ts

- Do take every colour, size, spacing and radius from `Tokens`; add a token
  in `colors.rs` or `metrics.rs` (and here) before adding a literal.
- Do check every new surface or text pair in both glass (over white) and opaque
  modes, and add it to the contrast tests in `colors.rs`.
- Do pair colour with a shape, icon or text for every state: selection,
  current place, focus, sort direction, errors, disabled items, usage.
- Do give every icon-only control a tooltip with its label and key, and every
  command a keyboard or palette path.
- Do use the same label for a command in the menu, the palette and the
  tooltip: they all read `rustphin_view::commands`.
- Do use JetBrains Mono only for data that is aligned or read exactly.
- Don't blur, or rely on blurring, anything inside the window.
- Don't introduce a second accent, gradients, or soft drop shadows.
- Don't put text on bare `window-glass`, and don't go below 10 px (capital
  labels only; other text at least 11 px).
- Don't add a radius other than 2 px.
- Don't use royal blue for anything but the default button, focus rings,
  floating surfaces' outline and glow, selection edges and progress (D-038);
  never for text, icons or large fills.
- Don't follow the system colour scheme; do use the system icon theme for file
  types.

## Changes from the earlier design

W-148 (2026-11-14, the maintainer) turned device rows into drive cards:
"used / total" figures, the filesystem and its traits, "NN% used", the usage
bar in `progress` rather than `text-muted` (D-039), a semibold name, and a
drive menu in place of the eject button.

W-147 (2026-11-14, D-038) replaced the navy colours with charcoal: every
surface lost its blue tint; selection, the current place, the selected
palette result and the current Settings page went from a royal-blue fill to
the neutral `selection` grey with a 2 px `primary-bright` edge; `primary` is
now the default button's fill (`#2D4CC8` → `#4169E1`, white words) and the
new `destructive` the destructive one's; `primary-subtle` and `secondary`
(cyan) were retired, the progress bar and focus rings took the lightened
royal blue, usage bars and the activity icon went neutral; menus and dialogs
took the `border-accent` outline and `glow`.

W-079 (2026-11-08) replaced a neutral-obsidian design with a navy one:
new `primary-subtle`, `chrome-raised-*`, `text-disabled`, `error-border` and
`track` tokens; selection `#3457D5` → `#2D4CC8`; `text-muted` `#C3CAD9` →
`#AEB8CC`; menu icons and keys, palette icons and trails, Properties'
thumbnail header, the two-zone status line, the result line with Undo, and
question titles that state the question. Its roomier sizes (36 px rows,
15 px names, a 52 px toolbar) were then reduced by D-026 (the 12 px type
scale) and W-096 (the top bar redesign) to the values above. The three font
families, the 2 px radius, the 4-pixel grid, the opaque fallback, the wording
rules, the keyboard model and solid floating surfaces did not change.

## Implementation status

The interface is Qt 6 and QML through cxx-qt (D-034, W-114, switched
2026-11-14); `crates/rustphin/qml/` draws every screen above with the
`Tokens` singleton, and the colours and sizes in the front matter are the
values in `colors.rs` and `metrics.rs`. Built as described: the title bar
and resize bands, the toolbar with Settings' items and `Toolbar::fit`, the
location crumbs, the tab strip, split view with path bars, the places panel
with groups, separators (W-152), folded counts, Remote and Devices, its menu
and drag and drop, a drive's Properties with its ring (W-153), The toolbar ends with the Command palette and Settings icon buttons (W-154, W-156); the mockups in `docs/mockup/` still draw the worded Actions button they replaced.
the three views with the zoom table, selection, rubber band, rename in place
and thumbnails, the status line, the filter bar, the in-place states, the
re-read and trash banners, the administrator bar, the context, New, trash
and View menus with submenu icons (W-118), the palette, the activity and
notice lines, the transfer dialog (W-149), Properties, batch rename,
Compress to…, the questions, the problems dialog and Settings with all eight
pages, with drives drawn in three styles (W-159), and Settings restyled after the maintainer's refined mockup (W-155:
`docs/mockup/SettingsDialog_Refined`, a React project kept out of the
repository) with its search, page headings, section panels, switches,
segments, sliders, chips and look tiles; the mockup's 6–8 px corners, larger
type and drop shadow were not taken, by the maintainer's choice, so the
dialog keeps this document's 2 px corners, type scale and flat depth.

Where the build differs from this document today:

- **Settings in a narrow window:** below 720 px (`size::SETTINGS_NARROW`)
  the page list is a row of tabs over the page (icon and name, the chosen
  one with the accent edge under it); Left and Right move between them and
  the row scrolls to keep the chosen one in view.
- **Line heights:** the `lineHeight` tokens (`text::LINE`, `HEADING_LINE`,
  `TITLE_LINE`) set the lines of text that wraps (sentences in dialogs,
  states, notes and tooltips). A single line is centred in its row and
  keeps the font's own height.
- **Error banner:** the pane's re-read banner has no "Details…" or dismiss
  button; Details is on the notice line, since operation results are
  window-level, and the problems dialog has no "Copy list".
- **Title bar menu:** right-clicking Rustphin's own title bar opens KWin's
  window menu by asking KWin for its "Window Operations Menu" shortcut over
  D-Bus (the maintainer's choice, 2026-11-15). KWin places that menu at the
  window's top-left corner, not under the pointer; Qt has no public call
  that would place it there.

Carried over and implemented: the palette's **Recover original names after
interruption** for the current folder (browsing never starts recovery; an
invalid record, changed file or occupied original name produces an
actionable error); bounded scrolling bodies for every dialog and folder state
at the minimum window; menus that open above their entry when there is no
room below.

Lint with `npx @google/design.md lint docs/DESIGN.md`. Contrast is checked by
the tests in `colors.rs` (glass over white and opaque, and the hidden-entry
opacities Settings offers). The linter's remaining warnings are `border`,
`border-accent`, `glow` and `error-border`, used as outline
colours, which the format's component properties cannot express; the prose
says where.
