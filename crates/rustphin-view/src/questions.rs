//! What a file operation needs decided, and how the question is put
//! (W-114f): its icon, title, sentence, facts, the names it is about and
//! its buttons, free of any toolkit. The Qt interface draws these; the
//! wording is the GPUI dialog's (mockups 19–22).

use std::path::{Path, PathBuf};

use jiff::tz::TimeZone;
use rustphin_core::{
    EntryKind, FileInfo, display_name,
    ops::{Conflict, OpId},
};
use serde::Serialize;

use crate::{
    format,
    operations::{Listed, upper_first, what},
};

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
        /// Asked from a window acting as administrator: the deletion runs
        /// with those rights.
        admin: bool,
    },
    /// Entries were dropped without a key saying whether to copy or move.
    Drop {
        /// The dropped entries.
        sources: Vec<PathBuf>,
        /// The folder they were dropped on.
        into: PathBuf,
        /// Dropped in a window acting as administrator.
        admin: bool,
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

impl Question {
    /// The operation that asks, for questions an operation is waiting on.
    pub fn asker(&self) -> Option<OpId> {
        match self {
            Self::Conflict { id, .. } | Self::Failed { id, .. } | Self::Password { id, .. } => {
                Some(*id)
            }
            _ => None,
        }
    }
}

/// An answer a question's button gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(into = "i32")]
pub enum Answer {
    /// Stop, or cancel (Esc).
    Cancel,
    /// Leave this entry out (S).
    Skip,
    /// Replace or merge (R).
    Replace,
    /// The primary action (Enter).
    Accept,
    /// Copy instead of move (C).
    Copy,
    /// Open a program's file with the default application instead (O).
    Open,
    /// Run a program or shortcut (R).
    Run,
}

impl From<Answer> for i32 {
    fn from(answer: Answer) -> Self {
        answer.number()
    }
}

impl Answer {
    /// The QML number of an answer.
    pub fn number(self) -> i32 {
        match self {
            Self::Cancel => 0,
            Self::Skip => 1,
            Self::Replace => 2,
            Self::Accept => 3,
            Self::Copy => 4,
            Self::Open => 5,
            Self::Run => 6,
        }
    }

    /// The answer a QML number names.
    pub fn from_number(number: i32) -> Option<Self> {
        Some(match number {
            0 => Self::Cancel,
            1 => Self::Skip,
            2 => Self::Replace,
            3 => Self::Accept,
            4 => Self::Copy,
            5 => Self::Open,
            6 => Self::Run,
            _ => return None,
        })
    }
}

/// How a button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Outlined.
    Plain,
    /// The primary action.
    Primary,
    /// A primary action that cannot be undone.
    Destructive,
}

/// One button of a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Button {
    /// What it answers.
    pub answer: Answer,
    /// Its label, with its key.
    pub label: String,
    /// Lucide icon name; empty for none.
    pub icon: &'static str,
    /// How it looks.
    pub style: Style,
    /// Whether it can be pressed.
    pub enabled: bool,
    /// The keys that press it: "escape", "enter" or a letter.
    pub keys: Vec<&'static str>,
}

/// One name a question is about, with what the listing knew of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    /// The entry's name.
    pub name: String,
    /// Lucide icon name; empty when unknown.
    pub icon: &'static str,
    /// Its size, for a file the listing knew.
    pub detail: String,
}

/// A question as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    /// Lucide icon name.
    pub icon: &'static str,
    /// The title.
    pub title: String,
    /// What happened and what the choices do.
    pub sentence: String,
    /// Label and value pairs.
    pub facts: Vec<(String, String)>,
    /// The names it is about.
    pub names: Vec<Name>,
    /// A line under the names.
    pub note: String,
    /// The buttons, the primary one last.
    pub buttons: Vec<Button>,
    /// Read out with the dialog: the choices in a sentence.
    pub description: String,
    /// A password field is shown.
    pub password: bool,
    /// The "do the same for all" box is shown.
    pub apply_to_all: bool,
}

