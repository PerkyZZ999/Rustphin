//! The libarchive reader against generated fixtures
//! (`tests/fixtures/archives`, see its README). libarchive is C, so these
//! tests do not run under Miri.

use std::{
    io::{self, Cursor, Seek, SeekFrom},
    path::PathBuf,
};

use super::*;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/archives")
        .join(name)
}

/// Every entry of the fixture `name`: its header and content.
fn entries(name: &str, password: Option<&str>) -> Result<Vec<(Header, Vec<u8>)>, ReadError> {
    let file = std::fs::File::open(fixture(name)).unwrap();
    let mut seen = Vec::new();
    read_entries(
        name.as_bytes(),
        Box::new(file),
        password,
        &mut |header, content| {
            let mut bytes = Vec::new();
            content.read_to_end(&mut bytes)?;
            seen.push((header.clone(), bytes));
            Ok(ControlFlow::Continue(()))
        },
    )?;
    Ok(seen)
}

fn names(entries: &[(Header, Vec<u8>)]) -> Vec<String> {
    let mut names: Vec<String> = entries
        .iter()
        .map(|(header, _)| {
            String::from_utf8_lossy(&header.name)
                .trim_end_matches('/')
                .to_owned()
        })
        .collect();
    names.sort();
    names
}

fn content_of<'a>(entries: &'a [(Header, Vec<u8>)], name: &str) -> &'a [u8] {
    &entries
        .iter()
        .find(|(header, _)| header.name == name.as_bytes())
        .unwrap_or_else(|| panic!("no entry {name}"))
        .1
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_cpio_gives_its_folder_files_and_link() {
    let seen = entries("bundle.cpio", None).unwrap();

    assert_eq!(names(&seen), ["link", "notes.txt", "sub", "sub/deep.txt"]);
    assert_eq!(content_of(&seen, "notes.txt"), b"hello\n");
    assert_eq!(content_of(&seen, "sub/deep.txt"), b"deep\n");
    let kind = |name: &str| {
        seen.iter()
            .find(|(header, _)| header.name == name.as_bytes())
            .map(|(header, _)| header.kind.clone())
            .unwrap()
    };
    assert_eq!(kind("sub"), Kind::Directory);
    assert_eq!(kind("notes.txt"), Kind::File);
    assert_eq!(kind("link"), Kind::Link(b"notes.txt".to_vec()));
    let notes = seen.iter().find(|(h, _)| h.name == b"notes.txt").unwrap();
    assert_eq!(notes.0.size, 6);
    assert!(notes.0.mode.is_some() && notes.0.modified.is_some());
}

