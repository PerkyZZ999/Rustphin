use std::{
    ffi::OsString,
    fs,
    os::unix::{ffi::OsStringExt, fs::symlink},
    path::{Path, PathBuf},
};

use proptest::prelude::*;

use super::{
    faults::FaultFs,
    tests::{Dirs, FolderTrash, Script, copy, dirs, file, move_, names, read, tree},
    *,
};
use crate::RealFs;

/// Runs `request`, undoes it, and checks that `src` and `dst` are as before.
fn assert_undo_restores(d: &Dirs, fs: &dyn Fs, request: Request, mut script: Script) -> Outcome {
    let before = (tree(&d.src), tree(&d.dst));
    let trash_dir = d.src.parent().unwrap().join("trash");
    fs::create_dir_all(&trash_dir).unwrap();
    let trash = FolderTrash::new(&trash_dir);

    let outcome = run(fs, &trash, &request, &mut script).unwrap();
    assert!(!outcome.changes.is_empty(), "the operation did nothing");
    assert_ne!(
        (tree(&d.src), tree(&d.dst)),
        before,
        "the operation changed nothing"
    );
    undo(fs, &trash, &outcome.changes, &mut Script::default()).unwrap();

    assert_eq!((tree(&d.src), tree(&d.dst)), before);
    outcome
}

#[test]
fn creating_a_folder_or_a_file_is_undone_by_removing_it() {
    let d = dirs();
    file(d.src.join("bystander"), "stays");
    assert_undo_restores(
        &d,
        &RealFs,
        Request::CreateDir {
            parent: d.src.clone(),
            name: "New Folder".into(),
        },
        Script::default(),
    );
    assert_undo_restores(
        &d,
        &RealFs,
        Request::CreateFile {
            parent: d.src.clone(),
            name: "new.txt".into(),
        },
        Script::default(),
    );
}

#[test]
fn a_rename_is_undone() {
    let d = dirs();
    file(d.src.join("old.txt"), "content");
    assert_undo_restores(
        &d,
        &RealFs,
        Request::Rename {
            path: d.src.join("old.txt"),
            new_name: "new.txt".into(),
        },
        Script::default(),
    );
}

#[test]
fn a_copy_is_undone_by_trashing_the_copies() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    file(d.src.join("a.txt"), "a");
    file(d.dst.join("bystander"), "stays");

    assert_undo_restores(
        &d,
        &RealFs,
        copy(&[d.src.join("folder"), d.src.join("a.txt")], &d.dst),
        Script::default(),
    );

    // Not deleted: the undone copies can still be found in the trash.
    let trashed = names(&d.src.parent().unwrap().join("trash"));
    assert!(trashed.contains(&"0/a.txt".to_owned()), "{trashed:?}");
    assert!(
        trashed.contains(&"1/folder/inner".to_owned()),
        "{trashed:?}"
    );
}

#[test]
fn a_move_is_undone_within_and_across_filesystems() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    symlink("inner", d.src.join("folder/link")).unwrap();
    file(d.src.join("a.txt"), "a");
    let sources = [d.src.join("folder"), d.src.join("a.txt")];

    assert_undo_restores(&d, &RealFs, move_(&sources, &d.dst), Script::default());
    let across = FaultFs::new().with_device(&d.dst);
    assert_undo_restores(&d, &across, move_(&sources, &d.dst), Script::default());
}

#[test]
fn a_move_that_merged_folders_is_undone_including_the_removed_source_folder() {
    let d = dirs();
    file(d.src.join("pics/new.jpg"), "new");
    file(d.src.join("pics/same.jpg"), "theirs");
    file(d.dst.join("pics/same.jpg"), "ours");
    let script = Script::conflicts(&[
        (ConflictChoice::Overwrite, false),
        (ConflictChoice::KeepBoth, false),
    ]);

    let outcome = assert_undo_restores(&d, &RealFs, move_(&[d.src.join("pics")], &d.dst), script);

    assert!(matches!(
        outcome.changes.last(),
        Some(Change::RemovedEmptyDir { .. })
    ));
}

