//! Compress and extract through the engine (W-111): layouts, hostile
//! archives, conflicts, Stop, injected faults, passwords and Undo.

use std::{
    collections::VecDeque,
    fs,
    io::{Cursor, Write as _},
    os::unix::fs::{PermissionsExt as _, symlink},
    path::{Path, PathBuf},
};

use super::{
    faults::{Fault, FaultFs},
    tests::{FolderTrash, Item, dirs, file, names, read, tree},
    *,
};
use crate::{
    RealFs,
    archive::{ForeignReader, Format, Level, NoForeignReader, ReadError},
};

/// A controller for archive operations: scripted conflicts and passwords,
/// and Stop after a number of progress reports.
#[derive(Default)]
struct Archivist {
    conflicts: VecDeque<ConflictChoice>,
    passwords: VecDeque<Option<String>>,
    asked_passwords: Vec<bool>,
    cancel_after: Option<usize>,
    reports: usize,
    asked: Vec<Conflict>,
}

impl Controller for Archivist {
    fn progress(&mut self, _: &Progress) {
        self.reports += 1;
    }

    fn cancelled(&self) -> bool {
        self.cancel_after.is_some_and(|after| self.reports >= after)
    }

    fn conflict(&mut self, conflict: &Conflict) -> Resolution {
        self.asked.push(conflict.clone());
        Resolution {
            choice: self
                .conflicts
                .pop_front()
                .unwrap_or_else(|| panic!("unexpected conflict: {conflict:?}")),
            apply_to_all: false,
        }
    }

    fn failure(&mut self, failure: &Failure) -> Recovery {
        panic!("unexpected failure: {failure:?}")
    }

    fn password(&mut self, _: &Path, wrong: bool) -> Option<String> {
        self.asked_passwords.push(wrong);
        self.passwords.pop_front().flatten()
    }
}

/// A ZIP of `entries` (name, content; a name ending in `/` is a folder).
fn zip_of(entries: &[(&str, &str)], password: Option<&str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
        for (name, content) in entries {
            let mut options = zip::write::SimpleFileOptions::default();
            if let Some(password) = password {
                options = options.with_aes_encryption(zip::AesMode::Aes256, password);
            }
            if let Some(folder) = name.strip_suffix('/') {
                zip.add_directory(folder, options).unwrap();
            } else {
                zip.start_file(*name, options).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
            }
        }
        zip.finish().unwrap();
    }
    bytes
}

/// A TAR whose entry names are written as given, `..` and `/` included,
/// as a hostile archive would hold them.
fn raw_tar(entries: &[(&[u8], tar::EntryType, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, kind, data) in entries {
        let mut header = tar::Header::new_old();
        header.as_old_mut().name[..name.len()].copy_from_slice(name);
        header.set_entry_type(*kind);
        header.set_mode(0o644);
        if *kind == tar::EntryType::Symlink {
            header.as_old_mut().linkname[..data.len()].copy_from_slice(data);
            header.set_size(0);
            header.set_cksum();
            builder.append(&header, std::io::empty()).unwrap();
        } else {
            header.set_size(data.len() as u64);
            header.set_cksum();
            builder.append(&header, *data).unwrap();
        }
    }
    builder.into_inner().unwrap()
}

fn extract(archive: &Path, into: &Path, layout: Layout) -> Request {
    Request::Extract {
        archive: archive.to_owned(),
        into: into.to_owned(),
        layout,
        trash_archive: false,
    }
}

/// Whether anything Rustphin left behind is still in `dir`.
fn no_leftovers(dir: &Path) -> bool {
    !names(dir).iter().any(|name| name.contains(TEMP_PREFIX))
}

#[test]
fn extract_here_keeps_one_top_folder_and_gathers_several_in_a_new_one() {
    let d = dirs();
    fs::write(
        d.src.join("one.zip"),
        zip_of(&[("docs/", ""), ("docs/a.txt", "a")], None),
    )
    .unwrap();
    fs::write(
        d.src.join("many.zip"),
        zip_of(&[("a.txt", "a"), ("b.txt", "b")], None),
    )
    .unwrap();
    let mut archivist = Archivist::default();

    let one = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("one.zip"), &d.dst, Layout::Here),
        &mut archivist,
    )
    .unwrap();
    let many = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("many.zip"), &d.dst, Layout::Here),
        &mut archivist,
    )
    .unwrap();

    assert!(one.completed && many.completed);
    assert_eq!(
        names(&d.dst),
        ["docs", "docs/a.txt", "many", "many/a.txt", "many/b.txt"]
    );
    assert_eq!(read(d.dst.join("many/b.txt")), "b");
    // Each published entry is a change Undo can take back.
    assert!(matches!(&one.changes[..], [Change::Copied { to, .. }] if to == &d.dst.join("docs")));
}

