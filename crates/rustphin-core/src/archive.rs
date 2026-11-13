//! Archive formats for the operation engine (W-111, D-032): which formats
//! there are, which names an entry may have, and reading and writing them.
//!
//! Archives are untrusted. Every entry name is checked by [`entry_path`]
//! before anything is written, links must stay inside the destination
//! ([`link_stays_inside`]), and only files, folders and such links are ever
//! made. The engine (`ops::archive`) does the writing, through `Fs`.

use std::{
    ffi::OsStr,
    io::{self, Read, Seek, Write},
    ops::ControlFlow,
    os::unix::ffi::OsStrExt as _,
    path::{Component, Path, PathBuf},
};

use jiff::tz::TimeZone;

use crate::fs::ReadSeek;

/// The archive formats Rustphin reads, and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// ZIP.
    Zip,
    /// TAR, uncompressed.
    Tar,
    /// TAR with gzip.
    TarGz,
    /// TAR with bzip2.
    TarBz2,
    /// TAR with xz.
    TarXz,
    /// TAR with Zstandard.
    TarZst,
    /// 7-Zip.
    SevenZ,
}

/// How hard to compress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    /// Quick, larger.
    Fast,
    /// The formats' usual balance.
    #[default]
    Normal,
    /// Slow, smallest.
    Smallest,
}

/// Name endings for each format, longest first, matched without case.
const SUFFIXES: [(&str, Format); 13] = [
    (".tar.gz", Format::TarGz),
    (".tar.bz2", Format::TarBz2),
    (".tar.xz", Format::TarXz),
    (".tar.zst", Format::TarZst),
    (".tgz", Format::TarGz),
    (".tbz2", Format::TarBz2),
    (".tbz", Format::TarBz2),
    (".txz", Format::TarXz),
    (".tzst", Format::TarZst),
    (".tar", Format::Tar),
    (".zip", Format::Zip),
    (".7z", Format::SevenZ),
    (".jar", Format::Zip),
];

impl Format {
    /// The usual ending of a new archive's name: `zip`, `tar.gz`…
    pub fn extension(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarBz2 => "tar.bz2",
            Self::TarXz => "tar.xz",
            Self::TarZst => "tar.zst",
            Self::SevenZ => "7z",
        }
    }

    /// The format a name's ending suggests.
    pub fn from_name(name: &[u8]) -> Option<Self> {
        suffix(name).map(|(_, format)| format)
    }

    /// Whether Rustphin can make archives of this format.
    pub fn writable(self) -> bool {
        !matches!(self, Self::TarBz2 | Self::Tar)
    }
}

/// The format and length of the archive ending of `name`, if any.
fn suffix(name: &[u8]) -> Option<(usize, Format)> {
    SUFFIXES.iter().find_map(|(ending, format)| {
        let ending = ending.as_bytes();
        (name.len() > ending.len()
            && name[name.len() - ending.len()..].eq_ignore_ascii_case(ending))
        .then_some((ending.len(), *format))
    })
}

/// `name` without its archive ending: the folder name Extract makes.
/// A name with no known ending is kept whole.
pub fn stem(name: &[u8]) -> &[u8] {
    match suffix(name) {
        Some((length, _)) => &name[..name.len() - length],
        None => name,
    }
}

/// The format of the archive called `name` that starts with `head`, judged
/// by its content: a ZIP or 7Z is recognised whatever its name; a
/// compressed stream counts as a TAR when its name says so. `None` when
/// Rustphin cannot open it itself (another format, or a lone compressed
/// file).
pub fn detect(name: &[u8], head: &[u8]) -> Option<Format> {
    let starts = |magic: &[u8]| head.starts_with(magic);
    if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") {
        return Some(Format::Zip);
    }
    if starts(b"7z\xBC\xAF\x27\x1C") {
        return Some(Format::SevenZ);
    }
    let named = Format::from_name(name);
    let compressed = if starts(&[0x1F, 0x8B]) {
        Some(Format::TarGz)
    } else if starts(b"BZh") {
        Some(Format::TarBz2)
    } else if starts(&[0xFD, b'7', b'z', b'X', b'Z', 0]) {
        Some(Format::TarXz)
    } else if starts(&[0x28, 0xB5, 0x2F, 0xFD]) {
        Some(Format::TarZst)
    } else {
        None
    };
    match compressed {
        Some(format) => (named == Some(format)).then_some(format),
        // Old TAR files have no "ustar" mark; their name says what they are.
        None => (named == Some(Format::Tar) || head.len() > 262 && &head[257..262] == b"ustar")
            .then_some(Format::Tar),
    }
}