#[test]
fn trashing_is_undone_by_restoring() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    file(d.src.join("a.txt"), "a");
    assert_undo_restores(
        &d,
        &RealFs,
        Request::Trash {
            paths: vec![d.src.join("folder"), d.src.join("a.txt")],
        },
        Script::default(),
    );
}

#[test]
fn restoring_from_the_trash_puts_entries_back_and_its_undo_trashes_them_again() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    file(d.src.join("b.txt"), "b");
    let before = tree(&d.src);
    let trash = FolderTrash::new(&d.dst);
    let trashed = run(
        &RealFs,
        &trash,
        &Request::Trash {
            paths: vec![d.src.join("a.txt"), d.src.join("b.txt")],
        },
        &mut Script::default(),
    )
    .unwrap();
    let items: Vec<TrashedItem> = trashed
        .changes
        .iter()
        .map(|change| match change {
            Change::Trashed { item } => item.clone(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    // Something took the place of the second entry.
    file(d.src.join("b.txt"), "newcomer");

    let mut script = Script::failures(&[Recovery::Skip]);
    let outcome = run(&RealFs, &trash, &Request::Restore { items }, &mut script).unwrap();

    assert_eq!(read(d.src.join("a.txt")), "a");
    assert_eq!(read(d.src.join("b.txt")), "newcomer");
    assert_eq!(read(d.dst.join("1/b.txt")), "b");
    assert_eq!(
        outcome.changes,
        [Change::Restored {
            original: d.src.join("a.txt")
        }]
    );
    assert_eq!(outcome.skipped[0].path, d.src.join("b.txt"));

    undo(&RealFs, &trash, &outcome.changes, &mut Script::default()).unwrap();
    assert_eq!(names(&d.src), ["b.txt"]);
    assert_eq!(read(d.dst.join("2/a.txt")), "a");
    assert_ne!(tree(&d.src), before);
}

/// Runs `request` on the real filesystem and returns its changes.
fn changes_of(request: Request, script: &mut Script) -> Vec<Change> {
    run(&RealFs, &NoTrash, &request, script).unwrap().changes
}

fn refused(changes: &[Change]) -> UndoFailure {
    undo(&RealFs, &NoTrash, changes, &mut Script::default()).unwrap_err()
}

#[test]
fn an_overwrite_or_a_permanent_delete_cannot_be_undone_and_says_so() {
    let d = dirs();
    file(d.src.join("a.txt"), "new");
    file(d.dst.join("a.txt"), "old");
    file(d.src.join("gone"), "gone");
    let replaced = changes_of(
        copy(&[d.src.join("a.txt")], &d.dst),
        &mut Script::conflicts(&[(ConflictChoice::Overwrite, false)]),
    );
    let deleted = changes_of(
        Request::Delete {
            paths: vec![d.src.join("gone")],
        },
        &mut Script::default(),
    );

    let failure = refused(&replaced);
    assert!(matches!(&failure.error, UndoError::Replaced(path) if *path == d.dst.join("a.txt")));
    assert_eq!(failure.remaining, replaced);
    assert_eq!(read(d.dst.join("a.txt")), "new");
    assert!(matches!(refused(&deleted).error, UndoError::Deleted(_)));
}

#[test]
fn undo_is_refused_whole_when_one_copy_was_edited_since() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    file(d.src.join("b.txt"), "b");
    let trash = FolderTrash::new(&d.src);
    let changes = changes_of(
        copy(&[d.src.join("a.txt"), d.src.join("b.txt")], &d.dst),
        &mut Script::default(),
    );
    fs::write(d.dst.join("a.txt"), "edited after copying").unwrap();

    let failure = undo(&RealFs, &trash, &changes, &mut Script::default()).unwrap_err();

    assert!(matches!(&failure.error, UndoError::Changed(path) if *path == d.dst.join("a.txt")));
    assert_eq!(failure.remaining.len(), 2);
    // Nothing was touched, not even the copy that had not changed.
    assert_eq!(read(d.dst.join("a.txt")), "edited after copying");
    assert_eq!(read(d.dst.join("b.txt")), "b");
}

#[test]
fn undo_of_a_move_is_refused_when_the_old_place_is_taken_or_the_entry_is_gone() {
    let d = dirs();
    file(d.src.join("a.txt"), "moved");
    file(d.src.join("b.txt"), "moved too");
    let first = changes_of(
        move_(&[d.src.join("a.txt")], &d.dst),
        &mut Script::default(),
    );
    let second = changes_of(
        move_(&[d.src.join("b.txt")], &d.dst),
        &mut Script::default(),
    );
    file(d.src.join("a.txt"), "newcomer");
    fs::remove_file(d.dst.join("b.txt")).unwrap();

    assert!(
        matches!(refused(&first).error, UndoError::Occupied(path) if path == d.src.join("a.txt"))
    );
    assert!(
        matches!(refused(&second).error, UndoError::Missing(path) if path == d.dst.join("b.txt"))
    );
    assert_eq!(read(d.src.join("a.txt")), "newcomer");
    assert_eq!(read(d.dst.join("a.txt")), "moved");
}

#[test]
fn a_refused_undo_touches_none_of_the_operations_entries() {
    let d = dirs();
    for name in ["a", "b", "c", "d"] {
        file(d.src.join(name), name);
    }
    file(d.dst.join("c"), "old c");
    let trash = FolderTrash::new(&d.src.parent().unwrap().join("dst"));
    let moved = changes_of(
        move_(&[d.src.join("a"), d.src.join("b")], &d.dst),
        &mut Script::default(),
    );
    // "c" replaces a file, then "d" is copied: the copy is the newest change.
    let copied = changes_of(
        copy(&[d.src.join("c"), d.src.join("d")], &d.dst),
        &mut Script::conflicts(&[(ConflictChoice::Overwrite, false)]),
    );
    let trash_request = Request::Trash {
        paths: vec![d.src.join("c"), d.src.join("d")],
    };
    let trashed = run(&RealFs, &trash, &trash_request, &mut Script::default())
        .unwrap()
        .changes;
    // The places of the older entry of each pair are taken.
    file(d.src.join("a"), "newcomer");
    file(d.src.join("c"), "newcomer");
    let before = (tree(&d.src), tree(&d.dst));

    for (changes, expected) in [
        (&moved, "Occupied"),
        (&copied, "Replaced"),
        (&trashed, "Occupied"),
    ] {
        let failure = undo(&RealFs, &trash, changes, &mut Script::default()).unwrap_err();
        assert!(
            format!("{:?}", failure.error).starts_with(expected),
            "{:?}",
            failure.error
        );
        assert_eq!(&failure.remaining, changes);
    }

    assert_eq!((tree(&d.src), tree(&d.dst)), before);
}

#[test]
fn an_entry_that_appears_at_the_old_place_during_undo_is_not_replaced() {
    let d = dirs();
    file(d.src.join("a.txt"), "moved");
    let changes = changes_of(
        move_(&[d.src.join("a.txt")], &d.dst),
        &mut Script::default(),
    );
    let old_place = d.src.join("a.txt");
    let late = old_place.clone();
    let fs = FaultFs::new().with_meddler(move |_| fs::write(&late, "arrived meanwhile").unwrap());

    let failure = undo(&fs, &NoTrash, &changes, &mut Script::default()).unwrap_err();

    assert!(matches!(failure.error, UndoError::Occupied(path) if path == old_place));
    assert_eq!(read(&old_place), "arrived meanwhile");
    assert_eq!(read(d.dst.join("a.txt")), "moved");
}

#[test]
fn a_created_folder_that_was_filled_since_is_kept() {
    let d = dirs();
    let changes = changes_of(
        Request::CreateDir {
            parent: d.src.clone(),
            name: "New Folder".into(),
        },
        &mut Script::default(),
    );
    file(d.src.join("New Folder/work.txt"), "work");

    let failure = refused(&changes);

    assert!(
        matches!(failure.error, UndoError::Io { .. }),
        "{:?}",
        failure.error
    );
    assert_eq!(failure.remaining, changes);
    assert_eq!(read(d.src.join("New Folder/work.txt")), "work");
}

#[test]
fn a_created_file_that_was_written_to_since_is_kept() {
    let d = dirs();
    let changes = changes_of(
        Request::CreateFile {
            parent: d.src.clone(),
            name: "new.txt".into(),
        },
        &mut Script::default(),
    );
    fs::write(d.src.join("new.txt"), "work").unwrap();

    assert!(matches!(refused(&changes).error, UndoError::Changed(_)));
    assert_eq!(read(d.src.join("new.txt")), "work");
}

#[test]
fn a_copied_folder_that_gained_files_since_goes_to_the_trash_with_them() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    let changes = changes_of(
        copy(&[d.src.join("folder")], &d.dst),
        &mut Script::default(),
    );
    file(d.dst.join("folder/added later"), "later work");
    let trash = FolderTrash::new(&d.src.parent().unwrap().join("src"));

    undo(&RealFs, &trash, &changes, &mut Script::default()).unwrap();

    // Folders are not checked for changes; what was added is not lost.
    assert!(names(&d.dst).is_empty());
    assert_eq!(read(d.src.join("0/folder/added later")), "later work");
}

