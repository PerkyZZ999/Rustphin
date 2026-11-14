//! What the application's file operations show, and in what order
//! (W-124): the progress line, the question in front and what answering it
//! means, the latest notice and when it goes, what an operation left out,
//! the entries an extraction or compression made, and what waits on the
//! clipboard. Free of any toolkit and of real clocks: the core's operation
//! events and the time they arrived are its input, and what it returns
//! tells the interface what to start and when to come back.
//!
//! The board never touches a file. What it decides to do comes back as a
//! [`Launch`] or a [`Reply`] for the core's operation engine (D-011).

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use jiff::tz::TimeZone;
use rustphin_core::ops::{
    Change, ConflictChoice, Finished, OpEvent, OpId, Progress, Recovery, Request, Resolution,
};
use rustphin_platform::clipboard::FileClip;

use crate::{
    operations::{
        Listed, Notice, Problems, activity, activity_icon, activity_verb, cut_sets, fraction, past,
        pick_up_notice, result_notice,
    },
    questions::{self, Answer, Question},
    transfer::{self, Card, Meter},
};

/// How long an operation may run before its progress is shown. Most finish
/// sooner and would only flash on screen.
pub const QUIET_PERIOD: Duration = Duration::from_millis(300);
/// How long a message about a finished operation stays.
pub const NOTICE_TIME: Duration = Duration::from_secs(6);
/// Where administrator runners number their operations from.
const ADMIN_FIRST_ID: u64 = 1 << 62;

/// The first operation number of the `run`th administrator runner, so its
/// operations are told apart from the user's own and from earlier runs.
pub fn admin_first_id(run: u64) -> u64 {
    ADMIN_FIRST_ID + (run << 40)
}

/// Whether `id` belongs to an administrator runner.
pub fn is_admin(id: OpId) -> bool {
    id.as_u64() >= ADMIN_FIRST_ID
}

/// An operation in progress.
#[derive(Debug)]
struct Running {
    id: OpId,
    label: String,
    done: String,
    undo: bool,
    started: Instant,
    progress: Progress,
    icon: String,
    verb: &'static str,
    meter: Meter,
}

impl Running {
    fn shown(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started) >= QUIET_PERIOD
    }
}

/// The progress line of the operation shown.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressLine {
    /// What it is doing: "Copying 3 items to “x”".
    pub label: String,
    /// Counts and speed.
    pub detail: String,
    /// The Lucide icon beside the line.
    pub icon: String,
    /// How far it is, when that is known.
    pub fraction: Option<f32>,
}

/// Words for an operation's progress line and final notice, given when it
/// starts, instead of the ones its request would get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The progress line: "Emptying the trash".
    pub label: String,
    /// What it did, in lower case: "emptied the trash".
    pub done: String,
    /// Its notice stays until dismissed.
    pub lasting: bool,
}

/// An operation to start through the active runner.
#[derive(Debug)]
pub struct Launch {
    /// What to do.
    pub request: Request,
    /// Its words, for [`OperationBoard::describe`] once it has an id.
    pub words: Option<Words>,
}

impl Launch {
    fn plain(request: Request) -> Self {
        Self {
            request,
            words: None,
        }
    }
}

/// What answering the question in front means.
#[derive(Debug)]
pub enum Reply {
    /// Tell operation `id` how to settle its conflict.
    Resolve(OpId, Resolution),
    /// Tell operation `id` how to go on after a failure.
    Recover(OpId, Recovery),
    /// Give operation `id` a password, or none to stop.
    Password(OpId, Option<String>),
    /// Start an operation.
    Start(Launch),
}

/// A notice's timer: give it back to [`OperationBoard::expire`] once
/// `after` has passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Expiry {
    /// How long the notice stays.
    pub after: Duration,
    serial: u64,
}

/// What the interface has to arrange after an event.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct FollowUp {
    /// Look again after this long: an operation may then be shown.
    pub redraw_after: Option<Duration>,
    /// A notice to take away later.
    pub expiry: Option<Expiry>,
}

/// The state of every file operation the interface shows.
#[derive(Debug, Default)]
pub struct OperationBoard {
    running: Vec<Running>,
    questions: VecDeque<Question>,
    notice: Option<Notice>,
    notice_serial: u64,
    problems: Option<Problems>,
    words: HashMap<OpId, Words>,
    reveal: HashSet<OpId>,
    created: (u64, Vec<PathBuf>),
    admin_changes: u64,
    clip: Option<FileClip>,
    cut_paths: HashSet<PathBuf>,
    cut_folders: HashSet<PathBuf>,
    clip_serial: u64,
    told_unshared: bool,
}

impl OperationBoard {
    /// An empty board.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes an event of the core's operation engine, arrived at `now`.
    pub fn on_event(&mut self, event: OpEvent, now: Instant) -> FollowUp {
        match event {
            OpEvent::Started { id, request, undo } => {
                if !undo && matches!(request, Request::Extract { .. } | Request::Compress { .. }) {
                    self.reveal.insert(id);
                }
                let (label, done) = match self.words.get(&id) {
                    Some(words) => (words.label.clone(), words.done.clone()),
                    None => (activity(&request, undo), past(&request)),
                };
                self.running.push(Running {
                    id,
                    label,
                    done,
                    undo,
                    started: now,
                    progress: Progress::default(),
                    icon: activity_icon(&request).lucide_name(),
                    verb: activity_verb(&request),
                    meter: Meter::new(),
                });
                FollowUp {
                    redraw_after: Some(QUIET_PERIOD),
                    expiry: None,
                }
            }
            OpEvent::Progress { id, progress } => {
                if let Some(running) = self.running.iter_mut().find(|running| running.id == id) {
                    running.meter.observe(&progress, now);
                    running.progress = progress;
                }
                FollowUp::default()
            }
            OpEvent::Conflict { id, conflict } => {
                self.questions
                    .push_back(Question::Conflict { id, conflict });
                FollowUp::default()
            }
            OpEvent::Failed { id, path, message } => {
                self.questions
                    .push_back(Question::Failed { id, path, message });
                FollowUp::default()
            }
            OpEvent::Password { id, archive, wrong } => {
                self.questions
                    .push_back(Question::Password { id, archive, wrong });
                FollowUp::default()
            }
            OpEvent::Finished { id, result } => FollowUp {
                redraw_after: None,
                expiry: self.finish(id, &result),
            },
        }
    }

