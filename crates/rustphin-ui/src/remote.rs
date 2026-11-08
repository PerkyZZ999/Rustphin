//! Opening remote folders through kio-fuse, for the location bar and the
//! places panel. Mounting waits on the network and possibly a password
//! prompt, so it runs in the background behind a "Connecting…" notice.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::{App, AppContext as _, Context, Global};
use rustphin_platform::remote::{KioFuse, MOUNT_TIMEOUT, RemoteAddress, RemoteError};

use crate::operations::hub;

/// The process-wide kio-fuse client, which remembers what it mounted.
struct Folders(Arc<KioFuse>);

impl Global for Folders {}

/// Makes remote folders available. Nothing connects until one is opened.
pub fn init(cx: &mut App) {
    cx.set_global(Folders(Arc::new(KioFuse::session())));
}

fn folders(cx: &App) -> Arc<KioFuse> {
    cx.try_global::<Folders>().map_or_else(
        || Arc::new(KioFuse::session()),
        |folders| Arc::clone(&folders.0),
    )
}

/// The remote address of `path` when it lies in a folder opened through
/// [`open`] during this run.
pub fn address_of(path: &Path, cx: &App) -> Option<String> {
    folders(cx).remote_address_of(path)
}

/// Mounts `address` in the background and hands the local folder to `then`.
/// Failures are reported as error notices.
pub fn open<V: 'static>(
    address: RemoteAddress,
    cx: &mut Context<V>,
    then: impl FnOnce(&mut V, PathBuf, &mut Context<V>) + 'static,
) {
    let shown = address.to_string();
    let ticket = hub(cx).update(cx, |hub, cx| {
        hub.say_until_withdrawn(format!("Connecting to {shown}\u{2026}"), cx)
    });
    let folders = folders(cx);
    cx.spawn(async move |view, cx| {
        let mounted = cx
            .background_spawn(async move { folders.mount(&address) })
            .await;
        let _ = view.update(cx, |view, cx| {
            hub(cx).update(cx, |hub, cx| hub.withdraw(ticket, cx));
            match mounted {
                Ok(path) => then(view, path, cx),
                Err(error) => {
                    tracing::warn!(address = %shown, %error, "remote folder not opened");
                    let text = failure(&shown, &error);
                    hub(cx).update(cx, |hub, cx| hub.say(text, true, cx));
                }
            }
        });
    })
    .detach();
}

/// What went wrong and what to do about it, as one notice.
fn failure(shown: &str, error: &RemoteError) -> String {
    let advice = match error {
        RemoteError::Unavailable => {
            "Remote folders need kio-fuse. Install the kio-fuse package, then try again.".to_owned()
        }
        RemoteError::Unsupported(scheme) => format!(
            "kio-fuse cannot open {scheme} addresses. Open it in Dolphin or another KDE application instead."
        ),
        RemoteError::TimedOut => format!(
            "There was no answer within {} seconds. Check the server and your connection, then try again.",
            MOUNT_TIMEOUT.as_secs()
        ),
        RemoteError::Failed(reason) => format!(
            "{}. Check the address and your connection, then try again.",
            capitalized(reason.trim_end_matches('.'))
        ),
    };
    format!("{shown} could not be opened. {advice}")
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_failure_names_the_address_and_what_to_do() {
        let address = "sftp://me@host/srv";
        let cases = [
            (RemoteError::Unavailable, "Install the kio-fuse package"),
            (
                RemoteError::Unsupported("http".into()),
                "cannot open http addresses",
            ),
            (RemoteError::TimedOut, "no answer within 120 seconds"),
            (
                RemoteError::Failed("no route to host".into()),
                "No route to host. Check the address",
            ),
        ];
        for (error, advice) in cases {
            let text = failure(address, &error);
            assert!(
                text.starts_with("sftp://me@host/srv could not be opened. "),
                "{text}"
            );
            assert!(text.contains(advice), "{text}");
        }
    }
}