#[test]
fn extract_into_a_new_folder_always_makes_one_named_after_the_archive() {
    let d = dirs();
    fs::write(d.src.join("pack.tar.gz"), {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o640);
        header.set_mtime(1_600_000_000);
        builder
            .append_data(&mut header, "solo.txt", &b"solo"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    })
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("pack.tar.gz"), &d.dst, Layout::NewFolder),
        &mut Archivist::default(),
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(names(&d.dst), ["pack", "pack/solo.txt"]);
    let meta = fs::metadata(d.dst.join("pack/solo.txt")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    assert_eq!(std::os::unix::fs::MetadataExt::mtime(&meta), 1_600_000_000);
}

#[test]
fn a_hostile_archive_writes_nothing_outside_and_says_what_it_left_out() {
    let d = dirs();
    let archive = d.src.join("evil.tar");
    fs::write(
        &archive,
        raw_tar(&[
            (b"../escaped.txt", tar::EntryType::Regular, b"x"),
            (b"/tmp/absolute.txt", tar::EntryType::Regular, b"x"),
            (b"box/fine.txt", tar::EntryType::Regular, b"fine"),
            (b"box/out", tar::EntryType::Symlink, b"../../outside"),
            (b"box/in", tar::EntryType::Symlink, b"fine.txt"),
            (b"box/dev", tar::EntryType::Char, b""),
        ]),
    )
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&archive, &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(names(&d.dst), ["box", "box/fine.txt", "box/in"]);
    assert_eq!(
        tree(&d.dst)[Path::new("box/in")],
        Item::Link(PathBuf::from("fine.txt"))
    );
    assert!(!d.dst.parent().unwrap().join("escaped.txt").exists());
    let reasons: Vec<&str> = outcome
        .skipped
        .iter()
        .map(|skip| skip.reason.as_str())
        .collect();
    assert_eq!(reasons.len(), 4, "{reasons:?}");
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("outside the folder"))
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("root of the disk"))
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("link points outside"))
    );
    assert!(reasons.iter().any(|reason| reason.contains("device")));
}

/// The links under `root` whose real target, once every link on the way is
/// followed, lies outside `root`.
fn links_resolving_outside(root: &Path) -> Vec<PathBuf> {
    let inside = fs::canonicalize(root).unwrap();
    tree(root)
        .into_iter()
        .filter(|(_, item)| matches!(item, Item::Link(_)))
        .map(|(relative, _)| relative)
        .filter(|relative| {
            fs::canonicalize(root.join(relative)).is_ok_and(|real| !real.starts_with(&inside))
        })
        .collect()
}

/// W-133: each link of a chain stays inside on its own, but `d/m` walks
/// through the link `d/l` and back up, two levels above the destination.
#[test]
fn a_link_that_climbs_back_through_another_link_is_left_out() {
    let d = dirs();
    let archive = d.src.join("chain.tar");
    fs::write(
        &archive,
        raw_tar(&[
            (b"d/l", tar::EntryType::Symlink, b".."),
            (b"d/m", tar::EntryType::Symlink, b"l/../.."),
        ]),
    )
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&archive, &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(links_resolving_outside(&d.dst), Vec::<PathBuf>::new());
    assert_eq!(names(&d.dst), ["d", "d/l"]);
    assert_eq!(outcome.skipped.len(), 1, "{:?}", outcome.skipped);
    assert_eq!(outcome.skipped[0].path, archive.join("d/m"));
    assert!(
        outcome.skipped[0].reason.contains("link points outside"),
        "{}",
        outcome.skipped[0].reason
    );
}

