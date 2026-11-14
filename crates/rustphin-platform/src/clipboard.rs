//! The file clipboard: the entries picked up with copy or cut, shared with
//! other file managers.
//!
//! A toolkit's own clipboard cannot carry files to other applications (GPUI's
//! could not; Qt's is used only for text here), so this
//! talks to the compositor directly through a data-control protocol (decision
//! D-010). Where the compositor has none, [`LocalClipboard`] keeps copy and
//! paste working inside the application.

use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use wl_clipboard_rs::{
    copy::{self, MimeSource, MimeType as CopyMime, Source},
    paste::{self, ClipboardType, MimeType as PasteMime, Seat},
    watch::Watcher,
};

use crate::uri::{paths_from_uri_list, uri_list};

const URI_LIST: &str = "text/uri-list";
/// KDE's marker: `1` when the files were cut, `0` when copied.
const KDE_CUT: &str = "application/x-kde-cutselection";
/// GNOME's format: `copy` or `cut` on the first line, then one URI per line.
const GNOME_FILES: &str = "x-special/gnome-copied-files";
const PLAIN_TEXT: &str = "text/plain;charset=utf-8";

/// Files on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileClip {
    /// The files, as absolute paths.
    pub paths: Vec<PathBuf>,
    /// Whether pasting moves them instead of copying.
    pub cut: bool,
}

/// Why the clipboard could not be used.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ClipboardError(String);

/// Stops a clipboard watch when dropped.
pub type WatchGuard = Box<dyn Send>;

/// A clipboard that can hold files.
pub trait FileClipboard: Send + Sync {
    /// Puts `clip` on the clipboard, replacing what was there.
    fn write(&self, clip: &FileClip) -> Result<(), ClipboardError>;

    /// The files on the clipboard, or `None` when it holds none.
    fn read(&self) -> Result<Option<FileClip>, ClipboardError>;

    /// Empties the clipboard.
    fn clear(&self) -> Result<(), ClipboardError>;

    /// Calls `on_change` from another thread whenever the clipboard's
    /// content changes, until the returned guard is dropped. `None` when
    /// changes cannot be observed.
    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Option<WatchGuard>;

    /// Whether other applications see this clipboard.
    fn is_shared(&self) -> bool;
}

/// The best clipboard this session offers: the compositor's when it has a
/// data-control protocol, otherwise one that only this application sees.
pub fn for_session() -> Arc<dyn FileClipboard> {
    match WaylandClipboard::connect() {
        Ok(clipboard) => Arc::new(clipboard),
        // The caller can tell from `is_shared` and say so to the user.
        Err(_) => Arc::new(LocalClipboard::default()),
    }
}

/// The formats offered for `clip`, as (MIME type, body) pairs.
fn offers(clip: &FileClip) -> Vec<(&'static str, Vec<u8>)> {
    let uris = uri_list(&clip.paths);
    let verb = if clip.cut { "cut" } else { "copy" };
    let gnome = std::iter::once(verb.to_owned())
        .chain(uris.lines().map(str::to_owned))
        .collect::<Vec<_>>()
        .join("\n");
    // Plain paths, for pasting into a terminal or an editor. Raw bytes: a
    // name that is not UTF-8 stays what it is.
    let mut text = Vec::new();
    for (index, path) in clip.paths.iter().enumerate() {
        if index > 0 {
            text.push(b'\n');
        }
        text.extend_from_slice(path.as_os_str().as_encoded_bytes());
    }
    vec![
        (URI_LIST, uris.into_bytes()),
        (
            KDE_CUT,
            if clip.cut {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            },
        ),
        (GNOME_FILES, gnome.into_bytes()),
        (PLAIN_TEXT, text),
    ]
}

