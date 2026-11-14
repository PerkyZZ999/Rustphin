//! A stand-in for KDE's kio-admin helper, for interface tests only (W-098).
//!
//! It claims `org.kde.kio.admin` on the **session** bus of an isolated test
//! session and answers like the real helper, but works as the ordinary user
//! inside one sandbox folder: anything outside it is refused as KIO refuses
//! access. Rustphin finds it when started with `RUSTPHIN_ADMIN_BUS=session`.
//! Never installed, never run against a real home.
//!
//! ```sh
//! cargo run -p rustphin-platform --example fake_admin_helper -- /tmp/sandbox
//! FAKE_ADMIN_REFUSE=1 cargo run ... # every request is refused, as a cancelled password
//! ```

use std::{
    ffi::OsString,
    fs,
    io::Write as _,
    os::unix::{
        ffi::OsStringExt as _,
        fs::{MetadataExt as _, PermissionsExt as _},
    },
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use zbus::{
    ObjectServer, blocking::connection::Builder, interface, object_server::SignalEmitter,
    zvariant::OwnedObjectPath,
};

const UDS_STRING: u32 = 0x0100_0000;
const UDS_NUMBER: u32 = 0x0200_0000;
const NAME: u32 = 6 | UDS_STRING;
const SIZE: u32 = 1 | UDS_NUMBER;
const ACCESS: u32 = 9 | UDS_NUMBER;
const FILE_TYPE: u32 = 13 | UDS_NUMBER;
const MODIFICATION_TIME: u32 = 10 | 0x0400_0000 | UDS_NUMBER;
const LINK_DEST: u32 = 14 | UDS_STRING;
const USER_ID: u32 = 30 | UDS_NUMBER;
const GROUP_ID: u32 = 31 | UDS_NUMBER;
const DEVICE_ID: u32 = 27 | UDS_NUMBER;
const INODE: u32 = 28 | UDS_NUMBER;

const DOES_NOT_EXIST: i32 = 111;
const ALREADY_EXISTS: i32 = 112;
const ACCESS_DENIED: i32 = 115;
const UNSUPPORTED_ACTION: i32 = 108;
const CANNOT_WRITE: i32 = 129;

enum Value {
    Number(i64),
    Text(String),
}

/// Writes an entry the way KIO's `QDataStream` does.
fn encode(fields: &[(u32, Value)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend((fields.len() as u32).to_be_bytes());
    for (id, value) in fields {
        out.extend(id.to_be_bytes());
        match value {
            Value::Number(number) => out.extend(number.to_be_bytes()),
            Value::Text(text) => {
                let units: Vec<u16> = text.encode_utf16().collect();
                out.extend((units.len() as u32 * 2).to_be_bytes());
                for unit in units {
                    out.extend(unit.to_be_bytes());
                }
            }
        }
    }
    out
}

fn describe(path: &Path) -> std::io::Result<Vec<u8>> {
    let own = fs::symlink_metadata(path)?;
    // KIO gives a link the type and size of what it points to.
    let shown = fs::metadata(path).unwrap_or_else(|_| own.clone());
    let name = path.file_name().map_or_else(
        || "/".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let mut fields = vec![
        (NAME, Value::Text(name)),
        (SIZE, Value::Number(shown.len() as i64)),
        (
            FILE_TYPE,
            Value::Number(i64::from(shown.mode() & 0o170_000)),
        ),
        (ACCESS, Value::Number(i64::from(own.mode() & 0o7777))),
        (MODIFICATION_TIME, Value::Number(own.mtime())),
        (USER_ID, Value::Number(i64::from(own.uid()))),
        (GROUP_ID, Value::Number(i64::from(own.gid()))),
        (DEVICE_ID, Value::Number(own.dev() as i64)),
        (INODE, Value::Number(own.ino() as i64)),
    ];
    if own.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        fields.push((
            LINK_DEST,
            Value::Text(target.to_string_lossy().into_owned()),
        ));
    }
    Ok(encode(&fields))
}

/// What a command will do when started, and its outcome.
struct Outcome {
    entries: Vec<Vec<u8>>,
    stat: Option<Vec<u8>>,
    data: Option<Vec<u8>>,
    error: i32,
    message: String,
}

impl Outcome {
    fn done() -> Self {
        Self {
            entries: Vec::new(),
            stat: None,
            data: None,
            error: 0,
            message: String::new(),
        }
    }

    fn failed(error: i32, message: String) -> Self {
        Self {
            error,
            message,
            ..Self::done()
        }
    }

    fn from_io(result: std::io::Result<Self>, path: &Path) -> Self {
        result.unwrap_or_else(|error| {
            let code = match error.kind() {
                std::io::ErrorKind::NotFound => DOES_NOT_EXIST,
                std::io::ErrorKind::AlreadyExists => ALREADY_EXISTS,
                std::io::ErrorKind::PermissionDenied => ACCESS_DENIED,
                std::io::ErrorKind::CrossesDevices => UNSUPPORTED_ACTION,
                _ => CANNOT_WRITE,
            };
            Self::failed(code, format!("{}: {error}", path.display()))
        })
    }
}

struct Helper {
    sandbox: PathBuf,
    refuse: bool,
    counter: AtomicU64,
}

impl Helper {
    /// The local path of a `file://` URL inside the sandbox.
    fn path(&self, url: &str) -> Result<PathBuf, Outcome> {
        let rest = url
            .strip_prefix("file://")
            .ok_or_else(|| Outcome::failed(ACCESS_DENIED, format!("not a file URL: {url}")))?;
        let path = PathBuf::from(OsString::from_vec(percent_decode(rest)));
        if !inside(&self.sandbox, &path) {
            return Err(Outcome::failed(
                ACCESS_DENIED,
                format!("Access denied to {}.", path.display()),
            ));
        }
        Ok(path)
    }

    async fn command(
        &self,
        server: &ObjectServer,
        kind: &str,
        interface_name: &'static str,
        outcome: Outcome,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        if self.refuse {
            return Err(zbus::fdo::Error::AccessDenied(
                "Not authorized to perform operation".into(),
            ));
        }
        let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        let path = format!("/org/kde/kio/admin/{kind}/{n}");
        let command = Command {
            outcome: Mutex::new(Some(outcome)),
            pending: Arc::new(Mutex::new(None)),
        };
        match interface_name {
            "ListDirCommand" => server.at(path.as_str(), ListDir(command)).await?,
            "StatCommand" => server.at(path.as_str(), Stat(command)).await?,
            "GetCommand" => server.at(path.as_str(), Get(command)).await?,
            "PutCommand" => server.at(path.as_str(), Put(command)).await?,
            "MkdirCommand" => server.at(path.as_str(), Mkdir(command)).await?,
            "DelCommand" => server.at(path.as_str(), Del(command)).await?,
            "RenameCommand" => server.at(path.as_str(), Rename(command)).await?,
            "CopyCommand" => server.at(path.as_str(), Copy(command)).await?,
            "ChmodCommand" => server.at(path.as_str(), Chmod(command)).await?,
            _ => server.at(path.as_str(), Chown(command)).await?,
        };
        OwnedObjectPath::try_from(path).map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
}

/// Whether `path` lies inside `sandbox` (a canonical folder): the only
/// guard between the interface tests and the real home. The path is refused
/// when it climbs with `..` or when links lead out of the sandbox: what
/// exists of it is resolved, and the rest must be plain names.
fn inside(sandbox: &Path, path: &Path) -> bool {
    let plain = path
        .components()
        .all(|part| matches!(part, Component::RootDir | Component::Normal(_)));
    if !path.is_absolute() || !plain {
        return false;
    }
    path.ancestors()
        .find(|existing| fs::symlink_metadata(existing).is_ok())
        .and_then(|existing| existing.canonicalize().ok())
        .is_some_and(|real| real.starts_with(sandbox))
}

fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && at + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&text[at + 1..at + 3], 16)
        {
            out.push(byte);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    out
}

#[interface(name = "org.kde.kio.admin")]
impl Helper {
    /// The helper's `listDir`.
    #[zbus(name = "listDir")]
    async fn list_dir(
        &self,
        url: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("listDir {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => Outcome::from_io(
                fs::read_dir(&path).and_then(|entries| {
                    let mut listed = Vec::new();
                    for entry in entries {
                        listed.push(describe(&entry?.path())?);
                    }
                    Ok(Outcome {
                        entries: listed,
                        ..Outcome::done()
                    })
                }),
                &path,
            ),
            Err(outcome) => outcome,
        };
        self.command(server, "listDir", "ListDirCommand", outcome)
            .await
    }

    /// The helper's `stat`.
    #[zbus(name = "stat")]
    async fn stat(
        &self,
        url: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("stat {url:?}");
        // `/` is how Rustphin asks for access: always there.
        let outcome = if url == "file:///" {
            Outcome {
                stat: Some(encode(&[
                    (NAME, Value::Text("/".into())),
                    (FILE_TYPE, Value::Number(0o040_000)),
                    (ACCESS, Value::Number(0o755)),
                ])),
                ..Outcome::done()
            }
        } else {
            match self.path(&url) {
                Ok(path) => Outcome::from_io(
                    describe(&path).map(|bytes| Outcome {
                        stat: Some(bytes),
                        ..Outcome::done()
                    }),
                    &path,
                ),
                Err(outcome) => outcome,
            }
        };
        self.command(server, "stat", "StatCommand", outcome).await
    }

    /// The helper's `get`.
    #[zbus(name = "get")]
    async fn get(
        &self,
        url: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("get {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => Outcome::from_io(
                fs::read(&path).map(|data| Outcome {
                    data: Some(data),
                    ..Outcome::done()
                }),
                &path,
            ),
            Err(outcome) => outcome,
        };
        self.command(server, "get", "GetCommand", outcome).await
    }

    /// The helper's `put`.
    #[zbus(name = "put")]
    async fn put(
        &self,
        url: String,
        permissions: i32,
        flags: i32,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("put {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => {
                let created = fs::OpenOptions::new()
                    .write(true)
                    .create_new(flags & 4 == 0)
                    .create(true)
                    .truncate(true)
                    .open(&path)
                    .and_then(|_| {
                        if permissions >= 0 {
                            fs::set_permissions(
                                &path,
                                fs::Permissions::from_mode(permissions as u32),
                            )?;
                        }
                        Ok(Outcome {
                            // The put command writes what it is sent here.
                            data: Some(path.as_os_str().as_encoded_bytes().to_vec()),
                            ..Outcome::done()
                        })
                    });
                Outcome::from_io(created, &path)
            }
            Err(outcome) => outcome,
        };
        self.command(server, "put", "PutCommand", outcome).await
    }

    /// The helper's `mkdir`.
    #[zbus(name = "mkdir")]
    async fn mkdir(
        &self,
        url: String,
        permissions: i32,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("mkdir {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => Outcome::from_io(
                fs::create_dir(&path).and_then(|()| {
                    if permissions >= 0 {
                        fs::set_permissions(&path, fs::Permissions::from_mode(permissions as u32))?;
                    }
                    Ok(Outcome::done())
                }),
                &path,
            ),
            Err(outcome) => outcome,
        };
        self.command(server, "mkdir", "MkdirCommand", outcome).await
    }

    /// The helper's `del`.
    #[zbus(name = "del")]
    async fn del(
        &self,
        url: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("del {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => Outcome::from_io(
                fs::symlink_metadata(&path).and_then(|meta| {
                    if meta.is_dir() {
                        fs::remove_dir_all(&path)?;
                    } else {
                        fs::remove_file(&path)?;
                    }
                    Ok(Outcome::done())
                }),
                &path,
            ),
            Err(outcome) => outcome,
        };
        self.command(server, "del", "DelCommand", outcome).await
    }

    /// The helper's `rename`.
    #[zbus(name = "rename")]
    async fn rename(
        &self,
        from: String,
        to: String,
        flags: i32,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("rename {from:?}");
        let outcome = match (self.path(&from), self.path(&to)) {
            (Ok(from), Ok(to)) => Outcome::from_io(
                if flags & 4 == 0 && fs::symlink_metadata(&to).is_ok() {
                    Err(std::io::ErrorKind::AlreadyExists.into())
                } else {
                    fs::rename(&from, &to).map(|()| Outcome::done())
                },
                &to,
            ),
            (Err(outcome), _) | (_, Err(outcome)) => outcome,
        };
        self.command(server, "rename", "RenameCommand", outcome)
            .await
    }

    /// The helper's `copy`.
    #[zbus(name = "copy")]
    async fn copy(
        &self,
        from: String,
        to: String,
        _permissions: i32,
        _flags: i32,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("copy {from:?}");
        let outcome = match (self.path(&from), self.path(&to)) {
            (Ok(from), Ok(to)) => Outcome::from_io(
                if fs::symlink_metadata(&to).is_ok() {
                    Err(std::io::ErrorKind::AlreadyExists.into())
                } else {
                    fs::copy(&from, &to).map(|_| Outcome::done())
                },
                &to,
            ),
            (Err(outcome), _) | (_, Err(outcome)) => outcome,
        };
        self.command(server, "copy", "CopyCommand", outcome).await
    }

    /// The helper's `chmod`.
    #[zbus(name = "chmod")]
    async fn chmod(
        &self,
        url: String,
        permissions: i32,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("chmod {url:?}");
        let outcome = match self.path(&url) {
            Ok(path) => Outcome::from_io(
                fs::set_permissions(&path, fs::Permissions::from_mode(permissions as u32))
                    .map(|()| Outcome::done()),
                &path,
            ),
            Err(outcome) => outcome,
        };
        self.command(server, "chmod", "ChmodCommand", outcome).await
    }

    /// The helper's `chown`.
    #[zbus(name = "chown")]
    async fn chown(
        &self,
        url: String,
        _user: String,
        _group: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        eprintln!("chown {url:?}");
        // The stand-in runs as the user: it can only claim success for
        // what it already owns.
        let outcome = match self.path(&url) {
            Ok(path) => {
                Outcome::from_io(fs::symlink_metadata(&path).map(|_| Outcome::done()), &path)
            }
            Err(outcome) => outcome,
        };
        self.command(server, "chown", "ChownCommand", outcome).await
    }
}

