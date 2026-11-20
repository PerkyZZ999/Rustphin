//! The state of the Compress to… and batch rename dialogs (W-111d, S9,
//! W-114f), free of any toolkit: what is typed and chosen, what the
//! dialog says about it, and the request it makes.

use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

use rustphin_core::{
    CaseChange, Preview, RenameRules,
    archive::{Format, Level},
    display_name,
    ops::Request,
};
use serde::Serialize;

use crate::operations::folder_name;

/// The formats offered, in the menu's order: ZIP first (D-032).
pub const FORMATS: [(Format, &str); 5] = [
    (Format::Zip, "ZIP"),
    (Format::TarGz, "TAR.GZ"),
    (Format::TarXz, "TAR.XZ"),
    (Format::TarZst, "TAR.ZST"),
    (Format::SevenZ, "7Z"),
];

/// The compression levels offered.
pub const LEVELS: [(Level, &str); 3] = [
    (Level::Fast, "Fast"),
    (Level::Normal, "Normal"),
    (Level::Smallest, "Smallest"),
];

/// The file name `name` and `format` give, or what is wrong with it.
pub fn archive_name(
    name: &str,
    format: Format,
    taken: &HashSet<Vec<u8>>,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Type a name for the archive.".into());
    }
    if name.contains('/') || name.contains('\0') {
        return Err("A name cannot contain \u{201c}/\u{201d}.".into());
    }
    let file = format!("{name}.{}", format.extension());
    if file.len() > 255 {
        return Err("The name is too long.".into());
    }
    if taken.contains(file.as_bytes()) {
        return Err(format!(
            "\u{201c}{file}\u{201d} already exists here. Choose another name."
        ));
    }
    Ok(file)
}

/// What each format is good for, said under the choices.
pub fn format_hint(format: Format) -> &'static str {
    match format {
        Format::Zip => "ZIP opens everywhere, including Windows and phones.",
        Format::TarGz => "TAR.GZ is the usual choice on Linux.",
        Format::TarXz => "TAR.XZ is smaller and slower to make.",
        Format::TarZst => "TAR.ZST is fast and small; older systems may not open it.",
        Format::SevenZ => "7Z is small; links are not kept.",
        Format::Tar | Format::TarBz2 | Format::Foreign => "",
    }
}

/// The Compress to… dialog.
#[derive(Debug, Clone)]
pub struct Compress {
    /// Entries to pack.
    pub sources: Vec<PathBuf>,
    /// Where the archive goes.
    pub folder: PathBuf,
    /// Every name in the folder.
    pub taken: HashSet<Vec<u8>>,
    /// The name typed, without ending.
    pub name: String,
    /// The format chosen (an index of [`FORMATS`]).
    pub format: usize,
    /// The level chosen (an index of [`LEVELS`]).
    pub level: usize,
}

impl Compress {
    /// The dialog for these entries, offering `base` as the name.
    pub fn new(
        sources: Vec<PathBuf>,
        folder: PathBuf,
        taken: HashSet<Vec<u8>>,
        base: String,
    ) -> Self {
        Self {
            sources,
            folder,
            taken,
            name: base,
            format: 0,
            level: 1,
        }
    }

    fn chosen(&self) -> (Format, Level) {
        (
            FORMATS
                .get(self.format)
                .map_or(Format::Zip, |(format, _)| *format),
            LEVELS
                .get(self.level)
                .map_or(Level::Normal, |(level, _)| *level),
        )
    }

    /// Its title.
    pub fn title(&self) -> String {
        match self.sources.as_slice() {
            [one] => format!("Compress {}", folder_name(one)),
            many => format!("Compress {} items", many.len()),
        }
    }

    /// What it would make, or what is wrong; and whether it can.
    pub fn note(&self) -> (String, bool) {
        let (format, _) = self.chosen();
        match archive_name(&self.name, format, &self.taken) {
            Ok(file) => (
                format!(
                    "Makes \u{201c}{file}\u{201d} in {}. {}",
                    folder_name(&self.folder),
                    format_hint(format)
                ),
                true,
            ),
            Err(problem) => (problem, false),
        }
    }

