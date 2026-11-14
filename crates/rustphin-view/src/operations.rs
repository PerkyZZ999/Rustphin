//! What file operations are called on screen: activity lines, notices,
//! progress details and the names new entries get (W-114b2). What is
//! running and shown is decided by [`crate::operation_board`] (W-124); the
//! words live here.

use std::{
    collections::HashSet,
    ffi::OsStr,
    os::unix::ffi::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
    time::Duration,
};

use rustphin_core::{
    display_name,
    ops::{Finished, Outcome, Progress, Request},
};
use rustphin_platform::clipboard::FileClip;

use crate::format::{self, count, plural};
pub use crate::naming::free_name;

/// What a drop does with the dropped entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropAction {
    /// Leave the originals where they are.
    Copy,
    /// Take the originals away.
    Move,
}

/// A message about something that just happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The message, a full sentence.
    pub text: String,
    /// Whether something went wrong. Such a message stays until dismissed.
    pub is_error: bool,
    /// Whether it reports an operation that can be undone, so an undo
    /// button belongs beside it.
    pub offers_undo: bool,
}

/// What an operation left out, for the error details dialog.
#[derive(Debug, Clone)]
pub struct Problems {
    /// What the operation was doing: "Copying 12 items to “Pictures”".
    pub title: String,
    /// The entries left out and why.
    pub entries: Vec<rustphin_core::ops::Skipped>,
}

impl Problems {
    /// As JSON for the details dialog: the operation, then each entry left
    /// out and why.
    pub fn json(&self) -> String {
        let entries: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|skipped| {
                serde_json::json!({
                    "path": skipped.path.display().to_string(),
                    "reason": skipped.reason,
                })
            })
            .collect();
        serde_json::json!({ "title": self.title, "entries": entries }).to_string()
    }
}

/// What a question's list of names shows beside a name, from the folder
/// listing the question came from (nothing is read for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Listed {
    /// It is a folder.
    pub folder: bool,
    /// Its size, for a file.
    pub size: Option<u64>,
}

/// The entries of a drop that can go into `into`: not those already in it,
/// not the folder itself and not a folder that contains it.
pub fn droppable(sources: Vec<PathBuf>, into: &Path) -> Vec<PathBuf> {
    sources
        .into_iter()
        .filter(|source| source.parent() != Some(into) && !into.starts_with(source))
        .collect()
}

/// What to say after a copy or cut. The first time on a desktop without a
/// shared clipboard, also that other applications will not see it.
pub fn pick_up_notice(clip: &FileClip, shared: bool, told_unshared: bool) -> String {
    let what = what(&clip.paths);
    let mut text = if clip.cut {
        format!("Cut {what}. Paste to move.")
    } else {
        format!("Copied {what}. Paste to copy.")
    };
    if !shared && !told_unshared {
        text.push_str(
            " This desktop offers no shared file clipboard, so pasting works only inside Rustphin.",
        );
    }
    text
}

/// `name` in curly quotes, as messages show names.
pub fn quoted(name: &std::ffi::OsStr) -> String {
    format!("“{}”", display_name(name.as_encoded_bytes()))
}

/// The quoted last component of `path`, or the whole path when it has none.
pub fn folder_name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => quoted(name),
        None => "“/”".to_owned(),
    }
}

/// The entries of a request in words: the name of one, the number of several.
pub fn what(paths: &[PathBuf]) -> String {
    match paths {
        [one] => folder_name(one),
        many => format!(
            "{} {}",
            count(many.len()),
            plural(many.len(), "item", "items")
        ),
    }
}

/// Where trashed items came from.
pub fn originals(items: &[rustphin_core::ops::TrashedItem]) -> Vec<PathBuf> {
    items.iter().map(|item| item.original.clone()).collect()
}

