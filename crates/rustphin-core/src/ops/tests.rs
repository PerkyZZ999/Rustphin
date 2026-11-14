use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs, io,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, PermissionsExt, symlink},
    },
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use super::{
    faults::{Fault, FaultFs},
    *,
};
use crate::{RealFs, archive::NoForeignReader};

/// What one entry of a directory tree is, for comparing trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Item {
    Dir,
    File(Vec<u8>),
    Link(PathBuf),
}

/// Everything under `root`, by path relative to it.
pub(super) fn tree(root: &Path) -> BTreeMap<PathBuf, Item> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Item>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                out.insert(relative, Item::Link(fs::read_link(&path).unwrap()));
            } else if kind.is_dir() {
                out.insert(relative, Item::Dir);
                walk(root, &path, out);
            } else {
                out.insert(relative, Item::File(fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// The paths in `tree`, as strings, for short assertions.
pub(super) fn names(root: &Path) -> Vec<String> {
    tree(root)
        .into_keys()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

pub(super) fn file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

pub(super) fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap()
}

/// The kind of the I/O error an operation failed with, if that is how it
/// failed.
pub(super) fn io_kind(error: &OpError) -> Option<io::ErrorKind> {
    match error {
        OpError::Io { source, .. } => Some(source.kind()),
        _ => None,
    }
}

/// A controller that follows a script and fails the test on anything unscripted.
#[derive(Default)]
pub(super) struct Script {
    pub(super) conflicts: VecDeque<Resolution>,
    failures: VecDeque<Recovery>,
    /// Cancel once this many progress reports were seen. The first report
    /// comes when the plan is ready, before anything is changed.
    cancel_after: Option<usize>,
    pub(super) asked: Vec<Conflict>,
    pub(super) failed: Vec<PathBuf>,
    pub(super) reports: Vec<Progress>,
}

impl Script {
    pub(super) fn conflicts(choices: &[(ConflictChoice, bool)]) -> Self {
        Self {
            conflicts: choices
                .iter()
                .map(|&(choice, apply_to_all)| Resolution {
                    choice,
                    apply_to_all,
                })
                .collect(),
            ..Self::default()
        }
    }

    pub(super) fn failures(recoveries: &[Recovery]) -> Self {
        Self {
            failures: recoveries.iter().copied().collect(),
            ..Self::default()
        }
    }
}

impl Controller for Script {
    fn progress(&mut self, progress: &Progress) {
        self.reports.push(progress.clone());
    }

    fn cancelled(&self) -> bool {
        self.cancel_after
            .is_some_and(|after| self.reports.len() >= after)
    }

    fn conflict(&mut self, conflict: &Conflict) -> Resolution {
        self.asked.push(conflict.clone());
        self.conflicts
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected conflict: {conflict:?}"))
    }

    fn failure(&mut self, failure: &Failure) -> Recovery {
        self.failed.push(failure.path.clone());
        self.failures
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected failure: {failure:?}"))
    }
}

/// A temporary directory with `src` and `dst` folders in it.
pub(super) struct Dirs {
    _temp: TempDir,
    pub(super) src: PathBuf,
    pub(super) dst: PathBuf,
}

pub(super) fn dirs() -> Dirs {
    let temp = tempfile::tempdir().unwrap();
    let (src, dst) = (temp.path().join("src"), temp.path().join("dst"));
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();
    Dirs {
        _temp: temp,
        src,
        dst,
    }
}

pub(super) fn copy(sources: &[PathBuf], into: &Path) -> Request {
    Request::Copy {
        sources: sources.to_vec(),
        into: into.to_owned(),
    }
}

pub(super) fn move_(sources: &[PathBuf], into: &Path) -> Request {
    Request::Move {
        sources: sources.to_vec(),
        into: into.to_owned(),
    }
}

fn go(request: &Request, script: &mut Script) -> Outcome {
    run(&RealFs, &NoTrash, &NoForeignReader, request, script).unwrap()
}

#[test]
fn creates_an_empty_folder_and_an_empty_file() {
    let d = dirs();
    let outcome = go(
        &Request::CreateDir {
            parent: d.src.clone(),
            name: "New Folder".into(),
        },
        &mut Script::default(),
    );
    go(
        &Request::CreateFile {
            parent: d.src.clone(),
            name: "notes.txt".into(),
        },
        &mut Script::default(),
    );

    assert_eq!(
        tree(&d.src),
        BTreeMap::from([
            ("New Folder".into(), Item::Dir),
            ("notes.txt".into(), Item::File(Vec::new())),
        ])
    );
    assert!(
        matches!(&outcome.changes[..], [Change::Created { path, .. }] if *path == d.src.join("New Folder"))
    );
}

#[test]
fn creating_over_an_existing_entry_is_refused_and_leaves_it_alone() {
    let d = dirs();
    file(d.src.join("taken"), "precious");

    let error = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &Request::CreateFile {
            parent: d.src.clone(),
            name: "taken".into(),
        },
        &mut Script::default(),
    )
    .unwrap_err();

    assert!(matches!(error, OpError::AlreadyExists(path) if path == d.src.join("taken")));
    assert_eq!(read(d.src.join("taken")), "precious");
}

fn from_template(parent: &Path, name: &str, template: Template) -> Request {
    Request::CreateFromTemplate {
        parent: parent.to_owned(),
        name: name.into(),
        template,
    }
}

#[test]
fn a_file_is_created_from_built_in_contents_or_a_template_file() {
    let d = dirs();
    let html = b"<!DOCTYPE html>\n".to_vec();
    let outcome = go(
        &from_template(&d.src, "page.html", Template::Contents(html)),
        &mut Script::default(),
    );
    let template = d.dst.join("Letter.odt");
    file(&template, "letter body");
    go(
        &from_template(&d.src, "Letter.odt", Template::File(template.clone())),
        &mut Script::default(),
    );

    assert_eq!(read(d.src.join("page.html")), "<!DOCTYPE html>\n");
    assert_eq!(read(d.src.join("Letter.odt")), "letter body");
    assert_eq!(read(&template), "letter body");
    // Nothing temporary is left beside them.
    assert_eq!(names(&d.src), ["Letter.odt", "page.html"]);
    assert!(
        matches!(&outcome.changes[..], [Change::Created { path, .. }] if *path == d.src.join("page.html"))
    );
}

#[test]
fn a_template_never_replaces_an_existing_entry() {
    let d = dirs();
    file(d.src.join("taken.txt"), "precious");

    let error = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &from_template(&d.src, "taken.txt", Template::Contents(b"new".to_vec())),
        &mut Script::default(),
    )
    .unwrap_err();

    assert!(matches!(error, OpError::AlreadyExists(path) if path == d.src.join("taken.txt")));
    assert_eq!(read(d.src.join("taken.txt")), "precious");
    assert_eq!(names(&d.src), ["taken.txt"]);
}

/// Makes "new" in `d.src` from `template` with `fault` at the first step.
fn make_with_fault(d: &Dirs, template: Template, fault: Fault) -> OpError {
    run(
        &FaultFs::failing_at(0, fault),
        &NoTrash,
        &NoForeignReader,
        &from_template(&d.src, "new", template),
        &mut Script::default(),
    )
    .unwrap_err()
}

#[test]
fn a_template_that_fails_to_be_copied_leaves_nothing_behind() {
    let d = dirs();
    file(d.dst.join("big.odt"), "template body");

    let error = make_with_fault(&d, Template::File(d.dst.join("big.odt")), Fault::Error);

    assert!(error.to_string().contains("injected fault"), "{error:?}");
    assert!(names(&d.src).is_empty(), "{:?}", names(&d.src));
}

#[test]
fn a_template_copy_that_reports_success_but_came_out_short_is_refused() {
    let d = dirs();
    file(d.dst.join("big.odt"), "template body");

    let error = make_with_fault(&d, Template::File(d.dst.join("big.odt")), Fault::ShortCopy);

    assert!(
        error.to_string().contains("changed while it was copied"),
        "{error:?}"
    );
    assert!(names(&d.src).is_empty(), "{:?}", names(&d.src));
}

#[test]
fn contents_that_fail_to_be_written_leave_nothing_behind() {
    let d = dirs();

    let error = make_with_fault(&d, Template::Contents(b"x".to_vec()), Fault::Error);

    assert!(error.to_string().contains("injected fault"), "{error:?}");
    assert!(names(&d.src).is_empty(), "{:?}", names(&d.src));
}

#[test]
fn undoing_a_file_made_from_a_template_removes_it() {
    let d = dirs();
    let outcome = go(
        &from_template(&d.src, "notes.txt", Template::Contents(b"hello".to_vec())),
        &mut Script::default(),
    );
    undo(&RealFs, &NoTrash, &outcome.changes, &mut Script::default()).unwrap();
    assert!(names(&d.src).is_empty());
}

#[test]
fn names_that_cannot_name_one_entry_are_refused() {
    let too_long = "x".repeat(256);
    for name in ["", ".", "..", "a/b", "nul\0l", too_long.as_str()] {
        assert!(
            matches!(
                validate_name(name.as_ref()),
                Err(OpError::InvalidName { .. })
            ),
            "{name:?} was accepted"
        );
    }
    for name in ["a", ".hidden", "with space.txt", "..."] {
        assert!(validate_name(name.as_ref()).is_ok(), "{name:?} was refused");
    }
}

#[test]
fn renames_within_the_folder() {
    let d = dirs();
    file(d.src.join("old.txt"), "content");

    let outcome = go(
        &Request::Rename {
            path: d.src.join("old.txt"),
            new_name: "new.txt".into(),
        },
        &mut Script::default(),
    );

    assert_eq!(names(&d.src), ["new.txt"]);
    assert_eq!(read(d.src.join("new.txt")), "content");
    assert!(matches!(
        &outcome.changes[..],
        [Change::Moved { from, to, .. }] if *from == d.src.join("old.txt") && *to == d.src.join("new.txt")
    ));
}

#[test]
fn renaming_onto_an_existing_name_is_refused_and_both_survive() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");

    let error = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &Request::Rename {
            path: d.src.join("a"),
            new_name: "b".into(),
        },
        &mut Script::default(),
    )
    .unwrap_err();

    assert!(matches!(error, OpError::AlreadyExists(_)));
    assert_eq!(
        (read(d.src.join("a")), read(d.src.join("b"))),
        ("A".into(), "B".into())
    );
}