    /// The request it makes now, if the name is usable.
    pub fn request(&self) -> Option<Request> {
        let (format, level) = self.chosen();
        let file = archive_name(&self.name, format, &self.taken).ok()?;
        Some(Request::Compress {
            sources: self.sources.clone(),
            archive: self.folder.join(file),
            format,
            level,
        })
    }
}

/// How batch rename makes the new names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Find and replace.
    #[default]
    Replace,
    /// A pattern with a counter.
    Pattern,
    /// Change of case.
    Case,
}

/// The batch rename dialog (S9).
#[derive(Debug, Clone)]
pub struct BatchRename {
    /// The folder.
    pub directory: PathBuf,
    /// The names chosen, in list order.
    pub selected: Vec<OsString>,
    /// Every other name in the folder.
    pub staying: Vec<OsString>,
    /// How names are made.
    pub mode: Mode,
    /// Text to find.
    pub find: String,
    /// Its replacement.
    pub replace: String,
    /// Find is a regular expression.
    pub regex: bool,
    /// The pattern ("@-#").
    pub pattern: String,
    /// The counter's start, as typed.
    pub start: String,
    /// The counter's step, as typed.
    pub step: String,
    /// The case change.
    pub case: CaseChange,
}

/// One row of the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The current name.
    pub from: String,
    /// The new name.
    pub to: String,
    /// It changes.
    pub changes: bool,
    /// What is wrong with it, in words.
    pub problems: String,
}

/// What the dialog shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchView {
    /// The title.
    pub title: String,
    /// What is wrong with the rule or the plan.
    pub problem: String,
    /// What is wrong with the new names, each said once.
    pub row_problems: String,
    /// Nothing would change.
    pub unchanged: bool,
    /// Rename can be pressed.
    pub ready: bool,
    /// The preview.
    pub rows: Vec<Row>,
}

/// A whole number as typed, or `None`.
fn whole_number(value: &str) -> Option<i64> {
    value.trim().parse().ok()
}

/// The problems of the preview's rows, each said once, in the order they
/// first appear; `None` when every row is fine.
pub fn row_problems(rows: impl IntoIterator<Item = Vec<String>>) -> Option<String> {
    let mut seen: Vec<String> = Vec::new();
    for message in rows.into_iter().flatten() {
        if !seen.contains(&message) {
            seen.push(message);
        }
    }
    (!seen.is_empty()).then(|| seen.join(" "))
}

impl BatchRename {
    /// The dialog for `selected` in `directory`.
    pub fn new(directory: PathBuf, selected: Vec<OsString>, staying: Vec<OsString>) -> Self {
        Self {
            directory,
            selected,
            staying,
            mode: Mode::Replace,
            find: String::new(),
            replace: String::new(),
            regex: false,
            pattern: "@-#".to_owned(),
            start: "1".to_owned(),
            step: "1".to_owned(),
            case: CaseChange::Lower,
        }
    }

    fn rules(&self) -> Result<RenameRules, String> {
        match self.mode {
            Mode::Replace => Ok(RenameRules::replace(
                self.find.clone(),
                self.replace.clone(),
                self.regex,
            )),
            Mode::Pattern => match (whole_number(&self.start), whole_number(&self.step)) {
                (Some(start), Some(step)) => {
                    Ok(RenameRules::pattern(self.pattern.clone(), start, step))
                }
                _ => Err("Start and step have to be whole numbers, such as 1 or -2.".to_owned()),
            },
            Mode::Case => Ok(RenameRules::case(self.case)),
        }
    }

    fn preview(&self) -> Result<Preview, String> {
        self.rules()
            .map(|rules| rules.preview(&self.selected, &self.staying))
    }

