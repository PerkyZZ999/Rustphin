//! Palette history, with loading and serialised writes off the UI thread.

use std::sync::Arc;

use gpui_kit::{App, AppContext as _, Context, Entity, Global};
use rustphin_core::RecentLocations;
use rustphin_platform::config;
use serde::{Deserialize, Serialize};

use crate::location::Location;

/// Recent folders: local ones by path, remote ones by address.
pub type Recent = RecentLocations<Location>;

const VERSION: u32 = 1;

#[derive(Default, Serialize, Deserialize)]
struct SavedHistory {
    version: u32,
    locations: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum ReadError {
    Damaged,
    NewerVersion,
}

// Version 1 readers before W-080c skip remote addresses as unknown entries.
fn decode(text: &str) -> Result<Recent, ReadError> {
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

fn encode(history: &Recent) -> Result<String, toml::ser::Error> {
    toml::to_string(&SavedHistory {
        version: VERSION,
        locations: history.paths().iter().map(Location::to_saved).collect(),
    })
}

enum Command {
    Visit(Location),
    Clear,
}

struct Update {
    locations: Arc<Recent>,
    status: Option<String>,
}

/// Latest snapshot and the channel to the single history writer.
pub struct History {
    pub locations: Arc<Recent>,
    pub status: Option<String>,
    commands: flume::Sender<Command>,
}

struct GlobalHistory(Entity<History>);
impl Global for GlobalHistory {}

/// Starts history loading and a serial worker. Visits made while it loads
/// queue behind the load; clear and later visits cannot be overwritten by
/// an older background save.
pub fn init(cx: &mut App) {
    let path = config::state_dir().map(|dir| dir.join("recent-locations.toml"));
    let (commands, receiver) = flume::unbounded();
    let (updates, received) = flume::unbounded();
    cx.background_executor().spawn(async move {
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
                Ok(None) => {},
                Err(error) => {
                    tracing::warn!(%error, "could not read recent folders");
                    writable = false;
                    status = Some("Recent folders could not be read. Check state-directory permissions and restart.".to_owned());
                }
            }
        } else {
            status = Some("No state directory is available. Recent folders will last only for this run.".to_owned());
        }
        let _ = updates.send(Update { locations: Arc::new(history.clone()), status: status.clone() });
        while let Ok(command) = receiver.recv_async().await {
            let cleared = matches!(command, Command::Clear);
            let changed = match command {
                Command::Visit(path) => history.visit(path),
                Command::Clear => { history.clear(); true }
            };
            if !changed { continue; }
            if writable && let Some(path) = &path {
                let saved = encode(&history).map_err(|error| error.to_string())
                    .and_then(|text| config::write_atomic(path, text.as_bytes()).map_err(|error| error.to_string()));
                status = match saved {
                    Ok(()) if cleared => Some("Recent folders cleared.".to_owned()),
                    Ok(()) => None,
                    Err(error) => {
                        tracing::warn!(%error, "could not save recent folders");
                        Some("Recent folders could not be saved. Check state-directory permissions and try again.".to_owned())
                    }
                };
            }
            if updates.send(Update { locations: Arc::new(history.clone()), status: status.clone() }).is_err() {
                break;
            }
        }
    }).detach();
    let history = cx.new(|cx: &mut Context<History>| {
        cx.spawn(async move |history, cx| {
            while let Ok(update) = received.recv_async().await {
                if history
                    .update(cx, |history, cx| {
                        history.locations = update.locations;
                        history.status = update.status;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        History {
            locations: Arc::default(),
            status: None,
            commands,
        }
    });
    cx.set_global(GlobalHistory(history));
}

/// The shared recent-folder history.
pub fn history(cx: &App) -> Entity<History> {
    cx.global::<GlobalHistory>().0.clone()
}

impl History {
    pub fn visit(&self, location: Location) {
        if let Err(error) = self.commands.send(Command::Visit(location)) {
            tracing::error!(%error, "recent-folder worker stopped");
        }
    }

    pub fn clear(&self) {
        if let Err(error) = self.commands.send(Command::Clear) {
            tracing::error!(%error, "recent-folder worker stopped");
        }
    }
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
