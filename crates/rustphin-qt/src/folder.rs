//! One folder's entries as a QML list model (W-114c): rows are made on
//! demand from the core's newest snapshot, which arrives from the core's
//! own threads and is swapped in on the Qt thread (D-009). No file is read
//! here.

// cxx-qt generates the unsafe FFI glue between Rust and the C++ QObject
// for the list model; nothing in the bridge file is written by hand but
// declarations, and the code here stays under the lint (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "folder_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{pin::Pin, sync::Arc};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use jiff::tz::TimeZone;
use rustphin_core::{DirectoryModel, ModelEvent, RealFs, Snapshot, SortKey, ViewOptions};
use rustphin_view::format;

/// The roles QML reads, after Qt's own (`Qt::UserRole` is 0x100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Name = 0x100,
    Size,
    Modified,
    Folder,
    Hidden,
}

impl Role {
    const ALL: [Role; 5] = [
        Role::Name,
        Role::Size,
        Role::Modified,
        Role::Folder,
        Role::Hidden,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Name => "name",
            Role::Size => "size",
            Role::Modified => "modified",
            Role::Folder => "folder",
            Role::Hidden => "hidden",
        }
    }
}

/// The Rust side of `FolderModel`.
pub struct FolderModelRust {
    count: i32,
    loading: bool,
    location: QString,
    summary: QString,
    problem: QString,
    model: Option<DirectoryModel>,
    snapshot: Option<Arc<Snapshot>>,
    options: ViewOptions,
    zone: TimeZone,
}

impl Default for FolderModelRust {
    fn default() -> Self {
        Self {
            count: 0,
            loading: false,
            location: QString::default(),
            summary: QString::default(),
            problem: QString::default(),
            model: None,
            snapshot: None,
            options: ViewOptions {
                details: true,
                ..ViewOptions::default()
            },
            zone: TimeZone::system(),
        }
    }
}

/// The status line under the list: "3 items", with "loading" while the
/// listing is not complete.
fn summary_text(len: usize, loading: bool) -> String {
    let items = format!(
        "{} {}",
        format::count(len),
        format::plural(len, "item", "items")
    );
    if loading {
        format!("{items}, loading…")
    } else {
        items
    }
}

/// The sort a column asks for.
fn sort_key(column: i32) -> SortKey {
    match column {
        1 => SortKey::Size,
        2 => SortKey::Modified,
        3 => SortKey::Type,
        _ => SortKey::Name,
    }
}

impl qobject::FolderModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let (Some(snapshot), Ok(row), Some(role)) = (
            self.rust().snapshot.as_ref(),
            usize::try_from(index.row()),
            Role::from_number(role),
        ) else {
            return QVariant::default();
        };
        if row >= snapshot.len() {
            return QVariant::default();
        }
        let text = |words: &str| QVariant::from(&QString::from(words));
        match role {
            Role::Name => text(&String::from_utf8_lossy(snapshot.name(row))),
            Role::Size => text(&format::size_column(
                snapshot.is_container(row),
                snapshot.metadata(row).map(|meta| meta.size),
            )),
            Role::Modified => text(
                &snapshot
                    .metadata(row)
                    .map(|meta| format::timestamp(meta.modified, &self.rust().zone))
                    .unwrap_or_default(),
            ),
            Role::Folder => QVariant::from(&snapshot.is_container(row)),
            Role::Hidden => QVariant::from(&snapshot.is_hidden(row)),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    fn open(mut self: Pin<&mut Self>, path: &QString) {
        if self.rust().model.is_none() {
            match DirectoryModel::new(Arc::new(RealFs)) {
                Ok((mut model, events)) => {
                    // Sizes and dates are read for the details view.
                    model.set_options(self.rust().options);
                    self.as_mut().rust_mut().model = Some(model);
                    let thread = self.qt_thread();
                    std::thread::spawn(move || {
                        // Ends when the model, and with it the sender, is gone.
                        while let Ok(event) = events.recv() {
                            let sent = match event {
                                ModelEvent::Snapshot(snapshot) => {
                                    thread.queue(move |model| model.show(snapshot))
                                }
                                ModelEvent::Failed { error, .. } => {
                                    let words = error.to_string();
                                    thread.queue(move |model| model.fail(&words))
                                }
                                ModelEvent::Moved { .. } => continue,
                            };
                            if sent.is_err() {
                                break;
                            }
                        }
                    });
                }
                Err(error) => {
                    self.fail(&error.to_string());
                    return;
                }
            }
        }
        let path = std::path::PathBuf::from(path.to_string());
        self.as_mut().rust_mut().location = QString::from(path.to_string_lossy().as_ref());
        self.as_mut().location_changed();
        if let Some(model) = self.as_mut().rust_mut().model.as_mut() {
            model.open(&path);
        }
    }

    fn sort_by(mut self: Pin<&mut Self>, column: i32, descending: bool) {
        let options = ViewOptions {
            sort: sort_key(column),
            descending,
            ..self.rust().options
        };
        let mut rust = self.as_mut().rust_mut();
        rust.options = options;
        if let Some(model) = rust.model.as_mut() {
            model.set_options(options);
        }
    }

    /// Swaps in the newest snapshot.
    fn show(mut self: Pin<&mut Self>, snapshot: Arc<Snapshot>) {
        let rows = i32::try_from(snapshot.len()).unwrap_or(i32::MAX);
        let loading = snapshot.is_loading();
        let summary = summary_text(snapshot.len(), loading);
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.snapshot = Some(snapshot);
            rust.count = rows;
            rust.loading = loading;
        }
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        self.as_mut().loading_changed();
        self.as_mut().rust_mut().summary = QString::from(summary.as_str());
        self.as_mut().summary_changed();
        if !self.problem().is_empty() {
            self.as_mut().rust_mut().problem = QString::default();
            self.as_mut().problem_changed();
        }
    }

    /// Says why the folder could not be shown.
    fn fail(mut self: Pin<&mut Self>, words: &str) {
        self.as_mut().rust_mut().problem =
            QString::from(format!("This folder could not be opened: {words}.").as_str());
        self.as_mut().problem_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_found_by_number_and_named_once() {
        for role in Role::ALL {
            assert_eq!(Role::from_number(role as i32), Some(role));
        }
        assert_eq!(Role::from_number(0), None);
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.dedup();
        assert_eq!(names.len(), Role::ALL.len());
    }

    #[test]
    fn the_summary_counts_items_and_says_when_loading() {
        assert_eq!(summary_text(1, false), "1 item");
        assert_eq!(summary_text(3, true), "3 items, loading…");
    }

    #[test]
    fn columns_sort_by_name_size_date_and_type() {
        assert_eq!(sort_key(0), SortKey::Name);
        assert_eq!(sort_key(1), SortKey::Size);
        assert_eq!(sort_key(2), SortKey::Modified);
        assert_eq!(sort_key(3), SortKey::Type);
        assert_eq!(sort_key(9), SortKey::Name);
    }
}
