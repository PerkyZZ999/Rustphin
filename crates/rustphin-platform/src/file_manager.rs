//! The `org.freedesktop.FileManager1` service other applications use to ask a
//! file manager to show a file.
//!
//! Parsing does not require a bus. The service can run on the session bus or,
//! in tests, on a private connection.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use zbus::interface;

use crate::uri::path_from_uri;

const NAME: &str = "org.freedesktop.FileManager1";
const PATH: &str = "/org/freedesktop/FileManager1";

/// What another application asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    /// Open these folders, selecting the named items.
    Show(Vec<Shown>),
    /// Show properties for these paths.
    Properties(Vec<PathBuf>),
}

/// A folder to show, and the names to select in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// The folder to open.
    pub folder: PathBuf,
    /// File names to select. Empty when the request was to open the folder itself.
    pub select: Vec<OsString>,
}

/// What `ShowItems` is asking for.
///
/// A URI that names a directory opens that directory. Any other local path
/// opens its parent and selects its name, including a path that is not there
/// yet. URIs that are not local files are left out. Items in one folder stay
/// together, in the order their URIs arrived.
pub fn show_items(uris: &[impl AsRef<[u8]>]) -> Vec<Shown> {
    let mut shown = Vec::new();
    for uri in uris {
        let Some(path) = path_from_uri(uri.as_ref()) else {
            continue;
        };
        let is_dir = std::fs::metadata(&path).is_ok_and(|meta| meta.is_dir());
        if is_dir {
            push(&mut shown, path, None);
            continue;
        }
        let Some(name) = path.file_name().map(OsStr::to_os_string) else {
            continue;
        };
        let Some(folder) = path.parent().map(Path::to_path_buf) else {
            continue;
        };
        if folder.as_os_str().is_empty() {
            continue;
        }
        push(&mut shown, folder, Some(name));
    }
    shown
}

/// What `ShowFolders` is asking for: each local path opened as a folder.
/// A path that names a file opens the folder that contains it, with nothing selected.
pub fn show_folders(uris: &[impl AsRef<[u8]>]) -> Vec<Shown> {
    let mut shown = Vec::new();
    for uri in uris {
        let Some(path) = path_from_uri(uri.as_ref()) else {
            continue;
        };
        let is_file = std::fs::metadata(&path).is_ok_and(|meta| meta.is_file());
        let folder = if is_file {
            path.parent().map(Path::to_path_buf)
        } else {
            Some(path)
        };
        if let Some(folder) = folder.filter(|folder| !folder.as_os_str().is_empty()) {
            push(&mut shown, folder, None);
        }
    }
    shown
}

/// Local paths a properties request names. Anything that is not a local file
/// is left out.
pub fn property_paths(uris: &[impl AsRef<[u8]>]) -> Vec<PathBuf> {
    uris.iter()
        .filter_map(|uri| path_from_uri(uri.as_ref()))
        .collect()
}

fn push(shown: &mut Vec<Shown>, folder: PathBuf, name: Option<OsString>) {
    if let Some(group) = shown.iter_mut().find(|group| group.folder == folder) {
        if let Some(name) = name {
            group.select.push(name);
        }
    } else {
        shown.push(Shown {
            folder,
            select: name.into_iter().collect(),
        });
    }
}

struct Service {
    requests: flume::Sender<Call>,
}

#[interface(name = "org.freedesktop.FileManager1")]
impl Service {
    fn show_items(&self, uris: Vec<String>, _startup_id: String) {
        let _ = self.requests.send(Call::Show(show_items(&uris)));
    }

    fn show_folders(&self, uris: Vec<String>, _startup_id: String) {
        let _ = self.requests.send(Call::Show(show_folders(&uris)));
    }

    /// Asks for the properties of each local path.
    fn show_item_properties(&self, uris: Vec<String>, _startup_id: String) {
        let _ = self.requests.send(Call::Properties(property_paths(&uris)));
    }
}

/// A running service and the requests it receives.
pub struct ServiceHandle {
    requests: flume::Receiver<Call>,
    _connection: zbus::blocking::Connection,
}