/// What a request does, for the progress line: "Copying 3 items to “x”".
/// With `undo`, what reverting it does.
pub fn activity(request: &Request, undo: bool) -> String {
    if undo {
        return format!("Undoing: {}", past(request));
    }
    match request {
        Request::CreateDir { name, .. } => format!("Creating folder {}", quoted(name)),
        Request::CreateFile { name, .. } | Request::CreateFromTemplate { name, .. } => {
            format!("Creating file {}", quoted(name))
        }
        Request::Rename { path, new_name } => {
            format!("Renaming {} to {}", folder_name(path), quoted(new_name))
        }
        Request::RenameBatch { pairs, .. } => format!(
            "Renaming {} {}",
            count(pairs.len()),
            plural(pairs.len(), "item", "items")
        ),
        Request::RecoverRenameBatch { directory } => {
            format!("Recovering original names in {}", folder_name(directory))
        }
        Request::Copy { sources, into } => {
            format!("Copying {} to {}", what(sources), folder_name(into))
        }
        Request::Move { sources, into } => {
            format!("Moving {} to {}", what(sources), folder_name(into))
        }
        Request::Duplicate { sources } => format!("Duplicating {}", what(sources)),
        Request::Trash { paths } => format!("Moving {} to the trash", what(paths)),
        Request::Restore { items } => format!("Restoring {}", what(&originals(items))),
        Request::Delete { paths } => format!("Deleting {}", what(paths)),
        Request::SetMode { path, .. } => format!("Changing permissions of {}", folder_name(path)),
        Request::Extract { archive, .. } => format!("Extracting {}", folder_name(archive)),
        Request::Compress {
            sources, archive, ..
        } => format!(
            "Compressing {} into {}",
            what(sources),
            folder_name(archive)
        ),
    }
}

/// What a request did, in lower case: "copied 3 items to “x”".
pub fn past(request: &Request) -> String {
    match request {
        Request::CreateDir { name, .. } => format!("created folder {}", quoted(name)),
        Request::CreateFile { name, .. } | Request::CreateFromTemplate { name, .. } => {
            format!("created file {}", quoted(name))
        }
        Request::Rename { path, new_name } => {
            format!("renamed {} to {}", folder_name(path), quoted(new_name))
        }
        Request::RenameBatch { pairs, .. } => format!(
            "renamed {} {}",
            count(pairs.len()),
            plural(pairs.len(), "item", "items")
        ),
        Request::RecoverRenameBatch { directory } => {
            format!("checked rename recovery in {}", folder_name(directory))
        }
        Request::Copy { sources, into } => {
            format!("copied {} to {}", what(sources), folder_name(into))
        }
        Request::Move { sources, into } => {
            format!("moved {} to {}", what(sources), folder_name(into))
        }
        Request::Duplicate { sources } => format!("duplicated {}", what(sources)),
        Request::Trash { paths } => format!("moved {} to the trash", what(paths)),
        Request::Restore { items } => {
            format!("restored {} from the trash", what(&originals(items)))
        }
        Request::Delete { paths } => format!("deleted {}", what(paths)),
        Request::SetMode { path, .. } => format!("changed permissions of {}", folder_name(path)),
        Request::Extract {
            archive,
            trash_archive: true,
            ..
        } => format!(
            "extracted {} and moved it to the trash",
            folder_name(archive)
        ),
        Request::Extract { archive, .. } => format!("extracted {}", folder_name(archive)),
        Request::Compress {
            sources, archive, ..
        } => format!("compressed {} into {}", what(sources), folder_name(archive)),
    }
}

