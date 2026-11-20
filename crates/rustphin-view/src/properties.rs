//! What the Properties dialog says (W-114f), free of any toolkit: the
//! facts of one entry, its size while a folder is measured, and its
//! permissions by class.

use std::path::Path;

use jiff::tz::TimeZone;
use rustphin_core::{
    display_name, execute_bit, has_bit, has_special, octal_mode, read_bit, symbolic_mode, write_bit,
};
use rustphin_platform::properties::{Facts, Kind};
use serde::Serialize;

use crate::format;

/// The permission classes, in order.
pub const PERMISSION_CLASSES: [&str; 3] = ["Owner", "Group", "Others"];

/// Where the size line stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeLine {
    /// The entry is being read.
    Reading,
    /// A file's size.
    Known(u64),
    /// A folder is being measured.
    Calculating,
    /// A folder's measured size.
    Folder {
        /// Bytes of its files.
        bytes: u64,
        /// Entries that could not be read.
        unreadable: u64,
        /// Entries counted.
        items: u64,
    },
    /// Measuring failed: why.
    Failed(String),
}

/// The size in words; `seen` counts what a measurement has found so far.
pub fn size_text(size: &SizeLine, seen: u64) -> String {
    match size {
        SizeLine::Reading => "Reading\u{2026}".to_owned(),
        SizeLine::Known(bytes) => format::size_exact(*bytes),
        SizeLine::Calculating => match seen {
            0 => "Calculating size\u{2026}".to_owned(),
            seen => format!(
                "Calculating\u{2026} {} {} so far",
                format::count(seen as usize),
                format::plural(seen as usize, "item", "items")
            ),
        },
        SizeLine::Folder {
            bytes,
            unreadable,
            items,
        } => {
            let size = format!(
                "{}, {} {}",
                format::size_exact(*bytes),
                format::count(*items as usize),
                format::plural(*items as usize, "item", "items")
            );
            if *unreadable == 0 {
                size
            } else {
                format!(
                    "{size}. {} {} could not be read.",
                    format::count(*unreadable as usize),
                    format::plural(*unreadable as usize, "item", "items")
                )
            }
        }
        SizeLine::Failed(message) => message.clone(),
    }
}

/// A time in words, or "Unknown".
pub fn time_text(seconds: Option<i64>, zone: &TimeZone) -> String {
    seconds
        .map(|seconds| format::timestamp_precise(seconds, zone))
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown".to_owned())
}

/// Whose type icon an entry's Properties shows: the entry itself, or for
/// a symbolic link what it points to, as the list draws links. Returns the
/// name to tell the type by and whether to draw a folder; `target_is_folder`
/// is what following the link found.
pub fn icon_subject<'a>(
    path: &'a Path,
    facts: &'a Facts,
    target_is_folder: bool,
) -> (&'a [u8], bool) {
    let name = |path: &'a Path| {
        path.file_name()
            .map(|name| name.as_encoded_bytes())
            .unwrap_or_default()
    };
    match (&facts.kind, &facts.target) {
        (Kind::Symlink, Some(target)) => (name(target), target_is_folder),
        (kind, _) => (name(path), *kind == Kind::Directory),
    }
}

/// The size line a freshly read entry starts with.
pub fn first_size(facts: &Facts) -> SizeLine {
    match (facts.kind, facts.size) {
        (Kind::Directory, _) => SizeLine::Calculating,
        (_, Some(bytes)) => SizeLine::Known(bytes),
        (_, None) => SizeLine::Reading,
    }
}

/// The type shown: a link says so, else the type's description.
pub fn type_label(facts: &Facts, description: &str) -> String {
    if facts.kind == Kind::Symlink {
        "Symbolic link".to_owned()
    } else {
        description.to_owned()
    }
}

/// The facts as label and value pairs, in the dialog's order.
pub fn facts_rows(
    path: &Path,
    facts: &Facts,
    size: &str,
    zone: &TimeZone,
) -> Vec<(String, String)> {
    let location = path
        .parent()
        .map(|folder| display_name(folder.as_os_str().as_encoded_bytes()).into_owned())
        .unwrap_or_default();
    let mut rows = vec![
        ("Location".to_owned(), location),
        ("Size".to_owned(), size.to_owned()),
        ("Modified".to_owned(), time_text(facts.modified, zone)),
        ("Accessed".to_owned(), time_text(facts.accessed, zone)),
    ];
    if let Some(created) = facts.created {
        rows.push(("Created".to_owned(), time_text(Some(created), zone)));
    }
    if let Some(target) = &facts.target {
        rows.push((
            "Points to".to_owned(),
            display_name(target.as_os_str().as_encoded_bytes()).into_owned(),
        ));
    }
    rows
}

/// One class of permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Class {
    /// "Owner", "Group", "Others".
    pub name: &'static str,
    /// Its heading: "OWNER · user".
    pub heading: String,
    /// Read, write and execute.
    pub bits: [bool; 3],
}

