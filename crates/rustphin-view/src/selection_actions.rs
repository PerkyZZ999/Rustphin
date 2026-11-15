//! What an action on the selection means in the folder shown (W-126),
//! free of any toolkit: the pane hands over its facts (the folder, whether
//! it is a trash or administrator mode, the selected entries, the names
//! taken) and gets back what to do, an engine request, a dialog to open
//! with its typed state, or something to say.
//!
//! Dialogs reach the window's dialog objects through a [`HandOff`]: the
//! pane keeps the dialog there and passes its ticket on with the signal,
//! and the dialog object takes it back by that ticket.

use std::{
    collections::{HashSet, VecDeque},
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt as _,
    path::{Path, PathBuf},
};

use rustphin_core::{
    Snapshot,
    archive::{Format, Level},
    ops::{Layout, Request},
};

use crate::{
    archives::{self, Opener},
    dialogs::{BatchRename, Compress},
    menu::{CompressHow, ExtractHow},
    naming::free_name,
    operations::{Listed, duplicate_names},
};

/// An action on the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAction {
    /// Delete: to the trash where there is one, else for good.
    MoveToTrash,
    /// Shift+Delete: for good, after asking.
    DeletePermanently,
    /// Copies beside the originals.
    Duplicate,
    /// A new folder in the folder shown.
    NewFolder,
    /// A new empty file in the folder shown.
    NewFile,
    /// The Extract menu.
    Extract(ExtractHow),
    /// The Compress menu.
    Compress(CompressHow),
    /// F2 with several selected.
    BatchRename,
}

impl SelectionAction {
    /// Whether the plan reads the selected entries' archive openers; the
    /// pane looks up types only then.
    pub fn reads_archives(self) -> bool {
        matches!(self, Self::Extract(_))
    }
}

/// One selected entry, as the pane's listing describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selected<'a> {
    /// Its name, as bytes.
    pub name: &'a [u8],
    /// It can be entered (a folder).
    pub is_dir: bool,
    /// What a question shows beside it.
    pub listed: Listed,
    /// Who opens it, if it is an archive; `None` too when the action does
    /// not [read archives](SelectionAction::reads_archives).
    pub archive: Option<Opener>,
}

/// Every name in a folder listing, shown or not: hidden entries and
/// entries the filter leaves out still take their names.
pub trait Listing {
    /// The names, read once per plan that needs them.
    fn listed_names(&self) -> HashSet<&[u8]>;
}

impl Listing for Snapshot {
    fn listed_names(&self) -> HashSet<&[u8]> {
        Snapshot::listed_names(self)
    }
}

impl Listing for [&[u8]] {
    fn listed_names(&self) -> HashSet<&[u8]> {
        self.iter().copied().collect()
    }
}

/// What the pane knows when an action is chosen.
pub struct Facts<'a, L: Listing + ?Sized> {
    /// The folder shown.
    pub folder: &'a Path,
    /// The folder is a trash's contents.
    pub in_trash: bool,
    /// The pane works as administrator (KDE's helper has no trash).
    pub as_administrator: bool,
    /// The selected entries, in the listing's order.
    pub selected: Vec<Selected<'a>>,
    /// Every name in the folder.
    pub listing: &'a L,
}

/// What to select once the entries an operation makes appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Select {
    /// Leave the selection.
    Nothing,
    /// Select all of these.
    All(Vec<Vec<u8>>),
    /// Select this one alone and start renaming it.
    AndRename(Vec<u8>),
}

/// A dialog the window opens, with what it acts on.
#[derive(Debug, Clone)]
pub enum Dialog {
    /// Compress to….
    Compress(Compress),
    /// Batch rename.
    BatchRename(BatchRename),
    /// Extract to…: the desktop's folder dialog.
    ExtractTo(ExtractTo),
}

/// Extract to…, waiting for the folder to be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractTo {
    /// The folder shown, where the folder dialog starts.
    pub folder: PathBuf,
    /// The archives.
    pub archives: Vec<PathBuf>,
    /// Trash the archives afterwards.
    pub trash: bool,
}