/// The message for a finished operation, if it deserves one. `done` says
/// what the operation did, in lower case; `undo` whether it was a revert.
/// Creating and renaming succeed silently: the result is visible in the list.
pub fn result_notice(done: &str, undo: bool, result: &Finished) -> Option<Notice> {
    match result {
        Finished::Done(outcome) => outcome_notice(done, outcome),
        Finished::Refused(reason) => Some(Notice {
            text: format!("Not done: {reason}."),
            is_error: true,
            offers_undo: false,
        }),
        Finished::Undone => Some(Notice {
            text: format!("Undid: {done}."),
            is_error: false,
            offers_undo: false,
        }),
        Finished::UndoFailed(reason) if undo => Some(Notice {
            text: format!("Could not undo ({done}): {reason}."),
            is_error: true,
            offers_undo: false,
        }),
        Finished::UndoFailed(reason) => Some(Notice {
            text: format!("Could not undo: {reason}."),
            is_error: true,
            offers_undo: false,
        }),
    }
}

/// What an operation does to each entry, in a question's title:
/// "Couldn't copy “x”".
pub fn activity_verb(request: &Request) -> &'static str {
    match request {
        Request::Copy { .. } | Request::Duplicate { .. } => "copy",
        Request::Move { .. } => "move",
        Request::Trash { .. } => "move to the trash",
        Request::Delete { .. } => "delete",
        Request::Restore { .. } => "restore",
        Request::Rename { .. }
        | Request::RenameBatch { .. }
        | Request::RecoverRenameBatch { .. } => "rename",
        Request::Extract { .. } => "extract",
        Request::Compress { .. } => "compress",
        _ => "change",
    }
}

/// The icon of an activity line: what kind of work it is (docs/DESIGN.md).
pub fn activity_icon(request: &Request) -> crate::icons::Icon {
    use crate::icons::Icon as I;
    match request {
        Request::Copy { .. } | Request::Duplicate { .. } => I::Copy,
        Request::Move { .. } => I::Move,
        Request::Trash { .. } | Request::Delete { .. } => I::Trash,
        Request::Restore { .. } => I::Undo2,
        Request::Rename { .. }
        | Request::RenameBatch { .. }
        | Request::RecoverRenameBatch { .. } => I::Pencil,
        Request::Extract { .. } => I::PackageOpen,
        Request::Compress { .. } => I::FileArchive,
        _ => I::Loader,
    }
}

/// The notice for a finished operation that did `done`, if one is shown.
pub fn outcome_notice(done: &str, outcome: &Outcome) -> Option<Notice> {
    let quiet = done.starts_with("created") || done.starts_with("renamed");
    let left_out = outcome.skipped.len();
    if left_out == 0 && outcome.completed {
        return (!quiet).then(|| Notice {
            text: format!("{}.", upper_first(done)),
            is_error: false,
            // Only what the history keeps: see `History::push`.
            offers_undo: !outcome.changes.is_empty()
                && outcome
                    .changes
                    .iter()
                    .all(rustphin_core::ops::Change::is_reversible),
        });
    }
    let mut text = if outcome.completed {
        format!("{}, with problems.", upper_first(done))
    } else if outcome.changes.is_empty() {
        // A folder copy is published only when complete (D-011), so a
        // stopped one can leave nothing at all.
        "Stopped before anything changed.".to_owned()
    } else {
        format!("Stopped part-way: {done}. What was done stays.")
    };
    if let Some(first) = outcome.skipped.first() {
        let name = first
            .path
            .file_name()
            .map(quoted)
            .unwrap_or_else(|| first.path.display().to_string());
        text.push_str(&format!(
            " {} {} left out; {name}: {}.",
            count(left_out),
            plural(left_out, "entry was", "entries were"),
            first.reason
        ));
    }
    Some(Notice {
        text,
        is_error: left_out > 0,
        offers_undo: false,
    })
}

/// `text` with its first letter in upper case.
pub fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// How far along an operation is, from 0 to 1, when that is known.
pub fn fraction(progress: &Progress) -> Option<f32> {
    if progress.bytes_total > 0 {
        Some((progress.bytes_done as f64 / progress.bytes_total as f64) as f32)
    } else if progress.items_total > 0 {
        Some((progress.items_done as f64 / progress.items_total as f64) as f32)
    } else {
        None
    }
}

