//! File types from the shared MIME database, by file name.
//!
//! Only names are examined. Reading file contents to tell types apart is left
//! for later; a name with no known pattern is reported as generic data.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// The type reported for directories.
pub const DIRECTORY: &str = "inode/directory";
/// The type reported when a name matches no pattern.
pub const UNKNOWN: &str = "application/octet-stream";

/// The name-matching part of the shared MIME database.
#[derive(Debug, Default)]
pub struct MimeDb {
    /// Lower-cased suffix after `*.` to its type and weight.
    suffixes: HashMap<String, (u32, String)>,
    /// Lower-cased whole file names (such as `makefile`) to their type and weight.
    literals: HashMap<String, (u32, String)>,
    generic_icons: HashMap<String, String>,
    mime_dirs: Vec<PathBuf>,
    descriptions: Mutex<HashMap<String, Option<String>>>,
}

impl MimeDb {
    /// Loads the database from the `mime` folder of each data directory.
    /// Earlier directories win when two give the same pattern the same weight.
    pub fn load(data_dirs: &[PathBuf]) -> Self {
        let mut db = Self {
            mime_dirs: data_dirs.iter().map(|dir| dir.join("mime")).collect(),
            ..Self::default()
        };
        for dir in db.mime_dirs.clone() {
            if let Ok(globs) = fs::read_to_string(dir.join("globs2")) {
                db.add_globs(&globs);
            }
            if let Ok(icons) = fs::read_to_string(dir.join("generic-icons")) {
                for (mime, icon) in icons.lines().filter_map(|line| line.split_once(':')) {
                    db.generic_icons
                        .entry(mime.to_owned())
                        .or_insert_with(|| icon.to_owned());
                }
            }
        }
        db
    }

    /// Reads lines of the `globs2` format: `weight:type:pattern[:flags]`.
    /// Patterns other than `*.suffix` and plain names are skipped.
    fn add_globs(&mut self, globs: &str) {
        for line in globs.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.splitn(4, ':');
            let (Some(weight), Some(mime), Some(pattern)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let Ok(weight) = weight.parse::<u32>() else {
                continue;
            };
            let wildcard = |text: &str| text.contains(['*', '?', '[']);
            let (table, key) = match pattern.strip_prefix("*.") {
                Some(suffix) if !wildcard(suffix) => (&mut self.suffixes, suffix),
                None if !wildcard(pattern) && pattern != "__NOGLOBS__" => {
                    (&mut self.literals, pattern)
                }
                _ => continue,
            };
            let entry = table
                .entry(key.to_lowercase())
                .or_insert((0, String::new()));
            if entry.1.is_empty() || weight > entry.0 {
                *entry = (weight, mime.to_owned());
            }
        }
    }

    /// The MIME type for an entry with this file name.
    pub fn mime_type(&self, name: &[u8], is_dir: bool) -> &str {
        if is_dir {
            return DIRECTORY;
        }
        let name = String::from_utf8_lossy(name).to_lowercase();
        if let Some((_, mime)) = self.literals.get(&name) {
            return mime;
        }
        // Try the longest suffix first, so "archive.tar.gz" matches "tar.gz".
        let mut rest = name.as_str();
        while let Some((_, suffix)) = rest.split_once('.') {
            if let Some((_, mime)) = self.suffixes.get(suffix) {
                return mime;
            }
            rest = suffix;
        }
        UNKNOWN
    }

    /// Icon names to try for a type, most specific first.
    pub fn icon_names(&self, mime: &str) -> Vec<String> {
        if mime == DIRECTORY {
            return vec!["folder".to_owned(), "inode-directory".to_owned()];
        }
        let mut names = vec![mime.replace('/', "-")];
        names.extend(self.generic_icons.get(mime).cloned());
        if let Some((media, _)) = mime.split_once('/') {
            names.push(format!("{media}-x-generic"));
        }
        names.push("unknown".to_owned());
        names.dedup();
        names
    }

    /// A human-readable name for a type, such as "Plain text document".
    /// Falls back to the type itself when the database has no description.
    pub fn description(&self, mime: &str) -> String {
        let mut cache = self
            .descriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .entry(mime.to_owned())
            .or_insert_with(|| {
                self.mime_dirs
                    .iter()
                    .find_map(|dir| read_comment(dir, mime))
            })
            .clone()
            .unwrap_or_else(|| mime.to_owned())
    }
}

