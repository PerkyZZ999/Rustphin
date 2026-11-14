//! Which selections the Extract and Compress menus apply to, and what
//! they call things (W-111, D-032). Pure: names and types in, words out.

use rustphin_core::archive::{self, Format};

/// Archive types Rustphin opens itself.
const OWN_TYPES: [&str; 10] = [
    "application/zip",
    "application/x-zip-compressed",
    "application/java-archive",
    "application/x-tar",
    "application/x-compressed-tar",
    "application/x-bzip-compressed-tar",
    "application/x-bzip2-compressed-tar",
    "application/x-xz-compressed-tar",
    "application/x-zstd-compressed-tar",
    "application/x-7z-compressed",
];

/// Archive types handed to Ark, 7-Zip or unrar (W-111f).
const OTHER_TYPES: [&str; 18] = [
    "application/vnd.rar",
    "application/x-rar",
    "application/x-rar-compressed",
    "application/x-cd-image",
    "application/x-iso9660-image",
    "application/vnd.ms-cab-compressed",
    "application/x-lha",
    "application/x-lzh-compressed",
    "application/x-arj",
    "application/x-cpio",
    "application/x-rpm",
    "application/vnd.debian.binary-package",
    "application/x-archive",
    "application/gzip",
    "application/x-bzip2",
    "application/x-xz",
    "application/zstd",
    "application/x-lzma",
];

/// Who opens an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opener {
    /// Rustphin's own engine.
    Rustphin,
    /// Ark or another installed tool.
    Other,
}

/// Who opens the file called `name` of type `mime`, if it is an archive.
pub fn opener(name: &[u8], mime: Option<&str>) -> Option<Opener> {
    if Format::from_name(name).is_some() || mime.is_some_and(|mime| OWN_TYPES.contains(&mime)) {
        Some(Opener::Rustphin)
    } else if mime.is_some_and(|mime| OTHER_TYPES.contains(&mime)) {
        Some(Opener::Other)
    } else {
        None
    }
}

