//! Opening remote folders through kio-fuse (W-080, W-114e). Mounting waits
//! on the network and perhaps a password prompt, so it runs in the
//! background behind a "Connecting…" notice.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustphin_platform::remote::{KioFuse, RemoteAddress};
use rustphin_view::remote::failure;

use crate::{hub, post};

thread_local! {
    static FOLDERS: RefCell<Option<Arc<KioFuse>>> = const { RefCell::new(None) };
}

fn folders() -> Arc<KioFuse> {
    FOLDERS.with(|folders| {
        Arc::clone(
            folders
                .borrow_mut()
                .get_or_insert_with(|| Arc::new(KioFuse::session())),
        )
    })
}

/// The remote address a kio-fuse path belongs to, if it is one.
pub fn address_of(path: &Path) -> Option<String> {
    folders().remote_address_of(path)
}

/// The mount a path is under, and its remote address.
pub fn root_of(path: &Path) -> Option<(PathBuf, String)> {
    folders().mount_root_of(path)
}

/// Mounts `address` and calls `then` on the Qt thread with where it is
/// mounted, or `None` once the failure has been said.
pub fn open(address: RemoteAddress, then: impl FnOnce(Option<PathBuf>) + Send + 'static) {
    let shown = address.to_string();
    let ticket = hub::say_until_withdrawn(format!("Connecting to {shown}\u{2026}"));
    let folders = folders();
    std::thread::spawn(move || {
        let mounted = folders.mount(&address);
        post::on_qt(move || {
            hub::withdraw(ticket);
            match mounted {
                Ok(path) => then(Some(path)),
                Err(error) => {
                    tracing::warn!(address = %shown, %error, "remote folder not opened");
                    hub::say(failure(&shown, &error), true);
                    then(None);
                }
            }
        });
    });
}
