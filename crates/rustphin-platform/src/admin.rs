//! Acting as administrator through KDE's kio-admin helper (D-028,
//! docs/project/ADMIN_MODE.md).
//!
//! `kio-admin-helper` runs as root on the system bus as `org.kde.kio.admin`.
//! Every call is checked against the polkit action
//! `org.kde.kio.admin.commands` for the calling connection; KDE's polkit
//! agent asks for the password and polkit remembers the answer for this
//! process for a while. Rustphin never sees the password and never runs as
//! root.
//!
//! An operation is a method on `/` that returns the path of a command
//! object. Calling `start()` on it makes the helper send signals back to this
//! connection only (`entries`, `statEntry`, `data`, `dataRequest`) and
//! finally `result(int error, string message)`.
//!
//! Directory entries are KIO's `UDSEntry`, serialised with Qt's
//! `QDataStream` into a byte array (see [`decode_entry`]). The interface is
//! private to KDE: anything unexpected is an error, never a guess.

use std::{
    os::unix::ffi::OsStrExt as _,
    path::Path,
    sync::{Mutex, PoisonError},
    thread,
    time::{Duration, Instant},
};

use zbus::{
    blocking::{Connection, MessageIterator},
    message::Type,
    zvariant::OwnedObjectPath,
};

use crate::uri::percent_encode;

const SERVICE: &str = "org.kde.kio.admin";
const ROOT_PATH: &str = "/";
const INTERFACE: &str = "org.kde.kio.admin";

/// For interface tests only: set to `session` to look for the helper on
/// the session bus, where a fake helper can serve a sandbox as the user.
/// Never needed in use: the real helper is on the system bus.
pub const TEST_BUS_VARIABLE: &str = "RUSTPHIN_ADMIN_BUS";

fn test_bus() -> bool {
    std::env::var_os(TEST_BUS_VARIABLE).is_some_and(|bus| bus == "session")
}

/// Whether KDE's helper is installed (its D-Bus service file is present),
/// so the toolbar can say so before anything is asked. Reads a few file
/// names; call off the UI thread.
pub fn installed() -> bool {
    test_bus()
        || crate::xdg::data_dirs().iter().any(|dir| {
            dir.join("dbus-1/system-services/org.kde.kio.admin.service")
                .is_file()
        })
}

/// The process's file-creation mask, from `/proc/self/status` (reading it
/// any other way changes it). `0o022` when it cannot be read.
fn process_umask() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                let value = line.strip_prefix("Umask:")?.trim();
                u32::from_str_radix(value, 8).ok()
            })
        })
        .unwrap_or(0o022)
}

/// How long an operation may take, including the time spent in KDE's
/// password dialog, before Rustphin stops waiting.
pub const ADMIN_TIMEOUT: Duration = Duration::from_secs(120);

/// Largest file read or written through the helper in one go.
pub const MAX_TRANSFER: usize = 256 * 1024 * 1024;

/// Size of the pieces a file is written in.
const CHUNK: usize = 1024 * 1024;

/// Most fields one entry may have; KIO sends a few dozen at most.
const MAX_FIELDS: u32 = 512;

/// KIO's field kinds (`UDSEntry::UDS_STRING`, `UDS_NUMBER`).
const UDS_STRING: u32 = 0x0100_0000;
const UDS_NUMBER: u32 = 0x0200_0000;

/// The fields Rustphin reads (KIO's `StandardFieldTypes`).
mod field {
    use super::{UDS_NUMBER, UDS_STRING};
    pub const SIZE: u32 = 1 | UDS_NUMBER;
    pub const USER: u32 = 3 | UDS_STRING;
    pub const GROUP: u32 = 5 | UDS_STRING;
    pub const NAME: u32 = 6 | UDS_STRING;
    pub const ACCESS: u32 = 9 | UDS_NUMBER;
    pub const MODIFICATION_TIME: u32 = 10 | 0x0400_0000 | UDS_NUMBER;
    pub const ACCESS_TIME: u32 = 11 | 0x0400_0000 | UDS_NUMBER;
    pub const CREATION_TIME: u32 = 12 | 0x0400_0000 | UDS_NUMBER;
    pub const FILE_TYPE: u32 = 13 | UDS_NUMBER;
    pub const LINK_DEST: u32 = 14 | UDS_STRING;
    pub const DEVICE_ID: u32 = 27 | UDS_NUMBER;
    pub const INODE: u32 = 28 | UDS_NUMBER;
    pub const LOCAL_USER_ID: u32 = 30 | UDS_NUMBER;
    pub const LOCAL_GROUP_ID: u32 = 31 | UDS_NUMBER;
    pub const MODIFICATION_TIME_NS: u32 = 34 | UDS_NUMBER;
    pub const ACCESS_TIME_NS: u32 = 35 | UDS_NUMBER;
    pub const CREATION_TIME_NS: u32 = 36 | UDS_NUMBER;
}

/// KIO's job flag that allows replacing an existing destination.
const KIO_OVERWRITE: i32 = 4;

/// KIO's error numbers (`KIO::Error`) Rustphin tells apart.
mod kio_error {
    pub const USER_CANCELED: i32 = 1;
    pub const UNSUPPORTED_ACTION: i32 = 108;
    pub const IS_DIRECTORY: i32 = 109;
    pub const IS_FILE: i32 = 110;
    pub const DOES_NOT_EXIST: i32 = 111;
    pub const FILE_ALREADY_EXIST: i32 = 112;
    pub const DIR_ALREADY_EXIST: i32 = 113;
    pub const ACCESS_DENIED: i32 = 115;
    pub const WRITE_ACCESS_DENIED: i32 = 116;
    pub const CANNOT_ENTER_DIRECTORY: i32 = 117;
    pub const DISK_FULL: i32 = 161;
}

/// Why acting as administrator failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdminError {
    /// kio-admin is not installed, or the system bus cannot be reached.
    #[error("KDE's administrator helper (kio-admin) is not available")]
    Unavailable,
    /// The password was not given, or polkit refused.
    #[error("administrator access was not granted")]
    Denied,
    /// The helper gave no answer in time.
    #[error("the administrator helper did not answer in time")]
    TimedOut,
    /// A name cannot be sent to the helper without risk of naming another
    /// file: it is not valid UTF-8 or not an absolute path.
    #[error("“{0}” cannot be handled as administrator: its name is not plain text")]
    Unaddressable(String),
    /// The helper answered in a way Rustphin does not understand.
    #[error("the administrator helper sent something unexpected: {0}")]
    Protocol(String),
    /// The operation failed; `code` is KIO's error number.
    #[error("{message}")]
    Failed {
        /// KIO's error number.
        code: i32,
        /// KIO's own description.
        message: String,
    },
}

