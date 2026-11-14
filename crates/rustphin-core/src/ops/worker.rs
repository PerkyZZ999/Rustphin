//! Operations on worker threads: requests go in, progress, questions and
//! results come back as events on a channel.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use super::{
    Conflict, ConflictChoice, Controller, Failure, History, Outcome, Progress, Recovery, Request,
    Resolution, Trash, run,
};
use crate::Fs;

/// Identifies one running or finished operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId(u64);

impl OpId {
    /// The id as a number, unique within its [`Operations`].
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// The id numbered `number`, as [`Self::as_u64`] gave it: for a view
    /// that keeps an operation's number apart from the operation.
    pub fn from_u64(number: u64) -> Self {
        Self(number)
    }
}

/// Something an operation has to tell or ask.
#[derive(Debug)]
pub enum OpEvent {
    /// An operation began. `undo` is true when it reverts `request`.
    Started {
        /// The operation.
        id: OpId,
        /// What is being done, or undone.
        request: Request,
        /// Whether this reverts `request` instead of performing it.
        undo: bool,
    },
    /// How far it is. Sent at most every [`PROGRESS_INTERVAL`].
    Progress {
        /// The operation.
        id: OpId,
        /// Its state.
        progress: Progress,
    },
    /// A name is taken. The operation waits for [`Operations::resolve`].
    Conflict {
        /// The operation.
        id: OpId,
        /// What collides with what.
        conflict: Conflict,
    },
    /// An entry failed. The operation waits for [`Operations::recover`].
    Failed {
        /// The operation.
        id: OpId,
        /// The entry.
        path: PathBuf,
        /// Why, in words for the user.
        message: String,
    },
    /// An archive is encrypted. The operation waits for
    /// [`Operations::give_password`].
    Password {
        /// The operation.
        id: OpId,
        /// The archive.
        archive: PathBuf,
        /// The password given before was not right.
        wrong: bool,
    },
    /// The operation is over.
    Finished {
        /// The operation.
        id: OpId,
        /// How it ended.
        result: Finished,
    },
}

/// How an operation ended.
#[derive(Debug)]
pub enum Finished {
    /// It ran; the outcome says whether to the end and what was left out.
    Done(Outcome),
    /// It was refused before anything changed. The message is for the user.
    Refused(String),
    /// The previous operation was reverted.
    Undone,
    /// The undo was refused or stopped; the message says why.
    UndoFailed(String),
}

/// Shortest time between two [`OpEvent::Progress`] events of one operation.
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

enum Answer {
    Resolution(Resolution),
    Recovery(Recovery),
    /// A password, or none to stop. Never logged.
    Password(Option<String>),
}

struct Running {
    cancel: Arc<AtomicBool>,
    answers: flume::Sender<Answer>,
}

struct Shared {
    fs: Arc<dyn Fs>,
    trash: Arc<dyn Trash>,
    events: flume::Sender<OpEvent>,
    history: Mutex<History>,
    running: Mutex<HashMap<OpId, Running>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic on another operation's thread must not disable this one.
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs file operations in the background, each on its own thread, and keeps
/// the session's undo history.
pub struct Operations {
    shared: Arc<Shared>,
    next_id: AtomicU64,
}

impl Operations {
    /// Creates the runner and the channel its events arrive on.
    pub fn new(fs: Arc<dyn Fs>, trash: Arc<dyn Trash>) -> (Self, flume::Receiver<OpEvent>) {
        Self::with_first_id(fs, trash, 0)
    }

    /// As [`Self::new`], numbering operations from `first`, so the ids of two
    /// runners that report to one consumer never meet.
    pub fn with_first_id(
        fs: Arc<dyn Fs>,
        trash: Arc<dyn Trash>,
        first: u64,
    ) -> (Self, flume::Receiver<OpEvent>) {
        let (events, receiver) = flume::unbounded();
        let shared = Arc::new(Shared {
            fs,
            trash,
            events,
            history: Mutex::default(),
            running: Mutex::default(),
        });
        (
            Self {
                shared,
                next_id: AtomicU64::new(first),
            },
            receiver,
        )
    }