/// The progress line under an activity: items, bytes and rate.
pub fn detail(progress: &Progress, elapsed: Duration) -> String {
    let mut text = format!(
        "{} of {}",
        count(progress.items_done as usize),
        count(progress.items_total as usize)
    );
    if progress.bytes_total > 0 {
        text.push_str(&format!(
            " · {} of {}",
            format::size(progress.bytes_done),
            format::size(progress.bytes_total)
        ));
        let seconds = elapsed.as_secs_f64();
        if seconds > 0.5 && progress.bytes_done > 0 {
            let rate = (progress.bytes_done as f64 / seconds) as u64;
            text.push_str(&format!(" · {}/s", format::size(rate)));
        }
    }
    text
}

/// The names that duplicating `names` side by side creates, in order, given
/// which names `exists` in the folder: what the engine will choose, so the
/// view can select the copies once they appear, as Dolphin does.
pub fn duplicate_names(names: &[Vec<u8>], exists: impl Fn(&[u8]) -> bool) -> Vec<Vec<u8>> {
    let mut made: Vec<Vec<u8>> = Vec::with_capacity(names.len());
    let mut taken: std::collections::HashSet<Vec<u8>> =
        std::collections::HashSet::with_capacity(names.len());
    for name in names {
        let target =
            rustphin_core::ops::free_duplicate_name(OsStr::from_bytes(name), |candidate| {
                let candidate = candidate.as_bytes();
                exists(candidate) || taken.contains(candidate)
            })
            .into_vec();
        taken.insert(target.clone());
        made.push(target);
    }
    made
}

/// The cut paths of `clip` and the folders they are in; empty for a copy.
pub fn cut_sets(clip: Option<&FileClip>) -> (HashSet<PathBuf>, HashSet<PathBuf>) {
    let paths: HashSet<PathBuf> = clip
        .filter(|clip| clip.cut)
        .map(|clip| clip.paths.iter().cloned().collect())
        .unwrap_or_default();
    let folders = paths
        .iter()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();
    (paths, folders)
}