    /// What the dialog shows now.
    pub fn view(&self) -> BatchView {
        let title = format!("Rename {} items", self.selected.len());
        match self.preview() {
            Err(problem) => BatchView {
                title,
                problem,
                row_problems: String::new(),
                unchanged: false,
                ready: false,
                rows: Vec::new(),
            },
            Ok(preview) => {
                let changed = preview.rows.iter().any(|row| row.from != row.to);
                let shown = |name: &OsString| display_name(name.as_encoded_bytes()).into_owned();
                BatchView {
                    title,
                    problem: preview.plan_problem.clone().unwrap_or_default(),
                    row_problems: row_problems(preview.rows.iter().map(|row| {
                        row.problems
                            .iter()
                            .map(|problem| problem.message())
                            .collect()
                    }))
                    .unwrap_or_default(),
                    unchanged: !changed && preview.can_apply(),
                    ready: preview.can_apply() && changed,
                    rows: preview
                        .rows
                        .iter()
                        .map(|row| Row {
                            from: shown(&row.from),
                            to: shown(&row.to),
                            changes: row.from != row.to,
                            problems: row
                                .problems
                                .iter()
                                .map(|problem| problem.message())
                                .collect::<Vec<_>>()
                                .join(" "),
                        })
                        .collect(),
                }
            }
        }
    }

    /// The request to run, when the preview has a real change and no problem.
    pub fn request(&self) -> Option<Request> {
        let preview = self.preview().ok()?;
        let changed = preview.rows.iter().any(|row| row.from != row.to);
        if !preview.can_apply() || !changed {
            return None;
        }
        Some(Request::RenameBatch {
            directory: self.directory.clone(),
            pairs: preview
                .rows
                .iter()
                .map(|row| (row.from.clone(), row.to.clone()))
                .collect(),
        })
    }
}

