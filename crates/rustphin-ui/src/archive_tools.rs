//! Extraction by an installed tool for archives Rustphin does not open
//! itself (W-111f): the tool runs in the background, and the notice bar
//! says how it went. Ark shows its own progress.

use std::path::PathBuf;

use gpui_kit::{App, AppContext as _};
use rustphin_core::ops::{Layout, Request};

use crate::operations::hub;

/// Extracts `archive` into `into` with Ark, 7-Zip or unrar; with `trash`,
/// moves the archive to the trash when the tool succeeded.
pub(crate) fn extract(archive: PathBuf, into: PathBuf, layout: Layout, trash: bool, cx: &mut App) {
    let name = crate::operations::folder_name(&archive);
    let folder = (layout == Layout::NewFolder).then(|| {
        use std::os::unix::ffi::OsStrExt;
        let name = archive
            .file_name()
            .map(OsStrExt::as_bytes)
            .unwrap_or_default();
        std::ffi::OsStr::from_bytes(crate::archives::folder_stem(name)).to_owned()
    });
    let task = {
        let archive = archive.clone();
        cx.background_spawn(async move {
            rustphin_platform::archive_tool::extract(&archive, &into, folder.as_deref())
        })
    };
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update(|cx| {
            hub(cx).update(cx, |hub, cx| match result {
                Ok(tool) => {
                    hub.say(format!("{tool} extracted {name}."), false, cx);
                    if trash {
                        hub.start(Request::Trash {
                            paths: vec![archive],
                        });
                    }
                }
                Err(error) => hub.say(format!("Could not extract {name}: {error}."), true, cx),
            });
        });
    })
    .detach();
}