/// Where an entry called `raw` goes, relative to the destination, or why it
/// may not be written. `.` parts are dropped; an absolute name, a `..` part,
/// a NUL byte or an empty name is refused, so nothing can be written
/// outside the destination.
pub fn entry_path(raw: &[u8]) -> Result<PathBuf, &'static str> {
    if raw.contains(&0) {
        return Err("its name holds a NUL byte");
    }
    if raw.starts_with(b"/") {
        return Err("its name starts at the root of the disk");
    }
    let mut path = PathBuf::new();
    for part in raw.split(|byte| *byte == b'/') {
        match part {
            b"" | b"." => {}
            b".." => return Err("its name leads outside the folder"),
            part => path.push(OsStr::from_bytes(part)),
        }
    }
    if path.as_os_str().is_empty() {
        return Err("it has no name");
    }
    Ok(path)
}

/// Whether a link at `entry` (relative to the destination) pointing to
/// `target` stays inside the destination. An absolute target never does.
pub fn link_stays_inside(entry: &Path, target: &[u8]) -> bool {
    if target.is_empty() || target.starts_with(b"/") || target.contains(&0) {
        return false;
    }
    let mut depth: usize = entry
        .parent()
        .map_or(0, |parent| parent.components().count());
    for part in Path::new(OsStr::from_bytes(target)).components() {
        match part {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(up) => depth = up,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// What an entry is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A regular file.
    File,
    /// A folder.
    Directory,
    /// A symbolic link to this target.
    Link(Vec<u8>),
    /// Anything else: a device, a pipe or a hard link. Never made.
    Special,
}

/// One entry as an archive describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Its name in the archive, `/`-separated bytes.
    pub name: Vec<u8>,
    /// What it is.
    pub kind: Kind,
    /// Its permission bits, when the archive keeps them.
    pub mode: Option<u32>,
    /// When it was last changed, in seconds since 1970, if known.
    pub modified: Option<i64>,
    /// Its size in bytes.
    pub size: u64,
}

/// Why an archive could not be read.
#[derive(Debug)]
pub enum ReadError {
    /// It is encrypted and no password was given.
    NeedsPassword,
    /// The password given is wrong.
    WrongPassword,
    /// It is damaged or not what its format says.
    Damaged(String),
    /// Reading or the visitor failed.
    Io(io::Error),
}

impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// What the visitor of [`read_entries`] is called with first: how many
/// entries and bytes the archive declares, when its format says so (ZIP
/// and 7Z) before reading it.
pub type Totals<'a> = &'a mut dyn FnMut(u64, u64) -> ControlFlow<()>;

/// Called for each entry with its header and its content.
pub type Visit<'a> = &'a mut dyn FnMut(&Header, &mut dyn Read) -> io::Result<ControlFlow<()>>;

/// Reads the archive `source` of `format`, calling `visit` for each entry in
/// order until it breaks.
pub fn read_entries(
    format: Format,
    source: Box<dyn ReadSeek>,
    password: Option<&str>,
    totals: Totals<'_>,
    visit: Visit<'_>,
) -> Result<(), ReadError> {
    match format {
        Format::Zip => read_zip(source, password, &TimeZone::system(), totals, visit),
        Format::SevenZ => read_seven_z(source, password, totals, visit),
        Format::Tar => read_tar(source, visit),
        Format::TarGz => read_tar(flate2::read::MultiGzDecoder::new(source), visit),
        Format::TarBz2 => read_tar(bzip2::read::MultiBzDecoder::new(source), visit),
        Format::TarXz => read_tar(lzma_rust2::XzReader::new(source, true), visit),
        Format::TarZst => read_tar(
            zstd::stream::read::Decoder::new(source).map_err(ReadError::Io)?,
            visit,
        ),
    }
}

fn read_zip(
    source: Box<dyn ReadSeek>,
    password: Option<&str>,
    zone: &TimeZone,
    totals: Totals<'_>,
    visit: Visit<'_>,
) -> Result<(), ReadError> {
    let damaged = |error: zip::result::ZipError| ReadError::Damaged(error.to_string());
    let mut archive = zip::ZipArchive::new(source).map_err(damaged)?;
    let mut bytes: u64 = 0;
    for index in 0..archive.len() {
        if let Ok(file) = archive.by_index_raw(index) {
            bytes = bytes.saturating_add(file.size());
        }
    }
    if totals(archive.len() as u64, bytes).is_break() {
        return Ok(());
    }
    for index in 0..archive.len() {
        let opened = match password {
            Some(password) => archive.by_index_decrypt(index, password.as_bytes()),
            None => archive.by_index(index),
        };
        let mut file = match opened {
            Ok(file) => file,
            Err(zip::result::ZipError::UnsupportedArchive(reason))
                if reason == zip::result::ZipError::PASSWORD_REQUIRED =>
            {
                return Err(ReadError::NeedsPassword);
            }
            Err(zip::result::ZipError::InvalidPassword) => return Err(ReadError::WrongPassword),
            Err(error) => return Err(damaged(error)),
        };
        let mode = file.unix_mode();
        let kind = if file.is_dir() {
            Kind::Directory
        } else if file.is_symlink() {
            let mut target = Vec::new();
            (&mut file).take(4096).read_to_end(&mut target)?;
            Kind::Link(target)
        } else if mode.is_some_and(|mode| mode & 0o170_000 != 0 && mode & 0o170_000 != 0o100_000) {
            Kind::Special
        } else {
            Kind::File
        };
        let header = Header {
            name: file.name_raw().to_vec(),
            kind,
            mode: mode.map(|mode| mode & 0o7777),
            modified: modified_of(&file, zone),
            size: file.size(),
        };
        if visit(&header, &mut file)?.is_break() {
            break;
        }
    }
    Ok(())
}

/// When a ZIP entry was last changed, in seconds since 1970: its extended
/// stamp, which is exact, or else its plain time read as local time.
fn modified_of<R: Read>(file: &zip::read::ZipFile<'_, R>, zone: &TimeZone) -> Option<i64> {
    file.extra_data_fields()
        .find_map(|field| match field {
            zip::extra_fields::ExtraField::ExtendedTimestamp(stamp) => stamp.mod_time(),
            _ => None,
        })
        .map(i64::from)
        .or_else(|| file.last_modified().and_then(|time| zip_time(time, zone)))
}

/// A ZIP time, which has no zone, as seconds since 1970 when read in `zone`.
fn zip_time(time: zip::DateTime, zone: &TimeZone) -> Option<i64> {
    let civil = jiff::civil::DateTime::new(
        i16::try_from(time.year()).ok()?,
        i8::try_from(time.month()).ok()?,
        i8::try_from(time.day()).ok()?,
        i8::try_from(time.hour()).ok()?,
        i8::try_from(time.minute()).ok()?,
        i8::try_from(time.second()).ok()?,
        0,
    )
    .ok()?;
    zone.to_ambiguous_timestamp(civil)
        .compatible()
        .ok()
        .map(jiff::Timestamp::as_second)
}

/// The ZIP time of `modified` (seconds since 1970) in `zone`, kept within
/// the years a ZIP time can hold.
fn dos_time(modified: i64, zone: &TimeZone) -> zip::DateTime {
    let Ok(stamp) = jiff::Timestamp::from_second(modified) else {
        return zip::DateTime::default();
    };
    let civil = zone.to_datetime(stamp);
    zip::DateTime::from_date_and_time(
        u16::try_from(civil.year().clamp(1980, 2107)).unwrap_or(1980),
        civil.month().unsigned_abs(),
        civil.day().unsigned_abs(),
        civil.hour().unsigned_abs(),
        civil.minute().unsigned_abs(),
        civil.second().unsigned_abs(),
    )
    .unwrap_or_default()
}

fn read_tar(reader: impl Read, visit: Visit<'_>) -> Result<(), ReadError> {
    let damaged = |error: io::Error| ReadError::Damaged(error.to_string());
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries().map_err(damaged)? {
        let mut entry = entry.map_err(damaged)?;
        let header = entry.header();
        let kind = match header.entry_type() {
            tar::EntryType::Regular | tar::EntryType::Continuous => Kind::File,
            tar::EntryType::Directory => Kind::Directory,
            tar::EntryType::Symlink => match entry.link_name_bytes() {
                Some(target) => Kind::Link(target.into_owned()),
                None => Kind::Special,
            },
            // Long names and extended headers are read by the tar crate
            // itself; anything else is not made.
            _ => Kind::Special,
        };
        let header = Header {
            name: entry.path_bytes().into_owned(),
            kind,
            mode: header.mode().ok().map(|mode| mode & 0o7777),
            modified: header
                .mtime()
                .ok()
                .and_then(|time| i64::try_from(time).ok()),
            size: header.size().unwrap_or(0),
        };
        if visit(&header, &mut entry)?.is_break() {
            break;
        }
    }
    Ok(())
}

fn read_seven_z(
    source: Box<dyn ReadSeek>,
    password: Option<&str>,
    totals: Totals<'_>,
    visit: Visit<'_>,
) -> Result<(), ReadError> {
    use sevenz_rust2::{ArchiveReader, Error, Password};
    let map = |error: Error| match error {
        Error::PasswordRequired => ReadError::NeedsPassword,
        Error::MaybeBadPassword(_) => ReadError::WrongPassword,
        other => ReadError::Damaged(other.to_string()),
    };
    let password = password.map_or_else(Password::empty, Password::from);
    let mut archive = ArchiveReader::new(source, password).map_err(map)?;
    let entries = &archive.archive().files;
    let bytes = entries.iter().map(|entry| entry.size).sum();
    if totals(entries.len() as u64, bytes).is_break() {
        return Ok(());
    }
    let mut failure: Option<io::Error> = None;
    archive
        .for_each_entries(|entry, content| {
            if entry.is_anti_item {
                return Ok(true);
            }
            let unix = (entry.has_windows_attributes && entry.windows_attributes & 0x8000 != 0)
                .then_some(entry.windows_attributes >> 16);
            let kind = if entry.is_directory {
                Kind::Directory
            } else if unix.is_some_and(|mode| mode & 0o170_000 == 0o120_000) {
                let mut target = Vec::new();
                content.take(4096).read_to_end(&mut target)?;
                Kind::Link(target)
            } else if unix
                .is_some_and(|mode| mode & 0o170_000 != 0 && mode & 0o170_000 != 0o100_000)
            {
                Kind::Special
            } else {
                Kind::File
            };
            let modified = entry.has_last_modified_date.then(|| {
                let time: std::time::SystemTime = entry.last_modified_date.into();
                time.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs() as i64)
            });
            let header = Header {
                name: entry.name.as_bytes().to_vec(),
                kind,
                mode: unix.map(|mode| mode & 0o7777),
                modified,
                size: entry.size,
            };
            match visit(&header, content) {
                Ok(ControlFlow::Continue(())) => Ok(true),
                Ok(ControlFlow::Break(())) => Ok(false),
                Err(error) => {
                    failure = Some(error);
                    Ok(false)
                }
            }
        })
        .map_err(map)?;
    match failure {
        Some(error) => Err(ReadError::Io(error)),
        None => Ok(()),
    }
}