impl ExtractTo {
    /// The requests extracting into `into` makes, one per archive.
    pub fn requests(self, into: &Path) -> Vec<Request> {
        extract_requests(self.archives, into, Layout::Here, self.trash)
    }
}

/// What to do.
#[derive(Debug, Clone)]
pub enum Plan {
    /// Nothing applies (the caller may try something else).
    Nothing,
    /// Say this.
    Say(String),
    /// Move to the trash, asking first when Settings says so.
    Trash {
        /// The entries.
        paths: Vec<PathBuf>,
        /// What the question shows beside each.
        listed: Vec<Listed>,
    },
    /// Delete for good, after asking.
    Delete {
        /// The entries.
        paths: Vec<PathBuf>,
        /// What the question shows beside each.
        listed: Vec<Listed>,
    },
    /// Start these operations, then select what they make.
    Start {
        /// The engine's requests, in order.
        requests: Vec<Request>,
        /// What to select when it appears.
        select: Select,
    },
    /// Open a dialog.
    Open(Dialog),
}

/// What Extract and Compress say in the trash.
pub const IN_TRASH: &str =
    "Archives cannot be extracted or made in the trash. Restore the files first.";

/// What renaming several entries at once says as administrator.
pub const BATCH_AS_ADMINISTRATOR: &str = "Several entries cannot be renamed at once as administrator: KDE's helper cannot lock the folder. Rename them one by one.";
/// What Extract and Compress say as administrator.
pub const AS_ADMINISTRATOR: &str = "Archives cannot be extracted or made as administrator.";

/// What `action` does with the facts.
pub fn plan<L: Listing + ?Sized>(action: SelectionAction, facts: &Facts<'_, L>) -> Plan {
    match action {
        SelectionAction::MoveToTrash | SelectionAction::DeletePermanently => remove(action, facts),
        SelectionAction::Duplicate => duplicate(facts),
        SelectionAction::NewFolder | SelectionAction::NewFile => {
            create(action == SelectionAction::NewFolder, facts)
        }
        // As the context menu, and as Dolphin, which offers no archive
        // actions in the trash: an archive made there would have no trash
        // record. KDE's administrator helper reads and writes no archives.
        SelectionAction::Extract(_) | SelectionAction::Compress(_) if facts.in_trash => {
            Plan::Say(IN_TRASH.to_owned())
        }
        SelectionAction::Extract(_) | SelectionAction::Compress(_) if facts.as_administrator => {
            Plan::Say(AS_ADMINISTRATOR.to_owned())
        }
        SelectionAction::Extract(how) => extract(how, facts),
        SelectionAction::Compress(how) => compress(how, facts),
        SelectionAction::BatchRename => batch_rename(facts),
    }
}

fn paths<L: Listing + ?Sized>(facts: &Facts<'_, L>) -> Vec<PathBuf> {
    facts
        .selected
        .iter()
        .map(|entry| facts.folder.join(OsStr::from_bytes(entry.name)))
        .collect()
}

fn remove<L: Listing + ?Sized>(action: SelectionAction, facts: &Facts<'_, L>) -> Plan {
    if facts.selected.is_empty() {
        return Plan::Nothing;
    }
    let paths = paths(facts);
    let listed = facts.selected.iter().map(|entry| entry.listed).collect();
    // What is already in the trash can only be deleted for good; so can
    // anything as administrator: KDE's helper has no trash.
    if action == SelectionAction::DeletePermanently || facts.in_trash || facts.as_administrator {
        Plan::Delete { paths, listed }
    } else {
        Plan::Trash { paths, listed }
    }
}

fn duplicate<L: Listing + ?Sized>(facts: &Facts<'_, L>) -> Plan {
    if facts.in_trash || facts.selected.is_empty() {
        return Plan::Nothing;
    }
    let names: Vec<Vec<u8>> = facts
        .selected
        .iter()
        .map(|entry| entry.name.to_vec())
        .collect();
    let taken = facts.listing.listed_names();
    Plan::Start {
        requests: vec![Request::Duplicate {
            sources: paths(facts),
        }],
        select: Select::All(duplicate_names(&names, |name| taken.contains(name))),
    }
}

