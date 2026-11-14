//! The application's file operations and their state (W-114f), on the Qt
//! thread: what is running, what waits for an answer, what just happened,
//! and the entries picked up for copying or moving. It outlives the
//! windows, so operations carry on while Rustphin runs in the background.
//!
//! All changes to files go through [`start`], which hands them to the
//! core's operation engine on its own threads (D-011).

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use cxx_qt::CxxQtThread;
use rustphin_core::{
    RealFs,
    ops::{
        Change, ConflictChoice, Finished, NoTrash, OpEvent, OpId, Operations, Progress, Recovery,
        Request, Resolution, Trash,
    },
};
use rustphin_platform::{
    clipboard::{self, FileClip, FileClipboard},
    trash::FreedesktopTrash,
};
use rustphin_view::{
    format::{count, plural},
    operations::{
        DropAction, Listed, Notice, Problems, activity, activity_icon, activity_verb, cut_sets,
        droppable, past, pick_up_notice, result_notice,
    },
    questions::Question,
};

use crate::{
    folder::qobject::FolderModel, operations::qobject::Operations as OperationsObject, post, prefs,
};

/// How long an operation may run before its progress is shown. Most finish
/// sooner and would only flash on screen.
pub const QUIET_PERIOD: Duration = Duration::from_millis(300);
/// How long a message about a finished operation stays.
const NOTICE_TIME: Duration = Duration::from_secs(6);
/// Where administrator runners number their operations from.
const ADMIN_FIRST_ID: u64 = 1 << 62;

/// An operation in progress.
pub struct Running {
    /// Its id, for cancelling.
    pub id: OpId,
    /// What it is doing: "Copying 3 items to “x”".
    pub label: String,
    /// The latest report.
    pub progress: Progress,
    done: String,
    undo: bool,
    started: Instant,
    /// The Lucide icon beside its line.
    pub icon: String,
    /// What it does to each entry, for "Couldn't copy “x”".
    pub verb: &'static str,
}

impl Running {
    /// Whether it has run long enough to be worth showing.
    pub fn is_visible(&self) -> bool {
        self.started.elapsed() >= QUIET_PERIOD
    }

    /// Counts and speed.
    pub fn detail(&self) -> String {
        rustphin_view::operations::detail(&self.progress, self.started.elapsed())
    }
}

/// The hub's state.
pub struct Hub {
    ops: Operations,
    admin_ops: Option<Operations>,
    admin_active: bool,
    #[expect(dead_code, reason = "administrator mode arrives with W-114f")]
    admin_runs: u64,
    /// How many administrator operations have finished: KDE's helper
    /// cannot watch folders, so views re-read when this changes.
    pub admin_changes: u64,
    trash: Option<Arc<FreedesktopTrash>>,
    labels: HashMap<OpId, (String, String)>,
    lasting: HashSet<OpId>,
    /// Operations in progress, oldest first.
    pub running: Vec<Running>,
    /// Open questions; the first is the one shown.
    pub questions: VecDeque<Question>,
    /// The latest message, if not yet dismissed.
    pub notice: Option<Notice>,
    /// What the operation behind the notice left out.
    pub problems: Option<Problems>,
    clipboard: Arc<dyn FileClipboard>,
    clip: Option<FileClip>,
    cut_paths: HashSet<PathBuf>,
    cut_folders: HashSet<PathBuf>,
    told_unshared: bool,
    _clipboard_watch: Option<clipboard::WatchGuard>,
    notice_serial: u64,
    reveal: HashSet<OpId>,
    /// What the latest extraction or compression made, numbered so views
    /// select it once (W-111, D-032).
    pub created: (u64, Vec<PathBuf>),
    /// Counts changes to the clipboard, so panes redraw cut entries.
    pub clip_serial: u64,
    watchers: Vec<CxxQtThread<OperationsObject>>,
    panes: Vec<CxxQtThread<FolderModel>>,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}

/// Runs `look` on the hub, starting it the first time. Only on the Qt
/// thread, once the application object exists.
pub fn with<R>(look: impl FnOnce(&mut Hub) -> R) -> R {
    HUB.with(|hub| {
        let mut hub = hub.borrow_mut();
        let hub = hub.get_or_insert_with(Hub::new);
        look(hub)
    })
}