/// Reads a clip out of the formats another application offers. `fetch`
/// returns the body of a MIME type, or `None` when it is not offered.
fn clip_from(fetch: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Option<FileClip> {
    if let Some(body) = fetch(URI_LIST) {
        let paths = paths_from_uri_list(&body);
        if paths.is_empty() {
            return None;
        }
        let cut = fetch(KDE_CUT).is_some_and(|flag| flag.trim_ascii() == b"1")
            || fetch(GNOME_FILES).is_some_and(|body| body.starts_with(b"cut\n"));
        return Some(FileClip { paths, cut });
    }
    // GNOME applications may offer only their own format.
    let body = fetch(GNOME_FILES)?;
    let (verb, uris) = body.split_at(body.iter().position(|&byte| byte == b'\n')?);
    let paths = paths_from_uri_list(uris);
    (!paths.is_empty()).then(|| FileClip {
        paths,
        cut: verb == b"cut",
    })
}

/// The compositor's clipboard, reached through a data-control protocol.
#[derive(Debug)]
pub struct WaylandClipboard(());

impl WaylandClipboard {
    /// Checks that the compositor offers a data-control protocol.
    pub fn connect() -> Result<Self, ClipboardError> {
        match paste::get_mime_types(ClipboardType::Regular, Seat::Unspecified) {
            // An empty clipboard still proves the protocol is there.
            Ok(_) | Err(paste::Error::ClipboardEmpty | paste::Error::NoSeats) => Ok(Self(())),
            Err(error) => Err(ClipboardError(error.to_string())),
        }
    }
}

impl FileClipboard for WaylandClipboard {
    fn write(&self, clip: &FileClip) -> Result<(), ClipboardError> {
        let sources = offers(clip)
            .into_iter()
            .map(|(mime, body)| MimeSource {
                source: Source::Bytes(body.into()),
                mime_type: CopyMime::Specific(mime.to_owned()),
            })
            .collect();
        // Served from a thread of this process until something else is copied.
        copy::Options::new()
            .copy_multi(sources)
            .map_err(|error| ClipboardError(error.to_string()))
    }

    fn read(&self) -> Result<Option<FileClip>, ClipboardError> {
        let offered = match paste::get_mime_types(ClipboardType::Regular, Seat::Unspecified) {
            Ok(offered) => offered,
            Err(paste::Error::ClipboardEmpty | paste::Error::NoSeats) => return Ok(None),
            Err(error) => return Err(ClipboardError(error.to_string())),
        };
        let mut fetch = |mime: &str| {
            if !offered.contains(mime) {
                return None;
            }
            let (mut pipe, _) = paste::get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                PasteMime::Specific(mime),
            )
            .ok()?;
            let mut body = Vec::new();
            pipe.read_to_end(&mut body).ok()?;
            Some(body)
        };
        Ok(clip_from(&mut fetch))
    }

    fn clear(&self) -> Result<(), ClipboardError> {
        copy::clear(copy::ClipboardType::Regular, copy::Seat::All)
            .map_err(|error| ClipboardError(error.to_string()))
    }

    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Option<WatchGuard> {
        let mut watcher = Watcher::new(
            wl_clipboard_rs::watch::ClipboardType::Regular,
            Seat::Unspecified,
        )
        .ok()?;
        let cancel = watcher.cancel_handle();
        thread::Builder::new()
            .name("rustphin-clipboard".to_owned())
            .spawn(move || {
                // Ends when cancelled or when the connection is lost.
                while let Ok(Some(_)) = watcher.next_event() {
                    on_change();
                }
            })
            .ok()?;
        struct Cancel(wl_clipboard_rs::watch::CancelHandle);
        impl Drop for Cancel {
            fn drop(&mut self) {
                self.0.cancel();
            }
        }
        Some(Box::new(Cancel(cancel)))
    }

    fn is_shared(&self) -> bool {
        true
    }
}

/// A clipboard only this application sees: the fallback where the compositor
/// offers no data-control protocol.
#[derive(Debug, Default)]
pub struct LocalClipboard {
    clip: Mutex<Option<FileClip>>,
}