fn create<L: Listing + ?Sized>(folder: bool, facts: &Facts<'_, L>) -> Plan {
    if facts.in_trash {
        return Plan::Nothing;
    }
    let taken = facts.listing.listed_names();
    let base = if folder { "New Folder" } else { "New File" };
    let name = free_name(base, |name| taken.contains(name.as_bytes()));
    let select = Select::AndRename(name.clone().into_bytes());
    let (parent, name) = (facts.folder.to_owned(), OsString::from(name));
    Plan::Start {
        requests: vec![if folder {
            Request::CreateDir { parent, name }
        } else {
            Request::CreateFile { parent, name }
        }],
        select,
    }
}

fn extract_requests(
    archives: Vec<PathBuf>,
    into: &Path,
    layout: Layout,
    trash: bool,
) -> Vec<Request> {
    archives
        .into_iter()
        .map(|archive| Request::Extract {
            archive,
            into: into.to_owned(),
            layout,
            trash_archive: trash,
        })
        .collect()
}

fn extract<L: Listing + ?Sized>(how: ExtractHow, facts: &Facts<'_, L>) -> Plan {
    let archives: Vec<PathBuf> = facts
        .selected
        .iter()
        .filter(|entry| !entry.is_dir && entry.archive.is_some())
        .map(|entry| facts.folder.join(OsStr::from_bytes(entry.name)))
        .collect();
    if archives.is_empty() {
        return Plan::Say("Select an archive to extract.".to_owned());
    }
    let (layout, trash) = match how {
        ExtractHow::To => {
            return Plan::Open(Dialog::ExtractTo(ExtractTo {
                folder: facts.folder.to_owned(),
                archives,
                trash: false,
            }));
        }
        ExtractHow::Here => (Layout::Here, false),
        ExtractHow::NewFolder => (Layout::NewFolder, false),
        ExtractHow::AndTrash => (Layout::Here, true),
    };
    Plan::Start {
        requests: extract_requests(archives, facts.folder, layout, trash),
        select: Select::Nothing,
    }
}

fn compress<L: Listing + ?Sized>(how: CompressHow, facts: &Facts<'_, L>) -> Plan {
    if facts.selected.is_empty() {
        return Plan::Say("Select what to compress.".to_owned());
    }
    let sources = paths(facts);
    let entries: Vec<(&[u8], bool)> = facts
        .selected
        .iter()
        .map(|entry| (entry.name, entry.is_dir))
        .collect();
    let taken = facts.listing.listed_names();
    let [zip, tar_gz] = archives::quick_names(&entries, |name| taken.contains(name));
    let (format, name) = match how {
        CompressHow::Zip => (Format::Zip, zip),
        CompressHow::TarGz => (Format::TarGz, tar_gz),
        CompressHow::Choose => {
            let base = archives::compress_base(&entries);
            let taken = taken.into_iter().map(<[u8]>::to_vec).collect();
            return Plan::Open(Dialog::Compress(Compress::new(
                sources,
                facts.folder.to_owned(),
                taken,
                base,
            )));
        }
    };
    Plan::Start {
        requests: vec![Request::Compress {
            sources,
            archive: facts.folder.join(name),
            format,
            level: Level::Normal,
        }],
        select: Select::Nothing,
    }
}

fn batch_rename<L: Listing + ?Sized>(facts: &Facts<'_, L>) -> Plan {
    if facts.in_trash || facts.selected.len() < 2 {
        return Plan::Nothing;
    }
    // The recovery journal needs a lock on the folder, which KDE's helper
    // does not offer (ADMIN_MODE.md): refused, not tried and failed.
    if facts.as_administrator {
        return Plan::Say(BATCH_AS_ADMINISTRATOR.to_owned());
    }
    let chosen: HashSet<&[u8]> = facts.selected.iter().map(|entry| entry.name).collect();
    let staying = facts
        .listing
        .listed_names()
        .into_iter()
        .filter(|name| !chosen.contains(name))
        .map(|name| OsStr::from_bytes(name).to_owned())
        .collect();
    let selected = facts
        .selected
        .iter()
        .map(|entry| OsStr::from_bytes(entry.name).to_owned())
        .collect();
    Plan::Open(Dialog::BatchRename(BatchRename::new(
        facts.folder.to_owned(),
        selected,
        staying,
    )))
}

