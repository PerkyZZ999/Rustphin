//! File operations as the interface sees them: what is running, what is
//! waiting for an answer, what just happened, and the entries picked up for
//! copying or moving.
//!
//! All changes to files go through [`Hub::start`], which hands them to the
//! core's operation engine on its own threads.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{App, AppContext as _, Context, Entity, Global};
use rustphin_core::{
    RealFs,
    ops::{
        Conflict, ConflictChoice, Finished, NoTrash, OpEvent, OpId, Operations, Progress, Recovery,
        Request, Resolution, Trash,
    },
};
use rustphin_platform::{
    clipboard::{self, FileClip, FileClipboard},
    trash::FreedesktopTrash,
};

use crate::format::{count, plural};

pub use rustphin_view::operations::*;

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
    /// The icon beside its line.
    pub icon: gpui_kit::assets::IconName,
    /// What it does to each entry, for "Couldn't copy “x”".
    pub verb: &'static str,
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
    /// An archive is encrypted (W-111e).
    Password {
        /// The operation that asks.
        id: OpId,
        /// The archive.
        archive: PathBuf,
        /// The password given before was not right.
        wrong: bool,
    },
    /// Permanent deletion needs the user's explicit yes.
    ConfirmDelete {
        /// What would be deleted.
        paths: Vec<PathBuf>,
        /// What the listing knew of each, in the same order.
        listed: Vec<Listed>,
    },
    /// Entries were dropped without a key saying whether to copy or move.
    Drop {
        /// The dropped entries.
        sources: Vec<PathBuf>,
        /// The folder they were dropped on.
        into: PathBuf,
    },
    /// Moving to the trash, when Settings asks to be asked (W-106e).
    ConfirmTrash {
        /// What would be moved.
        paths: Vec<PathBuf>,
        /// What the listing knew of each, in the same order.
        listed: Vec<Listed>,
    },
    /// Emptying the trash needs the user's explicit yes.
    ConfirmEmpty {
        /// Number of trashed entries.
        entries: usize,
        /// Everything to remove: the entries and their records.
        paths: Vec<PathBuf>,
    },
}

/// The application's file operations and their state.
pub struct Hub {
    ops: Operations,
    /// While the window acts as administrator: the runner that works
    /// through KDE's helper (W-098). Its operations are numbered from
    /// [`ADMIN_FIRST_ID`] up, so the two runners' ids never meet.
    admin_ops: Option<Operations>,
    /// Whether new operations go to `admin_ops`.
    admin_active: bool,
    /// How many administrator runners there have been.
    admin_runs: u64,
    /// How many administrator operations have finished. KDE's helper
    /// cannot watch folders, so views re-read when this changes.
    admin_changes: u64,
    /// The desktop trash, when there is one.
    trash: Option<Arc<FreedesktopTrash>>,
    /// Wording for operations whose request does not describe them well:
    /// what it is doing, and what it will have done.
    labels: HashMap<OpId, (String, String)>,
    /// Operations whose final notice stays until dismissed, such as a
    /// recovery at startup that the user may not be watching for.
    lasting: HashSet<OpId>,
    /// Operations in progress, oldest first.
    pub running: Vec<Running>,
    /// Open questions; the first is the one shown.
    pub questions: VecDeque<Question>,
    /// The latest message, if not yet dismissed.
    pub notice: Option<Notice>,
    /// The entries the operation behind [`Self::notice`] left out, with what
    /// it was doing, for the error details dialog. Empty otherwise.
    pub problems: Option<Problems>,
    /// The file clipboard: the desktop's, or a private one where the
    /// desktop offers none to share.
    clipboard: Arc<dyn FileClipboard>,
    /// The files on the clipboard as last read: ours or another application's.
    clip: Option<FileClip>,
    /// The cut paths and their folders, kept with `clip` so each row asks
    /// a set rather than the whole clipboard on every frame.
    cut_paths: HashSet<PathBuf>,
    cut_folders: HashSet<PathBuf>,
    /// Whether the user has been told that the clipboard is not shared.
    told_unshared: bool,
    _clipboard_watch: Option<clipboard::WatchGuard>,
    /// Counts notices, so a timer only dismisses the one it was set for.
    notice_serial: u64,
    /// Extractions and compressions whose results are selected when done.
    reveal: HashSet<OpId>,
    /// What the latest of them made, numbered so views select it once
    /// (W-111, D-032: the result is selected, no folder opens).
    pub created: (u64, Vec<PathBuf>),
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
        }
    });
    cx.set_global(GlobalHub(hub));
}

/// The application's [`Hub`].
pub fn hub(cx: &App) -> Entity<Hub> {
    cx.global::<GlobalHub>().0.clone()
}

/// Where administrator runners number their operations from.
const ADMIN_FIRST_ID: u64 = 1 << 62;