#[test]
fn copies_a_tree_with_links_and_names_that_are_not_utf8() {
    let d = dirs();
    let odd = OsString::from_vec(b"bad\xff name".to_vec());
    file(d.src.join("top/inner/deep.txt"), "deep");
    file(d.src.join("top").join(&odd), "odd");
    fs::create_dir(d.src.join("top/empty")).unwrap();
    symlink("inner/deep.txt", d.src.join("top/link")).unwrap();
    symlink("/nowhere/at/all", d.src.join("top/dangling")).unwrap();
    let before = tree(&d.src);

    let mut script = Script::default();
    let outcome = go(&copy(&[d.src.join("top")], &d.dst), &mut script);

    assert!(outcome.completed);
    assert_eq!(tree(&d.dst), before);
    assert_eq!(tree(&d.src), before);
    assert_eq!(
        tree(&d.dst)[Path::new("top/link")],
        Item::Link("inner/deep.txt".into())
    );
    assert!(matches!(
        &outcome.changes[..],
        [Change::Copied { to, .. }] if *to == d.dst.join("top")
    ));
    let last = script.reports.last().unwrap();
    assert_eq!((last.items_done, last.items_total), (7, 7));
    assert_eq!((last.bytes_done, last.bytes_total), (7, 7));
}

#[test]
fn a_copy_keeps_permissions_and_modification_time() {
    let d = dirs();
    let source = d.src.join("script.sh");
    file(&source, "#!/bin/sh");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o750)).unwrap();
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::new(1_500_000_000, 123_000);
    fs::File::open(&source).unwrap().set_modified(old).unwrap();

    go(&copy(&[source], &d.dst), &mut Script::default());

    let copied = fs::metadata(d.dst.join("script.sh")).unwrap();
    assert_eq!(copied.mode() & 0o7777, 0o750);
    assert_eq!(copied.modified().unwrap(), old);
}