impl AdminError {
    /// The closest standard error kind, for the core's file operations.
    pub fn io_kind(&self) -> std::io::ErrorKind {
        use std::io::ErrorKind as K;
        match self {
            Self::Denied => K::PermissionDenied,
            Self::TimedOut => K::TimedOut,
            Self::Unaddressable(_) => K::InvalidFilename,
            Self::Unavailable | Self::Protocol(_) => K::Other,
            Self::Failed { code, .. } => match *code {
                kio_error::DOES_NOT_EXIST => K::NotFound,
                kio_error::ACCESS_DENIED
                | kio_error::WRITE_ACCESS_DENIED
                | kio_error::CANNOT_ENTER_DIRECTORY => K::PermissionDenied,
                kio_error::FILE_ALREADY_EXIST | kio_error::DIR_ALREADY_EXIST => K::AlreadyExists,
                kio_error::IS_DIRECTORY => K::IsADirectory,
                kio_error::IS_FILE => K::NotADirectory,
                kio_error::DISK_FULL => K::StorageFull,
                kio_error::USER_CANCELED => K::Interrupted,
                _ => K::Other,
            },
        }
    }
}

impl From<AdminError> for std::io::Error {
    fn from(error: AdminError) -> Self {
        Self::new(error.io_kind(), error)
    }
}

impl From<zbus::Error> for AdminError {
    fn from(error: zbus::Error) -> Self {
        match &error {
            zbus::Error::MethodError(name, _, _) => match name.as_str().rsplit('.').next() {
                Some("ServiceUnknown" | "NameHasNoOwner" | "UnknownObject" | "UnknownMethod") => {
                    Self::Unavailable
                }
                Some("AccessDenied" | "AuthFailed" | "InteractiveAuthorizationRequired") => {
                    Self::Denied
                }
                _ => Self::Protocol(error.to_string()),
            },
            zbus::Error::InputOutput(io) if io.kind() == std::io::ErrorKind::TimedOut => {
                Self::TimedOut
            }
            zbus::Error::Address(_) => Self::Unavailable,
            _ => Self::Protocol(error.to_string()),
        }
    }
}

/// One decoded value of a KIO entry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Number(i64),
    /// `None` is Qt's null string.
    Text(Option<String>),
}

/// A KIO `UDSEntry`, as decoded from the helper.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UdsEntry {
    fields: Vec<(u32, Value)>,
}

impl UdsEntry {
    fn number(&self, id: u32) -> Option<i64> {
        self.fields.iter().find_map(|(field, value)| match value {
            Value::Number(number) if *field == id => Some(*number),
            _ => None,
        })
    }

    fn text(&self, id: u32) -> Option<&str> {
        self.fields.iter().find_map(|(field, value)| match value {
            Value::Text(Some(text)) if *field == id => Some(text.as_str()),
            _ => None,
        })
    }
}

/// Reads one entry as KIO writes it with `QDataStream` (big-endian): a
/// 32-bit field count, then per field a 32-bit id and either a 64-bit number
/// or a string (a 32-bit byte length, `0xFFFFFFFF` for none, then UTF-16).
/// Bounded by the input: nothing is allocated beyond what the bytes hold.
pub fn decode_entry(bytes: &[u8]) -> Result<UdsEntry, AdminError> {
    let mut reader = Reader { bytes, at: 0 };
    let count = reader.u32()?;
    if count > MAX_FIELDS {
        return Err(malformed("too many fields"));
    }
    let mut fields = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let id = reader.u32()?;
        let value = if id & UDS_STRING != 0 {
            Value::Text(reader.string()?)
        } else if id & UDS_NUMBER != 0 {
            Value::Number(reader.i64()?)
        } else {
            return Err(malformed("a field of unknown kind"));
        };
        fields.push((id, value));
    }
    if reader.at != bytes.len() {
        return Err(malformed("bytes after the last field"));
    }
    Ok(UdsEntry { fields })
}

fn malformed(what: &str) -> AdminError {
    AdminError::Protocol(format!("a malformed entry ({what})"))
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Result<&[u8], AdminError> {
        let end = self
            .at
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| malformed("cut short"))?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    fn u32(&mut self) -> Result<u32, AdminError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i64(&mut self) -> Result<i64, AdminError> {
        let bytes = self.take(8)?;
        let mut array = [0; 8];
        array.copy_from_slice(bytes);
        Ok(i64::from_be_bytes(array))
    }

    fn string(&mut self) -> Result<Option<String>, AdminError> {
        let len = self.u32()?;
        if len == u32::MAX {
            return Ok(None);
        }
        if len % 2 != 0 {
            return Err(malformed("a string of odd length"));
        }
        let units: Vec<u16> = self
            .take(len as usize)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect();
        String::from_utf16(&units)
            .map(Some)
            .map_err(|_| malformed("a string that is not UTF-16"))
    }
}

/// What kind of thing an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link (whatever it points to).
    Symlink,
    /// A device, socket or pipe.
    Other,
}

/// One file or folder as the helper describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEntry {
    /// The name, without any folder.
    pub name: String,
    /// What it is.
    pub kind: AdminKind,
    /// Its size in bytes.
    pub size: u64,
    /// Its permission bits (`0o7777`).
    pub permissions: u32,
    /// Seconds and nanoseconds since 1970, when known.
    pub modified: Option<(i64, u32)>,
    /// The last access, when known.
    pub accessed: Option<(i64, u32)>,
    /// The creation time, when known.
    pub created: Option<(i64, u32)>,
    /// The owner's user and group ids, when known.
    pub owner: Option<(u32, u32)>,
    /// The owner's user and group names, when the helper sends them.
    pub owner_names: (Option<String>, Option<String>),
    /// Device and inode, when known.
    pub identity: Option<(u64, u64)>,
    /// Where a symbolic link points.
    pub link_target: Option<String>,
    /// Whether it is a folder, following a symbolic link.
    pub leads_to_directory: bool,
}

impl AdminEntry {
    /// Reads the fields Rustphin uses from a KIO entry. An entry without a
    /// usable name is an error: it could not be addressed safely.
    pub fn from_uds(entry: &UdsEntry) -> Result<Self, AdminError> {
        let name = entry
            .text(field::NAME)
            .filter(|name| !name.is_empty() && !name.contains('/'))
            .ok_or_else(|| AdminError::Protocol("an entry without a usable name".into()))?
            .to_owned();
        Ok(Self::with_name(entry, name))
    }

