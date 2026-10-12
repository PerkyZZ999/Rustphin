//! Compact, append-only storage for the entries of one directory.

use std::{borrow::Cow, sync::Arc};

/// Number of entries in every chunk of an [`EntryStore`] except the last.
pub const CHUNK_LEN: usize = 8192;

/// What a directory entry is, without following symbolic links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A directory.
    Directory,
    /// A regular file.
    File,
    /// A symbolic link.
    Symlink,
    /// A socket, FIFO, device node or an entry whose type could not be read.
    Other,
}

/// A fixed block of entries. Names share one byte buffer.
#[derive(Debug, Default)]
pub struct Chunk {
    names: Vec<u8>,
    ends: Vec<u32>,
    kinds: Vec<EntryKind>,
    containers: Vec<bool>,
}

impl Chunk {
    /// Appends an entry. `container` says whether it can be entered like a
    /// directory, which is also true for a symbolic link to a directory.
    pub fn push(&mut self, name: &[u8], kind: EntryKind, container: bool) {
        self.names.extend_from_slice(name);
        self.ends.push(self.names.len() as u32);
        self.kinds.push(kind);
        self.containers.push(container);
    }

    /// Number of entries in the chunk.
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Whether the chunk holds no entries.
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    fn name(&self, index: usize) -> &[u8] {
        let start = if index == 0 {
            0
        } else {
            self.ends[index - 1] as usize
        };
        &self.names[start..self.ends[index] as usize]
    }
}

/// The entries of one directory, in the order they were listed.
///
/// Cloning is cheap: chunks are shared, so a store can grow while earlier
/// clones stay valid and unchanged.
#[derive(Debug, Clone, Default)]
pub struct EntryStore {
    chunks: Vec<Arc<Chunk>>,
    len: usize,
}

impl EntryStore {
    /// Appends a chunk. Every chunk before the last must hold [`CHUNK_LEN`] entries.
    pub fn push_chunk(&mut self, chunk: Chunk) {
        debug_assert!(
            self.len.is_multiple_of(CHUNK_LEN),
            "only the last chunk may be partial"
        );
        self.len += chunk.len();
        self.chunks.push(Arc::new(chunk));
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the store holds no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The entry's file name as raw bytes. File names are not required to be UTF-8.
    pub fn name(&self, index: u32) -> &[u8] {
        let index = index as usize;
        self.chunks[index / CHUNK_LEN].name(index % CHUNK_LEN)
    }

    /// The entry's kind.
    pub fn kind(&self, index: u32) -> EntryKind {
        let index = index as usize;
        self.chunks[index / CHUNK_LEN].kinds[index % CHUNK_LEN]
    }

    /// Whether the entry can be entered: a directory or a link to one.
    pub fn is_container(&self, index: u32) -> bool {
        let index = index as usize;
        self.chunks[index / CHUNK_LEN].containers[index % CHUNK_LEN]
    }

    /// Whether the entry is hidden by the dot-file convention.
    pub fn is_hidden(&self, index: u32) -> bool {
        self.name(index).first() == Some(&b'.')
    }
}

/// Renders a file name for display.
///
/// Invalid UTF-8 becomes U+FFFD and control characters become their visible
/// "control picture" symbols, so a hostile name cannot disturb the layout.
pub fn display_name(name: &[u8]) -> Cow<'_, str> {
    let text = String::from_utf8_lossy(name);
    if !text.chars().any(char::is_control) {
        return text;
    }
    Cow::Owned(
        text.chars()
            .map(|c| match c {
                '\u{0}'..='\u{1f}' => char::from_u32(0x2400 + c as u32).unwrap_or('\u{fffd}'),
                '\u{7f}' => '\u{2421}',
                c if c.is_control() => '\u{fffd}',
                c => c,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_spans_chunks() {
        let mut store = EntryStore::default();
        let mut chunk = Chunk::default();
        for n in 0..CHUNK_LEN {
            chunk.push(format!("f{n}").as_bytes(), EntryKind::File, false);
        }
        store.push_chunk(chunk);
        let early = store.clone();
        let mut chunk = Chunk::default();
        chunk.push(b".dir", EntryKind::Directory, true);
        store.push_chunk(chunk);

        assert_eq!(early.len(), CHUNK_LEN);
        assert_eq!(store.len(), CHUNK_LEN + 1);
        assert_eq!(store.name(0), b"f0");
        assert_eq!(
            store.name(CHUNK_LEN as u32 - 1),
            format!("f{}", CHUNK_LEN - 1).as_bytes()
        );
        let last = CHUNK_LEN as u32;
        assert_eq!(store.name(last), b".dir");
        assert_eq!(store.kind(last), EntryKind::Directory);
        assert!(store.is_container(last) && store.is_hidden(last));
        assert!(!store.is_hidden(0));
    }

    #[test]
    fn display_name_is_safe() {
        assert_eq!(display_name(b"plain.txt"), "plain.txt");
        assert_eq!(display_name(b"bad\xff.txt"), "bad\u{fffd}.txt");
        assert_eq!(
            display_name(b"a\nb\x1b[31m\x7f"),
            "a\u{240a}b\u{241b}[31m\u{2421}"
        );
        assert_eq!(
            display_name("caf\u{e9} \u{65e5}\u{672c}".as_bytes()),
            "caf\u{e9} \u{65e5}\u{672c}"
        );
    }
}
