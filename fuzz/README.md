# Parser fuzzing

Run from the repository root through Rust-Toolchain:

```sh
rust-tc fuzz uri
rust-tc fuzz places
rust-tc fuzz desktop
rust-tc fuzz images
rust-tc fuzz admin_entry
```

Each campaign defaults to 30 seconds after compilation, a 4,096-byte input
limit, a five-second per-input timeout, a 2 GiB RSS limit and seed 7501. An
optional second argument changes the campaign duration, for example
`rust-tc fuzz images 60`. Requires nightly Rust and `cargo-fuzz`.

- `uri`: byte-preserving URI/list round trips, local-path validation and safe
  filename display.
- `places`: malformed XML plus add/remove/rename/hide contracts through the
  shared places parser. Successful edits must remain readable with their
  intended bookmark values; a panic or lost bookmark is a failure.
- `desktop`: shortcut classification and Exec argument parsing. Uses only a
  generated temporary `.desktop` file; never invokes a launch API. Command
  arguments must be NUL-free, and parsing must preserve the source bytes.
- `admin_entry`: entries from KDE's kio-admin helper (KIO `UDSEntry` in Qt's
  `QDataStream` layout, W-098). Any input must decode or be refused without a
  panic; an accepted entry must have a plain name (not empty, no `/`),
  permission bits only, and valid nanoseconds. Seeds are entries KIO 6.31
  itself wrote.
- `images`: thumbnail generation and malformed image handling in a fresh
  temporary source/cache. Success must be discoverable through cache lookup;
  failure must create a failure marker; source bytes must remain unchanged.

Reviewed starting inputs live in `seeds/`. The command copies them into the
ignored mutable `corpus/`; libFuzzer discoveries and crashes stay in ignored
`corpus/` and `artifacts/`. Convert findings into small regression tests beside
the affected parser before fixing them, and retain useful new seeds. These
short campaigns are evidence within their input/time bounds, not proof that
every parser input is safe or that image decoding has a wall-clock deadline.

The independent workspace keeps libFuzzer out of the application build.
`rust-tc doctor` includes `fuzz-check` for harness formatting, Clippy and the
same dependency/licence policy as the application. It does not run campaigns;
run those explicitly after relevant parser changes. The fuzz lockfile starts
from the application versions; harness-only dependency updates are reviewed
separately. No UI toolkit is linked into the harness.
