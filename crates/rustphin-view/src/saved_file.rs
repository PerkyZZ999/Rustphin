//! What Rustphin does with its own saved files, the settings and the place
//! groups (W-140), as it does with the saved tabs: a file that could not
//! be read, is damaged, or was written by a newer Rustphin is kept as it
//! is for the whole run, and the user is told, so a change made meanwhile
//! never replaces it with the defaults.

use std::io;

use serde::de::DeserializeOwned;

/// One of the files this policy keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedFile {
    /// `settings.toml`.
    Settings,
    /// `place-groups.toml`.
    Groups,
}

impl SavedFile {
    /// What the file holds, starting a sentence.
    fn what(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Groups => "Place groups",
        }
    }

    /// What is off while the file is kept.
    fn saving(self) -> &'static str {
        match self {
            Self::Settings => "Saving settings",
            Self::Groups => "Saving place groups",
        }
    }

    /// The file's name.
    fn name(self) -> &'static str {
        match self {
            Self::Settings => "settings.toml",
            Self::Groups => "place-groups.toml",
        }
    }
}

/// Why a saved file must not be replaced during this run.
#[derive(Debug)]
pub enum FileIssue {
    /// The file could not be read.
    Read(io::Error),
    /// The file is not TOML, or holds values this build cannot read.
    Damaged(String),
    /// The file was written in a later format.
    NewerVersion(u32),
}

impl std::fmt::Display for FileIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "could not be read: {error}"),
            Self::Damaged(error) => write!(f, "is damaged: {error}"),
            Self::NewerVersion(version) => write!(f, "has newer format {version}"),
        }
    }
}

impl FileIssue {
    /// What the user is told about `file`.
    pub fn notice(&self, file: SavedFile) -> String {
        let (what, saving, name) = (file.what(), file.saving(), file.name());
        match self {
            Self::Read(_) => format!(
                "{what} could not be read. {saving} is off; the file was kept. Check {name} and its permissions, then restart."
            ),
            Self::Damaged(_) => format!(
                "{what} are damaged. {saving} is off; the file was kept. Quit, repair or move {name} aside, then restart."
            ),
            Self::NewerVersion(_) => format!(
                "{what} use a newer file format. {saving} is off; the file was kept. Use a newer Rustphin or quit and move {name} aside."
            ),
        }
    }
}

/// A saved file read as far as it can be.
#[derive(Debug)]
pub struct Decoded<T> {
    /// What it holds: `None` when there is no file, or nothing in it could
    /// be used.
    pub value: Option<T>,
    /// Why the file must be kept as it is, if so.
    pub issue: Option<FileIssue>,
}

/// Reads `saved`, the file's text if it exists, written in format
/// `supported` or older. A newer file is read as far as it is understood
/// and kept; one that is not TOML or holds values this build cannot read is
/// damaged and kept.
pub fn decode<T: DeserializeOwned>(
    saved: io::Result<Option<String>>,
    supported: u32,
) -> Decoded<T> {
    let text = match saved {
        Ok(Some(text)) => text,
        Ok(None) => {
            return Decoded {
                value: None,
                issue: None,
            };
        }
        Err(error) => {
            return Decoded {
                value: None,
                issue: Some(FileIssue::Read(error)),
            };
        }
    };
    let table: toml::Table = match toml::from_str(&text) {
        Ok(table) => table,
        Err(error) => {
            return Decoded {
                value: None,
                issue: Some(FileIssue::Damaged(error.message().to_owned())),
            };
        }
    };
    let newer = table
        .get("version")
        .and_then(toml::Value::as_integer)
        .filter(|&version| version > i64::from(supported))
        .map(|version| FileIssue::NewerVersion(u32::try_from(version).unwrap_or(u32::MAX)));
    match toml::Value::Table(table).try_into::<T>() {
        Ok(value) => Decoded {
            value: Some(value),
            issue: newer,
        },
        Err(error) => Decoded {
            value: None,
            issue: Some(newer.unwrap_or_else(|| FileIssue::Damaged(error.message().to_owned()))),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default)]
    struct Sample {
        version: u32,
        name: String,
    }

    fn decoded(text: &str) -> Decoded<Sample> {
        decode(Ok(Some(text.to_owned())), 2)
    }

    #[test]
    fn a_readable_file_of_a_known_version_is_used_and_may_be_replaced() {
        let read = decoded("version = 2\nname = \"a\"\n");
        assert_eq!(
            read.value,
            Some(Sample {
                version: 2,
                name: "a".into()
            })
        );
        assert!(read.issue.is_none());
    }

    #[test]
    fn no_file_is_nothing_to_keep() {
        let read = decode::<Sample>(Ok(None), 2);
        assert!(read.value.is_none());
        assert!(read.issue.is_none());
    }

    #[test]
    fn a_file_that_is_not_toml_or_holds_wrong_values_is_damaged() {
        let not_toml = decoded("name = [");
        assert!(not_toml.value.is_none());
        assert!(matches!(not_toml.issue, Some(FileIssue::Damaged(_))));
        let wrong_value = decoded("name = 3\n");
        assert!(wrong_value.value.is_none());
        assert!(matches!(wrong_value.issue, Some(FileIssue::Damaged(_))));
    }

    #[test]
    fn a_newer_file_is_read_as_far_as_it_is_understood_and_kept() {
        let read = decoded("version = 3\nname = \"a\"\nfuture = true\n");
        assert_eq!(read.value.map(|sample| sample.name), Some("a".into()));
        assert!(matches!(read.issue, Some(FileIssue::NewerVersion(3))));
        // A newer file this build cannot read is newer, not damaged.
        let unreadable = decoded("version = 3\nname = 3\n");
        assert!(unreadable.value.is_none());
        assert!(matches!(unreadable.issue, Some(FileIssue::NewerVersion(3))));
    }

    #[test]
    fn a_file_that_could_not_be_read_is_kept() {
        let read = decode::<Sample>(Err(io::ErrorKind::PermissionDenied.into()), 2);
        assert!(read.value.is_none());
        assert!(
            matches!(read.issue, Some(FileIssue::Read(error)) if error.kind() == io::ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn the_notice_names_the_file_and_says_it_was_kept() {
        assert_eq!(
            FileIssue::Damaged(String::new()).notice(SavedFile::Groups),
            "Place groups are damaged. Saving place groups is off; the file was kept. Quit, repair or move place-groups.toml aside, then restart."
        );
        assert_eq!(
            FileIssue::NewerVersion(9).notice(SavedFile::Settings),
            "Settings use a newer file format. Saving settings is off; the file was kept. Use a newer Rustphin or quit and move settings.toml aside."
        );
    }
}
