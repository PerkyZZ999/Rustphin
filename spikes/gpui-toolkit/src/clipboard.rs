//! DISPOSABLE SPIKE — file clipboard outside GPUI, over the data-control protocol
//! (`ext_data_control_v1` on KWin) through wl-clipboard-rs.
//!
//! Offers the formats file managers exchange: `text/uri-list`, GNOME's
//! `x-special/gnome-copied-files` and KDE's `application/x-kde-cutselection`.
//! URIs are not percent-encoded here; production code must encode them.

use std::{io::Read, path::PathBuf};

use wl_clipboard_rs::{copy, paste};

const URI_LIST: &str = "text/uri-list";
const KDE_CUT: &str = "application/x-kde-cutselection";
const GNOME_FILES: &str = "x-special/gnome-copied-files";

pub fn write_files(paths: &[PathBuf], cut: bool) -> Result<(), String> {
    let uris: Vec<String> = paths.iter().map(|p| format!("file://{}", p.display())).collect();
    let source = |mime: &str, data: String| copy::MimeSource {
        source: copy::Source::Bytes(data.into_bytes().into()),
        mime_type: copy::MimeType::Specific(mime.to_string()),
    };
    let verb = if cut { "cut" } else { "copy" };
    copy::Options::new()
        .copy_multi(vec![
            source(URI_LIST, uris.join("\r\n") + "\r\n"),
            source(KDE_CUT, if cut { "1" } else { "0" }.to_string()),
            source(GNOME_FILES, format!("{verb}\n{}", uris.join("\n"))),
            source(
                "text/plain;charset=utf-8",
                paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"),
            ),
        ])
        .map_err(|e| e.to_string())
}

fn read_mime(mime: &str) -> Option<String> {
    let (mut pipe, _) = paste::get_contents(
        paste::ClipboardType::Regular,
        paste::Seat::Unspecified,
        paste::MimeType::Specific(mime),
    )
    .ok()?;
    let mut data = String::new();
    pipe.read_to_string(&mut data).ok()?;
    Some(data)
}

/// Returns the file paths on the clipboard and whether they were cut.
pub fn read_files() -> Option<(Vec<PathBuf>, bool)> {
    let paths = read_mime(URI_LIST)?
        .lines()
        .filter_map(|line| line.trim().strip_prefix("file://").map(PathBuf::from))
        .collect();
    let cut = read_mime(KDE_CUT).is_some_and(|flag| flag.trim() == "1");
    Some((paths, cut))
}
