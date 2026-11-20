//! A batch-rename plan and its preview. Nothing here touches the filesystem.
//!
//! One plan does one kind of change: replace text, build names from a pattern,
//! or change case. The preview names every selected file and says why a result
//! cannot be applied. Applying the plan is a later step.

use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
};

use crate::ops::validate_name;

/// How the selected names are rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameRules {
    transform: Transform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Transform {
    Replace {
        find: String,
        replace: String,
        regex: bool,
    },
    Pattern {
        pattern: String,
        start: i64,
        step: i64,
    },
    Case(CaseChange),
}

/// A change of case applied to a whole name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseChange {
    /// Unicode lowercase.
    Lower,
    /// Unicode uppercase.
    Upper,
    /// The first character of each word in uppercase and the rest in lowercase.
    /// A word is a run of letters or digits.
    Words,
}

/// Why one new name cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameProblem {
    /// Two files in this plan would receive this name.
    DuplicateTarget,
    /// A file that is not changing already has this name.
    ExistingName,
    /// The name is empty, reserved, too long, or contains `/` or a null.
    Invalid(&'static str),
    /// Case was requested, but the current name is not Unicode text.
    NotText,
}

impl NameProblem {
    /// A sentence naming the problem and what has to change.
    pub fn message(&self) -> String {
        match self {
            Self::DuplicateTarget => {
                "Another file in this rename would get the same name. Give each file its own name."
                    .to_owned()
            }
            Self::ExistingName => {
                "A file that is staying already has this name. Choose a name that is free."
                    .to_owned()
            }
            Self::Invalid(reason) => {
                format!(
                    "This name cannot be used ({reason}). Change the rule so the name is a single entry."
                )
            }
            Self::NotText => {
                "This name is not text, so its case cannot be changed. Rename it as text first."
                    .to_owned()
            }
        }
    }
}

/// One selected file, in the order it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRow {
    /// The name as it is now.
    pub from: OsString,
    /// The name the rule produces. Unchanged when the rule cannot be applied to it.
    pub to: OsString,
    /// Empty when `to` may be used.
    pub problems: Vec<NameProblem>,
}

/// The whole selection, plus a problem that applies to the rule itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Why the rule cannot be used at all, when it is malformed.
    pub plan_problem: Option<String>,
    /// One row per selected name, in the same order.
    pub rows: Vec<PreviewRow>,
}

impl Preview {
    /// False when the rule is malformed or any row has a problem.
    /// A plan that changes nothing is allowed: applying it does nothing.
    pub fn can_apply(&self) -> bool {
        self.plan_problem.is_none() && self.rows.iter().all(|row| row.problems.is_empty())
    }
}

impl RenameRules {
    /// Replace every non-overlapping copy of `find` with `replace`.
    ///
    /// With `regex`, `find` is a regular expression and `replace` may refer to
    /// captured groups as `$1`, `$2` and so on. An empty `find` is rejected.
    pub fn replace(find: impl Into<String>, replace: impl Into<String>, regex: bool) -> Self {
        Self {
            transform: Transform::Replace {
                find: find.into(),
                replace: replace.into(),
                regex,
            },
        }
    }

    /// Build each name from `pattern`.
    ///
    /// A run of `#` is the counter for that file, padded with zeros to the
    /// length of the run. The first file receives `start`; each later file
    /// adds `step`. `@` is the file's current name without its extension.
    /// `\#`, `\@` and `\\` are those characters literally. The pattern
    /// replaces the name's stem; each file keeps its extension (see
    /// [`extension_start`]), as in Dolphin.
    pub fn pattern(pattern: impl Into<String>, start: i64, step: i64) -> Self {
        Self {
            transform: Transform::Pattern {
                pattern: pattern.into(),
                start,
                step,
            },
        }
    }

    /// Change the case of each whole name.
    pub fn case(change: CaseChange) -> Self {
        Self {
            transform: Transform::Case(change),
        }
    }

