#![no_main]

use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
};

use libfuzzer_sys::fuzz_target;
use rustphin_core::display_name;
use rustphin_platform::uri::{
    canonical_file_uri, file_uri, path_from_uri, paths_from_uri_list, uri_list,
};

fuzz_target!(|data: &[u8]| {
    let mut bytes = vec![b'/'];
    bytes.extend(data.iter().copied().filter(|byte| *byte != 0));
    let path = PathBuf::from(OsString::from_vec(bytes));
    assert_eq!(
        path_from_uri(file_uri(&path).as_bytes()),
        Some(path.clone())
    );
    assert_eq!(
        path_from_uri(canonical_file_uri(&path).as_bytes()),
        Some(path.clone())
    );
    assert_eq!(
        paths_from_uri_list(uri_list(std::slice::from_ref(&path)).as_bytes()),
        [path]
    );
    assert!(!display_name(data).chars().any(char::is_control));
    for path in paths_from_uri_list(data) {
        assert!(path.is_absolute());
        assert!(!path.as_os_str().as_bytes().contains(&0));
    }
});