/// W-133: a link made after the link that walks through it closes the same
/// chain in the other order.
#[test]
fn a_link_is_left_out_whatever_the_order_of_the_chain() {
    let d = dirs();
    let archive = d.src.join("chain.tar");
    fs::write(
        &archive,
        raw_tar(&[
            (b"d/m", tar::EntryType::Symlink, b"x/../.."),
            (b"d/x", tar::EntryType::Symlink, b".."),
        ]),
    )
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&archive, &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    assert_eq!(links_resolving_outside(&d.dst), Vec::<PathBuf>::new());
    assert_eq!(names(&d.dst), ["d", "d/x"]);
    assert_eq!(outcome.skipped.len(), 1, "{:?}", outcome.skipped);
    assert_eq!(outcome.skipped[0].path, archive.join("d/m"));
}

/// W-133: ordinary relative links, up and down, still extract.
#[test]
fn relative_links_that_stay_inside_are_kept() {
    let d = dirs();
    let archive = d.src.join("links.tar");
    fs::write(
        &archive,
        raw_tar(&[
            (b"top/notes.txt", tar::EntryType::Regular, b"notes"),
            (b"top/link", tar::EntryType::Symlink, b"notes.txt"),
            (b"top/sub/up", tar::EntryType::Symlink, b"../notes.txt"),
            (b"top/sub/here", tar::EntryType::Symlink, b"./up"),
            (b"top/sub/via", tar::EntryType::Symlink, b"../sub/up"),
        ]),
    )
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&archive, &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    assert_eq!(outcome.skipped, []);
    for link in ["top/link", "top/sub/up", "top/sub/here", "top/sub/via"] {
        assert_eq!(read(d.dst.join(link)), "notes", "{link}");
    }
}

/// W-133: an entry beneath a link is never written through it.
#[test]
fn a_file_beneath_a_link_is_not_written_through_it() {
    let d = dirs();
    let archive = d.src.join("through.tar");
    fs::write(
        &archive,
        raw_tar(&[
            (b"box/up", tar::EntryType::Symlink, b"."),
            (b"box/up/x", tar::EntryType::Regular, b"x"),
        ]),
    )
    .unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&archive, &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    assert_eq!(names(&d.dst), ["box", "box/up"]);
    assert_eq!(outcome.skipped.len(), 1, "{:?}", outcome.skipped);
    assert_eq!(outcome.skipped[0].path, archive.join("box/up/x"));
    assert!(
        outcome.skipped[0].reason.contains("could not be written"),
        "{}",
        outcome.skipped[0].reason
    );
}

#[test]
fn a_taken_name_asks_and_keep_both_gets_a_numbered_folder() {
    let d = dirs();
    fs::write(
        d.src.join("many.zip"),
        zip_of(&[("a.txt", "new"), ("b.txt", "b")], None),
    )
    .unwrap();
    file(d.dst.join("many/a.txt"), "old");
    let mut archivist = Archivist {
        conflicts: [ConflictChoice::Overwrite, ConflictChoice::KeepBoth].into(),
        ..Archivist::default()
    };

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("many.zip"), &d.dst, Layout::Here),
        &mut archivist,
    )
    .unwrap();

    assert!(outcome.completed);
    // A folder cannot replace a folder here: asked again, then kept both.
    assert_eq!(archivist.asked.len(), 2);
    assert!(!archivist.asked[0].can_overwrite);
    assert_eq!(read(d.dst.join("many/a.txt")), "old");
    assert_eq!(read(d.dst.join("many (1)/a.txt")), "new");
    assert!(no_leftovers(&d.dst));
}

#[test]
fn skip_and_cancel_at_a_conflict_leave_only_what_was_there() {
    for choice in [ConflictChoice::Skip, ConflictChoice::Cancel] {
        let d = dirs();
        fs::write(d.src.join("a.zip"), zip_of(&[("a.txt", "new")], None)).unwrap();
        file(d.dst.join("a.txt"), "old");
        let mut archivist = Archivist {
            conflicts: [choice].into(),
            ..Archivist::default()
        };
        let outcome = run(
            &RealFs,
            &NoTrash,
            &NoForeignReader,
            &extract(&d.src.join("a.zip"), &d.dst, Layout::Here),
            &mut archivist,
        )
        .unwrap();
        assert_eq!(outcome.completed, choice == ConflictChoice::Skip);
        assert_eq!(names(&d.dst), ["a.txt"]);
        assert_eq!(read(d.dst.join("a.txt")), "old");
        assert!(outcome.changes.is_empty());
    }
}

