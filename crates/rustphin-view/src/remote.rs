//! What Rustphin says about remote folders opened through kio-fuse
//! (W-080), free of any toolkit: their short names and why one could not
//! be opened.

use rustphin_platform::remote::{MOUNT_TIMEOUT, RemoteError};

/// The last folder name in a remote address, or its host at the top:
/// `sftp://me@host/srv/data` gives `data`, `sftp://me@host/` gives `me@host`.
pub fn short_name(shown: &str) -> &str {
    let trimmed = shown.trim_end_matches('/');
    let after_scheme = trimmed.split_once(":").map_or(trimmed, |(_, rest)| rest);
    after_scheme
        .trim_start_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(shown)
}

/// What went wrong and what to do about it, as one notice.
pub fn failure(shown: &str, error: &RemoteError) -> String {
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
    fn a_remote_folder_is_named_by_its_last_folder_or_host() {
        assert_eq!(short_name("sftp://me@host/srv/data"), "data");
        assert_eq!(short_name("sftp://me@host/srv/data/"), "data");
        assert_eq!(short_name("sftp://me@host/"), "me@host");
        assert_eq!(short_name("smb://nas"), "nas");
        assert_eq!(short_name("tar:/tmp/a.tar/"), "a.tar");
    }

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
