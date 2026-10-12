# Brief

Status: **agreed 2026-10-12**. Items marked (A) remain assumptions.

## Problem
Dolphin is a capable file manager, but its UI/UX has accumulated friction and its architecture keeps much work (sorting, role updates) bound to the main thread and to KDE Frameworks. The user wants a file manager with Dolphin's strengths, a modern UI/UX, some new features, and an architecture designed around Rust's strengths (parallelism, memory safety, cheap concurrency).

## Users
- Primary: the project author and KDE Plasma (Wayland) users who use Dolphin today.
- Secondary: Linux users on other desktops; supported where it costs little, never at the expense of Plasma integration. (A)

## Outcome
A Rust file manager, re-architected rather than ported line-by-line from Dolphin, that replaces Dolphin as the daily driver on Plasma for local file management, without losing interoperability with the rest of the desktop (drag-and-drop, clipboard, places, thumbnails, default-file-manager integration).

## Positioning
Plasma daily driver. Plasma integration (places, devices, service menus, icon theme) takes priority over cross-desktop generality.

## Defining features (beyond Dolphin)
1. **Place groups** — user-defined collapsible categories in the Places panel, each holding several folders ("sub-places").
2. **Command palette and fuzzy jump** — keyboard-driven palette for actions and fuzzy navigation to folders and recent locations.
3. **Advanced batch rename** — patterns, regex, counters, live preview.

## Design direction
Modern (2026), classy, professional. "Obsidian frosted glass": dark, translucent, blurred surfaces with royal-blue accents. Progressive disclosure: a calm default surface, with advanced controls revealed on demand (hover, selection, palette, expandable panels) rather than always visible.

Consequence: Rustphin carries its own theme rather than following the system colour scheme. It still uses the system icon theme for file-type icons. Dark only for now.

## MVP scope (core local workflow)
Browse; icons, compact and details views; tabs; split view; places (with groups); devices; file operations with undo; trash; thumbnails; drag-and-drop; clipboard. Command palette and batch rename are defining features and are in the MVP. Dark theme only. Wayland only.

Later: search, embedded terminal panel, service menus, version-control overlays, information panel, Miller columns, quick preview.

## Success signal
The author uses Rustphin instead of Dolphin as the default file manager for two consecutive weeks for local file management, and a directory of 500,000 entries opens, sorts and scrolls without visible UI stalls.

## Non-goals
- A line-by-line port of Dolphin's C++ or of KDE Frameworks (KIO, Solid, Baloo, KParts).
- Native reimplementation of network protocols in the MVP (kio-fuse mounts instead). (A)
- Non-Linux platforms and X11.
- Imitating Breeze widgets or following the system colour scheme.
- Being "faster because Rust": performance claims must come from measured design wins.

## Constraints (known)
- Language: Rust, minimum supported version 1.95 (installed toolchain: 1.98.1).
- Licence: GPL-3.0-or-later, because the work derives from Dolphin (GPL-2.0-or-later) and must be compatible with Apache-2.0 dependencies.
- UI: GPUI + GPUI-Kit (`gpui-kit` crate), conditional on a toolkit spike; Qt6/QML via cxx-qt is the fallback.
- Structure: UI-agnostic core so the frontend stays replaceable.

## Open questions
- Place groups vs Dolphin's `user-places.xbel`: the shared file has no notion of user-defined groups, so group membership needs Rustphin-owned storage layered over the shared places. (Discovery)
- Can GPUI on Wayland/X11 drag files out to other apps, exchange file-URI clipboard data, and get compositor blur behind a translucent window on KWin? (Discovery spike)