#[test]
fn stopping_part_way_leaves_nothing_half_extracted() {
    let d = dirs();
    let big = "x".repeat(400_000);
    fs::write(
        d.src.join("big.zip"),
        zip_of(&[("a.txt", &big), ("b.txt", &big)], None),
    )
    .unwrap();
    let mut archivist = Archivist {
        cancel_after: Some(4),
        ..Archivist::default()
    };

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("big.zip"), &d.dst, Layout::Here),
        &mut archivist,
    )
    .unwrap();

    assert!(!outcome.completed);
    assert!(names(&d.dst).is_empty(), "{:?}", names(&d.dst));
    assert!(outcome.changes.is_empty());
}

#[test]
fn a_fault_at_any_step_leaves_no_partial_result_behind() {
    let template = dirs();
    fs::write(
        template.src.join("t.zip"),
        zip_of(
            &[
                ("d/", ""),
                ("d/a.txt", "a"),
                ("d/e/b.txt", "b"),
                ("c.txt", "c"),
            ],
            None,
        ),
    )
    .unwrap();
    let mut step = 0;
    loop {
        let d = dirs();
        fs::copy(template.src.join("t.zip"), d.src.join("t.zip")).unwrap();
        let fs = FaultFs::failing_at(step, Fault::Error);
        let result = run(
            &fs,
            &NoTrash,
            &NoForeignReader,
            &extract(&d.src.join("t.zip"), &d.dst, Layout::Here),
            &mut Archivist::default(),
        );
        assert!(no_leftovers(&d.dst), "step {step}: {:?}", names(&d.dst));
        let listed = names(&d.dst);
        match &result {
            Ok(outcome) if outcome.completed && outcome.skipped.is_empty() => {
                assert_eq!(
                    listed,
                    ["t", "t/c.txt", "t/d", "t/d/a.txt", "t/d/e", "t/d/e/b.txt"],
                    "step {step}"
                );
            }
            // Either nothing, or the whole result with an entry left out.
            _ => assert!(
                listed.is_empty() || listed[0] == "t",
                "step {step}: {listed:?}"
            ),
        }
        if !fs.hit() {
            break;
        }
        step += 1;
    }
    assert!(step > 5, "the extraction took only {step} steps");
}

#[test]
fn an_encrypted_archive_asks_for_the_password_and_asks_again_when_wrong() {
    let d = dirs();
    fs::write(
        d.src.join("s.zip"),
        zip_of(&[("secret.txt", "hidden")], Some("right horse")),
    )
    .unwrap();
    let mut archivist = Archivist {
        passwords: [Some("wrong".to_owned()), Some("right horse".to_owned())].into(),
        ..Archivist::default()
    };

    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("s.zip"), &d.dst, Layout::Here),
        &mut archivist,
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(archivist.asked_passwords, [false, true]);
    assert_eq!(read(d.dst.join("secret.txt")), "hidden");
}

#[test]
fn no_password_stops_and_three_wrong_ones_refuse() {
    let d = dirs();
    fs::write(
        d.src.join("s.zip"),
        zip_of(&[("secret.txt", "hidden")], Some("right horse")),
    )
    .unwrap();
    let request = extract(&d.src.join("s.zip"), &d.dst, Layout::Here);

    let declined = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &request,
        &mut Archivist::default(),
    )
    .unwrap();
    assert!(!declined.completed);

    let mut guesses = Archivist {
        passwords: (0..3).map(|_| Some("nope".to_owned())).collect(),
        ..Archivist::default()
    };
    let refused = run(&RealFs, &NoTrash, &NoForeignReader, &request, &mut guesses);
    assert!(matches!(refused, Err(OpError::WrongPassword(_))));
    assert!(names(&d.dst).is_empty());
}

#[test]
fn an_archive_larger_than_the_free_space_is_refused_before_writing() {
    let d = dirs();
    fs::write(
        d.src.join("a.zip"),
        zip_of(&[("a.txt", &"x".repeat(5_000))], None),
    )
    .unwrap();
    let fs = FaultFs::new().with_free_space(1_000);

    let result = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("a.zip"), &d.dst, Layout::Here),
        &mut Archivist::default(),
    );

    assert!(matches!(
        result,
        Err(OpError::NotEnoughSpace {
            needed: 5_000,
            free: 1_000,
            ..
        })
    ));
    assert!(names(&d.dst).is_empty());
}

