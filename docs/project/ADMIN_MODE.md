# Act as Administrator: design and threat model (W-098)

Status: **approved by the maintainer on 2026-11-10 (D-028)**, as D-027 required
before any code. Written 2026-11-10. Built in W-098a–d (2026-11-10) and
carried into the Qt interface in W-114f (2026-11-14); "As built" below says
how the current build differs from this design. Open: W-098e, the
maintainer's check with the real helper (the checklist at the end).

## What the maintainer asked for

A toolbar button, before Back, that lets him work on files he does not own
(for example `/etc`), "exactly like Dolphin does". Dolphin's *Act as
Administrator* switches the view to `admin:/…`; KDE's own polkit dialog asks
for the password; the view is marked; the same button switches it off.

## How Dolphin does it (read from kio-admin's source, 2026-11-10)

- A root helper, `kio-admin-helper`, is started on demand on the **system**
  D-Bus as `org.kde.kio.admin` (package `kio-admin`, installed here).
- Every method call is checked against the polkit action
  `org.kde.kio.admin.commands` ("Manage files as administrator",
  `auth_admin_keep`). The subject is **the calling process's bus
  connection**. KDE's polkit agent shows the password dialog; polkit then
  remembers the answer for that process for a few minutes.
- Each operation is a method on `/` (`listDir(s)`, `stat(s)`, `get(s)`,
  `put(sii)`, `copy(ssii)`, `del(s)`, `mkdir(si)`, `chmod(si)`,
  `chown(sss)`, `rename(ssi)`, `file(si)`) that returns the path of a
  command object. The client calls `start()` on it and receives signals:
  `entries`, `statEntry`, `data`, `dataRequest`, and finally
  `result(int error, string message)`.
- Directory entries are KIO's own `UDSEntry`, written with Qt's
  `QDataStream` into a byte array inside a struct (`a(ay)` for a list).
- The helper runs KIO's `file:` jobs as root. It checks nothing but polkit;
  it does not canonicalise paths.

## Proposed design

### Mechanism

1. **`rustphin-platform::admin`**: a client of `org.kde.kio.admin` over the
   system bus with zbus (already a dependency, blocking API, off the UI
   thread). It decodes `UDSEntry` byte arrays with a small bounded parser of
   the `QDataStream` layout (field count, then id + 64-bit number or UTF-16
   string per field).
2. **`AdminFs`**, in the platform crate, implements the core's `Fs` trait on
   top of that client. The browser and the operation engine use it in
   place of `RealFs` while the mode is on, so every change still goes
   through the engine and its rules (D-011), and the core keeps no
   knowledge of D-Bus.
3. Rustphin **never asks for, sees or stores a password** and never runs as
   root. The only privileged code is KDE's helper.

### The mode, as the user sees it (mockup 35c)

- **Per window.** The shield turns the whole window into administrator
  mode: every tab and both sides of a split. (Dolphin does it per view; one
  window-wide state is easier to see and harder to forget. The maintainer to
  confirm.)
- Switching on: Rustphin makes a first harmless call (`stat` of the folder
  shown); KDE's dialog asks for the password. Cancel or a wrong password
  leaves the mode off with a line saying so.
- While on: the shield is red (`error` on `error-surface`), a 36 px bar
  under the toolbar says "Acting as administrator. Changes you make here can
  affect the whole system; be careful." with **Stop acting as
  administrator**, and the title says "(administrator)". When polkit's
  memory runs out, the next operation simply asks again.
- Switching off: the shield, Stop, or closing the window. The mode is
  **never saved** in the session and never comes back on its own.

### What works, and what changes, in the mode

| Works through the helper | Changes or is off |
|---|---|
| Browsing any folder; hidden files, sorting, filter, views | **Delete is always permanent** (the helper has no trash): Delete asks "Delete for good?" every time |
| New folder, new file, rename, copy, move, paste, duplicate, drag and drop within the window | **No live updates** (the helper cannot watch folders); a folder re-reads after Rustphin's own changes and on F5 |
| Properties, including owner, group and permissions (`chmod`, `chown`) | **No thumbnails** (they would read every picture as root) |
| Undo of renames and moves done in the mode | **Batch rename off** (its recovery journal needs a directory lock the helper does not offer) |
| | **Opening a file** starts the normal application as the maintainer, not as root: it can read what he can read and cannot save into root-owned folders, as in Dolphin with non-KDE apps |
| | Copies to or from the mode go through the helper (it reads and writes as root) |

