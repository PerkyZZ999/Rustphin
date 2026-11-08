//! Remote folders (`sftp://`, `smb://`, …) reached through kio-fuse
//! (decision D-004). Rustphin never connects to the network itself
//! (D-018): it asks the kio-fuse service on the session bus to mount an
//! address and then browses the local folder it returns. kio-fuse, and the
//! KIO worker behind it, make the connection and ask for any password.
//!
//! This is the same `org.kde.KIOFuse.VFS` interface Dolphin's terminal panel
//! uses. Calls block until the service answers; callers run them off the UI
//! thread.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
    time::Duration,
};

use zbus::blocking::Connection;

use crate::uri::percent_encode;

const SERVICE: &str = "org.kde.KIOFuse";
const PATH: &str = "/org/kde/KIOFuse";
const VFS: &str = "org.kde.KIOFuse.VFS";

/// How long a mount may take, including the time the user spends in a
/// password prompt, before Rustphin gives up waiting.
pub const MOUNT_TIMEOUT: Duration = Duration::from_secs(120);

/// Network bookmark schemes the places panel shows and opens through
/// kio-fuse. Other non-file bookmarks (`remote:/`, `recentlyused:/`, search
/// folders) are KDE views Rustphin does not offer.
const NETWORK_SCHEMES: &[&str] = &[
    "sftp", "fish", "smb", "ftp", "ftps", "webdav", "webdavs", "nfs",
];

/// An address typed by the user or read from a bookmark that names a
/// folder somewhere other than this machine's file system.
///
/// It may carry a password (`sftp://me:secret@host/`); [`fmt::Display`] and
/// [`fmt::Debug`] leave it out, so the address can be shown and logged.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RemoteAddress {
    text: String,
    scheme_len: usize,
}

impl RemoteAddress {
    /// Reads `text` as `scheme:/…` or `scheme://…`. Returns `None` for local
    /// paths, `file:` URIs and text that is not an address.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (scheme, rest) = text.split_once(':')?;
        let mut chars = scheme.chars();
        let valid = chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
            && scheme.len() >= 2
            && rest.starts_with('/')
            && !scheme.eq_ignore_ascii_case("file")
            && !text.contains('\0');
        valid.then(|| Self {
            text: format!("{}:{rest}", scheme.to_ascii_lowercase()),
            scheme_len: scheme.len(),
        })
    }

    /// The scheme, in lower case: `sftp`, `smb`, ….
    pub fn scheme(&self) -> &str {
        &self.text[..self.scheme_len]
    }

    /// Whether a bookmark with this address belongs in the places panel.
    pub fn is_network_folder(&self) -> bool {
        NETWORK_SCHEMES.contains(&self.scheme())
    }

    /// The address with any password removed.
    fn shown(&self) -> String {
        let rest = &self.text[self.scheme_len + 1..];
        let Some(after_slashes) = rest.strip_prefix("//") else {
            return self.text.clone();
        };
        let end = after_slashes
            .find(['/', '?', '#'])
            .unwrap_or(after_slashes.len());
        let (authority, tail) = after_slashes.split_at(end);
        let authority = match authority.rsplit_once('@') {
            Some((user_info, host)) => {
                let user = user_info
                    .split_once(':')
                    .map_or(user_info, |(user, _)| user);
                format!("{user}@{host}")
            }
            None => authority.to_owned(),
        };
        format!("{}://{authority}{tail}", self.scheme())
    }
}

impl fmt::Display for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.shown())
    }
}

impl fmt::Debug for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RemoteAddress").field(&self.shown()).finish()
    }
}

/// Why a remote folder could not be opened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RemoteError {
    /// kio-fuse is not installed or the session bus cannot be reached.
    #[error("kio-fuse is not available")]
    Unavailable,
    /// kio-fuse refuses this kind of address.
    #[error("kio-fuse cannot open {0} addresses")]
    Unsupported(String),
    /// The service gave no answer in time.
    #[error("there was no answer in time")]
    TimedOut,
    /// Mounting failed: the host is unreachable, the login was refused, ….
    #[error("{0}")]
    Failed(String),
}

