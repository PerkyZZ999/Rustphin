//! The Properties window's decisions (W-170, D-041), free of any toolkit:
//! which tabs an entry has, Dolphin's access choices for a class of users,
//! what waits for Apply, which checksum a pasted value is and whether it
//! matches, and the rows of Details.

use rustphin_core::{execute_bit, has_bit, read_bit, with_bit, write_bit};
use rustphin_platform::{
    checksum::Algorithm,
    properties::{Facts, Kind, Picture},
};
use serde::Serialize;

use crate::format;

/// A tab of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Tab {
    /// Name, type, opening, size and dates.
    General,
    /// Access and ownership.
    Permissions,
    /// MD5, SHA1, SHA256, SHA512.
    Checksums,
    /// What Rustphin reads from the file itself.
    Details,
}

impl Tab {
    /// The tab's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Permissions => "Permissions",
            Self::Checksums => "Checksums",
            Self::Details => "Details",
        }
    }

    /// Its Lucide icon.
    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "info",
            Self::Permissions => "lock",
            Self::Checksums => "hash",
            Self::Details => "list",
        }
    }
}

/// The tabs of an entry: a folder (or a link to one) has General and
/// Permissions; a file all four.
pub fn tabs(facts: &Facts, is_folder: bool) -> Vec<Tab> {
    if is_folder || facts.kind == Kind::Directory {
        vec![Tab::General, Tab::Permissions]
    } else if facts.kind == Kind::File {
        vec![Tab::General, Tab::Permissions, Tab::Checksums, Tab::Details]
    } else {
        vec![Tab::General, Tab::Permissions, Tab::Details]
    }
}

/// What a class of users may do, as Dolphin offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Access {
    /// Read and write (and enter, for a folder).
    ViewModify,
    /// Read only (and enter, for a folder).
    ViewOnly,
    /// Nothing.
    Forbidden,
    /// A mix the choices cannot say, such as write without read.
    Custom,
}

/// The choices of the drop-down, in order; Custom is shown, never chosen.
pub const ACCESS_CHOICES: [Access; 3] = [Access::ViewModify, Access::ViewOnly, Access::Forbidden];

impl Access {
    /// The choice in words; a folder's speak of its content.
    pub fn label(self, folder: bool) -> &'static str {
        match (self, folder) {
            (Self::ViewModify, false) => "Can view & modify",
            (Self::ViewModify, true) => "Can view & modify content",
            (Self::ViewOnly, false) => "Can only view",
            (Self::ViewOnly, true) => "Can only view content",
            (Self::Forbidden, _) => "Forbidden",
            (Self::Custom, _) => "Custom",
        }
    }
}

/// What class `class` (0 owner, 1 group, 2 others) may do in `mode`. For a
/// folder, viewing includes entering it.
pub fn access(mode: u32, class: u8, folder: bool) -> Access {
    let read = has_bit(mode, read_bit(class));
    let write = has_bit(mode, write_bit(class));
    let enter = has_bit(mode, execute_bit(class));
    match (read, write) {
        (false, false) if !folder || !enter => Access::Forbidden,
        (true, true) if !folder || enter => Access::ViewModify,
        (true, false) if !folder || enter => Access::ViewOnly,
        _ => Access::Custom,
    }
}

/// `mode` with class `class` given `choice`. A file's execute bit is left
/// alone (the Execute box sets it); a folder's follows read.
pub fn with_access(mode: u32, class: u8, choice: Access, folder: bool) -> u32 {
    let (read, write) = match choice {
        Access::ViewModify => (true, true),
        Access::ViewOnly => (true, false),
        Access::Forbidden => (false, false),
        Access::Custom => return mode,
    };
    let mode = with_bit(mode, read_bit(class), read);
    let mode = with_bit(mode, write_bit(class), write);
    if folder {
        with_bit(mode, execute_bit(class), read)
    } else {
        mode
    }
}

/// Whether the file may be run as a program: by its owner, as Dolphin's box
/// shows it.
pub fn runs(mode: u32) -> bool {
    has_bit(mode, execute_bit(0))
}

/// `mode` with running allowed for every class that may read the file, or
/// for none.
pub fn with_runs(mode: u32, on: bool) -> u32 {
    (0..3u8).fold(mode, |mode, class| {
        let allowed = on && has_bit(mode, read_bit(class));
        with_bit(mode, execute_bit(class), allowed)
    })
}