#[test]
fn a_failed_sync_never_publishes_a_copy() {
    let d = dirs();
    let source = d.src.join("a.txt");
    file(&source, "whole source");
    let fs = FaultFs::failing_sync();
    let mut script = Script::failures(&[Recovery::Skip]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &copy(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert!(names(&d.dst).is_empty());
    assert_eq!(read(&source), "whole source");
    assert_eq!(script.failed, [source]);
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.changes.is_empty());
    assert!(fs.hit());
}

#[test]
fn a_failed_sync_keeps_the_destination_that_would_be_replaced() {
    let d = dirs();
    let source = d.src.join("a.txt");
    file(&source, "new content");
    file(d.dst.join("a.txt"), "old content");
    let fs = FaultFs::failing_sync();
    let mut script = Script::conflicts(&[(ConflictChoice::Overwrite, false)]);
    script.failures.push_back(Recovery::Skip);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &copy(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(read(d.dst.join("a.txt")), "old content");
    assert_eq!(names(&d.dst), ["a.txt"]);
    assert_eq!(read(&source), "new content");
    assert_eq!(script.failed, [source]);
    assert!(outcome.changes.is_empty());
    assert!(fs.hit());
}

#[test]
fn a_failed_sync_keeps_a_cross_filesystem_moves_source() {
    let d = dirs();
    let source = d.src.join("a.txt");
    file(&source, "whole source");
    let fs = FaultFs::failing_sync().with_device(&d.dst);
    let mut script = Script::failures(&[Recovery::Skip]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(read(&source), "whole source");
    assert!(names(&d.dst).is_empty());
    assert_eq!(script.failed, [source]);
    assert!(outcome.changes.is_empty());
    assert!(fs.hit());
}

#[test]
fn retrying_a_failed_sync_copies_the_whole_file() {
    let d = dirs();
    let source = d.src.join("a.txt");
    file(&source, "whole source");
    let fs = FaultFs::failing_sync();
    let mut script = Script::failures(&[Recovery::Retry]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &copy(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(read(&source), "whole source");
    assert_eq!(script.failed, [source]);
    assert_eq!(names(&d.dst), ["a.txt"]);
    assert_eq!(read(d.dst.join("a.txt")), "whole source");
    assert!(outcome.skipped.is_empty());
    assert_eq!(outcome.changes.len(), 1);
    assert!(outcome.completed);
    let last = script.reports.last().unwrap();
    assert_eq!((last.bytes_done, last.bytes_total), (12, 12));
    assert!(fs.hit());
}

#[test]
fn a_copy_into_its_own_folder_gets_a_numbered_name_without_asking() {
    let d = dirs();
    file(d.src.join("report.tar.gz"), "data");
    file(d.src.join("report.tar (1).gz"), "taken");

    go(
        &copy(&[d.src.join("report.tar.gz")], &d.src),
        &mut Script::default(),
    );

    assert_eq!(
        names(&d.src),
        ["report.tar (1).gz", "report.tar (2).gz", "report.tar.gz"]
    );
    assert_eq!(read(d.src.join("report.tar (2).gz")), "data");
    assert_eq!(read(d.src.join("report.tar (1).gz")), "taken");
}

#[test]
fn a_published_move_copy_keeps_its_source_when_the_destination_directory_cannot_sync() {
    let d = dirs();
    let source = d.src.join("a");
    file(&source, "whole file");
    let fs = FaultFs::failing_directory_sync(0).with_device(&d.dst);
    let mut script = Script::failures(&[Recovery::Skip]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(read(&source), "whole file");
    assert_eq!(read(d.dst.join("a")), "whole file");
    assert_eq!(outcome.skipped.len(), 1);
    assert!(matches!(&outcome.changes[..], [Change::Copied { .. }]));
    assert!(fs.hit());
}

#[test]
fn retrying_a_published_move_sync_does_not_copy_or_replace_again() {
    let d = dirs();
    let source = d.src.join("a");
    file(&source, "whole file");
    let fs = FaultFs::failing_directory_sync(0).with_device(&d.dst);
    let mut script = Script::failures(&[Recovery::Retry]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert!(!source.exists());
    assert_eq!(names(&d.dst), ["a"]);
    assert_eq!(read(d.dst.join("a")), "whole file");
    assert!(outcome.skipped.is_empty());
    assert!(matches!(&outcome.changes[..], [Change::Moved { .. }]));
    assert_eq!(script.failed, std::slice::from_ref(&d.dst));
    assert_eq!(script.reports.last().unwrap().bytes_done, 10);
    assert!(fs.hit());
}

#[test]
fn a_folder_sync_failure_keeps_the_complete_source_tree() {
    let d = dirs();
    let source = d.src.join("folder");
    file(source.join("nested/a"), "whole file");
    let before = tree(&d.src);
    let fs = FaultFs::failing_directory_sync(0).with_device(&d.dst);
    let mut script = Script::failures(&[Recovery::Skip]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(&[source], &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(tree(&d.src), before);
    assert_eq!(read(d.dst.join("folder/nested/a")), "whole file");
    assert_eq!(outcome.skipped.len(), 1);
    assert!(fs.hit());
}

#[test]
fn a_source_directory_sync_failure_reports_a_visible_move_without_repeating_it() {
    let d = dirs();
    let source = d.src.join("a");
    file(&source, "whole file");
    let fs = FaultFs::failing_directory_sync(1).with_device(&d.dst);
    let mut script = Script::failures(&[Recovery::Skip]);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(std::slice::from_ref(&source), &d.dst),
        &mut script,
    )
    .unwrap();

    assert!(!source.exists());
    assert_eq!(read(d.dst.join("a")), "whole file");
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.skipped[0].reason.contains("source removal"));
    assert!(matches!(&outcome.changes[..], [Change::Moved { .. }]));
    assert!(fs.hit());
}

#[test]
fn numbered_names_keep_the_extension_and_hidden_names_whole() {
    assert_eq!(numbered("a.txt".as_ref(), 1), "a (1).txt");
    assert_eq!(numbered("folder".as_ref(), 2), "folder (2)");
    assert_eq!(numbered(".bashrc".as_ref(), 1), ".bashrc (1)");
}

#[test]
fn a_conflict_is_asked_about_and_skip_leaves_the_existing_file() {
    let d = dirs();
    file(d.src.join("a.txt"), "new");
    file(d.dst.join("a.txt"), "old");

    let mut script = Script::conflicts(&[(ConflictChoice::Skip, false)]);
    let outcome = go(&copy(&[d.src.join("a.txt")], &d.dst), &mut script);

    assert_eq!(read(d.dst.join("a.txt")), "old");
    assert_eq!(names(&d.dst), ["a.txt"]);
    assert!(outcome.changes.is_empty());
    assert_eq!(outcome.skipped[0].path, d.src.join("a.txt"));
    assert!(script.asked[0].can_overwrite);
    assert_eq!(script.asked[0].target, d.dst.join("a.txt"));
}

#[test]
fn overwrite_replaces_the_file_and_is_recorded_as_irreversible() {
    let d = dirs();
    file(d.src.join("a.txt"), "new");
    file(d.dst.join("a.txt"), "old");

    let mut script = Script::conflicts(&[(ConflictChoice::Overwrite, false)]);
    let outcome = go(&copy(&[d.src.join("a.txt")], &d.dst), &mut script);

    assert_eq!(read(d.dst.join("a.txt")), "new");
    assert_eq!(names(&d.dst), ["a.txt"]);
    assert_eq!(
        outcome.changes,
        [Change::Replaced {
            path: d.dst.join("a.txt")
        }]
    );
}

#[test]
fn keep_both_gives_the_new_entry_a_free_name() {
    let d = dirs();
    file(d.src.join("a.txt"), "new");
    file(d.dst.join("a.txt"), "old");

    let mut script = Script::conflicts(&[(ConflictChoice::KeepBoth, false)]);
    go(&copy(&[d.src.join("a.txt")], &d.dst), &mut script);

    assert_eq!(read(d.dst.join("a.txt")), "old");
    assert_eq!(read(d.dst.join("a (1).txt")), "new");
}

#[test]
fn apply_to_all_settles_later_conflicts_without_asking() {
    let d = dirs();
    for name in ["a", "b", "c"] {
        file(d.src.join(name), "new");
        file(d.dst.join(name), "old");
    }
    let sources = ["a", "b", "c"].map(|name| d.src.join(name));

    let mut script = Script::conflicts(&[
        (ConflictChoice::Skip, false),
        (ConflictChoice::Overwrite, true),
    ]);
    go(&copy(&sources, &d.dst), &mut script);

    assert_eq!(script.asked.len(), 2);
    assert_eq!(
        ["a", "b", "c"].map(|name| read(d.dst.join(name))),
        ["old", "new", "new"]
    );
}

#[test]
fn a_file_cannot_overwrite_a_folder_even_when_overwrite_applies_to_all() {
    let d = dirs();
    file(d.src.join("a"), "new a");
    file(d.src.join("b"), "file b");
    file(d.dst.join("a"), "old a");
    file(d.dst.join("b/inside"), "kept");

    let mut script = Script::conflicts(&[
        (ConflictChoice::Overwrite, true),
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::Skip, false),
    ]);
    go(
        &copy(&[d.src.join("a"), d.src.join("b")], &d.dst),
        &mut script,
    );

    assert_eq!(read(d.dst.join("a")), "new a");
    assert_eq!(read(d.dst.join("b/inside")), "kept");
    // Asked about "a", then twice about "b": overwrite is not possible there.
    assert_eq!(script.asked.len(), 3);
    assert!(!script.asked[1].can_overwrite);
}

#[test]
fn overwriting_a_folder_merges_and_asks_about_each_file_inside() {
    let d = dirs();
    file(d.src.join("pics/new.jpg"), "new");
    file(d.src.join("pics/same.jpg"), "theirs");
    file(d.dst.join("pics/same.jpg"), "ours");
    file(d.dst.join("pics/other.jpg"), "other");

    let mut script = Script::conflicts(&[
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::Skip, false),
    ]);
    go(&copy(&[d.src.join("pics")], &d.dst), &mut script);

    assert_eq!(
        names(&d.dst),
        ["pics", "pics/new.jpg", "pics/other.jpg", "pics/same.jpg"]
    );
    assert_eq!(read(d.dst.join("pics/same.jpg")), "ours");
    assert_eq!(script.asked[1].target, d.dst.join("pics/same.jpg"));
}

#[test]
fn cancelling_keeps_finished_entries_and_leaves_no_partial_or_temporary_ones() {
    let d = dirs();
    file(d.src.join("1-first"), "one");
    file(d.src.join("2-folder/a"), "a");
    file(d.src.join("2-folder/b"), "b");
    file(d.src.join("3-last"), "three");
    let sources = ["1-first", "2-folder", "3-last"].map(|name| d.src.join(name));
    let before = tree(&d.src);

    // Reports: one when the plan is ready, then one copy step and one finished
    // entry per file. The fourth report is the first step inside the folder.
    let mut script = Script {
        cancel_after: Some(4),
        ..Script::default()
    };
    let outcome = go(&copy(&sources, &d.dst), &mut script);

    assert!(!outcome.completed);
    assert_eq!(names(&d.dst), ["1-first"]);
    assert_eq!(outcome.changes.len(), 1);
    assert_eq!(tree(&d.src), before);
}

#[test]
fn moves_by_renaming_within_a_filesystem() {
    let d = dirs();
    file(d.src.join("folder/inner.txt"), "inner");
    let before = tree(&d.src);

    let outcome = go(
        &move_(&[d.src.join("folder")], &d.dst),
        &mut Script::default(),
    );

    assert_eq!(tree(&d.dst), before);
    assert!(names(&d.src).is_empty());
    assert!(matches!(
        &outcome.changes[..],
        [Change::Moved { from, to, .. }] if *from == d.src.join("folder") && *to == d.dst.join("folder")
    ));
}

#[test]
fn a_move_to_another_filesystem_copies_then_removes_the_source() {
    let d = dirs();
    file(d.src.join("folder/inner/deep.txt"), "deep");
    file(d.src.join("folder/top.txt"), "top");
    symlink("top.txt", d.src.join("folder/link")).unwrap();
    file(d.src.join("single.txt"), "single");
    let before = tree(&d.src);
    let fs = FaultFs::new().with_device(&d.dst);

    let mut script = Script::default();
    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(&[d.src.join("folder"), d.src.join("single.txt")], &d.dst),
        &mut script,
    )
    .unwrap();

    assert!(outcome.completed);
    assert_eq!(tree(&d.dst), before);
    assert!(names(&d.src).is_empty());
    assert!(outcome.skipped.is_empty());
    assert!(matches!(
        &outcome.changes[..],
        [Change::Moved { to: first, .. }, Change::Moved { to: second, .. }]
            if *first == d.dst.join("folder") && *second == d.dst.join("single.txt")
    ));
    let last = script.reports.last().unwrap();
    assert_eq!(
        (last.items_done, last.bytes_done),
        (last.items_total, last.bytes_total)
    );
}

#[test]
fn a_move_conflict_overwrite_replaces_and_skip_keeps_the_source() {
    let d = dirs();
    file(d.src.join("a"), "new a");
    file(d.src.join("b"), "new b");
    file(d.dst.join("a"), "old a");
    file(d.dst.join("b"), "old b");

    let mut script = Script::conflicts(&[
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::Skip, false),
    ]);
    go(
        &move_(&[d.src.join("a"), d.src.join("b")], &d.dst),
        &mut script,
    );

    assert_eq!(read(d.dst.join("a")), "new a");
    assert_eq!(read(d.dst.join("b")), "old b");
    assert_eq!(names(&d.src), ["b"]);
    assert_eq!(read(d.src.join("b")), "new b");
}

#[test]
fn moving_a_folder_onto_one_merges_and_removes_the_emptied_source() {
    let d = dirs();
    file(d.src.join("pics/new.jpg"), "new");
    file(d.src.join("pics/same.jpg"), "theirs");
    file(d.dst.join("pics/same.jpg"), "ours");

    let mut script = Script::conflicts(&[
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::KeepBoth, false),
    ]);
    let outcome = go(&move_(&[d.src.join("pics")], &d.dst), &mut script);

    assert_eq!(
        names(&d.dst),
        ["pics", "pics/new.jpg", "pics/same (1).jpg", "pics/same.jpg"]
    );
    assert_eq!(read(d.dst.join("pics/same.jpg")), "ours");
    assert!(names(&d.src).is_empty());
    assert!(matches!(
        outcome.changes.last(),
        Some(Change::RemovedEmptyDir { path, .. }) if *path == d.src.join("pics")
    ));
}

#[test]
fn a_merge_move_keeps_the_source_folder_when_something_stays_in_it() {
    let d = dirs();
    file(d.src.join("pics/same.jpg"), "theirs");
    file(d.dst.join("pics/same.jpg"), "ours");

    let mut script = Script::conflicts(&[
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::Skip, false),
    ]);
    go(&move_(&[d.src.join("pics")], &d.dst), &mut script);

    assert_eq!(read(d.src.join("pics/same.jpg")), "theirs");
    assert_eq!(read(d.dst.join("pics/same.jpg")), "ours");
}

#[test]
fn requests_that_cannot_work_are_refused_before_anything_changes() {
    let d = dirs();
    file(d.src.join("ok.txt"), "ok");
    file(d.src.join("folder/x"), "x");
    file(d.dst.join("ok.txt"), "other ok");
    let (before_src, before_dst) = (tree(&d.src), tree(&d.dst));
    let ok = d.src.join("ok.txt");
    let refused = |request: Request| {
        run(
            &RealFs,
            &NoTrash,
            &NoForeignReader,
            &request,
            &mut Script::default(),
        )
        .unwrap_err()
    };

    assert!(matches!(
        refused(copy(
            &[ok.clone(), d.src.join("folder")],
            &d.src.join("folder/x")
        )),
        OpError::NotADirectory(_)
    ));
    assert!(matches!(
        refused(copy(&[ok.clone(), d.src.join("folder")], &d.src.join("folder"))),
        OpError::IntoItself(path) if path == d.src.join("folder")
    ));
    assert!(matches!(
        refused(move_(&[d.src.join("folder/x"), ok.clone()], &d.src)),
        OpError::AlreadyThere(path) if path == ok
    ));
    assert!(matches!(
        refused(copy(&[ok.clone(), d.dst.join("ok.txt")], &d.src.join("folder"))),
        OpError::DuplicateName(name) if name == "ok.txt"
    ));
    assert!(matches!(
        refused(copy(&[ok.clone(), d.src.join("missing")], &d.dst)),
        OpError::Io { path, .. } if path == d.src.join("missing")
    ));
    assert_eq!((tree(&d.src), tree(&d.dst)), (before_src, before_dst));
}

#[test]
fn a_failing_file_can_be_skipped_and_the_rest_of_the_folder_is_copied() {
    let d = dirs();
    file(d.src.join("folder/a"), "a");
    file(d.src.join("folder/b"), "b");
    file(d.src.join("folder/c"), "c");
    // Steps: create folder, copy a, sync a (fails), copy b, …
    let fs = FaultFs::failing_at(2, faults::Fault::Error);

    let mut script = Script::failures(&[Recovery::Skip]);
    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &copy(&[d.src.join("folder")], &d.dst),
        &mut script,
    )
    .unwrap();

    let copied = names(&d.dst);
    assert_eq!(copied.len(), 3, "{copied:?}");
    assert_eq!(script.failed.len(), 1);
    let missing = script.failed[0].file_name().unwrap().to_owned();
    assert!(!d.dst.join("folder").join(&missing).exists());
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.completed);
}