#[test]
fn damaged_and_unknown_files_are_refused_with_nothing_left() {
    let d = dirs();
    let mut damaged = zip_of(&[("a.txt", &"y".repeat(2_000))], None);
    damaged.truncate(damaged.len() / 2);
    fs::write(d.src.join("broken.zip"), damaged).unwrap();
    fs::write(d.src.join("notes.txt"), "just text").unwrap();

    let broken = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("broken.zip"), &d.dst, Layout::Here),
        &mut Archivist::default(),
    );
    let unknown = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("notes.txt"), &d.dst, Layout::Here),
        &mut Archivist::default(),
    );

    assert!(matches!(broken, Err(OpError::Damaged { .. })));
    assert!(matches!(unknown, Err(OpError::NotAnArchive(_))));
    assert!(names(&d.dst).is_empty());
}

#[test]
fn extract_and_trash_trashes_the_archive_and_undo_brings_it_back() {
    let d = dirs();
    let archive = d.src.join("a.zip");
    fs::write(&archive, zip_of(&[("a.txt", "a"), ("b.txt", "b")], None)).unwrap();
    let trash_dir = d.dst.parent().unwrap().join("trash");
    fs::create_dir(&trash_dir).unwrap();
    let trash = FolderTrash::new(&trash_dir);
    let request = Request::Extract {
        archive: archive.clone(),
        into: d.dst.clone(),
        layout: Layout::Here,
        trash_archive: true,
    };

    let outcome = run(
        &RealFs,
        &trash,
        &NoForeignReader,
        &request,
        &mut Archivist::default(),
    )
    .unwrap();
    assert!(!archive.exists());
    assert_eq!(names(&d.dst), ["a", "a/a.txt", "a/b.txt"]);

    undo(&RealFs, &trash, &outcome.changes, &mut Archivist::default()).unwrap();
    assert!(archive.exists());
    assert!(names(&d.dst).is_empty());
}

#[test]
fn compressing_makes_an_archive_that_extracts_to_the_same_files() {
    for format in [
        Format::Zip,
        Format::TarGz,
        Format::TarXz,
        Format::TarZst,
        Format::SevenZ,
    ] {
        let d = dirs();
        file(d.src.join("photos/a.jpg"), "picture");
        file(d.src.join("photos/deep/b.jpg"), "deeper");
        file(d.src.join("notes.txt"), "notes");
        symlink("notes.txt", d.src.join("link")).unwrap();
        let archive = d.dst.join(format!("pack.{}", format.extension()));
        let mut sources = vec![d.src.join("photos"), d.src.join("notes.txt")];
        if format != Format::SevenZ {
            sources.push(d.src.join("link"));
        }

        let made = run(
            &RealFs,
            &NoTrash,
            &NoForeignReader,
            &Request::Compress {
                sources,
                archive: archive.clone(),
                format,
                level: Level::Fast,
            },
            &mut Archivist::default(),
        )
        .unwrap();
        assert!(made.completed, "{format:?}");
        assert!(made.skipped.is_empty(), "{format:?}: {:?}", made.skipped);
        assert!(no_leftovers(&d.dst));

        let out = d.dst.join("out");
        fs::create_dir(&out).unwrap();
        run(
            &RealFs,
            &NoTrash,
            &NoForeignReader,
            &extract(&archive, &out, Layout::Here),
            &mut Archivist::default(),
        )
        .unwrap();
        let unpacked = tree(&out.join("pack"));
        assert_eq!(
            unpacked[Path::new("photos/deep/b.jpg")],
            Item::File(b"deeper".to_vec()),
            "{format:?}"
        );
        assert_eq!(
            unpacked[Path::new("notes.txt")],
            Item::File(b"notes".to_vec()),
            "{format:?}"
        );
        if format != Format::SevenZ {
            assert_eq!(
                unpacked[Path::new("link")],
                Item::Link(PathBuf::from("notes.txt")),
                "{format:?}"
            );
        }
    }
}