#[test]
fn a_file_put_where_a_copied_folder_was_is_not_taken_for_it() {
    let d = dirs();
    file(d.src.join("folder/inner"), "inner");
    let changes = changes_of(
        copy(&[d.src.join("folder")], &d.dst),
        &mut Script::default(),
    );
    fs::rename(d.dst.join("folder"), d.dst.join("renamed away")).unwrap();
    file(d.dst.join("folder"), "a file now");

    assert!(matches!(refused(&changes).error, UndoError::Changed(_)));
    assert_eq!(read(d.dst.join("folder")), "a file now");
}

#[test]
fn undoing_a_trash_is_refused_when_something_took_the_place() {
    let d = dirs();
    file(d.src.join("a.txt"), "trashed");
    let trash = FolderTrash::new(&d.dst);
    let request = Request::Trash {
        paths: vec![d.src.join("a.txt")],
    };
    let changes = run(&RealFs, &trash, &request, &mut Script::default())
        .unwrap()
        .changes;
    file(d.src.join("a.txt"), "newcomer");

    let failure = undo(&RealFs, &trash, &changes, &mut Script::default()).unwrap_err();

    assert!(matches!(failure.error, UndoError::Occupied(path) if path == d.src.join("a.txt")));
    assert_eq!(read(d.src.join("a.txt")), "newcomer");
    assert_eq!(read(d.dst.join("0/a.txt")), "trashed");
}