    /// Reads a description of `path`: its name is the path's own, whatever
    /// KIO calls it (it calls `/` "/"), since it is never joined to a folder.
    pub fn described(entry: &UdsEntry, path: &Path) -> Self {
        let name = path.file_name().map_or_else(
            || "/".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        Self::with_name(entry, name)
    }

    fn with_name(entry: &UdsEntry, name: String) -> Self {
        let link_target = entry.text(field::LINK_DEST).map(str::to_owned);
        let file_type = entry.number(field::FILE_TYPE).unwrap_or(0) as u32 & 0o170_000;
        // KIO gives a link the type of what it points to; the target marks it.
        let kind = if link_target.is_some() {
            AdminKind::Symlink
        } else {
            match file_type {
                0o040_000 => AdminKind::Directory,
                0o100_000 => AdminKind::File,
                _ => AdminKind::Other,
            }
        };
        let time = |seconds: u32, nanos: u32| {
            entry.number(seconds).map(|secs| {
                let ns = entry
                    .number(nanos)
                    .and_then(|ns| u32::try_from(ns).ok())
                    .filter(|ns| *ns < 1_000_000_000)
                    .unwrap_or(0);
                (secs, ns)
            })
        };
        let id = |field: u32| entry.number(field).and_then(|id| u32::try_from(id).ok());
        Self {
            name,
            kind,
            size: entry
                .number(field::SIZE)
                .and_then(|size| u64::try_from(size).ok())
                .unwrap_or(0),
            permissions: entry.number(field::ACCESS).unwrap_or(0) as u32 & 0o7777,
            modified: time(field::MODIFICATION_TIME, field::MODIFICATION_TIME_NS),
            accessed: time(field::ACCESS_TIME, field::ACCESS_TIME_NS),
            created: time(field::CREATION_TIME, field::CREATION_TIME_NS),
            owner: id(field::LOCAL_USER_ID).zip(id(field::LOCAL_GROUP_ID)),
            owner_names: (
                entry.text(field::USER).map(str::to_owned),
                entry.text(field::GROUP).map(str::to_owned),
            ),
            identity: entry
                .number(field::DEVICE_ID)
                .and_then(|device| u64::try_from(device).ok())
                .zip(
                    entry
                        .number(field::INODE)
                        .and_then(|inode| u64::try_from(inode).ok()),
                ),
            link_target,
            leads_to_directory: file_type == 0o040_000,
        }
    }
}

/// The `file://` URL the helper is given for `path`. The helper reads URLs
/// as Qt strings, so a name that is not valid UTF-8 would reach it as a
/// different name: such paths are refused, never sent. Relative paths and
/// paths with `..` parts are refused as well.
pub fn admin_url(path: &Path) -> Result<String, AdminError> {
    let shown = || path.to_string_lossy().into_owned();
    let text = path
        .to_str()
        .ok_or_else(|| AdminError::Unaddressable(shown()))?;
    let plain = path.is_absolute()
        && !text.contains('\u{FFFD}')
        && !text.contains('\0')
        && path.components().all(|part| {
            matches!(
                part,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        });
    if !plain {
        return Err(AdminError::Unaddressable(shown()));
    }
    // The parts as Rust reads them: a `.` in the middle and doubled
    // slashes are already gone, so the helper is sent exactly this path.
    let normal: std::path::PathBuf = path.components().collect();
    Ok(format!(
        "file://{}",
        percent_encode(normal.as_os_str().as_bytes())
    ))
}

/// A signal the helper sent to this connection.
struct Signal {
    path: String,
    member: String,
    message: zbus::Message,
}

/// KDE's kio-admin helper on the system bus.
pub struct AdminHelper {
    connection: Connection,
    signals: flume::Receiver<Signal>,
    /// One operation at a time: signals are matched to the command running.
    busy: Mutex<()>,
    timeout: Duration,
}

impl AdminHelper {
    /// Connects to the system bus. Nothing is asked of the helper (so no
    /// password is asked for) until the first operation.
    pub fn system() -> Result<Self, AdminError> {
        let connection = zbus::blocking::connection::Builder::system()
            .and_then(|builder| builder.method_timeout(ADMIN_TIMEOUT).build())
            .map_err(|_| AdminError::Unavailable)?;
        Self::on(connection, ADMIN_TIMEOUT)
    }

    /// The helper where it lives: the system bus (or, for interface tests,
    /// the session bus named by [`TEST_BUS_VARIABLE`]).
    pub fn connect() -> Result<Self, AdminError> {
        if !test_bus() {
            return Self::system();
        }
        let connection = zbus::blocking::connection::Builder::session()
            .and_then(|builder| builder.method_timeout(ADMIN_TIMEOUT).build())
            .map_err(|_| AdminError::Unavailable)?;
        Self::on(connection, ADMIN_TIMEOUT)
    }

    /// Uses the helper reachable over `connection`; `timeout` bounds each
    /// wait for a signal. Fails only when the reader thread cannot start.
    pub fn on(connection: Connection, timeout: Duration) -> Result<Self, AdminError> {
        // kio-admin addresses its signals to the caller, which needs no
        // subscription; should a version broadcast them instead, this
        // subscription still brings them. A private connection has no bus
        // to ask, which is fine.
        if let Ok(builder) = zbus::MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(SERVICE)
            && let Ok(bus) = zbus::blocking::fdo::DBusProxy::new(&connection)
        {
            let _ = bus.add_match_rule(builder.build());
        }
        let (send, signals) = flume::unbounded();
        let incoming = MessageIterator::from(connection.clone());
        // One reader for the connection's lifetime: it forwards the
        // helper's signals and ends when the connection closes.
        let spawned = thread::Builder::new()
            .name("rustphin-admin".into())
            .spawn(move || {
                for message in incoming.flatten() {
                    if message.message_type() != Type::Signal {
                        continue;
                    }
                    let header = message.header();
                    let (Some(path), Some(member)) = (header.path(), header.member()) else {
                        continue;
                    };
                    let signal = Signal {
                        path: path.to_string(),
                        member: member.to_string(),
                        message: message.clone(),
                    };
                    if send.send(signal).is_err() {
                        break;
                    }
                }
            });
        spawned.map_err(|_| AdminError::Unavailable)?;
        Ok(Self {
            connection,
            signals,
            busy: Mutex::new(()),
            timeout,
        })
    }

    /// Lists a folder.
    pub fn list_dir(&self, path: &Path) -> Result<Vec<AdminEntry>, AdminError> {
        let mut entries = Vec::new();
        self.run(
            "listDir",
            "ListDirCommand",
            &(admin_url(path)?,),
            |signal| {
                if signal.member == "entries" {
                    let list: Vec<(Vec<u8>,)> = signal.message.body().deserialize()?;
                    for (bytes,) in list {
                        let entry = AdminEntry::from_uds(&decode_entry(&bytes)?)?;
                        if entry.name != "." && entry.name != ".." {
                            entries.push(entry);
                        }
                    }
                }
                Ok(())
            },
        )?;
        Ok(entries)
    }

    /// Describes one file or folder.
    pub fn stat(&self, path: &Path) -> Result<AdminEntry, AdminError> {
        let mut found = None;
        self.run("stat", "StatCommand", &(admin_url(path)?,), |signal| {
            if signal.member == "statEntry" {
                let (bytes,): (Vec<u8>,) = signal.message.body().deserialize()?;
                found = Some(AdminEntry::described(&decode_entry(&bytes)?, path));
            }
            Ok(())
        })?;
        found.ok_or_else(|| AdminError::Protocol("no description came back".into()))
    }

    /// Reads a whole file, at most [`MAX_TRANSFER`] bytes.
    pub fn read(&self, path: &Path) -> Result<Vec<u8>, AdminError> {
        let mut data = Vec::new();
        self.run("get", "GetCommand", &(admin_url(path)?,), |signal| {
            if signal.member == "data" {
                let chunk: Vec<u8> = signal.message.body().deserialize()?;
                if data.len() + chunk.len() > MAX_TRANSFER {
                    return Err(AdminError::Protocol("the file is too large".into()));
                }
                data.extend_from_slice(&chunk);
            }
            Ok(())
        })?;
        Ok(data)
    }

    /// Writes a new file with `data`, never replacing one that exists.
    pub fn write_new(&self, path: &Path, data: &[u8], permissions: u32) -> Result<(), AdminError> {
        if data.len() > MAX_TRANSFER {
            return Err(AdminError::Protocol("the file is too large".into()));
        }
        let mode = i32::try_from(permissions & 0o7777).unwrap_or(-1);
        let mut chunks = data.chunks(CHUNK);
        let mut finished = data.is_empty();
        let connection = self.connection.clone();
        self.run(
            "put",
            "PutCommand",
            &(admin_url(path)?, mode, 0i32),
            |signal| {
                if signal.member == "dataRequest" {
                    // An empty piece ends the file.
                    let piece = if finished {
                        &[][..]
                    } else {
                        chunks.next().unwrap_or_else(|| {
                            finished = true;
                            &[]
                        })
                    };
                    connection.call_method(
                        Some(SERVICE),
                        signal.path.as_str(),
                        Some("org.kde.kio.admin.PutCommand"),
                        "data",
                        &(piece,),
                    )?;
                }
                Ok(())
            },
        )
    }

    /// Makes a folder.
    pub fn create_dir(&self, path: &Path, permissions: u32) -> Result<(), AdminError> {
        let mode = i32::try_from(permissions & 0o7777).unwrap_or(-1);
        self.run("mkdir", "MkdirCommand", &(admin_url(path)?, mode), |_| {
            Ok(())
        })
    }

    /// Renames, replacing the destination only when `overwrite` is set.
    pub fn rename(&self, from: &Path, to: &Path, overwrite: bool) -> Result<(), AdminError> {
        let flags = if overwrite { KIO_OVERWRITE } else { 0 };
        self.run(
            "rename",
            "RenameCommand",
            &(admin_url(from)?, admin_url(to)?, flags),
            |_| Ok(()),
        )
    }

    /// Copies one file, never replacing the destination.
    pub fn copy_file(&self, from: &Path, to: &Path) -> Result<(), AdminError> {
        self.run(
            "copy",
            "CopyCommand",
            &(admin_url(from)?, admin_url(to)?, -1i32, 0i32),
            |_| Ok(()),
        )
    }

    /// Deletes for good. KIO deletes a folder with everything in it, so
    /// callers check a folder is empty first.
    pub fn delete(&self, path: &Path) -> Result<(), AdminError> {
        self.run("del", "DelCommand", &(admin_url(path)?,), |_| Ok(()))
    }

    /// Sets the permission bits.
    pub fn set_permissions(&self, path: &Path, permissions: u32) -> Result<(), AdminError> {
        let mode = i32::try_from(permissions & 0o7777).unwrap_or(0);
        self.run("chmod", "ChmodCommand", &(admin_url(path)?, mode), |_| {
            Ok(())
        })
    }

    /// Sets the owning user and group, by name.
    pub fn set_owner(&self, path: &Path, user: &str, group: &str) -> Result<(), AdminError> {
        self.run(
            "chown",
            "ChownCommand",
            &(admin_url(path)?, user, group),
            |_| Ok(()),
        )
    }

    /// Starts `method` and feeds the signals of its command object to
    /// `each` until the helper sends `result`.
    fn run<B>(
        &self,
        method: &str,
        command: &str,
        arguments: &B,
        mut each: impl FnMut(&Signal) -> Result<(), AdminError>,
    ) -> Result<(), AdminError>
    where
        B: zbus::export::serde::Serialize + zbus::zvariant::DynamicType,
    {
        let _one_at_a_time = self.busy.lock().unwrap_or_else(PoisonError::into_inner);
        // Signals of earlier commands are of no use any more.
        self.signals.drain();
        let reply = self.connection.call_method(
            Some(SERVICE),
            ROOT_PATH,
            Some(INTERFACE),
            method,
            arguments,
        )?;
        let path: OwnedObjectPath = reply.body().deserialize()?;
        if path.as_str().len() <= 1 {
            // The helper answers a refused authorisation with no object.
            return Err(AdminError::Denied);
        }
        let interface = format!("org.kde.kio.admin.{command}");
        self.connection.call_method(
            Some(SERVICE),
            path.as_str(),
            Some(interface.as_str()),
            "start",
            &(),
        )?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let signal = self
                .signals
                .recv_timeout(left)
                .map_err(|error| match error {
                    flume::RecvTimeoutError::Timeout => AdminError::TimedOut,
                    flume::RecvTimeoutError::Disconnected => AdminError::Unavailable,
                })?;
            if signal.path != path.as_str() {
                continue;
            }
            if signal.member == "result" {
                let (code, message): (i32, String) = signal.message.body().deserialize()?;
                return if code == 0 {
                    Ok(())
                } else {
                    Err(AdminError::Failed { code, message })
                };
            }
            each(&signal)?;
        }
    }
}

/// The core's view of the filesystem through the helper (D-028): every
/// read and change Rustphin makes while acting as administrator goes
/// through here, and through the operation engine above it.
///
/// What the helper cannot do stays unsupported rather than pretended:
/// watching folders, symbolic links, directory locks and syncing.
pub struct AdminFs {
    helper: std::sync::Arc<AdminHelper>,
    /// Entries from the latest listing of each folder, so describing every
    /// entry of a listing costs no further calls.
    known: Mutex<std::collections::HashMap<std::path::PathBuf, AdminEntry>>,
}

impl AdminFs {
    /// Works through `helper`.
    pub fn new(helper: std::sync::Arc<AdminHelper>) -> Self {
        Self {
            helper,
            known: Mutex::default(),
        }
    }

