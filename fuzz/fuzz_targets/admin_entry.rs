#![no_main]

use libfuzzer_sys::fuzz_target;
use rustphin_platform::admin::{AdminEntry, AdminKind, decode_entry};

// Entries from KDE's kio-admin helper (KIO `UDSEntry` in Qt's `QDataStream`
// layout). Any input must decode or be refused without panicking; an entry
// that is accepted must name something that can be addressed safely.
fuzz_target!(|data: &[u8]| {
    let Ok(entry) = decode_entry(data) else {
        return;
    };
    if let Ok(entry) = AdminEntry::from_uds(&entry) {
        assert!(!entry.name.is_empty());
        assert!(!entry.name.contains('/'));
        assert!(entry.permissions <= 0o7777);
        if entry.link_target.is_some() {
            assert_eq!(entry.kind, AdminKind::Symlink);
        }
        for (_, nanos) in [entry.modified, entry.accessed, entry.created]
            .into_iter()
            .flatten()
        {
            assert!(nanos < 1_000_000_000);
        }
    }
});