/// The folder a dialog's entries are in, by name, for its words.
pub fn place_of(path: &Path) -> String {
    folder_name(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_gets_the_formats_ending_and_a_taken_one_is_refused() {
        let taken: HashSet<Vec<u8>> = [b"report.zip".to_vec()].into_iter().collect();
        assert_eq!(
            archive_name("  photos ", Format::TarXz, &taken),
            Ok("photos.tar.xz".into())
        );
        assert!(
            archive_name("report", Format::Zip, &taken)
                .is_err_and(|problem| problem.contains("already exists"))
        );
        assert_eq!(
            archive_name("   ", Format::Zip, &taken),
            Err("Type a name for the archive.".into())
        );
        assert_eq!(
            archive_name("a/b", Format::Zip, &taken),
            Err("A name cannot contain \u{201c}/\u{201d}.".into())
        );
        assert_eq!(
            archive_name("a\0b", Format::Zip, &taken),
            Err("A name cannot contain \u{201c}/\u{201d}.".into())
        );
        assert_eq!(
            archive_name(&"x".repeat(260), Format::Zip, &taken),
            Err("The name is too long.".into())
        );
        // 251 bytes and ".zip" make the longest name a folder holds.
        assert_eq!(
            archive_name(&"x".repeat(251), Format::Zip, &taken).map(|name| name.len()),
            Ok(255)
        );
        assert_eq!(
            archive_name(&"x".repeat(252), Format::Zip, &taken),
            Err("The name is too long.".into())
        );
    }

    #[test]
    fn compressing_offers_zip_then_makes_the_chosen_archive() {
        let mut compress = Compress::new(
            vec!["/srv/a".into(), "/srv/b".into()],
            "/srv".into(),
            HashSet::new(),
            "Archive".into(),
        );
        assert_eq!(compress.title(), "Compress 2 items");
        assert_eq!(
            compress.note(),
            (
                "Makes \u{201c}Archive.zip\u{201d} in \u{201c}srv\u{201d}. ZIP opens everywhere, including Windows and phones.".to_owned(),
                true
            )
        );
        compress.format = 2;
        compress.level = 2;
        assert!(matches!(
            compress.request(),
            Some(Request::Compress { archive, format: Format::TarXz, level: Level::Smallest, .. })
                if archive == Path::new("/srv/Archive.tar.xz")
        ));
        compress.name = String::new();
        assert!(compress.request().is_none() && !compress.note().1);
    }

    #[test]
    fn a_pattern_numbers_the_files_and_bad_numbers_are_said() {
        let mut batch = BatchRename::new(
            "/srv".into(),
            vec!["b.jpg".into(), "a.jpg".into()],
            vec!["c.jpg".into()],
        );
        assert!(!batch.view().ready, "an empty find renames nothing");
        batch.mode = Mode::Pattern;
        batch.pattern = "photo-##".into();
        let view = batch.view();
        assert!(view.ready);
        assert_eq!(view.rows[0].to, "photo-01.jpg");
        assert_eq!(
            batch.request(),
            Some(Request::RenameBatch {
                directory: "/srv".into(),
                pairs: vec![
                    ("b.jpg".into(), "photo-01.jpg".into()),
                    ("a.jpg".into(), "photo-02.jpg".into()),
                ],
            })
        );
        batch.step = "two".into();
        assert!(batch.view().problem.contains("whole numbers"));
        assert!(batch.request().is_none());
    }

    #[test]
    fn problems_shared_by_rows_are_said_once_in_order() {
        let rows = vec![
            vec!["Names cannot contain /.".to_owned()],
            vec!["Names cannot contain /.".to_owned(), "Too long.".to_owned()],
        ];
        assert_eq!(
            row_problems(rows).as_deref(),
            Some("Names cannot contain /. Too long.")
        );
        assert_eq!(row_problems(Vec::<Vec<String>>::new()), None);
    }

    fn batch(selected: Vec<OsString>, staying: Vec<OsString>) -> BatchRename {
        BatchRename::new("/srv".into(), selected, staying)
    }

    #[test]
    fn a_new_name_taken_by_a_file_left_alone_is_refused() {
        let mut batch = batch(vec!["a.jpg".into(), "b.jpg".into()], vec!["c.jpg".into()]);
        batch.find = "a".into();
        batch.replace = "c".into();
        let view = batch.view();
        assert!(!view.ready && !view.unchanged);
        assert!(!view.rows[0].problems.is_empty(), "the clash is said");
        assert_eq!(view.rows[1].problems, "");
        assert_eq!(view.row_problems, view.rows[0].problems);
        assert_eq!(batch.request(), None);
    }

    #[test]
    fn names_that_are_not_utf8_are_renamed_byte_for_byte() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _};
        let latin1 = OsStr::from_bytes(b"caf\xe9.txt").to_owned();
        let mut batch = batch(vec![latin1, "b.txt".into()], Vec::new());
        batch.find = ".txt".into();
        batch.replace = ".md".into();
        assert_eq!(
            batch.request(),
            Some(Request::RenameBatch {
                directory: "/srv".into(),
                pairs: vec![
                    (
                        OsStr::from_bytes(b"caf\xe9.txt").to_owned(),
                        OsStr::from_bytes(b"caf\xe9.md").to_owned()
                    ),
                    ("b.txt".into(), "b.md".into()),
                ],
            })
        );
    }

    #[test]
    fn the_preview_says_which_rows_change_and_whether_anything_would() {
        let mut batch = batch(vec!["a.txt".into(), "b.jpg".into()], Vec::new());
        batch.find = "zzz".into();
        let view = batch.view();
        assert!(view.unchanged && !view.ready);
        assert!(view.rows.iter().all(|row| !row.changes));
        assert_eq!(batch.request(), None);
        batch.find = ".txt".into();
        batch.replace = ".md".into();
        let view = batch.view();
        assert!(view.ready && !view.unchanged);
        assert_eq!(
            view.rows.iter().map(|row| row.changes).collect::<Vec<_>>(),
            [true, false]
        );
    }

    #[test]
    fn a_dialog_names_its_folder() {
        assert_eq!(place_of(Path::new("/srv/data")), "\u{201c}data\u{201d}");
    }
}
