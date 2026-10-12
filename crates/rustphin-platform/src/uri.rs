//! `file://` URIs as file managers exchange them on the clipboard, in drags
//! and in trash records.

use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

/// Percent-encodes a path as in a URI: unreserved characters and `/` stay,
/// every other byte becomes `%XX`. Works for names that are not UTF-8.
pub fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || b"-_.~/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Reverses [`percent_encode`]. A `%` not followed by two hex digits is kept
/// as it is.
pub fn percent_decode(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut rest = text;
    while let [byte, tail @ ..] = rest {
        let hex = |digit: u8| (digit as char).to_digit(16).map(|value| value as u8);
        match (byte, tail) {
            (b'%', [high, low, after @ ..]) if hex(*high).is_some() && hex(*low).is_some() => {
                out.push(hex(*high).unwrap_or(0) << 4 | hex(*low).unwrap_or(0));
                rest = after;
            }
            _ => {
                out.push(*byte);
                rest = tail;
            }
        }
    }
    out
}

/// The `file://` URI of an absolute path.
pub fn file_uri(path: &Path) -> String {
    format!("file://{}", percent_encode(path.as_os_str().as_bytes()))
}

/// The `file://` URI of an absolute path as GLib and Qt write it: the
/// characters a URI path may contain literally (`!$&'()*+,;=:@`) are left
/// alone. The thumbnail cache is keyed by a hash of exactly this form, so it
/// must match what other applications produce.
pub fn canonical_file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~/!$&'()*+,;=:@".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The local path a `file:` URI names, or `None` for other schemes, other
/// hosts and relative paths.
pub fn path_from_uri(uri: &[u8]) -> Option<PathBuf> {
    let rest = uri.trim_ascii();
    let rest = rest
        .strip_prefix(b"file:")
        .or_else(|| rest.strip_prefix(b"FILE:"))?;
    // `file:///path`, `file://localhost/path` and the short `file:/path`.
    let path = match rest.strip_prefix(b"//") {
        Some(with_host) => {
            let slash = with_host.iter().position(|&byte| byte == b'/')?;
            let host = &with_host[..slash];
            if !host.is_empty() && !host.eq_ignore_ascii_case(b"localhost") {
                return None;
            }
            &with_host[slash..]
        }
        None => rest,
    };
    let decoded = percent_decode(path);
    (decoded.first() == Some(&b'/') && !decoded.contains(&0))
        .then(|| PathBuf::from(OsString::from_vec(decoded)))
}

/// A `text/uri-list` body for `paths`: one URI per line, lines ended by CR LF.
pub fn uri_list(paths: &[PathBuf]) -> String {
    paths.iter().map(|path| file_uri(path) + "\r\n").collect()
}

/// The local paths in a `text/uri-list` body. Comment lines and URIs that do
/// not name local files are skipped.
pub fn paths_from_uri_list(body: &[u8]) -> Vec<PathBuf> {
    body.split(|&byte| byte == b'\n')
        .map(|line| line.trim_ascii())
        .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
        .filter_map(path_from_uri)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_non_ascii_and_bytes_outside_utf8_survive_the_round_trip() {
        let paths = vec![
            PathBuf::from("/home/u/My Documents/résumé 100%.pdf"),
            PathBuf::from("/tmp/日本語/#hash?query&amp"),
            PathBuf::from(OsString::from_vec(b"/tmp/bad\xff\nname.txt".to_vec())),
        ];

        let body = uri_list(&paths);

        assert_eq!(
            body.lines().next(),
            Some("file:///home/u/My%20Documents/r%C3%A9sum%C3%A9%20100%25.pdf")
        );
        assert!(
            body.ends_with("file:///tmp/bad%FF%0Aname.txt\r\n"),
            "{body:?}"
        );
        assert!(body.is_ascii());
        assert_eq!(paths_from_uri_list(body.as_bytes()), paths);
    }

    #[test]
    fn lists_from_other_applications_are_read() {
        // Unencoded UTF-8 (as some applications send), a host, the short
        // form, a comment, bare LF line ends and a trailing NUL-free blank.
        let body = "# copied\nfile:///tmp/a b.txt\r\nfile://localhost/tmp/c\nfile:/tmp/d\n\
                    file:///tmp/%e6%97%a5.txt\n\n";
        assert_eq!(
            paths_from_uri_list(body.as_bytes()),
            ["/tmp/a b.txt", "/tmp/c", "/tmp/d", "/tmp/日.txt"].map(PathBuf::from)
        );
    }

    #[test]
    fn uris_that_do_not_name_local_files_are_skipped() {
        let body = "https://example.org/x\nfile://otherhost/tmp/x\nfile:relative\n\
                    sftp://host/x\nfile:///ok\nfile:///nul%00byte\n";
        assert_eq!(paths_from_uri_list(body.as_bytes()), [PathBuf::from("/ok")]);
    }

    #[test]
    fn the_canonical_form_escapes_only_what_a_uri_path_cannot_hold() {
        assert_eq!(
            canonical_file_uri(Path::new("/home/u/My Pics/a+b (1)&c=d,e;f@g:h!.png")),
            "file:///home/u/My%20Pics/a+b%20(1)&c=d,e;f@g:h!.png"
        );
        assert_eq!(
            canonical_file_uri(Path::new("/tmp/é#?%[]")),
            "file:///tmp/%C3%A9%23%3F%25%5B%5D"
        );
    }

    #[test]
    fn a_stray_percent_sign_is_kept() {
        assert_eq!(percent_decode(b"100%zz%4"), b"100%zz%4");
        assert_eq!(percent_decode(b"%41%7a"), b"Az");
    }
}