#[test]
fn an_undo_that_stops_half_way_keeps_what_is_still_in_effect() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    file(d.src.join("b.txt"), "b");
    let changes = changes_of(
        move_(&[d.src.join("a.txt"), d.src.join("b.txt")], &d.dst),
        &mut Script::default(),
    );
    // The newest change (b) is reverted first; the next rename fails.
    let fs = FaultFs::failing_at(1, faults::Fault::Error);

    let failure = undo(&fs, &NoTrash, &changes, &mut Script::default()).unwrap_err();

    assert!(matches!(failure.error, UndoError::Io { .. }));
    assert_eq!(failure.remaining, changes[..1]);
    assert_eq!(names(&d.src), ["b.txt"]);
    assert_eq!(names(&d.dst), ["a.txt"]);
}

#[test]
fn history_undoes_operations_newest_first_and_keeps_refused_ones() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    let original = (tree(&d.src), tree(&d.dst));
    let mut history = History::default();
    let mut perform = |request: Request| {
        let outcome = run(&RealFs, &NoTrash, &request, &mut Script::default()).unwrap();
        history.push(request, outcome.changes);
    };
    perform(Request::Rename {
        path: d.src.join("a.txt"),
        new_name: "b.txt".into(),
    });
    perform(move_(&[d.src.join("b.txt")], &d.dst));
    // Changes nothing, so there is nothing to undo.
    perform(Request::Rename {
        path: d.dst.join("b.txt"),
        new_name: "b.txt".into(),
    });
    let undo_last =
        |history: &mut History| history.undo_last(&RealFs, &NoTrash, &mut Script::default());

    // The move cannot be undone while its old place is taken; it stays on offer.
    file(d.src.join("b.txt"), "in the way");
    assert!(matches!(
        undo_last(&mut history),
        Some(Err(UndoError::Occupied(_)))
    ));
    assert!(matches!(
        history.last().unwrap().request,
        Request::Move { .. }
    ));
    fs::remove_file(d.src.join("b.txt")).unwrap();

    assert!(
        matches!(undo_last(&mut history), Some(Ok(record)) if matches!(record.request, Request::Move { .. }))
    );
    assert_eq!(names(&d.src), ["b.txt"]);
    assert!(
        matches!(undo_last(&mut history), Some(Ok(record)) if matches!(record.request, Request::Rename { .. }))
    );
    assert_eq!((tree(&d.src), tree(&d.dst)), original);
    assert!(undo_last(&mut history).is_none());
}

