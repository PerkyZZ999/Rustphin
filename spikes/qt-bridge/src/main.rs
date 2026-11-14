//! W-114a spike (throwaway): the same list through QtBridge for Rust 0.4.
//! QtBridge's list model hands out references to stored items, so every
//! snapshot is turned into one `Row` per entry on a worker thread.

use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use qtbridge::{QApp, QListModel, QListModelBase, QModelItem, QmlObject, qobject};
use rustphin_core::{DirectoryModel, ModelEvent, RealFs, SortKey, ViewOptions};

#[derive(QModelItem, Default, Clone)]
pub struct Row {
    name: String,
    folder: bool,
}

#[derive(Default)]
pub struct Entries {
    rows: Vec<Row>,
    next: Vec<Row>,
    generation: u64,
    loading: bool,
    model: Option<DirectoryModel>,
    asked: Option<(Instant, &'static str, u64, bool)>,
    frames: Vec<Instant>,
    first_frame_logged: bool,
}

fn own_memory_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("RssAnon:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

#[qobject(Base = QListModel)]
impl Entries {
    #[qslot]
    fn open(&mut self, path: String) {
        if self.model.is_none() {
            let Ok((model, events)) = DirectoryModel::new(Arc::new(RealFs)) else {
                return;
            };
            self.model = Some(model);
            let handle = self.get_thread_handle();
            std::thread::spawn(move || {
                while let Ok(event) = events.recv() {
                    if let ModelEvent::Snapshot(snapshot) = event {
                        let rows: Vec<Row> = (0..snapshot.len())
                            .map(|row| Row {
                                name: String::from_utf8_lossy(snapshot.name(row)).into_owned(),
                                folder: snapshot.is_container(row),
                            })
                            .collect();
                        let generation = snapshot.generation();
                        let loading = snapshot.is_loading();
                        handle.invoke_mut(move |entries: &mut Entries| {
                            entries.next = rows;
                            entries.generation = generation;
                            entries.loading = loading;
                            entries.reset();
                        });
                    }
                }
            });
        }
        let start = Instant::now();
        if let Some(model) = self.model.as_mut() {
            let generation = model.open(std::path::Path::new(&path));
            self.asked = Some((start, "first entries", generation, false));
        }
    }

    #[qslot]
    fn sort_by(&mut self, key: i32) {
        let sort = match key {
            1 => SortKey::Size,
            2 => SortKey::Modified,
            3 => SortKey::Type,
            _ => SortKey::Name,
        };
        let start = Instant::now();
        if let Some(model) = self.model.as_mut()
            && let Some(generation) = model.set_options(ViewOptions {
                details: true,
                sort,
                ..ViewOptions::default()
            })
        {
            self.asked = Some((start, "re-sort shown", generation, true));
        }
    }

    #[qslot]
    fn count(&self) -> i32 {
        self.rows.len() as i32
    }

    #[qslot]
    fn settled(&self) -> bool {
        !self.loading
    }

    #[qslot]
    fn swapped(&mut self) {
        if !self.first_frame_logged {
            self.first_frame_logged = true;
            let wall = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.);
            eprintln!(
                "spike: first frame at {wall:.3} (unix), own memory {} kB",
                own_memory_kb()
            );
        }
        if !self.rows.is_empty()
            && let Some((asked, what, generation, complete)) = self.asked
            && self.generation >= generation
            && !(complete && self.loading)
        {
            self.asked = None;
            eprintln!(
                "spike: {what}: {:.1} ms to the frame showing {} rows{}, own memory {} kB",
                asked.elapsed().as_secs_f64() * 1e3,
                self.rows.len(),
                if self.loading { " (still loading)" } else { "" },
                own_memory_kb()
            );
        }
    }

    #[qslot]
    fn tick(&mut self) {
        self.frames.push(Instant::now());
        if self.frames.len() == 1_201 {
            let intervals: Vec<f64> = self
                .frames
                .windows(2)
                .map(|p| (p[1] - p[0]).as_secs_f64() * 1e3)
                .collect();
            let mut sorted = intervals.clone();
            sorted.sort_by(f64::total_cmp);
            let median = sorted[sorted.len() / 2];
            let dropped = intervals.iter().filter(|&&ms| ms > median * 1.5).count();
            let percent = dropped as f64 * 100. / intervals.len() as f64;
            eprintln!(
                "spike: scroll benchmark result: {} intervals, median {median:.2} ms, max {:.2} ms, {dropped} dropped ({percent:.2}%) => {}",
                intervals.len(),
                sorted[sorted.len() - 1],
                if percent <= 5. { "PASS" } else { "FAIL" }
            );
            self.frames.clear();
        }
    }
}

impl QListModel for Entries {
    type Item = Row;

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    fn reset_unnotified(&mut self) {
        self.rows = std::mem::take(&mut self.next);
    }
}

fn main() {
    eprintln!("spike: start");
    let mut app = QApp::new();
    eprintln!("spike: app made");
    app.register::<Entries>();
    app.load_qml(include_bytes!("../qml/Main.qml"));
    eprintln!("spike: qml loaded");
    let code = app.run();
    eprintln!("spike: event loop ended with {code}");
}