    /// Starts `request`. Its events carry the returned id.
    pub fn start(&self, request: Request) -> OpId {
        self.spawn(Some(request))
    }

    /// Starts reverting the most recent operation that can be undone.
    /// Returns `None` when there is none.
    pub fn undo(&self) -> Option<OpId> {
        if lock(&self.shared.history).is_empty() {
            return None;
        }
        Some(self.spawn(None))
    }

    /// The request the next [`Self::undo`] would revert.
    pub fn undoable(&self) -> Option<Request> {
        lock(&self.shared.history)
            .last()
            .map(|record| record.request.clone())
    }

    /// Asks operation `id` to stop at the next safe point. What it has done
    /// stays done and can be undone.
    pub fn cancel(&self, id: OpId) {
        if let Some(running) = lock(&self.shared.running).get(&id) {
            running.cancel.store(true, Ordering::Relaxed);
            // Wake it if it is waiting for an answer.
            let _ = running.answers.send(Answer::Recovery(Recovery::Cancel));
        }
    }

    /// Answers the [`OpEvent::Conflict`] operation `id` is waiting on.
    pub fn resolve(&self, id: OpId, resolution: Resolution) {
        self.answer(id, Answer::Resolution(resolution));
    }

    /// Answers the [`OpEvent::Failed`] operation `id` is waiting on.
    pub fn recover(&self, id: OpId, recovery: Recovery) {
        self.answer(id, Answer::Recovery(recovery));
    }

    /// Answers the [`OpEvent::Password`] operation `id` is waiting on;
    /// `None` stops it.
    pub fn give_password(&self, id: OpId, password: Option<String>) {
        self.answer(id, Answer::Password(password));
    }

    fn answer(&self, id: OpId, answer: Answer) {
        if let Some(running) = lock(&self.shared.running).get(&id) {
            let _ = running.answers.send(answer);
        }
    }