#[test]
fn history_forgets_the_oldest_operation_beyond_its_limit() {
    let mut history = History::default();
    let change = |n: usize| Change::Restored {
        original: PathBuf::from(n.to_string()),
    };
    for n in 0..=History::LIMIT {
        history.push(Request::Restore { items: Vec::new() }, vec![change(n)]);
    }

    assert_eq!(history.len(), History::LIMIT);
    assert_eq!(history.last().unwrap().changes, [change(History::LIMIT)]);
}

#[test]
fn an_operation_that_cannot_be_undone_does_not_block_undoing_the_one_before() {
    let d = dirs();
    file(d.src.join("a.txt"), "a");
    file(d.src.join("gone"), "gone");
    let mut history = History::default();
    for request in [
        Request::Rename {
            path: d.src.join("a.txt"),
            new_name: "b.txt".into(),
        },
        Request::Delete {
            paths: vec![d.src.join("gone")],
        },
    ] {
        let outcome = run(&RealFs, &NoTrash, &request, &mut Script::default()).unwrap();
        history.push(request, outcome.changes);
    }

    let undone = history.undo_last(&RealFs, &NoTrash, &mut Script::default());

    assert!(matches!(undone, Some(Ok(record)) if matches!(record.request, Request::Rename { .. })));
    assert_eq!(names(&d.src), ["a.txt"]);
}

// ---- Property: any sequence of operations can be undone back to the start ----

/// A step of a generated sequence. Indices pick among the entries that exist
/// when the step runs.
#[derive(Debug, Clone)]
enum Step {
    CreateDir(usize, usize),
    CreateFile(usize, usize),
    Rename(usize, usize),
    Copy(usize, usize),
    Move(usize, usize),
    Trash(usize),
}

/// Names that collide often, with one that is not UTF-8.
fn name(index: usize) -> OsString {
    const NAMES: [&[u8]; 5] = [b"a", b"b.txt", b"c d", b"\xff\xfe", b".hidden"];
    OsString::from_vec(NAMES[index % NAMES.len()].to_vec())
}

fn step() -> impl Strategy<Value = Step> {
    let index = 0..1000usize;
    prop_oneof![
        (index.clone(), index.clone()).prop_map(|(a, b)| Step::CreateDir(a, b)),
        (index.clone(), index.clone()).prop_map(|(a, b)| Step::CreateFile(a, b)),
        (index.clone(), index.clone()).prop_map(|(a, b)| Step::Rename(a, b)),
        (index.clone(), index.clone()).prop_map(|(a, b)| Step::Copy(a, b)),
        (index.clone(), index.clone()).prop_map(|(a, b)| Step::Move(a, b)),
        index.prop_map(Step::Trash),
    ]
}

