//! The recent folders the palette offers (W-114e): one history for the
//! process, read and saved on its own thread (`rustphin_view::history`).

use std::{cell::RefCell, path::Path, sync::Arc};

use rustphin_platform::remote::RemoteAddress;
use rustphin_view::{
    history::{self, HistoryCommand, Recent},
    location::Location,
};

use crate::post;

struct State {
    commands: flume::Sender<HistoryCommand>,
    locations: Arc<Recent>,
    status: Option<String>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(look: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.get_or_insert_with(|| State {
            commands: history::spawn(|locations, status| {
                post::on_qt(move || {
                    STATE.with(|state| {
                        if let Some(state) = state.borrow_mut().as_mut() {
                            state.locations = locations;
                            state.status = status;
                        }
                    });
                });
            }),
            locations: Arc::default(),
            status: None,
        });
        look(state)
    })
}

/// Starts reading the history, so it is ready when the palette opens.
pub fn start() {
    with(|_| ());
}

/// Records a visit to the folder at `path` (a remote one by its address).
pub fn visit(path: &Path) {
    let location =
        match crate::remote::address_of(path).and_then(|text| RemoteAddress::parse(&text)) {
            Some(address) => Location::Remote(address),
            None => Location::Folder(path.to_owned()),
        };
    with(|state| {
        if let Err(error) = state.commands.send(HistoryCommand::Visit(location)) {
            tracing::error!(%error, "recent-folder worker stopped");
        }
    });
}

/// Forgets every recent folder.
pub fn clear() {
    with(|state| {
        if let Err(error) = state.commands.send(HistoryCommand::Clear) {
            tracing::error!(%error, "recent-folder worker stopped");
        }
    });
}

/// The recent folders, newest first, and what the palette says about them.
pub fn snapshot() -> (Arc<Recent>, Option<String>) {
    with(|state| (Arc::clone(&state.locations), state.status.clone()))
}

/// Looks for batch renames a killed Rustphin left unfinished, in the
/// saved tabs and recent folders, and restores their original names
/// (D-020). Nothing is recovered while another Rustphin runs.
pub fn recover_interrupted_renames() {
    std::thread::spawn(|| {
        let program = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "rustphin".to_owned());
        if rustphin_platform::process::others_running(&program) {
            tracing::info!("another Rustphin is running; rename recovery left to it");
            return;
        }
        let session = rustphin_platform::config::state_dir()
            .map(|dir| dir.join("session.toml"))
            .and_then(|path| rustphin_platform::config::read(&path).ok().flatten())
            .and_then(|text| rustphin_view::session::Session::parse(&text).ok())
            .unwrap_or_default();
        let folders = history::recovery_candidates(
            &session,
            history::saved_locations(history::RECENT_FOLDERS_CHECKED),
        );
        let found: Vec<std::path::PathBuf> = folders
            .into_iter()
            .filter(|folder| rustphin_core::ops::has_rename_journal(&rustphin_core::RealFs, folder))
            .collect();
        if found.is_empty() {
            return;
        }
        post::on_qt(move || {
            for directory in found {
                let name = rustphin_view::operations::folder_name(&directory);
                tracing::warn!(folder = %directory.display(), "recovering an interrupted batch rename");
                crate::hub::start_lasting(
                    rustphin_core::ops::Request::RecoverRenameBatch { directory },
                    format!("Restoring original names in {name} after an interrupted rename"),
                    format!("restored the original names in {name} after an interrupted rename"),
                );
            }
        });
    });
}
