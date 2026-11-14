//! The batch-rename recovery record is untrusted input read at startup
//! (D-020). These tests write records through a test encoder that follows
//! the record's layout, then damage them one way each, and recover through
//! the public `recover_rename_batch`: a damaged record is refused as
//! invalid data and nothing in the folder changes.

use std::{ffi::OsStr, fs, io, os::unix::ffi::OsStrExt as _, path::Path};

use proptest::prelude::*;

use crate::{
    Fs as _, RealFs,
    ops::{
        OpError, RENAME_JOURNAL, recover_rename_batch,
        tests::{Dirs, dirs, file, tree},
    },
};

/// One slot of a record as the test writes it.
struct Entry {
    origin: Vec<u8>,
    temporary: Vec<u8>,
    target: Vec<u8>,
    kind: u8,
}

fn entry(origin: &[u8], temporary: &[u8], target: &[u8]) -> Entry {
    Entry {
        origin: origin.to_vec(),
        temporary: temporary.to_vec(),
        target: target.to_vec(),
        kind: 0,
    }
}

/// `a` and `b` swapping names: the record a batch writes before moving them.
fn swap() -> Vec<Entry> {
    vec![
        entry(b"a", b".rustphin-part-rename-0", b"b"),
        entry(b"b", b".rustphin-part-rename-1", b"a"),
    ]
}

/// The record of `entries` in `directory`, declaring `count` slots. Each
/// slot carries the details of its origin as it is now (or of `a`, for an
/// origin that does not exist), so a well-formed record recovers.
fn encode(directory: &Path, count: u64, entries: &[Entry]) -> Vec<u8> {
    let root = RealFs.info(&directory.join(".")).unwrap();
    let mut bytes = b"RUSTPHIN-RENAME\x01".to_vec();
    bytes.extend_from_slice(&root.device.to_le_bytes());
    bytes.extend_from_slice(&root.inode.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for entry in entries {
        for name in [&entry.origin, &entry.temporary, &entry.target] {
            bytes.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(name);
        }
        bytes.push(entry.kind);
        let info = RealFs
            .info(&directory.join(OsStr::from_bytes(&entry.origin)))
            .or_else(|_| RealFs.info(&directory.join("a")))
            .unwrap();
        for n in [info.device, info.inode, info.size] {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        bytes.extend_from_slice(&info.modified_ns.to_le_bytes());
        bytes.extend_from_slice(&info.mode.to_le_bytes());
    }
    bytes
}

/// A folder holding `a` and `b` and the record `make` writes for it.
fn folder_with_record(make: impl FnOnce(&Path) -> Vec<u8>) -> Dirs {
    let d = dirs();
    file(d.src.join("a"), "A");
    file(d.src.join("b"), "B");
    let bytes = make(&d.src);
    fs::write(d.src.join(RENAME_JOURNAL), bytes).unwrap();
    d
}

/// Recovery refuses the record as invalid data and changes nothing.
fn assert_refused(make: impl FnOnce(&Path) -> Vec<u8>) {
    let d = folder_with_record(make);
    let before = tree(&d.src);

    let error = recover_rename_batch(&RealFs, &d.src).unwrap_err();

    assert!(
        matches!(&error, OpError::Io { source, .. } if source.kind() == io::ErrorKind::InvalidData),
        "{error:?}"
    );
    assert_eq!(tree(&d.src), before);
}

#[test]
fn a_well_formed_record_is_read_and_removed() {
    let d = folder_with_record(|dir| encode(dir, 2, &swap()));

    assert_eq!(recover_rename_batch(&RealFs, &d.src).unwrap(), 2);

    assert_eq!(fs::read(d.src.join("a")).unwrap(), b"A");
    assert_eq!(fs::read(d.src.join("b")).unwrap(), b"B");
    assert!(!d.src.join(RENAME_JOURNAL).exists());
}

/// One test per way a record is damaged.
macro_rules! refused {
    ($($name:ident: $make:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_refused($make);
        }
    )*};
}

refused! {
    a_truncated_record_is_refused: |dir: &Path| {
        let mut bytes = encode(dir, 2, &swap());
        bytes.pop();
        bytes
    };
    trailing_bytes_are_refused: |dir: &Path| {
        let mut bytes = encode(dir, 2, &swap());
        bytes.push(0);
        bytes
    };
    a_record_of_no_slots_is_refused: |dir: &Path| encode(dir, 0, &[]);
    a_count_beyond_the_slots_written_is_refused: |dir: &Path| encode(dir, 3, &swap());
    a_count_no_record_could_hold_is_refused: |dir: &Path| encode(dir, u64::MAX, &swap());
    a_name_longer_than_the_record_is_refused: |dir: &Path| {
        let mut bytes = encode(dir, 1, &swap()[..1]);
        // The first name's length, just after the 40-byte header.
        bytes[40..42].copy_from_slice(&u16::MAX.to_le_bytes());
        bytes
    };
    a_bad_kind_is_refused: |dir: &Path| {
        let mut entries = swap();
        entries[1].kind = 4;
        encode(dir, 2, &entries)
    };
    a_duplicate_origin_is_refused: |dir: &Path| encode(dir, 2, &[
        entry(b"a", b".rustphin-part-rename-0", b"b"),
        entry(b"a", b".rustphin-part-rename-1", b"c"),
    ]);
    a_duplicate_target_is_refused: |dir: &Path| encode(dir, 2, &[
        entry(b"a", b".rustphin-part-rename-0", b"c"),
        entry(b"b", b".rustphin-part-rename-1", b"c"),
    ]);
    a_duplicate_temporary_name_is_refused: |dir: &Path| encode(dir, 2, &[
        entry(b"a", b".rustphin-part-rename-0", b"b"),
        entry(b"b", b".rustphin-part-rename-0", b"a"),
    ]);
    an_entry_renamed_to_itself_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-0", b"a"),
    ]);
    a_temporary_name_without_the_prefix_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b"7", b"b"),
    ]);
    a_temporary_name_without_a_number_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-", b"b"),
    ]);
    a_temporary_name_ending_in_other_than_digits_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-1x", b"b"),
    ]);
    a_temporary_name_that_is_a_source_is_refused: |dir: &Path| encode(dir, 2, &[
        entry(b".rustphin-part-rename-5", b".rustphin-part-rename-0", b"c"),
        entry(b"b", b".rustphin-part-rename-5", b"a"),
    ]);
    a_temporary_name_that_is_a_target_is_refused: |dir: &Path| encode(dir, 2, &[
        entry(b"a", b".rustphin-part-rename-0", b".rustphin-part-rename-1"),
        entry(b"b", b".rustphin-part-rename-1", b"a"),
    ]);
    a_name_holding_a_slash_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-0", b"sub/b"),
    ]);
    an_empty_name_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-0", b""),
    ]);
    a_parent_folder_name_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"..", b".rustphin-part-rename-0", b"b"),
    ]);
    the_records_own_name_is_refused: |dir: &Path| encode(dir, 1, &[
        entry(b"a", b".rustphin-part-rename-0", RENAME_JOURNAL.as_bytes()),
    ]);
}