impl Hub {
    /// Sends new operations through `admin` (KDE's helper, W-098) or, with
    /// `None`, through the user's own filesystem. Operations already running
    /// on the other runner carry on. As administrator there is no trash.
    pub fn set_admin(&mut self, admin: Option<Arc<dyn rustphin_core::Fs>>, cx: &mut Context<Self>) {
        match admin {
            Some(fs) => {
                self.admin_runs += 1;
                let first = ADMIN_FIRST_ID + (self.admin_runs << 40);
                let (ops, events) = Operations::with_first_id(fs, Arc::new(NoTrash), first);
                cx.spawn(async move |hub, cx| {
                    while let Ok(event) = events.recv_async().await {
                        if hub.update(cx, |hub, cx| hub.on_event(event, cx)).is_err() {
                            break;
                        }
                    }
                })
                .detach();
                self.admin_ops = Some(ops);
                self.admin_active = true;
            }
            None => self.admin_active = false,
        }
    }

    /// The runner new operations go to.
    fn active(&self) -> &Operations {
        match &self.admin_ops {
            Some(admin) if self.admin_active => admin,
            _ => &self.ops,
        }
    }

    /// The runner that numbered operation `id`.
    /// How many administrator operations have finished so far.
    pub fn admin_changes(&self) -> u64 {
        self.admin_changes
    }

    fn runner_of(&self, id: OpId) -> &Operations {
        match &self.admin_ops {
            Some(admin) if id.as_u64() >= ADMIN_FIRST_ID => admin,
            _ => &self.ops,
        }
    }

    /// Starts a file operation in the background.
    pub fn start(&mut self, request: Request) {
        self.active().start(request);
    }

    /// Starts a file operation that the progress line and the final notice
    /// describe with `label` and, in lower case, `done`; that notice stays
    /// until dismissed.
    pub fn start_lasting(&mut self, request: Request, label: String, done: String) {
        let id = self.active().start(request);
        self.labels.insert(id, (label, done));
        self.lasting.insert(id);
    }