#[test]
fn retrying_a_failed_file_copies_it_whole() {
    let d = dirs();
    file(d.src.join("a.txt"), "the whole content");
    let fs = FaultFs::failing_at(0, faults::Fault::Error);

    let mut script = Script::failures(&[Recovery::Retry]);
    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &copy(&[d.src.join("a.txt")], &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(names(&d.dst), ["a.txt"]);
    assert_eq!(read(d.dst.join("a.txt")), "the whole content");
    assert!(outcome.skipped.is_empty());
}

#[test]
fn a_cross_filesystem_move_keeps_the_originals_it_could_not_copy() {
    let d = dirs();
    file(d.src.join("folder/a"), "a");
    file(d.src.join("folder/b"), "b");
    // Steps: create folder, copy first file, sync first file (fails), …
    let fs = FaultFs::failing_at(2, faults::Fault::Error).with_device(&d.dst);

    let mut script = Script::failures(&[Recovery::Skip]);
    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(&[d.src.join("folder")], &d.dst),
        &mut script,
    )
    .unwrap();

    let kept = script.failed[0].strip_prefix(&d.src).unwrap().to_owned();
    assert_eq!(names(&d.src), ["folder", kept.to_str().unwrap()]);
    assert_eq!(names(&d.dst).len(), 2);
    assert!(!d.dst.join(&kept).exists());
    assert!(matches!(
        outcome.changes.last(),
        Some(Change::PartlyMoved { from, .. }) if *from == d.src.join("folder")
    ));
    // One note about the file that failed, none about the folder that had to stay.
    assert_eq!(outcome.skipped.len(), 1);
}

#[test]
fn a_source_that_changes_during_a_cross_filesystem_move_is_not_removed() {
    /// Rewrites the source right after it was copied.
    struct Meddler<'a>(&'a Path);
    impl Controller for Meddler<'_> {
        fn progress(&mut self, progress: &Progress) {
            if progress.items_done == 1 {
                fs::write(self.0, "changed after the copy was taken").unwrap();
            }
        }
        fn cancelled(&self) -> bool {
            false
        }
        fn conflict(&mut self, conflict: &Conflict) -> Resolution {
            panic!("unexpected conflict: {conflict:?}")
        }
        fn failure(&mut self, failure: &Failure) -> Recovery {
            panic!("unexpected failure: {failure:?}")
        }
    }
    let d = dirs();
    let source = d.src.join("a.txt");
    file(&source, "original");
    let fs = FaultFs::new().with_device(&d.dst);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(std::slice::from_ref(&source), &d.dst),
        &mut Meddler(&source),
    )
    .unwrap();

    assert_eq!(read(&source), "changed after the copy was taken");
    assert_eq!(read(d.dst.join("a.txt")), "original");
    assert_eq!(outcome.skipped[0].path, source);
}

#[test]
fn an_entry_that_appears_just_before_a_move_is_asked_about_not_replaced() {
    let d = dirs();
    file(d.src.join("a.txt"), "moved");
    let target = d.dst.join("a.txt");
    let late = target.clone();
    let fs = FaultFs::new().with_meddler(move |step| {
        if step == 0 {
            fs::write(&late, "arrived meanwhile").unwrap();
        }
    });

    let mut script = Script::conflicts(&[(ConflictChoice::Skip, false)]);
    run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(&[d.src.join("a.txt")], &d.dst),
        &mut script,
    )
    .unwrap();

    assert_eq!(script.asked[0].target, target);
    assert_eq!(read(&target), "arrived meanwhile");
    assert_eq!(read(d.src.join("a.txt")), "moved");
}

#[test]
fn cancelling_between_entries_stops_before_the_next_one() {
    let d = dirs();
    for name in ["a", "b", "c"] {
        file(d.src.join(name), name);
    }
    let sources = ["a", "b", "c"].map(|name| d.src.join(name));

    let mut script = Script {
        cancel_after: Some(2),
        ..Script::default()
    };
    let outcome = go(&move_(&sources, &d.dst), &mut script);

    assert!(!outcome.completed);
    assert_eq!(names(&d.dst), ["a"]);
    assert_eq!(names(&d.src), ["b", "c"]);
}

#[test]
fn cancelling_in_the_middle_of_a_large_file_leaves_nothing_behind() {
    let d = dirs();
    let big = d.src.join("big.bin");
    fs::write(&big, vec![7u8; crate::COPY_STEP * 2 + 1]).unwrap();

    let mut script = Script {
        cancel_after: Some(2),
        ..Script::default()
    };
    let outcome = go(&copy(&[big], &d.dst), &mut script);

    assert!(!outcome.completed);
    assert!(names(&d.dst).is_empty(), "{:?}", names(&d.dst));
    assert!(outcome.changes.is_empty());
}

/// A trash that is a plain folder: each entry is renamed into a numbered
/// subfolder of it.
pub(super) struct FolderTrash {
    dir: PathBuf,
    next: std::sync::atomic::AtomicUsize,
}