/// One entry to put in a new archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Where it is on disk.
    pub path: PathBuf,
    /// Its name in the archive, `/`-separated bytes.
    pub name: Vec<u8>,
    /// What it is.
    pub kind: Kind,
    /// Its permission bits.
    pub mode: u32,
    /// When it was last changed, in seconds since 1970.
    pub modified: i64,
    /// Its size in bytes.
    pub size: u64,
}

/// Opens an entry's content for writing it into the archive.
pub type Open<'a> = &'a dyn Fn(&Path) -> io::Result<Box<dyn Read>>;

/// Progress in bytes; breaking stops the writing.
pub type Written<'a> = &'a mut dyn FnMut(u64) -> ControlFlow<()>;

/// The error an archive operation fails with when it was asked to stop.
/// (Not [`io::ErrorKind::Interrupted`]: copying loops retry that.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("stopped")]
pub struct Stopped;

/// Whether `error` is a [`Stopped`].
pub fn is_stopped(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|inner| inner.is::<Stopped>())
}

/// Writes `entries` as a `format` archive at `level` into `out`. Entries
/// whose names a format cannot hold (not UTF-8 in ZIP and 7Z, links in 7Z)
/// are passed to `left_out` with a reason instead. Returns after the
/// archive is complete; stopping through `written` fails with [`Stopped`].
pub fn write_archive<W: Write + Seek>(
    format: Format,
    level: Level,
    out: &mut W,
    entries: &[Source],
    open: Open<'_>,
    written: Written<'_>,
    left_out: &mut dyn FnMut(&Source, &'static str),
) -> io::Result<()> {
    match format {
        Format::Zip => write_zip(
            level,
            out,
            entries,
            &TimeZone::system(),
            open,
            written,
            left_out,
        ),
        Format::SevenZ => write_seven_z(level, out, entries, open, written, left_out),
        Format::Tar => write_tar(out, entries, open, written).map(drop),
        Format::TarGz => {
            let encoder =
                flate2::write::GzEncoder::new(out, flate2::Compression::new(gzip_level(level)));
            write_tar(encoder, entries, open, written)?
                .finish()
                .map(drop)
        }
        Format::TarXz => {
            let options = lzma_rust2::XzOptions::with_preset(xz_preset(level));
            let encoder = lzma_rust2::XzWriter::new(out, options)?;
            write_tar(encoder, entries, open, written)?
                .finish()
                .map(drop)
        }
        Format::TarZst => {
            let encoder = zstd::stream::write::Encoder::new(out, zstd_level(level))?;
            write_tar(encoder, entries, open, written)?
                .finish()
                .map(drop)
        }
        Format::TarBz2 => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Rustphin does not make .tar.bz2 archives",
        )),
    }
}