impl From<zbus::Error> for RemoteError {
    fn from(error: zbus::Error) -> Self {
        match &error {
            zbus::Error::MethodError(name, message, _) => {
                let message = message.clone().unwrap_or_default();
                match name.as_str().rsplit('.').next() {
                    Some("ServiceUnknown" | "NameHasNoOwner" | "UnknownObject") => {
                        Self::Unavailable
                    }
                    Some("SchemeNotSupported") => Self::Unsupported(String::new()),
                    // kio-fuse writes "KIOFuse failed to mount <url>: <reason>".
                    _ => Self::Failed(
                        message
                            .rsplit_once(": ")
                            .map_or(message.as_str(), |(_, reason)| reason)
                            .to_owned(),
                    ),
                }
            }
            zbus::Error::InputOutput(io) if io.kind() == std::io::ErrorKind::TimedOut => {
                Self::TimedOut
            }
            zbus::Error::Address(_) => Self::Unavailable,
            _ => Self::Failed(error.to_string()),
        }
    }
}

/// A folder kio-fuse mounted for us: its local root and remote address.
struct Mount {
    root: PathBuf,
    remote: String,
}

/// The kio-fuse service on the session bus, and the remote folders it has
/// mounted for this process.
pub struct KioFuse {
    /// A fixed connection; `None` connects to the session bus per request,
    /// so nothing is opened until a remote folder is asked for.
    connection: Option<Connection>,
    mounts: Mutex<Vec<Mount>>,
}

impl Default for KioFuse {
    fn default() -> Self {
        Self::session()
    }
}

impl KioFuse {
    /// The service on the user's session bus, started by D-Bus on first use.
    pub fn session() -> Self {
        Self {
            connection: None,
            mounts: Mutex::default(),
        }
    }

    /// Uses the service reachable over `connection`.
    pub fn on(connection: Connection) -> Self {
        Self {
            connection: Some(connection),
            mounts: Mutex::default(),
        }
    }

    fn connection(&self) -> Result<Connection, RemoteError> {
        match &self.connection {
            Some(connection) => Ok(connection.clone()),
            None => zbus::blocking::connection::Builder::session()
                .and_then(|builder| builder.method_timeout(MOUNT_TIMEOUT).build())
                .map_err(|_| RemoteError::Unavailable),
        }
    }

    fn call(&self, method: &str, argument: &str) -> Result<String, RemoteError> {
        let reply =
            self.connection()?
                .call_method(Some(SERVICE), PATH, Some(VFS), method, &(argument,))?;
        Ok(reply.body().deserialize::<String>()?)
    }

    /// Mounts `address` and returns the local folder that shows it.
    pub fn mount(&self, address: &RemoteAddress) -> Result<PathBuf, RemoteError> {
        let reply = self
            .call("mountUrl", &address.text)
            .map_err(|error| match error {
                RemoteError::Unsupported(_) => RemoteError::Unsupported(address.scheme().into()),
                other => other,
            })?;
        let root = PathBuf::from(reply);
        if !root.is_absolute() {
            return Err(RemoteError::Failed(
                "kio-fuse returned an unusable folder".into(),
            ));
        }
        // kio-fuse's own form of the address, without a password; ours if
        // it cannot say.
        let remote = self
            .call("remoteUrl", &root.to_string_lossy())
            .ok()
            .and_then(|text| RemoteAddress::parse(&text))
            .unwrap_or_else(|| address.clone())
            .to_string();
        let mut mounts = self.mounts.lock().unwrap_or_else(PoisonError::into_inner);
        mounts.retain(|mount| mount.root != root);
        mounts.push(Mount {
            root: root.clone(),
            remote,
        });
        Ok(root)
    }

    /// The remote address of `path` when it lies in a folder this process
    /// mounted, such as `sftp://host/projects/sub dir`.
    pub fn remote_address_of(&self, path: &Path) -> Option<String> {
        let (root, remote) = self.mount_root_of(path)?;
        let relative = path
            .strip_prefix(&root)
            .ok()?
            .as_os_str()
            .as_encoded_bytes();
        Some(if relative.is_empty() {
            remote
        } else {
            format!(
                "{}/{}",
                remote.trim_end_matches('/'),
                readable_path(relative)
            )
        })
    }

    /// The local root and remote address of the mounted folder that holds
    /// `path`, when this process mounted one.
    pub fn mount_root_of(&self, path: &Path) -> Option<(PathBuf, String)> {
        let mounts = self.mounts.lock().unwrap_or_else(PoisonError::into_inner);
        mounts
            .iter()
            .find(|mount| path.starts_with(&mount.root))
            .map(|mount| (mount.root.clone(), mount.remote.clone()))
    }
}