#[test]
fn compressing_never_replaces_and_a_stop_leaves_nothing() {
    let d = dirs();
    file(d.src.join("big.bin"), &"z".repeat(2_000_000));
    let archive = d.dst.join("big.zip");
    let request = Request::Compress {
        sources: vec![d.src.join("big.bin")],
        archive: archive.clone(),
        format: Format::Zip,
        level: Level::Fast,
    };

    let mut stopping = Archivist {
        cancel_after: Some(3),
        ..Archivist::default()
    };
    let stopped = run(&RealFs, &NoTrash, &NoForeignReader, &request, &mut stopping).unwrap();
    assert!(!stopped.completed);
    assert!(names(&d.dst).is_empty());

    file(&archive, "already here");
    let refused = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &request,
        &mut Archivist::default(),
    );
    assert!(matches!(refused, Err(OpError::AlreadyExists(_))));
    assert_eq!(read(&archive), "already here");
}

#[test]
fn undoing_a_compression_trashes_the_archive() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    let trash_dir = d.dst.parent().unwrap().join("trash");
    fs::create_dir(&trash_dir).unwrap();
    let trash = FolderTrash::new(&trash_dir);
    let archive = d.dst.join("a.zip");
    let made = run(
        &RealFs,
        &trash,
        &NoForeignReader,
        &Request::Compress {
            sources: vec![d.src.join("a.txt")],
            archive: archive.clone(),
            format: Format::Zip,
            level: Level::Normal,
        },
        &mut Archivist::default(),
    )
    .unwrap();

    undo(&RealFs, &trash, &made.changes, &mut Archivist::default()).unwrap();
    assert!(!archive.exists());
    assert_eq!(read(d.src.join("a.txt")), "a");
}

#[test]
fn the_archive_stays_when_anything_was_left_out_or_cancelled() {
    let d = dirs();
    let trash_dir = d.dst.parent().unwrap().join("trash");
    fs::create_dir(&trash_dir).unwrap();
    let trash = FolderTrash::new(&trash_dir);
    let hostile = d.src.join("evil.tar");
    fs::write(
        &hostile,
        raw_tar(&[
            (b"fine.txt", tar::EntryType::Regular, b"fine"),
            (b"../escaped.txt", tar::EntryType::Regular, b"x"),
        ]),
    )
    .unwrap();
    let trashing = |archive: &Path| Request::Extract {
        archive: archive.to_owned(),
        into: d.dst.clone(),
        layout: Layout::Here,
        trash_archive: true,
    };

    let partial = run(
        &RealFs,
        &trash,
        &NoForeignReader,
        &trashing(&hostile),
        &mut Archivist::default(),
    )
    .unwrap();
    assert!(partial.completed && !partial.skipped.is_empty());
    assert!(hostile.exists(), "an archive with entries left out stays");

    let plain = d.src.join("a.zip");
    fs::write(&plain, zip_of(&[("fine.txt", "new")], None)).unwrap();
    let mut cancelling = Archivist {
        conflicts: [ConflictChoice::Cancel].into(),
        ..Archivist::default()
    };
    let cancelled = run(
        &RealFs,
        &trash,
        &NoForeignReader,
        &trashing(&plain),
        &mut cancelling,
    )
    .unwrap();
    assert!(!cancelled.completed);
    assert!(plain.exists(), "a cancelled extraction keeps its archive");
}

#[test]
fn folders_the_archive_only_implies_get_the_usual_rights() {
    let d = dirs();
    fs::write(
        d.src.join("deep.zip"),
        zip_of(&[("top/inner/a.txt", "a")], None),
    )
    .unwrap();

    run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("deep.zip"), &d.dst, Layout::Here),
        &mut Archivist::default(),
    )
    .unwrap();

    for folder in ["top", "top/inner"] {
        let mode = fs::metadata(d.dst.join(folder))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode & 0o700, 0o700, "{folder}: {mode:o}");
        assert_ne!(mode & 0o055, 0, "{folder} is not private: {mode:o}");
    }
}

/// Stands in for libarchive (D-036): a folder, a file inside it, a file at
/// the top and a hostile `../` name. The top file holds what the reader
/// read from the source it was given. "locked.iso" needs the password
/// "secret".
struct FakeForeign;

