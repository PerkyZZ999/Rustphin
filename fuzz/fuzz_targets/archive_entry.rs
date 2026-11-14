#![no_main]

use std::{
    ffi::OsStr,
    io::Read,
    ops::ControlFlow,
    os::unix::ffi::OsStrExt as _,
    path::{Component, Path},
};

use libfuzzer_sys::fuzz_target;
use rustphin_core::archive::{
    Format, NoForeignReader, entry_path, link_stays_inside, read_entries,
};

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
            // W-133: no `..` after a name, which a link made by the same
            // archive could lead somewhere else (`l/../..`).
            let parts: Vec<Component<'_>> =
                Path::new(OsStr::from_bytes(target)).components().collect();
            if let Some(first) = parts
                .iter()
                .position(|part| matches!(part, Component::Normal(_)))
            {
                assert!(
                    !parts[first..]
                        .iter()
                        .any(|part| matches!(part, Component::ParentDir))
                );
            }
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
        std::path::Path::new("fuzz"),
        Box::new(std::io::Cursor::new(rest.to_vec())),
        Some("password"),
        &NoForeignReader,
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
