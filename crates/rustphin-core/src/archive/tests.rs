use std::{collections::BTreeMap, io::Cursor};

use super::*;

#[test]
fn formats_are_known_by_content_and_tar_streams_also_by_name() {
    assert_eq!(detect(b"a.zip", b"PK\x03\x04rest"), Some(Format::Zip));
    // A ZIP is a ZIP whatever it is called (.odt, .jar, no ending).
    assert_eq!(detect(b"report.odt", b"PK\x03\x04"), Some(Format::Zip));
    assert_eq!(
        detect(b"a.7z", b"7z\xBC\xAF\x27\x1Cxx"),
        Some(Format::SevenZ)
    );
    assert_eq!(detect(b"a.tar.gz", &[0x1F, 0x8B, 8]), Some(Format::TarGz));
    assert_eq!(detect(b"a.TGZ", &[0x1F, 0x8B, 8]), Some(Format::TarGz));
    // A lone compressed file is not a TAR: the foreign reader takes it.
    assert_eq!(detect(b"notes.txt.gz", &[0x1F, 0x8B, 8]), None);
    // A name that lies about the content is not believed.
    assert_eq!(detect(b"a.tar.xz", &[0x1F, 0x8B, 8]), None);
    let mut tar = vec![0; 512];
    tar[257..262].copy_from_slice(b"ustar");
    assert_eq!(detect(b"anything", &tar), Some(Format::Tar));
    assert_eq!(detect(b"a.rar", b"Rar!\x1A\x07"), None);
}

#[test]
fn the_folder_name_drops_the_whole_archive_ending() {
    assert_eq!(stem(b"photos.tar.gz"), b"photos");
    assert_eq!(stem(b"Backup.ZIP"), b"Backup");
    assert_eq!(stem(b"v1.2.tgz"), b"v1.2");
    assert_eq!(stem(b"plain"), b"plain");
    // A name that is only an ending keeps it.
    assert_eq!(stem(b".zip"), b".zip");
}

#[test]
fn entry_names_cannot_leave_the_destination() {
    assert_eq!(entry_path(b"a/./b/c.txt"), Ok(PathBuf::from("a/b/c.txt")));
    assert_eq!(entry_path(b"dir/"), Ok(PathBuf::from("dir")));
    assert!(entry_path(b"../evil").is_err());
    assert!(entry_path(b"a/../../evil").is_err());
    assert!(entry_path(b"/etc/passwd").is_err());
    assert!(entry_path(b"a\0b").is_err());
    assert!(entry_path(b"./").is_err());
    // Bytes that are not text are a name like any other.
    assert_eq!(
        entry_path(b"caf\xe9"),
        Ok(PathBuf::from(OsStr::from_bytes(b"caf\xe9")))
    );
}

#[test]
fn links_must_point_inside_the_destination() {
    let entry = Path::new("a/b/link");
    assert!(link_stays_inside(entry, b"../c.txt"));
    assert!(link_stays_inside(entry, b"../../top.txt"));
    assert!(!link_stays_inside(entry, b"../../../outside"));
    assert!(!link_stays_inside(entry, b"/etc/passwd"));
    assert!(!link_stays_inside(Path::new("link"), b".."));
    assert!(link_stays_inside(Path::new("link"), b"./x/y"));
    // A name before `..` may be a link the archive makes (W-133).
    assert!(!link_stays_inside(Path::new("link"), b"./x/../y"));
    assert!(!link_stays_inside(entry, b"l/../.."));
    assert!(!link_stays_inside(Path::new("link"), b""));
}

/// Five hours behind UTC, as Toronto is in winter.
fn five_hours_behind() -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(-5))
}

/// 2023-11-14 22:13:20 UTC: the time `sample` gives every entry.
const SAMPLE_TIME: i64 = 1_700_000_000;

fn zip_of_sample(zone: &TimeZone) -> Vec<u8> {
    let (_dir, entries) = sample();
    let open: Open<'_> = &|path| Ok(Box::new(std::fs::File::open(path)?));
    let mut out = Cursor::new(Vec::new());
    write_zip(
        Level::Fast,
        &mut out,
        &entries,
        zone,
        open,
        &mut |_| ControlFlow::Continue(()),
        &mut |_, reason| panic!("left out: {reason}"),
    )
    .unwrap();
    out.into_inner()
}

