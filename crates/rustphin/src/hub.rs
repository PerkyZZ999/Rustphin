//! The application's file operations on the Qt thread (W-114f, W-124):
//! the engine's runners, the user's own and the administrator's, the
//! clipboard, and the timers. What is shown and decided lives in
//! `rustphin-view`'s [`OperationBoard`]; the hub feeds it the engine's
//! events and acts on what it returns. It outlives the windows, so
//! operations carry on while Rustphin runs in the background.
//!
//! All changes to files go through [`start`], which hands them to the
//! core's operation engine on its own threads (D-011).

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use cxx_qt::CxxQtThread;
use rustphin_core::{
    RealFs,
    ops::{NoTrash, OpEvent, OpId, Operations, Request},
};
use rustphin_platform::{
    clipboard::{self, FileClip, FileClipboard},
    trash::FreedesktopTrash,
};
use rustphin_view::{
    format::{count, plural},
    operation_board::{
        Expiry, FollowUp, Launch, OperationBoard, Reply, TrashEmptying, Words, admin_first_id,
        is_admin,
    },
    operations::{DropAction, Listed, droppable},
    selection_actions::HandOff,
};

use crate::{
    folder::qobject::FolderModel, operations::qobject::Operations as OperationsObject, post, prefs,
};

/// The hub's state: the engine's runners and the board they report to.
pub struct Hub {
    ops: Operations,
    admin_ops: Option<Operations>,
    admin_active: bool,
    admin_runs: u64,
    trash: Option<Arc<FreedesktopTrash>>,
    /// What the operations show, and what the clipboard holds.
    pub board: OperationBoard,
    /// Dialogs a pane asked for, until the window's dialog takes them.
    pub dialogs: HandOff,
    clipboard: Arc<dyn FileClipboard>,
    _clipboard_watch: Option<clipboard::WatchGuard>,
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
        hub.try_borrow()
            .ok()
            .is_some_and(|hub| hub.as_ref().is_some_and(|hub| hub.board.is_cut(path)))
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

/// Takes the board's notice away once `expiry` has passed, unless a newer
/// one replaced it.
fn schedule(expiry: Option<Expiry>) {
    if let Some(expiry) = expiry {
        post::after(expiry.after, move || change(|hub| hub.board.expire(expiry)));
    }
}

impl Hub {
    fn new() -> Self {
        let fs = Arc::new(RealFs);
        let trash = FreedesktopTrash::for_user(fs.clone()).map(Arc::new);
        let engine_trash: Arc<dyn rustphin_core::ops::Trash> = match &trash {
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
            trash,
            board: OperationBoard::new(),
            dialogs: HandOff::default(),
            clipboard,
            _clipboard_watch: watch,
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
            Some(admin) if is_admin(id) => admin,
            _ => &self.ops,
        }
    }

    /// Starts `launch` through the active runner, with its words.
    fn launch(&mut self, launch: Launch) {
        let id = self.active().start(launch.request);
        if let Some(words) = launch.words {
            self.board.describe(id, words);
        }
    }

    /// The desktop trash, when there is one.
    pub fn trash(&self) -> Option<Arc<FreedesktopTrash>> {
        self.trash.clone()
    }

    fn on_event(&mut self, event: OpEvent) {
        let FollowUp {
            redraw_after,
            expiry,
        } = self.board.on_event(event, Instant::now());
        if let Some(delay) = redraw_after {
            // Shown once it has run long enough.
            post::after(delay, || change(|_| {}));
        }
        schedule(expiry);
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
                hub.board.clip_read(clip);
            });
        });
    });
}

/// Sends new operations through `admin` (KDE's helper, W-098) or, with
/// `None`, through the user's own filesystem.
pub fn set_admin(admin: Option<Arc<dyn rustphin_core::Fs>>) {
    change(|hub| match admin {
        Some(fs) => {
            hub.admin_runs += 1;
            let first = admin_first_id(hub.admin_runs);
            let (ops, events) = Operations::with_first_id(fs, Arc::new(NoTrash), first);
            forward(events);
            hub.admin_ops = Some(ops);
            hub.admin_active = true;
        }
        None => hub.admin_active = false,
    });
}