fn gzip_level(level: Level) -> u32 {
    match level {
        Level::Fast => 1,
        Level::Normal => 6,
        Level::Smallest => 9,
    }
}

fn xz_preset(level: Level) -> u32 {
    match level {
        Level::Fast => 1,
        Level::Normal => 6,
        Level::Smallest => 9,
    }
}

fn zstd_level(level: Level) -> i32 {
    match level {
        Level::Fast => 1,
        Level::Normal => 3,
        Level::Smallest => 19,
    }
}

/// A reader that reports its bytes and can be stopped.
struct Counted<'a, R> {
    inner: R,
    written: Written<'a>,
}

impl<R: Read> Read for Counted<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        if (self.written)(read as u64).is_break() {
            return Err(io::Error::other(Stopped));
        }
        Ok(read)
    }
}

fn write_tar<W: Write>(
    out: W,
    entries: &[Source],
    open: Open<'_>,
    written: Written<'_>,
) -> io::Result<W> {
    let mut builder = tar::Builder::new(out);
    builder.follow_symlinks(false);
    for entry in entries {
        let name = Path::new(OsStr::from_bytes(&entry.name));
        let mut header = tar::Header::new_gnu();
        header.set_mode(entry.mode & 0o7777);
        header.set_mtime(u64::try_from(entry.modified).unwrap_or(0));
        match &entry.kind {
            Kind::Directory => {
                header.set_entry_type(tar::EntryType::Directory);
                header.set_size(0);
                builder.append_data(&mut header, name, io::empty())?;
            }
            Kind::Link(target) => {
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);
                builder.append_link(&mut header, name, Path::new(OsStr::from_bytes(target)))?;
            }
            Kind::File => {
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(entry.size);
                let content = open(&entry.path)?;
                // The size is fixed in the header: a file that changed
                // size while read would make a broken archive.
                let mut content = Counted {
                    inner: content.take(entry.size),
                    written: &mut *written,
                };
                builder.append_data(&mut header, name, &mut content)?;
            }
            Kind::Special => {}
        }
    }
    builder.into_inner()
}

