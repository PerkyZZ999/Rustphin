#![no_main]

use std::{io::Read, ops::ControlFlow, path::Component};

use libfuzzer_sys::fuzz_target;
use rustphin_core::archive::{Format, entry_path, link_stays_inside, read_entries};

// Archive entries are untrusted (W-111): an accepted name must stay inside
// the destination, an accepted link must not lead out of it, and any bytes
// handed to a reader must be read or refused without panicking.
fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    if selector < 128 {
        let (name, target) = rest.split_at(rest.len() / 2);
        let Ok(path) = entry_path(name) else {
            return;
        };
        assert!(path.is_relative());
        assert!(
            path.components()
                .all(|part| matches!(part, Component::Normal(_)))
        );
        if link_stays_inside(&path, target) {
            assert!(!target.starts_with(b"/"));
        }
        return;
    }
    let formats = [
        Format::Zip,
        Format::Tar,
        Format::TarGz,
        Format::TarBz2,
        Format::TarXz,
        Format::TarZst,
        Format::SevenZ,
    ];
    let format = formats[usize::from(selector) % formats.len()];
    let _ = read_entries(
        format,
        Box::new(std::io::Cursor::new(rest.to_vec())),
        Some("password"),
        &mut |_, _| ControlFlow::Continue(()),
        &mut |header, content| {
            if let Ok(path) = entry_path(&header.name) {
                assert!(
                    path.components()
                        .all(|part| matches!(part, Component::Normal(_)))
                );
            }
            // Read a bounded amount: a hostile size must not make the
            // target allocate without limit.
            let mut sink = Vec::new();
            content.take(1 << 16).read_to_end(&mut sink)?;
            Ok(ControlFlow::Continue(()))
        },
    );
});