    fn finish(&mut self, id: OpId, result: &Finished) -> Option<Expiry> {
        if self.reveal.remove(&id)
            && let Finished::Done(outcome) = result
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
        if is_admin(id) {
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
        let lasting = self.words.remove(&id).is_some_and(|words| words.lasting);
        let (done, undo) = finished
            .as_ref()
            .map_or(("", false), |running| (running.done.as_str(), running.undo));
        let notice = result_notice(done, undo, result)?;
        let problems = match result {
            Finished::Done(outcome) if !outcome.skipped.is_empty() => Some(Problems {
                title: finished
                    .as_ref()
                    .map(|running| running.label.clone())
                    .unwrap_or_default(),
                entries: outcome.skipped.clone(),
            }),
            _ => None,
        };
        let expires = !notice.is_error && !lasting;
        let expiry = self.present(notice, expires);
        self.problems = problems;
        expiry
    }

    fn present(&mut self, notice: Notice, expires: bool) -> Option<Expiry> {
        self.notice_serial += 1;
        self.problems = None;
        self.notice = Some(notice);
        expires.then_some(Expiry {
            after: NOTICE_TIME,
            serial: self.notice_serial,
        })
    }

    /// Gives operation `id`, just started, the words of its progress line
    /// and notice.
    pub fn describe(&mut self, id: OpId, words: Words) {
        self.words.insert(id, words);
    }

    /// Shows `text` as the latest message; an error stays until dismissed.
    pub fn say(&mut self, text: String, is_error: bool) -> Option<Expiry> {
        self.present(
            Notice {
                text,
                is_error,
                offers_undo: false,
            },
            !is_error,
        )
    }

    /// Shows `text`, which reports no error, until [`Self::withdraw`] is
    /// called with the returned ticket or a later message replaces it.
    pub fn say_until_withdrawn(&mut self, text: String) -> u64 {
        let _lasts = self.present(
            Notice {
                text,
                is_error: false,
                offers_undo: false,
            },
            false,
        );
        self.notice_serial
    }

    /// Removes the message shown under `ticket`, unless a later one
    /// replaced it.
    pub fn withdraw(&mut self, ticket: u64) {
        if self.notice_serial == ticket {
            self.notice = None;
        }
    }

    /// Takes away the message `expiry` was given for, and what its
    /// operation left out, unless a later message replaced it.
    pub fn expire(&mut self, expiry: Expiry) {
        if self.notice_serial == expiry.serial {
            self.notice = None;
            self.problems = None;
        }
    }

    /// Dismisses the message shown.
    pub fn dismiss(&mut self) {
        self.notice = None;
        self.problems = None;
    }

    /// Says that there is nothing to undo.
    pub fn nothing_to_undo(&mut self) -> Option<Expiry> {
        self.say("There is nothing to undo.".to_owned(), false)
    }

    /// The operation to stop: `id`, or else the oldest running.
    pub fn to_cancel(&self, id: Option<OpId>) -> Option<OpId> {
        id.or_else(|| self.running.first().map(|running| running.id))
    }

    /// Moves `paths` to the trash, after asking when `ask`; as
    /// administrator, where there is no trash, asks to delete for good.
    pub fn trash(
        &mut self,
        paths: Vec<PathBuf>,
        listed: Vec<Listed>,
        ask: bool,
        admin: bool,
    ) -> Option<Launch> {
        if paths.is_empty() {
            return None;
        }
        if admin {
            self.questions
                .push_back(Question::ConfirmDelete { paths, listed });
        } else if ask {
            self.questions
                .push_back(Question::ConfirmTrash { paths, listed });
        } else {
            return Some(Launch::plain(Request::Trash { paths }));
        }
        None
    }

    /// Asks before deleting `paths` for good.
    pub fn delete(&mut self, paths: Vec<PathBuf>, listed: Vec<Listed>) {
        if !paths.is_empty() {
            self.questions
                .push_back(Question::ConfirmDelete { paths, listed });
        }
    }

    /// Empties the trash, `entries` entries made of `paths`, after asking
    /// when `ask`.
    pub fn empty_trash(&mut self, entries: usize, paths: Vec<PathBuf>, ask: bool) -> TrashEmptying {
        if paths.is_empty() {
            TrashEmptying::Said(self.say("The trash is already empty.".to_owned(), false))
        } else if ask {
            self.questions
                .push_back(Question::ConfirmEmpty { entries, paths });
            TrashEmptying::Asked
        } else {
            TrashEmptying::Start(emptying(paths))
        }
    }

    /// Copies or moves `sources` dropped on `into` as `copy` says, or asks
    /// which when it is `None`.
    pub fn drop_files(
        &mut self,
        sources: Vec<PathBuf>,
        into: PathBuf,
        copy: Option<bool>,
    ) -> Option<Launch> {
        match copy {
            Some(true) => Some(Launch::plain(Request::Copy { sources, into })),
            Some(false) => Some(Launch::plain(Request::Move { sources, into })),
            None => {
                self.questions.push_back(Question::Drop { sources, into });
                None
            }
        }
    }

    /// Answers the question in front and takes it away.
    pub fn answer(
        &mut self,
        answer: Answer,
        apply_to_all: bool,
        password: Option<String>,
    ) -> Option<Reply> {
        let question = self.questions.pop_front()?;
        Some(match (question, answer) {
            (Question::Conflict { id, .. }, answer) => {
                let choice = match answer {
                    Answer::Cancel => ConflictChoice::Cancel,
                    Answer::Skip => ConflictChoice::Skip,
                    Answer::Replace => ConflictChoice::Overwrite,
                    Answer::Accept | Answer::Copy | Answer::Open | Answer::Run => {
                        ConflictChoice::KeepBoth
                    }
                };
                Reply::Resolve(
                    id,
                    Resolution {
                        choice,
                        apply_to_all,
                    },
                )
            }
            (Question::Failed { id, .. }, answer) => Reply::Recover(
                id,
                match answer {
                    Answer::Accept => Recovery::Retry,
                    Answer::Skip => Recovery::Skip,
                    _ => Recovery::Cancel,
                },
            ),
            (Question::Password { id, .. }, Answer::Accept) => Reply::Password(id, password),
            (Question::Password { id, .. }, _) => Reply::Password(id, None),
            (Question::ConfirmDelete { paths, .. }, Answer::Accept) => {
                Reply::Start(Launch::plain(Request::Delete { paths }))
            }
            (Question::ConfirmTrash { paths, .. }, Answer::Accept) => {
                Reply::Start(Launch::plain(Request::Trash { paths }))
            }
            (Question::ConfirmEmpty { paths, .. }, Answer::Accept) => Reply::Start(emptying(paths)),
            (Question::Drop { sources, into }, Answer::Accept) => {
                Reply::Start(Launch::plain(Request::Move { sources, into }))
            }
            (Question::Drop { sources, into }, Answer::Copy) => {
                Reply::Start(Launch::plain(Request::Copy { sources, into }))
            }
            _ => return None,
        })
    }

    /// Takes what the clipboard holds now; `true` when that changed.
    pub fn clip_read(&mut self, clip: Option<FileClip>) -> bool {
        if self.clip == clip {
            return false;
        }
        self.set_clip(clip);
        true
    }

    /// Records `clip`, just written to the clipboard, and says so; `shared`
    /// when other applications can paste it.
    pub fn picked_up(&mut self, clip: FileClip, shared: bool) -> Option<Expiry> {
        let text = pick_up_notice(&clip, shared, self.told_unshared);
        self.told_unshared = true;
        self.set_clip(Some(clip));
        self.say(text, false)
    }

    /// What pasting `clip` into `into` starts. A move happens once, so a
    /// cut clip is gone afterwards, as in other file managers.
    pub fn paste(&mut self, clip: FileClip, into: PathBuf) -> Launch {
        let sources = clip.paths;
        if clip.cut {
            self.set_clip(None);
            Launch::plain(Request::Move { sources, into })
        } else {
            Launch::plain(Request::Copy { sources, into })
        }
    }

    fn set_clip(&mut self, clip: Option<FileClip>) {
        (self.cut_paths, self.cut_folders) = cut_sets(clip.as_ref());
        self.clip = clip;
        self.clip_serial += 1;
    }

    /// Whether `path` was cut and waits to be pasted somewhere.
    pub fn is_cut(&self, path: &Path) -> bool {
        path.parent()
            .is_some_and(|folder| self.cut_folders.contains(folder))
            && self.cut_paths.contains(path)
    }

    /// Whether anything is on the clipboard to paste.
    pub fn can_paste(&self) -> bool {
        self.clip.is_some()
    }

    /// Counts changes to the clipboard, so panes redraw cut entries.
    pub fn clip_serial(&self) -> u64 {
        self.clip_serial
    }

    /// What the latest extraction or compression made, numbered so views
    /// select it once (W-111, D-032).
    pub fn created(&self) -> (u64, &[PathBuf]) {
        (self.created.0, &self.created.1)
    }

    /// How many administrator operations have finished: KDE's helper
    /// cannot watch folders, so views re-read when this changes.
    pub fn admin_changes(&self) -> u64 {
        self.admin_changes
    }

    /// How many operations are running, shown or not.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Measures every running operation's speed up to `now`, and says how
    /// long to wait before the next call; nothing once none is running.
    /// Speeds and times remaining change only here and with progress.
    pub fn tick(&mut self, now: Instant) -> Option<Duration> {
        for running in &mut self.running {
            running.meter.advance(now);
        }
        (!self.running.is_empty()).then_some(transfer::SAMPLE)
    }

    /// The progress line of the oldest operation that has run long enough
    /// by `now` to be worth showing.
    pub fn progress(&self, now: Instant) -> Option<ProgressLine> {
        self.running
            .iter()
            .find(|running| running.shown(now))
            .map(|running| ProgressLine {
                label: running.label.clone(),
                detail: transfer::figures(&running.progress, &running.meter),
                icon: running.icon.clone(),
                fraction: fraction(&running.progress),
            })
    }

    /// A card for each operation that has run long enough by `now` to be
    /// worth showing, oldest first: the transfer dialog (W-149).
    pub fn transfers(&self, now: Instant) -> Vec<Card> {
        self.running
            .iter()
            .filter(|running| running.shown(now))
            .map(|running| Card {
                id: running.id.as_u64().to_string(),
                label: running.label.clone(),
                icon: running.icon.clone(),
                fraction: fraction(&running.progress).unwrap_or(-1.),
                done: transfer::done_text(&running.progress),
                pace: transfer::pace_text(&running.meter),
                current: transfer::current_text(&running.progress),
                graph: running.meter.graph(),
            })
            .collect()
    }

    /// The question in front, as it is put.
    pub fn question(&self, zone: &TimeZone) -> Option<questions::Shown> {
        let question = self.questions.front()?;
        let verb = question.asker().map_or("change", |id| {
            self.running
                .iter()
                .find(|running| running.id == id)
                .map_or("change", |running| running.verb)
        });
        Some(questions::show(question, verb, zone))
    }

    /// The latest message, if not yet dismissed.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// What the operation behind the notice left out.
    pub fn problems(&self) -> Option<&Problems> {
        self.problems.as_ref()
    }
}

/// What emptying the trash comes to.
#[derive(Debug)]
pub enum TrashEmptying {
    /// It was already empty; the message may expire.
    Said(Option<Expiry>),
    /// The user is asked first.
    Asked,
    /// Start it now.
    Start(Launch),
}

fn emptying(paths: Vec<PathBuf>) -> Launch {
    Launch {
        request: Request::Delete { paths },
        words: Some(Words {
            label: "Emptying the trash".to_owned(),
            done: "emptied the trash".to_owned(),
            lasting: false,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rustphin_core::{
        EntryKind, FileInfo,
        ops::{Conflict, Outcome, Skipped},
    };

    use super::*;

    fn id(number: u64) -> OpId {
        OpId::from_u64(number)
    }

    fn info() -> FileInfo {
        FileInfo {
            kind: EntryKind::File,
            size: 1,
            modified_ns: 0,
            device: 0,
            inode: 0,
            mode: 0o644,
        }
    }

    fn copy(into: &str) -> Request {
        Request::Copy {
            sources: vec!["/a/report.pdf".into()],
            into: into.into(),
        }
    }

    fn started(id: OpId, request: Request) -> OpEvent {
        OpEvent::Started {
            id,
            request,
            undo: false,
        }
    }

    fn conflict(id: OpId, name: &str) -> OpEvent {
        OpEvent::Conflict {
            id,
            conflict: Conflict {
                source: Path::new("/a").join(name),
                target: Path::new("/b").join(name),
                source_info: info(),
                target_info: info(),
                keep_both: Path::new("/b").join(format!("{name} (1)")),
                can_overwrite: true,
            },
        }
    }

    fn finished(id: OpId, changes: Vec<Change>, skipped: &[&str]) -> OpEvent {
        OpEvent::Finished {
            id,
            result: Finished::Done(Outcome {
                completed: true,
                changes,
                skipped: skipped
                    .iter()
                    .map(|path| Skipped {
                        path: path.into(),
                        reason: "Permission denied".to_owned(),
                    })
                    .collect(),
            }),
        }
    }

    fn cancelled(id: OpId) -> OpEvent {
        OpEvent::Finished {
            id,
            result: Finished::Done(Outcome {
                completed: false,
                changes: Vec::new(),
                skipped: Vec::new(),
            }),
        }
    }

    fn title(board: &OperationBoard) -> Option<String> {
        board.question(&TimeZone::UTC).map(|shown| shown.title)
    }

    fn notice(board: &OperationBoard) -> Option<String> {
        board.notice().map(|notice| notice.text.clone())
    }

    #[test]
    fn a_conflict_is_asked_until_answered_and_the_answer_goes_to_its_operation() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(conflict(id(1), "photo.jpg"), now);
        assert_eq!(
            title(&board).as_deref(),
            Some("\u{201c}photo.jpg\u{201d} already exists")
        );

        let reply = board.answer(Answer::Replace, true, None);
        let Some(Reply::Resolve(asker, resolution)) = reply else {
            panic!("expected a resolution, got {reply:?}");
        };
        assert_eq!(asker, id(1));
        assert_eq!(resolution.choice, ConflictChoice::Overwrite);
        assert!(resolution.apply_to_all);
        assert_eq!(title(&board), None);
    }

    #[test]
    fn a_question_goes_when_its_operation_ends_without_an_answer() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(conflict(id(1), "photo.jpg"), now);

        let _ = board.on_event(cancelled(id(1)), now);

        assert_eq!(title(&board), None);
        assert_eq!(board.running_count(), 0);
        assert!(notice(&board).unwrap().starts_with("Stopped"));
    }

    #[test]
    fn questions_of_two_operations_are_asked_in_turn_and_each_ends_with_its_own() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(started(id(2), copy("/c")), now);
        let _ = board.on_event(conflict(id(1), "first"), now);
        let _ = board.on_event(conflict(id(2), "second"), now);
        let _ = board.on_event(conflict(id(1), "third"), now);
        assert_eq!(
            title(&board).as_deref(),
            Some("\u{201c}first\u{201d} already exists")
        );

        let reply = board.answer(Answer::Skip, false, None);
        assert!(matches!(reply, Some(Reply::Resolve(asker, _)) if asker == id(1)));
        assert_eq!(
            title(&board).as_deref(),
            Some("\u{201c}second\u{201d} already exists")
        );

        // The second operation ends: its question goes, the first's stays.
        let _ = board.on_event(cancelled(id(2)), now);
        assert_eq!(
            title(&board).as_deref(),
            Some("\u{201c}third\u{201d} already exists")
        );
        let reply = board.answer(Answer::Accept, false, None);
        let Some(Reply::Resolve(asker, resolution)) = reply else {
            panic!("expected a resolution, got {reply:?}");
        };
        assert_eq!(asker, id(1));
        assert_eq!(resolution.choice, ConflictChoice::KeepBoth);
        assert!(board.answer(Answer::Accept, false, None).is_none());
    }

    #[test]
    fn a_notice_goes_on_its_own_timer_but_not_on_one_it_outlived() {
        let mut board = OperationBoard::new();
        let first = board.say("Copied 1 item.".to_owned(), false).unwrap();
        assert_eq!(first.after, NOTICE_TIME);
        let second = board.say("Moved 2 items.".to_owned(), false).unwrap();

        board.expire(first);
        assert_eq!(notice(&board).as_deref(), Some("Moved 2 items."));

        board.expire(second);
        assert_eq!(notice(&board), None);
    }

    #[test]
    fn errors_and_lasting_notices_have_no_timer() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        assert_eq!(board.say("It broke.".to_owned(), true), None);

        let _ = board.on_event(started(id(1), copy("/b")), now);
        board.describe(
            id(1),
            Words {
                label: "Restoring".to_owned(),
                done: "restored the files".to_owned(),
                lasting: true,
            },
        );
        let follow_up = board.on_event(finished(id(1), Vec::new(), &[]), now);
        assert_eq!(follow_up.expiry, None);
        assert!(board.notice().is_some());
    }