impl LocalClipboard {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<FileClip>> {
        self.clip
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl FileClipboard for LocalClipboard {
    fn write(&self, clip: &FileClip) -> Result<(), ClipboardError> {
        *self.lock() = Some(clip.clone());
        Ok(())
    }

    fn read(&self) -> Result<Option<FileClip>, ClipboardError> {
        Ok(self.lock().clone())
    }

    fn clear(&self) -> Result<(), ClipboardError> {
        *self.lock() = None;
        Ok(())
    }

    fn watch(&self, _: Box<dyn Fn() + Send>) -> Option<WatchGuard> {
        // Only this application writes to it, and it knows when it does.
        None
    }

    fn is_shared(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn clip(paths: &[&str], cut: bool) -> FileClip {
        FileClip {
            paths: paths.iter().map(PathBuf::from).collect(),
            cut,
        }
    }

    /// What another application would read back from our offers.
    fn round_trip(clip: &FileClip, formats: &[&str]) -> Option<FileClip> {
        let offered: HashMap<_, _> = offers(clip)
            .into_iter()
            .filter(|(mime, _)| formats.contains(mime))
            .collect();
        clip_from(&mut |mime| offered.get(mime).cloned())
    }

    #[test]
    fn a_copy_and_a_cut_are_offered_in_the_formats_file_managers_read() {
        let copied = clip(&["/home/u/a b.txt", "/home/u/é"], false);
        let offered: HashMap<_, _> = offers(&copied).into_iter().collect();
        assert_eq!(
            offered[URI_LIST],
            b"file:///home/u/a%20b.txt\r\nfile:///home/u/%C3%A9\r\n"
        );
        assert_eq!(offered[KDE_CUT], b"0");
        assert_eq!(
            offered[GNOME_FILES],
            b"copy\nfile:///home/u/a%20b.txt\nfile:///home/u/%C3%A9"
        );
        assert_eq!(offered[PLAIN_TEXT], "/home/u/a b.txt\n/home/u/é".as_bytes());

        let cut: HashMap<_, _> = offers(&clip(&["/x"], true)).into_iter().collect();
        assert_eq!(cut[KDE_CUT], b"1");
        assert!(cut[GNOME_FILES].starts_with(b"cut\n"));
    }

    #[test]
    fn what_is_offered_reads_back_the_same_whichever_formats_the_reader_uses() {
        for original in [
            clip(&["/home/u/a b.txt", "/tmp/日本語 100%"], false),
            clip(&["/home/u/moved"], true),
        ] {
            // A KDE reader, a GNOME reader, and one that knows only the URI list.
            assert_eq!(
                round_trip(&original, &[URI_LIST, KDE_CUT]),
                Some(original.clone())
            );
            assert_eq!(
                round_trip(&original, &[GNOME_FILES]),
                Some(original.clone())
            );
            assert_eq!(
                round_trip(&original, &[URI_LIST, GNOME_FILES]),
                Some(original.clone())
            );
            let plain = round_trip(&original, &[URI_LIST]).unwrap();
            assert_eq!(plain.paths, original.paths);
            assert!(!plain.cut, "without a marker it is a copy");
        }
    }

    #[test]
    fn a_clipboard_without_files_holds_no_clip() {
        let text_only: HashMap<&str, Vec<u8>> = HashMap::from([(PLAIN_TEXT, b"hello".to_vec())]);
        assert_eq!(clip_from(&mut |mime| text_only.get(mime).cloned()), None);
        let links: HashMap<&str, Vec<u8>> =
            HashMap::from([(URI_LIST, b"https://example.org/\r\n".to_vec())]);
        assert_eq!(clip_from(&mut |mime| links.get(mime).cloned()), None);
    }

    #[test]
    fn the_local_clipboard_keeps_the_last_clip_until_cleared() {
        let clipboard = LocalClipboard::default();
        assert_eq!(clipboard.read().unwrap(), None);
        clipboard.write(&clip(&["/a"], false)).unwrap();
        clipboard.write(&clip(&["/b"], true)).unwrap();
        assert_eq!(clipboard.read().unwrap(), Some(clip(&["/b"], true)));
        clipboard.clear().unwrap();
        assert_eq!(clipboard.read().unwrap(), None);
        assert!(!clipboard.is_shared());
    }
}