/// The footer's note: how many changes wait for Apply, or nothing.
pub fn pending_note(mode_changed: bool, group_changed: bool) -> String {
    match usize::from(mode_changed) + usize::from(group_changed) {
        0 => String::new(),
        1 => "1 change waits for Apply.".to_owned(),
        n => format!("{n} changes wait for Apply."),
    }
}

/// The checksums, in the tab's order.
pub const ALGORITHMS: [Algorithm; 4] = [
    Algorithm::Md5,
    Algorithm::Sha1,
    Algorithm::Sha256,
    Algorithm::Sha512,
];

/// The checksum's name.
pub fn algorithm_name(algorithm: Algorithm) -> &'static str {
    match algorithm {
        Algorithm::Md5 => "MD5",
        Algorithm::Sha1 => "SHA1",
        Algorithm::Sha256 => "SHA256",
        Algorithm::Sha512 => "SHA512",
    }
}

/// What was pasted in the "Check a download" field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pasted {
    /// Nothing.
    Empty,
    /// Not a checksum Rustphin knows.
    Unknown,
    /// A checksum of this kind, in lowercase without spaces.
    Sum(Algorithm, String),
}

/// Which checksum `text` is, by its length in hexadecimal digits; spaces
/// around it and the case of its letters do not matter.
pub fn pasted(text: &str) -> Pasted {
    let text = text.trim();
    if text.is_empty() {
        return Pasted::Empty;
    }
    if !text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Pasted::Unknown;
    }
    let algorithm = match text.len() {
        32 => Algorithm::Md5,
        40 => Algorithm::Sha1,
        64 => Algorithm::Sha256,
        128 => Algorithm::Sha512,
        _ => return Pasted::Unknown,
    };
    Pasted::Sum(algorithm, text.to_ascii_lowercase())
}

/// How the pasted checksum compares, in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// It matches (said with a check icon), or not (an alert icon).
    pub matches: bool,
    /// What it means and what to do.
    pub words: String,
}

/// The verdict on `pasted` once its checksum `calculated` is known; `None`
/// while there is nothing to compare. Unknown text says which kinds work.
pub fn verdict(pasted: &Pasted, calculated: Option<&str>) -> Option<Verdict> {
    match pasted {
        Pasted::Empty => None,
        Pasted::Unknown => Some(Verdict {
            matches: false,
            words: "This is not a checksum Rustphin knows (MD5, SHA1, SHA256 or SHA512). Copy it again from the website."
                .to_owned(),
        }),
        Pasted::Sum(algorithm, expected) => {
            let name = algorithm_name(*algorithm);
            calculated.map(|sum| {
                if sum.eq_ignore_ascii_case(expected) {
                    Verdict {
                        matches: true,
                        words: format!("Matches the {name} checksum: the file arrived intact."),
                    }
                } else {
                    Verdict {
                        matches: false,
                        words: format!(
                            "Does not match the {name} checksum: the file differs from the one the website describes. Download it again."
                        ),
                    }
                }
            })
        }
    }
}

/// A calculation's progress in words: "Calculating… 42%".
pub fn progress_text(done: u64, total: u64) -> String {
    match done.saturating_mul(100).checked_div(total) {
        Some(percent) => format!("Calculating\u{2026} {}%", percent.min(100)),
        None => "Calculating\u{2026}".to_owned(),
    }
}

/// A row of a section: a label and its value, with an optional quieter
/// second part ("(1,572,864 bytes)").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The label.
    pub label: String,
    /// The value.
    pub value: String,
    /// Said after the value, quieter.
    pub aside: String,
    /// The value is data (paths, dates, numbers in the mono face).
    pub data: bool,
}

fn row(label: &str, value: String, aside: String, data: bool) -> Row {
    Row {
        label: label.to_owned(),
        value,
        aside,
        data,
    }
}

/// A section of rows under a heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// The heading.
    pub title: String,
    /// The rows.
    pub rows: Vec<Row>,
}

/// General's Size section: the size, and for a file what it takes on disk.
pub fn size_rows(size: &str, facts: &Facts) -> Vec<Row> {
    let (shown, exact) = split_exact(size);
    let mut rows = vec![row("Size", shown, exact, false)];
    if let Some(on_disk) = facts.on_disk {
        rows.push(row(
            "On disk",
            format::size(on_disk),
            format!("({} bytes)", format::count(on_disk as usize)),
            false,
        ));
    }
    rows
}

