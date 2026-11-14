//! The W-114a spike's bridge: one list model over the core's snapshots.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel;

        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;

        include!("helpers.h");
        fn spike_blur() -> bool;
        fn spike_copy(paths: &QStringList, cut: bool);
        fn spike_paste() -> QStringList;
        fn spike_keep_running();
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qproperty(i32, count)]
        #[qproperty(bool, loading)]
        #[qproperty(QString, status)]
        type EntryModel = super::EntryModelRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut EntryModel>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut EntryModel>);

        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &EntryModel, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &EntryModel, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &EntryModel) -> QHash_i32_QByteArray;

        #[qinvokable]
        fn open(self: Pin<&mut EntryModel>, path: &QString);
        #[qinvokable]
        #[cxx_name = "sortBy"]
        fn sort_by(self: Pin<&mut EntryModel>, key: i32, descending: bool);
        #[qinvokable]
        fn swapped(self: Pin<&mut EntryModel>);
        #[qinvokable]
        fn tick(self: Pin<&mut EntryModel>);
        #[qinvokable]
        fn blur(self: Pin<&mut EntryModel>) -> bool;
        #[qinvokable]
        fn copy(self: Pin<&mut EntryModel>, row: i32);
        #[qinvokable]
        fn paste(self: Pin<&mut EntryModel>) -> QString;
        #[qinvokable]
        #[cxx_name = "keepRunning"]
        fn keep_running(self: Pin<&mut EntryModel>);
    }

    impl cxx_qt::Threading for EntryModel {}
}

use std::{
    pin::Pin,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use rustphin_core::{DirectoryModel, ModelEvent, RealFs, SortKey, Snapshot, ViewOptions};

const NAME: i32 = 0x100;
const SIZE: i32 = 0x101;
const MODIFIED: i32 = 0x102;
const FOLDER: i32 = 0x103;
const URI: i32 = 0x104;

/// Frames the scroll benchmark counts.
const BENCH_FRAMES: usize = 1_200;

pub struct EntryModelRust {
    count: i32,
    loading: bool,
    status: QString,
    model: Option<DirectoryModel>,
    snapshot: Option<Arc<Snapshot>>,
    /// When the current request was made, until its first frame with rows.
    asked: Option<(Instant, &'static str, u64, bool)>,
    frames: Vec<Instant>,
    first_frame_logged: bool,
}

impl Default for EntryModelRust {
    fn default() -> Self {
        Self {
            count: 0,
            loading: false,
            status: QString::default(),
            model: None,
            snapshot: None,
            asked: None,
            frames: Vec::new(),
            first_frame_logged: false,
        }
    }
}

fn wall() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.)
}

fn own_memory_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("RssAnon:"))
                .and_then(|line| line.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

fn human(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KiB", size as f64 / 1024.)
    } else {
        format!("{:.1} MiB", size as f64 / 1048576.)
    }
}

