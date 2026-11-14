//! Paths and names as they cross to QML and back (W-132). File names are
//! bytes, QML text is Unicode: a path goes over as its `file://` URI and a
//! bare name percent-encoded, both plain ASCII that keeps every byte. Lossy
//! text is for what the user reads only.

use std::path::{Path, PathBuf};

use cxx_qt_lib::{QList, QString, QStringList};
use rustphin_platform::uri::{file_uri, path_from_uri, percent_decode, percent_encode};

/// `path` for QML: its `file://` URI.
pub fn to_qml(path: &Path) -> QString {
    QString::from(&file_uri(path))
}

/// Paths for QML, each as its `file://` URI.
pub fn list_to_qml(paths: &[PathBuf]) -> QStringList {
    let list: QList<QString> = paths.iter().map(|path| to_qml(path)).collect();
    QStringList::from(&list)
}

/// The path QML handed back as a `file:` URI; none for anything else.
pub fn from_qml(uri: &QString) -> Option<PathBuf> {
    path_from_uri(uri.to_string().as_bytes())
}

/// Names in a folder for QML, each percent-encoded.
pub fn names_to_qml<'a>(names: impl IntoIterator<Item = &'a [u8]>) -> QStringList {
    let list: QList<QString> = names
        .into_iter()
        .map(|name| QString::from(&percent_encode(name)))
        .collect();
    QStringList::from(&list)
}

/// The names [`names_to_qml`] handed to QML, back as bytes.
pub fn names_from_qml(names: &QStringList) -> Vec<Vec<u8>> {
    QList::<QString>::from(names)
        .iter()
        .map(|name| percent_decode(name.to_string().as_bytes()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    use super::*;

    #[test]
    fn a_path_outside_utf8_comes_back_from_qml_byte_for_byte() {
        let path = Path::new(OsStr::from_bytes(b"/srv/caf\xe9/n\xe9 #1%.txt"));
        assert_eq!(from_qml(&to_qml(path)).as_deref(), Some(path));
        let list = list_to_qml(&[path.to_path_buf(), PathBuf::from("/srv")]);
        let back: Vec<Option<PathBuf>> =
            QList::<QString>::from(&list).iter().map(from_qml).collect();
        assert_eq!(
            back,
            [Some(path.to_path_buf()), Some(PathBuf::from("/srv"))]
        );
    }

    #[test]
    fn names_outside_utf8_come_back_from_qml_byte_for_byte() {
        let names: [&[u8]; 3] = [b"caf\xe9", b"50% off", b"plain.txt"];
        assert_eq!(names_from_qml(&names_to_qml(names)), names);
    }

    #[test]
    fn plain_text_is_not_taken_for_a_path() {
        assert_eq!(from_qml(&QString::from("/srv/data")), None);
        assert_eq!(from_qml(&QString::from("")), None);
    }
}