/// A path for showing inside an address, as Dolphin shows one: names stay
/// readable, and only what would change the address's meaning is encoded:
/// `%`, `#`, `?`, control characters and bytes that are not UTF-8.
fn readable_path(bytes: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return percent_encode(bytes);
    };
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() || matches!(c, '%' | '#' | '?') {
            let mut buffer = [0; 4];
            for byte in c.encode_utf8(&mut buffer).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        ffi::OsStr,
        os::unix::{ffi::OsStrExt as _, net::UnixStream},
        sync::{Arc, Mutex},
        thread,
    };

    use zbus::{blocking::connection::Builder, interface};

    use super::*;

    #[derive(Debug, zbus::DBusError)]
    #[zbus(prefix = "org.kde.KIOFuse.VFS.Error")]
    enum MockError {
        #[zbus(error)]
        ZBus(zbus::Error),
        SchemeNotSupported(String),
        CannotMount(String),
        #[zbus(name = "RemoteURLNotFound")]
        RemoteUrlNotFound(String),
    }

    /// A pretend kio-fuse that "mounts" under `/run/kio-fuse-test` and
    /// remembers what it was asked.
    #[derive(Default)]
    struct MockVfs {
        asked: Arc<Mutex<Vec<String>>>,
    }

    const ROOT: &str = "/run/kio-fuse-test";

    #[interface(name = "org.kde.KIOFuse.VFS")]
    impl MockVfs {
        #[zbus(name = "mountUrl")]
        fn mount_url(&self, remote_url: String) -> Result<String, MockError> {
            self.asked.lock().unwrap().push(remote_url.clone());
            let (scheme, rest) = remote_url.split_once("://").unwrap();
            match scheme {
                "http" => Err(MockError::SchemeNotSupported(format!(
                    "KIOFuse does not suport mounting of URLs with a scheme of {scheme}"
                ))),
                "sftp" if rest.starts_with("unreachable") => Err(MockError::CannotMount(
                    "KIOFuse failed to mount sftp://unreachable: No route to host".into(),
                )),
                _ => {
                    let host_and_path = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
                    Ok(format!("{ROOT}/{scheme}/{host_and_path}"))
                }
            }
        }

        #[zbus(name = "remoteUrl")]
        fn remote_url(&self, local_url: String) -> Result<String, MockError> {
            let relative = local_url
                .strip_prefix(&format!("{ROOT}/"))
                .ok_or_else(|| MockError::RemoteUrlNotFound(local_url.clone()))?;
            let (scheme, rest) = relative.split_once('/').unwrap();
            // As kio-fuse does: the user name stays, the password does not.
            Ok(format!("{scheme}://me@{rest}"))
        }
    }

    /// Answers every call as a bus does when kio-fuse is not installed.
    struct Absent;

    #[interface(name = "org.kde.KIOFuse.VFS")]
    impl Absent {
        #[zbus(name = "mountUrl")]
        fn mount_url(&self, _remote_url: String) -> zbus::fdo::Result<String> {
            Err(zbus::fdo::Error::ServiceUnknown(
                "The name org.kde.KIOFuse was not provided by any .service files".into(),
            ))
        }
    }

    /// Never answers, like a service stuck behind an unanswered prompt.
    struct Silent;

    #[interface(name = "org.kde.KIOFuse.VFS")]
    impl Silent {
        #[zbus(name = "mountUrl")]
        async fn mount_url(&self, _remote_url: String) -> String {
            std::future::pending::<String>().await
        }
    }

    fn client_for<T: zbus::object_server::Interface>(service: T, timeout: Duration) -> KioFuse {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let builder = Builder::async_io_unix_stream(theirs)
            .server(guid)
            .unwrap()
            .p2p()
            .serve_at(PATH, service)
            .unwrap();
        // Both ends shake hands at the same time.
        let service = thread::spawn(move || builder.build().unwrap());
        let client = Builder::async_io_unix_stream(ours)
            .p2p()
            .method_timeout(timeout)
            .build()
            .unwrap();
        // Keep the service end alive for the rest of the test process.
        std::mem::forget(service.join().unwrap());
        KioFuse::on(client)
    }

    fn address(text: &str) -> RemoteAddress {
        RemoteAddress::parse(text).unwrap()
    }

    #[test]
    fn addresses_are_told_apart_from_local_paths() {
        for remote in [
            "sftp://host/dir",
            " SMB://server/share ",
            "fish://me@host",
            "tar:/archives/a.tar",
            "webdavs://cloud.example/remote.php",
        ] {
            assert!(RemoteAddress::parse(remote).is_some(), "{remote}");
        }
        for local in [
            "/home/me",
            "~/Documents",
            "relative/dir",
            "notes:draft",
            "a:/single-letter",
            "file:///home/me",
            "FILE:/home/me",
            "2ftp://host",
            "sftp:",
            "",
        ] {
            assert!(RemoteAddress::parse(local).is_none(), "{local}");
        }
        assert_eq!(address("SMB://server/share").scheme(), "smb");
        assert_eq!(
            address("SMB://server/Share").to_string(),
            "smb://server/Share"
        );
    }

    #[test]
    fn passwords_are_never_shown_or_logged() {
        let typed = address("sftp://me:hunter2@host:22/srv?x#y");
        assert_eq!(typed.to_string(), "sftp://me@host:22/srv?x#y");
        assert!(!format!("{typed:?}").contains("hunter2"));
        assert_eq!(address("smb://:secret@nas/").to_string(), "smb://@nas/");
        assert_eq!(address("tar:/a:b@c").to_string(), "tar:/a:b@c");
    }

    #[test]
    fn only_network_bookmarks_belong_in_places() {
        assert!(address("sftp://host/").is_network_folder());
        assert!(address("smb://nas/share").is_network_folder());
        assert!(!address("remote:/").is_network_folder());
        assert!(!address("recentlyused:/files").is_network_folder());
        assert!(!address("trash:/").is_network_folder());
    }

    #[test]
    fn a_mounted_address_opens_its_local_folder_and_maps_paths_back() {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let vfs = MockVfs {
            asked: Arc::clone(&asked),
        };
        let kio = client_for(vfs, MOUNT_TIMEOUT);

        let root = kio.mount(&address("sftp://me:pw@host/projects")).unwrap();

        assert_eq!(root, Path::new("/run/kio-fuse-test/sftp/host/projects"));
        // kio-fuse needs the password the user typed; it is not kept by us.
        assert_eq!(asked.lock().unwrap()[0], "sftp://me:pw@host/projects");
        assert_eq!(
            kio.remote_address_of(&root.join("sub dir/é")).as_deref(),
            Some("sftp://me@host/projects/sub dir/é")
        );
        // Only what would change the address's meaning is encoded.
        let odd = root.join(OsStr::from_bytes(b"100% #1?\n\xff"));
        assert_eq!(
            kio.remote_address_of(&odd).as_deref(),
            Some("sftp://me@host/projects/100%25%20%231%3F%0A%FF")
        );
        assert_eq!(
            kio.remote_address_of(&root.join("100% #1?\n")).as_deref(),
            Some("sftp://me@host/projects/100%25 %231%3F%0A")
        );
        assert_eq!(
            kio.remote_address_of(&root).as_deref(),
            Some("sftp://me@host/projects")
        );
        assert_eq!(kio.remote_address_of(Path::new("/home/me")), None);
        assert_eq!(
            kio.mount_root_of(&root.join("sub dir")),
            Some((root.clone(), "sftp://me@host/projects".to_owned()))
        );
        assert_eq!(kio.mount_root_of(root.parent().unwrap()), None);
    }

    #[test]
    fn refusals_and_failures_say_why() {
        let kio = client_for(MockVfs::default(), MOUNT_TIMEOUT);
        assert_eq!(
            kio.mount(&address("http://example.org/")),
            Err(RemoteError::Unsupported("http".into()))
        );
        assert_eq!(
            kio.mount(&address("sftp://unreachable/")),
            Err(RemoteError::Failed("No route to host".into()))
        );
        assert_eq!(kio.remote_address_of(Path::new(ROOT)), None);
    }

    #[test]
    fn a_missing_service_is_reported_as_unavailable() {
        let kio = client_for(Absent, MOUNT_TIMEOUT);
        assert_eq!(
            kio.mount(&address("sftp://host/")),
            Err(RemoteError::Unavailable)
        );
    }

    #[test]
    fn a_service_that_never_answers_times_out() {
        let kio = client_for(Silent, Duration::from_millis(100));
        assert_eq!(
            kio.mount(&address("sftp://host/")),
            Err(RemoteError::TimedOut)
        );
    }
}
