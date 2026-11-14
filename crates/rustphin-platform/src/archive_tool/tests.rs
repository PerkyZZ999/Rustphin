//! Choosing between libarchive and the installed tool, and reading
//! through a tool with the engine's password. Stand-in tools (shell
//! scripts) show how the tool's exit and words are understood; one test,
//! ignored unless asked for, runs the real 7-Zip (the `7zip` package).

use std::{os::unix::fs::PermissionsExt as _, path::PathBuf};

use super::*;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/archives")
        .join(name)
}

/// Every entry read from the fixture `name`, as (header, content), with the
/// given tool and a fresh work folder, which must be gone afterwards.
fn read_headers(
    name: &str,
    password: Option<&str>,
    tool: Option<Tool>,
) -> Result<Vec<(Header, Vec<u8>)>, ReadError> {
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
            seen.push((header.clone(), bytes));
            Ok(ControlFlow::Continue(()))
        },
    );
    let left: Vec<_> = fs::read_dir(work.path()).unwrap().collect();
    assert!(left.is_empty(), "work folders left behind: {left:?}");
    result.map(|()| seen)
}

/// As [`read_headers`], each entry as (name, content).
fn read(
    name: &str,
    password: Option<&str>,
    tool: Option<Tool>,
) -> Result<Vec<(String, Vec<u8>)>, ReadError> {
    read_headers(name, password, tool).map(|seen| {
        seen.into_iter()
            .map(|(header, content)| (String::from_utf8_lossy(&header.name).into_owned(), content))
            .collect()
    })
}

