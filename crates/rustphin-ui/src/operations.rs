//! File operations as the interface sees them: what is running, what is
//! waiting for an answer, what just happened, and the entries picked up for
//! copying or moving.
//!
//! All changes to files go through [`Hub::start`], which hands them to the
//! core's operation engine on its own threads.

use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{App, AppContext as _, Context, Entity, Global};
use rustphin_core::{
    RealFs, display_name,
    ops::{
        Conflict, ConflictChoice, Finished, NoTrash, OpEvent, OpId, Operations, Outcome, Progress,
        Recovery, Request, Resolution, Trash,
    },
};
use rustphin_platform::{
    clipboard::{self, FileClip, FileClipboard},
    trash::FreedesktopTrash,
};

use crate::format::{self, count, plural};

/// How long an operation may run before its progress is shown. Most finish
/// sooner and would only flash on screen.
pub const QUIET_PERIOD: Duration = Duration::from_millis(300);
/// How long a message about a finished operation stays.
const NOTICE_TIME: Duration = Duration::from_secs(6);

/// An operation in progress.
pub struct Running {
    /// Its id, for cancelling.
    pub id: OpId,
    /// What it is doing, as a sentence fragment: "Copying 3 items to “x”".
    pub label: String,
    /// The latest report.
    pub progress: Progress,
    /// What it will have done, in lower case: "copied 3 items to “x”".
    done: String,
    /// Whether it reverts an earlier operation.
    undo: bool,
    started: Instant,
}

impl Running {
    /// Whether it has run long enough to be worth showing.
    pub fn is_visible(&self) -> bool {
        self.started.elapsed() >= QUIET_PERIOD
    }

    /// Done part of the whole, from 0 to 1, when the size is known.
    pub fn fraction(&self) -> Option<f32> {
        fraction(&self.progress)
    }

    /// Counts and speed: "12 of 340 · 1.2 GiB of 4.0 GiB · 85.0 MiB/s".
    pub fn detail(&self) -> String {
        detail(&self.progress, self.started.elapsed())
    }
}

/// Something an operation needs decided before it can go on.
#[derive(Debug, Clone)]
pub enum Question {
    /// A name is taken at the destination.
    Conflict {
        /// The operation that asks.
        id: OpId,
        /// What collides with what.
        conflict: Conflict,
    },
    /// An entry failed.
    Failed {
        /// The operation that asks.
        id: OpId,
        /// The entry.
        path: PathBuf,
        /// Why.
        message: String,
    },
    /// Permanent deletion needs the user's explicit yes.
    ConfirmDelete {
        /// What would be deleted.
        paths: Vec<PathBuf>,
    },
    /// Entries were dropped without a key saying whether to copy or move.
    Drop {
        /// The dropped entries.
        sources: Vec<PathBuf>,
        /// The folder they were dropped on.
        into: PathBuf,
    },
    /// Emptying the trash needs the user's explicit yes.
    ConfirmEmpty {
        /// Number of trashed entries.
        entries: usize,
        /// Everything to remove: the entries and their records.
        paths: Vec<PathBuf>,
    },
}

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

/// The application's file operations and their state.
pub struct Hub {
    ops: Operations,
    /// The desktop trash, when there is one.
    trash: Option<Arc<FreedesktopTrash>>,
    /// Wording for operations whose request does not describe them well:
    /// what it is doing, and what it will have done.
    labels: HashMap<OpId, (String, String)>,
    /// Operations in progress, oldest first.
    pub running: Vec<Running>,
    /// Open questions; the first is the one shown.
    pub questions: VecDeque<Question>,
    /// The latest message, if not yet dismissed.
    pub notice: Option<Notice>,
    /// The file clipboard: the desktop's, or a private one where the
    /// desktop offers none to share.
    clipboard: Arc<dyn FileClipboard>,
    /// The files on the clipboard as last read: ours or another application's.
    pub clip: Option<FileClip>,
    /// Whether the user has been told that the clipboard is not shared.
    told_unshared: bool,
    _clipboard_watch: Option<clipboard::WatchGuard>,
    /// Counts notices, so a timer only dismisses the one it was set for.
    notice_serial: u64,
}

struct GlobalHub(Entity<Hub>);

impl Global for GlobalHub {}