    /// Names the selection would have, without writing anything.
    ///
    /// `selected` is the counter order. `staying` is every other name in the
    /// folder; a name that is also in `selected` is ignored there, because
    /// that file is part of the plan. Names are compared as raw bytes.
    pub fn preview(&self, selected: &[OsString], staying: &[OsString]) -> Preview {
        let mut plan_problem = None;
        let mut rows: Vec<PreviewRow> = selected
            .iter()
            .map(|name| PreviewRow {
                from: name.clone(),
                to: name.clone(),
                problems: Vec::new(),
            })
            .collect();

        match &self.transform {
            Transform::Replace {
                find,
                replace,
                regex,
            } => {
                if find.is_empty() {
                    plan_problem = Some(
                        "There is nothing to find. Enter the text that should be replaced."
                            .to_owned(),
                    );
                } else if *regex {
                    match regex::bytes::Regex::new(find) {
                        Ok(expression) => {
                            for row in &mut rows {
                                let replaced =
                                    expression.replace_all(row.from.as_bytes(), replace.as_bytes());
                                row.to = OsStr::from_bytes(&replaced).to_owned();
                            }
                        }
                        Err(error) => {
                            plan_problem = Some(format!(
                                "The regular expression could not be read ({error}). Check it and try again."
                            ));
                        }
                    }
                } else {
                    let find_bytes = find.as_bytes();
                    let replace_bytes = replace.as_bytes();
                    for row in &mut rows {
                        row.to = OsStr::from_bytes(&replace_bytes_all(
                            row.from.as_bytes(),
                            find_bytes,
                            replace_bytes,
                        ))
                        .to_owned();
                    }
                }
            }
            Transform::Pattern {
                pattern,
                start,
                step,
            } => match parse_pattern(pattern) {
                Ok(pieces) => fill_pattern(&pieces, *start, *step, &mut rows),
                Err(problem) => plan_problem = Some(problem),
            },
            Transform::Case(change) => {
                for row in &mut rows {
                    match std::str::from_utf8(row.from.as_bytes()) {
                        Ok(text) => {
                            let changed = match change {
                                CaseChange::Lower => text.to_lowercase(),
                                CaseChange::Upper => text.to_uppercase(),
                                CaseChange::Words => words_case(text),
                            };
                            row.to = OsString::from(changed);
                        }
                        Err(_) => row.problems.push(NameProblem::NotText),
                    }
                }
            }
        }

        if plan_problem.is_none() {
            flag_names(&mut rows, staying);
        }

        Preview { plan_problem, rows }
    }
}

fn replace_bytes_all(name: &[u8], find: &[u8], replace: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.len());
    let mut rest = name;
    while let Some(at) = rest.windows(find.len()).position(|window| window == find) {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(replace);
        rest = &rest[at + find.len()..];
    }
    out.extend_from_slice(rest);
    out
}

enum Piece {
    Literal(Vec<u8>),
    Counter { width: usize },
    Original,
}

fn parse_pattern(pattern: &str) -> Result<Vec<Piece>, String> {
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut chars = pattern.chars().peekable();
    let flush = |literal: &mut String, pieces: &mut Vec<Piece>| {
        if !literal.is_empty() {
            pieces.push(Piece::Literal(literal.as_bytes().to_vec()));
            literal.clear();
        }
    };
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some(escaped @ ('\\' | '#' | '@')) => literal.push(escaped),
                Some(other) => {
                    return Err(format!(
                        "The pattern's \\{other} is not a known escape. Use \\\\, \\# or \\@."
                    ));
                }
                None => {
                    return Err(
                        "The pattern ends with a \\. Remove it, or add the character it should keep."
                            .to_owned(),
                    );
                }
            },
            '#' => {
                flush(&mut literal, &mut pieces);
                let mut width = 1usize;
                while chars.peek() == Some(&'#') {
                    chars.next();
                    width += 1;
                }
                if width > 64 {
                    return Err(
                        "A counter can be at most 64 digits. Shorten the run of #.".to_owned()
                    );
                }
                pieces.push(Piece::Counter { width });
            }
            '@' => {
                flush(&mut literal, &mut pieces);
                pieces.push(Piece::Original);
            }
            other => literal.push(other),
        }
    }
    flush(&mut literal, &mut pieces);
    Ok(pieces)
}

/// Where the extension of `name` starts, as Dolphin keeps it when it renames
/// or duplicates: at the last dot, or at `.tar` for a compound `.tar.*`
/// ending; a leading dot (`.bashrc`) is not an extension. `name.len()` when
/// there is none.
pub fn extension_start(name: &[u8]) -> usize {
    match name
        .iter()
        .rposition(|&byte| byte == b'.')
        .filter(|&dot| dot > 0)
    {
        Some(dot) if dot > 4 && name[..dot].ends_with(b".tar") => dot - 4,
        Some(dot) => dot,
        None => name.len(),
    }
}