/// A stand-in tool: a shell script that runs `body` with `$out` set to the
/// folder it is to extract into, whichever tool's arguments it gets.
fn fake_tool(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("tool");
    let script = [
        "#!/bin/sh",
        r#"for arg; do last="$arg"; case "$arg" in -o*) out="${arg#-o}";; esac; done"#,
        r#"out="${out:-$last}""#,
        body,
        "",
    ]
    .join("\n");
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
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
    let bin = tempfile::tempdir().unwrap();
    let loud = fake_tool(bin.path(), "echo 'the tool was asked' >&2; exit 2");

    let seen = read("bundle.cpio", None, Some(Tool::SevenZip(loud))).unwrap();

    assert_eq!(
        seen,
        [
            ("notes.txt".to_owned(), b"hello\n".to_vec()),
            ("sub".to_owned(), Vec::new()),
            ("sub/deep.txt".to_owned(), b"deep\n".to_vec()),
            ("link".to_owned(), Vec::new()),
        ]
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
#[ignore = "needs 7-Zip installed; run with --run-ignored only"]
fn the_real_seven_zip_reads_what_libarchive_cannot_with_the_engines_password() {
    let tool = find(std::env::var_os("PATH").as_deref())
        .filter(|tool| matches!(tool, Tool::SevenZip(_)))
        .unwrap_or_else(|| panic!("7-Zip is not installed (the 7zip package)"));

    let none = read("locked.7z", None, Some(tool.clone()));
    let wrong = read("locked.7z", Some("guess"), Some(tool.clone()));
    let right = read("locked.7z", Some("secret"), Some(tool)).unwrap();

    assert!(matches!(none, Err(ReadError::NeedsPassword)), "{none:?}");
    assert!(matches!(wrong, Err(ReadError::WrongPassword)), "{wrong:?}");
    assert_eq!(right, [("notes.txt".to_owned(), b"hello\n".to_vec())]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn what_the_tool_extracts_is_handed_over_as_it_was_made() {
    let bin = tempfile::tempdir().unwrap();
    let tool = fake_tool(
        bin.path(),
        r#"mkdir "$out/folder" && printf 'hello\n' > "$out/folder/notes.txt" \
            && chmod 640 "$out/folder/notes.txt" && ln -s folder/notes.txt "$out/link" \
            && mkfifo "$out/pipe""#,
    );

    let seen = read_headers("locked.7z", Some("secret"), Some(Tool::SevenZip(tool))).unwrap();

    let summary: Vec<_> = seen
        .iter()
        .map(|(header, content)| {
            (
                String::from_utf8_lossy(&header.name).into_owned(),
                header.kind.clone(),
                header.size,
                content.clone(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("folder".to_owned(), Kind::Directory, 0, Vec::new()),
            (
                "folder/notes.txt".to_owned(),
                Kind::File,
                6,
                b"hello\n".to_vec()
            ),
            (
                "link".to_owned(),
                Kind::Link(b"folder/notes.txt".to_vec()),
                0,
                Vec::new()
            ),
            ("pipe".to_owned(), Kind::Special, 0, Vec::new()),
        ]
    );
    assert_eq!(seen[1].0.mode, Some(0o640));
}

#[test]
#[cfg_attr(miri, ignore)]
fn seven_zip_saying_password_asks_for_one_or_reports_it_wrong() {
    let bin = tempfile::tempdir().unwrap();
    let tool = Tool::SevenZip(fake_tool(
        bin.path(),
        "echo 'ERROR: Wrong password : notes.txt' >&2; exit 2",
    ));

    let none = read("locked.7z", None, Some(tool.clone()));
    let given = read("locked.7z", Some("guess"), Some(tool));

    assert!(matches!(none, Err(ReadError::NeedsPassword)), "{none:?}");
    assert!(matches!(given, Err(ReadError::WrongPassword)), "{given:?}");
}

#[test]
#[cfg_attr(miri, ignore)]
fn unrar_exiting_with_11_asks_for_a_password_or_reports_it_wrong() {
    let bin = tempfile::tempdir().unwrap();
    let tool = Tool::Unrar(fake_tool(bin.path(), "exit 11"));

    let none = read("locked.7z", None, Some(tool.clone()));
    let given = read("locked.7z", Some("guess"), Some(tool));

    assert!(matches!(none, Err(ReadError::NeedsPassword)), "{none:?}");
    assert!(matches!(given, Err(ReadError::WrongPassword)), "{given:?}");
}

#[test]
#[cfg_attr(miri, ignore)]
fn seven_zip_exiting_with_11_is_damage_not_a_password_question() {
    let bin = tempfile::tempdir().unwrap();
    let tool = Tool::SevenZip(fake_tool(bin.path(), "exit 11"));

    let result = read("locked.7z", None, Some(tool));

    assert!(
        matches!(&result, Err(ReadError::Damaged(message))
            if message == "7-Zip could not extract it (exit code 11)"),
        "{result:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_failed_extraction_is_damage_and_what_it_made_is_removed() {
    let bin = tempfile::tempdir().unwrap();
    // `read_headers` checks that the work folder is gone.
    let tool = Tool::Unrar(fake_tool(
        bin.path(),
        r#"printf 'half' > "$out/notes.txt"; echo 'notes.txt - CRC failed' >&2; exit 3"#,
    ));

    let result = read("locked.7z", None, Some(tool));

    assert!(
        matches!(&result, Err(ReadError::Damaged(message))
            if message == "unrar could not extract it (exit code 3)"),
        "{result:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_tool_that_cannot_start_is_an_io_error_naming_it() {
    let bin = tempfile::tempdir().unwrap();
    let tool = Tool::SevenZip(bin.path().join("missing-7z"));

    let result = read("locked.7z", None, Some(tool));

    assert!(
        matches!(&result, Err(ReadError::Io(error))
            if error.kind() == io::ErrorKind::NotFound
                && error.to_string().starts_with("7-Zip could not be started: ")),
        "{result:?}"
    );
}

#[test]
fn a_work_folder_is_new_private_and_leaves_one_already_there_alone() {
    let work = tempfile::tempdir().unwrap();
    let taken = work
        .path()
        .join(format!(".extract-{}-0", std::process::id()));
    fs::create_dir(&taken).unwrap();
    fs::write(taken.join("someone's"), "kept").unwrap();

    let folder = work_folder(work.path(), &AtomicU64::new(0)).unwrap();

    assert_eq!(
        folder.0,
        work.path()
            .join(format!(".extract-{}-1", std::process::id()))
    );
    assert_eq!(
        fs::metadata(&folder.0).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(fs::read_to_string(taken.join("someone's")).unwrap(), "kept");
}

#[test]
fn a_work_folder_in_a_folder_closed_to_writing_is_refused() {
    let work = tempfile::tempdir().unwrap();
    fs::set_permissions(work.path(), fs::Permissions::from_mode(0o500)).unwrap();
    if rustix::process::geteuid().is_root() {
        // Root writes there anyway; nothing to show.
        return;
    }

    let result = work_folder(work.path(), &AtomicU64::new(0));

    fs::set_permissions(work.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied)
    );
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
