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

- [x] **Glass.** Open Rustphin over your wallpaper and over a white window (a web page). Is the frosted background pleasant? Is all text easy to read over the white window?
  Rating: OK · Notes:Perfect.
- [x] **Headings.** "Places", dialog titles and "This folder is empty" should look semibold, like the mockups.
  Rating:OK · Notes:
- [x] **Overall impression** compared with the mockups in `docs/mockup/`. Anything that looks off (spacing, colours, icons)?
  Rating:OK · Notes:

## 2. Daily browsing

- [x] Open your usual folders from Places, the breadcrumb, Back/Forward/Up (Alt+Left, Alt+Right, Alt+Up, Backspace).
  Rating:OK · Notes:
- [x] Switch views with the three modes (details, icons, compact) and zoom (Ctrl+= / Ctrl+-). Are the choices remembered after restarting?
  Rating:OK · Notes:
- [x] **Compact view with your mouse wheel:** does one wheel step scroll a sensible distance sideways?
  Rating:OK· Notes:
- [x] Sorting by Name, Size, Modified (click the headings). Folders stay on top?
  Rating:OK · Notes:
- [x] A very big folder (thousands of files): does it open quickly and scroll smoothly?
  Rating:Odd · Notes: Seems fine, but I don't see any scrollbar on the right where it should be showing ... even when I scroll lol.
- [x] Thumbnails of your photos (JPEG, PNG, WebP, GIF). Do they appear, and quickly enough?
  Rating:OK · Notes:
- [x] Hidden files (Ctrl+H), the filter bar (Ctrl+I or /), type-ahead (type the start of a name).
  Rating:Odd · Notes: Hidden files/folders are shown when enabled but there isn't any visual difference with a regular file/folder, we should reduce opacity of the hidden files/folders so that user can know that they are hidden files/folders... Like Dolphin does...
- [x] Tabs (Ctrl+T, Ctrl+W, Ctrl+Shift+T) and split view (F3). After closing and reopening Rustphin, are your tabs back?
  Rating:OK · Notes:
- [x] A folder you can't open (for example `/root`): "Could not open this folder" with a Back button.
  Rating:OK · Notes:

## 3. File operations (please try on copies, not irreplaceable files)

- [x] Copy, cut and paste (Ctrl+C/X/V) within Rustphin, and between Rustphin and Dolphin, both ways.
  Rating: · Notes:
- [x] Drag and drop between folders, tabs and the split view; drop onto Places.
  Rating: · Notes:
- [x] **Drag a file into an application that isn't a KDE one** (Firefox, Chrome, a GTK app). Does it accept the file?
  Rating: · Notes:
- [x] Rename (F2), new folder (F10), duplicate (Ctrl+D), then undo (Ctrl+Z).
  Rating: · Notes:
- [x] Move to trash (Delete), restore from Trash, empty Trash. Delete for good (Shift+Delete) asks first?
  Rating: · Notes:
- [x] A name conflict (paste where the same name exists): are the choices clear?
  Rating: · Notes:
- [ ] A large copy (a few GB): progress, speed, stopping it halfway. Is anything left behind after stopping?
  Rating: · Notes:
- [ ] Batch rename (select several, F2): try replacing text and numbering.
  Rating: · Notes:
- [x] Properties (Alt+Enter) on a file and a folder; change permissions on a test file.
  Rating:OK · Notes:

## 4. Devices and network (need your hardware)

- [ ] **USB drive:** plug it in; mount it from Devices; open it; eject it.
  Rating: · Notes: Didnt try for now will do later.
- [ ] **Busy drive:** with a file on the USB drive open in another app, try to eject. Does Rustphin say why it can't?
  Rating: · Notes: Didnt try for now will do later.
- [ ] A network folder you use (sftp://, smb://), typed in the location bar (Ctrl+L) or from a bookmark.
  Rating: · Notes: I do not see my Remote folders under/above the Devices in the left sidebar , there should be like in Dolphin ...

## 5. Keyboard and palette

- [x] Actions palette (Ctrl+Shift+P): find and run a few commands; jump to a recent folder.
  Rating:OK · Notes:
- [ ] Any shortcut you use in Dolphin that doesn't work, or does something else here? (See `docs/project/KEYBOARD.md`.)
  Rating: · Notes:

## 6. Opening things

- [x] Double-click files of different types: do they open in the right application? "Open with" lists sensible apps?
  Rating:Odd · Notes: In the context-menu right click, the Open With option shows all the apps but their icons are not the right ones they are folder icons ...
- [x] Running a script or program asks first?
  Rating:Odd · Notes: It does but the popup buttons are "Run (R)" and "Keep (ENTER)" which I dont understand what "Keep" would mean here ... I'd replace that with Do Not Run or Cancel maybe ...
- [ ] "Show in folder" from another app (for example Firefox's downloads) opens Rustphin?
  Rating: · Notes: Wasn't able to test since I gotta make Rustphin the default file manager but I didnt do that.. Ill do it later on.

---

## Other findings

| When | What you did | What happened | What you expected |
| ---- | ------------ | ------------- | ----------------- |
|      |              |               |                   |

## Overall

- Would you use it instead of Dolphin for a day? Why or why not: Not right now. But really soon yes, it needs some refinements on the UI/UX side of things.
- The three things to fix first:
  1. Custom Window Titlebar that fits the design and UI of the app.
     
     2. I would make a switch and put the tab bar where the topbar (action, path bar, etc..) is (see screenshot attached to my message.)
        
        3. Some of the actions that are in the Command Palette / Actions button should be directly in the topbar where back, up ,forward buttons are and on the right side of the path bar too ... (we gotta discuss which actions we would put there.)
        
        4. I would use way more Icons for buttons everywhere.
