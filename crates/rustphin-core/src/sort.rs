//! Ordering of entries.

use std::cmp::Ordering;

use rayon::prelude::*;

use crate::{EntryStore, ViewOptions, details::Details};

/// What entries are ordered by. Ties always fall back to the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Natural order of the file name.
    #[default]
    Name,
    /// Size in bytes. Folders compare as equal and so stay in name order.
    Size,
    /// Time of last modification.
    Modified,
    /// File extension, compared without regard to case; entries without one first.
    Type,
}

impl SortKey {
    /// Whether ordering by this key needs each entry's [`crate::Metadata`].
    pub fn needs_details(self) -> bool {
        matches!(self, Self::Size | Self::Modified)
    }
}

/// Natural, ASCII-case-insensitive comparison of file names.
///
/// Runs of ASCII digits compare by numeric value, so `file2` sorts before
/// `file10`. Ties are broken by raw bytes, which makes the order total.
/// Non-ASCII text currently compares by byte value, not by locale collation.
pub fn natural_cmp(a: &[u8], b: &[u8]) -> Ordering {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (run_a, next_i) = digit_run(a, i);
            let (run_b, next_j) = digit_run(b, j);
            match run_a.len().cmp(&run_b.len()).then_with(|| run_a.cmp(run_b)) {
                Ordering::Equal => {}
                other => return other,
            }
            (i, j) = (next_i, next_j);
        } else {
            match a[i].to_ascii_lowercase().cmp(&b[j].to_ascii_lowercase()) {
                Ordering::Equal => {}
                other => return other,
            }
            (i, j) = (i + 1, j + 1);
        }
    }
    (a.len() - i).cmp(&(b.len() - j)).then_with(|| a.cmp(b))
}

/// Returns the digit run starting at `start` without leading zeros, and the index after it.
fn digit_run(bytes: &[u8], start: usize) -> (&[u8], usize) {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut first = start;
    while first + 1 < end && bytes[first] == b'0' {
        first += 1;
    }
    (&bytes[first..end], end)
}

/// The extension of a file name: the text after the last dot, unless that dot
/// is the first byte (a hidden file) or the last.
fn extension(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&byte| byte == b'.') {
        Some(dot) if dot > 0 && dot + 1 < name.len() => &name[dot + 1..],
        _ => b"",
    }
}

fn extension_cmp(a: &[u8], b: &[u8]) -> Ordering {
    let lower = |name| extension(name).iter().map(u8::to_ascii_lowercase);
    lower(a).cmp(lower(b))
}