/// The command objects the helper hands out.
// zbus's `interface` macro generates undocumented items for every signal,
// outside the annotated impl; the allow covers this plumbing module only.
#[allow(missing_docs)]
mod commands {
    use super::*;

    /// Signals go to the caller only, as kio-admin's `sendSignal` does.
    pub(crate) fn to_caller<'a>(
        emitter: SignalEmitter<'a>,
        header: &zbus::message::Header<'_>,
    ) -> SignalEmitter<'a> {
        match header.sender() {
            Some(sender) => emitter.set_destination(sender.to_owned().into()),
            None => emitter,
        }
    }

    /// The state every command object shares.
    pub(crate) struct Command {
        pub(crate) outcome: Mutex<Option<Outcome>>,
        /// A put command's open file, while data arrives.
        pub(crate) pending: Arc<Mutex<Option<PathBuf>>>,
    }

    impl Command {
        fn take(&self) -> Outcome {
            self.outcome
                .lock()
                .ok()
                .and_then(|mut outcome| outcome.take())
                .unwrap_or_else(|| Outcome::failed(CANNOT_WRITE, "started twice".into()))
        }
    }

    pub(crate) struct ListDir(pub(crate) Command);
    #[interface(name = "org.kde.kio.admin.ListDirCommand")]
    impl ListDir {
        /// The helper's `start`.
        #[zbus(name = "start")]
        async fn start(
            &self,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
            #[zbus(header)] header: zbus::message::Header<'_>,
        ) -> zbus::fdo::Result<()> {
            let emitter = to_caller(emitter, &header);
            let outcome = self.0.take();
            let list: Vec<(Vec<u8>,)> = outcome.entries.into_iter().map(|bytes| (bytes,)).collect();
            if !list.is_empty() {
                Self::entries(&emitter, list).await?;
            }
            Self::result(&emitter, outcome.error, outcome.message).await?;
            Ok(())
        }

        /// Entries of the folder listed.
        #[zbus(signal, name = "entries")]
        async fn entries(emitter: &SignalEmitter<'_>, list: Vec<(Vec<u8>,)>) -> zbus::Result<()>;

        /// The command ended; `error` is 0 or KIO's error number.
        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    pub(crate) struct Stat(pub(crate) Command);
    #[interface(name = "org.kde.kio.admin.StatCommand")]
    impl Stat {
        /// The helper's `start`.
        #[zbus(name = "start")]
        async fn start(
            &self,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
            #[zbus(header)] header: zbus::message::Header<'_>,
        ) -> zbus::fdo::Result<()> {
            let emitter = to_caller(emitter, &header);
            let outcome = self.0.take();
            if let Some(bytes) = outcome.stat {
                Self::stat_entry(&emitter, (bytes,)).await?;
            }
            Self::result(&emitter, outcome.error, outcome.message).await?;
            Ok(())
        }

        /// The one entry described.
        #[zbus(signal, name = "statEntry")]
        async fn stat_entry(emitter: &SignalEmitter<'_>, entry: (Vec<u8>,)) -> zbus::Result<()>;

        /// The command ended; `error` is 0 or KIO's error number.
        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    pub(crate) struct Get(pub(crate) Command);
    #[interface(name = "org.kde.kio.admin.GetCommand")]
    impl Get {
        /// The helper's `start`.
        #[zbus(name = "start")]
        async fn start(
            &self,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
            #[zbus(header)] header: zbus::message::Header<'_>,
        ) -> zbus::fdo::Result<()> {
            let emitter = to_caller(emitter, &header);
            let outcome = self.0.take();
            if let Some(data) = outcome.data {
                for piece in data.chunks(64 * 1024) {
                    Self::data(&emitter, piece.to_vec()).await?;
                }
            }
            Self::result(&emitter, outcome.error, outcome.message).await?;
            Ok(())
        }

        /// A piece of the file read.
        #[zbus(signal, name = "data")]
        async fn data(emitter: &SignalEmitter<'_>, data: Vec<u8>) -> zbus::Result<()>;

        /// The command ended; `error` is 0 or KIO's error number.
        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    pub(crate) struct Put(pub(crate) Command);
    #[interface(name = "org.kde.kio.admin.PutCommand")]
    impl Put {
        /// The helper's `start`.
        #[zbus(name = "start")]
        async fn start(
            &self,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
            #[zbus(header)] header: zbus::message::Header<'_>,
        ) -> zbus::fdo::Result<()> {
            let emitter = to_caller(emitter, &header);
            let outcome = self.0.take();
            if outcome.error != 0 {
                Self::result(&emitter, outcome.error, outcome.message).await?;
                return Ok(());
            }
            let path = outcome
                .data
                .map(|bytes| PathBuf::from(String::from_utf8_lossy(&bytes).into_owned()));
            if let Ok(mut pending) = self.0.pending.lock() {
                *pending = path;
            }
            Self::data_request(&emitter).await?;
            Ok(())
        }

        /// The helper's `data`.
        #[zbus(name = "data")]
        async fn data(
            &self,
            piece: Vec<u8>,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
            #[zbus(header)] header: zbus::message::Header<'_>,
        ) -> zbus::fdo::Result<()> {
            let emitter = to_caller(emitter, &header);
            let path = self
                .0
                .pending
                .lock()
                .ok()
                .and_then(|pending| pending.clone());
            if piece.is_empty() {
                Self::result(&emitter, 0, String::new()).await?;
                return Ok(());
            }
            if let Some(path) = path {
                let written = fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .and_then(|mut file| file.write_all(&piece));
                if let Err(error) = written {
                    Self::result(&emitter, CANNOT_WRITE, error.to_string()).await?;
                    return Ok(());
                }
            }
            Self::data_request(&emitter).await?;
            Ok(())
        }

        /// Asks for the next piece to write.
        #[zbus(signal, name = "dataRequest")]
        async fn data_request(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

        /// The command ended; `error` is 0 or KIO's error number.
        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    /// Mkdir, del, rename, copy, chmod and chown: the work is done when the
    /// command is created; `start` reports it.
    macro_rules! simple_command {
        ($name:ident, $interface:literal) => {
            pub(crate) struct $name(pub(crate) Command);
            #[interface(name = $interface)]
            impl $name {
                /// The helper's `start`.
                #[zbus(name = "start")]
                async fn start(
                    &self,
                    #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
                    #[zbus(header)] header: zbus::message::Header<'_>,
                ) -> zbus::fdo::Result<()> {
                    let emitter = to_caller(emitter, &header);
                    let outcome = self.0.take();
                    Self::result(&emitter, outcome.error, outcome.message).await?;
                    Ok(())
                }

                /// The command ended; `error` is 0 or KIO's error number.
                #[zbus(signal, name = "result")]
                async fn result(
                    emitter: &SignalEmitter<'_>,
                    error: i32,
                    message: String,
                ) -> zbus::Result<()>;
            }
        };
    }

    simple_command!(Mkdir, "org.kde.kio.admin.MkdirCommand");
    simple_command!(Del, "org.kde.kio.admin.DelCommand");
    simple_command!(Rename, "org.kde.kio.admin.RenameCommand");
    simple_command!(Copy, "org.kde.kio.admin.CopyCommand");
    simple_command!(Chmod, "org.kde.kio.admin.ChmodCommand");
    simple_command!(Chown, "org.kde.kio.admin.ChownCommand");
}

use commands::{Chmod, Chown, Command, Copy, Del, Get, ListDir, Mkdir, Put, Rename, Stat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: fake_admin_helper <sandbox folder>")?;
    let helper = Helper {
        sandbox: sandbox.canonicalize()?,
        refuse: std::env::var_os("FAKE_ADMIN_REFUSE").is_some(),
        counter: AtomicU64::new(0),
    };
    let _connection = Builder::session()?
        .name("org.kde.kio.admin")?
        .serve_at("/", helper)?
        .build()?;
    eprintln!("fake kio-admin helper serving {}", sandbox.display());
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A canonical sandbox and, beside it, the folder it must keep out of.
    fn sandbox() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (sandbox, home) = (root.join("sandbox"), root.join("home"));
        fs::create_dir_all(sandbox.join("docs")).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join("precious"), "real").unwrap();
        std::os::unix::fs::symlink(&home, sandbox.join("to-home")).unwrap();
        std::os::unix::fs::symlink(home.join("precious"), sandbox.join("to-precious")).unwrap();
        (temp, sandbox, home)
    }

    #[test]
    fn paths_in_the_sandbox_are_served_even_before_they_exist() {
        let (_temp, sandbox, _home) = sandbox();

        assert!(inside(&sandbox, &sandbox));
        assert!(inside(&sandbox, &sandbox.join("docs")));
        assert!(inside(&sandbox, &sandbox.join("docs/new file")));
        assert!(inside(&sandbox, &sandbox.join("docs/new folder/new file")));
    }

    #[test]
    fn climbing_out_with_dots_is_refused() {
        let (_temp, sandbox, _home) = sandbox();

        assert!(!inside(&sandbox, &sandbox.join("../home/precious")));
        assert!(!inside(&sandbox, &sandbox.join("docs/../../home")));
        assert!(!inside(&sandbox, &sandbox.join("missing/../../home")));
    }

    #[test]
    fn links_leading_out_of_the_sandbox_are_refused() {
        let (_temp, sandbox, _home) = sandbox();

        assert!(!inside(&sandbox, &sandbox.join("to-home")));
        assert!(!inside(&sandbox, &sandbox.join("to-home/precious")));
        assert!(!inside(&sandbox, &sandbox.join("to-home/new file")));
        assert!(!inside(&sandbox, &sandbox.join("to-precious")));
    }

    #[test]
    fn a_relative_path_is_refused() {
        let (_temp, sandbox, _home) = sandbox();

        assert!(!inside(&sandbox, Path::new("docs")));
    }
}