fn zip_options(
    level: Level,
    entry: &Source,
    zone: &TimeZone,
) -> zip::write::FullFileOptions<'static, 'static> {
    let time = dos_time(entry.modified, zone);
    let compression = match level {
        Level::Fast => 1,
        Level::Normal => 6,
        Level::Smallest => 9,
    };
    let mut options = zip::write::FullFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression))
        .unix_permissions(entry.mode & 0o7777)
        .last_modified_time(time)
        .large_file(entry.size >= u64::from(u32::MAX));
    // The extended stamp (Info-ZIP "UT": a flag byte saying only the
    // modification time follows, then that time) says exactly when, as Ark,
    // unzip and 7-Zip read it. Times it cannot hold are left to the plain one.
    if let Ok(seconds) = u32::try_from(entry.modified) {
        let mut stamp = vec![1_u8];
        stamp.extend_from_slice(&seconds.to_le_bytes());
        // The field is well formed, so this cannot be refused; were it, the
        // plain time alone still says when.
        let _ = options.add_extra_field(EXTENDED_TIMESTAMP, stamp, false);
    }
    options
}

/// The Info-ZIP extended timestamp field's id.
const EXTENDED_TIMESTAMP: u16 = 0x5455;

fn write_zip<W: Write + Seek>(
    level: Level,
    out: &mut W,
    entries: &[Source],
    zone: &TimeZone,
    open: Open<'_>,
    written: Written<'_>,
    left_out: &mut dyn FnMut(&Source, &'static str),
) -> io::Result<()> {
    let zip_error = |error: zip::result::ZipError| match error {
        zip::result::ZipError::Io(error) => error,
        other => io::Error::other(other.to_string()),
    };
    let mut archive = zip::ZipWriter::new(out);
    for entry in entries {
        let Ok(name) = std::str::from_utf8(&entry.name) else {
            left_out(entry, "ZIP archives need names that are valid text");
            continue;
        };
        let options = zip_options(level, entry, zone);
        match &entry.kind {
            Kind::Directory => archive
                .add_directory(format!("{name}/"), options)
                .map_err(zip_error)?,
            Kind::Link(target) => {
                let Ok(target) = std::str::from_utf8(target) else {
                    left_out(entry, "ZIP archives need link targets that are valid text");
                    continue;
                };
                archive
                    .add_symlink(name, target, options)
                    .map_err(zip_error)?;
            }
            Kind::File => {
                archive.start_file(name, options).map_err(zip_error)?;
                let content = open(&entry.path)?;
                let mut content = Counted {
                    inner: content.take(entry.size),
                    written: &mut *written,
                };
                io::copy(&mut content, &mut archive)?;
            }
            Kind::Special => {}
        }
    }
    archive.finish().map_err(zip_error)?;
    Ok(())
}

fn write_seven_z<W: Write + Seek>(
    level: Level,
    out: &mut W,
    entries: &[Source],
    open: Open<'_>,
    written: Written<'_>,
    left_out: &mut dyn FnMut(&Source, &'static str),
) -> io::Result<()> {
    use sevenz_rust2::{ArchiveEntry, ArchiveWriter, encoder_options::Lzma2Options};
    let preset = xz_preset(level);
    let mut archive =
        ArchiveWriter::new(out).map_err(|error| io::Error::other(error.to_string()))?;
    archive.set_content_methods(vec![Lzma2Options::from_level(preset).into()]);
    for entry in entries {
        let Ok(name) = std::str::from_utf8(&entry.name) else {
            left_out(entry, "7Z archives need names that are valid text");
            continue;
        };
        let mut item = match &entry.kind {
            Kind::Directory => ArchiveEntry::new_directory(name),
            Kind::File => ArchiveEntry::new_file(name),
            Kind::Link(_) => {
                left_out(entry, "7Z archives made by Rustphin do not keep links");
                continue;
            }
            Kind::Special => continue,
        };
        item.has_windows_attributes = true;
        let type_bits = if entry.kind == Kind::Directory {
            0o040_000
        } else {
            0o100_000
        };
        item.windows_attributes = 0x8000 | ((type_bits | (entry.mode & 0o7777)) << 16);
        if entry.kind == Kind::Directory {
            item.windows_attributes |= 0x10;
        }
        item.has_last_modified_date = true;
        let modified = std::time::UNIX_EPOCH
            + std::time::Duration::from_secs(u64::try_from(entry.modified).unwrap_or(0));
        item.last_modified_date = modified
            .try_into()
            .map_err(|_| io::Error::other("a modification time 7Z cannot hold"))?;
        if entry.kind == Kind::File {
            let content = open(&entry.path)?;
            let content = Counted {
                inner: content.take(entry.size),
                written: &mut *written,
            };
            archive
                .push_archive_entry(item, Some(content))
                .map_err(|error| io::Error::other(error.to_string()))?;
        } else {
            archive
                .push_archive_entry::<&[u8]>(item, None)
                .map_err(|error| io::Error::other(error.to_string()))?;
        }
    }
    archive.finish()?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