/// Like [`with`], then tells the windows and panes that something changed.
fn change<R>(act: impl FnOnce(&mut Hub) -> R) -> R {
    let result = with(act);
    with(Hub::notify);
    result
}

/// Starts a file operation in the background.
pub fn start(request: Request) {
    change(|hub| {
        hub.active().start(request);
    });
}

/// Whether `path` was cut and is waiting to be pasted somewhere.
pub fn is_cut(path: &Path) -> bool {
    HUB.with(|hub| {
        hub.try_borrow().ok().is_some_and(|hub| {
            hub.as_ref().is_some_and(|hub| {
                path.parent()
                    .is_some_and(|folder| hub.cut_folders.contains(folder))
                    && hub.cut_paths.contains(path)
            })
        })
    })
}

/// Lets an `Operations` object follow the hub.
pub fn watch(thread: CxxQtThread<OperationsObject>) {
    with(|hub| hub.watchers.push(thread));
}

/// Lets a pane follow the clipboard and what operations make.
pub fn watch_pane(thread: CxxQtThread<FolderModel>) {
    with(|hub| hub.panes.push(thread));
}

impl Hub {
    fn new() -> Self {
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
        forward(events);
        let clipboard = clipboard::for_session();
        if !clipboard.is_shared() {
            tracing::warn!("no shared file clipboard; copy and paste stay inside Rustphin");
        }
        let watch = clipboard.watch(Box::new(|| {
            post::on_qt(refresh_clip);
        }));
        post::on_qt(refresh_clip);
        Self {
            ops,
            admin_ops: None,
            admin_active: false,
            admin_runs: 0,
            admin_changes: 0,
            trash,
            labels: HashMap::new(),
            lasting: HashSet::new(),
            running: Vec::new(),
            questions: VecDeque::new(),
            notice: None,
            problems: None,
            clipboard,
            clip: None,
            cut_paths: HashSet::new(),
            cut_folders: HashSet::new(),
            told_unshared: false,
            _clipboard_watch: watch,
            notice_serial: 0,
            reveal: HashSet::new(),
            created: (0, Vec::new()),
            clip_serial: 0,
            watchers: Vec::new(),
            panes: Vec::new(),
        }
    }

    /// Tells every window and pane that the hub changed.
    fn notify(&mut self) {
        self.watchers
            .retain(|thread| thread.queue(|object| object.refresh()).is_ok());
        self.panes
            .retain(|thread| thread.queue(|pane| pane.hub_changed()).is_ok());
    }

    fn active(&self) -> &Operations {
        match &self.admin_ops {
            Some(admin) if self.admin_active => admin,
            _ => &self.ops,
        }
    }

    fn runner_of(&self, id: OpId) -> &Operations {
        match &self.admin_ops {
            Some(admin) if id.as_u64() >= ADMIN_FIRST_ID => admin,
            _ => &self.ops,
        }
    }

    /// The desktop trash, when there is one.
    pub fn trash(&self) -> Option<Arc<FreedesktopTrash>> {
        self.trash.clone()
    }

    /// Whether anything is on the clipboard to paste.
    pub fn has_clip(&self) -> bool {
        self.clip.is_some()
    }