impl qobject::EntryModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(snapshot) = self.rust().snapshot.as_ref() else {
            return QVariant::default();
        };
        let row = index.row() as usize;
        if row >= snapshot.len() {
            return QVariant::default();
        }
        match role {
            NAME => QVariant::from(&QString::from(
                String::from_utf8_lossy(snapshot.name(row)).as_ref(),
            )),
            SIZE => {
                let text = match snapshot.metadata(row) {
                    Some(_) if snapshot.is_container(row) => "—".to_owned(),
                    Some(meta) => human(meta.size),
                    None => String::new(),
                };
                QVariant::from(&QString::from(text.as_str()))
            }
            MODIFIED => {
                let text = snapshot
                    .metadata(row)
                    .map(|meta| {
                        let days = meta.modified.div_euclid(86_400);
                        let secs = meta.modified.rem_euclid(86_400);
                        format!("day {days} {:02}:{:02}", secs / 3600, secs % 3600 / 60)
                    })
                    .unwrap_or_default();
                QVariant::from(&QString::from(text.as_str()))
            }
            FOLDER => QVariant::from(&snapshot.is_container(row)),
            URI => {
                let path = snapshot.entry_path(row);
                QVariant::from(&QString::from(
                    format!("file://{}", path.to_string_lossy()).as_str(),
                ))
            }
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        roles.insert(NAME, QByteArray::from("name"));
        roles.insert(SIZE, QByteArray::from("size"));
        roles.insert(MODIFIED, QByteArray::from("modified"));
        roles.insert(FOLDER, QByteArray::from("folder"));
        roles.insert(URI, QByteArray::from("uri"));
        roles
    }

    fn open(mut self: Pin<&mut Self>, path: &QString) {
        if self.rust().model.is_none() {
            let Ok((model, events)) = DirectoryModel::new(Arc::new(RealFs)) else {
                eprintln!("spike: the model could not start");
                return;
            };
            self.as_mut().rust_mut().model = Some(model);
            let thread = self.qt_thread();
            std::thread::spawn(move || {
                while let Ok(event) = events.recv() {
                    if let ModelEvent::Snapshot(snapshot) = event {
                        // Only the newest snapshot matters; older queued ones
                        // are replaced on arrival.
                        if thread
                            .queue(move |mut model| model.as_mut().show(snapshot))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            });
        }
        let path = path.to_string();
        let mut rust = self.as_mut().rust_mut();
        let start = Instant::now();
        if let Some(model) = rust.model.as_mut() {
            let generation = model.open(std::path::Path::new(&path));
            rust.asked = Some((start, "first entries", generation, false));
        }
    }

    fn sort_by(mut self: Pin<&mut Self>, key: i32, descending: bool) {
        let sort = match key {
            1 => SortKey::Size,
            2 => SortKey::Modified,
            3 => SortKey::Type,
            _ => SortKey::Name,
        };
        let mut rust = self.as_mut().rust_mut();
        let start = Instant::now();
        if let Some(model) = rust.model.as_mut()
            && let Some(generation) = model.set_options(ViewOptions {
                details: true,
                sort,
                descending,
                ..ViewOptions::default()
            })
        {
            rust.asked = Some((start, "re-sort shown", generation, true));
        }
    }

    fn show(mut self: Pin<&mut Self>, snapshot: Arc<Snapshot>) {
        let rows = snapshot.len().min(i32::MAX as usize) as i32;
        let loading = snapshot.is_loading();
        self.as_mut().begin_reset_model();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.snapshot = Some(snapshot);
            rust.count = rows;
        }
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        if *self.loading() != loading {
            self.as_mut().set_loading(loading);
        }
        let text = format!("{rows} entries{}", if loading { ", loading" } else { "" });
        self.as_mut().set_status(QString::from(text.as_str()));
    }

    /// Called after every frame reaches the screen.
    fn swapped(mut self: Pin<&mut Self>) {
        let rows = *self.count();
        let mut rust = self.as_mut().rust_mut();
        if !rust.first_frame_logged {
            rust.first_frame_logged = true;
            eprintln!(
                "spike: first frame at {:.3} (unix), own memory {} kB",
                wall(),
                own_memory_kb()
            );
        }
        let shown = rust
            .snapshot
            .as_ref()
            .map(|s| (s.generation(), s.is_loading()));
        if rows > 0
            && let Some((asked, what, generation, complete)) = rust.asked
            && let Some((shown_generation, loading)) = shown
            && shown_generation >= generation
            && !(complete && loading)
        {
            rust.asked = None;
            eprintln!(
                "spike: {what}: {:.1} ms to the frame showing {rows} rows{}, own memory {} kB",
                asked.elapsed().as_secs_f64() * 1e3,
                if loading { " (still loading)" } else { "" },
                own_memory_kb()
            );
        }
    }

    /// Called on every animation frame while the scroll benchmark runs.
    fn tick(mut self: Pin<&mut Self>) {
        let mut rust = self.as_mut().rust_mut();
        rust.frames.push(Instant::now());
        if rust.frames.len() == BENCH_FRAMES + 1 {
            let intervals: Vec<f64> = rust
                .frames
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).as_secs_f64() * 1e3)
                .collect();
            let mut sorted = intervals.clone();
            sorted.sort_by(f64::total_cmp);
            let median = sorted[sorted.len() / 2];
            let dropped = intervals.iter().filter(|&&ms| ms > median * 1.5).count();
            let percent = dropped as f64 * 100. / intervals.len() as f64;
            eprintln!(
                "spike: scroll benchmark result: {} intervals, median {:.2} ms, max {:.2} ms, {dropped} dropped ({percent:.2}%) => {}",
                intervals.len(),
                median,
                sorted[sorted.len() - 1],
                if percent <= 5. { "PASS" } else { "FAIL" }
            );
            rust.frames.clear();
        }
    }

    fn blur(self: Pin<&mut Self>) -> bool {
        let available = qobject::spike_blur();
        eprintln!("spike: KWin blur effect available: {available}");
        available
    }

    fn copy(self: Pin<&mut Self>, row: i32) {
        let Some(snapshot) = self.rust().snapshot.as_ref() else {
            return;
        };
        let path = snapshot.entry_path(row as usize);
        let list: cxx_qt_lib::QStringList =
            std::iter::once(QString::from(path.to_string_lossy().as_ref())).collect();
        qobject::spike_copy(&list, false);
        eprintln!("spike: copied {}", path.display());
    }

    fn paste(self: Pin<&mut Self>) -> QString {
        let pasted = qobject::spike_paste();
        let list: cxx_qt_lib::QList<QString> = (&pasted).into();
        let text: Vec<String> = list.iter().map(|path| path.to_string()).collect();
        eprintln!("spike: clipboard holds {} path(s): {text:?}", text.len());
        QString::from(text.join("\n").as_str())
    }

    fn keep_running(self: Pin<&mut Self>) {
        qobject::spike_keep_running();
    }
}
