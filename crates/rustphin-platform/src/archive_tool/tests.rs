//! Choosing between libarchive and the installed tool, and reading
//! through 7-Zip with the engine's password. The 7-Zip tests need 7-Zip
//! installed (the `7zip` package) and are skipped, saying so, without it.

use std::path::PathBuf;

use super::*;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/archives")
        .join(name)
}

/// Every entry read from the fixture `name`, as (name, content), with the
/// given tool and a fresh work folder, which must be gone afterwards.
fn read(
    name: &str,
    password: Option<&str>,
    tool: Option<Tool>,
) -> Result<Vec<(String, Vec<u8>)>, ReadError> {
    let work = tempfile::tempdir().unwrap();
    let path = fixture(name);
    let mut seen = Vec::new();
    let result = ForeignArchives::with_tool(tool, Some(work.path().to_owned())).read_entries(
        &path,
        Box::new(fs::File::open(&path).unwrap()),
        password,
        &mut |header, content| {
            let mut bytes = Vec::new();
            content.read_to_end(&mut bytes)?;
            seen.push((String::from_utf8_lossy(&header.name).into_owned(), bytes));
            Ok(ControlFlow::Continue(()))
        },
    );
    let left: Vec<_> = fs::read_dir(work.path()).unwrap().collect();
    assert!(left.is_empty(), "work folders left behind: {left:?}");
    result.map(|()| seen)
}

fn seven_zip() -> Option<Tool> {
    let tool =
        find(std::env::var_os("PATH").as_deref()).filter(|tool| matches!(tool, Tool::SevenZip(_)));
    if tool.is_none() {
        eprintln!("7-Zip is not installed; skipped");
    }
    tool
}

#[test]
fn seven_zip_is_preferred_then_unrar_and_ark_is_not_used() {
    let bin = tempfile::tempdir().unwrap();
    let path = bin.path().as_os_str();
    assert_eq!(find(Some(path)), None);
    fs::write(bin.path().join("ark"), "").unwrap();
    assert_eq!(find(Some(path)), None);
    fs::write(bin.path().join("unrar"), "").unwrap();
    assert_eq!(
        find(Some(path)),
        Some(Tool::Unrar(bin.path().join("unrar")))
    );
    fs::write(bin.path().join("7za"), "").unwrap();
    assert_eq!(
        find(Some(path)),
        Some(Tool::SevenZip(bin.path().join("7za")))
    );
    assert_eq!(find(None), None);
}

#[test]
fn each_tool_gets_the_password_never_overwrites_and_never_asks() {
    let archive = Path::new("/home/someone/film.rar");
    let into = Path::new("/cache/.extract-1-0");
    let seven = Tool::SevenZip("7z".into());
    let unrar = Tool::Unrar("unrar".into());
    assert_eq!(
        arguments(&seven, archive, into, None),
        [
            "x",
            "-aos",
            "-p",
            "-o/cache/.extract-1-0",
            "--",
            "/home/someone/film.rar"
        ]
    );
    assert_eq!(
        arguments(&seven, archive, into, Some("s3cret"))[2],
        "-ps3cret"
    );
    assert_eq!(
        arguments(&unrar, archive, into, None),
        [
            "x",
            "-o-",
            "-p-",
            "--",
            "/home/someone/film.rar",
            "/cache/.extract-1-0/"
        ]
    );
    assert_eq!(
        arguments(&unrar, archive, into, Some("s3cret"))[2],
        "-ps3cret"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn what_libarchive_reads_never_reaches_the_tool() {
    // No tool at all: libarchive reads it alone.
    let seen = read("bundle.cpio", None, None).unwrap();

    assert!(
        seen.iter()
            .any(|(name, content)| name == "notes.txt" && content == b"hello\n")
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn without_a_tool_an_archive_libarchive_cannot_read_says_to_install_7_zip() {
    let result = read("locked.7z", Some("secret"), None);

    assert!(
        matches!(&result, Err(ReadError::Damaged(message)) if message.contains("7-Zip")),
        "{result:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn as_administrator_what_libarchive_cannot_read_is_refused_not_given_to_a_tool() {
    let read_as_administrator = |name: &str| {
        let path = fixture(name);
        let mut seen = 0;
        ForeignArchives::as_administrator()
            .read_entries(
                &path,
                Box::new(fs::File::open(&path).unwrap()),
                Some("secret"),
                &mut |_, _| {
                    seen += 1;
                    Ok(ControlFlow::Continue(()))
                },
            )
            .map(|()| seen)
    };

    // libarchive reads the content the administrator's Fs opened…
    assert!(read_as_administrator("bundle.cpio").unwrap() > 0);
    // …but 7-Zip would open the path with the user's own rights.
    let refused = read_as_administrator("locked.7z");
    assert!(
        matches!(&refused, Err(ReadError::Damaged(message)) if message.contains("administrator")),
        "{refused:?}"
    );
}

/// Content that fails once read, as a failing disk would.
struct GoneDisk;

impl io::Read for GoneDisk {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the disk went away"))
    }
}

impl io::Seek for GoneDisk {
    fn seek(&mut self, _: io::SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_failing_source_is_an_io_error_and_never_reaches_the_tool() {
    let work = tempfile::tempdir().unwrap();
    // A tool that cannot start: reaching it would say so instead.
    let tool = Tool::SevenZip(work.path().join("no-such-7z"));

    let result = ForeignArchives::with_tool(Some(tool), Some(work.path().to_owned())).read_entries(
        &fixture("bundle.cpio"),
        Box::new(GoneDisk),
        None,
        &mut |_, _| Ok(ControlFlow::Continue(())),
    );

    assert!(
        matches!(&result, Err(ReadError::Io(error)) if error.to_string() == "the disk went away"),
        "{result:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn seven_zip_reads_what_libarchive_cannot_with_the_engines_password() {
    let Some(tool) = seven_zip() else { return };

    let none = read("locked.7z", None, Some(tool.clone()));
    let wrong = read("locked.7z", Some("guess"), Some(tool.clone()));
    let right = read("locked.7z", Some("secret"), Some(tool)).unwrap();

    assert!(matches!(none, Err(ReadError::NeedsPassword)), "{none:?}");
    assert!(matches!(wrong, Err(ReadError::WrongPassword)), "{wrong:?}");
    assert_eq!(right, [("notes.txt".to_owned(), b"hello\n".to_vec())]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn folders_come_before_what_they_hold() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("b/inner")).unwrap();
    fs::write(root.path().join("b/inner/z.txt"), "z").unwrap();
    fs::write(root.path().join("a.txt"), "a").unwrap();
    std::os::unix::fs::symlink("a.txt", root.path().join("c-link")).unwrap();
    let mut seen = Vec::new();

    let flow = walk(root.path(), Path::new(""), &mut |header, _| {
        seen.push((
            String::from_utf8_lossy(&header.name).into_owned(),
            header.kind.clone(),
        ));
        Ok(ControlFlow::Continue(()))
    })
    .unwrap();

    assert!(flow.is_continue());

    assert_eq!(
        seen,
        [
            ("a.txt".to_owned(), Kind::File),
            ("b".to_owned(), Kind::Directory),
            ("b/inner".to_owned(), Kind::Directory),
            ("b/inner/z.txt".to_owned(), Kind::File),
            ("c-link".to_owned(), Kind::Link(b"a.txt".to_vec())),
        ]
    );
}