    /// The operation that asks the question shown, and what it does.
    pub fn verb_of(&self, id: OpId) -> &'static str {
        self.running
            .iter()
            .find(|running| running.id == id)
            .map_or("change", |running| running.verb)
    }

    fn set_clip(&mut self, clip: Option<FileClip>) {
        (self.cut_paths, self.cut_folders) = cut_sets(clip.as_ref());
        self.clip = clip;
        self.clip_serial += 1;
    }

    fn present(&mut self, text: String, is_error: bool, offers_undo: bool, expires: bool) {
        self.notice_serial += 1;
        let serial = self.notice_serial;
        self.problems = None;
        self.notice = Some(Notice {
            text,
            is_error,
            offers_undo,
        });
        if expires {
            post::after(NOTICE_TIME, move || {
                change(|hub| {
                    if hub.notice_serial == serial {
                        hub.notice = None;
                        hub.problems = None;
                    }
                });
            });
        }
    }

    fn on_event(&mut self, event: OpEvent) {
        match event {
            OpEvent::Started { id, request, undo } => {
                if !undo && matches!(request, Request::Extract { .. } | Request::Compress { .. }) {
                    self.reveal.insert(id);
                }
                let (label, done) = self
                    .labels
                    .remove(&id)
                    .unwrap_or_else(|| (activity(&request, undo), past(&request)));
                self.running.push(Running {
                    id,
                    icon: activity_icon(&request).lucide_name(),
                    verb: activity_verb(&request),
                    label,
                    done,
                    undo,
                    progress: Progress::default(),
                    started: Instant::now(),
                });
                // Shown once it has run long enough.
                post::after(QUIET_PERIOD, || change(|_| {}));
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
            OpEvent::Password { id, archive, wrong } => {
                self.questions
                    .push_back(Question::Password { id, archive, wrong });
            }
            OpEvent::Finished { id, result } => self.finish(id, result),
        }
    }

    fn finish(&mut self, id: OpId, result: Finished) {
        if self.reveal.remove(&id)
            && let Finished::Done(outcome) = &result
        {
            let made: Vec<PathBuf> = outcome
                .changes
                .iter()
                .filter_map(|change| match change {
                    Change::Copied { to, .. } => Some(to.clone()),
                    _ => None,
                })
                .collect();
            if !made.is_empty() {
                self.created = (self.created.0 + 1, made);
            }
        }
        if id.as_u64() >= ADMIN_FIRST_ID {
            self.admin_changes += 1;
        }
        let finished = self
            .running
            .iter()
            .position(|running| running.id == id)
            .map(|index| self.running.remove(index));
        // Questions it can no longer hear the answer to.
        self.questions
            .retain(|question| question.asker() != Some(id));
        let (done, undo) = finished
            .as_ref()
            .map_or(("", false), |running| (running.done.as_str(), running.undo));
        let problems = match &result {
            Finished::Done(outcome) if !outcome.skipped.is_empty() => Some(Problems {
                title: finished
                    .as_ref()
                    .map(|running| running.label.clone())
                    .unwrap_or_default(),
                entries: outcome.skipped.clone(),
            }),
            _ => None,
        };
        let lasting = self.lasting.remove(&id);
        if let Some(notice) = result_notice(done, undo, &result) {
            let expires = !notice.is_error && !lasting;
            self.present(notice.text, notice.is_error, notice.offers_undo, expires);
            self.problems = problems;
        }
    }
}

/// Hands the engine's events to the hub on the Qt thread.
fn forward(events: flume::Receiver<OpEvent>) {
    std::thread::spawn(move || {
        while let Ok(event) = events.recv() {
            post::on_qt(move || change(|hub| hub.on_event(event)));
        }
    });
}

/// Reads the clipboard again, in the background.
fn refresh_clip() {
    let clipboard = with(|hub| hub.clipboard.clone());
    std::thread::spawn(move || {
        // An unreadable clipboard holds nothing for us.
        let clip = clipboard.read().ok().flatten();
        post::on_qt(move || {
            change(|hub| {
                if hub.clip != clip {
                    hub.set_clip(clip);
                }
            });
        });
    });
}

#[expect(dead_code, reason = "administrator mode arrives with W-114f")]
/// Sends new operations through `admin` (KDE's helper, W-098) or, with
/// `None`, through the user's own filesystem.
pub fn set_admin(admin: Option<Arc<dyn rustphin_core::Fs>>) {
    change(|hub| match admin {
        Some(fs) => {
            hub.admin_runs += 1;
            let first = ADMIN_FIRST_ID + (hub.admin_runs << 40);
            let (ops, events) = Operations::with_first_id(fs, Arc::new(NoTrash), first);
            forward(events);
            hub.admin_ops = Some(ops);
            hub.admin_active = true;
        }
        None => hub.admin_active = false,
    });
}

#[expect(dead_code, reason = "rename recovery arrives with W-114f")]
/// Starts an operation the progress line and final notice describe with
/// `label` and `done`; its notice stays until dismissed.
pub fn start_lasting(request: Request, label: String, done: String) {
    change(|hub| {
        let id = hub.active().start(request);
        hub.labels.insert(id, (label, done));
        hub.lasting.insert(id);
    });
}

/// Reverts the latest operation, or says that there is none.
pub fn undo() {
    change(|hub| {
        if hub.active().undo().is_none() {
            hub.present("There is nothing to undo.".to_owned(), false, false, true);
        }
    });
}