    /// Reverts the latest operation, or says that there is none.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.active().undo().is_none() {
            self.say("There is nothing to undo.".to_owned(), false, cx);
        }
    }

    /// Asks operation `id` to stop.
    pub fn cancel(&mut self, id: OpId) {
        self.runner_of(id).cancel(id);
    }

    /// Asks for confirmation, then deletes `paths` for good.
    pub fn delete_after_confirming(
        &mut self,
        paths: Vec<PathBuf>,
        listed: Vec<Listed>,
        cx: &mut Context<Self>,
    ) {
        if !paths.is_empty() {
            self.questions
                .push_back(Question::ConfirmDelete { paths, listed });
            cx.notify();
        }
    }

    /// Moves `paths` to the trash, first asking when `ask` (Settings,
    /// Confirmations).
    pub fn trash_after_confirming(
        &mut self,
        paths: Vec<PathBuf>,
        listed: Vec<Listed>,
        ask: bool,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        if ask {
            self.questions
                .push_back(Question::ConfirmTrash { paths, listed });
        } else {
            self.start(Request::Trash { paths });
        }
        cx.notify();
    }

    /// Answers the trash confirmation being shown.
    pub fn confirm_trash(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if let Some(Question::ConfirmTrash { .. }) = self.questions.front()
            && let Some(Question::ConfirmTrash { paths, .. }) = self.questions.pop_front()
            && confirmed
        {
            self.start(Request::Trash { paths });
        }
        cx.notify();
    }

    /// Answers the conflict being shown.
    pub fn resolve(&mut self, choice: ConflictChoice, apply_to_all: bool, cx: &mut Context<Self>) {
        if let Some(Question::Conflict { id, .. }) = self.questions.front() {
            self.runner_of(*id).resolve(
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
            self.runner_of(*id).recover(*id, recovery);
            self.questions.pop_front();
            cx.notify();
        }
    }

    /// Answers the password question being shown; `None` stops the
    /// operation. The password is handed on and not kept here.
    pub fn give_password(&mut self, password: Option<String>, cx: &mut Context<Self>) {
        if let Some(Question::Password { id, .. }) = self.questions.front() {
            self.runner_of(*id).give_password(*id, password);
            self.questions.pop_front();
            cx.notify();
        }
    }

    /// Answers the delete confirmation being shown.
    pub fn confirm_delete(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if let Some(Question::ConfirmDelete { .. }) = self.questions.front()
            && let Some(Question::ConfirmDelete { paths, .. }) = self.questions.pop_front()
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
                        hub.say(
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
                Err(error) => hub.say(format!("The trash could not be read: {error}."), true, cx),
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
                    hub.say("The trash is already empty.".to_owned(), false, cx);
                }
                Ok((entries, paths)) => {
                    let ask = crate::preferences::preferences(cx)
                        .read(cx)
                        .settings()
                        .confirmations()
                        .empty_trash;
                    if ask {
                        hub.questions
                            .push_back(Question::ConfirmEmpty { entries, paths });
                    } else {
                        hub.empty_now(paths);
                    }
                    cx.notify();
                }
                Err(error) => hub.say(format!("The trash could not be read: {error}."), true, cx),
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
            self.empty_now(paths);
        }
        cx.notify();
    }

    /// Removes everything in the trash: `paths`, entries and records.
    fn empty_now(&mut self, paths: Vec<PathBuf>) {
        let id = self.active().start(Request::Delete { paths });
        self.labels.insert(
            id,
            (
                "Emptying the trash".to_owned(),
                "emptied the trash".to_owned(),
            ),
        );
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
                    hub.set_clip(clip);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Whether `path` was cut and is waiting to be pasted somewhere.
    pub fn is_cut(&self, path: &Path) -> bool {
        self.cut_paths.contains(path)
    }

    /// Whether anything in the folder `folder` is cut. Cheap enough to ask
    /// before checking every row.
    pub fn has_cut_in(&self, folder: &Path) -> bool {
        self.cut_folders.contains(folder)
    }

    /// The files on the clipboard as last read.
    pub fn clip(&self) -> Option<&FileClip> {
        self.clip.as_ref()
    }

    /// Replaces what is known of the clipboard, and the cut sets with it.
    fn set_clip(&mut self, clip: Option<FileClip>) {
        (self.cut_paths, self.cut_folders) = cut_sets(clip.as_ref());
        self.clip = clip;
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
                    hub.set_clip(Some(clip));
                    hub.say(text, false, cx);
                }
                Err(error) => {
                    hub.say(
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
                        hub.say("The clipboard holds no files.".to_owned(), false, cx);
                    });
                    return;
                }
                Err(error) => {
                    let _ = hub.update(cx, |hub, cx| {
                        hub.say(
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
                    hub.set_clip(None);
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

    /// Shows `text` as the latest message.
    pub fn say(&mut self, text: String, is_error: bool, cx: &mut Context<Self>) {
        self.show(text, is_error, false, cx);
    }

    /// Shows `text`, which does not report an error, until [`Hub::withdraw`]
    /// is called with the returned ticket or a later message replaces it.
    pub fn say_until_withdrawn(&mut self, text: String, cx: &mut Context<Self>) -> u64 {
        self.present(text, false, false, false, cx)
    }

    /// Removes the message shown under `ticket`, unless a later one replaced it.
    pub fn withdraw(&mut self, ticket: u64, cx: &mut Context<Self>) {
        if self.notice_serial == ticket && self.notice.is_some() {
            self.notice = None;
            cx.notify();
        }
    }

    /// Dismisses the message being shown.
    pub fn dismiss_notice(&mut self, cx: &mut Context<Self>) {
        self.notice = None;
        self.problems = None;
        cx.notify();
    }

    fn show(&mut self, text: String, is_error: bool, offers_undo: bool, cx: &mut Context<Self>) {
        self.present(text, is_error, offers_undo, !is_error, cx);
    }

    /// Shows a message; `expires` hides it after [`NOTICE_TIME`]. Returns its ticket.
    fn present(
        &mut self,
        text: String,
        is_error: bool,
        offers_undo: bool,
        expires: bool,
        cx: &mut Context<Self>,
    ) -> u64 {
        self.notice_serial += 1;
        let serial = self.notice_serial;
        self.problems = None;
        self.notice = Some(Notice {
            text,
            is_error,
            offers_undo,
        });
        if expires {
            cx.spawn(async move |hub, cx| {
                cx.background_executor().timer(NOTICE_TIME).await;
                let _ = hub.update(cx, |hub, cx| {
                    if hub.notice_serial == serial {
                        hub.notice = None;
                        hub.problems = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
        serial
    }

    fn on_event(&mut self, event: OpEvent, cx: &mut Context<Self>) {
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
                    icon: crate::icon_map::gpui_icon(activity_icon(&request)),
                    verb: activity_verb(&request),
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
            OpEvent::Password { id, archive, wrong } => {
                self.questions
                    .push_back(Question::Password { id, archive, wrong });
            }
            OpEvent::Finished { id, result } => {
                if self.reveal.remove(&id)
                    && let Finished::Done(outcome) = &result
                {
                    let made: Vec<PathBuf> = outcome
                        .changes
                        .iter()
                        .filter_map(|change| match change {
                            rustphin_core::ops::Change::Copied { to, .. } => Some(to.clone()),
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
                self.questions.retain(|question| match question {
                    Question::Conflict { id: asker, .. }
                    | Question::Failed { id: asker, .. }
                    | Question::Password { id: asker, .. } => *asker != id,
                    Question::ConfirmDelete { .. }
                    | Question::ConfirmTrash { .. }
                    | Question::ConfirmEmpty { .. }
                    | Question::Drop { .. } => true,
                });
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
                    self.present(
                        notice.text,
                        notice.is_error,
                        notice.offers_undo,
                        expires,
                        cx,
                    );
                    self.problems = problems;
                }
            }
        }
        cx.notify();
    }
}