impl FolderTrash {
    pub(super) fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
            next: Default::default(),
        }
    }
}

impl Trash for FolderTrash {
    fn trash(&self, path: &Path) -> io::Result<TrashedItem> {
        let slot = self
            .dir
            .join(self.next.fetch_add(1, Ordering::Relaxed).to_string());
        fs::create_dir(&slot)?;
        let content = slot.join(path.file_name().unwrap());
        RealFs.rename_new(path, &content)?;
        Ok(TrashedItem {
            original: path.to_owned(),
            record: slot,
            content,
        })
    }

    fn restore(&self, item: &TrashedItem) -> io::Result<()> {
        RealFs.rename_new(&item.content, &item.original)?;
        fs::remove_dir(&item.record)
    }
}

#[test]
fn trashing_moves_entries_into_the_trash_and_records_where() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    file(d.src.join("a.txt"), "a");
    file(d.src.join("stays"), "stays");
    let trash = FolderTrash::new(&d.dst);

    let outcome = run(
        &RealFs,
        &trash,
        &NoForeignReader,
        &Request::Trash {
            paths: vec![d.src.join("folder"), d.src.join("a.txt")],
        },
        &mut Script::default(),
    )
    .unwrap();

    assert_eq!(names(&d.src), ["stays"]);
    assert_eq!(
        names(&d.dst),
        ["0", "0/folder", "0/folder/inner", "1", "1/a.txt"]
    );
    assert!(matches!(
        &outcome.changes[..],
        [Change::Trashed { item: first }, Change::Trashed { item: second }]
            if first.original == d.src.join("folder") && second.content == d.dst.join("1/a.txt")
    ));
}

#[test]
fn an_entry_that_cannot_be_trashed_stays_and_is_reported() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");

    let mut script = Script::failures(&[Recovery::Skip]);
    let outcome = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &Request::Trash {
            paths: vec![d.src.join("a.txt")],
        },
        &mut script,
    )
    .unwrap();

    assert_eq!(read(d.src.join("a.txt")), "a");
    assert!(outcome.changes.is_empty());
    assert_eq!(outcome.skipped[0].path, d.src.join("a.txt"));
}

fn delete(paths: &[PathBuf]) -> Request {
    Request::Delete {
        paths: paths.to_vec(),
    }
}

#[test]
fn deleting_removes_a_whole_tree_but_never_what_a_link_points_to() {
    let d = dirs();
    file(d.src.join("folder/sub/deep"), "deep");
    file(d.src.join("folder/top"), "top");
    file(d.dst.join("precious/data"), "precious");
    symlink(d.dst.join("precious"), d.src.join("folder/link-to-folder")).unwrap();
    symlink(d.dst.join("precious"), d.src.join("link")).unwrap();
    file(d.src.join("stays"), "stays");

    let outcome = go(
        &delete(&[d.src.join("folder"), d.src.join("link")]),
        &mut Script::default(),
    );

    assert_eq!(names(&d.src), ["stays"]);
    assert_eq!(read(d.dst.join("precious/data")), "precious");
    assert_eq!(
        outcome.changes,
        [
            Change::Deleted {
                path: d.src.join("folder")
            },
            Change::Deleted {
                path: d.src.join("link")
            }
        ]
    );
}

#[test]
fn a_delete_that_names_a_folder_and_something_inside_it_removes_both_once() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");

    let mut script = Script::default();
    let outcome = go(
        &delete(&[d.src.join("folder/inner"), d.src.join("folder")]),
        &mut script,
    );

    assert!(names(&d.src).is_empty());
    assert!(outcome.skipped.is_empty());
    assert_eq!(script.reports.last().unwrap().items_total, 2);
}

#[test]
fn a_file_that_cannot_be_deleted_keeps_its_folders_and_is_reported_once() {
    let d = dirs();
    file(d.src.join("folder/a"), "a");
    file(d.src.join("folder/b"), "b");
    // Steps: remove first file (fails), remove second file, …
    let fs = FaultFs::failing_at(0, faults::Fault::Error);

    let mut script = Script::failures(&[Recovery::Skip]);
    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &delete(&[d.src.join("folder")]),
        &mut script,
    )
    .unwrap();

    let kept = script.failed[0].clone();
    assert_eq!(tree(&d.src).len(), 2, "the folder and the kept file");
    assert!(kept.exists());
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.changes.is_empty());
}

#[test]
fn cancelling_a_delete_stops_before_the_next_entry() {
    let d = dirs();
    for name in ["a", "b", "c"] {
        file(d.src.join(name), name);
    }
    let sources = ["a", "b", "c"].map(|name| d.src.join(name));

    let mut script = Script {
        cancel_after: Some(2),
        ..Script::default()
    };
    let outcome = go(&delete(&sources), &mut script);

    assert!(!outcome.completed);
    assert_eq!(names(&d.src), ["b", "c"]);
}

// ---- The safety gate: a fault at every step ----

/// Sources for the sweeps: a file that will meet a conflict, and a folder
/// with nested content and a link.
fn fixture(d: &Dirs) -> Vec<PathBuf> {
    file(d.src.join("one.txt"), "the new one, longer than the old");
    file(d.src.join("folder/a"), "content of a");
    file(d.src.join("folder/sub/b"), "content of b");
    symlink("a", d.src.join("folder/link")).unwrap();
    file(d.dst.join("one.txt"), "old");
    file(d.dst.join("existing.txt"), "keep me");
    vec![d.src.join("folder"), d.src.join("one.txt")]
}

/// Entries of `tree` under the top-level name `top`.
fn under(tree: &BTreeMap<PathBuf, Item>, top: &str) -> BTreeMap<PathBuf, Item> {
    tree.iter()
        .filter(|(path, _)| path.starts_with(top))
        .map(|(path, item)| (path.clone(), item.clone()))
        .collect()
}

fn is_temporary(path: &Path) -> bool {
    path.components().any(|part| {
        part.as_os_str()
            .as_bytes()
            .starts_with(TEMP_PREFIX.as_bytes())
    })
}

/// What must hold however and wherever an operation on [`fixture`] stopped.
/// `original` is the source tree before it started.
///
/// With `may_leave_out`, the user chose to skip entries that failed, so the
/// folder may lack entries; those it has must still be whole.
fn assert_nothing_lost(
    d: &Dirs,
    original: &BTreeMap<PathBuf, Item>,
    moving: bool,
    may_leave_out: bool,
    context: &str,
) {
    let (src, mut dst) = (tree(&d.src), tree(&d.dst));
    dst.retain(|path, _| !is_temporary(path));

    // Untouched bystander.
    assert_eq!(
        dst.get(Path::new("existing.txt")),
        Some(&Item::File(b"keep me".to_vec())),
        "{context}"
    );
    // The conflicting file is the whole old one or the whole new one.
    let one = dst.get(Path::new("one.txt"));
    assert!(
        one == Some(&Item::File(b"old".to_vec())) || one == original.get(Path::new("one.txt")),
        "{context}: one.txt is {one:?}"
    );
    // The folder is under its final name only when complete.
    let folder = under(&dst, "folder");
    let whole_entries_only = folder
        .iter()
        .all(|(path, item)| original.get(path) == Some(item));
    assert!(
        folder.is_empty()
            || folder == under(original, "folder")
            || (may_leave_out && whole_entries_only),
        "{context}: partial folder under its final name: {folder:?}"
    );
    if moving {
        // Every source entry is intact where it was, or whole at the destination.
        for (path, item) in original {
            let whole_at_destination = dst.get(path) == Some(item);
            assert!(
                src.get(path) == Some(item) || whole_at_destination,
                "{context}: {} is neither intact at the source nor whole at the destination",
                path.display()
            );
        }
    } else {
        assert_eq!(&src, original, "{context}: a copy changed its source");
    }
}

/// Runs the fixture operation once per changing filesystem call, with that
/// call suffering `fault`, and checks the invariants after each run. Returns
/// the number of steps a full run takes.
fn sweep(fault: faults::Fault, moving: bool, across_filesystems: bool) -> usize {
    for step in 0.. {
        let d = dirs();
        let sources = fixture(&d);
        let original = tree(&d.src);
        let mut fs = FaultFs::failing_at(step, fault);
        if across_filesystems {
            fs = fs.with_device(&d.dst);
        }
        let request = if moving {
            move_(&sources, &d.dst)
        } else {
            copy(&sources, &d.dst)
        };
        let mut script = Script {
            conflicts: [Resolution {
                choice: ConflictChoice::Overwrite,
                apply_to_all: true,
            }]
            .into(),
            failures: std::iter::repeat_n(Recovery::Skip, 64).collect(),
            ..Script::default()
        };

        let outcome = run(&fs, &NoTrash, &NoForeignReader, &request, &mut script).unwrap();

        let context = format!("{fault:?} at step {step}");
        assert_nothing_lost(
            &d,
            &original,
            moving,
            fault != faults::Fault::Crash,
            &context,
        );
        if fault != faults::Fault::Crash {
            // The engine kept running, so it must have cleaned up after itself.
            let left: Vec<_> = tree(&d.dst)
                .into_keys()
                .filter(|path| is_temporary(path))
                .collect();
            assert!(
                left.is_empty(),
                "{context}: temporary entries left: {left:?}"
            );
        }
        if !fs.hit() {
            // The run got through without reaching the fault: all steps are covered.
            assert!(outcome.completed && outcome.skipped.is_empty());
            assert_eq!(tree(&d.dst).len(), original.len() + 1);
            return step;
        }
    }
    unreachable!()
}