    #[test]
    fn a_withdrawn_message_goes_unless_a_newer_one_replaced_it() {
        let mut board = OperationBoard::new();
        let ticket = board.say_until_withdrawn("Connecting…".to_owned());
        assert_eq!(notice(&board).as_deref(), Some("Connecting…"));
        assert!(!board.notice().unwrap().is_error);
        board.withdraw(ticket);
        assert_eq!(notice(&board), None);

        let ticket = board.say_until_withdrawn("Connecting…".to_owned());
        let _ = board.say("Could not connect.".to_owned(), true);
        board.withdraw(ticket);
        assert_eq!(notice(&board).as_deref(), Some("Could not connect."));
    }

    #[test]
    fn what_an_extraction_made_is_revealed_once() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let before = board.created().0;
        let extract = Request::Extract {
            archive: "/a/photos.zip".into(),
            into: "/a".into(),
            layout: rustphin_core::ops::Layout::Here,
            trash_archive: false,
        };
        let _ = board.on_event(started(id(1), extract), now);
        let _ = board.on_event(started(id(2), copy("/a")), now);
        let made = Change::Copied {
            to: "/a/photos".into(),
            info: info(),
        };
        let _ = board.on_event(finished(id(1), vec![made.clone()], &[]), now);
        assert_eq!(
            board.created(),
            (before + 1, &[PathBuf::from("/a/photos")][..])
        );