fn zip_modified(bytes: Vec<u8>, name: &[u8], zone: &TimeZone) -> Option<i64> {
    let mut modified = None;
    read_zip(
        Box::new(Cursor::new(bytes)),
        None,
        zone,
        &mut |_, _| ControlFlow::Continue(()),
        &mut |header, _| {
            if header.name == name {
                modified = header.modified;
            }
            Ok(ControlFlow::Continue(()))
        },
    )
    .unwrap();
    modified
}

#[test]
fn zip_times_are_written_in_local_time_with_the_exact_time_beside_them() {
    // Other programs show a ZIP's plain time as it is, as local time; the
    // extended stamp beside it holds the exact moment.
    let bytes = zip_of_sample(&five_hours_behind());
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let file = archive.by_name("top/a.txt").unwrap();
    let time = file.last_modified().unwrap();
    assert_eq!(
        (
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute()
        ),
        (2023, 11, 14, 17, 13)
    );
    let stamp = file.extra_data_fields().find_map(|field| match field {
        zip::extra_fields::ExtraField::ExtendedTimestamp(stamp) => stamp.mod_time(),
        _ => None,
    });
    assert_eq!(stamp, Some(1_700_000_000));
}

#[test]
fn a_zip_time_is_read_from_its_exact_stamp_or_else_as_local_time() {
    // With the stamp, the reader's own zone does not matter.
    let bytes = zip_of_sample(&five_hours_behind());
    let far_east = TimeZone::fixed(jiff::tz::offset(9));
    assert_eq!(
        zip_modified(bytes, b"top/a.txt", &far_east),
        Some(SAMPLE_TIME)
    );
    // Without one, as most Windows tools write them, the plain time is local.
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let time = zip::DateTime::from_date_and_time(2023, 11, 14, 17, 13, 20).unwrap();
    out.start_file(
        "plain.txt",
        zip::write::SimpleFileOptions::default().last_modified_time(time),
    )
    .unwrap();
    out.write_all(b"plain").unwrap();
    let bytes = out.finish().unwrap().into_inner();
    assert_eq!(
        zip_modified(bytes, b"plain.txt", &five_hours_behind()),
        Some(SAMPLE_TIME)
    );
}

/// 300,000 bytes that vary, so a reader that drops, repeats or reorders
/// a block is caught.
fn big_content() -> Vec<u8> {
    (0..300_000u32).map(|n| (n * 31 % 251) as u8).collect()
}

/// Files to write: name → content; folders end in `/`.
fn sample() -> (tempfile::TempDir, Vec<Source>) {
    let dir = tempfile::tempdir().unwrap();
    let file = |name: &str, content: &[u8]| {
        let path = dir.path().join(name.replace('/', "_"));
        std::fs::write(&path, content).unwrap();
        Source {
            path,
            name: name.as_bytes().to_vec(),
            kind: Kind::File,
            mode: 0o640,
            modified: SAMPLE_TIME,
            size: content.len() as u64,
        }
    };
    let entries = vec![
        Source {
            path: dir.path().to_path_buf(),
            name: b"top".to_vec(),
            kind: Kind::Directory,
            mode: 0o750,
            modified: SAMPLE_TIME,
            size: 0,
        },
        file("top/a.txt", b"alpha"),
        file("top/big.bin", &big_content()),
    ];
    (dir, entries)
}

fn read_back(format: Format, bytes: Vec<u8>) -> BTreeMap<Vec<u8>, (Kind, Vec<u8>, Option<u32>)> {
    let mut found = BTreeMap::new();
    read_entries(
        format,
        Path::new("test"),
        Box::new(Cursor::new(bytes)),
        None,
        &NoForeignReader,
        &mut |_, _| ControlFlow::Continue(()),
        &mut |header, content| {
            let mut data = Vec::new();
            content.read_to_end(&mut data)?;
            let name = header
                .name
                .strip_suffix(b"/")
                .unwrap_or(&header.name)
                .to_vec();
            found.insert(name, (header.kind.clone(), data, header.mode));
            Ok(ControlFlow::Continue(()))
        },
    )
    .unwrap();
    found
}