#[test]
fn a_copy_that_dies_at_any_step_leaves_sources_and_final_names_whole() {
    assert!(sweep(faults::Fault::Crash, false, false) >= 7);
}

#[test]
fn a_copy_with_an_error_at_any_step_cleans_up_and_loses_nothing() {
    assert!(sweep(faults::Fault::Error, false, false) >= 7);
}

#[test]
fn a_move_that_dies_at_any_step_loses_nothing() {
    assert!(sweep(faults::Fault::Crash, true, false) >= 2);
}

#[test]
fn a_cross_filesystem_move_that_dies_at_any_step_loses_nothing() {
    assert!(sweep(faults::Fault::Crash, true, true) >= 12);
}

#[test]
fn a_cross_filesystem_move_with_an_error_at_any_step_loses_nothing() {
    assert!(sweep(faults::Fault::Error, true, true) >= 12);
}

#[test]
fn a_copy_that_comes_out_short_at_any_step_is_never_put_in_place() {
    assert!(sweep(faults::Fault::ShortCopy, false, false) >= 7);
}

#[test]
fn a_short_copy_at_any_step_of_a_cross_filesystem_move_keeps_the_original() {
    assert!(sweep(faults::Fault::ShortCopy, true, true) >= 12);
}

#[test]
fn an_entry_that_appears_while_copying_is_asked_about_not_replaced() {
    /// Puts a file at the destination while the copy is being written.
    struct Racer {
        target: PathBuf,
        asked: usize,
    }
    impl Controller for Racer {
        fn progress(&mut self, _: &Progress) {
            if !self.target.exists() {
                fs::write(&self.target, "arrived meanwhile").unwrap();
            }
        }
        fn cancelled(&self) -> bool {
            false
        }
        fn conflict(&mut self, conflict: &Conflict) -> Resolution {
            assert_eq!(conflict.target, self.target);
            self.asked += 1;
            Resolution {
                choice: ConflictChoice::KeepBoth,
                apply_to_all: false,
            }
        }
        fn failure(&mut self, failure: &Failure) -> Recovery {
            panic!("unexpected failure: {failure:?}")
        }
    }
    let d = dirs();
    file(d.src.join("a.txt"), "copied");
    let mut racer = Racer {
        target: d.dst.join("a.txt"),
        asked: 0,
    };

    run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &copy(&[d.src.join("a.txt")], &d.dst),
        &mut racer,
    )
    .unwrap();

    assert_eq!(racer.asked, 1);
    assert_eq!(read(d.dst.join("a.txt")), "arrived meanwhile");
    assert_eq!(read(d.dst.join("a (1).txt")), "copied");
    assert_eq!(names(&d.dst), ["a (1).txt", "a.txt"]);
}

const KILL_STEP: &str = "RUSTPHIN_TEST_KILL_STEP";
const KILL_DIR: &str = "RUSTPHIN_TEST_KILL_DIR";

/// Not a test of its own: the body of the child process that
/// [`a_process_killed_at_any_step_of_a_move_loses_nothing`] starts (with
/// `--ignored --exact`) and kills.
#[test]
#[ignore = "the body of a child process its parent test runs"]
fn kill_child() {
    let (Ok(step), Ok(dir)) = (std::env::var(KILL_STEP), std::env::var(KILL_DIR)) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let (src, dst) = (dir.join("src"), dir.join("dst"));
    let fs = FaultFs::failing_at(step.parse().unwrap(), faults::Fault::Kill).with_device(&dst);
    let mut script = Script::conflicts(&[(ConflictChoice::Overwrite, true)]);
    run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &move_(&[src.join("folder"), src.join("one.txt")], &dst),
        &mut script,
    )
    .unwrap();
}

#[test]
fn a_process_killed_at_any_step_of_a_move_loses_nothing() {
    use std::os::unix::process::ExitStatusExt;

    for step in 0.. {
        let d = dirs();
        fixture(&d);
        let original = tree(&d.src);

        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "ops::tests::kill_child"])
            .env(KILL_STEP, step.to_string())
            .env(KILL_DIR, d.src.parent().unwrap())
            .output()
            .unwrap()
            .status;

        assert_nothing_lost(
            &d,
            &original,
            true,
            false,
            &format!("killed at step {step}"),
        );
        if status.success() {
            // The child finished before reaching the step: every step was covered.
            assert!(step >= 12, "only {step} steps were exercised");
            assert!(names(&d.src).is_empty());
            return;
        }
        assert_eq!(
            status.signal(),
            Some(9),
            "the child failed on its own: {status:?}"
        );
    }
}

#[test]
fn a_batch_rename_is_one_undo_of_every_file() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    file(d.src.join("c"), "C");
    let request = Request::RenameBatch {
        directory: d.src.clone(),
        pairs: vec![
            ("a".into(), "a-1".into()),
            ("b".into(), "b-1".into()),
            ("c".into(), "c-1".into()),
        ],
    };
    let outcome = go(&request, &mut Script::default());
    assert!(outcome.completed);
    assert_eq!(read(d.src.join("a-1")), "A");
    assert_eq!(read(d.src.join("b-1")), "B");
    assert!(matches!(
        outcome.changes.as_slice(),
        [Change::BatchRenamed { items }] if items.len() == 3
    ));

    let mut history = History::default();
    history.push(request, outcome.changes);
    assert_eq!(history.len(), 1);
    history
        .undo_last(&RealFs, &NoTrash, &mut Script::default())
        .unwrap()
        .unwrap();
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
    assert_eq!(read(d.src.join("c")), "C");
}

#[test]
fn a_batch_rename_can_exchange_two_names_and_undo_the_exchange() {
    let d = dirs();
    file(d.src.join("left"), "L");
    file(d.src.join("right"), "R");
    let request = Request::RenameBatch {
        directory: d.src.clone(),
        pairs: vec![
            ("left".into(), "right".into()),
            ("right".into(), "left".into()),
        ],
    };
    let outcome = go(&request, &mut Script::default());
    assert_eq!(read(d.src.join("left")), "R");
    assert_eq!(read(d.src.join("right")), "L");
    let mut history = History::default();
    history.push(request, outcome.changes);
    history
        .undo_last(&RealFs, &NoTrash, &mut Script::default())
        .unwrap()
        .unwrap();
    assert_eq!(read(d.src.join("left")), "L");
    assert_eq!(read(d.src.join("right")), "R");
}

#[test]
fn a_batch_rename_can_take_a_name_another_selected_file_is_leaving() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    go(
        &Request::RenameBatch {
            directory: d.src.clone(),
            pairs: vec![("a".into(), "b".into()), ("b".into(), "c".into())],
        },
        &mut Script::default(),
    );
    assert_eq!(read(d.src.join("b")), "A");
    assert_eq!(read(d.src.join("c")), "B");
    assert!(!d.src.join("a").exists());
}

#[test]
fn a_batch_rename_refuses_a_name_that_is_staying_and_changes_nothing() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let error = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &Request::RenameBatch {
            directory: d.src.clone(),
            pairs: vec![("a".into(), "b".into())],
        },
        &mut Script::default(),
    )
    .unwrap_err();
    assert!(matches!(error, OpError::AlreadyExists(_)));
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
}

