#![no_main]

use std::{fs, os::unix::ffi::OsStrExt, path::PathBuf};

use libfuzzer_sys::fuzz_target;
use rustphin_platform::apps::{LaunchKind, command_lines, launch_kind};

fuzz_target!(|data: &[u8]| {
    let file = tempfile::Builder::new()
        .suffix(".desktop")
        .tempfile()
        .unwrap();
    fs::write(file.path(), data).unwrap();
    // Classification parses a shortcut but must never launch it.
    assert!(matches!(
        launch_kind(file.path()),
        LaunchKind::Desktop { .. } | LaunchKind::Unreadable(_)
    ));
    assert_eq!(fs::read(file.path()).unwrap(), data);
    if let Ok(text) = std::str::from_utf8(data)
        && let Some(lines) = command_lines(text, &[PathBuf::from("/fuzz/a")], "Fuzz", None)
    {
        for argv in lines {
            for arg in argv {
                assert!(!arg.as_bytes().contains(&0));
            }
        }
    }
});