### Threat model

| Threat | Mitigation | What remains |
|---|---|---|
| A bug in Rustphin damages system files while the mode is on | Every change goes through the operation engine (temporary name, verify, no silent overwrite); the mode is explicit, marked, per window, and never restored at start | Durability is weaker than normal: the helper cannot `fsync` or lock directories, so a power cut during a change can lose it |
| Password captured or logged | Rustphin never handles it; KDE's polkit agent does | None in Rustphin |
| Another program borrows Rustphin's authorisation | polkit authorises Rustphin's own bus connection only; nothing is mounted or exported (unlike mounting `admin:/` through kio-fuse, which would let every program of the user use root access through the mount, rejected for that reason) | Code injected into Rustphin's own process would have the access, as with Dolphin |
| A file name that is not valid UTF-8 reaches the helper as a different name (the helper takes URLs as Qt strings) | Names are sent as percent-encoded `file://` URLs; any name that does not round-trip exactly is **refused** with a message, never sent | Such files cannot be changed in the mode |
| Symlink races in folders other users can write (classic root-in-`/tmp` attacks) | Operations name the exact paths Rustphin listed; the mode marks folders not owned by root with a warning line | KIO, as root, follows symlinks the way `cp` would; Dolphin has the same exposure |
| Malformed data from the helper | The `UDSEntry` parser is bounded (field count, string length) and rejects anything malformed; it gets unit tests from real samples and a fuzz target (a real parser, so a real target) | None expected |
| KDE changes this private interface | Rustphin checks the helper exists and answers as expected; anything unexpected ends the mode with a message instead of guessing | The feature may need updating after a KDE upgrade |
| The helper is missing (`kio-admin` not installed) | The shield explains how to install it | None |

### Testing

- Parser and URL round-trip: unit tests and a fuzz target.
- `AdminFs` and the mode: against a **fake helper** on a private D-Bus (as
  the device and kio-fuse tests already do), including denial, cancel,
  expiry and every error path. No test ever runs anything as root.
- On screen: the mode's look and flows in the isolated session with the
  fake helper.
- **the maintainer** does the real check on his desktop (the real password prompt
  and a harmless change, for example a test folder under `/opt`), since
  agents must not type passwords.

### Slices

1. W-098a: helper client, `UDSEntry` parser, URL round-trip, fake helper,
   tests and fuzz target.
2. W-098b: read-only mode: `AdminFs` listing and stat, shield, bar, title,
   prompt and denial handling.
3. W-098c: changes: new folder or file, rename, copy, move, delete for good,
   undo, Properties with owner and permissions.
4. W-098d: replay, docs (ARCHITECTURE trust boundary, DESIGN).
5. W-098e: the maintainer's real-desktop check (open).

## Decisions for the maintainer

1. Approve the mechanism: Rustphin talks to KDE's kio-admin helper directly,
   the way Dolphin does (not kio-fuse, not a root helper of our own).
2. Per-window mode (recommended) or per-pane like Dolphin?
3. Accept the changes in the table: permanent delete with confirmation, no
   live updates, no thumbnails, no batch rename in the mode.

## As built

### Where it lives (the Qt interface, since W-114f)