/// The folder "Extract into" makes for the archive called `name`: the name
/// without its archive ending, or else without its last ending, as Ark
/// names it ("notes.txt.gz" gives "notes.txt").
pub fn folder_stem(name: &[u8]) -> &[u8] {
    let stem = archive::stem(name);
    if stem.len() < name.len() {
        return stem;
    }
    match name.iter().rposition(|&byte| byte == b'.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

/// [`folder_stem`] as text for the menu.
pub fn folder_for(name: &[u8]) -> String {
    String::from_utf8_lossy(folder_stem(name)).into_owned()
}

/// The name, without ending, a new archive of these entries gets, as
/// Dolphin's Compress menu gives it: one folder's name, one file's name
/// without its last ending, or the common start of several names when it
/// is at least five characters long, else "Archive".
pub fn compress_base(entries: &[(&[u8], bool)]) -> String {
    match entries {
        [] => "Archive".to_owned(),
        [(name, true)] => String::from_utf8_lossy(name).into_owned(),
        [(name, false)] => {
            let name = String::from_utf8_lossy(name);
            match name.rfind('.') {
                Some(dot) if dot > 0 => name[..dot].to_owned(),
                _ => name.into_owned(),
            }
        }
        [(first, _), rest @ ..] => {
            let mut common = first.len();
            for (name, _) in rest {
                common = common.min(
                    first
                        .iter()
                        .zip(name.iter())
                        .take_while(|(a, b)| a == b)
                        .count(),
                );
            }
            let prefix = String::from_utf8_lossy(&first[..common]);
            let prefix = prefix.trim_end_matches([' ', '-', '_', '.', '(']);
            if prefix.chars().count() >= 5 {
                prefix.to_owned()
            } else {
                "Archive".to_owned()
            }
        }
    }
}

/// `base.extension`, or the first free `base (n).extension` among names
/// `is_taken` reports.
pub fn free_archive_name(base: &str, format: Format, is_taken: impl Fn(&[u8]) -> bool) -> String {
    let extension = format.extension();
    let first = format!("{base}.{extension}");
    if !is_taken(first.as_bytes()) {
        return first;
    }
    (1..)
        .map(|n| format!("{base} ({n}).{extension}"))
        .find(|candidate| !is_taken(candidate.as_bytes()))
        .unwrap_or(first)
}

/// The files the Compress menu's two quick entries make, ZIP then TAR.GZ:
/// the menu shows these names, numbered when taken, and makes exactly them.
pub fn quick_names(entries: &[(&[u8], bool)], is_taken: impl Fn(&[u8]) -> bool) -> [String; 2] {
    let base = compress_base(entries);
    [Format::Zip, Format::TarGz].map(|format| free_archive_name(&base, format, &is_taken))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archives_are_known_by_name_or_type_and_others_go_to_ark() {
        assert_eq!(opener(b"a.zip", None), Some(Opener::Rustphin));
        assert_eq!(opener(b"photos.tar.zst", None), Some(Opener::Rustphin));
        assert_eq!(
            opener(b"report", Some("application/x-7z-compressed")),
            Some(Opener::Rustphin)
        );
        assert_eq!(
            opener(b"film.rar", Some("application/vnd.rar")),
            Some(Opener::Other)
        );
        assert_eq!(
            opener(b"disc.iso", Some("application/x-cd-image")),
            Some(Opener::Other)
        );
        assert_eq!(opener(b"notes.txt", Some("text/plain")), None);
    }

    #[test]
    fn the_new_archive_is_named_as_dolphin_names_it() {
        assert_eq!(
            compress_base(&[(b"Holiday photos", true)]),
            "Holiday photos"
        );
        assert_eq!(
            compress_base(&[(b"report.final.pdf", false)]),
            "report.final"
        );
        assert_eq!(compress_base(&[(b".bashrc", false)]), ".bashrc");
        assert_eq!(
            compress_base(&[
                (b"invoice-2026-01.pdf", false),
                (b"invoice-2026-02.pdf", false)
            ]),
            "invoice-2026-0"
        );
        assert_eq!(
            compress_base(&[(b"cat.png", false), (b"car.png", false)]),
            "Archive"
        );
        // The shared start is trimmed of separators before it is counted.
        assert_eq!(
            compress_base(&[(b"same - one", false), (b"same - two", false)]),
            "Archive"
        );
        assert_eq!(
            compress_base(&[
                (b"project-notes-a.md", false),
                (b"project-notes-b.md", false)
            ]),
            "project-notes"
        );
    }

    #[test]
    fn a_taken_name_gets_a_number() {
        let taken = |name: &[u8]| name == b"pack.zip" || name == b"pack (1).zip";
        assert_eq!(
            free_archive_name("pack", Format::Zip, taken),
            "pack (2).zip"
        );
        assert_eq!(
            free_archive_name("pack", Format::TarGz, taken),
            "pack.tar.gz"
        );
    }

    #[test]
    fn the_quick_entries_name_the_file_they_will_make() {
        let taken = |name: &[u8]| name == b"photos.zip";
        assert_eq!(
            quick_names(&[(b"photos", true)], taken),
            ["photos (1).zip", "photos.tar.gz"]
        );
    }

    #[test]
    fn the_extract_folder_drops_the_archive_ending() {
        assert_eq!(folder_for(b"Holiday.tar.gz"), "Holiday");
        assert_eq!(folder_for(b"v2.0.ZIP"), "v2.0");
        // Archives other tools open lose their last ending too.
        assert_eq!(folder_for(b"notes.txt.gz"), "notes.txt");
        assert_eq!(folder_for(b"film.rar"), "film");
        assert_eq!(folder_for(b"README"), "README");
        assert_eq!(folder_for(b".hidden"), ".hidden");
    }
}
