//! A folder as saved tabs and recent folders remember it: a local folder, or
//! a remote one by address, since kio-fuse mount paths do not outlive a run.

use std::path::PathBuf;

use rustphin_core::RecentKey;
use rustphin_platform::{
    remote::RemoteAddress,
    uri::{file_uri, path_from_uri},
};

/// Where a folder is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    /// A folder on this machine.
    Folder(PathBuf),
    /// A remote folder, opened again through kio-fuse. Kept without password.
    Remote(RemoteAddress),
}

impl From<PathBuf> for Location {
    fn from(path: PathBuf) -> Self {
        Self::Folder(path)
    }
}

impl Location {
    /// The text saved for it: a byte-preserving `file://` URI, or the remote
    /// address without any password.
    pub fn to_saved(&self) -> String {
        match self {
            Self::Folder(path) => file_uri(path),
            Self::Remote(address) => address.to_string(),
        }
    }

    /// Reads saved text: an absolute path (the first session format), a
    /// local `file:` URI or a remote address. Anything else is `None`.
    pub fn from_saved(value: &str) -> Option<Self> {
        if value.starts_with('/') {
            Some(Self::Folder(PathBuf::from(value)))
        } else if let Some(path) = path_from_uri(value.as_bytes()) {
            Some(Self::Folder(path))
        } else {
            RemoteAddress::parse(value).map(Self::Remote)
        }
    }
}

impl RecentKey for Location {
    fn is_usable(&self) -> bool {
        match self {
            Self::Folder(path) => path.is_usable(),
            Self::Remote(_) => true,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn saved_text_round_trips_and_never_holds_a_password() {
        let remote = Location::Remote(RemoteAddress::parse("sftp://me:pw@host/a b").unwrap());
        assert_eq!(remote.to_saved(), "sftp://me@host/a b");
        assert_eq!(
            Location::from_saved(&remote.to_saved()),
            Some(Location::Remote(
                RemoteAddress::parse("sftp://me@host/a b").unwrap()
            ))
        );
        let folder = Location::Folder("/data/50% off".into());
        assert_eq!(Location::from_saved(&folder.to_saved()), Some(folder));
        assert_eq!(
            Location::from_saved("/legacy/path"),
            Some(Location::Folder("/legacy/path".into()))
        );
        assert_eq!(Location::from_saved("file://otherhost/a"), None);
        assert_eq!(Location::from_saved("relative"), None);
    }
}