/// Merges folders and keeps both files, so nothing is ever replaced.
struct NeverReplace;

impl Controller for NeverReplace {
    fn progress(&mut self, _: &Progress) {}
    fn cancelled(&self) -> bool {
        false
    }
    fn conflict(&mut self, conflict: &Conflict) -> Resolution {
        let both_folders = conflict.source_info.kind == EntryKind::Directory
            && conflict.target_info.kind == EntryKind::Directory;
        Resolution {
            choice: if both_folders {
                ConflictChoice::Overwrite
            } else {
                ConflictChoice::KeepBoth
            },
            apply_to_all: false,
        }
    }
    fn failure(&mut self, failure: &Failure) -> Recovery {
        panic!("unexpected failure: {failure:?}")
    }
}

/// `mount` is a folder standing in for a mount point; like a real one it
/// cannot itself be renamed, moved or trashed, so no step picks it.
fn request_for(step: &Step, root: &Path, mount: &Path) -> Option<Request> {
    let mut all: Vec<PathBuf> = tree(root).into_keys().map(|path| root.join(path)).collect();
    let mut folders: Vec<PathBuf> = all
        .iter()
        .filter(|path| path.is_dir() && !path.is_symlink())
        .cloned()
        .collect();
    folders.push(root.to_owned());
    all.retain(|path| path != mount);
    all.sort();
    let entry = |index: usize| (!all.is_empty()).then(|| all[index % all.len()].clone());
    let folder = |index: usize| folders[index % folders.len()].clone();
    Some(match *step {
        Step::CreateDir(parent, n) => Request::CreateDir {
            parent: folder(parent),
            name: name(n),
        },
        Step::CreateFile(parent, n) => Request::CreateFile {
            parent: folder(parent),
            name: name(n),
        },
        Step::Rename(path, n) => Request::Rename {
            path: entry(path)?,
            new_name: name(n),
        },
        Step::Copy(path, into) => Request::Copy {
            sources: vec![entry(path)?],
            into: folder(into),
        },
        Step::Move(path, into) => Request::Move {
            sources: vec![entry(path)?],
            into: folder(into),
        },
        Step::Trash(path) => Request::Trash {
            paths: vec![entry(path)?],
        },
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn any_sequence_of_operations_is_undone_back_to_the_starting_tree(
        steps in prop::collection::vec(step(), 1..14),
        across_filesystems: bool,
    ) {
        let d = dirs();
        let root = d.src.clone();
        file(root.join("a/one.txt"), "one");
        file(root.join("a/deep/two.txt"), "two");
        file(root.join("b.txt"), "three");
        symlink("b.txt", root.join("link")).unwrap();
        fs::create_dir(root.join("empty")).unwrap();
        let trash = FolderTrash::new(&d.dst);
        // Half the runs treat one folder as another filesystem, so moves in
        // and out of it copy and remove instead of renaming.
        let mut fs = FaultFs::new();
        if across_filesystems {
            fs = fs.with_device(&root.join("a"));
        }
        let original = tree(&root);

        let mut history = History::default();
        for step in &steps {
            let Some(request) = request_for(step, &root, &root.join("a")) else { continue };
            // Refused requests (a folder into itself, a taken name) change nothing.
            if let Ok(outcome) = run(&fs, &trash, &request, &mut NeverReplace) {
                prop_assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
                history.push(request, outcome.changes);
            }
        }
        while let Some(result) = history.undo_last(&fs, &trash, &mut NeverReplace) {
            prop_assert!(result.is_ok(), "{:?}", result);
        }

        prop_assert_eq!(tree(&root), original);
    }
}
