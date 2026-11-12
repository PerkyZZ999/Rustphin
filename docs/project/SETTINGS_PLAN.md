# Settings window: plan for the maintainer to approve (W-106)

Status: **approved as written 2026-11-12 (D-029, R-019) and built the same
day (W-106b–g).** Three differences from this plan were accepted by
The maintainer (D-030): no "system format" for dates, hidden files fade from 60 %
(WCAG AA), toolbar items move by buttons rather than dragging.

## What it is

One **Settings** window for Rustphin, like Dolphin's "Configure Dolphin…",
reached from:

- the Actions menu ("Settings…") and the palette ("Workspace: Settings"),
- **Ctrl+Shift+,** (Dolphin's key for its settings).

Layout: a dialog of about **760 × 560 px** over the dimmed window, drawn
with the same surfaces as Properties (D-017: menus and dialogs are solid).
A **list of pages on the left** (icon and name, 200 px), the chosen page on
the right, scrolling on its own. At narrow widths the list becomes a row of
tabs above the page.

**Changes apply at once**, as in KDE's newer settings pages; no OK/Apply.
Each page has **"Reset to defaults"** at the bottom, and the window has a
Close button (Esc). A change that cannot apply at once (for example the
title bar style) says "Applies to new windows".

## Pages

### 1. General

| Setting | Choices | Default | Today |
|---|---|---|---|
| When Rustphin starts | Restore the last tabs · Open the home folder · Open a chosen folder | Restore | always restores |
| Folders opened by other applications | In a new tab of the open window · In a new window | New tab | new tab |
| Default file manager | Shows whether Rustphin opens folders for the desktop, with "Make Rustphin the default" | n/a | desktop entry only |
| Terminal | Shows KDE's terminal ("Ghostty"), with "Use another…" (command) | KDE's | KDE's (W-108) |
| Open files and folders with | A single click · A double click | KDE's setting | double click |

### 2. Appearance

| Setting | Choices | Default | Today |
|---|---|---|---|
| Window look | Glass (blur when the desktop offers it) · Opaque | Glass | `RUSTPHIN_OPAQUE` variable |
| Title bar | Rustphin's own · KDE's (the desktop draws it) | Rustphin's | Rustphin's |
| Hidden files | Faded (the opacity, 30–100 %) | 60 % | 60 % fixed |
| Text size | Smaller · Normal · Larger (the whole interface, on the type scale) | Normal | fixed |

The design stays dark only (BRIEF.md); no light theme or accent colour
choice in this plan.

### 3. Views

| Setting | Choices | Default | Today |
|---|---|---|---|
| Default view | Icons · Compact · Details, and its size | Details, 24 px | remembered globally |
| Remember the view for each folder | On · Off (one view everywhere) | Off | one view everywhere |
| Sort by, direction, folders first | as the view menu | Name, ascending, folders first | remembered globally |
| Show hidden files | On · Off | Off | toggle |
| Details columns | Size · Modified · Type (on/off each); later Permissions, Owner | all on | all on |
| Dates | "2026-11-12 14:32" · "Today 14:32" · system format | ISO | ISO |
| Sizes | Binary (KiB, MiB) · Decimal (kB, MB) | Binary | binary |
| Previews | Show thumbnails · up to N MB per file | On, 50 MB | on |

### 4. Toolbar (customisation, W-106 as the maintainer asked)

As Dolphin's "Configure Toolbars…": two lists side by side, **Available
actions** and **On the toolbar**, with search, add, remove, move up/down,
drag between and within the lists, and **separators**. Each toolbar item can
show its icon only, or its icon and words. "Reset to defaults" restores
today's toolbar. A small preview of the toolbar sits above the lists. The
responsive rules (what leaves at 1000, 760 and 560 px) apply to whatever the
user chose, from the right end inwards.

### 5. Places

| Setting | Choices | Default |
|---|---|---|
| Sections | Show Devices · Show Remote (each on/off) | both on |
| Hidden places | Show them (faded) · Keep them hidden | hidden |
| Groups | The list of groups: rename, reorder, delete (as in the panel) | n/a |
| Panel width | 200–360 px, also by dragging the panel's edge | 240 px |

### 6. Confirmations

| Ask before | Default | Today |
|---|---|---|
| Moving to the trash | Off | off |
| Deleting for good | On (cannot be turned off) | on |
| Emptying the trash | On | on |
| Running an executable file | Ask · Always open as text · Always run | Ask | ask |
| Closing a window with several tabs | Off | off |

### 7. Shortcuts (read-only first)

The keyboard table of KEYBOARD.md, searchable, grouped as in the palette.
Changing keys is later work.

### 8. About

The logo, name and version; "File manager for KDE Plasma"; the licence
(GPL-3.0-or-later, with the full text one click away); credits (Dolphin and
KDE for the behaviour it follows; GPUI and GPUI-Kit; Lucide icons; Inter,
Space Grotesk and JetBrains Mono with their licences); the privacy line ("No
telemetry. Rustphin never uses the network."); where its files are
(settings, place groups, session) with **Open settings folder**. No links
are fetched.

## Storage and code

- Everything in `~/.config/rustphin/settings.toml` (today's file, version 3),
  in sections `[general]`, `[appearance]`, `[view]`, `[toolbar]`,
  `[places]`, `[confirmations]`; written atomically; read leniently (unknown
  or damaged values fall back to defaults, a newer file is never
  overwritten), as today.
- The settings model is pure Rust in `rustphin-ui` with unit tests (parse,
  defaults, migration from version 2); one shared settings entity that every
  window observes, so changes show at once everywhere.
- The window is a new view, `settings_dialog.rs`, with one module per page;
  each control is labelled for screen readers and reachable by keyboard
  (Tab, arrows in the page list, Space/Enter).

## Order of work (after approval)

| Step | Content |
|---|---|
| W-106a | Requirement, decisions and HTML mockups of the window and each page, for the maintainer to approve |
| W-106b | Settings model and storage, the window with its page list, About, Ctrl+Shift+, |
| W-106c | General and Views (moving today's remembered settings in) |
| W-106d | Appearance (window look, title bar, faded hidden files, text size) |
| W-106e | Confirmations and Places |
| W-106f | Toolbar customisation |
| W-106g | Shortcuts (read-only) |

Each step: design first, tests, on-screen check in an isolated session,
`rust-tc doctor`, one commit, reinstall.

## Decisions for the maintainer

1. **Dialog over the window** (recommended, like Properties) or a separate
   window?
2. **Apply at once** with "Reset to defaults" (recommended) or OK / Apply /
   Cancel?
3. **The pages:** keep, drop, rename or add any? ("Customizations" is
   proposed as **Toolbar**; it can hold more later, such as the context
   menu's entries.)
4. **Remember the view for each folder** as an option (Dolphin's
   default), or keep one view everywhere only?
5. Anything in the tables you do not want, or something missing?