/// Fills the pattern for each row. `@` and the counters make the name's stem;
/// each file keeps its own extension, as Dolphin's batch rename does.
fn fill_pattern(pieces: &[Piece], start: i64, step: i64, rows: &mut [PreviewRow]) {
    let mut counter = start;
    let last = rows.len().saturating_sub(1);
    for (index, row) in rows.iter_mut().enumerate() {
        let from = row.from.as_bytes();
        let (stem, extension) = from.split_at(extension_start(from));
        let mut name = Vec::new();
        for piece in pieces {
            match piece {
                Piece::Literal(bytes) => name.extend_from_slice(bytes),
                Piece::Original => name.extend_from_slice(stem),
                Piece::Counter { width } => {
                    name.extend_from_slice(format_counter(counter, *width).as_bytes());
                }
            }
        }
        name.extend_from_slice(extension);
        row.to = OsStr::from_bytes(&name).to_owned();
        if index == last {
            break;
        }
        match counter.checked_add(step) {
            Some(next) => counter = next,
            None => {
                for later in &mut rows[index + 1..] {
                    later
                        .problems
                        .push(NameProblem::Invalid("the counter is too large"));
                }
                break;
            }
        }
    }
}

fn format_counter(value: i64, width: usize) -> String {
    let digits = value.unsigned_abs().to_string();
    let pad = width.saturating_sub(digits.len());
    let mut out = String::with_capacity(usize::from(value < 0) + pad + digits.len());
    if value < 0 {
        out.push('-');
    }
    out.extend(std::iter::repeat_n('0', pad));
    out.push_str(&digits);
    out
}