/// Reads the untranslated `<comment>` of a type's definition file.
fn read_comment(mime_dir: &Path, mime: &str) -> Option<String> {
    // A type is "media/subtype"; anything else must not become a path.
    let (media, subtype) = mime.split_once('/')?;
    if [media, subtype]
        .iter()
        .any(|part| part.is_empty() || part.contains(['/', '\\']) || *part == "..")
    {
        return None;
    }
    let xml = fs::read_to_string(mime_dir.join(media).join(format!("{subtype}.xml"))).ok()?;
    let start = xml.find("<comment>")? + "<comment>".len();
    let end = start + xml[start..].find("</comment>")?;
    let text = xml[start..end]
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    Some(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn database() -> (tempfile::TempDir, MimeDb) {
        let dir = tempfile::tempdir().unwrap();
        let mime = dir.path().join("mime");
        fs::create_dir_all(mime.join("text")).unwrap();
        fs::write(
            mime.join("globs2"),
            "# comment\n\
             50:text/plain:*.txt\n\
             50:application/gzip:*.gz\n\
             50:application/x-compressed-tar:*.tar.gz\n\
             50:text/x-makefile:makefile\n\
             40:text/x-old:*.md\n\
             60:text/markdown:*.md\n\
             50:application/x-sharedlib:*.so.[0-9]*\n\
             0:model/materialx:__NOGLOBS__\n\
             not a valid line\n",
        )
        .unwrap();
        fs::write(mime.join("generic-icons"), "text/markdown:text-x-generic\n").unwrap();
        fs::write(
            mime.join("text/plain.xml"),
            "<mime-type type=\"text/plain\">\n  <comment>Plain text &amp; notes</comment>\n  \
             <comment xml:lang=\"fr\">Texte brut</comment>\n</mime-type>",
        )
        .unwrap();
        let db = MimeDb::load(&[dir.path().to_path_buf()]);
        (dir, db)
    }

    #[test]
    fn matches_suffixes_without_regard_to_case() {
        let (_dir, db) = database();
        assert_eq!(db.mime_type(b"notes.txt", false), "text/plain");
        assert_eq!(db.mime_type(b"NOTES.TXT", false), "text/plain");
    }

    #[test]
    fn the_longest_matching_suffix_wins() {
        let (_dir, db) = database();
        assert_eq!(
            db.mime_type(b"backup.tar.gz", false),
            "application/x-compressed-tar"
        );
        assert_eq!(db.mime_type(b"log.gz", false), "application/gzip");
    }

    #[test]
    fn dots_earlier_in_the_name_do_not_hide_the_extension() {
        let (_dir, db) = database();
        assert_eq!(db.mime_type(b"report.final.v2.txt", false), "text/plain");
    }

    #[test]
    fn the_heavier_pattern_wins_for_the_same_suffix() {
        let (_dir, db) = database();
        assert_eq!(db.mime_type(b"README.md", false), "text/markdown");
    }

    #[test]
    fn whole_names_match_and_directories_are_directories() {
        let (_dir, db) = database();
        assert_eq!(db.mime_type(b"Makefile", false), "text/x-makefile");
        assert_eq!(db.mime_type(b"notes.txt", true), DIRECTORY);
    }

    #[test]
    fn unmatched_and_non_utf8_names_are_generic_data() {
        let (_dir, db) = database();
        assert_eq!(db.mime_type(b"mystery", false), UNKNOWN);
        assert_eq!(db.mime_type(b".hidden", false), UNKNOWN);
        assert_eq!(db.mime_type(b"bad\xff.bin", false), UNKNOWN);
        assert_eq!(db.mime_type(b"lib.so.6", false), UNKNOWN);
    }

    #[test]
    fn icon_names_go_from_specific_to_generic() {
        let (_dir, db) = database();
        assert_eq!(
            db.icon_names("text/markdown"),
            ["text-markdown", "text-x-generic", "unknown"]
        );
        assert_eq!(
            db.icon_names("application/gzip"),
            ["application-gzip", "application-x-generic", "unknown"]
        );
        assert_eq!(db.icon_names(DIRECTORY), ["folder", "inode-directory"]);
    }

    #[test]
    fn descriptions_come_from_the_database_or_fall_back_to_the_type() {
        let (_dir, db) = database();
        assert_eq!(db.description("text/plain"), "Plain text & notes");
        assert_eq!(db.description("text/markdown"), "text/markdown");
        assert_eq!(db.description("../../etc/passwd"), "../../etc/passwd");
    }
}