/// Tells the windows that administrator access is being asked for.
pub fn set_admin_pending() {
    change(|_| {});
}

/// Starts an operation the progress line and final notice describe with
/// `label` and `done`; its notice stays until dismissed.
pub fn start_lasting(request: Request, label: String, done: String) {
    change(|hub| {
        hub.launch(Launch {
            request,
            words: Some(Words {
                label,
                done,
                lasting: true,
            }),
        });
    });
}

/// Reverts the latest operation, or says that there is none.
pub fn undo() {
    change(|hub| {
        if hub.active().undo().is_none() {
            schedule(hub.board.nothing_to_undo());
        }
    });
}

/// Asks the operation shown in progress, or the oldest, to stop.
pub fn cancel(id: Option<OpId>) {
    with(|hub| {
        if let Some(id) = hub.board.to_cancel(id) {
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
        let admin = hub.admin_active;
        if let Some(launch) = hub.board.trash(paths, listed, ask, admin) {
            hub.launch(launch);
        }
    });
}

/// Asks for confirmation, then deletes `paths` for good.
pub fn delete(paths: Vec<PathBuf>, listed: Vec<Listed>) {
    if !paths.is_empty() {
        change(|hub| hub.board.delete(paths, listed));
    }
}

/// Answers the question shown.
pub fn answer(
    answer: rustphin_view::questions::Answer,
    apply_to_all: bool,
    password: Option<String>,
) {
    change(
        |hub| match hub.board.answer(answer, apply_to_all, password) {
            Some(Reply::Resolve(id, resolution)) => hub.runner_of(id).resolve(id, resolution),
            Some(Reply::Recover(id, recovery)) => hub.runner_of(id).recover(id, recovery),
            Some(Reply::Password(id, password)) => hub.runner_of(id).give_password(id, password),
            Some(Reply::Start(launch)) => hub.launch(launch),
            None => {}
        },
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
                        schedule(hub.board.say(
                            format!(
                                "{} {} no record of where it came from and cannot be restored. Move it out by hand.",
                                count(without_record),
                                plural(without_record, "entry has", "entries have")
                            ),
                            true,
                        ));
                    }
                    if !items.is_empty() {
                        hub.active().start(Request::Restore { items });
                    }
                }
                Err(error) => {
                    schedule(
                        hub.board
                            .say(format!("The trash could not be read: {error}."), true),
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
                Ok((entries, paths)) => match hub.board.empty_trash(entries, paths, ask) {
                    TrashEmptying::Said(expiry) => schedule(expiry),
                    TrashEmptying::Asked => {}
                    TrashEmptying::Start(launch) => hub.launch(launch),
                },
                Err(error) => {
                    schedule(
                        hub.board
                            .say(format!("The trash could not be read: {error}."), true),
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
    let copy = action.map(|action| action == DropAction::Copy);
    change(|hub| {
        if let Some(launch) = hub.board.drop_files(sources, into, copy) {
            hub.launch(launch);
        }
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
                    let shared = hub.clipboard.is_shared();
                    schedule(hub.board.picked_up(clip, shared));
                }
                Err(error) => schedule(hub.board.say(
                    format!("The clipboard could not be written: {error}."),
                    true,
                )),
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
                let launch = hub.board.paste(clip, into);
                hub.launch(launch);
            });
        });
    });
}

/// Shows `text` as the latest message. Any thread.
pub fn say(text: String, is_error: bool) {
    post::on_qt(move || change(|hub| schedule(hub.board.say(text, is_error))));
}

/// Dismisses the message being shown.
pub fn dismiss_notice() {
    change(|hub| hub.board.dismiss());
}

/// Shows `text`, which reports no error, until [`withdraw`] is called with
/// the returned ticket or a later message replaces it.
pub fn say_until_withdrawn(text: String) -> u64 {
    change(|hub| hub.board.say_until_withdrawn(text))
}

/// Removes the message shown under `ticket`, unless a later one replaced it.
pub fn withdraw(ticket: u64) {
    change(|hub| hub.board.withdraw(ticket));
}
