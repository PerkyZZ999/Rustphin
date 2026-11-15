# Compress and extract (W-111): plan for approval

> **Status (2026-11-14): built; this plan is kept as history.** Option A was
> built in W-111a–g (2026-11-13) and moved to the Qt interface in W-114e–f.
> The current behaviour is described in REQUIREMENTS.md (R-021),
> ARCHITECTURE.md (`rustphin-core` `archive.rs` and `ops/archive.rs`;
> `rustphin-platform` `libarchive.rs` and `archive_tool.rs`), the menus in
> `crates/rustphin-view/src/menu.rs` and the dialog in `dialogs.rs`, and
> UI_SCENARIOS.md (AR1–AR10). How it differs from this plan:
>
> - **No Ark.** Formats the core does not read itself (RAR, ISO, CAB, CPIO,
>   lone `.gz` and others) are read through libarchive inside Rustphin's own
>   engine, with the same progress, Stop, conflict and password questions,
>   undo and entry checks; 7-Zip (`7z`, `7zz`, `7za`), then unrar, only for
>   what libarchive cannot open (D-036, W-119). The reader is handed to the
>   engine with `Fs` and `Trash` (`archive::ForeignReader`, W-129), so the
>   core has no C dependency. Writing stays ZIP, TAR and 7Z.
> - **Compress to…** offers ZIP, TAR.GZ, TAR.XZ, TAR.ZST and 7Z at three
>   levels (Fast, Normal, Smallest), as planned; it is a QML dialog, and
>   Extract to… uses Qt's folder dialog, which starts in the folder shown
>   (W-113, W-114g), not the GPUI portal prompt.
> - **Several archives** get "Extract each into its own folder" in place of
>   "Extract into "name/"", and "Extract and trash archives".
> - **Read-only folders** (W-146e): the three entries that extract into the
>   folder shown are disabled there and the first reads "Extract here
>   (read-only folder)"; from the palette they say "This folder is
>   read-only. Use Extract to… and choose another folder." "Actions menu"
>   in this plan is the command palette (Files: Extract here, …).
> - **Trash and administrator:** no Extract or Compress in either, from the
>   menu or the palette (W-140); links whose target climbs out after a name
>   are left out (W-133).

**Approved 2026-11-13 with all five recommendations (D-032, R-021).**

The maintainer asked on 2026-11-13 for compressing and extracting from the context menu, with several choices: extract here, into a new folder, and into a folder followed by removing the archive. This plan says what Dolphin offers, how Rustphin would do it, and what the maintainer needs to decide.

## What Dolphin does

