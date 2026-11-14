# Archive fixtures (D-036)

Generated for the libarchive reader's tests from three files: `notes.txt`
("hello\n"), `sub/deep.txt` ("deep\n") and `link`, a symbolic link to
`notes.txt`.

- `bundle.cpio`: `bsdtar --format cpio -cf bundle.cpio notes.txt sub link`
- `disc.iso.gz`: `bsdtar --format iso9660 -cf disc.iso notes.txt sub`, then `gzip -9 -n`
- `notes.txt.gz`: `gzip -n` of `notes.txt` alone
- `locked.7z`: `7z a -p'secret' -mhe=off locked.7z notes.txt` (encryption libarchive cannot read)
- `locked.zip`: `7z a -tzip -p'secret' -mem=AES256 locked.zip notes.txt`