    fn known(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::HashMap<std::path::PathBuf, AdminEntry>> {
        self.known.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The entry at `path`: from the latest listing when there is one,
    /// otherwise asked of the helper.
    fn entry(&self, path: &Path) -> std::io::Result<AdminEntry> {
        if let Some(entry) = self.known().get(path) {
            return Ok(entry.clone());
        }
        Ok(self.helper.stat(path)?)
    }

    /// Forgets what is known about `path` after a change to it.
    fn forget(&self, path: &Path) {
        self.known().remove(path);
    }
}

fn entry_kind(entry: &AdminEntry) -> rustphin_core::EntryKind {
    use rustphin_core::EntryKind as K;
    match entry.kind {
        AdminKind::File => K::File,
        AdminKind::Directory => K::Directory,
        AdminKind::Symlink => K::Symlink,
        AdminKind::Other => K::Other,
    }
}

impl rustphin_core::Fs for AdminFs {
    fn read_dir(
        &self,
        path: &Path,
    ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<rustphin_core::RawEntry>> + '_>>
    {
        let entries = self.helper.list_dir(path)?;
        let mut known = self.known();
        known.retain(|known_path, _| known_path.parent() != Some(path));
        let raw: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                let raw = rustphin_core::RawEntry {
                    name: entry.name.as_bytes().to_vec(),
                    kind: entry_kind(&entry),
                };
                known.insert(path.join(&entry.name), entry);
                Ok(raw)
            })
            .collect();
        Ok(Box::new(raw.into_iter()))
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.entry(path).is_ok_and(|entry| entry.leads_to_directory)
    }

    fn metadata(&self, path: &Path) -> std::io::Result<rustphin_core::Metadata> {
        let entry = self.entry(path)?;
        Ok(rustphin_core::Metadata {
            size: entry.size,
            modified: entry.modified.map_or(0, |(seconds, _)| seconds),
        })
    }

    /// The helper cannot watch: folders re-read after Rustphin's own
    /// changes and on F5 (the mode's documented limit).
    fn watch(
        &self,
        _path: &Path,
        _on_change: Box<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<rustphin_core::WatchGuard> {
        Ok(Box::new(()))
    }

    fn info(&self, path: &Path) -> std::io::Result<rustphin_core::FileInfo> {
        let entry = self.helper.stat(path)?;
        let (device, inode) = entry.identity.unwrap_or((0, 0));
        Ok(rustphin_core::FileInfo {
            kind: entry_kind(&entry),
            size: entry.size,
            modified_ns: entry.modified.map_or(0, |(seconds, nanos)| {
                i128::from(seconds) * 1_000_000_000 + i128::from(nanos)
            }),
            device,
            inode,
            mode: entry.permissions,
        })
    }

    /// KIO sets the mode it is given exactly, where `mkdir(2)` filters it
    /// through the umask: the user's umask is applied here, so a new folder
    /// gets the mode it would get without the helper, never `0o777`.
    fn create_dir(&self, path: &Path, mode: u32) -> std::io::Result<()> {
        self.forget(path);
        Ok(self.helper.create_dir(path, mode & !process_umask())?)
    }

    fn create_file(&self, path: &Path) -> std::io::Result<()> {
        self.forget(path);
        Ok(self.helper.write_new(path, &[], 0o644)?)
    }

    /// KIO copies the content, permission bits and modification time as
    /// root; it reports no progress until the end and cannot be synced.
    fn copy_file(
        &self,
        from: &Path,
        to: &Path,
        progress: &mut dyn FnMut(u64) -> std::ops::ControlFlow<()>,
    ) -> std::io::Result<()> {
        let size = self.entry(from)?.size;
        self.forget(to);
        self.helper.copy_file(from, to)?;
        let _ = progress(size);
        Ok(())
    }

    fn read_link(&self, path: &Path) -> std::io::Result<std::path::PathBuf> {
        self.entry(path)?
            .link_target
            .map(std::path::PathBuf::from)
            .ok_or_else(|| std::io::ErrorKind::InvalidInput.into())
    }

    fn rename_new(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.forget(from);
        self.forget(to);
        self.helper
            .rename(from, to, false)
            .map_err(|error| match error {
                // KIO's file worker reports a rename across filesystems as an
                // unsupported action; the engine then copies and deletes.
                AdminError::Failed {
                    code: kio_error::UNSUPPORTED_ACTION,
                    ..
                } => std::io::ErrorKind::CrossesDevices.into(),
                other => other.into(),
            })
    }

    fn rename_over(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.forget(from);
        self.forget(to);
        Ok(self.helper.rename(from, to, true)?)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        if self.helper.stat(path)?.kind == AdminKind::Directory {
            return Err(std::io::ErrorKind::IsADirectory.into());
        }
        self.forget(path);
        Ok(self.helper.delete(path)?)
    }

    /// KIO deletes a folder with everything in it; this removes only an
    /// empty one, as the engine expects.
    fn remove_dir(&self, path: &Path) -> std::io::Result<()> {
        if self.helper.stat(path)?.kind != AdminKind::Directory {
            return Err(std::io::ErrorKind::NotADirectory.into());
        }
        if !self.helper.list_dir(path)?.is_empty() {
            return Err(std::io::ErrorKind::DirectoryNotEmpty.into());
        }
        self.forget(path);
        Ok(self.helper.delete(path)?)
    }

    /// Only the permission bits: the helper cannot set times.
    fn set_attributes(&self, path: &Path, info: &rustphin_core::FileInfo) -> std::io::Result<()> {
        self.forget(path);
        Ok(self.helper.set_permissions(path, info.mode)?)
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        Ok(self.helper.read(path)?)
    }

    fn read_bounded(&self, path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
        let entry = self.helper.stat(path)?;
        if entry.kind != AdminKind::File {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        if entry.size > limit as u64 {
            return Err(std::io::ErrorKind::FileTooLarge.into());
        }
        let data = self.helper.read(path)?;
        if data.len() > limit {
            return Err(std::io::ErrorKind::FileTooLarge.into());
        }
        Ok(data)
    }

    fn write_new(&self, path: &Path, content: &[u8]) -> std::io::Result<()> {
        self.forget(path);
        Ok(self.helper.write_new(path, content, 0o600)?)
    }

    fn set_mode(&self, path: &Path, mode: u32) -> std::io::Result<()> {
        self.forget(path);
        Ok(self.helper.set_permissions(path, mode)?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        },
    };

    use zbus::{
        blocking::connection::Builder, interface, object_server::SignalEmitter,
        zvariant::OwnedObjectPath,
    };

    use super::*;

    /// Entries serialised by KIO 6.31 itself, the way kio-admin sends them
    /// (a throwaway C++ program built against the installed KIO wrote them).
    const KIO_FILE: &str = "00000007010000060000000a006600730074006100620200000100000000000004d20200000d00000000000080000200000900000000000001a40600000a00000000671db4800200001e00000000000000000200001f0000000000000000";
    const KIO_DIR: &str = "00000003010000060000000e00730079007300740065006d00640200000d00000000000040000200000900000000000001ed";
    const KIO_LINK: &str = "000000040100000600000010007200e900730075006d00e9002027130100000e00000016002f006f00700074002f0074006100720067006500740200000d000000000000800002000001ffffffffffffffff";
    const KIO_NULL_NAME: &str = "0000000101000006ffffffff";

    fn bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
            .collect()
    }

    fn entry(hex: &str) -> AdminEntry {
        AdminEntry::from_uds(&decode_entry(&bytes(hex)).unwrap()).unwrap()
    }

    #[test]
    fn entries_written_by_kio_are_read_as_kio_meant_them() {
        let file = entry(KIO_FILE);
        assert_eq!(file.name, "fstab");
        assert_eq!(file.kind, AdminKind::File);
        assert_eq!(file.size, 1234);
        assert_eq!(file.permissions, 0o644);
        assert_eq!(file.modified, Some((1_730_000_000, 0)));
        assert_eq!(file.owner, Some((0, 0)));

        let dir = entry(KIO_DIR);
        assert_eq!(
            (dir.name.as_str(), dir.kind),
            ("systemd", AdminKind::Directory)
        );
        assert_eq!(dir.permissions, 0o755);

        // A link has the type of its target; its destination marks it.
        let link = entry(KIO_LINK);
        assert_eq!(link.name, "résumé ✓");
        assert_eq!(link.kind, AdminKind::Symlink);
        assert_eq!(link.link_target.as_deref(), Some("/opt/target"));
        // A negative size is no size.
        assert_eq!(link.size, 0);
    }

    #[test]
    fn an_entry_without_a_usable_name_is_refused() {
        let nameless = decode_entry(&bytes(KIO_NULL_NAME)).unwrap();
        assert!(AdminEntry::from_uds(&nameless).is_err());
        assert!(AdminEntry::from_uds(&decode_entry(&bytes("00000000")).unwrap()).is_err());
    }

    #[test]
    fn malformed_entries_are_errors_not_guesses() {
        let file = bytes(KIO_FILE);
        // Cut anywhere, the entry is refused.
        for len in 0..file.len() {
            assert!(decode_entry(&file[..len]).is_err(), "cut at {len}");
        }
        let mut extra = file.clone();
        extra.push(0);
        assert!(decode_entry(&extra).is_err());
        // A count far larger than the bytes can hold.
        assert!(decode_entry(&bytes("ffffffff")).is_err());
        // A field that is neither a number nor a string.
        assert!(decode_entry(&bytes("0000000100000006")).is_err());
        // A string of odd length, and one that is not UTF-16.
        assert!(decode_entry(&bytes("000000010100000600000001aa")).is_err());
        assert!(decode_entry(&bytes("000000010100000600000002d800")).is_err());
    }

    #[test]
    fn only_plain_absolute_paths_are_sent_to_the_helper() {
        assert_eq!(
            admin_url(Path::new("/etc/a b#?%")).unwrap(),
            "file:///etc/a%20b%23%3F%25"
        );
        assert_eq!(
            admin_url(Path::new("/opt/résumé")).unwrap(),
            "file:///opt/r%C3%A9sum%C3%A9"
        );
        // Spelled differently, the same path is sent the one way.
        assert_eq!(admin_url(Path::new("//etc/./x/")).unwrap(), "file:///etc/x");
        let not_utf8 = PathBuf::from(std::ffi::OsStr::from_bytes(b"/etc/bad\xff"));
        for refused in [
            not_utf8,
            PathBuf::from("etc/relative"),
            PathBuf::from("/etc/../shadow"),
            PathBuf::from("/etc/repl\u{FFFD}aced"),
        ] {
            assert!(
                matches!(admin_url(&refused), Err(AdminError::Unaddressable(_))),
                "{refused:?}"
            );
        }
    }

    #[test]
    fn kio_errors_become_the_matching_standard_kinds() {
        use std::io::ErrorKind as K;
        let failed = |code| AdminError::Failed {
            code,
            message: String::new(),
        };
        assert_eq!(failed(111).io_kind(), K::NotFound);
        assert_eq!(failed(112).io_kind(), K::AlreadyExists);
        assert_eq!(failed(115).io_kind(), K::PermissionDenied);
        assert_eq!(failed(161).io_kind(), K::StorageFull);
        assert_eq!(failed(999).io_kind(), K::Other);
        assert_eq!(AdminError::Denied.io_kind(), K::PermissionDenied);
    }

    /// A pretend kio-admin helper: answers like the real one, with a
    /// listing of the KIO samples, and remembers what was written.
    struct FakeHelper {
        refuse: bool,
        result: i32,
        counter: Arc<AtomicU32>,
        written: Arc<Mutex<Vec<Vec<u8>>>>,
        asked: Arc<Mutex<Vec<String>>>,
        silent: bool,
    }

    impl FakeHelper {
        async fn command<T: zbus::object_server::Interface>(
            &self,
            server: &zbus::ObjectServer,
            kind: &str,
            command: T,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            if self.refuse {
                return Err(zbus::fdo::Error::AccessDenied("not authorised".into()));
            }
            let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
            let path = format!("/org/kde/kio/admin/{kind}/{n}");
            server.at(path.as_str(), command).await?;
            Ok(OwnedObjectPath::try_from(path).unwrap())
        }
    }

    #[interface(name = "org.kde.kio.admin")]
    impl FakeHelper {
        #[zbus(name = "listDir")]
        async fn list_dir(
            &self,
            url: String,
            #[zbus(object_server)] server: &zbus::ObjectServer,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.asked.lock().unwrap().push(format!("listDir {url}"));
            let command = Simple {
                result: self.result,
                silent: self.silent,
            };
            self.command(server, "listDir", ListDir { command }).await
        }

        #[zbus(name = "stat")]
        async fn stat(
            &self,
            url: String,
            #[zbus(object_server)] server: &zbus::ObjectServer,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.asked.lock().unwrap().push(format!("stat {url}"));
            self.command(server, "stat", Stat).await
        }

        #[zbus(name = "put")]
        async fn put(
            &self,
            url: String,
            permissions: i32,
            flags: i32,
            #[zbus(object_server)] server: &zbus::ObjectServer,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.asked
                .lock()
                .unwrap()
                .push(format!("put {url} {permissions:o} {flags}"));
            let put = Put {
                written: Arc::clone(&self.written),
            };
            self.command(server, "put", put).await
        }

        #[zbus(name = "rename")]
        async fn rename(
            &self,
            from: String,
            to: String,
            flags: i32,
            #[zbus(object_server)] server: &zbus::ObjectServer,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.asked
                .lock()
                .unwrap()
                .push(format!("rename {from} {to} {flags}"));
            let command = Simple {
                result: self.result,
                silent: false,
            };
            self.command(server, "rename", Rename { command }).await
        }

        #[zbus(name = "mkdir")]
        async fn mkdir(
            &self,
            url: String,
            permissions: i32,
            #[zbus(object_server)] server: &zbus::ObjectServer,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.asked
                .lock()
                .unwrap()
                .push(format!("mkdir {url} {permissions:o}"));
            self.command(server, "mkdir", Mkdir).await
        }
    }

    struct Mkdir;

    #[interface(name = "org.kde.kio.admin.MkdirCommand")]
    impl Mkdir {
        #[zbus(name = "start")]
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            Self::result(&emitter, 0, String::new()).await.unwrap();
        }

        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    /// The part every command shares: `start` ends with `result`.
    struct Simple {
        result: i32,
        silent: bool,
    }

    /// Where a late answer to an earlier, abandoned command comes from.
    const STRAY: &str = "/org/kde/kio/admin/listDir/999";

    struct ListDir {
        command: Simple,
    }

    #[interface(name = "org.kde.kio.admin.ListDirCommand")]
    impl ListDir {
        #[zbus(name = "start")]
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            if self.command.silent {
                return;
            }
            // First, a late failure from a command given up on earlier: it
            // must not be taken for this one's result.
            let stray = SignalEmitter::new(emitter.connection(), STRAY).unwrap();
            Self::result(&stray, kio_error::ACCESS_DENIED, "stray".into())
                .await
                .unwrap();
            let list = vec![(bytes(KIO_FILE),), (bytes(KIO_DIR),)];
            Self::entries(&emitter, list).await.unwrap();
            let message = if self.command.result == 0 {
                String::new()
            } else {
                "The file or folder /etc/missing does not exist.".into()
            };
            Self::result(&emitter, self.command.result, message)
                .await
                .unwrap();
        }

        #[zbus(signal, name = "entries")]
        async fn entries(emitter: &SignalEmitter<'_>, list: Vec<(Vec<u8>,)>) -> zbus::Result<()>;

        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    struct Stat;

    #[interface(name = "org.kde.kio.admin.StatCommand")]
    impl Stat {
        #[zbus(name = "start")]
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            Self::stat_entry(&emitter, (bytes(KIO_DIR),)).await.unwrap();
            Self::result(&emitter, 0, String::new()).await.unwrap();
        }

        #[zbus(signal, name = "statEntry")]
        async fn stat_entry(emitter: &SignalEmitter<'_>, entry: (Vec<u8>,)) -> zbus::Result<()>;

        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    struct Put {
        written: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    #[interface(name = "org.kde.kio.admin.PutCommand")]
    impl Put {
        #[zbus(name = "start")]
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            Self::data_request(&emitter).await.unwrap();
        }

        /// KIO's put protocol: each piece is asked for; an empty one ends.
        #[zbus(name = "data")]
        async fn data(&self, piece: Vec<u8>, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            let done = piece.is_empty();
            self.written.lock().unwrap().push(piece);
            if done {
                Self::result(&emitter, 0, String::new()).await.unwrap();
            } else {
                Self::data_request(&emitter).await.unwrap();
            }
        }

        #[zbus(signal, name = "dataRequest")]
        async fn data_request(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    struct Rename {
        command: Simple,
    }

    #[interface(name = "org.kde.kio.admin.RenameCommand")]
    impl Rename {
        #[zbus(name = "start")]
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            let message = if self.command.result == 0 {
                String::new()
            } else {
                "The file or folder /etc/b already exists.".into()
            };
            Self::result(&emitter, self.command.result, message)
                .await
                .unwrap();
        }

        #[zbus(signal, name = "result")]
        async fn result(
            emitter: &SignalEmitter<'_>,
            error: i32,
            message: String,
        ) -> zbus::Result<()>;
    }

    struct Setup {
        helper: AdminHelper,
        written: Arc<Mutex<Vec<Vec<u8>>>>,
        asked: Arc<Mutex<Vec<String>>>,
    }

    fn helper(refuse: bool, result: i32, silent: bool, timeout: Duration) -> Setup {
        let written = Arc::new(Mutex::new(Vec::new()));
        let asked = Arc::new(Mutex::new(Vec::new()));
        let fake = FakeHelper {
            refuse,
            result,
            counter: Arc::default(),
            written: Arc::clone(&written),
            asked: Arc::clone(&asked),
            silent,
        };
        let (ours, theirs) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let builder = Builder::async_io_unix_stream(theirs)
            .server(guid)
            .unwrap()
            .p2p()
            .serve_at(ROOT_PATH, fake)
            .unwrap();
        // Both ends shake hands at the same time.
        let service = thread::spawn(move || builder.build().unwrap());
        let client = Builder::async_io_unix_stream(ours).p2p().build().unwrap();
        // Keep the service end alive for the rest of the test process.
        std::mem::forget(service.join().unwrap());
        Setup {
            helper: AdminHelper::on(client, timeout).unwrap(),
            written,
            asked,
        }
    }

    fn working() -> Setup {
        helper(false, 0, false, Duration::from_secs(10))
    }

    #[test]
    fn a_listing_arrives_as_entries_then_a_result() {
        let setup = working();
        let entries = setup.helper.list_dir(Path::new("/etc")).unwrap();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["fstab", "systemd"]);
        assert_eq!(*setup.asked.lock().unwrap(), ["listDir file:///etc"]);
        // A second command does not see the first one's signals.
        let again = setup.helper.list_dir(Path::new("/etc")).unwrap();
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn a_description_arrives_as_one_entry() {
        let setup = working();
        let found = setup.helper.stat(Path::new("/etc/systemd")).unwrap();
        assert_eq!(found.kind, AdminKind::Directory);
        // A description is named after the path asked about: KIO calls the
        // root "/", which no listed name may be.
        assert_eq!(found.name, "systemd");
        assert_eq!(setup.helper.stat(Path::new("/")).unwrap().name, "/");
    }

    #[test]
    fn a_file_is_written_in_pieces_and_ended_with_an_empty_one() {
        let setup = working();
        let data = vec![7u8; CHUNK + 10];
        setup
            .helper
            .write_new(Path::new("/etc/new"), &data, 0o640)
            .unwrap();
        let written = setup.written.lock().unwrap();
        let sizes: Vec<_> = written.iter().map(Vec::len).collect();
        assert_eq!(sizes, [CHUNK, 10, 0]);
        // Never allowed to replace an existing file.
        assert_eq!(*setup.asked.lock().unwrap(), ["put file:///etc/new 640 0"]);

        let empty = working();
        empty
            .helper
            .write_new(Path::new("/etc/empty"), &[], 0o600)
            .unwrap();
        assert_eq!(*empty.written.lock().unwrap(), [Vec::<u8>::new()]);
    }

    #[test]
    fn a_rename_replaces_only_when_asked() {
        let setup = working();
        setup
            .helper
            .rename(Path::new("/etc/a"), Path::new("/etc/b"), false)
            .unwrap();
        setup
            .helper
            .rename(Path::new("/etc/a"), Path::new("/etc/b"), true)
            .unwrap();
        assert_eq!(
            *setup.asked.lock().unwrap(),
            [
                "rename file:///etc/a file:///etc/b 0",
                "rename file:///etc/a file:///etc/b 4"
            ]
        );
    }

    #[test]
    fn a_refused_password_is_reported_as_denied() {
        let setup = helper(true, 0, false, Duration::from_secs(10));
        assert_eq!(
            setup.helper.list_dir(Path::new("/root")),
            Err(AdminError::Denied)
        );
    }

    #[test]
    fn a_failed_operation_carries_kios_reason() {
        let setup = helper(
            false,
            kio_error::DOES_NOT_EXIST,
            false,
            Duration::from_secs(10),
        );
        let error = setup
            .helper
            .list_dir(Path::new("/etc/missing"))
            .unwrap_err();
        assert_eq!(error.io_kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            error.to_string(),
            "The file or folder /etc/missing does not exist."
        );
        let exists = helper(
            false,
            kio_error::FILE_ALREADY_EXIST,
            false,
            Duration::from_secs(10),
        );
        let error = exists
            .helper
            .rename(Path::new("/etc/a"), Path::new("/etc/b"), false)
            .unwrap_err();
        assert_eq!(error.io_kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn a_helper_that_never_answers_times_out() {
        let setup = helper(false, 0, true, Duration::from_millis(200));
        assert_eq!(
            setup.helper.list_dir(Path::new("/etc")),
            Err(AdminError::TimedOut)
        );
    }

    #[test]
    fn unsafe_names_never_reach_the_helper() {
        let setup = working();
        let not_utf8 = PathBuf::from(std::ffi::OsStr::from_bytes(b"/etc/bad\xff"));
        assert!(matches!(
            setup.helper.list_dir(&not_utf8),
            Err(AdminError::Unaddressable(_))
        ));
        assert!(setup.asked.lock().unwrap().is_empty());
    }

    fn admin_fs(setup: &Setup) -> AdminFs {
        // The fake's listings and descriptions come from one helper object
        // shared with the test, through an `Arc`.
        AdminFs::new(std::sync::Arc::new(AdminHelper {
            connection: setup.helper.connection.clone(),
            signals: setup.helper.signals.clone(),
            busy: Mutex::new(()),
            timeout: setup.helper.timeout,
        }))
    }

    #[test]
    fn a_listing_describes_its_entries_without_further_calls() {
        use rustphin_core::{EntryKind, Fs as _};
        let setup = working();
        let fs = admin_fs(&setup);
        let listed: Vec<_> = fs
            .read_dir(Path::new("/etc"))
            .unwrap()
            .map(|entry| entry.unwrap())
            .map(|entry| (String::from_utf8(entry.name).unwrap(), entry.kind))
            .collect();
        assert_eq!(
            listed,
            [
                ("fstab".to_owned(), EntryKind::File),
                ("systemd".to_owned(), EntryKind::Directory)
            ]
        );
        let fstab = fs.metadata(Path::new("/etc/fstab")).unwrap();
        assert_eq!((fstab.size, fstab.modified), (1234, 1_730_000_000));
        assert!(fs.is_dir(Path::new("/etc/systemd")));
        assert!(!fs.is_dir(Path::new("/etc/fstab")));
        assert_eq!(*setup.asked.lock().unwrap(), ["listDir file:///etc"]);
    }

    #[test]
    fn a_new_folder_gets_the_mode_mkdir_would_give_it_not_a_world_writable_one() {
        use std::os::unix::fs::PermissionsExt as _;

        use rustphin_core::{Fs as _, RealFs};
        let setup = working();
        let fs = admin_fs(&setup);
        fs.create_dir(Path::new("/etc/new"), 0o777).unwrap();
        // What the same request gives without the helper, umask applied.
        let scratch = tempfile::tempdir().unwrap();
        let plain = scratch.path().join("new");
        RealFs.create_dir(&plain, 0o777).unwrap();
        let expected = std::fs::metadata(&plain).unwrap().permissions().mode() & 0o7777;
        assert_eq!(
            *setup.asked.lock().unwrap(),
            [format!("mkdir file:///etc/new {expected:o}")]
        );
    }

    #[test]
    fn a_folder_with_something_in_it_is_never_handed_to_kios_delete() {
        use rustphin_core::Fs as _;
        let setup = working();
        let fs = admin_fs(&setup);
        let error = fs.remove_dir(Path::new("/etc/systemd")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::DirectoryNotEmpty);
        let asked = setup.asked.lock().unwrap();
        assert!(
            asked.iter().all(|call| !call.starts_with("del")),
            "{asked:?}"
        );
    }

    #[test]
    fn a_rename_across_filesystems_asks_the_engine_to_copy_instead() {
        use rustphin_core::Fs as _;
        let setup = helper(
            false,
            kio_error::UNSUPPORTED_ACTION,
            false,
            Duration::from_secs(10),
        );
        let fs = admin_fs(&setup);
        let error = fs
            .rename_new(Path::new("/etc/a"), Path::new("/mnt/b"))
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::CrossesDevices);
    }
}