impl ForeignReader for FakeForeign {
    fn read_entries(
        &self,
        path: &Path,
        mut source: Box<dyn crate::fs::ReadSeek>,
        password: Option<&str>,
        visit: crate::archive::Visit<'_>,
    ) -> Result<(), ReadError> {
        use crate::archive::{Header, Kind};
        if path.file_name().unwrap().as_bytes() == b"locked.iso" {
            match password {
                None => return Err(ReadError::NeedsPassword),
                Some("secret") => {}
                Some(_) => return Err(ReadError::WrongPassword),
            }
        }
        let mut read = Vec::new();
        source.read_to_end(&mut read)?;
        let entries: [(&[u8], Kind, &[u8]); 4] = [
            (b"sub/", Kind::Directory, b""),
            (b"sub/deep.txt", Kind::File, b"deep"),
            (b"notes.txt", Kind::File, &read),
            (b"../escape.txt", Kind::File, b"out"),
        ];
        for (entry, kind, content) in entries {
            let header = Header {
                name: entry.to_vec(),
                kind,
                mode: Some(0o644),
                modified: None,
                size: content.len() as u64,
            };
            if visit(&header, &mut &content[..])?.is_break() {
                break;
            }
        }
        Ok(())
    }
}

#[test]
fn formats_rustphin_does_not_read_itself_go_through_the_foreign_reader() {
    let d = dirs();
    fs::write(d.src.join("disc.iso"), b"not a format Rustphin knows").unwrap();

    let outcome = run(
        &RealFs,
        &NoTrash,
        &FakeForeign,
        &extract(&d.src.join("disc.iso"), &d.dst, Layout::NewFolder),
        &mut Archivist::default(),
    )
    .unwrap();

    assert!(outcome.completed);
    // The same checks as for Rustphin's own formats: nothing outside.
    assert_eq!(
        names(&d.dst),
        ["disc", "disc/notes.txt", "disc/sub", "disc/sub/deep.txt"]
    );
    // The reader got the archive's content from its start.
    assert_eq!(
        read(d.dst.join("disc/notes.txt")),
        "not a format Rustphin knows"
    );
    assert!(!d.dst.parent().unwrap().join("escape.txt").exists());
    assert!(no_leftovers(&d.dst));
}

#[test]
fn without_a_foreign_reader_an_unknown_format_is_not_an_archive() {
    let d = dirs();
    fs::write(d.src.join("disc.iso"), b"not a format Rustphin knows").unwrap();

    let result = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &extract(&d.src.join("disc.iso"), &d.dst, Layout::NewFolder),
        &mut Archivist::default(),
    );

    assert!(
        matches!(result, Err(OpError::NotAnArchive(_))),
        "{result:?}"
    );
    assert!(names(&d.dst).is_empty());
}

#[test]
fn the_foreign_reader_reads_through_the_engines_fs() {
    let d = dirs();
    // Longer than the 512 bytes read to judge the format, so the fault is
    // met by the reader, not by that first look.
    fs::write(d.src.join("disc.iso"), vec![b'x'; 4096]).unwrap();
    let fs = FaultFs::new().failing_reads_after(1024);

    let result = run(
        &fs,
        &NoTrash,
        &FakeForeign,
        &extract(&d.src.join("disc.iso"), &d.dst, Layout::NewFolder),
        &mut Archivist::default(),
    );

    assert!(
        matches!(&result, Err(OpError::Io { source, .. }) if source.to_string() == "injected read fault"),
        "{result:?}"
    );
    assert!(names(&d.dst).is_empty());
}

#[test]
fn the_foreign_reader_gets_the_password_the_user_gives() {
    let d = dirs();
    fs::write(d.src.join("locked.iso"), b"encrypted").unwrap();
    let mut archivist = Archivist {
        passwords: [Some("guess".to_owned()), Some("secret".to_owned())].into(),
        ..Archivist::default()
    };

    let outcome = run(
        &RealFs,
        &NoTrash,
        &FakeForeign,
        &extract(&d.src.join("locked.iso"), &d.dst, Layout::NewFolder),
        &mut archivist,
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(archivist.asked_passwords, [false, true]);
    assert_eq!(read(d.dst.join("locked/sub/deep.txt")), "deep");
}
