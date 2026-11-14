# Archive fixtures (D-036)

Generated for the libarchive reader's tests from three files: `notes.txt`
("hello\n"), `sub/deep.txt` ("deep\n") and `link`, a symbolic link to
`notes.txt`.

- `bundle.cpio`: `bsdtar --format cpio -cf bundle.cpio notes.txt sub link`
- `disc.iso.gz`: `bsdtar --format iso9660 -cf disc.iso notes.txt sub`, then `gzip -9 -n`
- `notes.txt.gz`: `gzip -n` of `notes.txt` alone
- `locked.7z`: `7z a -p'secret' -mhe=off locked.7z notes.txt` (encryption libarchive cannot read)

TAR fixtures, written with Python's `tarfile` (mode 0644, uid 1000, mtime
1700000000), the trailing padding cut to the two end blocks:

- `pax-cafe.tar`: PAX format, one entry `café.txt` ("hello\n"); its name is
  in a pax `path` record, which libarchive cannot convert in the C locale
- `hardlink.tar`: ustar, `notes.txt` ("hello\n") and `again.txt`, a hard
  link to it
- `gap.tar`: ustar `notes.txt` ("hello\n"), then one 512-byte block of
  0x55 bytes (a header with a bad checksum), then ustar `deep.txt` ("deep\n")