/// Asks the operation shown in progress, or the oldest, to stop.
pub fn cancel(id: Option<OpId>) {
    with(|hub| {
        let id = id.or_else(|| hub.running.first().map(|running| running.id));
        if let Some(id) = id {
            hub.runner_of(id).cancel(id);
        }
    });
}

/// Moves `paths` to the trash, asking first when Settings says so; as
/// administrator, or in the trash, deletes for good after asking.
pub fn trash(paths: Vec<PathBuf>, listed: Vec<Listed>) {
    if paths.is_empty() {
        return;
    }
    let ask = prefs::settings().confirmations().trash;
    change(|hub| {
        if hub.admin_active {
            hub.questions
                .push_back(Question::ConfirmDelete { paths, listed });
        } else if ask {
            hub.questions
                .push_back(Question::ConfirmTrash { paths, listed });
        } else {
            hub.active().start(Request::Trash { paths });
        }
    });
}

/// Asks for confirmation, then deletes `paths` for good.
pub fn delete(paths: Vec<PathBuf>, listed: Vec<Listed>) {
    if !paths.is_empty() {
        change(|hub| {
            hub.questions
                .push_back(Question::ConfirmDelete { paths, listed });
        });
    }
}

/// Answers the question shown.
pub fn answer(
    answer: rustphin_view::questions::Answer,
    apply_to_all: bool,
    password: Option<String>,
) {
    use rustphin_view::questions::Answer as A;
    change(|hub| {
        let Some(question) = hub.questions.pop_front() else {
            return;
        };
        match (question, answer) {
            (Question::Conflict { id, .. }, answer) => {
                let choice = match answer {
                    A::Cancel => ConflictChoice::Cancel,
                    A::Skip => ConflictChoice::Skip,
                    A::Replace => ConflictChoice::Overwrite,
                    A::Accept | A::Copy | A::Open | A::Run => ConflictChoice::KeepBoth,
                };
                hub.runner_of(id).resolve(
                    id,
                    Resolution {
                        choice,
                        apply_to_all,
                    },
                );
            }
            (Question::Failed { id, .. }, answer) => {
                let recovery = match answer {
                    A::Accept => Recovery::Retry,
                    A::Skip => Recovery::Skip,
                    _ => Recovery::Cancel,
                };
                hub.runner_of(id).recover(id, recovery);
            }
            (Question::Password { id, .. }, A::Accept) => {
                hub.runner_of(id).give_password(id, password);
            }
            (Question::Password { id, .. }, _) => hub.runner_of(id).give_password(id, None),
            (Question::ConfirmDelete { paths, .. }, A::Accept) => {
                hub.active().start(Request::Delete { paths });
            }
            (Question::ConfirmTrash { paths, .. }, A::Accept) => {
                hub.active().start(Request::Trash { paths });
            }
            (Question::ConfirmEmpty { paths, .. }, A::Accept) => empty_now(hub, paths),
            (Question::Drop { sources, into }, A::Accept) => {
                hub.active().start(Request::Move { sources, into });
            }
            (Question::Drop { sources, into }, A::Copy) => {
                hub.active().start(Request::Copy { sources, into });
            }
            _ => {}
        }
    });
}

fn empty_now(hub: &mut Hub, paths: Vec<PathBuf>) {
    let id = hub.active().start(Request::Delete { paths });
    hub.labels.insert(
        id,
        (
            "Emptying the trash".to_owned(),
            "emptied the trash".to_owned(),
        ),
    );
}

/// Brings the trashed entries whose contents are `contents` back from the
/// trash directory `dir`.
pub fn restore(dir: PathBuf, contents: Vec<PathBuf>) {
    let Some(trash) = with(|hub| hub.trash()) else {
        return;
    };
    std::thread::spawn(move || {
        let found = trash.items_for(&dir, &contents);
        post::on_qt(move || {
            change(|hub| match found {
                Ok((items, without_record)) => {
                    if without_record > 0 {
                        hub.present(
                            format!(
                                "{} {} no record of where it came from and cannot be restored. Move it out by hand.",
                                count(without_record),
                                plural(without_record, "entry has", "entries have")
                            ),
                            true,
                            false,
                            false,
                        );
                    }
                    if !items.is_empty() {
                        hub.active().start(Request::Restore { items });
                    }
                }
                Err(error) => {
                    hub.present(
                        format!("The trash could not be read: {error}."),
                        true,
                        false,
                        false,
                    );
                }
            });
        });
    });
}