/// Creates the application's [`Hub`]. Call once at start-up.
pub fn init(cx: &mut App) {
    let fs = Arc::new(RealFs);
    let trash = FreedesktopTrash::for_user(fs.clone()).map(Arc::new);
    let engine_trash: Arc<dyn Trash> = match &trash {
        Some(trash) => trash.clone(),
        None => {
            tracing::warn!("no data directory is known, so there is no trash");
            Arc::new(NoTrash)
        }
    };
    let (ops, events) = Operations::new(fs, engine_trash);
    let clipboard = clipboard::for_session();
    if !clipboard.is_shared() {
        tracing::warn!("no shared file clipboard; copy and paste stay inside Rustphin");
    }
    let hub = cx.new(|cx: &mut Context<Hub>| {
        // Follow what other applications put on the clipboard.
        let (changed, changes) = flume::unbounded::<()>();
        let watch = clipboard.watch(Box::new(move || {
            let _ = changed.send(());
        }));
        cx.spawn(async move |hub, cx| {
            while changes.recv_async().await.is_ok() {
                // A copy announces itself once per format; read once.
                while changes.try_recv().is_ok() {}
                if hub.update(cx, |hub, cx| hub.refresh_clip(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |hub, cx| {
            while let Ok(event) = events.recv_async().await {
                if hub.update(cx, |hub, cx| hub.on_event(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Hub {
            ops,
            trash,
            labels: HashMap::new(),
            running: Vec::new(),
            questions: VecDeque::new(),
            notice: None,
            clipboard,
            clip: None,
            told_unshared: false,
            _clipboard_watch: watch,
            notice_serial: 0,
        }
    });
    cx.set_global(GlobalHub(hub));
}

/// The application's [`Hub`].
pub fn hub(cx: &App) -> Entity<Hub> {
    cx.global::<GlobalHub>().0.clone()
}

impl Hub {
    /// Starts a file operation in the background.
    pub fn start(&mut self, request: Request) {
        self.ops.start(request);
    }

    /// Reverts the latest operation, or says that there is none.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.ops.undo().is_none() {
            self.set_notice("There is nothing to undo.".to_owned(), false, cx);
        }
    }

    /// Asks operation `id` to stop.
    pub fn cancel(&mut self, id: OpId) {
        self.ops.cancel(id);
    }

    /// Asks for confirmation, then deletes `paths` for good.
    pub fn delete_after_confirming(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if !paths.is_empty() {
            self.questions.push_back(Question::ConfirmDelete { paths });
            cx.notify();
        }
    }

    /// Answers the conflict being shown.
    pub fn resolve(&mut self, choice: ConflictChoice, apply_to_all: bool, cx: &mut Context<Self>) {
        if let Some(Question::Conflict { id, .. }) = self.questions.front() {
            self.ops.resolve(
                *id,
                Resolution {
                    choice,
                    apply_to_all,
                },
            );
            self.questions.pop_front();
            cx.notify();
        }
    }

    /// Answers the failure being shown.
    pub fn recover(&mut self, recovery: Recovery, cx: &mut Context<Self>) {
        if let Some(Question::Failed { id, .. }) = self.questions.front() {
            self.ops.recover(*id, recovery);
            self.questions.pop_front();
            cx.notify();
        }
    }

    /// Answers the delete confirmation being shown.
    pub fn confirm_delete(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if let Some(Question::ConfirmDelete { .. }) = self.questions.front()
            && let Some(Question::ConfirmDelete { paths }) = self.questions.pop_front()
            && confirmed
        {
            self.start(Request::Delete { paths });
        }
        cx.notify();
    }

    /// The desktop trash, when there is one.
    pub fn trash(&self) -> Option<Arc<FreedesktopTrash>> {
        self.trash.clone()
    }

    /// Brings the trashed entries whose contents are `contents` back from
    /// the trash directory `dir`.
    pub fn restore(&mut self, dir: PathBuf, contents: Vec<PathBuf>, cx: &mut Context<Self>) {
        let Some(trash) = self.trash() else { return };
        cx.spawn(async move |hub, cx| {
            let found = cx
                .background_spawn(async move { trash.items_for(&dir, &contents) })
                .await;
            let _ = hub.update(cx, |hub, cx| match found {
                Ok((items, without_record)) => {
                    if without_record > 0 {
                        hub.set_notice(
                            format!(
                                "{} {} no record of where it came from and cannot be restored. Move it out by hand.",
                                count(without_record),
                                plural(without_record, "entry has", "entries have")
                            ),
                            true,
                            cx,
                        );
                    }
                    if !items.is_empty() {
                        hub.start(Request::Restore { items });
                    }
                }
                Err(error) => hub.set_notice(format!("The trash could not be read: {error}."), true, cx),
            });
        })
        .detach();
    }

    /// Asks for confirmation, then removes everything in the trash directory `dir`.
    pub fn empty_trash(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let Some(trash) = self.trash() else { return };
        cx.spawn(async move |hub, cx| {
            let found = cx
                .background_spawn(async move {
                    let entries = trash.list_dir(&dir)?.len();
                    Ok::<_, std::io::Error>((entries, trash.everything_in(&dir)?))
                })
                .await;
            let _ = hub.update(cx, |hub, cx| match found {
                Ok((_, paths)) if paths.is_empty() => {
                    hub.set_notice("The trash is already empty.".to_owned(), false, cx);
                }
                Ok((entries, paths)) => {
                    hub.questions
                        .push_back(Question::ConfirmEmpty { entries, paths });
                    cx.notify();
                }
                Err(error) => {
                    hub.set_notice(format!("The trash could not be read: {error}."), true, cx)
                }
            });
        })
        .detach();
    }

    /// Answers the empty-trash confirmation being shown.
    pub fn confirm_empty(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if let Some(Question::ConfirmEmpty { .. }) = self.questions.front()
            && let Some(Question::ConfirmEmpty { paths, .. }) = self.questions.pop_front()
            && confirmed
        {
            let id = self.ops.start(Request::Delete { paths });
            self.labels.insert(
                id,
                (
                    "Emptying the trash".to_owned(),
                    "emptied the trash".to_owned(),
                ),
            );
        }
        cx.notify();
    }

    /// Copies or moves dropped `sources` into `into`; asks which when
    /// `action` is `None`. Entries that are already there, or that contain
    /// the destination, are left out without a word.
    pub fn drop_files(
        &mut self,
        sources: Vec<PathBuf>,
        into: PathBuf,
        action: Option<DropAction>,
        cx: &mut Context<Self>,
    ) {
        let sources = droppable(sources, &into);
        if sources.is_empty() {
            return;
        }
        match action {
            Some(DropAction::Copy) => self.start(Request::Copy { sources, into }),
            Some(DropAction::Move) => self.start(Request::Move { sources, into }),
            None => self.questions.push_back(Question::Drop { sources, into }),
        }
        cx.notify();
    }

    /// Answers the drop question being shown; `None` cancels the drop.
    pub fn answer_drop(&mut self, action: Option<DropAction>, cx: &mut Context<Self>) {
        if let Some(Question::Drop { .. }) = self.questions.front()
            && let Some(Question::Drop { sources, into }) = self.questions.pop_front()
        {
            match action {
                Some(DropAction::Copy) => self.start(Request::Copy { sources, into }),
                Some(DropAction::Move) => self.start(Request::Move { sources, into }),
                None => {}
            }
        }
        cx.notify();
    }

    /// Reads the clipboard again, in the background.
    fn refresh_clip(&mut self, cx: &mut Context<Self>) {
        let clipboard = self.clipboard.clone();
        cx.spawn(async move |hub, cx| {
            let read = cx.background_spawn(async move { clipboard.read() }).await;
            let _ = hub.update(cx, |hub, cx| {
                // An unreadable clipboard holds nothing for us.
                let clip = read.ok().flatten();
                if hub.clip != clip {
                    hub.clip = clip;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Whether `path` was cut and is waiting to be pasted somewhere.
    pub fn is_cut(&self, path: &Path) -> bool {
        self.clip
            .as_ref()
            .is_some_and(|clip| clip.cut && clip.paths.iter().any(|cut| cut == path))
    }

    /// Whether anything in the folder `folder` is cut. Cheap enough to ask
    /// before checking every row.
    pub fn has_cut_in(&self, folder: &Path) -> bool {
        self.clip.as_ref().is_some_and(|clip| {
            clip.cut && clip.paths.iter().any(|cut| cut.parent() == Some(folder))
        })
    }

    /// Puts `paths` on the clipboard, to be copied or, with `cut`, moved by
    /// the next paste, here or in another application.
    pub fn pick_up(&mut self, paths: Vec<PathBuf>, cut: bool, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        let clip = FileClip { paths, cut };
        let clipboard = self.clipboard.clone();
        cx.spawn(async move |hub, cx| {
            let written = {
                let clip = clip.clone();
                cx.background_spawn(async move { clipboard.write(&clip) })
                    .await
            };
            let _ = hub.update(cx, |hub, cx| match written {
                Ok(()) => {
                    let text = pick_up_notice(&clip, hub.clipboard.is_shared(), hub.told_unshared);
                    hub.told_unshared = true;
                    hub.clip = Some(clip);
                    hub.set_notice(text, false, cx);
                }
                Err(error) => {
                    hub.set_notice(
                        format!("The clipboard could not be written: {error}."),
                        true,
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    /// Copies or moves the files on the clipboard into `into`.
    pub fn paste(&mut self, into: PathBuf, cx: &mut Context<Self>) {
        let clipboard = self.clipboard.clone();
        cx.spawn(async move |hub, cx| {
            let read = {
                let clipboard = clipboard.clone();
                cx.background_spawn(async move { clipboard.read() }).await
            };
            let clip = match read {
                Ok(Some(clip)) => clip,
                Ok(None) => {
                    let _ = hub.update(cx, |hub, cx| {
                        hub.set_notice("The clipboard holds no files.".to_owned(), false, cx);
                    });
                    return;
                }
                Err(error) => {
                    let _ = hub.update(cx, |hub, cx| {
                        hub.set_notice(
                            format!("The clipboard could not be read: {error}."),
                            true,
                            cx,
                        );
                    });
                    return;
                }
            };
            if clip.cut {
                // A move happens once: the entries are no longer where the
                // clipboard says, so it is emptied, as other file managers do.
                let _ = cx.background_spawn(async move { clipboard.clear() }).await;
            }
            let _ = hub.update(cx, |hub, cx| {
                if clip.cut {
                    hub.clip = None;
                    hub.start(Request::Move {
                        sources: clip.paths,
                        into,
                    });
                } else {
                    hub.start(Request::Copy {
                        sources: clip.paths,
                        into,
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dismisses the message being shown.
    pub fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.notice = None;
        cx.notify();
    }

    fn set_notice(&mut self, text: String, is_error: bool, cx: &mut Context<Self>) {
        self.show(text, is_error, false, cx);
    }

    fn show(&mut self, text: String, is_error: bool, offers_undo: bool, cx: &mut Context<Self>) {
        self.notice_serial += 1;
        let serial = self.notice_serial;
        self.notice = Some(Notice {
            text,
            is_error,
            offers_undo,
        });
        if !is_error {
            cx.spawn(async move |hub, cx| {
                cx.background_executor().timer(NOTICE_TIME).await;
                let _ = hub.update(cx, |hub, cx| {
                    if hub.notice_serial == serial {
                        hub.notice = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    fn on_event(&mut self, event: OpEvent, cx: &mut Context<Self>) {
        match event {
            OpEvent::Started { id, request, undo } => {
                let (label, done) = self
                    .labels
                    .remove(&id)
                    .unwrap_or_else(|| (activity(&request, undo), past(&request)));
                self.running.push(Running {
                    id,
                    label,
                    done,
                    undo,
                    progress: Progress::default(),
                    started: Instant::now(),
                });
                // Redraw when it has run long enough to be shown.
                cx.spawn(async move |hub, cx| {
                    cx.background_executor().timer(QUIET_PERIOD).await;
                    let _ = hub.update(cx, |_, cx| cx.notify());
                })
                .detach();
            }
            OpEvent::Progress { id, progress } => {
                if let Some(running) = self.running.iter_mut().find(|running| running.id == id) {
                    running.progress = progress;
                }
            }
            OpEvent::Conflict { id, conflict } => {
                self.questions
                    .push_back(Question::Conflict { id, conflict });
            }
            OpEvent::Failed { id, path, message } => {
                self.questions
                    .push_back(Question::Failed { id, path, message });
            }
            OpEvent::Finished { id, result } => {
                let finished = self
                    .running
                    .iter()
                    .position(|running| running.id == id)
                    .map(|index| self.running.remove(index));
                // Questions it can no longer hear the answer to.
                self.questions.retain(|question| match question {
                    Question::Conflict { id: asker, .. } | Question::Failed { id: asker, .. } => {
                        *asker != id
                    }
                    Question::ConfirmDelete { .. }
                    | Question::ConfirmEmpty { .. }
                    | Question::Drop { .. } => true,
                });
                let (done, undo) = finished
                    .as_ref()
                    .map_or(("", false), |running| (running.done.as_str(), running.undo));
                if let Some(notice) = result_notice(done, undo, &result) {
                    self.show(notice.text, notice.is_error, notice.offers_undo, cx);
                }
            }
        }
        cx.notify();
    }
}

/// The entries of a drop that can go into `into`: not those already in it,
/// not the folder itself and not a folder that contains it.
fn droppable(sources: Vec<PathBuf>, into: &Path) -> Vec<PathBuf> {
    sources
        .into_iter()
        .filter(|source| source.parent() != Some(into) && !into.starts_with(source))
        .collect()
}

/// What to say after a copy or cut. The first time on a desktop without a
/// shared clipboard, also that other applications will not see it.
fn pick_up_notice(clip: &FileClip, shared: bool, told_unshared: bool) -> String {
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

fn quoted(name: &std::ffi::OsStr) -> String {
    format!("“{}”", display_name(name.as_encoded_bytes()))
}

fn folder_name(path: &Path) -> String {
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

fn originals(items: &[rustphin_core::ops::TrashedItem]) -> Vec<PathBuf> {
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
        Request::CreateFile { name, .. } => format!("Creating file {}", quoted(name)),
        Request::Rename { path, new_name } => {
            format!("Renaming {} to {}", folder_name(path), quoted(new_name))
        }
        Request::Copy { sources, into } => {
            format!("Copying {} to {}", what(sources), folder_name(into))
        }
        Request::Move { sources, into } => {
            format!("Moving {} to {}", what(sources), folder_name(into))
        }
        Request::Trash { paths } => format!("Moving {} to the trash", what(paths)),
        Request::Restore { items } => format!("Restoring {}", what(&originals(items))),
        Request::Delete { paths } => format!("Deleting {}", what(paths)),
    }
}

/// What a request did, in lower case: "copied 3 items to “x”".
pub fn past(request: &Request) -> String {
    match request {
        Request::CreateDir { name, .. } => format!("created folder {}", quoted(name)),
        Request::CreateFile { name, .. } => format!("created file {}", quoted(name)),
        Request::Rename { path, new_name } => {
            format!("renamed {} to {}", folder_name(path), quoted(new_name))
        }
        Request::Copy { sources, into } => {
            format!("copied {} to {}", what(sources), folder_name(into))
        }
        Request::Move { sources, into } => {
            format!("moved {} to {}", what(sources), folder_name(into))
        }
        Request::Trash { paths } => format!("moved {} to the trash", what(paths)),
        Request::Restore { items } => {
            format!("restored {} from the trash", what(&originals(items)))
        }
        Request::Delete { paths } => format!("deleted {}", what(paths)),
    }
}

/// The message for a finished operation, if it deserves one. `done` says
/// what the operation did, in lower case; `undo` whether it was a revert.
/// Creating and renaming succeed silently: the result is visible in the list.
fn result_notice(done: &str, undo: bool, result: &Finished) -> Option<Notice> {
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

fn outcome_notice(done: &str, outcome: &Outcome) -> Option<Notice> {
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

fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn fraction(progress: &Progress) -> Option<f32> {
    if progress.bytes_total > 0 {
        Some((progress.bytes_done as f64 / progress.bytes_total as f64) as f32)
    } else if progress.items_total > 0 {
        Some((progress.items_done as f64 / progress.items_total as f64) as f32)
    } else {
        None
    }
}

fn detail(progress: &Progress, elapsed: Duration) -> String {
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

/// A free name like `base`, `base 2`, `base 3`, … given a test for taken names.
pub fn free_name(base: &str, is_taken: impl Fn(&str) -> bool) -> String {
    if !is_taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !is_taken(name))
        .unwrap_or_else(|| base.to_owned())
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
        let notice = result_notice("moved 9 items to “x”", false, &done(false, &[])).unwrap();
        assert_eq!(
            notice.text,
            "Stopped part-way: moved 9 items to “x”. What was done stays."
        );
        assert!(!notice.is_error);
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
}