/// How many dialogs wait at most; one asked for and never opened (its
/// folder dialog cancelled) gives way to newer ones.
const WAITING: usize = 4;

/// Dialogs a pane asked for, waiting for the window's dialog objects to
/// take them by ticket. Owned by the application's hub.
#[derive(Debug, Default)]
pub struct HandOff {
    last: i32,
    waiting: VecDeque<(i32, Dialog)>,
}

impl HandOff {
    /// Keeps `dialog` and returns its ticket (never 0).
    pub fn keep(&mut self, dialog: Dialog) -> i32 {
        self.last = if self.last == i32::MAX {
            1
        } else {
            self.last + 1
        };
        if self.waiting.len() == WAITING {
            self.waiting.pop_front();
        }
        self.waiting.push_back((self.last, dialog));
        self.last
    }

    /// The dialog kept under `ticket`, once.
    pub fn take(&mut self, ticket: i32) -> Option<Dialog> {
        let at = self.waiting.iter().position(|(kept, _)| *kept == ticket)?;
        self.waiting.remove(at).map(|(_, dialog)| dialog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/u/Documents";

    fn file(name: &str) -> Selected<'_> {
        Selected {
            name: name.as_bytes(),
            is_dir: false,
            listed: Listed {
                folder: false,
                size: Some(3),
            },
            archive: None,
        }
    }

    fn dir(name: &str) -> Selected<'_> {
        Selected {
            is_dir: true,
            listed: Listed {
                folder: true,
                size: None,
            },
            ..file(name)
        }
    }