/// Applies `edits` (offset, byte, what) to `bytes`: 0 overwrites the byte
/// at the offset, 1 inserts one there, 2 removes it.
fn damage(mut bytes: Vec<u8>, edits: &[(prop::sample::Index, u8, u8)]) -> Vec<u8> {
    for (at, byte, what) in edits {
        if bytes.is_empty() {
            break;
        }
        let at = at.index(bytes.len());
        match what {
            0 => bytes[at] = *byte,
            1 => bytes.insert(at, *byte),
            _ => {
                bytes.remove(at);
            }
        }
    }
    bytes
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// A damaged record never panics and never moves an entry: either it is
    /// refused and kept, or it still describes `a` and `b` where they are.
    #[test]
    fn a_damaged_record_never_moves_an_entry(
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>(), 0u8..3), 1..6),
    ) {
        let d = folder_with_record(|dir| damage(encode(dir, 2, &swap()), &edits));
        let record = fs::read(d.src.join(RENAME_JOURNAL)).unwrap();

        let result = recover_rename_batch(&RealFs, &d.src);

        prop_assert_eq!(fs::read(d.src.join("a")).unwrap(), b"A");
        prop_assert_eq!(fs::read(d.src.join("b")).unwrap(), b"B");
        if result.is_err() {
            prop_assert_eq!(fs::read(d.src.join(RENAME_JOURNAL)).unwrap(), record);
        }
    }

    /// Any bytes after a header that matches the folder are read without
    /// panicking, and refused without a change.
    #[test]
    fn any_bytes_after_a_good_header_are_refused_without_a_change(
        tail in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let d = folder_with_record(|dir| {
            let mut bytes = encode(dir, 0, &[]);
            bytes.truncate(40);
            bytes.extend_from_slice(&tail);
            bytes
        });
        let before = tree(&d.src);

        let result = recover_rename_batch(&RealFs, &d.src);

        prop_assert!(result.is_err());
        prop_assert_eq!(tree(&d.src), before);
    }
}
