# Privacy and local data

This describes what the Qt interface (since W-114i, 2026-11-14) reads,
writes and connects to, checked against the source on 2026-11-14. The
privacy audit W-077a–h (2026-11-06 to 2026-11-08) was made on the earlier
GPUI interface; its dated evidence is in VALIDATION.md. R-108 is accepted
under decision D-018. Neither this inventory nor the earlier replay
guarantees secure erasure or the absence of network activity on every
execution path.

## Connections and external applications

Rustphin has no application-owned HTTP client, remote service, updater or
telemetry collector, and its source opens no TCP or UDP socket. `deny.toml`
enforces it: `async-net`, `http`, `reqwest`, `hyper`, `ureq`, `isahc`,
`curl`, `surf`, `attohttpc`, `rustls`, `native-tls`, `openssl`, `sentry` and
`opentelemetry` are banned outright, so adding any of them fails
`rust-tc doctor` and needs a new decision. The GPUI toolkit's retained
network crates, which D-018 accepted as an exception, left with GPUI at
W-114i; none of the banned crates is in `Cargo.lock`. Two indirect paths
remain and are reviewed, not banned: `async-io` (through zbus) can open a
TCP transport if the environment's D-Bus address names one, and Qt's QML
module links Qt Network, although Rustphin's QML is loaded only from its own
compiled resources. These are source and dependency findings, not proof
about every execution path.

Local IPC is required: Wayland for display and the file clipboard; the
session D-Bus for the single-instance service (`org.rustphin.Rustphin`),
FileManager1, kio-fuse, KWin's global shortcuts (`org.kde.kglobalaccel`, only to open the window menu when the title bar is right-clicked) and accessibility; the system D-Bus for UDisks2 and,
in administrator mode, KDE's kio-admin helper. Bus addresses come from the
desktop environment; Rustphin does not enforce Unix-only D-Bus addresses.
Opening a file hands its path to the chosen desktop application; "Open
Terminal Here" starts the configured terminal in the folder; explicitly
approved programs and shortcuts can run; archives libarchive cannot read are
handed by path to an installed 7-Zip or unrar. Those programs' own
connections and history, and filesystem access to an already mounted remote
filesystem, are outside Rustphin's application-owned connection contract.

Rustphin asks kio-fuse to mount an address typed in the location bar or a
network bookmark (`org.kde.KIOFuse.VFS.mountUrl` on the session bus,
W-080). kio-fuse and its KIO worker make the connection and show any
password prompt. A password typed inside an address is passed to kio-fuse
only; Rustphin never shows, logs or saves it. The local mount folder and its
remote address are remembered in memory for the run. Saved tabs, recent
folders and each folder's view store a remote folder by its address without
password.

The W-077d traces (five application lifetimes, 60 socket calls, all
AF_UNIX) were made on the GPUI build. No trace has been repeated on the Qt
build: that its sockets are local only is not verified.

## Data inventory and clearing

Absolute XDG overrides take precedence; a relative or empty value is ignored.
Defaults below are relative to the home folder (`~`). Rustphin writes its
own files through `config::write_atomic`: a private (0600) temporary file
`<name>.tmp-<pid>-<n>` beside the target, synced, then renamed over it;
parent folders it creates follow the process umask. None of these files is
encrypted.

A file of Rustphin's own that cannot be read, is damaged or was written by a
newer Rustphin is kept unchanged for the whole run: saving that file is off
until the next start, and Rustphin says which file and why (`settings.toml`,
`place-groups.toml`, `views.toml` since W-140/W-143; `window.toml` since W-160; `session.toml` since
W-077g). A missing file is created on the first save.

### Rustphin's own files