fn words_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_uppercase());
            while chars.peek().is_some_and(|next| next.is_alphanumeric()) {
                let next = chars.next().unwrap_or(ch);
                out.extend(next.to_lowercase());
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn flag_names(rows: &mut [PreviewRow], staying: &[OsString]) {
    let moving: HashSet<Vec<u8>> = rows
        .iter()
        .map(|row| row.from.as_bytes().to_vec())
        .collect();
    let mut occupied: HashSet<Vec<u8>> = staying
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .filter(|name| !moving.contains(name))
        .collect();
    for row in rows.iter() {
        if row.to.as_bytes() == row.from.as_bytes() {
            occupied.insert(row.from.as_bytes().to_vec());
        }
    }

    let mut seen = HashSet::new();
    let mut duplicated = HashSet::new();
    for row in rows.iter() {
        let name = row.to.as_bytes().to_vec();
        if !seen.insert(name.clone()) {
            duplicated.insert(name);
        }
    }

    for row in rows.iter_mut() {
        if let Err(error) = validate_name(&row.to)
            && row.to.as_bytes() != row.from.as_bytes()
        {
            let crate::ops::OpError::InvalidName { reason, .. } = error else {
                continue;
            };
            row.problems.push(NameProblem::Invalid(reason));
        }
        let target = row.to.as_bytes().to_vec();
        if duplicated.contains(&target) {
            row.problems.push(NameProblem::DuplicateTarget);
        }
        if row.to.as_bytes() != row.from.as_bytes() && occupied.contains(&target) {
            row.problems.push(NameProblem::ExistingName);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn row(preview: &Preview, index: usize) -> &PreviewRow {
        &preview.rows[index]
    }

    #[test]
    fn plain_replace_changes_every_copy_and_leaves_the_rest() {
        let preview = RenameRules::replace("img", "photo", false)
            .preview(&names(&["img_img.JPG", "note.txt"]), &[]);
        assert_eq!(row(&preview, 0).to, "photo_photo.JPG");
        assert_eq!(row(&preview, 1).to, "note.txt");
        assert!(preview.can_apply());
    }

    #[test]
    fn plain_replace_matches_bytes_that_are_not_text() {
        let raw = OsStr::from_bytes(b"a\xffa").to_owned();
        let preview = RenameRules::replace("a", "b", false).preview(&[raw], &[]);
        assert_eq!(row(&preview, 0).to.as_bytes(), b"b\xffb");
        assert!(preview.can_apply());
    }

    #[test]
    fn an_empty_search_is_rejected_before_any_name_changes() {
        let preview = RenameRules::replace("", "x", false).preview(&names(&["a"]), &[]);
        assert!(preview.plan_problem.is_some());
        assert_eq!(row(&preview, 0).to, "a");
        assert!(!preview.can_apply());
    }

    #[test]
    fn a_regular_expression_can_keep_a_captured_group() {
        let preview =
            RenameRules::replace(r"(\d+)", "n$1", true).preview(&names(&["shot12.jpg"]), &[]);
        assert_eq!(row(&preview, 0).to, "shotn12.jpg");
        assert!(preview.can_apply());
    }

    #[test]
    fn a_broken_regular_expression_names_the_problem() {
        let preview = RenameRules::replace("(", "x", true).preview(&names(&["a"]), &[]);
        assert!(
            preview
                .plan_problem
                .as_ref()
                .is_some_and(|problem| problem.contains("regular expression"))
        );
        assert!(!preview.can_apply());
    }

    #[test]
    fn the_counter_uses_start_step_and_padding() {
        let preview = RenameRules::pattern("day-##", 3, 2).preview(&names(&["a", "b", "c"]), &[]);
        assert_eq!(row(&preview, 0).to, "day-03");
        assert_eq!(row(&preview, 1).to, "day-05");
        assert_eq!(row(&preview, 2).to, "day-07");
    }

    #[test]
    fn a_negative_step_counts_down_and_keeps_the_sign_outside_the_padding() {
        let preview = RenameRules::pattern("###", 1, -2).preview(&names(&["a", "b"]), &[]);
        assert_eq!(row(&preview, 0).to, "001");
        assert_eq!(row(&preview, 1).to, "-001");
    }

    #[test]
    fn the_pattern_keeps_the_current_name_and_a_literal_hash() {
        let preview = RenameRules::pattern(r"\@-@-#", 1, 1).preview(&names(&["cat"]), &[]);
        assert_eq!(row(&preview, 0).to, "@-cat-1");
    }

    #[test]
    fn the_pattern_names_the_stem_and_each_file_keeps_its_extension() {
        let preview = RenameRules::pattern("@-#", 1, 1).preview(
            &names(&[
                "IMG_4819.jpg",
                "notes.tar.gz",
                ".bashrc",
                "README",
                "a.b.PNG",
            ]),
            &[],
        );
        assert_eq!(row(&preview, 0).to, "IMG_4819-1.jpg");
        assert_eq!(row(&preview, 1).to, "notes-2.tar.gz");
        assert_eq!(row(&preview, 2).to, ".bashrc-3");
        assert_eq!(row(&preview, 3).to, "README-4");
        assert_eq!(row(&preview, 4).to, "a.b-5.PNG");
    }

    #[test]
    fn a_pattern_without_the_name_still_keeps_the_extension() {
        let preview =
            RenameRules::pattern("holiday ##", 1, 1).preview(&names(&["x.jpg", "y.mp4"]), &[]);
        assert_eq!(row(&preview, 0).to, "holiday 01.jpg");
        assert_eq!(row(&preview, 1).to, "holiday 02.mp4");
    }

    #[test]
    fn a_trailing_backslash_is_a_problem_with_the_pattern() {
        let preview = RenameRules::pattern(r"name\", 1, 1).preview(&names(&["a"]), &[]);
        assert!(preview.plan_problem.is_some());
        assert_eq!(row(&preview, 0).to, "a");
        assert!(!preview.can_apply());
    }

    #[test]
    fn a_counter_that_overflows_blocks_the_later_files() {
        let preview = RenameRules::pattern("#", i64::MAX, 1).preview(&names(&["a", "b"]), &[]);
        assert!(row(&preview, 0).problems.is_empty());
        assert_eq!(row(&preview, 0).to, OsString::from(i64::MAX.to_string()));
        assert!(
            row(&preview, 1)
                .problems
                .contains(&NameProblem::Invalid("the counter is too large"))
        );
        assert!(!preview.can_apply());
    }

    #[test]
    fn case_changes_use_unicode_and_word_boundaries() {
        let lower = RenameRules::case(CaseChange::Lower).preview(&names(&["ÉCOLE.JPG"]), &[]);
        assert_eq!(row(&lower, 0).to, "école.jpg");
        let upper = RenameRules::case(CaseChange::Upper).preview(&names(&["école"]), &[]);
        assert_eq!(row(&upper, 0).to, "ÉCOLE");
        let words = RenameRules::case(CaseChange::Words).preview(&names(&["img_0123.jpg"]), &[]);
        assert_eq!(row(&words, 0).to, "Img_0123.Jpg");
    }

    #[test]
    fn case_cannot_be_changed_when_the_name_is_not_text() {
        let raw = OsStr::from_bytes(b"a\xff").to_owned();
        let preview = RenameRules::case(CaseChange::Lower).preview(std::slice::from_ref(&raw), &[]);
        assert_eq!(row(&preview, 0).to, raw);
        assert_eq!(row(&preview, 0).problems, vec![NameProblem::NotText]);
        assert!(!preview.can_apply());
    }

    #[test]
    fn two_files_given_the_same_name_are_both_flagged() {
        let preview = RenameRules::pattern("same", 1, 1).preview(&names(&["a", "b"]), &[]);
        assert!(
            row(&preview, 0)
                .problems
                .contains(&NameProblem::DuplicateTarget)
        );
        assert!(
            row(&preview, 1)
                .problems
                .contains(&NameProblem::DuplicateTarget)
        );
        assert!(!preview.can_apply());
    }

    #[test]
    fn a_name_already_used_by_a_file_that_stays_is_flagged() {
        let preview = RenameRules::replace("a", "c", false).preview(&names(&["a"]), &names(&["c"]));
        assert!(
            row(&preview, 0)
                .problems
                .contains(&NameProblem::ExistingName)
        );
    }

    #[test]
    fn taking_a_name_that_the_other_file_is_keeping_is_flagged() {
        let preview =
            RenameRules::replace("old", "new", false).preview(&names(&["old", "new"]), &[]);
        assert_eq!(row(&preview, 0).to, "new");
        assert_eq!(row(&preview, 1).to, "new");
        assert!(
            row(&preview, 0)
                .problems
                .contains(&NameProblem::DuplicateTarget)
        );
        assert!(
            row(&preview, 0)
                .problems
                .contains(&NameProblem::ExistingName)
        );
        assert!(
            row(&preview, 1)
                .problems
                .contains(&NameProblem::DuplicateTarget)
        );
        assert!(!preview.can_apply());
    }

    #[test]
    fn a_file_may_take_a_name_that_another_selected_file_is_leaving() {
        let preview =
            RenameRules::pattern("@-gone", 1, 1).preview(&names(&["keep", "keep-gone"]), &[]);
        // keep -> keep-gone, keep-gone -> keep-gone-gone. The freed name is taken.
        assert!(preview.can_apply(), "{preview:?}");
        assert_eq!(row(&preview, 0).to, "keep-gone");
        assert_eq!(row(&preview, 1).to, "keep-gone-gone");
    }

    #[test]
    fn a_slash_an_empty_name_and_a_reserved_name_are_rejected() {
        let slash = RenameRules::replace("a", "b/c", false).preview(&names(&["a"]), &[]);
        assert!(matches!(
            row(&slash, 0).problems.as_slice(),
            [NameProblem::Invalid(_)]
        ));
        let empty = RenameRules::replace("a", "", false).preview(&names(&["a"]), &[]);
        assert!(matches!(
            row(&empty, 0).problems.as_slice(),
            [NameProblem::Invalid(_)]
        ));
        let dot = RenameRules::pattern(".", 1, 1).preview(&names(&["a"]), &[]);
        assert!(matches!(
            row(&dot, 0).problems.as_slice(),
            [NameProblem::Invalid(_)]
        ));
        assert!(!slash.can_apply());
    }

    #[test]
    fn an_unchanged_name_is_not_a_conflict_even_when_the_listing_includes_it() {
        let preview =
            RenameRules::replace("z", "q", false).preview(&names(&["a"]), &names(&["a", "b"]));
        assert_eq!(row(&preview, 0).to, "a");
        assert!(row(&preview, 0).problems.is_empty());
        assert!(preview.can_apply());
    }

    #[test]
    fn a_name_kept_by_a_selected_file_is_occupied() {
        // "cd" is selected but the rule leaves its name as it is.
        let preview = RenameRules::replace("ab", "cd", false)
            .preview(&names(&["ab", "cd"]), &names(&["ab", "cd", "other"]));

        assert!(
            row(&preview, 0)
                .problems
                .contains(&NameProblem::ExistingName),
            "{preview:?}"
        );
        assert!(!preview.can_apply());
    }

    #[test]
    fn selected_files_leaving_their_names_free_them() {
        // A swap: "out-2" takes "out-1" while "out-1" takes "out-2".
        let preview = RenameRules::pattern("out-#", 1, 1).preview(
            &names(&["out-2", "out-1"]),
            &names(&["out-1", "out-2", "other"]),
        );

        assert_eq!(row(&preview, 0).to, "out-1");
        assert_eq!(row(&preview, 1).to, "out-2");
        assert!(preview.can_apply(), "{preview:?}");
    }
}
