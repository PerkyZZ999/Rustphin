#![no_main]

use std::fs;

use libfuzzer_sys::fuzz_target;
use rustphin_platform::thumbnails::{ThumbSize, ThumbnailCache};

fuzz_target!(|data: &[u8]| {
    let scratch = tempfile::tempdir().unwrap();
    let source = scratch.path().join("input");
    fs::write(&source, data).unwrap();
    let cache = ThumbnailCache::new(scratch.path().join("cache"));
    match cache.generate(&source, 1, ThumbSize::Normal) {
        Ok(path) => assert_eq!(cache.lookup(&source, 1, ThumbSize::Normal), Some(path)),
        Err(_) => assert!(cache.has_failed(&source, 1)),
    }
    assert_eq!(fs::read(&source).unwrap(), data);
});
