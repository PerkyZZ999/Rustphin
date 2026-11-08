# Privacy and local data

This describes the implementation audited for W-077a–f on 2026-11-06. R-108
acceptance remains open. W-077d adds bounded live connection and clearing
evidence in VALIDATION.md; neither the inventory nor that replay guarantees
secure erasure or absence of network activity on every execution path.

## Connections and external applications

Rustphin has no application-owned HTTP client, remote service, updater or
telemetry collector. The Linux application leaves the pinned GPUI default
`NullHttpClient` installed; its `send` returns `No HttpClient available`.
Application source has no TCP/UDP client or telemetry setup. These are source
review findings, not proof about every dependency execution path.

Local IPC is required: Wayland for display, clipboard and effects; session
D-Bus for FileManager1 and accessibility; system D-Bus for UDisks2. Bus addresses
come from the desktop environment. Rustphin does not enforce Unix-only D-Bus
addresses. Opening a file hands its path to the chosen desktop application
(or `xdg-open`); explicitly approved programs/shortcuts can also run. Their
connections and history, and filesystem access to an already mounted remote
filesystem, are outside Rustphin's application-owned connection contract.

The active Linux dependency graph contains `async-net` through the pinned
toolkit's `smol`, and `http`/`http-body` through `gpui-pre-http-client`.
Consequently R-108's literal validation text, “no network crates or sockets in
the dependency audit”, is not satisfied. Required local sockets also exist.
No requirement or locked toolkit decision is changed by this finding. Five
W-077d application lifetimes were traced from startup through normal exit with
all threads followed: 60 socket calls, all AF_UNIX, and no observed IPv4/IPv6
sockets. The private replay covers startup, folder visits, palette clearing,
write failure/retry and restarts, not external application launches, alternate
bus transports or every desktop/runtime path. The literal criterion remains
unsatisfied.

## Data inventory and clearing

Absolute XDG overrides take precedence. Defaults below are relative to `HOME`.
Rustphin's own config/state files are replaced atomically with mode 0600;
parent directories follow the process umask. These files are not encrypted.

| Location | Stored information | Clearing or retention |
|---|---|---|
| `$XDG_STATE_HOME/rustphin/recent-locations.toml` (default `.local/state/…`) | At most 10,000 distinct successful folder visits, newest first; byte-safe local file URIs | Ctrl+Shift+P → **History: Clear recent folders**. Wait for **Recent folders cleared.** New visits start recording again. Read/save errors are reported in the palette; do not treat an error as a successful disk clear. |
| `$XDG_STATE_HOME/rustphin/session.toml` | Open tabs, byte-safe local file URIs for each pane and active indices (format 2; legacy UTF-8 format 1 upgrades on save) | Clearing recent folders does not clear this file. Quit every Rustphin instance, then remove this file to forget saved tabs. Start with explicit folder arguments to choose replacement tabs. If the file is unreadable, damaged or from a newer format, Rustphin keeps it unchanged, turns tab saving off for that run (explicit folders included) and shows why. |
| `$XDG_CONFIG_HOME/rustphin/settings.toml` (default `.config/…`) | Global view mode, zoom, sorting and hidden-file preference | Quit all instances, then remove the file to reset preferences. No per-folder view database or configurable key-binding file is implemented. |
| `$XDG_CONFIG_HOME/rustphin/place-groups.toml` | Group titles, ordering and bookmark identifiers | Remove groups through the places UI; to reset all groups, quit all instances and remove this file. Shared bookmarks remain. |
| `$XDG_DATA_HOME/user-places.xbel` (default `.local/share/…`) | Shared KDE bookmarks, including paths and metadata | Manage individual places through the UI. This is shared with Dolphin and file dialogs; deleting it also removes their bookmarks. |
| `$XDG_CACHE_HOME/thumbnails/` (default `.cache/…`) | Shared PNG previews containing original file URI, modification time and sometimes image dimensions; failure markers in `fail/rustphin-<version>/` | No disk-cache clear command is implemented. After closing thumbnail-producing applications, remove selected cache entries or use the desktop's cache controls. Other applications can recreate them. Thumbnail files are 0600, created directories 0700. |
| `$XDG_DATA_HOME/Trash/` and per-volume freedesktop trash | Deleted content and `.trashinfo` with original path/deletion date | Restore individual items, or explicitly confirm Empty Trash for permanent removal. History clearing never removes recoverable content or trash records. |
| `.rustphin-rename-journal` in a batch's working directory | Original, staged and final names plus file identity needed for recovery | Removed after successful completion/recovery and directory synchronization. Keep it and staged entries after interruption or a recovery error; use **Recover original names after interruption**. Never delete recovery material as a privacy-cleanup shortcut. |
| stderr and any launcher capture | `tracing` diagnostics; warnings/errors can include paths even at the default `info` filter | Rustphin creates no log file or rotating-log store. `RUSTPHIN_LOG` sets the filter. Captured stderr belongs to the terminal/launcher/journal; clear it through its owner. Opt-in frame/profiler diagnostics also contain local paths/source locations. |

To remove Rustphin-owned saved folder visits outside the application, quit all
instances and remove `recent-locations.toml` and `session.toml` from the state
directory. This does not remove bookmarks, previews, trash, recovery records,
external application history or captured logs. A running instance can save
paths again, so closing all instances matters. Removal/atomic replacement is
logical deletion, not erasure of backups, snapshots or storage remnants.

Back/Forward navigation, operation undo (up to 100 reversible operations),
palette query text and the in-app clipboard live in memory for the run.
Closing the application releases them. Shared clipboard owners/history tools
may retain copied paths independently. Rustphin does not write KDE's recent
document database or an operation-history database.

## Remaining verification

- W-077d observes normal clear/restart, permission-failure/retry, damaged/newer
  history protection, and offline removal of the history/session files in a
  private generated-data home. After clearing, new successful visits are saved
  again; old history-only paths do not return. Source and replay agree that
  damaged/newer files remain protected instead of claiming a disk clear.
- W-077e orders session publication within each workspace and retires stale
  pending snapshots after a newer attempt, including failure. Tab churn and
  split/single-tab restart are replayed privately. Cross-process merging and
  immediate-shutdown flushing are not proved. W-077f verifies byte-safe
  session paths, legacy version-1 upgrade and real raw-name split restoration.
  W-077g keeps unreadable, damaged and newer-format session files unchanged for
  the run, with or without explicit launch folders, and reports the reason.
  Broader connection paths remain outside the connection replay.
- W-077c reviews licences/advisory exceptions under the existing policy and
  verifies clean linked builds on Rust 1.95.0 and published stable 1.99.1.
  `rust-tc doctor` passes; this does not make a release attribution bundle or
  accept the remaining privacy criteria. Dated evidence is in VALIDATION.md.