#[test]
#[cfg_attr(miri, ignore)]
fn an_iso_is_read_through_its_gzip_wrapping() {
    let seen = entries("disc.iso.gz", None).unwrap();

    assert!(
        names(&seen).contains(&"notes.txt".to_owned()),
        "{:?}",
        names(&seen)
    );
    assert_eq!(content_of(&seen, "notes.txt"), b"hello\n");
    assert_eq!(content_of(&seen, "sub/deep.txt"), b"deep\n");
    // The disc's own root is not an entry to make.
    assert_eq!(names(&seen), ["notes.txt", "sub", "sub/deep.txt"]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_lone_compressed_file_is_one_file_named_after_the_archive() {
    let seen = entries("notes.txt.gz", None).unwrap();

    assert_eq!(names(&seen), ["notes.txt"]);
    assert_eq!(seen[0].0.kind, Kind::File);
    assert_eq!(seen[0].1, b"hello\n");
}

#[test]
#[cfg_attr(miri, ignore)]
fn the_visitor_can_stop_after_any_entry() {
    let file = std::fs::File::open(fixture("bundle.cpio")).unwrap();
    let mut visited = 0;
    read_entries(b"bundle.cpio", Box::new(file), None, &mut |_, _| {
        visited += 1;
        Ok(ControlFlow::Break(()))
    })
    .unwrap();

    assert_eq!(visited, 1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn the_visitors_own_failure_is_kept_apart_from_libarchives() {
    let file = std::fs::File::open(fixture("bundle.cpio")).unwrap();
    let result = read_entries(b"bundle.cpio", Box::new(file), None, &mut |_, _| {
        Err(io::Error::new(io::ErrorKind::StorageFull, "disk full"))
    });

    assert!(
        matches!(&result, Err(ReadError::Io(error)) if error.kind() == io::ErrorKind::StorageFull),
        "{result:?}"
    );
}

/// Content that fails after `good` bytes, as a failing disk would.
struct Failing {
    inner: Cursor<Vec<u8>>,
    good: u64,
}

impl io::Read for Failing {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.inner.position() >= self.good {
            return Err(io::Error::other("the disk went away"));
        }
        let left = usize::try_from(self.good - self.inner.position()).unwrap();
        let take = buffer.len().min(left);
        self.inner.read(&mut buffer[..take])
    }
}

impl Seek for Failing {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.inner.seek(to)
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_failing_source_is_an_io_error_not_damage() {
    let bytes = std::fs::read(fixture("bundle.cpio")).unwrap();
    let source = Failing {
        inner: Cursor::new(bytes),
        good: 0,
    };
    let result = read_entries(b"bundle.cpio", Box::new(source), None, &mut |_, _| {
        Ok(ControlFlow::Continue(()))
    });

    assert!(
        matches!(&result, Err(ReadError::Io(error)) if error.to_string() == "the disk went away"),
        "{result:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn something_that_is_no_archive_is_damaged() {
    let result = read_entries(
        b"notes.bin",
        Box::new(Cursor::new(b"\x00\x01 plainly not an archive".to_vec())),
        None,
        &mut |_, _| Ok(ControlFlow::Continue(())),
    );

    assert!(matches!(result, Err(ReadError::Damaged(_))), "{result:?}");
}

/// What [`probe`] says of the fixture `name`, given as content alone (no
/// path), and where it left that content.
fn probed(name: &str) -> (Probe, u64) {
    let mut source = Cursor::new(std::fs::read(fixture(name)).unwrap());
    let verdict = probe(&mut source).unwrap();
    (verdict, source.position())
}

#[test]
#[cfg_attr(miri, ignore)]
fn probe_accepts_what_libarchive_reads_and_sends_the_rest_to_7_zip() {
    assert_eq!(probed("bundle.cpio"), (Probe::Readable, 0));
    assert_eq!(probed("disc.iso.gz"), (Probe::Readable, 0));
    assert_eq!(probed("notes.txt.gz"), (Probe::Readable, 0));
    // 7Z encryption libarchive cannot undo.
    assert_eq!(
        probed("locked.7z"),
        (Probe::Unreadable("it is encrypted".to_owned()), 0)
    );
    assert!(matches!(probed("README.md"), (Probe::Unreadable(_), 0)));
}

#[test]
#[cfg_attr(miri, ignore)]
fn the_reader_gets_the_probed_source_whole() {
    let mut source = Cursor::new(std::fs::read(fixture("bundle.cpio")).unwrap());
    assert_eq!(probe(&mut source).unwrap(), Probe::Readable);
    let mut seen = Vec::new();

    read_entries(
        b"bundle.cpio",
        Box::new(source),
        None,
        &mut |header, content| {
            let mut bytes = Vec::new();
            content.read_to_end(&mut bytes)?;
            seen.push((header.clone(), bytes));
            Ok(ControlFlow::Continue(()))
        },
    )
    .unwrap();

    assert_eq!(seen, entries("bundle.cpio", None).unwrap());
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_failing_source_fails_the_probe_rather_than_go_to_7_zip() {
    let bytes = std::fs::read(fixture("bundle.cpio")).unwrap();
    let mut source = Failing {
        inner: Cursor::new(bytes),
        good: 0,
    };

    let result = probe(&mut source);

    assert!(
        matches!(&result, Err(error) if error.to_string() == "the disk went away"),
        "{result:?}"
    );
}