Dolphin's menus come from Ark's two plugins (Ark's `compressfileitemaction.cpp` and `extractfileitemaction.cpp`, read 2026-11-13). They run Ark's code inside Dolphin's process; they do not start the `ark` program.

- **Compress ›** (local files, one or more, except a single archive): `Compress to "name.tar.gz"`, `Compress to "name.zip"`, `Compress to…`. The last opens Ark's dialog, where you choose the name, format and options. The name comes from the selection: one item's name, or the common start of several names, or "Archive". In a read-only folder the first two are disabled. The new archive is selected when it is done.
- **Extract ›** (one or more archives Ark can read):
  - `Extract here`: if the archive holds a single folder or file, it lands here as it is. Otherwise a new folder named after the archive is made first.
  - `Extract and trash archive`: the same, then the archives go to the trash, but only if every one succeeded.
  - `Extract to…`: Ark's dialog asks where to extract.

  If the archive's folder is read-only, only `Extract to…` is offered.
- Progress appears in Plasma's notifications, and errors in message boxes. Archives extract one after another, and the first failure stops the rest.

## The menus Rustphin would show

Same order and wording as Dolphin, plus the "new folder" choice the maintainer asked for:

- **Extract ›**
  - **Extract here.** As Ark: a single top-level folder or file lands as it is; anything else goes into a new folder named after the archive.
  - **Extract into "name/".** Always into a new folder named after the archive.
  - **Extract and trash archive.** As "Extract here", then the archive goes to the trash, only if everything extracted. Trash, not permanent deletion (D-011), so it can be undone.
  - **Extract to….** KDE's folder dialog (through the desktop portal, which the pinned GPUI supports), then as "Extract here" into the chosen folder.
- **Compress ›**
  - **Compress to "name.zip"**
  - **Compress to "name.tar.gz"**
  - **Compress to….** A small Rustphin dialog in the design's style: name, format (ZIP, TAR.GZ, TAR.XZ, TAR.ZST, 7Z) and compression level (fast, normal, smallest).

The menus appear on the selection in every view, in the palette ("Files: Extract here", …) and, for archives, in the Actions menu. Nothing appears in the trash or in administrator mode. In a read-only folder the "here" entries are disabled and say why.

## Who does the work

This is decision 1. Three ways:

- **A. Rustphin's own engine, with Ark for the rest (recommended).** ZIP, TAR (plain, .gz, .bz2, .xz, .zst) and 7Z are read and written by Rust libraries inside the core's operation engine, like copy and move today:
  - progress, Stop and errors in Rustphin's own operation bar;
  - name clashes through the existing "already exists" question (skip, replace, keep both);
  - Undo trashes what was made;
  - fault injection through `Fs`, as for every other operation;
  - every write uses a temporary name, and nothing is overwritten without asking (D-011).

  Formats the libraries cannot handle (RAR, ISO, CAB, LHA, …) are handed to Ark when it is installed, or to the 7-Zip or unrar programs. There is no RAR library that is free and compatible with Rustphin's licence. Those extractions show Ark's progress and dialogs, not Rustphin's.
- **B. Ark for everything.** Rustphin runs `ark --batch --autosubfolder …` and `ark --batch --add-to …`. It is quick to build, reads every format Ark knows, and Ark asks for passwords and name clashes itself. But progress shows in Plasma's notifications instead of Rustphin, there is no Undo and no Stop in Rustphin, it needs Ark installed, and the files are written outside Rustphin's engine, so D-011's guarantees (temporary names, no silent overwrite) rest on Ark.
- **C. Rustphin's own engine only.** As A, but no RAR or ISO at all.

## Safety (A and C)

Archives are untrusted input:
- **No escaping the destination.** Entries with absolute paths, `..` steps or links that lead outside the destination are refused and named in the report.
- **Ordinary files and folders only.** Device nodes and pipes are skipped; set-user-ID and set-group-ID bits are dropped.
- **Space is checked first.** Archives that declare their sizes (ZIP, 7Z) are compared with the free space before starting; a stream that keeps growing past it is stopped.
- **Fuzzing.** The code that checks entries before anything is written gets its own fuzz target (`archive_entry`).
- **Passwords.** For an encrypted ZIP or 7Z, Rustphin asks once per archive in its own dialog. The password is kept only for that operation, never logged and never saved. A wrong password is asked again, then skipped with a reason.
- **Names are bytes.** Names that are not UTF-8 are kept as bytes (ZIP marks the encoding; TAR names are bytes).

## Requirement (draft, R-021, after approval)

- **Extract** offers here, into "name/", to a chosen folder, and extract-then-trash, as listed above, for ZIP, TAR (plain, .gz, .bz2, .xz, .zst) and 7Z; with option A, also every format Ark or 7-Zip can read.
- **Compress** makes ZIP and TAR.GZ named as Dolphin does, and Compress to… offers the five formats and three levels.
- **Name clashes** go through the existing conflict question; nothing is overwritten without a choice; partial results of a stopped or failed operation are cleaned up.
- **Progress and Stop** in the operation bar; **Undo** removes what the operation made (to the trash); extract-then-trash restores the archive too.
- **Hostile archives** cannot write outside the destination, cannot create devices or set-ID files, and cannot fill the disk unnoticed; an archive that is damaged or has a wrong password ends with a readable reason.
- **Encrypted ZIP and 7Z** ask for a password, which is never shown in logs or kept.

## Order of work

- **W-111a — reading:** list and extract ZIP, TAR (all five) and 7Z in the core engine, with the entry checks, conflicts, progress, Stop, cleanup and fault injection; the fuzz target.
- **W-111b — the Extract menu:** the four entries, the palette and the Actions menu; "Extract to…" through the portal's folder dialog; extract-then-trash; Undo.
- **W-111c — writing:** ZIP and TAR.GZ (Compress to "name.…"), then TAR.XZ, TAR.ZST and 7Z with three levels; naming as Dolphin; selecting the result.
- **W-111d — Compress to…:** the dialog, with mockup first (`docs/mockup/`), as for Settings.
- **W-111e — passwords:** the password question for encrypted ZIP and 7Z.
- **W-111f — other formats (only with A):** handing other formats to Ark, then 7-Zip or unrar, with a readable message when none is installed.
- **W-111g — UI scenarios and docs:** UI_SCENARIOS (each entry, a clash, Stop, Undo, a hostile archive), DESIGN.md and KEYBOARD.md if needed.

New dependencies, each checked for licence before it is added: `zip` (MIT), `tar` (MIT or Apache-2.0), `flate2` (already in the build), `bzip2`, `liblzma` or `lzma-rust2`, `zstd` and `sevenz-rust2` (MIT or Apache-2.0). All are GPLv3-compatible as listed; this is confirmed again when each is added.

## For the maintainer to decide

1. **Who does the work:** A (recommended), B or C, as above.
2. **The extra "Extract into "name/"" entry:** keep it, as you asked, or offer only Ark's three? Recommended: keep it.
3. **ZIP or TAR.GZ first under Compress?** Ark puts TAR.GZ first. Recommended: ZIP first, since it opens everywhere, including Windows and phones.
4. **Making encrypted archives in Compress to…:** recommended later, not in this work. Opening encrypted archives is in.
5. **Opening the folder after extracting:** Ark has a setting for it, off by default. Recommended: do not open it, and select the result in the current view instead.