/// The part of a file name to select when renaming starts: everything before
/// the extension, so typing replaces the name and keeps the type.
pub fn stem_len(name: &str, is_folder: bool) -> usize {
    if is_folder {
        return name.len();
    }
    match name.rfind('.') {
        Some(dot) if dot > 0 => dot,
        _ => name.len(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rustphin_core::ops::Skipped;

    use super::*;

    fn copy(sources: &[&str], into: &str) -> Request {
        Request::Copy {
            sources: sources.iter().map(PathBuf::from).collect(),
            into: into.into(),
        }
    }

    #[test]
    fn one_entry_is_named_and_several_are_counted() {
        assert_eq!(
            activity(&copy(&["/a/report.pdf"], "/home/u/Documents"), false),
            "Copying “report.pdf” to “Documents”"
        );
        assert_eq!(
            activity(&copy(&["/a/1", "/a/2", "/a/3"], "/"), false),
            "Copying 3 items to “/”"
        );
        assert_eq!(
            activity(
                &Request::Trash {
                    paths: vec!["/a/old".into()]
                },
                false
            ),
            "Moving “old” to the trash"
        );
    }

    #[test]
    fn an_undo_says_what_it_reverts() {
        let request = Request::Rename {
            path: "/a/old.txt".into(),
            new_name: "new.txt".into(),
        };
        assert_eq!(
            activity(&request, true),
            "Undoing: renamed “old.txt” to “new.txt”"
        );
    }

    fn done(completed: bool, skipped: &[(&str, &str)]) -> Finished {
        Finished::Done(Outcome {
            completed,
            changes: Vec::new(),
            skipped: skipped
                .iter()
                .map(|(path, reason)| Skipped {
                    path: path.into(),
                    reason: (*reason).to_owned(),
                })
                .collect(),
        })
    }

    #[test]
    fn creating_and_renaming_succeed_without_a_message() {
        assert_eq!(
            result_notice("created folder “x”", false, &done(true, &[])),
            None
        );
        assert_eq!(
            result_notice("renamed “a” to “b”", false, &done(true, &[])),
            None
        );
    }

    #[test]
    fn a_finished_copy_is_confirmed_and_an_undo_is_named() {
        assert_eq!(
            result_notice("copied 3 items to “x”", false, &done(true, &[])),
            Some(Notice {
                text: "Copied 3 items to “x”.".into(),
                is_error: false,
                offers_undo: false,
            })
        );
        assert_eq!(
            result_notice("moved 3 items to “x”", true, &Finished::Undone),
            Some(Notice {
                text: "Undid: moved 3 items to “x”.".into(),
                is_error: false,
                offers_undo: false,
            })
        );
    }

    #[test]
    fn problems_say_what_was_left_out_and_why_and_stay_until_dismissed() {
        let notice = result_notice(
            "copied 3 items to “x”",
            false,
            &done(
                true,
                &[("/a/locked.txt", "permission denied"), ("/a/b", "busy")],
            ),
        )
        .unwrap();
        assert_eq!(
            notice.text,
            "Copied 3 items to “x”, with problems. 2 entries were left out; “locked.txt”: permission denied."
        );
        assert!(notice.is_error);

        let refused = result_notice(
            "renamed “a” to “b”",
            false,
            &Finished::Refused("\"/x/b\" already exists".into()),
        )
        .unwrap();
        assert_eq!(refused.text, "Not done: \"/x/b\" already exists.");
        assert!(refused.is_error);

        let undo = result_notice(
            "moved “a” to “x”",
            true,
            &Finished::UndoFailed("something else is now at \"/a\"".into()),
        )
        .unwrap();
        assert_eq!(
            undo.text,
            "Could not undo (moved “a” to “x”): something else is now at \"/a\"."
        );
        assert!(undo.is_error);
    }

    #[test]
    fn a_finished_operation_that_changed_something_offers_undo() {
        let outcome = Outcome {
            completed: true,
            changes: vec![rustphin_core::ops::Change::Restored {
                original: "/a".into(),
            }],
            skipped: Vec::new(),
        };
        let notice = result_notice("moved “a” to the trash", false, &Finished::Done(outcome));
        assert!(notice.unwrap().offers_undo);

        let deleted = Outcome {
            completed: true,
            changes: vec![rustphin_core::ops::Change::Deleted { path: "/a".into() }],
            skipped: Vec::new(),
        };
        let notice = result_notice("deleted “a”", false, &Finished::Done(deleted));
        assert!(!notice.unwrap().offers_undo);
    }

    #[test]
    fn a_cancelled_operation_says_that_finished_work_stays() {
        let mut partial = done(false, &[]);
        if let Finished::Done(outcome) = &mut partial {
            outcome
                .changes
                .push(rustphin_core::ops::Change::Deleted { path: "/a".into() });
        }
        let notice = result_notice("moved 9 items to “x”", false, &partial).unwrap();
        assert_eq!(
            notice.text,
            "Stopped part-way: moved 9 items to “x”. What was done stays."
        );
        assert!(!notice.is_error);
    }

    #[test]
    fn a_cancelled_operation_that_changed_nothing_says_so() {
        // A folder copy is published only when complete (D-011): stopped,
        // it leaves nothing, and the notice must not claim otherwise.
        let notice = result_notice("copied “big”", false, &done(false, &[])).unwrap();
        assert_eq!(notice.text, "Stopped before anything changed.");
        assert!(!notice.offers_undo);
    }

    #[test]
    fn progress_is_measured_in_bytes_when_known_and_in_entries_otherwise() {
        let mut progress = Progress {
            items_done: 1,
            items_total: 4,
            ..Progress::default()
        };
        assert_eq!(fraction(&progress), Some(0.25));
        assert_eq!(detail(&progress, Duration::from_secs(2)), "1 of 4");
        progress.bytes_done = 1024 * 1024;
        progress.bytes_total = 4 * 1024 * 1024;
        assert_eq!(fraction(&progress), Some(0.25));
        assert_eq!(
            detail(&progress, Duration::from_secs(2)),
            "1 of 4 · 1.0 MiB of 4.0 MiB · 512.0 KiB/s"
        );
        assert_eq!(fraction(&Progress::default()), None);
    }

    #[test]
    fn the_missing_shared_clipboard_is_mentioned_once() {
        let clip = FileClip {
            paths: vec!["/a/report.pdf".into()],
            cut: true,
        };
        assert_eq!(
            pick_up_notice(&clip, true, false),
            "Cut “report.pdf”. Paste to move."
        );
        assert_eq!(
            pick_up_notice(&clip, false, false),
            "Cut “report.pdf”. Paste to move. This desktop offers no shared file clipboard, so pasting works only inside Rustphin."
        );
        assert_eq!(
            pick_up_notice(&clip, false, true),
            "Cut “report.pdf”. Paste to move."
        );
    }

    #[test]
    fn a_drop_leaves_out_what_is_already_there_or_contains_the_destination() {
        let sources = ["/a/x.txt", "/b/y.txt", "/b", "/b/c", "/other/z"]
            .map(PathBuf::from)
            .to_vec();
        assert_eq!(
            droppable(sources, Path::new("/b/c")),
            ["/a/x.txt", "/b/y.txt", "/other/z"].map(PathBuf::from)
        );
        assert!(droppable(vec!["/a/x".into()], Path::new("/a")).is_empty());
    }

    #[test]
    fn a_new_entry_gets_the_first_free_name() {
        let taken = ["New Folder", "New Folder 2"];
        assert_eq!(
            free_name("New Folder", |name| taken.contains(&name)),
            "New Folder 3"
        );
        assert_eq!(
            free_name("New File", |name| taken.contains(&name)),
            "New File"
        );
    }

    #[test]
    fn renaming_preselects_the_name_without_the_extension() {
        assert_eq!(stem_len("report.final.pdf", false), "report.final".len());
        assert_eq!(stem_len(".bashrc", false), ".bashrc".len());
        assert_eq!(stem_len("README", false), "README".len());
        assert_eq!(stem_len("photos.2024", true), "photos.2024".len());
    }

    #[test]
    fn only_cut_files_and_their_folders_are_marked() {
        let clip = |cut| FileClip {
            paths: vec![PathBuf::from("/a/one"), PathBuf::from("/b/two")],
            cut,
        };
        let (paths, folders) = cut_sets(Some(&clip(true)));
        assert!(paths.contains(Path::new("/b/two")) && !paths.contains(Path::new("/a")));
        assert!(folders.contains(Path::new("/a")) && folders.contains(Path::new("/b")));
        let (paths, folders) = cut_sets(Some(&clip(false)));
        assert!(
            paths.is_empty() && folders.is_empty(),
            "a copy marks nothing"
        );
        assert!(cut_sets(None).0.is_empty());
    }

    #[test]
    fn duplicate_names_follow_the_engine_and_avoid_each_other() {
        let listed: [&[u8]; 3] = [b"a.txt", b"a copy.txt", b"notes"];
        let made = duplicate_names(
            &[b"a.txt".to_vec(), b"notes".to_vec(), b"\xff".to_vec()],
            |name| listed.contains(&name),
        );
        assert_eq!(
            made,
            [
                b"a copy (1).txt".to_vec(),
                b"notes copy".to_vec(),
                b"\xff copy".to_vec()
            ]
        );
        // Two sources whose copies would collide get different names.
        let made = duplicate_names(&[b"x.txt".to_vec(), b"x.txt".to_vec()], |_| false);
        assert_eq!(made, [b"x copy.txt".to_vec(), b"x copy (1).txt".to_vec()]);
    }
}