    fn archive(name: &str) -> Selected<'_> {
        Selected {
            archive: Some(Opener::Rustphin),
            ..file(name)
        }
    }

    fn facts<'a>(selected: Vec<Selected<'a>>, listing: &'a [&'a [u8]]) -> Facts<'a, [&'a [u8]]> {
        Facts {
            folder: Path::new(HOME),
            in_trash: false,
            as_administrator: false,
            selected,
            listing,
        }
    }

    fn at(name: &str) -> PathBuf {
        Path::new(HOME).join(name)
    }

    fn requests(plan: Plan) -> (Vec<Request>, Select) {
        match plan {
            Plan::Start { requests, select } => (requests, select),
            other => panic!("not started: {other:?}"),
        }
    }

    #[test]
    fn delete_trashes_except_in_the_trash_or_as_administrator_where_it_deletes_after_asking() {
        let listing: &[&[u8]] = &[b"a", b"b"];
        let removed = |in_trash, as_administrator, action| {
            let facts = Facts {
                in_trash,
                as_administrator,
                ..facts(vec![file("a"), dir("b")], listing)
            };
            plan(action, &facts)
        };
        let expected_paths = vec![at("a"), at("b")];
        match removed(false, false, SelectionAction::MoveToTrash) {
            Plan::Trash { paths, listed } => {
                assert_eq!(paths, expected_paths);
                assert_eq!(listed[1], dir("b").listed);
            }
            other => panic!("not trashed: {other:?}"),
        }
        for (in_trash, admin, action) in [
            (true, false, SelectionAction::MoveToTrash),
            (false, true, SelectionAction::MoveToTrash),
            (false, false, SelectionAction::DeletePermanently),
        ] {
            match removed(in_trash, admin, action) {
                Plan::Delete { paths, .. } => assert_eq!(paths, expected_paths),
                other => panic!("not deleted ({in_trash}, {admin}): {other:?}"),
            }
        }
        let nothing = plan(SelectionAction::MoveToTrash, &facts(vec![], listing));
        assert!(matches!(nothing, Plan::Nothing));
    }

    #[test]
    fn extract_takes_only_archives_and_lays_them_out_as_chosen() {
        let listing: &[&[u8]] = &[];
        let selected = || {
            vec![
                archive("a.zip"),
                file("notes.txt"),
                Selected {
                    archive: Some(Opener::Other),
                    ..file("film.rar")
                },
                // A folder named like an archive is not one.
                Selected {
                    archive: Some(Opener::Rustphin),
                    ..dir("old.zip")
                },
            ]
        };
        let archives = vec![at("a.zip"), at("film.rar")];
        for (how, layout, trash) in [
            (ExtractHow::Here, Layout::Here, false),
            (ExtractHow::NewFolder, Layout::NewFolder, false),
            (ExtractHow::AndTrash, Layout::Here, true),
        ] {
            let (made, select) = requests(plan(
                SelectionAction::Extract(how),
                &facts(selected(), listing),
            ));
            let expected: Vec<Request> = archives
                .iter()
                .map(|archive| Request::Extract {
                    archive: archive.clone(),
                    into: PathBuf::from(HOME),
                    layout,
                    trash_archive: trash,
                })
                .collect();
            assert_eq!(made, expected, "{how:?}");
            assert_eq!(select, Select::Nothing);
        }
        match plan(
            SelectionAction::Extract(ExtractHow::To),
            &facts(selected(), listing),
        ) {
            Plan::Open(Dialog::ExtractTo(to)) => {
                assert_eq!(
                    to,
                    ExtractTo {
                        folder: PathBuf::from(HOME),
                        archives: archives.clone(),
                        trash: false,
                    }
                );
                let into = Path::new("/tmp/out");
                assert_eq!(
                    to.requests(into),
                    archives
                        .iter()
                        .map(|archive| Request::Extract {
                            archive: archive.clone(),
                            into: into.to_owned(),
                            layout: Layout::Here,
                            trash_archive: false,
                        })
                        .collect::<Vec<_>>()
                );
            }
            other => panic!("no folder dialog: {other:?}"),
        }
        let none = plan(
            SelectionAction::Extract(ExtractHow::Here),
            &facts(vec![file("notes.txt")], listing),
        );
        assert!(matches!(none, Plan::Say(words) if words == "Select an archive to extract."));
    }

    #[test]
    fn nothing_is_extracted_or_compressed_in_the_trash() {
        // The palette offers these wherever the menu hides them; an archive
        // written into the trash's files would have no trash record.
        let listing: &[&[u8]] = &[b"a.zip"];
        let in_trash = || Facts {
            in_trash: true,
            ..facts(vec![archive("a.zip")], listing)
        };
        let extract = plan(SelectionAction::Extract(ExtractHow::Here), &in_trash());
        assert!(
            matches!(extract, Plan::Say(ref words) if words == IN_TRASH),
            "{extract:?}"
        );
        let extract_to = plan(SelectionAction::Extract(ExtractHow::To), &in_trash());
        assert!(
            matches!(extract_to, Plan::Say(ref words) if words == IN_TRASH),
            "{extract_to:?}"
        );
        let compress = plan(SelectionAction::Compress(CompressHow::Zip), &in_trash());
        assert!(
            matches!(compress, Plan::Say(ref words) if words == IN_TRASH),
            "{compress:?}"
        );
        let choose = plan(SelectionAction::Compress(CompressHow::Choose), &in_trash());
        assert!(
            matches!(choose, Plan::Say(ref words) if words == IN_TRASH),
            "{choose:?}"
        );
    }

    #[test]
    fn several_entries_are_not_renamed_at_once_as_administrator() {
        let listing: &[&[u8]] = &[b"a.txt", b"b.txt"];
        let as_administrator = |selected| Facts {
            as_administrator: true,
            ..facts(selected, listing)
        };
        match plan(
            SelectionAction::BatchRename,
            &as_administrator(vec![file("a.txt"), file("b.txt")]),
        ) {
            Plan::Say(words) => assert_eq!(words, BATCH_AS_ADMINISTRATOR),
            other => panic!("expected a refusal, got {other:?}"),
        }
        // One entry is renamed in place, which the helper can do.
        assert!(matches!(
            plan(
                SelectionAction::BatchRename,
                &as_administrator(vec![file("a.txt")])
            ),
            Plan::Nothing
        ));
    }

    #[test]
    fn nothing_is_extracted_or_compressed_as_administrator() {
        let listing: &[&[u8]] = &[b"a.zip"];
        let as_administrator = || Facts {
            as_administrator: true,
            ..facts(vec![archive("a.zip")], listing)
        };
        let extract = plan(
            SelectionAction::Extract(ExtractHow::AndTrash),
            &as_administrator(),
        );
        assert!(
            matches!(extract, Plan::Say(ref words) if words == AS_ADMINISTRATOR),
            "{extract:?}"
        );
        let compress = plan(
            SelectionAction::Compress(CompressHow::TarGz),
            &as_administrator(),
        );
        assert!(
            matches!(compress, Plan::Say(ref words) if words == AS_ADMINISTRATOR),
            "{compress:?}"
        );
    }

    #[test]
    fn compress_names_come_from_the_listing_and_avoid_every_name_taken() {
        // "photos.zip" is taken, though it may be hidden by the filter.
        let listing: &[&[u8]] = &[b"photos", b"photos.zip", b"photos (1).zip"];
        let (made, _) = requests(plan(
            SelectionAction::Compress(CompressHow::Zip),
            &facts(vec![dir("photos")], listing),
        ));
        assert_eq!(
            made,
            vec![Request::Compress {
                sources: vec![at("photos")],
                archive: at("photos (2).zip"),
                format: Format::Zip,
                level: Level::Normal,
            }]
        );
        let (made, _) = requests(plan(
            SelectionAction::Compress(CompressHow::TarGz),
            &facts(vec![dir("photos")], listing),
        ));
        assert!(
            matches!(&made[..], [Request::Compress { archive, format: Format::TarGz, .. }]
                if *archive == at("photos.tar.gz"))
        );
        // A folder keeps its whole name, dots and all, because the listing
        // says it is a folder: nothing is read from the disk for it.
        match plan(
            SelectionAction::Compress(CompressHow::Choose),
            &facts(vec![dir("v1.2")], &[b"v1.2", b"v1.2.zip"]),
        ) {
            Plan::Open(Dialog::Compress(state)) => {
                assert_eq!(state.name, "v1.2");
                assert_eq!(state.sources, vec![at("v1.2")]);
                assert_eq!(state.folder, PathBuf::from(HOME));
                assert!(state.taken.contains(b"v1.2.zip".as_slice()));
                assert!(!state.note().1, "v1.2.zip is taken");
            }
            other => panic!("no dialog: {other:?}"),
        }
        let none = plan(
            SelectionAction::Compress(CompressHow::Zip),
            &facts(vec![], listing),
        );
        assert!(matches!(none, Plan::Say(words) if words == "Select what to compress."));
    }

    #[test]
    fn batch_rename_takes_two_or_more_and_keeps_clear_of_every_other_name() {
        let listing: &[&[u8]] = &[b"a.txt", b"b.txt", b".hidden", b"c.txt"];
        let one = plan(
            SelectionAction::BatchRename,
            &facts(vec![file("a.txt")], listing),
        );
        assert!(matches!(one, Plan::Nothing));
        let in_trash = plan(
            SelectionAction::BatchRename,
            &Facts {
                in_trash: true,
                ..facts(vec![file("a.txt"), file("b.txt")], listing)
            },
        );
        assert!(matches!(in_trash, Plan::Nothing));
        match plan(
            SelectionAction::BatchRename,
            &facts(vec![file("a.txt"), file("b.txt")], listing),
        ) {
            Plan::Open(Dialog::BatchRename(state)) => {
                assert_eq!(state.directory, PathBuf::from(HOME));
                assert_eq!(
                    state.selected,
                    vec![OsString::from("a.txt"), "b.txt".into()]
                );
                let mut staying = state.staying.clone();
                staying.sort();
                assert_eq!(staying, vec![OsString::from(".hidden"), "c.txt".into()]);
                assert_eq!(state.view().title, "Rename 2 items");
            }
            other => panic!("no dialog: {other:?}"),
        }
    }

    #[test]
    fn duplicates_and_new_entries_get_names_nothing_listed_has() {
        // "a copy.txt" is what the engine would make first.
        let listing: &[&[u8]] = &[b"a.txt", b"a copy.txt", b"New Folder", b"New File"];
        let (made, select) = requests(plan(
            SelectionAction::Duplicate,
            &facts(vec![file("a.txt")], listing),
        ));
        assert_eq!(
            made,
            vec![Request::Duplicate {
                sources: vec![at("a.txt")]
            }]
        );
        let Select::All(names) = select else {
            panic!("copies not selected: {select:?}");
        };
        assert_eq!(names, vec![b"a copy (1).txt".to_vec()]);

        let (made, select) = requests(plan(SelectionAction::NewFolder, &facts(vec![], listing)));
        assert_eq!(
            made,
            vec![Request::CreateDir {
                parent: PathBuf::from(HOME),
                name: "New Folder 2".into(),
            }]
        );
        assert_eq!(select, Select::AndRename(b"New Folder 2".to_vec()));
        let (made, _) = requests(plan(SelectionAction::NewFile, &facts(vec![], listing)));
        assert_eq!(
            made,
            vec![Request::CreateFile {
                parent: PathBuf::from(HOME),
                name: "New File 2".into(),
            }]
        );
        for action in [
            SelectionAction::Duplicate,
            SelectionAction::NewFolder,
            SelectionAction::NewFile,
        ] {
            let facts = Facts {
                in_trash: true,
                ..facts(vec![file("a.txt")], listing)
            };
            assert!(matches!(plan(action, &facts), Plan::Nothing), "{action:?}");
        }
    }

    #[test]
    fn only_extracting_reads_which_entries_are_archives() {
        assert!(SelectionAction::Extract(ExtractHow::Here).reads_archives());
        assert!(SelectionAction::Extract(ExtractHow::To).reads_archives());
        assert!(!SelectionAction::Compress(CompressHow::Zip).reads_archives());
        assert!(!SelectionAction::MoveToTrash.reads_archives());
    }

    #[test]
    fn dialogs_waiting_together_are_each_taken_by_their_own_ticket() {
        let mut hand_off = HandOff::default();
        let extract = |n: usize| {
            Dialog::ExtractTo(ExtractTo {
                folder: PathBuf::from(HOME),
                archives: vec![at(&format!("{n}.zip"))],
                trash: false,
            })
        };
        let tickets: Vec<i32> = (0..WAITING).map(|n| hand_off.keep(extract(n))).collect();
        for (n, ticket) in tickets.into_iter().enumerate() {
            assert!(
                matches!(hand_off.take(ticket), Some(Dialog::ExtractTo(to)) if to.archives == [at(&format!("{n}.zip"))]),
                "dialog {n}"
            );
        }
    }

    #[test]
    fn a_dialog_is_taken_once_by_its_ticket_and_old_ones_give_way() {
        let mut hand_off = HandOff::default();
        let extract = |n: usize| {
            Dialog::ExtractTo(ExtractTo {
                folder: PathBuf::from(HOME),
                archives: vec![at(&format!("{n}.zip"))],
                trash: false,
            })
        };
        let first = hand_off.keep(extract(1));
        let second = hand_off.keep(extract(2));
        assert_ne!(first, 0);
        assert_ne!(first, second);
        assert!(
            matches!(hand_off.take(second), Some(Dialog::ExtractTo(to)) if to.archives == [at("2.zip")])
        );
        assert!(hand_off.take(second).is_none(), "taken once");
        for n in 3..3 + WAITING {
            hand_off.keep(extract(n));
        }
        assert!(hand_off.take(first).is_none(), "the oldest gave way");
    }
}