/// Asks for confirmation, then removes everything in the trash directory `dir`.
pub fn empty_trash(dir: PathBuf) {
    let Some(trash) = with(|hub| hub.trash()) else {
        return;
    };
    let ask = prefs::settings().confirmations().empty_trash;
    std::thread::spawn(move || {
        let found = trash
            .list_dir(&dir)
            .and_then(|entries| Ok((entries.len(), trash.everything_in(&dir)?)));
        post::on_qt(move || {
            change(|hub| match found {
                Ok((_, paths)) if paths.is_empty() => {
                    hub.present("The trash is already empty.".to_owned(), false, false, true);
                }
                Ok((entries, paths)) if ask => {
                    hub.questions
                        .push_back(Question::ConfirmEmpty { entries, paths });
                }
                Ok((_, paths)) => empty_now(hub, paths),
                Err(error) => {
                    hub.present(
                        format!("The trash could not be read: {error}."),
                        true,
                        false,
                        false,
                    );
                }
            });
        });
    });
}

/// Copies or moves dropped `sources` into `into`; asks which when `action`
/// is `None`.
pub fn drop_files(sources: Vec<PathBuf>, into: PathBuf, action: Option<DropAction>) {
    let sources = droppable(sources, &into);
    if sources.is_empty() {
        return;
    }
    change(|hub| match action {
        Some(DropAction::Copy) => {
            hub.active().start(Request::Copy { sources, into });
        }
        Some(DropAction::Move) => {
            hub.active().start(Request::Move { sources, into });
        }
        None => hub.questions.push_back(Question::Drop { sources, into }),
    });
}

/// Puts `paths` on the clipboard, to be copied or, with `cut`, moved by the
/// next paste, here or in another application.
pub fn pick_up(paths: Vec<PathBuf>, cut: bool) {
    if paths.is_empty() {
        return;
    }
    let clip = FileClip { paths, cut };
    let clipboard = with(|hub| hub.clipboard.clone());
    std::thread::spawn(move || {
        let written = clipboard.write(&clip);
        post::on_qt(move || {
            change(|hub| match written {
                Ok(()) => {
                    let text = pick_up_notice(&clip, hub.clipboard.is_shared(), hub.told_unshared);
                    hub.told_unshared = true;
                    hub.set_clip(Some(clip));
                    hub.present(text, false, false, true);
                }
                Err(error) => hub.present(
                    format!("The clipboard could not be written: {error}."),
                    true,
                    false,
                    false,
                ),
            });
        });
    });
}

/// Copies or moves the files on the clipboard into `into`.
pub fn paste(into: PathBuf) {
    let clipboard = with(|hub| hub.clipboard.clone());
    std::thread::spawn(move || {
        let read = clipboard.read();
        let clip = match read {
            Ok(Some(clip)) => clip,
            Ok(None) => {
                return say("The clipboard holds no files.".to_owned(), false);
            }
            Err(error) => {
                return say(format!("The clipboard could not be read: {error}."), true);
            }
        };
        if clip.cut {
            // A move happens once: the clipboard is emptied, as other file
            // managers do.
            let _ = clipboard.clear();
        }
        post::on_qt(move || {
            change(|hub| {
                if clip.cut {
                    hub.set_clip(None);
                    hub.active().start(Request::Move {
                        sources: clip.paths,
                        into,
                    });
                } else {
                    hub.active().start(Request::Copy {
                        sources: clip.paths,
                        into,
                    });
                }
            });
        });
    });
}

/// Shows `text` as the latest message. Any thread.
pub fn say(text: String, is_error: bool) {
    post::on_qt(move || change(|hub| hub.present(text, is_error, false, !is_error)));
}

/// Dismisses the message being shown.
pub fn dismiss_notice() {
    change(|hub| {
        hub.notice = None;
        hub.problems = None;
    });
}

/// Shows `text`, which reports no error, until [`withdraw`] is called with
/// the returned ticket or a later message replaces it.
pub fn say_until_withdrawn(text: String) -> u64 {
    change(|hub| {
        hub.present(text, false, false, false);
        hub.notice_serial
    })
}

/// Removes the message shown under `ticket`, unless a later one replaced it.
pub fn withdraw(ticket: u64) {
    change(|hub| {
        if hub.notice_serial == ticket {
            hub.notice = None;
        }
    });
}