/// The name of the entry at `path`, or "/".
pub fn label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
        None => "/".to_owned(),
    }
}

/// An entry as last read: "folder, modified …" or "1.2 MiB, modified …".
pub fn describe(info: &FileInfo, zone: &TimeZone) -> String {
    let modified = format::timestamp(info.modified_ns.div_euclid(1_000_000_000) as i64, zone);
    if info.kind == EntryKind::Directory {
        format!("folder, modified {modified}")
    } else {
        format!("{}, modified {modified}", format::size(info.size))
    }
}

fn button(answer: Answer, label: &str, icon: &'static str, style: Style) -> Button {
    Button {
        answer,
        label: label.to_owned(),
        icon,
        style,
        enabled: true,
        keys: match answer {
            Answer::Cancel => vec!["escape"],
            Answer::Skip => vec!["s"],
            Answer::Replace => vec!["r"],
            Answer::Accept => vec!["enter"],
            Answer::Copy => vec!["c"],
            Answer::Open => vec!["o"],
            Answer::Run => vec!["r"],
        },
    }
}

/// Asks before running a program or starting an application shortcut;
/// a program's file can be opened instead. Enter and Esc keep it.
pub fn launch(name: &str, location: &str, program: bool) -> Shown {
    let body = if program {
        "This file is a program. Running it executes it on this computer. Open hands it to the default application instead."
    } else {
        "This is an application shortcut. Running it starts that application."
    };
    let mut cancel = button(Answer::Cancel, "Cancel (Enter)", "", Style::Plain);
    cancel.keys = vec!["escape", "enter"];
    let mut buttons = vec![cancel];
    if program {
        buttons.push(button(Answer::Open, "Open (O)", "file-text", Style::Plain));
    }
    // Cancel is the Enter default, so nothing here is filled as a default
    // button would be: a filled Run would promise that Enter runs it.
    buttons.push(button(Answer::Run, "Run (R)", "play", Style::Plain));
    Shown {
        icon: if program { "terminal" } else { "app-window" },
        title: format!("Run \u{201c}{name}\u{201d}?"),
        sentence: body.to_owned(),
        facts: vec![("Location".to_owned(), location.to_owned())],
        names: Vec::new(),
        note: String::new(),
        buttons,
        description: format!(
            "{body} Enter or Escape keeps it; R runs it{}.",
            if program { "; O opens it" } else { "" }
        ),
        password: false,
        apply_to_all: false,
    }
}

