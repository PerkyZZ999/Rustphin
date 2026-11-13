# Background mode (W-110): plan for approval

The maintainer asked on 2026-11-13 for Rustphin to stay running after its last window closes and to be able to start at login, so that windows open at once. This plan says what that means, what was measured before writing it, how it would be built, and what the maintainer needs to decide.

## Why

A Rustphin process spends most of its start in the UI toolkit's font scan, which Rustphin cannot change (D-010). With the fonts cached the first frame comes after about 0.6 s. When other programs push the fonts out of memory, it takes about 3.5 s even with the parallel pre-read (W-109b), and 8.5–9 s before it. A process that is already running skips all of that.

## What was found

- **Dolphin does not do this.** `dolphin --daemon` (Dolphin's `main.cpp`) opens no window. It only answers `org.freedesktop.FileManager1` requests, and for each one it starts a new `dolphin` process (`Dolphin::openNewWindow`, through `KIO::ApplicationLauncherJob`). A normal Dolphin process quits when its last window closes. Dolphin starts fast because Qt reads fontconfig's cache; Rustphin's toolkit cannot. The nearest model is Nautilus: its D-Bus service starts `nautilus --gapplication-service`, one resident process that opens every window itself. This plan follows Nautilus. Where Rustphin can, it follows Dolphin's rules for where a folder opens.
- **The toolkit allows it.** GPUI quits when its last window closes unless the app sets `QuitMode::Explicit`. In a throwaway test (isolated session, release build, not committed), Rustphin with that mode kept running after Ctrl+Q closed its only window. Three seconds later it opened a new one: **27 ms** from the request to a frame describing the folder.
- **Memory while idle:** 139.5 MB resident with no window, the same as with one window open (139–146 MB). About 40–45 MB of that is Rustphin's own memory. About 100 MB is mapped files (libraries, fonts, the type database), which the kernel can drop and re-read when it needs memory. Nothing was released on close in that test, so this is the starting point, not the target.
- **Today:** every `rustphin` launch is a whole new process that pays the full start. The `FileManager1` service belongs to the first process, and its requests are handled by an open window.

## How it would work

1. **One Rustphin per session.** The running Rustphin owns a session-bus name of its own (proposed: `org.rustphin.Rustphin`). `rustphin [folders]` first asks the bus whether that name has an owner. If it does, the launch forwards its folders and its Wayland activation token, then exits in milliseconds without starting the UI toolkit. Otherwise it becomes the running Rustphin. As in Dolphin, a launch with no folders opens a new window. Folders named on the command line, or sent by other applications, follow the General page's "folders from other applications" choice (new tab or new window).
2. **The last window closes, the process stays.** When the setting is on, closing the last window leaves Rustphin running with no window, no tray icon and nothing on the taskbar. Before that it saves the session, as today, and lets go of what only windows need: listings, watchers, thumbnails in memory, and the places watcher. Requests from other applications then open a new window.
3. **Quitting for good:** Ctrl+Q, "Workspace: Quit" in the palette, `rustphin --quit` (for scripts, and before reinstalling a new build), or logging out. The window's close button only closes that window.
4. **Start at login:** `rustphin --background` starts with no window. When the Settings switch is on, Rustphin writes an autostart entry (`~/.config/autostart/rustphin.desktop`); when it is off, Rustphin removes it. Nothing else on the system changes.
5. **Settings, General page:** "Keep Rustphin running after its last window closes", and under it "Also start it when you log in" (only available while the first is on). The page says how much memory the running Rustphin uses.
6. **Windows get the focus.** KWin only lets a window take the focus when it carries the activation token of the click that asked for it. The launch passes its token to the running Rustphin. GPUI currently logs "activate is not implemented on Linux", so this needs checking early (W-110d). Without it, new windows might open behind others.
7. **When it goes wrong:** if the running Rustphin does not answer within 2 s, the launch starts a process of its own and logs why, so a hung Rustphin never stops a window from opening. A crash leaves nothing behind; the next launch becomes the running Rustphin.

## Requirement (draft, R-020, after approval)

- With the setting on, closing the last window leaves Rustphin running, and the next window (from the launcher, a folder sent by another application, or the command line) shows its folder within 200 ms of the request on a warm machine, whatever the font cache holds.
- Only one Rustphin process runs per session. A second launch hands its request to the first and exits without starting the UI toolkit.
- With no window open, Rustphin's own memory is at most 60 MB after the last window closes and it uses no CPU while nothing happens (no timers).
- Ctrl+Q, the palette's Quit and `rustphin --quit` end the process. Closing windows does not.
- The login switch adds or removes exactly one autostart entry and changes nothing else.
- New windows get the focus when the request carries an activation token.
- With the setting off, Rustphin behaves exactly as today.

## Order of work

- **W-110a — one Rustphin per session:** the bus name, forwarding launches, `--quit`, and the 2 s fallback. Tested against a private bus, as the `FileManager1` service is.
- **W-110b — staying without windows:** `QuitMode::Explicit` when the setting is on; releasing per-window resources; opening a window for requests when none is open.
- **W-110c — `--background` and the Settings switches:** the autostart entry and the General page.
- **W-110d — window focus through the activation token:** a check of what the pinned GPUI supports, then a fix inside Rustphin if possible. If it needs the toolkit, raise it rather than fork it (D-010).
- **W-110e — idle cost:** measure memory and CPU wake-ups with no window, then trim until the requirement is met.
- **W-110f — UI scenarios and docs:** UI_SCENARIOS (close and reopen, a second launch, quit, the login entry); the reinstall step becomes `rustphin --quit`, then replace the binary.

## For the maintainer to decide

1. **Default for "keep running":** on or off for new installs? Recommended: on. It is the point of the feature, and the cost is roughly 40–60 MB.
2. **Default for "start at login":** recommended off, so that installing Rustphin never adds an autostart entry by itself. You would turn it on once.
3. **Ctrl+Q:** today it closes the window. Recommended: in background mode it quits Rustphin entirely (every window, and the process), as in Nautilus and most applications, after the usual question when several tabs are open. The close button keeps closing one window.
4. **No tray icon:** recommended, as in Dolphin and Nautilus. Quitting is Ctrl+Q, the palette or logging out.
5. **The bus name:** `org.rustphin.Rustphin` is a placeholder; any reverse-domain name works.