/// The permissions as the dialog shows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Permissions {
    /// "rwxr-xr-x (0755)".
    pub words: String,
    /// The three classes.
    pub classes: Vec<Class>,
    /// A note under them, if any.
    pub note: String,
    /// The boxes can be changed.
    pub editable: bool,
}

/// The permissions of `mode` for the entry `facts` describe.
pub fn permissions(facts: &Facts, mode: u32) -> Permissions {
    let headings = [
        format!("OWNER \u{b7} {}", facts.owner),
        format!("GROUP \u{b7} {}", facts.group),
        "OTHERS".to_owned(),
    ];
    let editable = facts.kind != Kind::Symlink;
    Permissions {
        words: format!("{} ({})", symbolic_mode(mode), octal_mode(mode)),
        classes: (0..3u8)
            .zip(headings)
            .map(|(class, heading)| Class {
                name: PERMISSION_CLASSES[usize::from(class)],
                heading,
                bits: [
                    has_bit(mode, read_bit(class)),
                    has_bit(mode, write_bit(class)),
                    has_bit(mode, execute_bit(class)),
                ],
            })
            .collect(),
        note: if !editable {
            "Permissions of a shortcut are not changed here.".to_owned()
        } else if has_special(mode) {
            "Setuid, setgid or the sticky bit is set, and is kept.".to_owned()
        } else {
            String::new()
        },
        editable,
    }
}

/// The bit of `class` (0–2) and `which` (0 read, 1 write, 2 execute).
pub fn bit(class: u8, which: u8) -> u32 {
    match which {
        0 => read_bit(class),
        1 => write_bit(class),
        _ => execute_bit(class),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(kind: Kind) -> Facts {
        Facts {
            kind,
            target: None,
            size: Some(2048),
            modified: None,
            accessed: None,
            created: None,
            mode: 0o4755,
            owner: "ana".into(),
            group: "staff".into(),
            group_id: 50,
            owned_by_user: true,
            on_disk: Some(4096),
        }
    }

    #[test]
    fn a_link_shows_the_icon_of_what_it_points_to() {
        let mut link = facts(Kind::Symlink);
        link.target = Some("/data/photos/cat.jpg".into());
        assert_eq!(
            icon_subject(Path::new("/a/shortcut"), &link, false),
            (b"cat.jpg".as_slice(), false)
        );
        link.target = Some("/data/photos".into());
        assert_eq!(
            icon_subject(Path::new("/a/shortcut"), &link, true),
            (b"photos".as_slice(), true)
        );
        // Anything else is itself.
        assert_eq!(
            icon_subject(Path::new("/a/notes.txt"), &facts(Kind::File), false),
            (b"notes.txt".as_slice(), false)
        );
        assert_eq!(
            icon_subject(Path::new("/a/docs"), &facts(Kind::Directory), false),
            (b"docs".as_slice(), true)
        );
    }

    #[test]
    fn a_folder_says_how_far_its_measuring_is_and_what_was_unreadable() {
        assert_eq!(
            size_text(&SizeLine::Calculating, 0),
            "Calculating size\u{2026}"
        );
        assert_eq!(
            size_text(&SizeLine::Calculating, 3),
            "Calculating\u{2026} 3 items so far"
        );
        let measured = SizeLine::Folder {
            bytes: 10,
            unreadable: 1,
            items: 2,
        };
        assert!(size_text(&measured, 0).ends_with("2 items. 1 item could not be read."));
        assert_eq!(first_size(&facts(Kind::Directory)), SizeLine::Calculating);
        assert_eq!(first_size(&facts(Kind::File)), SizeLine::Known(2048));
    }

    #[test]
    fn permissions_name_their_owner_and_keep_special_bits_in_view() {
        let shown = permissions(&facts(Kind::File), 0o4755);
        assert_eq!(shown.classes[0].heading, "OWNER \u{b7} ana");
        assert_eq!(shown.classes[2].bits, [true, false, true]);
        assert!(shown.note.starts_with("Setuid"));
        let link = permissions(&facts(Kind::Symlink), 0o777);
        assert!(!link.editable);
        assert_eq!(bit(1, 1), 0o020);
        assert_eq!(type_label(&facts(Kind::Symlink), "Folder"), "Symbolic link");
    }

    #[test]
    fn facts_list_location_times_and_a_links_target() {
        let mut link = facts(Kind::Symlink);
        link.target = Some("/srv/data".into());
        let rows = facts_rows(Path::new("/home/ana/x"), &link, "2 KiB", &TimeZone::UTC);
        assert_eq!(rows[0], ("Location".to_owned(), "/home/ana".to_owned()));
        assert_eq!(rows[2].1, "Unknown");
        assert_eq!(rows.last().map(|row| row.0.as_str()), Some("Points to"));
    }
}
