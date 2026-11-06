# Privacy and local data

This describes the implementation audited for W-077a on 2026-11-06. R-108
acceptance remains open; this inventory is not a runtime network trace or a
secure-erasure guarantee.

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
No requirement or locked toolkit decision is changed by this finding. Runtime
connection observation and reconciliation of this criterion remain open.

## Data inventory and clearing

Absolute XDG overrides take precedence. Defaults below are relative to `HOME`.
Rustphin's own config/state files are replaced atomically with mode 0600;
parent directories follow the process umask. These files are not encrypted.

| Location | Stored information | Clearing or retention |
|---|---|---|
| `$XDG_STATE_HOME/rustphin/recent-locations.toml` (default `.local/state/…`) | At most 10,000 distinct successful folder visits, newest first; byte-safe local file URIs | Ctrl+Shift+P → **History: Clear recent folders**. Wait for **Recent folders cleared.** New visits start recording again. Read/save errors are reported in the palette; do not treat an error as a successful disk clear. |
| `$XDG_STATE_HOME/rustphin/session.toml` | Open tabs, each pane's current path and active indices | Clearing recent folders does not clear this file. Quit every Rustphin instance, then remove this file to forget saved tabs. Start with explicit folder arguments to choose replacement tabs. |
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

- Observe connections in a private generated-data UI session; distinguish local
  IPC and launched children from application-owned Internet connections.
- Exercise clear/save failure and restart behavior, including damaged or newer
  history files and concurrent session saves. Current source refuses writes to
  damaged/newer history rather than claiming a disk clear.
- Review licences/advisory exceptions and clean linked builds under W-077.
  `rust-tc doctor` checks the reviewed dependency policy; a policy pass does not
  make a release attribution bundle or accept the remaining privacy criteria.