fn names(paths: &[PathBuf], listed: &[Listed]) -> Vec<Name> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let known = listed.get(index).copied();
            Name {
                name: label(path),
                icon: match known {
                    Some(Listed { folder: true, .. }) => "folder",
                    Some(_) => "file",
                    None => "",
                },
                detail: known
                    .and_then(|known| known.size)
                    .map(format::size)
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// How `question` is put. `verb` is what the asking operation does to each
/// entry ("copy"), for a failure.
pub fn show(question: &Question, verb: &str, zone: &TimeZone) -> Shown {
    let blank = Shown {
        icon: "",
        title: String::new(),
        sentence: String::new(),
        facts: Vec::new(),
        names: Vec::new(),
        note: String::new(),
        buttons: Vec::new(),
        description: String::new(),
        password: false,
        apply_to_all: false,
    };
    match question {
        Question::Conflict { conflict, .. } => {
            let folder = conflict.target_info.kind == EntryKind::Directory;
            let place = conflict.target.parent().map(label).unwrap_or_default();
            let existing = describe(&conflict.target_info, zone);
            let incoming = describe(&conflict.source_info, zone);
            let mut replace = button(
                Answer::Replace,
                if folder { "Merge (R)" } else { "Replace (R)" },
                if folder { "merge" } else { "replace" },
                Style::Plain,
            );
            replace.enabled = conflict.can_overwrite;
            Shown {
                icon: "copy",
                title: format!("\u{201c}{}\u{201d} already exists", label(&conflict.target)),
                sentence: format!(
                    "{} with this name is already in \u{201c}{place}\u{201d}. Keep both gives the new one another name; Stop cancels the rest.",
                    if folder { "A folder" } else { "A file" }
                ),
                description: format!(
                    "Existing: {existing}. Incoming: {incoming}. Choose an available action. Keep both uses another name; Stop cancels."
                ),
                facts: vec![
                    ("Existing".to_owned(), existing),
                    ("Incoming".to_owned(), incoming),
                    ("Keep both".to_owned(), label(&conflict.keep_both)),
                ],
                buttons: vec![
                    button(Answer::Cancel, "Stop (Esc)", "", Style::Plain),
                    button(Answer::Skip, "Skip (S)", "skip-forward", Style::Plain),
                    replace,
                    button(
                        Answer::Accept,
                        "Keep both (Enter)",
                        "copy-plus",
                        Style::Primary,
                    ),
                ],
                apply_to_all: true,
                ..blank
            }
        }
        Question::Failed { path, message, .. } => Shown {
            icon: "triangle-alert",
            title: format!("Couldn't {verb} \u{201c}{}\u{201d}", label(path)),
            sentence: format!(
                "{}. Skip leaves this entry out and continues with the rest.",
                upper_first(message)
            ),
            description: format!(
                "{message}. Skip leaves this entry out and continues with the rest. Try again retries it; Stop cancels."
            ),
            facts: path
                .parent()
                .map(|folder| ("From".to_owned(), folder.display().to_string()))
                .into_iter()
                .collect(),
            buttons: vec![
                button(Answer::Cancel, "Stop (Esc)", "", Style::Plain),
                button(Answer::Skip, "Skip (S)", "skip-forward", Style::Plain),
                button(
                    Answer::Accept,
                    "Try again (Enter)",
                    "rotate-cw",
                    Style::Primary,
                ),
            ],
            ..blank
        },
        Question::Password { archive, wrong, .. } => Shown {
            icon: "lock-keyhole",
            title: if *wrong {
                format!("Wrong password for \u{201c}{}\u{201d}", label(archive))
            } else {
                format!("\u{201c}{}\u{201d} is encrypted", label(archive))
            },
            sentence: if *wrong {
                "That password did not open it. Type it again; nothing has been extracted."
                    .to_owned()
            } else {
                "Type its password to extract it. It is used for this archive only and is not saved."
                    .to_owned()
            },
            description: "Type the archive's password. It is used for this archive only and not saved. Stop or Escape cancels.".to_owned(),
            buttons: vec![
                button(Answer::Cancel, "Stop (Esc)", "", Style::Plain),
                button(
                    Answer::Accept,
                    "Extract (Enter)",
                    "package-open",
                    Style::Primary,
                ),
            ],
            password: true,
            ..blank
        },
        Question::Drop { sources, into, .. } => Shown {
            icon: "arrow-left-right",
            title: "Move or copy?".to_owned(),
            sentence: format!(
                "Move or copy {} into \u{201c}{}\u{201d}?",
                what(sources),
                label(into)
            ),
            description: "Choose move or copy, or Cancel to keep the files where they are."
                .to_owned(),
            names: names(sources, &[]),
            note: "Hold Shift while dropping to move, Ctrl to copy, without being asked."
                .to_owned(),
            buttons: vec![
                button(Answer::Cancel, "Cancel (Esc)", "", Style::Plain),
                button(Answer::Copy, "Copy here (C)", "copy", Style::Plain),
                button(
                    Answer::Accept,
                    "Move here (Enter)",
                    "folder-input",
                    Style::Primary,
                ),
            ],
            ..blank
        },
        Question::ConfirmDelete { paths, listed, .. } => Shown {
            icon: "circle-x",
            title: "Delete for good?".to_owned(),
            sentence: format!(
                "Delete {} for good? This does not use the trash. It cannot be undone.",
                what(paths)
            ),
            description: "This does not use the trash. It cannot be undone. Cancel or Escape leaves it as it is.".to_owned(),
            names: names(paths, listed),
            buttons: vec![
                button(Answer::Cancel, "Cancel (Esc)", "", Style::Plain),
                button(
                    Answer::Accept,
                    "Delete for good (Enter)",
                    "circle-x",
                    Style::Destructive,
                ),
            ],
            ..blank
        },
        Question::ConfirmTrash { paths, listed } => Shown {
            icon: "trash",
            title: "Move to the trash?".to_owned(),
            sentence: format!(
                "Move {} to the trash? It can be restored from there.",
                what(paths)
            ),
            description:
                "It can be restored from the trash. Cancel or Escape leaves it where it is."
                    .to_owned(),
            names: names(paths, listed),
            buttons: vec![
                button(Answer::Cancel, "Cancel (Esc)", "", Style::Plain),
                button(
                    Answer::Accept,
                    "Move to trash (Enter)",
                    "trash",
                    Style::Primary,
                ),
            ],
            ..blank
        },
        Question::ConfirmEmpty { entries, .. } => Shown {
            icon: "trash",
            title: "Empty the trash?".to_owned(),
            sentence: format!(
                "It holds {} {}. Everything in the trash is deleted for good. It cannot be undone.",
                format::count(*entries),
                format::plural(*entries, "item", "items")
            ),
            description: "Everything in the trash is deleted for good. It cannot be undone. Cancel or Escape leaves it as it is.".to_owned(),
            buttons: vec![
                button(Answer::Cancel, "Cancel (Esc)", "", Style::Plain),
                button(
                    Answer::Accept,
                    "Empty the trash (Enter)",
                    "trash",
                    Style::Destructive,
                ),
            ],
            ..blank
        },
    }
}

/// Asks before closing a window with several tabs (Settings,
/// Confirmations). Enter closes; Esc keeps it open.
pub fn close_window(count: usize) -> Shown {
    let sentence = format!(
        "This window has {count} tabs. Closing it closes them all; they open again \
         next time when Rustphin restores the last tabs."
    );
    Shown {
        icon: "x",
        title: "Close the window?".to_owned(),
        description: format!("{sentence} Enter closes; Escape keeps it open."),
        sentence,
        facts: Vec::new(),
        names: Vec::new(),
        note: String::new(),
        buttons: vec![
            button(Answer::Cancel, "Cancel (Esc)", "", Style::Plain),
            button(
                Answer::Accept,
                &format!("Close {count} tabs (Enter)"),
                "x",
                Style::Primary,
            ),
        ],
        password: false,
        apply_to_all: false,
    }
}

/// The answer a key gives in a question's dialog (Dolphin's single
/// letters), if its button is offered: the lookup QML makes over the
/// buttons' keys. K keeps both names in a conflict, as Enter does.
pub fn answer_for_key(shown: &Shown, key: &str) -> Option<Answer> {
    let key = if key == "k" && shown.apply_to_all {
        "enter"
    } else {
        key
    };
    shown
        .buttons
        .iter()
        .find(|button| button.enabled && button.keys.contains(&key))
        .map(|button| button.answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(kind: EntryKind) -> FileInfo {
        FileInfo {
            kind,
            size: 2048,
            modified_ns: 0,
            device: 0,
            inode: 0,
            mode: 0o644,
        }
    }

    #[test]
    fn a_conflict_names_both_entries_and_offers_merge_for_folders() {
        let conflict = Conflict {
            source: "/a/photos".into(),
            target: "/b/photos".into(),
            source_info: info(EntryKind::Directory),
            target_info: info(EntryKind::Directory),
            keep_both: "/b/photos (1)".into(),
            can_overwrite: true,
        };
        let shown = show(
            &Question::Conflict {
                id: OpId::from_u64(1),
                conflict,
            },
            "copy",
            &TimeZone::UTC,
        );
        assert_eq!(shown.title, "\u{201c}photos\u{201d} already exists");
        assert!(shown.sentence.starts_with("A folder with this name"));
        assert!(
            shown
                .buttons
                .iter()
                .any(|b| b.label == "Merge (R)" && b.enabled)
        );
        assert_eq!(shown.facts[2], ("Keep both".into(), "photos (1)".into()));
        assert!(shown.apply_to_all);
        assert_eq!(answer_for_key(&shown, "r"), Some(Answer::Replace));
        assert_eq!(answer_for_key(&shown, "s"), Some(Answer::Skip));
        assert_eq!(answer_for_key(&shown, "k"), Some(Answer::Accept));
        assert_eq!(answer_for_key(&shown, "escape"), Some(Answer::Cancel));
        assert_eq!(answer_for_key(&shown, "c"), None);
    }

    #[test]
    fn a_file_that_cannot_be_replaced_offers_no_replace_key() {
        let conflict = Conflict {
            source: "/a/x".into(),
            target: "/b/x".into(),
            source_info: info(EntryKind::File),
            target_info: info(EntryKind::Directory),
            keep_both: "/b/x (1)".into(),
            can_overwrite: false,
        };
        let shown = show(
            &Question::Conflict {
                id: OpId::from_u64(1),
                conflict,
            },
            "copy",
            &TimeZone::UTC,
        );
        assert_eq!(answer_for_key(&shown, "r"), None);
    }

    #[test]
    fn confirmations_list_the_names_with_what_the_listing_knew() {
        let shown = show(
            &Question::ConfirmDelete {
                paths: vec!["/a/b.txt".into(), "/a/c".into()],
                listed: vec![
                    Listed {
                        folder: false,
                        size: Some(2048),
                    },
                    Listed {
                        folder: true,
                        size: None,
                    },
                ],
                admin: false,
            },
            "",
            &TimeZone::UTC,
        );
        assert_eq!(
            shown.sentence,
            "Delete 2 items for good? This does not use the trash. It cannot be undone."
        );
        assert_eq!(shown.names[0].detail, "2.0 KiB");
        assert_eq!(shown.names[1].icon, "folder");
        assert_eq!(
            shown.buttons.last().map(|b| b.style),
            Some(Style::Destructive)
        );
    }

    #[test]
    fn a_failure_names_the_verb_and_the_folder() {
        let shown = show(
            &Question::Failed {
                id: OpId::from_u64(1),
                path: "/srv/locked.txt".into(),
                message: "permission denied".into(),
            },
            "copy",
            &TimeZone::UTC,
        );
        assert_eq!(shown.title, "Couldn't copy \u{201c}locked.txt\u{201d}");
        assert!(shown.sentence.starts_with("Permission denied."));
        assert_eq!(shown.facts, [("From".to_owned(), "/srv".to_owned())]);
    }

    #[test]
    fn running_a_program_is_asked_and_enter_keeps_it() {
        let shown = launch("setup.sh", "/srv", true);
        assert_eq!(shown.title, "Run \u{201c}setup.sh\u{201d}?");
        assert_eq!(answer_for_key(&shown, "enter"), Some(Answer::Cancel));
        assert_eq!(answer_for_key(&shown, "o"), Some(Answer::Open));
        assert_eq!(answer_for_key(&shown, "r"), Some(Answer::Run));
        let shortcut = launch("Kate", "/srv", false);
        assert_eq!(answer_for_key(&shortcut, "o"), None);
    }

    #[test]
    fn closing_several_tabs_says_how_many_and_enter_closes() {
        let shown = close_window(3);
        assert!(shown.sentence.starts_with("This window has 3 tabs."));
        assert_eq!(answer_for_key(&shown, "enter"), Some(Answer::Accept));
        assert_eq!(answer_for_key(&shown, "escape"), Some(Answer::Cancel));
    }

    #[test]
    fn answers_survive_their_qml_numbers() {
        for answer in [
            Answer::Cancel,
            Answer::Skip,
            Answer::Replace,
            Answer::Accept,
            Answer::Copy,
            Answer::Open,
            Answer::Run,
        ] {
            assert_eq!(Answer::from_number(answer.number()), Some(answer));
        }
        assert_eq!(Answer::from_number(9), None);
    }
}