/// `format::size_exact`'s "1.5 MiB (1,572,864 bytes)" as its two parts.
fn split_exact(size: &str) -> (String, String) {
    match size.split_once(" (") {
        Some((shown, rest)) if rest.contains("bytes)") => {
            let (bytes, after) = rest.split_once(')').unwrap_or((rest, ""));
            (format!("{shown}{after}"), format!("({bytes})"))
        }
        _ => (size.to_owned(), String::new()),
    }
}

/// Details: the picture's header, when it is one, then the file's own facts.
pub fn details(
    picture: Option<&Picture>,
    type_name: &str,
    mime: &str,
    size: Option<u64>,
    modified: &str,
    target: Option<&str>,
) -> Vec<Section> {
    let mut sections = Vec::new();
    if let Some(picture) = picture {
        let megapixels = f64::from(picture.width) * f64::from(picture.height) / 1_000_000.;
        sections.push(Section {
            title: "Picture".to_owned(),
            rows: vec![
                row(
                    "Dimensions",
                    format!("{} \u{d7} {} pixels", picture.width, picture.height),
                    format!("({megapixels:.1} megapixels)"),
                    false,
                ),
                row(
                    "Format",
                    format!("{}, {}", picture.format, picture.colour),
                    String::new(),
                    false,
                ),
            ],
        });
    }
    let mut file = vec![row("Type", type_name.to_owned(), mime.to_owned(), false)];
    if let Some(size) = size {
        file.push(row("Size", format::size(size), String::new(), false));
    }
    file.push(row("Modified", modified.to_owned(), String::new(), true));
    if let Some(target) = target {
        file.push(row("Points to", target.to_owned(), String::new(), true));
    }
    sections.push(Section {
        title: "File".to_owned(),
        rows: file,
    });
    sections
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn facts(kind: Kind) -> Facts {
        Facts {
            kind,
            target: None,
            size: Some(1_572_864),
            modified: None,
            accessed: None,
            created: None,
            mode: 0o644,
            owner: "alex".into(),
            group: "photos".into(),
            group_id: 50,
            owned_by_user: true,
            on_disk: Some(1_576_960),
        }
    }

    #[test]
    fn a_file_has_four_tabs_and_a_folder_two() {
        assert_eq!(
            tabs(&facts(Kind::File), false),
            [Tab::General, Tab::Permissions, Tab::Checksums, Tab::Details]
        );
        assert_eq!(
            tabs(&facts(Kind::Directory), true),
            [Tab::General, Tab::Permissions]
        );
        // A link to a folder is drawn as a folder.
        assert_eq!(
            tabs(&facts(Kind::Symlink), true),
            [Tab::General, Tab::Permissions]
        );
        assert_eq!(
            tabs(&facts(Kind::Symlink), false),
            [Tab::General, Tab::Permissions, Tab::Details]
        );
    }

    #[test]
    fn each_class_reads_as_one_of_dolphins_choices_or_custom() {
        let mode = 0o640;
        assert_eq!(access(mode, 0, false), Access::ViewModify);
        assert_eq!(access(mode, 1, false), Access::ViewOnly);
        assert_eq!(access(mode, 2, false), Access::Forbidden);
        // Write without read is no choice.
        assert_eq!(access(0o200, 0, false), Access::Custom);
        // A folder one can read but not enter is no choice either.
        assert_eq!(access(0o750, 0, true), Access::ViewModify);
        assert_eq!(access(0o750, 1, true), Access::ViewOnly);
        assert_eq!(access(0o640, 1, true), Access::Custom);
        assert_eq!(access(0o710, 1, true), Access::Custom);
        assert_eq!(Access::ViewOnly.label(true), "Can only view content");
    }

    #[test]
    fn a_choice_sets_read_and_write_and_for_a_folder_entering() {
        // Others forbidden: the 4 goes; the owner's execute bit stays.
        assert_eq!(with_access(0o744, 2, Access::Forbidden, false), 0o740);
        assert_eq!(with_access(0o600, 1, Access::ViewOnly, false), 0o640);
        assert_eq!(with_access(0o700, 2, Access::ViewModify, true), 0o707);
        assert_eq!(with_access(0o755, 2, Access::Forbidden, true), 0o750);
        // Special bits and other classes are untouched; Custom changes nothing.
        assert_eq!(with_access(0o4755, 1, Access::ViewOnly, false), 0o4755);
        assert_eq!(with_access(0o200, 0, Access::Custom, false), 0o200);
        for choice in ACCESS_CHOICES {
            for folder in [false, true] {
                let mode = with_access(0o000, 1, choice, folder);
                assert_eq!(access(mode, 1, folder), choice, "{choice:?} {folder}");
            }
        }
    }

    #[test]
    fn running_is_allowed_for_those_who_may_read() {
        assert!(!runs(0o644));
        assert_eq!(with_runs(0o640, true), 0o750);
        assert!(runs(0o750));
        assert_eq!(with_runs(0o755, false), 0o644);
    }

    #[test]
    fn the_footer_counts_what_waits_for_apply() {
        assert_eq!(pending_note(false, false), "");
        assert_eq!(pending_note(true, false), "1 change waits for Apply.");
        assert_eq!(pending_note(false, true), "1 change waits for Apply.");
        assert_eq!(pending_note(true, true), "2 changes wait for Apply.");
    }

    #[test]
    fn a_pasted_checksum_is_known_by_its_length() {
        assert_eq!(pasted("  "), Pasted::Empty);
        assert_eq!(
            pasted(" 900150983CD24FB0D6963F7D28E17F72\n"),
            Pasted::Sum(Algorithm::Md5, "900150983cd24fb0d6963f7d28e17f72".into())
        );
        assert!(matches!(
            pasted(&"a".repeat(40)),
            Pasted::Sum(Algorithm::Sha1, _)
        ));
        assert!(matches!(
            pasted(&"b".repeat(64)),
            Pasted::Sum(Algorithm::Sha256, _)
        ));
        assert!(matches!(
            pasted(&"c".repeat(128)),
            Pasted::Sum(Algorithm::Sha512, _)
        ));
        assert_eq!(pasted(&"a".repeat(33)), Pasted::Unknown);
        assert_eq!(pasted(&"g".repeat(32)), Pasted::Unknown);
    }

    #[test]
    fn the_verdict_says_whether_the_file_arrived_intact() {
        let md5 = pasted("900150983CD24FB0D6963F7D28E17F72");
        assert_eq!(verdict(&md5, None), None, "nothing to compare yet");
        let same = verdict(&md5, Some("900150983cd24fb0d6963f7d28e17f72")).unwrap();
        assert!(same.matches);
        assert_eq!(
            same.words,
            "Matches the MD5 checksum: the file arrived intact."
        );
        let other = verdict(&md5, Some("00000000000000000000000000000000")).unwrap();
        assert!(!other.matches);
        assert!(other.words.starts_with("Does not match the MD5 checksum"));
        assert!(!verdict(&Pasted::Unknown, None).unwrap().matches);
        assert_eq!(verdict(&Pasted::Empty, Some("x")), None);
    }

    #[test]
    fn progress_is_said_in_percent() {
        assert_eq!(progress_text(42, 100), "Calculating\u{2026} 42%");
        assert_eq!(progress_text(5, 0), "Calculating\u{2026}");
    }

    #[test]
    fn the_size_section_splits_the_exact_bytes_and_adds_the_disk() {
        let rows = size_rows("1.5 MiB (1,572,864 bytes)", &facts(Kind::File));
        assert_eq!(rows[0].value, "1.5 MiB");
        assert_eq!(rows[0].aside, "(1,572,864 bytes)");
        assert_eq!(rows[1].label, "On disk");
        assert_eq!(rows[1].aside, "(1,576,960 bytes)");
        let mut folder = facts(Kind::Directory);
        folder.on_disk = None;
        let rows = size_rows("Calculating size\u{2026}", &folder);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, "Calculating size\u{2026}");
    }

    #[test]
    fn details_show_a_pictures_header_then_the_file() {
        let picture = Picture {
            width: 4032,
            height: 3024,
            format: "JPEG",
            colour: "colour, 8 bits per channel".into(),
        };
        let sections = details(
            Some(&picture),
            "JPEG image",
            "image/jpeg",
            Some(1_572_864),
            "2026-10-27 16:41:08",
            None,
        );
        assert_eq!(sections[0].title, "Picture");
        assert_eq!(sections[0].rows[0].value, "4032 \u{d7} 3024 pixels");
        assert_eq!(sections[0].rows[0].aside, "(12.2 megapixels)");
        assert_eq!(
            sections[0].rows[1].value,
            "JPEG, colour, 8 bits per channel"
        );
        let file = &sections[1];
        let labels: Vec<&str> = file.rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, ["Type", "Size", "Modified"]);
        let plain = details(None, "Plain text", "text/plain", None, "x", Some("/etc/a"));
        assert_eq!(plain.len(), 1);
        assert_eq!(plain[0].rows.last().unwrap().label, "Points to");
    }
}
