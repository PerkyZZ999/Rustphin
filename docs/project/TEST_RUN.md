# Test run: findings

Rustphin build `b0285e7`, installed 2026-11-09 at `~/.local/bin/rustphin`.
Start it from the desktop icon or the application menu (Rustphin, under System).
Dolphin stays your default for folders; nothing here changes that.

For each item, tick the box once tried and write what you saw under **Notes**.
"Fine" is a useful answer too. Anything not listed goes under "Other findings"
at the end. If something crashes or freezes, note the time: the log is in
`journalctl --user --since "<time>"`.

How to rate: **OK** works as expected · **Odd** works but feels wrong · **Broken** fails.

---

## 1. Look and feel (things only your desktop can show)

- [ ] **Glass.** Open Rustphin over your wallpaper and over a white window (a web page). Is the frosted background pleasant? Is all text easy to read over the white window?
  Rating: · Notes:
- [ ] **Headings.** "Places", dialog titles and "This folder is empty" should look semibold, like the mockups.
  Rating: · Notes:
- [ ] **Overall impression** compared with the mockups in `docs/mockup/`. Anything that looks off (spacing, colours, icons)?
  Rating: · Notes:

## 2. Daily browsing

- [ ] Open your usual folders from Places, the breadcrumb, Back/Forward/Up (Alt+Left, Alt+Right, Alt+Up, Backspace).
  Rating: · Notes:
- [ ] Switch views with the three modes (details, icons, compact) and zoom (Ctrl+= / Ctrl+-). Are the choices remembered after restarting?
  Rating: · Notes:
- [ ] **Compact view with your mouse wheel:** does one wheel step scroll a sensible distance sideways?
  Rating: · Notes:
- [ ] Sorting by Name, Size, Modified (click the headings). Folders stay on top?
  Rating: · Notes:
- [ ] A very big folder (thousands of files): does it open quickly and scroll smoothly?
  Rating: · Notes:
- [ ] Thumbnails of your photos (JPEG, PNG, WebP, GIF). Do they appear, and quickly enough?
  Rating: · Notes:
- [ ] Hidden files (Ctrl+H), the filter bar (Ctrl+I or /), type-ahead (type the start of a name).
  Rating: · Notes:
- [ ] Tabs (Ctrl+T, Ctrl+W, Ctrl+Shift+T) and split view (F3). After closing and reopening Rustphin, are your tabs back?
  Rating: · Notes:
- [ ] A folder you can't open (for example `/root`): "Could not open this folder" with a Back button.
  Rating: · Notes:

## 3. File operations (please try on copies, not irreplaceable files)

- [ ] Copy, cut and paste (Ctrl+C/X/V) within Rustphin, and between Rustphin and Dolphin, both ways.
  Rating: · Notes:
- [ ] Drag and drop between folders, tabs and the split view; drop onto Places.
  Rating: · Notes:
- [ ] **Drag a file into an application that isn't a KDE one** (Firefox, Chrome, a GTK app). Does it accept the file?
  Rating: · Notes:
- [ ] Rename (F2), new folder (F10), duplicate (Ctrl+D), then undo (Ctrl+Z).
  Rating: · Notes:
- [ ] Move to trash (Delete), restore from Trash, empty Trash. Delete for good (Shift+Delete) asks first?
  Rating: · Notes:
- [ ] A name conflict (paste where the same name exists): are the choices clear?
  Rating: · Notes:
- [ ] A large copy (a few GB): progress, speed, stopping it halfway. Is anything left behind after stopping?
  Rating: · Notes:
- [ ] Batch rename (select several, F2): try replacing text and numbering.
  Rating: · Notes:
- [ ] Properties (Alt+Enter) on a file and a folder; change permissions on a test file.
  Rating: · Notes:

## 4. Devices and network (need your hardware)

- [ ] **USB drive:** plug it in; mount it from Devices; open it; eject it.
  Rating: · Notes:
- [ ] **Busy drive:** with a file on the USB drive open in another app, try to eject. Does Rustphin say why it can't?
  Rating: · Notes:
- [ ] A network folder you use (sftp://, smb://), typed in the location bar (Ctrl+L) or from a bookmark.
  Rating: · Notes:

## 5. Keyboard and palette

- [ ] Actions palette (Ctrl+Shift+P): find and run a few commands; jump to a recent folder.
  Rating: · Notes:
- [ ] Any shortcut you use in Dolphin that doesn't work, or does something else here? (See `docs/project/KEYBOARD.md`.)
  Rating: · Notes:

## 6. Opening things

- [ ] Double-click files of different types: do they open in the right application? "Open with" lists sensible apps?
  Rating: · Notes:
- [ ] Running a script or program asks first?
  Rating: · Notes:
- [ ] "Show in folder" from another app (for example Firefox's downloads) opens Rustphin?
  Rating: · Notes:

---

## Other findings

| When | What you did | What happened | What you expected |
|---|---|---|---|
| | | | |

## Overall

- Would you use it instead of Dolphin for a day? Why or why not:
- The three things to fix first:
  1.
  2.
  3.
