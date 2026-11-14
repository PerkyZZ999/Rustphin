//! The archive formats Rustphin does not read itself (RAR, ISO, CAB, LHA,
//! CPIO, RPM, DEB, lone compressed files…), read through libarchive inside
//! the operation engine (D-036).
//!
//! [`read_entries`] reads `Format::Foreign` for the engine (through
//! `archive_tool::ForeignArchives`): it hands each entry's header and content to the
//! engine, which checks names and links and writes through `Fs` as for
//! ZIP and TAR. Names stay bytes. [`probe`] says beforehand whether
//! libarchive can read an archive at all; what it cannot (encrypted RAR
//! and 7Z, ARJ…) goes to 7-Zip (`archive_tool`).

use std::{
    cell::Cell,
    ffi::{CStr, CString, c_char, c_int, c_void},
    io::{self, Read, Seek, SeekFrom},
    ops::ControlFlow,
    ptr::NonNull,
};

use rustphin_core::{
    ReadSeek,
    archive::{self, Header, Kind, ReadError, Visit},
};

/// The part of libarchive's C interface Rustphin uses (`archive.h`,
/// `archive_entry.h`).
mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    /// `struct archive`, opaque.
    #[repr(C)]
    pub struct Archive {
        _private: [u8; 0],
    }

    /// `struct archive_entry`, opaque.
    #[repr(C)]
    pub struct Entry {
        _private: [u8; 0],
    }

    pub type ReadCallback =
        unsafe extern "C" fn(*mut Archive, *mut c_void, *mut *const c_void) -> isize;
    pub type SkipCallback = unsafe extern "C" fn(*mut Archive, *mut c_void, i64) -> i64;
    pub type SeekCallback = unsafe extern "C" fn(*mut Archive, *mut c_void, i64, c_int) -> i64;

    pub const EOF: c_int = 1;
    pub const OK: c_int = 0;
    pub const RETRY: c_int = -10;
    pub const WARN: c_int = -20;
    pub const FATAL: c_int = -30;

    /// `ARCHIVE_FORMAT_BASE_MASK` and `ARCHIVE_FORMAT_RAW`.
    pub const FORMAT_BASE_MASK: c_int = 0x00ff_0000;
    pub const FORMAT_RAW: c_int = 0x0009_0000;

    pub const IFMT: u32 = 0o170_000;
    pub const IFREG: u32 = 0o100_000;
    pub const IFDIR: u32 = 0o040_000;
    pub const IFLNK: u32 = 0o120_000;

    #[link(name = "archive")]
    unsafe extern "C" {
        pub fn archive_read_new() -> *mut Archive;
        pub fn archive_read_support_filter_all(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_7zip(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_ar(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_cab(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_cpio(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_iso9660(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_lha(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_rar(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_rar5(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_tar(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_xar(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_zip(archive: *mut Archive) -> c_int;
        pub fn archive_filter_count(archive: *mut Archive) -> c_int;
        pub fn archive_read_support_format_raw(archive: *mut Archive) -> c_int;
        pub fn archive_read_add_passphrase(archive: *mut Archive, phrase: *const c_char) -> c_int;
        pub fn archive_read_set_read_callback(archive: *mut Archive, call: ReadCallback) -> c_int;
        pub fn archive_read_set_skip_callback(archive: *mut Archive, call: SkipCallback) -> c_int;
        pub fn archive_read_set_seek_callback(archive: *mut Archive, call: SeekCallback) -> c_int;
        pub fn archive_read_set_callback_data(archive: *mut Archive, data: *mut c_void) -> c_int;
        pub fn archive_read_open1(archive: *mut Archive) -> c_int;
        pub fn archive_read_next_header(archive: *mut Archive, entry: *mut *mut Entry) -> c_int;
        pub fn archive_read_data(archive: *mut Archive, buffer: *mut c_void, size: usize) -> isize;
        pub fn archive_read_free(archive: *mut Archive) -> c_int;
        pub fn archive_error_string(archive: *mut Archive) -> *const c_char;
        pub fn archive_format(archive: *mut Archive) -> c_int;

        pub fn archive_entry_pathname(entry: *mut Entry) -> *const c_char;
        pub fn archive_entry_pathname_utf8(entry: *mut Entry) -> *const c_char;
        pub fn archive_entry_filetype(entry: *mut Entry) -> u32;
        pub fn archive_entry_perm(entry: *mut Entry) -> u32;
        pub fn archive_entry_mtime(entry: *mut Entry) -> i64;
        pub fn archive_entry_mtime_is_set(entry: *mut Entry) -> c_int;
        pub fn archive_entry_size(entry: *mut Entry) -> i64;
        pub fn archive_entry_size_is_set(entry: *mut Entry) -> c_int;
        pub fn archive_entry_symlink(entry: *mut Entry) -> *const c_char;
        pub fn archive_entry_hardlink(entry: *mut Entry) -> *const c_char;
        pub fn archive_entry_is_encrypted(entry: *mut Entry) -> c_int;
    }
}

/// What libarchive reads from: the archive's content, a buffer it reads
/// into, and the first error the content gave.
struct Client<'s> {
    source: Box<dyn ReadSeek + 's>,
    buffer: Vec<u8>,
    error: Option<io::Error>,
}

/// An open libarchive reader and the client its callbacks use, reading
/// content that lives for `'s`.
struct Reader<'s> {
    archive: NonNull<ffi::Archive>,
    /// From `Box::into_raw`; freed in `drop`, after the reader that uses it.
    client: NonNull<Client<'s>>,
}

/// Bytes libarchive reads at a time.
const BUFFER: usize = 64 * 1024;

impl<'s> Reader<'s> {
    /// Opens `source` with every filter libarchive has and the archive
    /// formats, raw (a lone compressed file) last. Not mtree, which takes
    /// any text for a list of files, nor libarchive's other text formats.
    fn open(source: Box<dyn ReadSeek + 's>, password: Option<&str>) -> Result<Self, ReadError> {
        let client = Box::new(Client {
            source,
            buffer: vec![0; BUFFER],
            error: None,
        });
        let client = Box::into_non_null(client);
        // SAFETY: `archive_read_new` has no preconditions; it returns null
        // only when out of memory.
        let Some(archive) = NonNull::new(unsafe { ffi::archive_read_new() }) else {
            // SAFETY: `client` came from `Box::into_raw` and nothing else
            // holds it yet.
            drop(unsafe { Box::from_raw(client.as_ptr()) });
            return Err(ReadError::Io(io::Error::other(
                "libarchive could not start",
            )));
        };
        let reader = Self { archive, client };
        let a = archive.as_ptr();
        let phrase = match password.map(CString::new).transpose() {
            Ok(phrase) => phrase,
            // A password with a NUL byte cannot be the right one.
            Err(_) => return Err(ReadError::WrongPassword),
        };
        // SAFETY: `a` is a new reader this function owns. The callbacks
        // have libarchive's signatures, and the data they get is
        // `reader.client`, which stays valid until `drop` has freed the
        // reader. libarchive copies the passphrase.
        let status = unsafe {
            ffi::archive_read_support_filter_all(a);
            ffi::archive_read_support_format_7zip(a);
            ffi::archive_read_support_format_ar(a);
            ffi::archive_read_support_format_cab(a);
            ffi::archive_read_support_format_cpio(a);
            ffi::archive_read_support_format_iso9660(a);
            ffi::archive_read_support_format_lha(a);
            ffi::archive_read_support_format_rar(a);
            ffi::archive_read_support_format_rar5(a);
            ffi::archive_read_support_format_tar(a);
            ffi::archive_read_support_format_xar(a);
            ffi::archive_read_support_format_zip(a);
            ffi::archive_read_support_format_raw(a);
            if let Some(phrase) = &phrase {
                ffi::archive_read_add_passphrase(a, phrase.as_ptr());
            }
            ffi::archive_read_set_read_callback(a, read_callback);
            ffi::archive_read_set_skip_callback(a, skip_callback);
            ffi::archive_read_set_seek_callback(a, seek_callback);
            ffi::archive_read_set_callback_data(a, reader.client.as_ptr().cast());
            ffi::archive_read_open1(a)
        };
        if status == ffi::OK || status == ffi::WARN {
            Ok(reader)
        } else {
            Err(reader.failure(password))
        }
    }

    /// libarchive's own words for its last error.
    fn message(&self) -> String {
        // SAFETY: the reader is open; the string, if any, stays valid until
        // the next call on it and is copied at once.
        let text = unsafe { ffi::archive_error_string(self.archive.as_ptr()) };
        if text.is_null() {
            return "libarchive could not read it".to_owned();
        }
        // SAFETY: libarchive's error string is NUL-terminated.
        unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .into_owned()
    }

    /// The error the content gave, if it gave one.
    fn take_source_error(&self) -> Option<io::Error> {
        // SAFETY: `client` is valid while the reader lives, and no callback
        // runs now: libarchive calls them only inside its own functions,
        // on this thread.
        unsafe { (*self.client.as_ptr()).error.take() }
    }

    /// Why the last call failed, as the engine understands it.
    fn failure(&self, password: Option<&str>) -> ReadError {
        if let Some(error) = self.take_source_error() {
            return ReadError::Io(error);
        }
        let message = self.message();
        password_error(&message, password).unwrap_or(ReadError::Damaged(message))
    }

    /// The next entry, or `None` at the end. A file that is neither an
    /// archive nor compressed is refused: raw accepts any bytes.
    fn next(&self, password: Option<&str>) -> Result<Option<NonNull<ffi::Entry>>, ReadError> {
        let entry = self.next_header(password)?;
        if entry.is_some() && self.is_raw() && self.filters() < 2 {
            return Err(ReadError::Damaged("it is not an archive".to_owned()));
        }
        Ok(entry)
    }

    fn next_header(
        &self,
        password: Option<&str>,
    ) -> Result<Option<NonNull<ffi::Entry>>, ReadError> {
        loop {
            let mut entry = std::ptr::null_mut();
            // SAFETY: the reader is open and `entry` is a place for the
            // pointer; the entry it points at belongs to the reader.
            let status =
                unsafe { ffi::archive_read_next_header(self.archive.as_ptr(), &mut entry) };
            match status {
                ffi::OK | ffi::WARN => {
                    return NonNull::new(entry).map(Some).ok_or_else(|| {
                        ReadError::Damaged("libarchive gave an entry with no header".to_owned())
                    });
                }
                ffi::EOF => return Ok(None),
                ffi::RETRY => {}
                _ => return Err(self.failure(password)),
            }
        }
    }

    /// How many filters libarchive applies; the last is its own reading of
    /// the client, so one means the content was not compressed.
    fn filters(&self) -> c_int {
        // SAFETY: the reader is open.
        unsafe { ffi::archive_filter_count(self.archive.as_ptr()) }
    }

    /// Whether libarchive read the file as a lone compressed stream.
    fn is_raw(&self) -> bool {
        // SAFETY: the reader is open.
        let format = unsafe { ffi::archive_format(self.archive.as_ptr()) };
        format & ffi::FORMAT_BASE_MASK == ffi::FORMAT_RAW
    }
}

impl Drop for Reader<'_> {
    fn drop(&mut self) {
        // SAFETY: the reader is freed once, here, before the client its
        // callbacks use; `client` came from `Box::into_raw` and nothing
        // uses it after the reader is gone.
        unsafe {
            ffi::archive_read_free(self.archive.as_ptr());
            drop(Box::from_raw(self.client.as_ptr()));
        }
    }
}

/// The password error libarchive's `message` describes, if any. Its
/// errors have no codes for this, only words.
fn password_error(message: &str, password: Option<&str>) -> Option<ReadError> {
    let lower = message.to_ascii_lowercase();
    if !lower.contains("passphrase") && !lower.contains("password") {
        return None;
    }
    if lower.contains("not supported") || lower.contains("unsupported") {
        return None;
    }
    Some(match password {
        None => ReadError::NeedsPassword,
        Some(_) => ReadError::WrongPassword,
    })
}

/// # Safety
/// `data` is the `Client` registered in [`Reader::open`] and `out` a place
/// for a buffer pointer, as libarchive passes them.
unsafe extern "C" fn read_callback(
    _archive: *mut ffi::Archive,
    data: *mut c_void,
    out: *mut *const c_void,
) -> isize {
    // SAFETY: `data` is the live `Client` of this reader, and libarchive
    // calls one callback at a time, so this is the only reference to it.
    let client = unsafe { &mut *data.cast::<Client<'_>>() };
    loop {
        match client.source.read(&mut client.buffer) {
            Ok(read) => {
                // SAFETY: `out` is valid for one pointer; the buffer stays
                // put until the next call, as libarchive expects.
                unsafe { *out = client.buffer.as_ptr().cast() };
                return isize::try_from(read).unwrap_or(0);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                client.error = Some(error);
                return ffi::FATAL as isize;
            }
        }
    }
}

/// # Safety
/// As [`read_callback`].
unsafe extern "C" fn skip_callback(
    _archive: *mut ffi::Archive,
    data: *mut c_void,
    request: i64,
) -> i64 {
    if request <= 0 {
        return 0;
    }
    // SAFETY: as in `read_callback`.
    let client = unsafe { &mut *data.cast::<Client<'_>>() };
    // Skipping 0 bytes tells libarchive to read past the data instead.
    match client.source.seek(SeekFrom::Current(request)) {
        Ok(_) => request,
        Err(_) => 0,
    }
}

/// # Safety
/// As [`read_callback`].
unsafe extern "C" fn seek_callback(
    _archive: *mut ffi::Archive,
    data: *mut c_void,
    offset: i64,
    whence: c_int,
) -> i64 {
    // SAFETY: as in `read_callback`.
    let client = unsafe { &mut *data.cast::<Client<'_>>() };
    let to = match whence {
        0 => match u64::try_from(offset) {
            Ok(offset) => SeekFrom::Start(offset),
            Err(_) => return i64::from(ffi::FATAL),
        },
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return i64::from(ffi::FATAL),
    };
    match client.source.seek(to) {
        Ok(position) => i64::try_from(position).unwrap_or(i64::from(ffi::FATAL)),
        Err(error) => {
            client.error = Some(error);
            i64::from(ffi::FATAL)
        }
    }
}

/// The bytes of a C string libarchive owns, if there is one.
///
/// # Safety
/// `text` is null or a NUL-terminated string that stays valid while the
/// result is used.
unsafe fn bytes<'a>(text: *const c_char) -> Option<&'a [u8]> {
    // SAFETY: as the caller promises.
    (!text.is_null()).then(|| unsafe { CStr::from_ptr(text) }.to_bytes())
}

/// The header of `entry`. A lone compressed file's one entry is named
/// after the archive `name` without its last ending.
fn header_of(entry: NonNull<ffi::Entry>, name: &[u8], raw: bool) -> Result<Header, ReadError> {
    let e = entry.as_ptr();
    // SAFETY: `e` is the reader's current entry; each string is copied
    // before the next call on the reader.
    unsafe {
        let path = if raw {
            archive::folder_stem(name).to_vec()
        } else {
            bytes(ffi::archive_entry_pathname(e))
                .or_else(|| bytes(ffi::archive_entry_pathname_utf8(e)))
                .ok_or_else(|| ReadError::Damaged("an entry has no readable name".to_owned()))?
                .to_vec()
        };
        let kind = match ffi::archive_entry_filetype(e) & ffi::IFMT {
            // A hard link to an earlier entry is never made. libarchive
            // gives a TAR hard link no type at all, only the link.
            _ if bytes(ffi::archive_entry_hardlink(e)).is_some() => Kind::Special,
            ffi::IFDIR => Kind::Directory,
            ffi::IFLNK => match bytes(ffi::archive_entry_symlink(e)) {
                Some(target) => Kind::Link(target.to_vec()),
                None => Kind::Special,
            },
            // A lone compressed stream has no type.
            ffi::IFREG | 0 => Kind::File,
            _ => Kind::Special,
        };
        let perm = ffi::archive_entry_perm(e) & 0o7777;
        Ok(Header {
            name: path,
            kind,
            mode: (perm != 0).then_some(perm),
            modified: (ffi::archive_entry_mtime_is_set(e) != 0)
                .then(|| ffi::archive_entry_mtime(e)),
            size: if ffi::archive_entry_size_is_set(e) != 0 {
                u64::try_from(ffi::archive_entry_size(e)).unwrap_or(0)
            } else {
                0
            },
        })
    }
}

/// An entry's content, read through libarchive. Remembers libarchive's
/// own failure, so it is told apart from the visitor's.
struct Content<'a, 's> {
    reader: &'a Reader<'s>,
    failed: &'a Cell<Option<String>>,
}

impl Read for Content<'_, '_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            // SAFETY: the reader is open at an entry; `buffer` is valid for
            // `buffer.len()` bytes.
            let read = unsafe {
                ffi::archive_read_data(
                    self.reader.archive.as_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if let Ok(read) = usize::try_from(read) {
                return Ok(read);
            }
            if read == ffi::RETRY as isize {
                continue;
            }
            if let Some(error) = self.reader.take_source_error() {
                return Err(error);
            }
            let message = self.reader.message();
            self.failed.set(Some(message.clone()));
            return Err(io::Error::other(message));
        }
    }
}

/// Reads the archive called `name` from `source` through libarchive,
/// calling `visit` for each entry until it breaks.
pub fn read_entries(
    name: &[u8],
    source: Box<dyn ReadSeek>,
    password: Option<&str>,
    visit: Visit<'_>,
) -> Result<(), ReadError> {
    let reader = Reader::open(source, password)?;
    let failed = Cell::new(None);
    while let Some(entry) = reader.next(password)? {
        let header = header_of(entry, name, reader.is_raw())?;
        // A disc image lists its own root as ".": nothing to make.
        if header.kind == Kind::Directory && matches!(header.name.as_slice(), b"." | b"./") {
            continue;
        }
        let mut content = Content {
            reader: &reader,
            failed: &failed,
        };
        match visit(&header, &mut content) {
            Ok(ControlFlow::Continue(())) => {}
            Ok(ControlFlow::Break(())) => break,
            Err(error) => {
                return Err(match failed.take() {
                    // libarchive failed, not the visitor.
                    Some(message) => {
                        password_error(&message, password).unwrap_or(ReadError::Damaged(message))
                    }
                    None => ReadError::Io(error),
                });
            }
        }
    }
    Ok(())
}

/// Whether libarchive can read an archive, judged by its first entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// libarchive reads it.
    Readable,
    /// libarchive cannot read it, for this reason (an unknown format, or
    /// encryption libarchive does not undo), and it goes to 7-Zip.
    Unreadable(String),
}

/// Whether libarchive can read the archive `source`, which was opened
/// through the engine's `Fs`. `source` is left at its start for the reader.
/// `Err` when `source` itself fails: that is no reason to try 7-Zip.
pub fn probe(source: &mut dyn ReadSeek) -> io::Result<Probe> {
    let verdict = {
        let reader = match Reader::open(Box::new(&mut *source), None) {
            Ok(reader) => reader,
            Err(ReadError::Io(error)) => return Err(error),
            Err(error) => return Ok(Probe::Unreadable(describe(&error))),
        };
        match reader.next(None) {
            Ok(Some(entry)) => {
                // SAFETY: `entry` is the reader's current entry.
                let encrypted = unsafe { ffi::archive_entry_is_encrypted(entry.as_ptr()) } != 0;
                if encrypted {
                    // Among the formats that reach here, libarchive decrypts
                    // none: ZIP, the one it can, Rustphin reads itself.
                    Probe::Unreadable("it is encrypted".to_owned())
                } else {
                    Probe::Readable
                }
            }
            Ok(None) => Probe::Readable,
            Err(ReadError::Io(error)) => return Err(error),
            Err(error) => Probe::Unreadable(describe(&error)),
        }
    };
    source.rewind()?;
    Ok(verdict)
}

/// A [`ReadError`] in a few words.
fn describe(error: &ReadError) -> String {
    match error {
        ReadError::NeedsPassword | ReadError::WrongPassword => "it is encrypted".to_owned(),
        ReadError::Damaged(message) => message.clone(),
        ReadError::Io(error) => error.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