#[test]
fn every_format_rustphin_makes_reads_back_the_same() {
    let (_dir, entries) = sample();
    let open: Open<'_> = &|path| Ok(Box::new(std::fs::File::open(path)?));
    for format in [
        Format::Zip,
        Format::Tar,
        Format::TarGz,
        Format::TarXz,
        Format::TarZst,
        Format::SevenZ,
    ] {
        let mut out = Cursor::new(Vec::new());
        let mut bytes_seen = 0;
        write_archive(
            format,
            Level::Fast,
            &mut out,
            &entries,
            open,
            &mut |bytes| {
                bytes_seen += bytes;
                ControlFlow::Continue(())
            },
            &mut |_, reason| panic!("left out: {reason}"),
        )
        .unwrap();
        assert_eq!(bytes_seen, 300_005, "{format:?} reports every byte");
        let found = read_back(format, out.into_inner());
        assert_eq!(found.len(), 3, "{format:?}: {:?}", found.keys());
        assert_eq!(found[&b"top/a.txt".to_vec()].1, b"alpha", "{format:?}");
        assert!(
            found[&b"top/big.bin".to_vec()].1 == big_content(),
            "{format:?}: big.bin differs"
        );
        assert_eq!(found[&b"top".to_vec()].0, Kind::Directory, "{format:?}");
        assert_eq!(
            found[&b"top/a.txt".to_vec()].2,
            Some(0o640),
            "{format:?} keeps modes"
        );
    }
}

#[test]
fn stopping_while_writing_is_reported_as_stopped() {
    let (_dir, entries) = sample();
    let open: Open<'_> = &|path| Ok(Box::new(std::fs::File::open(path)?));
    let mut out = Cursor::new(Vec::new());
    let error = write_archive(
        Format::TarGz,
        Level::Fast,
        &mut out,
        &entries,
        open,
        &mut |_| ControlFlow::Break(()),
        &mut |_, _| {},
    )
    .unwrap_err();
    assert!(is_stopped(&error), "{error}");
}

#[test]
fn names_a_format_cannot_hold_are_left_out_with_a_reason() {
    let (_dir, mut entries) = sample();
    entries[1].name = b"top/caf\xe9.txt".to_vec();
    let open: Open<'_> = &|path| Ok(Box::new(std::fs::File::open(path)?));
    let mut left = Vec::new();
    let mut out = Cursor::new(Vec::new());
    write_archive(
        Format::Zip,
        Level::Fast,
        &mut out,
        &entries,
        open,
        &mut |_| ControlFlow::Continue(()),
        &mut |entry, reason| left.push((entry.name.clone(), reason)),
    )
    .unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].0, b"top/caf\xe9.txt");
    // TAR keeps the same name as bytes.
    let mut out = Cursor::new(Vec::new());
    write_archive(
        Format::Tar,
        Level::Fast,
        &mut out,
        &entries,
        open,
        &mut |_| ControlFlow::Continue(()),
        &mut |_, reason| panic!("left out: {reason}"),
    )
    .unwrap();
    assert!(read_back(Format::Tar, out.into_inner()).contains_key(b"top/caf\xe9.txt".as_slice()));
}

#[test]
fn a_damaged_archive_says_so() {
    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
        zip.start_file("a.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[1; 1000]).unwrap();
        zip.finish().unwrap();
    }
    bytes.truncate(bytes.len() / 2);
    let result = read_entries(
        Format::Zip,
        Path::new("test.zip"),
        Box::new(Cursor::new(bytes)),
        None,
        &NoForeignReader,
        &mut |_, _| ControlFlow::Continue(()),
        &mut |_, _| Ok(ControlFlow::Continue(())),
    );
    assert!(matches!(result, Err(ReadError::Damaged(_))));
}

#[test]
fn an_encrypted_zip_asks_for_its_password_and_refuses_a_wrong_one() {
    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default()
            .with_aes_encryption(zip::AesMode::Aes256, "right horse");
        zip.start_file("secret.txt", options).unwrap();
        zip.write_all(b"hidden").unwrap();
        zip.finish().unwrap();
    }
    let read = |password: Option<&str>| {
        let mut content = Vec::new();
        let result = read_entries(
            Format::Zip,
            Path::new("test.zip"),
            Box::new(Cursor::new(bytes.clone())),
            password,
            &NoForeignReader,
            &mut |_, _| ControlFlow::Continue(()),
            &mut |_, data| {
                data.read_to_end(&mut content)?;
                Ok(ControlFlow::Continue(()))
            },
        );
        (result, content)
    };
    assert!(matches!(read(None).0, Err(ReadError::NeedsPassword)));
    assert!(matches!(
        read(Some("wrong")).0,
        Err(ReadError::WrongPassword)
    ));
    let (result, content) = read(Some("right horse"));
    assert!(result.is_ok());
    assert_eq!(content, b"hidden");
}