/// Builds the display order of `store` under `options`: the indices of visible
/// entries, sorted by the chosen key.
///
/// Containers come first when requested, in either direction. A key that needs
/// `details` falls back to the name for entries whose details are missing, and
/// for every entry when `details` is `None`.
pub(crate) fn build_order(
    store: &EntryStore,
    details: Option<&Details>,
    options: ViewOptions,
) -> Vec<u32> {
    let detail = |index| details.and_then(|details| details.get(index));
    let by_key = |a: u32, b: u32| match options.sort {
        SortKey::Name => Ordering::Equal,
        SortKey::Type => extension_cmp(store.name(a), store.name(b)),
        SortKey::Size if store.is_container(a) && store.is_container(b) => Ordering::Equal,
        SortKey::Size => detail(a).map(|d| d.size).cmp(&detail(b).map(|d| d.size)),
        SortKey::Modified => detail(a)
            .map(|d| d.modified)
            .cmp(&detail(b).map(|d| d.modified)),
    };

    let mut order: Vec<u32> = (0..store.len() as u32)
        .into_par_iter()
        .filter(|&i| options.show_hidden || !store.is_hidden(i))
        .collect();
    order.par_sort_unstable_by(|&a, &b| {
        let containers = if options.folders_first {
            store.is_container(b).cmp(&store.is_container(a))
        } else {
            Ordering::Equal
        };
        let keyed = by_key(a, b).then_with(|| natural_cmp(store.name(a), store.name(b)));
        containers.then(if options.descending {
            keyed.reverse()
        } else {
            keyed
        })
    });
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chunk, EntryKind, Metadata};

    fn sorted(mut names: Vec<&str>) -> Vec<&str> {
        names.sort_by(|a, b| natural_cmp(a.as_bytes(), b.as_bytes()));
        names
    }

    #[test]
    fn numbers_compare_by_value() {
        assert_eq!(
            sorted(vec!["file10", "file2", "File1", "file02", "file", "file2a"]),
            ["file", "File1", "file02", "file2", "file2a", "file10"]
        );
    }

    #[test]
    fn order_is_total_and_case_insensitive() {
        assert_eq!(sorted(vec!["b", "A", "a", "B"]), ["A", "a", "B", "b"]);
        assert_eq!(natural_cmp(b"a1", b"a01"), Ordering::Greater);
        assert_eq!(natural_cmp(b"same", b"same"), Ordering::Equal);
        assert_eq!(natural_cmp(b"", b"a"), Ordering::Less);
        assert_eq!(
            natural_cmp(b"99999999999999999999999", b"100000000000000000000000"),
            Ordering::Less
        );
    }

    #[test]
    fn extension_ignores_hidden_prefix_and_trailing_dot() {
        assert_eq!(extension(b"photo.JPG"), b"JPG");
        assert_eq!(extension(b"archive.tar.gz"), b"gz");
        assert_eq!(extension(b".bashrc"), b"");
        assert_eq!(extension(b"README"), b"");
        assert_eq!(extension(b"odd."), b"");
    }

    /// Five entries: two folders and three files, with sizes and times chosen
    /// so that every key gives a different order. The folders' own sizes run
    /// against their name order, so ordering folders by size would show.
    fn sample() -> (EntryStore, Details) {
        let mut chunk = Chunk::default();
        let mut details = Vec::new();
        for (name, kind, size, modified) in [
            ("zeta.txt", EntryKind::File, 30, 100),
            (".config", EntryKind::Directory, 9000, 400),
            ("alpha", EntryKind::Directory, 100, 300),
            ("Beta.md", EntryKind::File, 10, 500),
            ("gamma.TXT", EntryKind::File, 20, 200),
        ] {
            chunk.push(name.as_bytes(), kind, kind == EntryKind::Directory);
            details.push(Some(Metadata { size, modified }));
        }
        let mut store = EntryStore::default();
        store.push_chunk(chunk);
        (store, Details::new(details))
    }

    fn names(store: &EntryStore, details: Option<&Details>, options: ViewOptions) -> Vec<String> {
        build_order(store, details, options)
            .into_iter()
            .map(|i| String::from_utf8_lossy(store.name(i)).into_owned())
            .collect()
    }

    #[test]
    fn folders_first_and_hidden_filter() {
        let (store, _) = sample();
        assert_eq!(
            names(&store, None, ViewOptions::default()),
            ["alpha", "Beta.md", "gamma.TXT", "zeta.txt"]
        );
        let all = ViewOptions {
            show_hidden: true,
            folders_first: false,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, None, all),
            [".config", "alpha", "Beta.md", "gamma.TXT", "zeta.txt"]
        );
    }

    #[test]
    fn sorts_files_by_size_and_keeps_folders_in_name_order() {
        let (store, details) = sample();
        let by_size = ViewOptions {
            sort: SortKey::Size,
            show_hidden: true,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, Some(&details), by_size),
            [".config", "alpha", "Beta.md", "gamma.TXT", "zeta.txt"]
        );
    }

    #[test]
    fn descending_reverses_entries_but_folders_stay_first() {
        let (store, details) = sample();
        let options = ViewOptions {
            sort: SortKey::Size,
            descending: true,
            show_hidden: true,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, Some(&details), options),
            ["alpha", ".config", "zeta.txt", "gamma.TXT", "Beta.md"]
        );
    }

    #[test]
    fn sorts_by_modification_time() {
        let (store, details) = sample();
        let options = ViewOptions {
            sort: SortKey::Modified,
            folders_first: false,
            show_hidden: true,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, Some(&details), options),
            ["zeta.txt", "gamma.TXT", "alpha", ".config", "Beta.md"]
        );
    }

    #[test]
    fn sorts_by_extension_without_regard_to_case() {
        let mut chunk = Chunk::default();
        for name in ["apple.zip", "Beta.md", "gamma.TXT", "notes", "delta.txt"] {
            chunk.push(name.as_bytes(), EntryKind::File, false);
        }
        let mut store = EntryStore::default();
        store.push_chunk(chunk);
        let options = ViewOptions {
            sort: SortKey::Type,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, None, options),
            ["notes", "Beta.md", "delta.txt", "gamma.TXT", "apple.zip"]
        );
    }

    #[test]
    fn a_key_without_its_details_falls_back_to_the_name() {
        let (store, _) = sample();
        let options = ViewOptions {
            sort: SortKey::Size,
            descending: true,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, None, options),
            ["alpha", "zeta.txt", "gamma.TXT", "Beta.md"]
        );
    }

    #[test]
    fn entries_with_unreadable_details_sort_before_the_rest() {
        let (store, _) = sample();
        let details = Details::new(vec![
            Some(Metadata {
                size: 30,
                modified: 0,
            }),
            None,
            None,
            None,
            Some(Metadata {
                size: 20,
                modified: 0,
            }),
        ]);
        let options = ViewOptions {
            sort: SortKey::Size,
            ..ViewOptions::default()
        };
        assert_eq!(
            names(&store, Some(&details), options),
            ["alpha", "Beta.md", "gamma.TXT", "zeta.txt"]
        );
    }
}