        // A copy, or the same operation reported again, reveals nothing new.
        let pasted = Change::Copied {
            to: "/a/report.pdf".into(),
            info: info(),
        };
        let _ = board.on_event(finished(id(2), vec![pasted], &[]), now);
        let _ = board.on_event(finished(id(1), vec![made], &[]), now);
        assert_eq!(board.created().0, before + 1);
    }

    #[test]
    fn an_undo_reveals_nothing() {
        // Kept as it was before W-124: only extraction and compression
        // select what they made; an undo leaves the selection alone.
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(
            OpEvent::Started {
                id: id(1),
                request: Request::Compress {
                    sources: vec!["/a/x".into()],
                    archive: "/a/x.zip".into(),
                    format: rustphin_core::archive::Format::Zip,
                    level: rustphin_core::archive::Level::Normal,
                },
                undo: true,
            },
            now,
        );
        let restored = Change::Copied {
            to: "/a/x".into(),
            info: info(),
        };
        let _ = board.on_event(finished(id(1), vec![restored], &[]), now);
        assert_eq!(board.created().0, 0);
    }

    #[test]
    fn what_an_operation_left_out_stays_with_its_notice_only() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let follow_up = board.on_event(finished(id(1), Vec::new(), &["/a/secret"]), now);
        let problems = board.problems().unwrap();
        assert_eq!(
            problems.title,
            "Copying \u{201c}report.pdf\u{201d} to \u{201c}b\u{201d}"
        );
        assert_eq!(problems.entries.len(), 1);
        assert_eq!(problems.entries[0].path, PathBuf::from("/a/secret"));
        // Something was left out: an error, which stays until dismissed.
        assert!(board.notice().unwrap().is_error);
        assert_eq!(follow_up.expiry, None);

        // A later operation's problems replace them.
        let _ = board.on_event(started(id(2), copy("/c")), now);
        let _ = board.on_event(finished(id(2), Vec::new(), &["/a/one", "/a/two"]), now);
        let problems = board.problems().unwrap();
        assert_eq!(
            problems.title,
            "Copying \u{201c}report.pdf\u{201d} to \u{201c}c\u{201d}"
        );
        assert_eq!(problems.entries.len(), 2);

        // The next message takes them away, and so does dismissing.
        let _ = board.say("Copied to the clipboard.".to_owned(), false);
        assert!(board.problems().is_none());
        let _ = board.on_event(started(id(3), copy("/b")), now);
        let _ = board.on_event(finished(id(3), Vec::new(), &["/a/two"]), now);
        assert!(board.problems().is_some());
        board.dismiss();
        assert!(board.problems().is_none());
        assert!(board.notice().is_none());
    }

    #[test]
    fn a_finished_operation_without_a_notice_leaves_the_problems_shown() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(finished(id(1), Vec::new(), &["/a/secret"]), now);
        // Creating succeeds silently, so it takes nothing away.
        let create = Request::CreateDir {
            parent: "/b".into(),
            name: "new".into(),
        };
        let _ = board.on_event(started(id(2), create), now);
        let _ = board.on_event(finished(id(2), Vec::new(), &[]), now);
        assert!(board.problems().is_some());
    }

    #[test]
    fn pasting_a_cut_clip_moves_it_once() {
        let mut board = OperationBoard::new();
        let cut = FileClip {
            paths: vec!["/a/report.pdf".into()],
            cut: true,
        };
        let serial = board.clip_serial();
        let _ = board.picked_up(cut.clone(), true);
        assert!(board.is_cut(Path::new("/a/report.pdf")));
        assert!(board.can_paste());
        assert!(board.clip_serial() > serial);

        let launch = board.paste(cut, "/b".into());
        assert!(
            matches!(launch.request, Request::Move { ref into, .. } if into == Path::new("/b"))
        );
        assert!(!board.is_cut(Path::new("/a/report.pdf")));
        assert!(!board.can_paste());
    }

    #[test]
    fn pasting_a_copied_clip_keeps_it_for_another_paste() {
        let mut board = OperationBoard::new();
        let copied = FileClip {
            paths: vec!["/a/report.pdf".into()],
            cut: false,
        };
        let _ = board.picked_up(copied.clone(), true);
        assert!(!board.is_cut(Path::new("/a/report.pdf")));

        let launch = board.paste(copied, "/b".into());
        assert!(matches!(launch.request, Request::Copy { .. }));
        assert!(board.can_paste());
    }

    #[test]
    fn an_operation_is_shown_once_it_has_run_for_the_quiet_period() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let follow_up = board.on_event(started(id(1), copy("/b")), now);
        assert_eq!(follow_up.redraw_after, Some(QUIET_PERIOD));
        assert_eq!(board.progress(now + QUIET_PERIOD / 2), None);
        let line = board.progress(now + QUIET_PERIOD).unwrap();
        assert_eq!(
            line.label,
            "Copying \u{201c}report.pdf\u{201d} to \u{201c}b\u{201d}"
        );
        assert_eq!(board.to_cancel(None), Some(id(1)));
    }

    fn progressed(id: OpId, bytes_done: u64, bytes_total: u64) -> OpEvent {
        OpEvent::Progress {
            id,
            progress: Progress {
                items_done: 1,
                items_total: 3,
                bytes_done,
                bytes_total,
                current: "/a/report.pdf".into(),
            },
        }
    }

    /// Feeds operation `id` `per_second` bytes in each of the `seconds`
    /// after `start`, ticking the board as the hub does.
    fn run_steadily(
        board: &mut OperationBoard,
        id: OpId,
        start: Instant,
        per_second: u64,
        seconds: u32,
    ) {
        let total = 100 * 1024 * 1024;
        let _ = board.on_event(progressed(id, 0, total), start);
        for second in 1..=seconds {
            let now = start + transfer::SAMPLE * second;
            let _ = board.on_event(progressed(id, per_second * u64::from(second), total), now);
            let _ = board.tick(now);
        }
    }

    #[test]
    fn the_board_asks_to_be_ticked_only_while_something_runs() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        assert_eq!(board.tick(now), None);

        let _ = board.on_event(started(id(1), copy("/b")), now);
        assert_eq!(board.tick(now), Some(transfer::SAMPLE));

        let _ = board.on_event(cancelled(id(1)), now);
        assert_eq!(board.tick(now), None);
    }

    #[test]
    fn the_progress_line_tells_the_speed_and_then_the_time_remaining() {
        const MIB: u64 = 1024 * 1024;
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(progressed(id(1), 0, 100 * MIB), now);
        let detail = |board: &OperationBoard, at| board.progress(at).unwrap().detail;
        assert_eq!(
            detail(&board, now + QUIET_PERIOD),
            "1 of 3 · 0 B of 100.0 MiB"
        );

        run_steadily(&mut board, id(1), now, 10 * MIB, 1);
        assert_eq!(
            detail(&board, now + transfer::SAMPLE),
            "1 of 3 · 10.0 MiB of 100.0 MiB · 10.0 MiB/s"
        );

        run_steadily(&mut board, id(1), now, 10 * MIB, 3);
        assert_eq!(
            detail(&board, now + transfer::SAMPLE * 3),
            "1 of 3 · 30.0 MiB of 100.0 MiB · 10.0 MiB/s · about 7 s left"
        );
    }

    #[test]
    fn a_tick_without_progress_slows_the_speed_shown() {
        const MIB: u64 = 1024 * 1024;
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        run_steadily(&mut board, id(1), now, 10 * MIB, 3);

        let later = now + transfer::SAMPLE * 4;
        let _ = board.tick(later);
        assert_eq!(
            board.progress(later).unwrap().detail,
            "1 of 3 · 30.0 MiB of 100.0 MiB · 7.0 MiB/s · about 10 s left"
        );
    }

    #[test]
    fn every_operation_shown_has_its_own_card_until_it_ends() {
        const MIB: u64 = 1024 * 1024;
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(started(id(2), copy("/c")), now);
        run_steadily(&mut board, id(1), now, 10 * MIB, 4);
        run_steadily(&mut board, id(2), now, 20 * MIB, 4);
        let later = now + transfer::SAMPLE * 4;

        let cards = board.transfers(later);
        assert_eq!(cards.len(), 2);
        assert_eq!(
            cards[0],
            Card {
                id: "1".to_owned(),
                label: "Copying \u{201c}report.pdf\u{201d} to \u{201c}b\u{201d}".to_owned(),
                icon: "copy".to_owned(),
                fraction: 0.4,
                done: "1 of 3 items · 40.0 MiB of 100.0 MiB".to_owned(),
                pace: "10.0 MiB/s · about 6 s left".to_owned(),
                current: "/a/report.pdf".to_owned(),
                graph: transfer::Graph {
                    points: vec![1.; 4],
                    peak: "Peak 10.0 MiB/s".to_owned(),
                    span: transfer::GRAPH_SAMPLES,
                },
            }
        );
        assert_eq!(cards[1].id, "2");
        assert_eq!(cards[1].pace, "20.0 MiB/s · about 1 s left");

        let _ = board.on_event(cancelled(id(1)), later);
        let cards = board.transfers(later);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "2");
    }

    #[test]
    fn an_operation_gets_a_card_only_after_the_quiet_period() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        assert_eq!(board.transfers(now + QUIET_PERIOD / 2), Vec::new());

        let cards = board.transfers(now + QUIET_PERIOD);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].fraction, -1.);
        assert_eq!(cards[0].current, "Preparing…");
        assert_eq!(cards[0].pace, "");
    }

    #[test]
    fn trashing_asks_first_when_set_to_and_deletes_for_good_as_administrator() {
        let mut board = OperationBoard::new();
        let paths = || vec![PathBuf::from("/a/old")];
        let listed = || vec![Listed::default()];

        let launch = board.trash(paths(), listed(), false, false).unwrap();
        assert!(matches!(launch.request, Request::Trash { .. }));

        assert!(board.trash(paths(), listed(), true, false).is_none());
        let reply = board.answer(Answer::Accept, false, None);
        assert!(matches!(
            reply,
            Some(Reply::Start(Launch {
                request: Request::Trash { .. },
                ..
            }))
        ));

        assert!(board.trash(paths(), listed(), false, true).is_none());
        let reply = board.answer(Answer::Accept, false, None);
        assert!(matches!(
            reply,
            Some(Reply::Start(Launch {
                request: Request::Delete { .. },
                ..
            }))
        ));
    }

    #[test]
    fn emptying_the_trash_is_described_as_such() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let TrashEmptying::Start(launch) = board.empty_trash(1, vec!["/t/files/x".into()], false)
        else {
            panic!("expected to start");
        };
        board.describe(id(5), launch.words.unwrap());
        let _ = board.on_event(started(id(5), launch.request), now);
        assert_eq!(
            board.progress(now + QUIET_PERIOD).unwrap().label,
            "Emptying the trash"
        );

        assert!(matches!(
            board.empty_trash(0, Vec::new(), true),
            TrashEmptying::Said(Some(_))
        ));
        assert_eq!(
            notice(&board).as_deref(),
            Some("The trash is already empty.")
        );
    }

    #[test]
    fn administrator_operations_are_told_apart_and_counted() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let admin = OpId::from_u64(admin_first_id(1));
        assert!(is_admin(admin));
        assert!(!is_admin(id(7)));

        let _ = board.on_event(started(id(7), copy("/b")), now);
        let _ = board.on_event(finished(id(7), Vec::new(), &[]), now);
        assert_eq!(board.admin_changes(), 0);
        let _ = board.on_event(started(admin, copy("/root")), now);
        let _ = board.on_event(finished(admin, Vec::new(), &[]), now);
        assert_eq!(board.admin_changes(), 1);
    }

    fn failed(id: OpId) -> OpEvent {
        OpEvent::Failed {
            id,
            path: "/a/report.pdf".into(),
            message: "Permission denied".to_owned(),
        }
    }

    fn password(id: OpId) -> OpEvent {
        OpEvent::Password {
            id,
            archive: "/a/locked.zip".into(),
            wrong: false,
        }
    }

    fn started_request(reply: Option<Reply>) -> Option<Request> {
        match reply {
            Some(Reply::Start(launch)) => Some(launch.request),
            _ => None,
        }
    }

    fn recovery(reply: Option<Reply>) -> Option<(OpId, Recovery)> {
        match reply {
            Some(Reply::Recover(asker, recovery)) => Some((asker, recovery)),
            _ => None,
        }
    }

    fn asked_failure(board: &mut OperationBoard) {
        let now = Instant::now();
        let _ = board.on_event(started(id(4), copy("/b")), now);
        let _ = board.on_event(failed(id(4)), now);
    }

    #[test]
    fn a_failure_answered_accept_is_tried_again() {
        let mut board = OperationBoard::new();
        asked_failure(&mut board);
        let reply = board.answer(Answer::Accept, false, None);
        assert_eq!(recovery(reply), Some((id(4), Recovery::Retry)));
    }

    #[test]
    fn a_failure_answered_skip_leaves_the_entry_out() {
        let mut board = OperationBoard::new();
        asked_failure(&mut board);
        let reply = board.answer(Answer::Skip, false, None);
        assert_eq!(recovery(reply), Some((id(4), Recovery::Skip)));
    }

    #[test]
    fn a_failure_answered_anything_else_stops_the_operation() {
        for answer in [Answer::Cancel, Answer::Replace, Answer::Copy] {
            let mut board = OperationBoard::new();
            asked_failure(&mut board);
            let reply = board.answer(answer, false, None);
            assert_eq!(
                recovery(reply),
                Some((id(4), Recovery::Cancel)),
                "{answer:?}"
            );
        }
    }

    #[test]
    fn a_password_given_goes_to_its_operation() {
        let mut board = OperationBoard::new();
        let _ = board.on_event(password(id(6)), Instant::now());
        let reply = board.answer(Answer::Accept, false, Some("secret".to_owned()));
        assert!(
            matches!(reply, Some(Reply::Password(asker, Some(ref given))) if asker == id(6) && given == "secret"),
            "{reply:?}"
        );
    }

    #[test]
    fn a_declined_password_question_sends_none() {
        let mut board = OperationBoard::new();
        let _ = board.on_event(password(id(6)), Instant::now());
        let reply = board.answer(Answer::Cancel, false, Some("typed".to_owned()));
        assert!(
            matches!(reply, Some(Reply::Password(asker, None)) if asker == id(6)),
            "{reply:?}"
        );
    }

    #[test]
    fn deleting_always_asks_first_and_starts_only_on_accept() {
        let mut board = OperationBoard::new();
        board.delete(vec!["/a/old".into()], vec![Listed::default()]);
        assert!(title(&board).is_some(), "asked");
        assert_eq!(
            started_request(board.answer(Answer::Accept, false, None)),
            Some(Request::Delete {
                paths: vec!["/a/old".into()]
            })
        );
        board.delete(Vec::new(), Vec::new());
        assert_eq!(title(&board), None, "nothing to delete, nothing asked");
    }

    #[test]
    fn a_confirmation_answered_other_than_accept_starts_nothing() {
        let paths = || vec![PathBuf::from("/a/old")];
        let listed = || vec![Listed::default()];
        for answer in [Answer::Cancel, Answer::Skip, Answer::Copy, Answer::Replace] {
            let mut board = OperationBoard::new();
            board.delete(paths(), listed());
            assert!(
                board.answer(answer, false, None).is_none(),
                "delete {answer:?}"
            );
            assert!(board.trash(paths(), listed(), true, false).is_none());
            assert!(
                board.answer(answer, false, None).is_none(),
                "trash {answer:?}"
            );
            assert!(matches!(
                board.empty_trash(1, paths(), true),
                TrashEmptying::Asked
            ));
            assert!(
                board.answer(answer, false, None).is_none(),
                "empty {answer:?}"
            );
            assert_eq!(title(&board), None, "each question was taken away");
        }
    }

    #[test]
    fn emptying_the_trash_starts_when_accepted() {
        let mut board = OperationBoard::new();
        let _ = board.empty_trash(1, vec!["/t/files/x".into()], true);
        let Some(Reply::Start(launch)) = board.answer(Answer::Accept, false, None) else {
            panic!("not started");
        };
        assert_eq!(
            launch.request,
            Request::Delete {
                paths: vec!["/t/files/x".into()]
            }
        );
        assert_eq!(launch.words.unwrap().label, "Emptying the trash");
    }

    #[test]
    fn a_drop_answered_accept_moves_and_copy_copies() {
        let sources = || vec![PathBuf::from("/a/report.pdf")];
        let mut board = OperationBoard::new();
        assert!(board.drop_files(sources(), "/b".into(), None).is_none());
        assert_eq!(
            started_request(board.answer(Answer::Accept, false, None)),
            Some(Request::Move {
                sources: sources(),
                into: "/b".into()
            })
        );
        assert!(board.drop_files(sources(), "/b".into(), None).is_none());
        assert_eq!(
            started_request(board.answer(Answer::Copy, false, None)),
            Some(Request::Copy {
                sources: sources(),
                into: "/b".into()
            })
        );
        assert!(board.drop_files(sources(), "/b".into(), None).is_none());
        assert!(board.answer(Answer::Cancel, false, None).is_none());
    }

    #[test]
    fn a_drop_with_a_known_choice_starts_at_once() {
        let sources = || vec![PathBuf::from("/a/report.pdf")];
        let mut board = OperationBoard::new();
        let copied = board
            .drop_files(sources(), "/b".into(), Some(true))
            .unwrap();
        assert_eq!(
            copied.request,
            Request::Copy {
                sources: sources(),
                into: "/b".into()
            }
        );
        let moved = board
            .drop_files(sources(), "/b".into(), Some(false))
            .unwrap();
        assert_eq!(
            moved.request,
            Request::Move {
                sources: sources(),
                into: "/b".into()
            }
        );
        assert_eq!(title(&board), None);
    }

    #[test]
    fn progress_goes_to_its_own_operation() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(started(id(2), copy("/c")), now);
        let _ = board.on_event(
            OpEvent::Progress {
                id: id(2),
                progress: Progress {
                    items_done: 3,
                    items_total: 9,
                    ..Progress::default()
                },
            },
            now,
        );
        // The oldest is shown, and it has made no progress.
        let line = board.progress(now + QUIET_PERIOD).unwrap();
        assert_eq!(line.detail, "0 of 0");
        assert_eq!(board.running_count(), 2);
    }

    #[test]
    fn an_operation_that_left_nothing_out_shows_no_problems() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let _ = board.on_event(started(id(1), copy("/b")), now);
        let _ = board.on_event(finished(id(1), Vec::new(), &[]), now);
        assert!(board.notice().is_some());
        assert!(board.problems().is_none());
    }

    #[test]
    fn nothing_to_undo_is_said_and_goes() {
        let mut board = OperationBoard::new();
        let expiry = board.nothing_to_undo().unwrap();
        assert_eq!(notice(&board).as_deref(), Some("There is nothing to undo."));
        board.expire(expiry);
        assert_eq!(notice(&board), None);
    }

    #[test]
    fn each_administrator_run_numbers_its_operations_apart() {
        let first = admin_first_id(0);
        let second = admin_first_id(1);
        assert!(is_admin(OpId::from_u64(first)));
        assert_eq!(second - first, 1 << 40);
    }

    #[test]
    fn reading_the_clipboard_says_whether_it_changed() {
        let mut board = OperationBoard::new();
        let clip = FileClip {
            paths: vec!["/a/report.pdf".into()],
            cut: true,
        };
        let serial = board.clip_serial();
        assert!(!board.clip_read(None), "nothing, as before");
        assert_eq!(board.clip_serial(), serial);
        assert!(board.clip_read(Some(clip.clone())));
        assert!(board.is_cut(Path::new("/a/report.pdf")));
        assert!(!board.clip_read(Some(clip)), "the same clip again");
        assert!(board.clip_read(None));
        assert!(!board.can_paste());
    }

    #[test]
    fn only_the_cut_entries_themselves_are_cut() {
        let mut board = OperationBoard::new();
        let _ = board.clip_read(Some(FileClip {
            paths: vec!["/a/report.pdf".into()],
            cut: true,
        }));
        assert!(board.is_cut(Path::new("/a/report.pdf")));
        // Same folder, another name; same name, another folder.
        assert!(!board.is_cut(Path::new("/a/other.pdf")));
        assert!(!board.is_cut(Path::new("/b/report.pdf")));
    }

    #[test]
    fn the_question_names_what_its_operation_does() {
        let now = Instant::now();
        let mut board = OperationBoard::new();
        let moving = Request::Move {
            sources: vec!["/a/report.pdf".into()],
            into: "/b".into(),
        };
        let _ = board.on_event(started(id(1), copy("/c")), now);
        let _ = board.on_event(started(id(2), moving), now);
        let _ = board.on_event(failed(id(2)), now);
        // The failing operation's own verb, not the oldest running one's.
        assert_eq!(
            title(&board).as_deref(),
            Some("Couldn't move \u{201c}report.pdf\u{201d}")
        );
    }
}