- `rustphin-platform::admin`: `AdminHelper` (the helper on the system bus,
  or on the session bus when `RUSTPHIN_ADMIN_BUS=session`, for tests with
  `cargo run -p rustphin-platform --example fake_admin_helper -- <sandbox>`),
  `decode_entry` (the bounded `UDSEntry` parser, at most 512 fields; fuzz
  target `admin_entry`), `admin_url` (refuses any path that is not absolute,
  plain and valid UTF-8 rather than send a different name) and `AdminFs`
  (the core's `Fs` over the helper). Each call allows interactive
  authorisation, so KDE's polkit agent may ask for the password; one call
  runs at a time; an operation may take up to 120 s, password dialog
  included; a file is read or written whole, at most 256 MiB. `installed()`
  looks for `dbus-1/system-services/org.kde.kio.admin.service` in the XDG
  data folders.
- `crates/rustphin/src/admin.rs`: the on/off state. Switching on stats `/`
  through the helper (this is when KDE asks); the notice says "Asking for
  administrator access… Enter your password in KDE's dialog." Refused:
  "Administrator access was not granted: the password was not given.";
  helper missing: "KDE's administrator helper could not be reached. Install
  the kio-admin package, then try again." Switching off says "No longer
  acting as administrator."
- The hub runs administrator operations on a runner of their own with
  `AdminFs`, numbered in a range of their own (`admin_first_id`, each
  run 2^40 apart); panes switch their
  directory model to `AdminFs` and back (`FolderModel::switch_filesystem`),
  starting their history afresh.
- Toolbar: the shield (`admin` item) reads "Act as administrator", "Act as
  administrator (needs KDE's kio-admin package)" (disabled) when the helper
  is not installed, "Waiting for the password…" while asked, and "Stop
  acting as administrator" in the danger colour while on. Main.qml shows the
  bar "Acting as administrator. Changes you make here can affect the whole
  system; be careful." with **Stop acting as administrator** on
  the `danger_surface` token.
- In the mode: Delete always deletes for good after asking
  (`selection_actions`); no thumbnails (`FolderModel::thumbnail`); no live
  updates (`AdminFs::watch` is unsupported; folders re-read after
  Rustphin's own changes and on F5); Extract and Compress are refused with
  "Archives cannot be extracted or made as administrator." (W-140), and the
  administrator runner never hands an archive to 7-Zip (W-129); Properties
  shows owner and group through the helper (`properties::described_as_administrator`).

### Differences from the design

- **One state for the whole process, not per window.** The Qt interface
  keeps the mode in one place for the process: while it is on, every
  window, tab and pane lists and works through the helper, and closing a
  window does not switch it off (with background mode, R-020, the process
  outlives its windows). The shield or Stop switches it off; the mode is
  still never saved in the session. (Read from the code; not run with
  several windows.)
- **The title is not marked.** The design's "(administrator)" in the window
  title is not built in the Qt interface; the red shield and the bar mark the
  mode.
- **No warning line for folders not owned by root.** Proposed in the threat
  model; not built.
- **Batch rename is not hidden.** The Qt interface still offers it; the
  engine needs a directory lock, which `AdminFs` does not offer
  (`lock_directory` is unsupported), so it ends with an error instead of
  renaming. Inferred from the code; not run (administrator mode is never run
  against the real helper by agents).

### Found while building (W-098a–c, 2026-11-10)

- **New folders take the user's umask.** KIO sets the mode it is given
  exactly, where `mkdir(2)` filters it; without this a new folder made as
  root was world-writable. Covered by a regression test.
- **Moves across filesystems keep the original.** The helper cannot `fsync`,
  so the engine cannot prove the copy is safe on disk and leaves the source
  where it was (D-011), with the usual notice. Moves within one filesystem
  are renames and work.
- **Owner and group are shown, not changed.** Properties has never offered
  changing the owner; the helper's `chown` is wired in `AdminHelper` for
  when it does.
- **Switching the mode starts each pane's history afresh**, so Back is
  unavailable right after switching.
- **Opening a file** in the mode starts its application as the maintainer, as
  planned.

## The maintainer's check on the real desktop (W-098e)

Agents must not type passwords, and the isolated session shares the real
system bus, so only this check uses the real helper. With the installed
build (after `rustphin --quit`, so the new build answers launches):

1. Open `/etc`; click the shield. KDE's own dialog should ask for the
   password ("Manage files as administrator"). Cancel it: the notice should
   say "Administrator access was not granted…", and the mode stays off.
2. Click the shield again and enter the password: red shield, red bar
   "Acting as administrator…"; `/etc` lists as before.
3. Open `/opt` (or another root-owned folder you do not mind touching).
   New folder, name it `rustphin-test`; it should appear at once, owned by
   root, `rwxr-xr-x`.
4. Rename it to `rustphin-test-2`; open Properties (owner root); tick Group
   › Write, Apply, then Undo.
5. Delete it: "Delete for good?" should ask; confirm, and it disappears.
6. Click "Stop acting as administrator": the bar goes and `/opt` lists as
   you.
7. Wait about five minutes with the mode on, then make another change:
   the password should be asked again (polkit's memory ran out), not fail.

Note anything that differs in TEST_RUN.md.