| Location | Stored information | Clearing or retention |
|---|---|---|
| `$XDG_CONFIG_HOME/rustphin/settings.toml` (default `~/.config/…`) | The settings (D-029): start behaviour and start folder (as a saved location), which folders open in tabs, a terminal command, click behaviour, keep running and start at login; appearance (look, title bar, text size, hidden-file opacity); confirmations; places panel choices and width; toolbar items; default view (mode, zoom, sorting, hidden files, folders first, per-folder views on or off, details columns, date and size styles, thumbnails and their size limit) | Settings' pages that hold settings each have "Reset to defaults". To reset everything, quit Rustphin (`rustphin --quit`) and remove the file. No configurable key-binding file exists. |
| `$XDG_CONFIG_HOME/rustphin/place-groups.toml` | Group titles, order, folded state (also of the Devices and Remote sections), the KDE bookmark identifiers each group holds (D-008), and the places panel's separators with the bookmark identifier each one follows (W-152) | Remove groups through the places panel; to reset all groups, quit Rustphin and remove the file. The shared bookmarks remain. |
| `$XDG_STATE_HOME/rustphin/session.toml` (default `~/.local/state/…`) | The start window's open tabs: each pane's folder as a byte-safe local `file://` URI or a remote address without password, the active tab and pane (format 2; UTF-8 format 1 upgrades on save) | Quit Rustphin, then remove the file to forget saved tabs. Clearing recent folders does not clear it. Start with folders on the command line to choose other tabs. |
| `$XDG_STATE_HOME/rustphin/recent-locations.toml` | At most 10,000 distinct folders visited successfully, newest first, as byte-safe local URIs or remote addresses without password | Ctrl+Shift+P → **History: Clear recent folders**; wait for **Recent folders cleared.** New visits are recorded again. A read or save error is shown in the palette; do not treat it as a successful clear. |
| `$XDG_STATE_HOME/rustphin/window.toml` | The size new windows open at: the width and height the user last gave a window, and whether it was maximised (W-160). No position, folder or screen name | Quit Rustphin and remove the file; the next window opens at 1000 × 700 px. |
| `$XDG_STATE_HOME/rustphin/views.toml` | Only while Settings › Views › Default view › "Each folder" is on (off by default): up to 2,000 folders, each as a saved location with its mode, zoom, sorting and filter options; the oldest leave first | Turning the setting off stops new entries but keeps the file. Quit Rustphin and remove the file to forget them. Clearing recent folders does not clear it. |
| `$XDG_CONFIG_HOME/autostart/rustphin.desktop` | Only when Settings › General › "Also start it when you log in" is on: an autostart entry naming the Rustphin program's path, run with `--background` (W-110c) | Turn the setting off: Rustphin removes the entry, and never one it did not write. |
| `.rustphin-rename-journal` in a batch rename's folder | Original, staged and final names plus the file identity needed for recovery (W-071) | Removed after a successful rename or recovery. After an interruption it is kept, with the staged `.rustphin-part-rename-<n>` entries, and recovered at the next start (D-020) or with **Files: Recover original names after interruption**. Never delete recovery material as a privacy-cleanup shortcut. |
| `.rustphin-part-…` entries in a destination folder | A copy, extraction or new file being written under its temporary name | Removed or renamed into place when the operation ends, fails or is stopped. One can remain after a crash or power loss; it holds a partial copy of the source. |
| `$XDG_CACHE_HOME/rustphin/.extract-<pid>-<n>/` (default `~/.cache/…`) | Only while 7-Zip or unrar reads an archive libarchive cannot: the archive's unpacked content, in a private 0700 folder | Removed when the read ends, whether it succeeded or not. A crash during the read can leave it; remove it with Rustphin closed. |
| stderr and any launcher capture | `tracing` diagnostics; warnings and errors can include paths even at the default `info` filter. Qt's own warnings go to the journal unless `QT_FORCE_STDERR_LOGGING=1` | Rustphin creates no log file. `RUSTPHIN_LOG` sets the filter. Captured stderr and the journal belong to the terminal, launcher or systemd; clear them there. Opt-in `RUSTPHIN_FRAME_LOG` and `RUSTPHIN_SCROLL_BENCH` lines contain timings and memory only. |

### Shared files Rustphin writes

| Location | Stored information | Clearing or retention |
|---|---|---|
| `$XDG_DATA_HOME/user-places.xbel` (default `~/.local/share/…`) | KDE's shared bookmarks, including paths and metadata; Rustphin adds, renames, hides, removes and reorders them | Manage places through the panel. The file is shared with Dolphin and KDE's file dialogs; deleting it also removes their bookmarks. Rustphin never writes over a file it could not parse. |
| `$XDG_CACHE_HOME/thumbnails/` | Shared PNG previews holding the original file's URI, modification time and sometimes image dimensions; Rustphin's failure markers in `fail/rustphin-<version>/` | No clear command. With thumbnail-producing applications closed, remove entries or use the desktop's cache controls; other applications can recreate them. Files Rustphin writes are 0600, folders it creates 0700. |
| `$XDG_DATA_HOME/Trash/` and per-volume `.Trash-<uid>` / `.Trash/<uid>` | Trashed content and `.trashinfo` records with original path and deletion date | Restore items, or confirm Empty Trash for permanent removal. Clearing history never touches the trash. |
| `$XDG_CONFIG_HOME/mimeapps.list` | Only after Settings › General › Default file manager › "Make Rustphin the default": Rustphin as the default for folders (`inode/directory`); and after Open with › Other application… with "Always open “type” files with it" ticked: the chosen application as that type's default (W-169) | Change the default back in System Settings › Default Applications (File manager, or the file type under File Associations). |

### Read but never written

KDE's `kdeglobals` (terminal, click behaviour, icon theme), `mimeapps.list`
and the applications' `.desktop` files, icon themes, the shared MIME
database, `user-dirs.dirs` and the templates folders, network-folder
entries in `$XDG_DATA_HOME/remoteview/`, the mount table, and `/proc` (other
running Rustphin copies, own memory).

### Outside the files

To remove Rustphin's saved folder visits outside the application, quit it
(`rustphin --quit`, since it may keep running without a window) and remove
`recent-locations.toml`, `session.toml` and `views.toml` from the state
folder, and the start folder from `settings.toml` if one is set. This does
not remove bookmarks, previews, trash, recovery records, other applications'
history or captured logs. A running Rustphin can save paths again, so it
must be quit first. Removal and atomic replacement are logical deletion, not
erasure of backups, snapshots or storage remnants.

Back/Forward navigation, operation undo (the last 100 reversible
operations), recently closed tabs, palette query text, archive passwords and
the in-app clipboard live in memory for the run. Quitting releases them.
The shared Wayland clipboard and clipboard history tools may keep copied
paths independently. Rustphin does not write KDE's recent-documents
database or an operation-history database. Administrator mode is never
saved.

## Remaining verification

- On the GPUI build, W-077d observed clear/restart, permission failure and
  retry, protection of damaged and newer history files, and offline removal
  of the history and session files in a private home; W-077e–g verified
  ordered session saves, byte-safe session paths and protection of damaged
  session files. The Qt interface reuses the same `rustphin-view` code for
  history and sessions, but these replays have not been repeated on it.
- Cross-process merging of saved files and flushing on immediate shutdown
  are not proved.
- No connection trace has been made on the Qt build (see above).
- The protection of damaged settings and place-groups files was seen on
  screen (W-140); that of `views.toml` (W-143) and `window.toml` (W-160) is
  unit-tested only.