    /// Runs `request`, or an undo when there is none, on a new thread.
    fn spawn(&self, request: Option<Request>) -> OpId {
        let id = OpId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let cancel = Arc::new(AtomicBool::new(false));
        let (answers, answer_receiver) = flume::unbounded();
        lock(&self.shared.running).insert(
            id,
            Running {
                cancel: cancel.clone(),
                answers,
            },
        );
        let shared = self.shared.clone();
        let work = move || {
            let mut controller = ChannelController {
                id,
                events: shared.events.clone(),
                answers: answer_receiver,
                cancel,
                last_progress: None,
            };
            let result = match request {
                Some(request) => perform(&shared, id, request, &mut controller),
                None => revert(&shared, id, &mut controller),
            };
            lock(&shared.running).remove(&id);
            let _ = shared.events.send(OpEvent::Finished { id, result });
        };
        if let Err(error) = thread::Builder::new()
            .name("rustphin-operation".to_owned())
            .spawn(work)
        {
            lock(&self.shared.running).remove(&id);
            let _ = self.shared.events.send(OpEvent::Finished {
                id,
                result: Finished::Refused(format!("no thread could be started: {error}")),
            });
        }
        id
    }
}

impl Drop for Operations {
    /// Stops every operation at its next safe point. Their threads hold the
    /// shared state, so without this one waiting for an answer would wait
    /// forever.
    fn drop(&mut self) {
        for running in lock(&self.shared.running).values() {
            running.cancel.store(true, Ordering::Relaxed);
            let _ = running.answers.send(Answer::Recovery(Recovery::Cancel));
        }
    }
}

fn perform(
    shared: &Shared,
    id: OpId,
    request: Request,
    controller: &mut ChannelController,
) -> Finished {
    let _ = shared.events.send(OpEvent::Started {
        id,
        request: request.clone(),
        undo: false,
    });
    match run(&*shared.fs, &*shared.trash, &request, controller) {
        Ok(outcome) => {
            lock(&shared.history).push(request, outcome.changes.clone());
            Finished::Done(outcome)
        }
        Err(error) => Finished::Refused(error.to_string()),
    }
}

fn revert(shared: &Shared, id: OpId, controller: &mut ChannelController) -> Finished {
    // The history is locked for the whole undo: two undos must not interleave.
    let mut history = lock(&shared.history);
    let Some(request) = history.last().map(|record| record.request.clone()) else {
        return Finished::UndoFailed("there is nothing to undo".to_owned());
    };
    let _ = shared.events.send(OpEvent::Started {
        id,
        request,
        undo: true,
    });
    match history.undo_last(&*shared.fs, &*shared.trash, controller) {
        Some(Ok(_)) => Finished::Undone,
        Some(Err(error)) => Finished::UndoFailed(error.to_string()),
        None => Finished::UndoFailed("there is nothing to undo".to_owned()),
    }
}

/// Forwards an operation's reports and questions over the channel and blocks
/// its thread until a question is answered.
struct ChannelController {
    id: OpId,
    events: flume::Sender<OpEvent>,
    answers: flume::Receiver<Answer>,
    cancel: Arc<AtomicBool>,
    last_progress: Option<Instant>,
}

impl Controller for ChannelController {
    fn progress(&mut self, progress: &Progress) {
        let now = Instant::now();
        let due = self
            .last_progress
            .is_none_or(|last| now.duration_since(last) >= PROGRESS_INTERVAL);
        if due || progress.items_done == progress.items_total {
            self.last_progress = Some(now);
            let _ = self.events.send(OpEvent::Progress {
                id: self.id,
                progress: progress.clone(),
            });
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn conflict(&mut self, conflict: &Conflict) -> Resolution {
        let cancel = Resolution {
            choice: ConflictChoice::Cancel,
            apply_to_all: false,
        };
        let asked = self.events.send(OpEvent::Conflict {
            id: self.id,
            conflict: conflict.clone(),
        });
        if asked.is_err() {
            // Nobody is listening any more.
            return cancel;
        }
        loop {
            match self.answers.recv() {
                Ok(Answer::Resolution(resolution)) => return resolution,
                Ok(Answer::Recovery(Recovery::Cancel)) | Err(_) => return cancel,
                // An answer to a different kind of question: keep waiting.
                Ok(Answer::Recovery(_) | Answer::Password(_)) => {}
            }
        }
    }

    fn failure(&mut self, failure: &Failure) -> Recovery {
        let asked = self.events.send(OpEvent::Failed {
            id: self.id,
            path: failure.path.clone(),
            message: failure.error.to_string(),
        });
        if asked.is_err() {
            return Recovery::Cancel;
        }
        loop {
            match self.answers.recv() {
                Ok(Answer::Recovery(recovery)) => return recovery,
                Err(_) => return Recovery::Cancel,
                Ok(Answer::Resolution(_) | Answer::Password(_)) => {}
            }
        }
    }

    fn password(&mut self, archive: &Path, wrong: bool) -> Option<String> {
        let asked = self.events.send(OpEvent::Password {
            id: self.id,
            archive: archive.to_owned(),
            wrong,
        });
        if asked.is_err() {
            return None;
        }
        loop {
            match self.answers.recv() {
                Ok(Answer::Password(password)) => return password,
                Ok(Answer::Recovery(Recovery::Cancel)) | Err(_) => return None,
                Ok(Answer::Recovery(_) | Answer::Resolution(_)) => {}
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::fs;

    use super::{
        super::{
            NoTrash,
            faults::{Fault, FaultFs},
            tests::{copy, dirs, file, move_, names, read, tree},
        },
        *,
    };
    use crate::RealFs;

    const TIMEOUT: Duration = Duration::from_secs(20);

    fn operations(fs: impl Fs) -> (Operations, flume::Receiver<OpEvent>) {
        Operations::new(Arc::new(fs), Arc::new(NoTrash))
    }

    /// Waits for the next event that is not a progress report.
    fn next(events: &flume::Receiver<OpEvent>) -> OpEvent {
        loop {
            match events.recv_timeout(TIMEOUT).expect("an event in time") {
                OpEvent::Progress { .. } => {}
                event => return event,
            }
        }
    }

    fn finished(events: &flume::Receiver<OpEvent>, id: OpId) -> Finished {
        match next(events) {
            OpEvent::Finished { id: done, result } if done == id => result,
            other => panic!("expected the end of {id:?}, got {other:?}"),
        }
    }

    #[test]
    fn an_operation_reports_its_start_progress_and_end() {
        let d = dirs();
        file(d.src.join("a.txt"), "a");
        let (ops, events) = operations(RealFs);

        let id = ops.start(copy(&[d.src.join("a.txt")], &d.dst));

        assert!(matches!(
            events.recv_timeout(TIMEOUT).unwrap(),
            OpEvent::Started { id: started, undo: false, request: Request::Copy { .. } } if started == id
        ));
        let mut last = None;
        let result = loop {
            match events.recv_timeout(TIMEOUT).unwrap() {
                OpEvent::Progress { progress, .. } => last = Some(progress),
                OpEvent::Finished { result, .. } => break result,
                other => panic!("unexpected {other:?}"),
            }
        };
        let last = last.expect("at least the final progress report");
        assert_eq!((last.items_done, last.items_total), (1, 1));
        assert!(matches!(result, Finished::Done(outcome) if outcome.completed));
        assert_eq!(read(d.dst.join("a.txt")), "a");
    }

    #[test]
    fn a_runner_numbers_its_operations_from_where_it_is_told() {
        let d = dirs();
        file(d.src.join("a.txt"), "a");
        let (ops, events) = Operations::with_first_id(Arc::new(RealFs), Arc::new(NoTrash), 1 << 62);

        let id = ops.start(copy(&[d.src.join("a.txt")], &d.dst));

        assert_eq!(id.as_u64(), 1 << 62);
        assert!(matches!(next(&events), OpEvent::Started { id: started, .. } if started == id));
        assert!(matches!(finished(&events, id), Finished::Done(_)));
        assert_eq!(
            ops.start(copy(&[d.src.join("a.txt")], &d.src)).as_u64(),
            (1 << 62) + 1
        );
    }

    #[test]
    fn a_conflict_waits_for_the_answer() {
        let d = dirs();
        file(d.src.join("a.txt"), "new");
        file(d.dst.join("a.txt"), "old");
        let (ops, events) = operations(RealFs);

        let id = ops.start(copy(&[d.src.join("a.txt")], &d.dst));
        assert!(matches!(next(&events), OpEvent::Started { .. }));
        let OpEvent::Conflict { conflict, .. } = next(&events) else {
            panic!("expected a conflict");
        };
        assert_eq!(conflict.target, d.dst.join("a.txt"));
        // The question can name what keeping both would create.
        assert_eq!(conflict.keep_both, d.dst.join("a (1).txt"));
        // Still waiting: nothing has been written.
        assert_eq!(names(&d.dst), ["a.txt"]);
        ops.resolve(
            id,
            Resolution {
                choice: ConflictChoice::KeepBoth,
                apply_to_all: false,
            },
        );

        assert!(matches!(finished(&events, id), Finished::Done(outcome) if outcome.completed));
        assert_eq!(read(d.dst.join("a.txt")), "old");
        assert_eq!(read(d.dst.join("a (1).txt")), "new");
    }

    #[test]
    fn cancelling_while_a_question_is_open_ends_the_operation() {
        let d = dirs();
        file(d.src.join("a.txt"), "new");
        file(d.dst.join("a.txt"), "old");
        let (ops, events) = operations(RealFs);

        let id = ops.start(copy(&[d.src.join("a.txt")], &d.dst));
        assert!(matches!(next(&events), OpEvent::Started { .. }));
        assert!(matches!(next(&events), OpEvent::Conflict { .. }));
        ops.cancel(id);

        assert!(matches!(finished(&events, id), Finished::Done(outcome) if !outcome.completed));
        assert_eq!(read(d.dst.join("a.txt")), "old");
        assert_eq!(names(&d.dst), ["a.txt"]);
    }

    #[test]
    fn a_failing_entry_waits_for_skip_or_retry() {
        let d = dirs();
        file(d.src.join("a.txt"), "content");
        let (ops, events) = operations(FaultFs::failing_at(0, Fault::Error));

        let id = ops.start(copy(&[d.src.join("a.txt")], &d.dst));
        assert!(matches!(next(&events), OpEvent::Started { .. }));
        let OpEvent::Failed { path, message, .. } = next(&events) else {
            panic!("expected a failure");
        };
        assert_eq!(path, d.src.join("a.txt"));
        assert!(message.contains("injected fault"), "{message}");
        ops.recover(id, Recovery::Retry);

        assert!(matches!(
            finished(&events, id),
            Finished::Done(outcome) if outcome.completed && outcome.skipped.is_empty()
        ));
        assert_eq!(read(d.dst.join("a.txt")), "content");
    }

    #[test]
    fn a_refused_request_ends_with_the_reason_and_is_not_offered_for_undo() {
        let d = dirs();
        let (ops, events) = operations(RealFs);

        let id = ops.start(Request::CreateDir {
            parent: d.src.clone(),
            name: "a/b".into(),
        });
        assert!(matches!(next(&events), OpEvent::Started { .. }));

        assert!(matches!(
            finished(&events, id),
            Finished::Refused(message) if message.contains("cannot contain")
        ));
        assert!(ops.undoable().is_none());
        assert!(ops.undo().is_none());
    }

    #[test]
    fn undo_reverts_the_latest_operation_and_then_the_one_before() {
        let d = dirs();
        file(d.src.join("a.txt"), "a");
        let before = (tree(&d.src), tree(&d.dst));
        let (ops, events) = operations(RealFs);
        let run_to_end = |request: Request| {
            let id = ops.start(request);
            assert!(matches!(next(&events), OpEvent::Started { .. }));
            assert!(matches!(finished(&events, id), Finished::Done(_)));
        };
        run_to_end(Request::Rename {
            path: d.src.join("a.txt"),
            new_name: "b.txt".into(),
        });
        run_to_end(move_(&[d.src.join("b.txt")], &d.dst));
        assert!(matches!(ops.undoable(), Some(Request::Move { .. })));

        for expected in ["Move", "Rename"] {
            let id = ops.undo().unwrap();
            let OpEvent::Started {
                request,
                undo: true,
                ..
            } = next(&events)
            else {
                panic!("expected the start of an undo");
            };
            assert!(format!("{request:?}").starts_with(expected), "{request:?}");
            assert!(matches!(finished(&events, id), Finished::Undone));
        }

        assert_eq!((tree(&d.src), tree(&d.dst)), before);
        assert!(ops.undo().is_none());
    }

    #[test]
    fn a_refused_undo_says_why_and_stays_on_offer() {
        let d = dirs();
        file(d.src.join("a.txt"), "moved");
        let (ops, events) = operations(RealFs);
        let id = ops.start(move_(&[d.src.join("a.txt")], &d.dst));
        assert!(matches!(next(&events), OpEvent::Started { .. }));
        assert!(matches!(finished(&events, id), Finished::Done(_)));
        fs::write(d.src.join("a.txt"), "in the way").unwrap();

        let id = ops.undo().unwrap();
        assert!(matches!(next(&events), OpEvent::Started { undo: true, .. }));

        assert!(matches!(
            finished(&events, id),
            Finished::UndoFailed(message) if message.contains("something else is now at")
        ));
        assert!(ops.undoable().is_some());
        assert_eq!(read(d.dst.join("a.txt")), "moved");
    }

    #[test]
    fn an_operation_whose_listener_went_away_stops_at_its_next_question() {
        let d = dirs();
        file(d.src.join("a.txt"), "new");
        file(d.dst.join("a.txt"), "old");
        let (ops, events) = operations(RealFs);
        // The operation's thread holds the shared state while it lives.
        let shared = Arc::downgrade(&ops.shared);

        ops.start(copy(&[d.src.join("a.txt")], &d.dst));
        assert!(matches!(next(&events), OpEvent::Started { .. }));
        assert!(matches!(next(&events), OpEvent::Conflict { .. }));
        drop(ops);
        drop(events);

        // Dropping the runner cancels the operation; wait for its thread to
        // let go of the shared state.
        let deadline = Instant::now() + TIMEOUT;
        while shared.strong_count() > 0 {
            assert!(
                Instant::now() < deadline,
                "the operation thread is still alive"
            );
            thread::yield_now();
        }
        assert_eq!(read(d.dst.join("a.txt")), "old");
    }
}