impl ServiceHandle {
    /// Claims the well-known name on the session bus.
    ///
    /// `None` when the bus cannot be reached or another file manager already
    /// owns the name. The caller keeps this value: dropping it disconnects.
    pub fn session() -> Option<Self> {
        let (requests, incoming) = flume::unbounded();
        let connection = zbus::blocking::connection::Builder::session()
            .ok()?
            .name(NAME)
            .ok()?
            .serve_at(PATH, Service { requests })
            .ok()?
            .build()
            .ok()?;
        Some(Self {
            requests: incoming,
            _connection: connection,
        })
    }

    /// The next request, if one is waiting.
    pub fn try_recv(&self) -> Option<Call> {
        self.requests.try_recv().ok()
    }

    /// A clone of the queue, so a task can wait without borrowing this handle.
    pub fn receiver(&self) -> flume::Receiver<Call> {
        self.requests.clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        os::unix::{ffi::OsStringExt, net::UnixStream},
        thread,
        time::Duration,
    };

    use zbus::blocking::{Proxy, connection::Builder};

    use super::*;
    use crate::uri::file_uri;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn a_file_opens_its_folder_and_is_selected() {
        let dir = dir();
        let file = dir.path().join("note.txt");
        std::fs::write(&file, b"note").unwrap();
        let shown = show_items(&[file_uri(&file)]);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].folder, dir.path());
        assert_eq!(shown[0].select, [OsString::from("note.txt")]);
    }

    #[test]
    fn files_in_one_folder_stay_together_and_two_folders_stay_apart() {
        let dir = dir();
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let c = other.join("c.txt");
        std::fs::write(&a, b"").unwrap();
        std::fs::write(&b, b"").unwrap();
        std::fs::write(&c, b"").unwrap();
        let shown = show_items(&[file_uri(&a), file_uri(&c), file_uri(&b)]);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].folder, dir.path());
        assert_eq!(
            shown[0].select,
            [OsString::from("a.txt"), OsString::from("b.txt")]
        );
        assert_eq!(shown[1].folder, other);
        assert_eq!(shown[1].select, [OsString::from("c.txt")]);
    }

    #[test]
    fn a_directory_opens_itself_and_a_missing_file_is_still_named() {
        let dir = dir();
        let folder = dir.path().join("docs");
        std::fs::create_dir(&folder).unwrap();
        let missing = dir.path().join("gone.txt");
        let shown = show_items(&[
            file_uri(&folder),
            file_uri(&missing),
            "file://files.example/remote".into(),
        ]);
        assert_eq!(
            shown,
            vec![
                Shown {
                    folder: folder.clone(),
                    select: vec![],
                },
                Shown {
                    folder: dir.path().to_path_buf(),
                    select: vec![OsString::from("gone.txt")],
                },
            ]
        );
    }

    #[test]
    fn a_name_that_is_not_text_is_kept_as_bytes() {
        let dir = dir();
        let name = OsString::from_vec(b"a\xff".to_vec());
        let file = dir.path().join(&name);
        std::fs::write(&file, b"x").unwrap();
        let shown = show_items(&[file_uri(&file)]);
        assert_eq!(shown[0].select, [name]);
    }

    #[test]
    fn show_items_on_a_private_bus_delivers_the_folder_and_the_name() {
        let dir = dir();
        let file = dir.path().join("picked.txt");
        std::fs::write(&file, b"x").unwrap();
        let (requests, incoming) = flume::unbounded();
        let (server_stream, client_stream) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let server = thread::spawn(move || {
            Builder::async_io_unix_stream(server_stream)
                .server(guid)
                .unwrap()
                .p2p()
                .serve_at(PATH, Service { requests })
                .unwrap()
                .build()
                .unwrap()
        });
        let client = Builder::async_io_unix_stream(client_stream)
            .p2p()
            .build()
            .unwrap();
        let server = server.join().unwrap();

        let proxy = Proxy::new(&client, NAME, PATH, NAME).unwrap();
        proxy
            .call::<_, _, ()>("ShowItems", &(vec![file_uri(&file)], String::new()))
            .unwrap();
        let Call::Show(shown) = incoming.recv_timeout(Duration::from_secs(2)).unwrap() else {
            panic!("expected a show request");
        };
        drop(server);
        assert_eq!(shown[0].folder, dir.path());
        assert_eq!(shown[0].select, [OsString::from("picked.txt")]);
    }
}
