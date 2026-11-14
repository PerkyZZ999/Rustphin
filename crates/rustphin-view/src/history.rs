//! The palette's recent folders, saved in the state directory, read and
//! written on a thread of their own (W-080c, W-114e), free of any toolkit.

use std::sync::Arc;

use rustphin_core::RecentLocations;
use rustphin_platform::config;
use serde::{Deserialize, Serialize};

use crate::location::Location;

/// Recently visited locations, newest first.
pub type Recent = RecentLocations<Location>;

const VERSION: u32 = 1;

#[derive(Default, Serialize, Deserialize)]
struct SavedHistory {
    version: u32,
    locations: Vec<String>,
}

/// Why saved history was not used.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// It could not be read as history.
    Damaged,
    /// A newer Rustphin wrote it.
    NewerVersion,
}

// Version 1 readers before W-080c skip remote addresses as unknown entries.
/// Reads saved history; only `file://` and remote addresses are kept.
pub fn decode(text: &str) -> Result<Recent, ReadError> {
    let saved: SavedHistory = toml::from_str(text).map_err(|_| ReadError::Damaged)?;
    if saved.version > VERSION {
        return Err(ReadError::NewerVersion);
    }
    Ok(RecentLocations::from_paths(
        saved
            .locations
            .iter()
            // Only URIs: this file never held bare paths.
            .filter(|value| !value.starts_with('/'))
            .filter_map(|value| Location::from_saved(value)),
    ))
}

/// The history as saved.
pub fn encode(history: &Recent) -> Result<String, toml::ser::Error> {
    toml::to_string(&SavedHistory {
        version: VERSION,
        locations: history.paths().iter().map(Location::to_saved).collect(),
    })
}

/// The saved recent folders, newest first, at most `limit`, read directly
/// from disk (call off the UI thread). Damaged or missing history gives none.
pub fn saved_locations(limit: usize) -> Vec<Location> {
    config::state_dir()
        .map(|dir| dir.join("recent-locations.toml"))
        .and_then(|path| config::read(&path).ok().flatten())
        .and_then(|text| decode(&text).ok())
        .map(|recent| recent.paths().iter().take(limit).cloned().collect())
        .unwrap_or_default()
}

/// What the history worker is asked to do.
#[derive(Debug)]
pub enum HistoryCommand {
    /// A folder was visited.
    Visit(Location),
    /// Forget every folder.
    Clear,
}

/// Starts the worker that reads, changes and saves the history; it calls
/// `on_update` with every new state and a status for the palette, and
/// stops when the returned sender is dropped.
pub fn spawn(
    on_update: impl Fn(Arc<Recent>, Option<String>) + Send + 'static,
) -> flume::Sender<HistoryCommand> {
    let path = config::state_dir().map(|dir| dir.join("recent-locations.toml"));
    let (commands, receiver) = flume::unbounded();
    std::thread::spawn(move || {
        let mut history = Recent::default();
        let mut status = None;
        let mut writable = path.is_some();
        if let Some(path) = &path {
            match config::read(path) {
                Ok(Some(text)) => match decode(&text) {
                    Ok(saved) => history = saved,
                    Err(error) => {
                        writable = false;
                        status = Some(match error {
                            ReadError::Damaged => "Saved recent folders are damaged. Move the state file aside and restart to save history again.",
                            ReadError::NewerVersion => "Recent folders were saved by a newer Rustphin. Use that version to change the saved history.",
                        }.to_owned());
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, "could not read recent folders");
                    writable = false;
                    status = Some("Recent folders could not be read. Check state-directory permissions and restart.".to_owned());
                }
            }
        } else {
            status = Some(
                "No state directory is available. Recent folders will last only for this run."
                    .to_owned(),
            );
        }
        on_update(Arc::new(history.clone()), status.clone());
        while let Ok(command) = receiver.recv() {
            let cleared = matches!(command, HistoryCommand::Clear);
            let changed = match command {
                HistoryCommand::Visit(path) => history.visit(path),
                HistoryCommand::Clear => {
                    history.clear();
                    true
                }
            };
            if !changed {
                continue;
            }
            if writable && let Some(path) = &path {
                let saved = encode(&history)
                    .map_err(|error| error.to_string())
                    .and_then(|text| {
                        config::write_atomic(path, text.as_bytes())
                            .map_err(|error| error.to_string())
                    });
                status = match saved {
                    Ok(()) if cleared => Some("Recent folders cleared.".to_owned()),
                    Ok(()) => None,
                    Err(error) => {
                        tracing::warn!(%error, "could not save recent folders");
                        Some("Recent folders could not be saved. Check state-directory permissions and try again.".to_owned())
                    }
                };
            }
            on_update(Arc::new(history.clone()), status.clone());
        }
    });
    commands
}

#[cfg(test)]
// A failed test setup should fail the test immediately.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rustphin_platform::remote::RemoteAddress;
    use std::{ffi::OsString, os::unix::ffi::OsStringExt as _, path::PathBuf};

    fn folder(path: &str) -> Location {
        Location::Folder(PathBuf::from(path))
    }

    #[test]
    fn saved_history_round_trips_raw_names_remote_folders_and_visit_order() {
        let bad = PathBuf::from(OsString::from_vec(b"/data/\xff\n%".to_vec()));
        let remote = Location::Remote(RemoteAddress::parse("sftp://me@host/srv/sub dir").unwrap());
        let history = Recent::from_paths([bad.into(), remote, folder("/data/Été")]);
        assert_eq!(decode(&encode(&history).unwrap()), Ok(history));
    }

    #[test]
    fn saved_history_rejects_other_hosts_relative_nul_bare_and_duplicate_locations() {
        let text = "version = 1\nlocations = ['sftp://host/x', 'file://remote/a', 'relative', '/bare', 'file:///a', 'file:///a', 'file:///b%00c']";
        assert_eq!(
            decode(text).unwrap().paths(),
            [
                Location::Remote(RemoteAddress::parse("sftp://host/x").unwrap()),
                folder("/a")
            ]
        );
    }

    #[test]
    fn a_future_format_is_refused_so_it_cannot_be_overwritten() {
        assert_eq!(
            decode("version = 2\nlocations = []"),
            Err(ReadError::NewerVersion)
        );
    }

    #[test]
    fn damaged_history_is_reported_instead_of_silently_discarded() {
        assert_eq!(decode("locations = 7"), Err(ReadError::Damaged));
    }

    #[test]
    fn clearing_removes_every_path_from_the_saved_file() {
        let mut history = Recent::from_paths([folder("/private/folder")]);
        history.clear();
        let saved = encode(&history).unwrap();
        assert!(!saved.contains("private"));
        assert!(decode(&saved).unwrap().paths().is_empty());
    }
}