#[test]
fn a_failed_batch_rename_puts_every_file_back() {
    // Sweep record creation, all renames and all namespace-sync boundaries.
    for step in 0.. {
        let d = dirs();
        file(d.src.join("a"), "A");
        file(d.src.join("b"), "B");
        let request = Request::RenameBatch {
            directory: d.src.clone(),
            pairs: vec![("a".into(), "a2".into()), ("b".into(), "b2".into())],
        };
        let fs = FaultFs::failing_at(step, faults::Fault::Error);
        let result = run(
            &fs,
            &NoTrash,
            &NoForeignReader,
            &request,
            &mut Script::default(),
        );
        if !fs.hit() {
            assert!(result.unwrap().completed);
            assert!(step >= 9);
            break;
        }
        match result {
            Ok(outcome) if outcome.completed => {
                // Only a sync after journal removal can fail after commit.
                assert_eq!(outcome.skipped.len(), 1);
                assert!(outcome.skipped[0].reason.contains("record was removed"));
                assert_eq!(names(&d.src), ["a2", "b2"]);
                assert_eq!(read(d.src.join("a2")), "A");
                assert_eq!(read(d.src.join("b2")), "B");
                assert!(matches!(
                    &outcome.changes[..],
                    [Change::BatchRenamed { .. }]
                ));
            }
            other => {
                match other {
                    Ok(outcome) => assert!(outcome.changes.is_empty(), "step {step} left a change"),
                    Err(error) => assert!(matches!(error, OpError::Io { .. })),
                }
                // An initial journal-directory sync error retains a valid
                // record before staging; explicit recovery safely clears it.
                recover_rename_batch(&RealFs, &d.src).unwrap();
                assert_eq!(names(&d.src), ["a", "b"]);
                assert_eq!(read(d.src.join("a")), "A", "step {step}");
                assert_eq!(read(d.src.join("b")), "B", "step {step}");
            }
        }
    }
}

#[test]
fn permissions_change_and_come_back_and_a_failure_changes_nothing() {
    let d = dirs();
    let path = d.src.join("note");
    file(&path, "hi");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let outcome = go(
        &Request::SetMode {
            path: path.clone(),
            mode: 0o755,
        },
        &mut Script::default(),
    );
    assert_eq!(mode_of(&path), 0o755);
    super::undo::undo(&RealFs, &NoTrash, &outcome.changes, &mut Script::default()).unwrap();
    assert_eq!(mode_of(&path), 0o644);

    let fs = FaultFs::failing_at(0, faults::Fault::Error);
    let failed = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &Request::SetMode {
            path: path.clone(),
            mode: 0o707,
        },
        &mut Script::default(),
    );
    assert!(failed.is_err());
    assert!(fs.hit());
    assert_eq!(mode_of(&path), 0o644);
}

fn mode_of(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().mode() & 0o777
}

fn interrupted_batch(d: &Dirs) {
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let request = Request::RenameBatch {
        directory: d.src.clone(),
        pairs: vec![("a".into(), "b".into()), ("b".into(), "a".into())],
    };
    let _ = run(
        &FaultFs::failing_at(3, faults::Fault::Crash),
        &NoTrash,
        &NoForeignReader,
        &request,
        &mut Script::default(),
    );
}

#[test]
fn interrupted_batch_recovery_restores_a_swap_and_is_idempotent() {
    let d = dirs();
    interrupted_batch(&d);
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 2);
    assert_eq!(names(&d.src), ["a", "b"]);
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 0);
}

#[test]
fn recovery_refuses_a_taken_original_name_without_touching_anything() {
    let d = dirs();
    interrupted_batch(&d);
    file(d.src.join("a"), "arrived meanwhile");
    let before = tree(&d.src);

    let error = recover_rename_batch(&RealFs, &d.src).unwrap_err();

    assert!(
        matches!(&error, OpError::AlreadyExists(path) if *path == d.src.join("a")),
        "{error:?}"
    );
    assert_eq!(tree(&d.src), before);
}

#[test]
fn recovery_refuses_a_changed_staged_file_without_touching_anything() {
    let d = dirs();
    interrupted_batch(&d);
    let staged = fs::read_dir(&d.src)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .as_bytes()
                .starts_with(b".rustphin-part-rename-")
        })
        .unwrap();
    file(staged, "changed after interruption");
    let before = tree(&d.src);

    let error = recover_rename_batch(&RealFs, &d.src).unwrap_err();

    assert!(
        matches!(&error, OpError::Io { source, .. }
            if source.to_string().contains("missing or changed")),
        "{error:?}"
    );
    assert_eq!(tree(&d.src), before);
}

#[test]
fn recovery_refuses_a_malformed_journal_without_touching_files() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join(RENAME_JOURNAL), "not a recovery record");
    let before = tree(&d.src);

    let error = recover_rename_batch(&RealFs, &d.src).unwrap_err();

    assert_eq!(
        io_kind(&error),
        Some(io::ErrorKind::InvalidData),
        "{error:?}"
    );
    assert_eq!(tree(&d.src), before);
}

fn batch_swap(directory: &Path) -> Request {
    Request::RenameBatch {
        directory: directory.to_owned(),
        pairs: vec![("a".into(), "b".into()), ("b".into(), "a".into())],
    }
}

#[test]
fn a_batch_never_stages_entries_when_its_journal_directory_cannot_sync() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let fs = FaultFs::failing_directory_sync(0);

    let error = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &batch_swap(&d.src),
        &mut Script::default(),
    )
    .unwrap_err();

    assert!(matches!(error, OpError::Io { .. }));
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
    assert!(d.src.join(RENAME_JOURNAL).is_file());
    assert!(fs.hit());
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 2);
    assert_eq!(names(&d.src), ["a", "b"]);
}

#[test]
fn a_batch_keeps_recovery_until_published_names_are_synced() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let fs = FaultFs::failing_directory_sync(1);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &batch_swap(&d.src),
        &mut Script::default(),
    )
    .unwrap();

    assert!(!outcome.completed);
    assert!(outcome.changes.is_empty());
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
    assert_eq!(names(&d.src), ["a", "b"]);
    assert_eq!(outcome.skipped.len(), 1);
    assert!(fs.hit());
}

#[test]
fn a_batch_does_not_roll_back_after_its_record_has_been_removed() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let fs = FaultFs::failing_directory_sync(2);

    let outcome = run(
        &fs,
        &NoTrash,
        &NoForeignReader,
        &batch_swap(&d.src),
        &mut Script::default(),
    )
    .unwrap();

    assert!(outcome.completed);
    assert!(matches!(
        &outcome.changes[..],
        [Change::BatchRenamed { .. }]
    ));
    assert_eq!(read(d.src.join("a")), "B");
    assert_eq!(read(d.src.join("b")), "A");
    assert_eq!(names(&d.src), ["a", "b"]);
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.skipped[0].reason.contains("record was removed"));
    assert!(fs.hit());
}

#[test]
fn recovery_reports_a_late_sync_failure_after_restoring_original_names() {
    let d = dirs();
    interrupted_batch(&d);
    let fs = FaultFs::failing_directory_sync(1);

    let error = recover_rename_batch(&fs, &d.src).unwrap_err();

    assert!(error.to_string().contains("record was removed"));
    assert_eq!(names(&d.src), ["a", "b"]);
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
    assert!(fs.hit());
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 0);
}

const BATCH_MODE: &str = "RUSTPHIN_TEST_BATCH_MODE";

/// Not a test of its own: the body of the child process that
/// `kill_batch_at_every_step` starts (with `--ignored --exact`) and kills.
#[test]
#[ignore = "the body of a child process its parent test runs"]
fn batch_kill_child() {
    let (Ok(step), Ok(directory), Ok(mode)) = (
        std::env::var(KILL_STEP),
        std::env::var(KILL_DIR),
        std::env::var(BATCH_MODE),
    ) else {
        return;
    };
    let directory = PathBuf::from(directory);
    let fs = FaultFs::failing_at(step.parse().unwrap(), faults::Fault::Kill);
    match mode.as_str() {
        "rename" => {
            run(
                &fs,
                &NoTrash,
                &NoForeignReader,
                &batch_swap(&directory),
                &mut Script::default(),
            )
            .unwrap();
        }
        "undo" => {
            let outcome = go(&batch_swap(&directory), &mut Script::default());
            super::undo::undo(&fs, &NoTrash, &outcome.changes, &mut Script::default()).unwrap();
        }
        "recover" => {
            recover_rename_batch(&fs, &directory).unwrap();
        }
        _ => panic!("unknown batch child mode"),
    }
}

fn kill_batch_at_every_step(mode: &str) {
    use std::os::unix::process::ExitStatusExt;
    for step in 0.. {
        let d = dirs();
        if mode == "recover" {
            interrupted_batch(&d);
        } else {
            file(d.src.join("a"), "A");
            file(d.src.join("b"), "B");
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "ops::tests::batch_kill_child"])
            .env(KILL_STEP, step.to_string())
            .env(KILL_DIR, &d.src)
            .env(BATCH_MODE, mode)
            .output()
            .unwrap()
            .status;
        recover_rename_batch(&RealFs, &d.src).unwrap();
        assert_eq!(names(&d.src), ["a", "b"], "{mode}, step {step}");
        let contents = (read(d.src.join("a")), read(d.src.join("b")));
        assert!(
            contents == ("A".into(), "B".into()) || contents == ("B".into(), "A".into()),
            "{mode}, step {step}: {contents:?}"
        );
        if status.success() {
            assert!(step >= 3);
            break;
        }
        assert_eq!(
            status.signal(),
            Some(9),
            "{mode} child failed without SIGKILL: {status:?}"
        );
    }
}

