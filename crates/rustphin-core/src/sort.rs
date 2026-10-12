//! Ordering of entries.

use std::cmp::Ordering;

use rayon::prelude::*;

use crate::{EntryStore, ViewOptions};

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

/// Builds the display order of `store` under `options`: the indices of visible
/// entries, sorted by name with containers first when requested.
pub(crate) fn build_order(store: &EntryStore, options: ViewOptions) -> Vec<u32> {
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
        containers.then_with(|| natural_cmp(store.name(a), store.name(b)))
    });
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chunk, EntryKind};

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
    fn folders_first_and_hidden_filter() {
        let mut chunk = Chunk::default();
        chunk.push(b"zeta.txt", EntryKind::File, false);
        chunk.push(b".config", EntryKind::Directory, true);
        chunk.push(b"alpha", EntryKind::Directory, true);
        chunk.push(b"Beta.txt", EntryKind::File, false);
        chunk.push(b"link", EntryKind::Symlink, true);
        let mut store = EntryStore::default();
        store.push_chunk(chunk);
        let names = |options| -> Vec<String> {
            build_order(&store, options)
                .into_iter()
                .map(|i| String::from_utf8_lossy(store.name(i)).into_owned())
                .collect()
        };

        assert_eq!(
            names(ViewOptions {
                show_hidden: false,
                folders_first: true
            }),
            ["alpha", "link", "Beta.txt", "zeta.txt"]
        );
        assert_eq!(
            names(ViewOptions {
                show_hidden: true,
                folders_first: false
            }),
            [".config", "alpha", "Beta.txt", "link", "zeta.txt"]
        );
    }
}
