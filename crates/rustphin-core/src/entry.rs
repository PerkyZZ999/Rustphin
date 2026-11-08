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
    /// The file name each symbolic link points to, by position in the
    /// chunk, in increasing order. Only links have one, so this stays small.
    link_targets: Vec<(u32, Box<[u8]>)>,
}

impl Chunk {
    /// Records the file name the entry pushed last points to, when it is a
    /// symbolic link; its type is shown as the target's, as Dolphin does.
    pub fn set_link_target(&mut self, target_name: &[u8]) {
        if let Some(last) = self.ends.len().checked_sub(1) {
            self.link_targets
                .push((last as u32, target_name.to_vec().into_boxed_slice()));
        }
    }

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

    /// The file name a symbolic link entry points to, if it was read.
    pub fn link_target(&self, index: u32) -> Option<&[u8]> {
        let index = index as usize;
        let chunk = &self.chunks[index / CHUNK_LEN];
        let local = (index % CHUNK_LEN) as u32;
        chunk
            .link_targets
            .binary_search_by_key(&local, |(position, _)| *position)
            .ok()
            .map(|found| &*chunk.link_targets[found].1)
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
    fn link_targets_are_found_in_any_chunk_and_only_for_links() {
        let mut store = EntryStore::default();
        let mut chunk = Chunk::default();
        for index in 0..CHUNK_LEN + 3 {
            let name = format!("entry-{index}");
            if index == 5 || index == CHUNK_LEN + 1 {
                chunk.push(name.as_bytes(), EntryKind::Symlink, false);
                chunk.set_link_target(format!("target-{index}").as_bytes());
            } else {
                chunk.push(name.as_bytes(), EntryKind::File, false);
            }
            if chunk.len() == CHUNK_LEN {
                store.push_chunk(std::mem::take(&mut chunk));
            }
        }
        store.push_chunk(chunk);

        assert_eq!(store.link_target(5), Some(&b"target-5"[..]));
        let second = (CHUNK_LEN + 1) as u32;
        assert_eq!(
            store.link_target(second),
            Some(format!("target-{second}").as_bytes())
        );
        assert_eq!(store.link_target(1), None);
        assert_eq!(store.link_target(second - CHUNK_LEN as u32), None);
    }

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