#[test]
fn killed_batch_renames_are_recoverable_at_every_mutation() {
    kill_batch_at_every_step("rename");
}
#[test]
fn killed_batch_undos_are_recoverable_at_every_mutation() {
    kill_batch_at_every_step("undo");
}
#[test]
fn killed_recovery_can_be_retried_at_every_mutation() {
    kill_batch_at_every_step("recover");
}

#[test]
fn a_second_batch_or_recovery_cannot_enter_a_locked_directory() {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let before = tree(&d.src);
    let guard = RealFs.lock_directory(&d.src).unwrap();
    let batch = run(
        &RealFs,
        &NoTrash,
        &NoForeignReader,
        &batch_swap(&d.src),
        &mut Script::default(),
    )
    .unwrap_err();
    let recovery = recover_rename_batch(&RealFs, &d.src).unwrap_err();
    assert_eq!(
        io_kind(&batch),
        Some(io::ErrorKind::WouldBlock),
        "{batch:?}"
    );
    assert_eq!(
        io_kind(&recovery),
        Some(io::ErrorKind::WouldBlock),
        "{recovery:?}"
    );
    assert_eq!(tree(&d.src), before);
    drop(guard);
    assert!(go(&batch_swap(&d.src), &mut Script::default()).completed);
}

#[test]
fn recovery_does_not_guess_from_temporary_names_or_accept_links() {
    let d = dirs();
    file(d.src.join(".rustphin-part-rename-0"), "not ours");
    let before = tree(&d.src);
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 0);
    assert_eq!(tree(&d.src), before);
    file(d.dst.join("record"), "not a recovery record");
    symlink(d.dst.join("record"), d.src.join(RENAME_JOURNAL)).unwrap();
    let before = tree(&d.src);
    let error = recover_rename_batch(&RealFs, &d.src).unwrap_err();
    assert_eq!(
        io_kind(&error),
        Some(io::ErrorKind::InvalidData),
        "{error:?}"
    );
    assert_eq!(tree(&d.src), before);
    assert_eq!(read(d.dst.join("record")), "not a recovery record");
}

#[test]
fn a_copied_recovery_record_cannot_change_another_directory() {
    let d = dirs();
    interrupted_batch(&d);
    fs::copy(d.src.join(RENAME_JOURNAL), d.dst.join(RENAME_JOURNAL)).unwrap();
    for entry in fs::read_dir(&d.src).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != RENAME_JOURNAL {
            fs::hard_link(entry.path(), d.dst.join(entry.file_name())).unwrap();
        }
    }
    let before = tree(&d.dst);

    let error = recover_rename_batch(&RealFs, &d.dst).unwrap_err();

    assert_eq!(
        io_kind(&error),
        Some(io::ErrorKind::InvalidData),
        "{error:?}"
    );
    assert_eq!(tree(&d.dst), before);
}

#[test]
fn bounded_reads_refuse_oversized_files_and_links() {
    let d = dirs();
    file(d.src.join("record"), "12345");
    assert_eq!(
        RealFs
            .read_bounded(&d.src.join("record"), 4)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        RealFs.read_bounded(&d.src.join("record"), 5).unwrap(),
        b"12345"
    );
    symlink("record", d.src.join("link")).unwrap();
    assert!(RealFs.read_bounded(&d.src.join("link"), 5).is_err());
}

#[test]
fn recovery_preserves_raw_names_and_long_names() {
    let d = dirs();
    let raw = OsString::from_vec(b"bad\xff\nname".to_vec());
    let long = OsString::from("x".repeat(255));
    file(d.src.join(&raw), "raw");
    file(d.src.join(&long), "long");
    let before = tree(&d.src);
    let request = Request::RenameBatch {
        directory: d.src.clone(),
        pairs: vec![(raw.clone(), long.clone()), (long, raw)],
    };
    let _ = run(
        &FaultFs::failing_at(3, faults::Fault::Crash),
        &NoTrash,
        &NoForeignReader,
        &request,
        &mut Script::default(),
    );
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 2);
    assert_eq!(tree(&d.src), before);
}

#[test]
fn a_recovery_request_reports_a_pause_and_keeps_its_record() {
    let d = dirs();
    interrupted_batch(&d);
    let request = Request::RecoverRenameBatch {
        directory: d.src.clone(),
    };
    let outcome = run(
        &FaultFs::failing_at(0, faults::Fault::Error),
        &NoTrash,
        &NoForeignReader,
        &request,
        &mut Script::default(),
    )
    .unwrap();
    assert!(!outcome.completed);
    assert_eq!(outcome.skipped.len(), 1);
    assert!(d.src.join(RENAME_JOURNAL).is_file());
    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 2);
    assert_eq!(names(&d.src), ["a", "b"]);
}

#[test]
fn recovery_never_removes_a_record_changed_by_another_program() {
    let d = dirs();
    interrupted_batch(&d);
    let record = d.src.join(RENAME_JOURNAL);
    let replacement = record.clone();
    let fs = FaultFs::new().with_meddler(move |step| {
        if step == 0 {
            file(&replacement, "changed recovery record; keep me");
        }
    });
    let error = recover_rename_batch(&fs, &d.src).unwrap_err();
    assert!(
        matches!(&error, OpError::Io { source, .. }
            if source.to_string().contains("record changed")),
        "{error:?}"
    );
    assert_eq!(read(record), "changed recovery record; keep me");
    assert_eq!(read(d.src.join("a")), "A");
    assert_eq!(read(d.src.join("b")), "B");
}

#[test]
fn a_rename_journal_is_found_only_where_one_was_left() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!has_rename_journal(&RealFs, dir.path()));
    std::fs::write(dir.path().join(RENAME_JOURNAL), b"").unwrap();
    assert!(has_rename_journal(&RealFs, dir.path()));
    assert!(!has_rename_journal(&RealFs, &dir.path().join("absent")));
}

#[test]
fn duplicates_are_named_as_dolphin_names_them() {
    let name = |text: &str| duplicate_name(std::ffi::OsStr::new(text));
    assert_eq!(name("a.txt"), "a copy.txt");
    assert_eq!(name("b.tar.gz"), "b copy.tar.gz");
    assert_eq!(name("notes"), "notes copy");
    assert_eq!(name(".bashrc"), ".bashrc copy");
    assert_eq!(name("v1.2.final.pdf"), "v1.2.final copy.pdf");
    let raw = duplicate_name(std::ffi::OsStr::from_bytes(b"\xff.bin"));
    assert_eq!(raw.as_bytes(), b"\xff copy.bin");
}

#[test]
fn a_taken_duplicate_name_is_numbered_after_copy() {
    let free = |text: &str, taken: &[&str]| {
        free_duplicate_name(std::ffi::OsStr::new(text), |name| {
            taken.iter().any(|t| std::ffi::OsStr::new(t) == name)
        })
    };
    assert_eq!(free("a.txt", &[]), "a copy.txt");
    assert_eq!(free("a.txt", &["a copy.txt"]), "a copy (1).txt");
    assert_eq!(
        free("a.txt", &["a copy.txt", "a copy (1).txt"]),
        "a copy (2).txt"
    );
    assert_eq!(free("b.tar.gz", &["b copy.tar.gz"]), "b copy (1).tar.gz");
    assert_eq!(free("notes", &["notes copy"]), "notes copy (1)");
}

#[test]
fn duplicating_copies_beside_the_original_numbers_repeats_and_undoes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("report.txt");
    let folder = dir.path().join("photos");
    fs::write(&file, b"twelve bytes").unwrap();
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("inner.png"), b"png").unwrap();

    let request = Request::Duplicate {
        sources: vec![file.clone(), folder.clone()],
    };
    let first = go(&request, &mut Script::default());
    let second = go(&request, &mut Script::default());

    assert!(first.completed && second.completed);
    assert_eq!(
        fs::read(dir.path().join("report copy.txt")).unwrap(),
        b"twelve bytes"
    );
    assert_eq!(
        fs::read(dir.path().join("report copy (1).txt")).unwrap(),
        b"twelve bytes"
    );
    assert_eq!(
        fs::read(dir.path().join("photos copy/inner.png")).unwrap(),
        b"png"
    );
    assert!(dir.path().join("photos copy (1)/inner.png").exists());
    assert_eq!(fs::read(&file).unwrap(), b"twelve bytes");

    let bin = tempfile::tempdir().unwrap();
    let trash = FolderTrash::new(bin.path());
    super::undo::undo(&RealFs, &trash, &second.changes, &mut Script::default()).unwrap();
    assert!(!dir.path().join("report copy (1).txt").exists());
    assert!(!dir.path().join("photos copy (1)").exists());
    assert!(dir.path().join("report copy.txt").exists());
}
