//! Startup recovery of interrupted batch renames (D-020).
//!
//! A batch rename stages names under a journal in its folder. If Rustphin is
//! killed part-way, the next start restores the original names through the
//! engine's journal-checked recovery. The folders checked are the ones a batch
//! could have run in: the saved tabs and the recent folders. Nothing is
//! recovered while another Rustphin runs, since its batch may still be going.

use std::path::PathBuf;

use gpui_kit::{App, AppContext as _};
use rustphin_core::{RealFs, ops::Request};

use crate::{
    history,
    location::Location,
    operations::{folder_name, hub},
    session::Session,
};

/// Recent folders checked besides the saved tabs.
const RECENT_FOLDERS_CHECKED: usize = 200;

/// The local folders to check, saved tabs first, each once.
fn candidates(session: &Session, recent: Vec<Location>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    let tabs = session
        .tabs
        .iter()
        .flat_map(|tab| tab.panes.iter().cloned());
    for location in tabs.chain(recent) {
        if let Location::Folder(path) = location
            && !folders.contains(&path)
        {
            folders.push(path);
        }
    }
    folders
}

/// Looks for unfinished batch renames in the background and recovers them.
pub fn start(session: &Session, cx: &mut App) {
    let session = session.clone();
    cx.spawn(async move |cx| {
        let found = cx
            .background_spawn(async move {
                if rustphin_platform::process::others_running("rustphin") {
                    tracing::info!("another Rustphin is running; rename recovery left to it");
                    return Vec::new();
                }
                let folders = candidates(&session, history::saved_locations(RECENT_FOLDERS_CHECKED));
                folders
                    .into_iter()
                    .filter(|folder| rustphin_core::ops::has_rename_journal(&RealFs, folder))
                    .collect::<Vec<_>>()
            })
            .await;
        if found.is_empty() {
            return;
        }
        cx.update(|cx| {
            hub(cx).update(cx, |hub, _| {
                for directory in found {
                    let name = folder_name(&directory);
                    tracing::warn!(folder = %directory.display(), "recovering an interrupted batch rename");
                    hub.start_lasting(
                        Request::RecoverRenameBatch { directory },
                        format!("Restoring original names in {name} after an interrupted rename"),
                        format!("restored the original names in {name} after an interrupted rename"),
                    );
                }
            });
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use rustphin_platform::remote::RemoteAddress;

    use super::*;
    use crate::session::SessionTab;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn saved_tabs_come_first_and_each_local_folder_once() {
        let folder = |path: &str| Location::Folder(PathBuf::from(path));
        let session = Session::new(
            vec![SessionTab {
                panes: vec![
                    folder("/work"),
                    Location::Remote(RemoteAddress::parse("sftp://host/x").unwrap()),
                ],
                active_pane: 0,
            }],
            0,
        );
        let recent = vec![folder("/photos"), folder("/work"), folder("/music")];

        assert_eq!(
            candidates(&session, recent),
            [
                PathBuf::from("/work"),
                PathBuf::from("/photos"),
                PathBuf::from("/music")
            ]
        );
    }
}
