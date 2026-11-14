//! How fast an operation goes and how long it still needs (W-149): a
//! smoothed speed from one sample a second, the time remaining and its
//! wording, the last minute's samples for the speed graph, and the transfer
//! dialog's cards. Free of real clocks: the time of every observation is
//! given by the caller ([`crate::operation_board`]).

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use rustphin_core::ops::Progress;
use serde::Serialize;

use crate::format::{self, count, plural};

/// How long one sample of the speed covers.
pub const SAMPLE: Duration = Duration::from_secs(1);
/// How many samples the speed graph keeps: the last minute.
pub const GRAPH_SAMPLES: usize = 60;
/// How many samples are needed before the time remaining is told.
pub const STEADY_SAMPLES: usize = 3;
/// The weight of the newest sample in the smoothed speed.
const SMOOTHING: f64 = 0.3;

/// What an operation's work is counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Bytes of file content.
    Bytes,
    /// Entries: an operation that copies no content (deleting, trashing).
    Items,
}

/// The work `progress` reports: the unit, how much is done and how much
/// there is. Nothing while no total is known.
fn work(progress: &Progress) -> Option<(Unit, u64, u64)> {
    if progress.bytes_total > 0 {
        Some((Unit::Bytes, progress.bytes_done, progress.bytes_total))
    } else if progress.items_total > 0 {
        Some((Unit::Items, progress.items_done, progress.items_total))
    } else {
        None
    }
}

/// What is being measured since `origin`.
#[derive(Debug)]
struct Measured {
    unit: Unit,
    origin: Instant,
    /// Whole samples closed since `origin`.
    closed: u64,
    /// The work done when the last sample closed.
    marked: u64,
    /// The work done at the latest report.
    latest: u64,
    total: u64,
}

/// Measures one operation's speed from its progress reports.
#[derive(Debug, Default)]
pub struct Meter {
    measured: Option<Measured>,
    samples: VecDeque<u64>,
    smoothed: Option<f64>,
}

impl Meter {
    /// A meter that has seen nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes a progress report that arrived at `now`. Measuring starts with
    /// the first report whose total is known, and starts again when the
    /// unit changes.
    pub fn observe(&mut self, progress: &Progress, now: Instant) {
        // What is reported at `now` was done before it: it belongs to the
        // sample that `now` ends or is in, not to one that ended earlier.
        self.close(now, true);
        let Some((unit, done, total)) = work(progress) else {
            return;
        };
        match &mut self.measured {
            Some(measured) if measured.unit == unit => {
                measured.latest = done;
                measured.total = total;
                self.close(now, false);
            }
            _ => {
                self.samples.clear();
                self.smoothed = None;
                self.measured = Some(Measured {
                    unit,
                    origin: now,
                    closed: 0,
                    marked: done,
                    latest: done,
                    total,
                });
            }
        }
    }

    /// Closes every sample that has ended by `now`. Seconds without a
    /// report count as seconds in which nothing moved.
    pub fn advance(&mut self, now: Instant) {
        self.close(now, false);
    }

    /// Closes the samples that ended by `now`, or only those that ended
    /// `before` it.
    fn close(&mut self, now: Instant, before: bool) {
        let Some(measured) = &mut self.measured else {
            return;
        };
        let mut elapsed = now.saturating_duration_since(measured.origin);
        if before {
            elapsed = elapsed.saturating_sub(Duration::from_nanos(1));
        }
        let due = (elapsed.as_secs() / SAMPLE.as_secs()).max(measured.closed);
        let mut pending = due.saturating_sub(measured.closed);
        measured.closed = due;
        // After a very long silence (a suspended machine) only the samples
        // the graph keeps are worth making: what moved before them is in
        // none of those.
        let keep = GRAPH_SAMPLES as u64;
        if pending > keep {
            measured.marked = measured.latest;
            pending = keep;
        }
        for _ in 0..pending {
            let sample = measured.latest.saturating_sub(measured.marked);
            measured.marked = measured.latest;
            if self.samples.len() == GRAPH_SAMPLES {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
            let sample = sample as f64;
            self.smoothed = Some(match self.smoothed {
                Some(smoothed) => smoothed + SMOOTHING * (sample - smoothed),
                None => sample,
            });
        }
    }

    /// The smoothed speed in units a second, once a whole second was
    /// measured.
    pub fn rate(&self) -> Option<(Unit, u64)> {
        let measured = self.measured.as_ref()?;
        let smoothed = self.smoothed?;
        Some((measured.unit, smoothed.round() as u64))
    }

    /// How long the rest takes at the smoothed speed. Told only once the
    /// speed is steady ([`STEADY_SAMPLES`]) and not zero.
    pub fn remaining(&self) -> Option<Duration> {
        let measured = self.measured.as_ref()?;
        let smoothed = self.smoothed?;
        let left = measured.total.checked_sub(measured.latest)?;
        if self.samples.len() < STEADY_SAMPLES || smoothed < 1. || left == 0 {
            return None;
        }
        Duration::try_from_secs_f64(left as f64 / smoothed).ok()
    }

    /// The speed in each of the last seconds, oldest first, at most
    /// [`GRAPH_SAMPLES`].
    pub fn samples(&self) -> impl Iterator<Item = u64> + '_ {
        self.samples.iter().copied()
    }

    /// The speed as the activity line shows it: "48.6 MiB/s".
    pub fn rate_text(&self) -> Option<String> {
        self.rate().map(|(unit, rate)| rate_text(unit, rate))
    }

    /// The time remaining as it is shown: "about 2 s left".
    pub fn remaining_text(&self) -> Option<String> {
        self.remaining().map(time_left)
    }

    /// The speed graph: each sample as a part of the highest one.
    pub fn graph(&self) -> Graph {
        let peak = self.samples().max().unwrap_or(0);
        let unit = self
            .measured
            .as_ref()
            .map_or(Unit::Bytes, |measured| measured.unit);
        Graph {
            points: self
                .samples()
                .map(|sample| {
                    if peak == 0 {
                        0.
                    } else {
                        (sample as f64 / peak as f64) as f32
                    }
                })
                .collect(),
            peak: if peak == 0 {
                String::new()
            } else {
                format!("Peak {}", rate_text(unit, peak))
            },
            span: GRAPH_SAMPLES,
        }
    }
}

/// A speed in words: "48.6 MiB/s", "120 items/s".
pub fn rate_text(unit: Unit, rate: u64) -> String {
    match unit {
        Unit::Bytes => format!("{}/s", format::size(rate)),
        Unit::Items => {
            let items = usize::try_from(rate).unwrap_or(usize::MAX);
            format!("{} {}/s", count(items), plural(items, "item", "items"))
        }
    }
}

/// A time remaining in words, rounded so it does not flicker: exact below
/// ten seconds, in steps of 5 s below a minute, then whole minutes, then
/// hours with minutes in steps of 5.
pub fn time_left(left: Duration) -> String {
    let seconds = left.as_secs_f64().round() as u64;
    let step = |value: u64, step: u64| (value + step / 2) / step * step;
    if seconds < 10 {
        return format!("about {} s left", seconds.max(1));
    }
    let seconds = step(seconds, 5);
    if seconds < 60 {
        return format!("about {seconds} s left");
    }
    let minutes = (left.as_secs_f64() / 60.).round() as u64;
    if minutes < 60 {
        return format!("about {minutes} min left");
    }
    let minutes = step(minutes, 5);
    match (minutes / 60, minutes % 60) {
        (hours, 0) => format!("about {hours} h left"),
        (hours, minutes) => format!("about {hours} h {minutes} min left"),
    }
}

/// The figures of the activity line: entries, bytes, speed and the time
/// remaining, each only when known ("4 of 12 · 51.2 MiB of 150.4 MiB ·
/// 48.6 MiB/s · about 2 s left").
pub fn figures(progress: &Progress, meter: &Meter) -> String {
    let mut parts = vec![format!(
        "{} of {}",
        count(progress.items_done as usize),
        count(progress.items_total as usize)
    )];
    parts.extend(bytes_text(progress));
    parts.extend(meter.rate_text());
    parts.extend(meter.remaining_text());
    parts.join(" · ")
}

/// "51.2 MiB of 150.4 MiB", for an operation that copies content.
fn bytes_text(progress: &Progress) -> Option<String> {
    (progress.bytes_total > 0).then(|| {
        format!(
            "{} of {}",
            format::size(progress.bytes_done),
            format::size(progress.bytes_total)
        )
    })
}

/// The speed graph of a card.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Graph {
    /// One point a second, oldest first, each from 0 to 1 of the peak.
    pub points: Vec<f32>,
    /// "Peak 52.0 MiB/s"; empty while nothing moved.
    pub peak: String,
    /// How many points fill the graph's width.
    pub span: usize,
}

/// One running operation in the transfer dialog.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Card {
    /// The operation's number, as text: it does not fit a QML number.
    pub id: String,
    /// What it is doing: "Copying 3 items to “x”".
    pub label: String,
    /// The Lucide icon beside the sentence.
    pub icon: String,
    /// How far it is from 0 to 1; negative while that is unknown.
    pub fraction: f32,
    /// What is done: "4 of 12 items · 51.2 MiB of 150.4 MiB".
    pub done: String,
    /// How fast and how long still: "48.6 MiB/s · about 2 s left".
    pub pace: String,
    /// The entry being worked on now, for display.
    pub current: String,
    /// The speed over the last minute.
    pub graph: Graph,
}

/// What a card says is done: entries, then bytes when content is copied.
pub fn done_text(progress: &Progress) -> String {
    let total = progress.items_total as usize;
    let mut parts = vec![format!(
        "{} of {} {}",
        count(progress.items_done as usize),
        count(total),
        plural(total, "item", "items")
    )];
    parts.extend(bytes_text(progress));
    parts.join(" · ")
}

/// A card's speed and time remaining, each only when known.
pub fn pace_text(meter: &Meter) -> String {
    let parts: Vec<String> = meter
        .rate_text()
        .into_iter()
        .chain(meter.remaining_text())
        .collect();
    parts.join(" · ")
}

/// The entry being worked on, for display; "Preparing…" before the first.
pub fn current_text(progress: &Progress) -> String {
    if progress.current.as_os_str().is_empty() {
        "Preparing…".to_owned()
    } else {
        rustphin_core::display_name(progress.current.as_os_str().as_encoded_bytes()).into_owned()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    fn bytes(done: u64, total: u64) -> Progress {
        Progress {
            items_done: 4,
            items_total: 12,
            bytes_done: done,
            bytes_total: total,
            current: "/a/b.bin".into(),
        }
    }

    fn items(done: u64, total: u64) -> Progress {
        Progress {
            items_done: done,
            items_total: total,
            ..Progress::default()
        }
    }

    /// A meter fed `per_second` bytes in each of `seconds` seconds of a
    /// transfer of `total` bytes, and the time of its last report.
    fn steady(per_second: u64, seconds: u64, total: u64) -> (Meter, Instant) {
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&bytes(0, total), start);
        for second in 1..=seconds {
            meter.observe(
                &bytes(per_second * second, total),
                start + SAMPLE * second as u32,
            );
        }
        (meter, start + SAMPLE * seconds as u32)
    }

    #[test]
    fn no_speed_is_told_before_a_whole_second_was_measured() {
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&bytes(0, 100 * MIB), start);
        meter.observe(&bytes(30 * MIB, 100 * MIB), start + SAMPLE / 2);
        assert_eq!(meter.rate(), None);

        meter.advance(start + SAMPLE);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 30 * MIB)));
    }

    #[test]
    fn the_first_report_only_sets_the_starting_point() {
        // A copy resumed part-way did not move its first bytes just now.
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&bytes(50 * MIB, 100 * MIB), start);
        meter.observe(&bytes(60 * MIB, 100 * MIB), start + SAMPLE);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 10 * MIB)));
    }

    #[test]
    fn one_fast_second_moves_the_speed_only_part_of_the_way() {
        let (mut meter, at) = steady(10 * MIB, 3, 1000 * MIB);
        meter.observe(&bytes(30 * MIB + 110 * MIB, 1000 * MIB), at + SAMPLE);
        // 10 + 0.3 × (110 − 10) = 40.
        assert_eq!(meter.rate(), Some((Unit::Bytes, 40 * MIB)));
    }

    #[test]
    fn the_time_remaining_waits_for_a_steady_speed() {
        let (meter, _) = steady(10 * MIB, STEADY_SAMPLES as u64 - 1, 100 * MIB);
        assert!(meter.rate().is_some());
        assert_eq!(meter.remaining(), None);

        let (meter, _) = steady(10 * MIB, STEADY_SAMPLES as u64, 100 * MIB);
        assert_eq!(meter.remaining(), Some(Duration::from_secs(7)));
        assert_eq!(meter.remaining_text().unwrap(), "about 7 s left");
    }

    #[test]
    fn seconds_without_a_report_slow_the_speed_until_no_time_is_told() {
        let (mut meter, at) = steady(10 * MIB, 3, 100 * MIB);
        meter.advance(at + SAMPLE);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 7 * MIB)));
        assert_eq!(meter.remaining(), Some(Duration::from_secs(10)));

        meter.advance(at + SAMPLE * 60);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 0)));
        assert_eq!(meter.remaining(), None);
        assert_eq!(meter.rate_text().unwrap(), "0 B/s");
    }

    #[test]
    fn nothing_is_left_to_wait_for_once_all_is_done() {
        let (meter, _) = steady(10 * MIB, 5, 50 * MIB);
        assert_eq!(meter.remaining(), None);
    }

    #[test]
    fn an_operation_without_content_is_measured_in_entries() {
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&items(0, 1000), start);
        for second in 1..=3 {
            meter.observe(&items(120 * second, 1000), start + SAMPLE * second as u32);
        }
        assert_eq!(meter.rate(), Some((Unit::Items, 120)));
        assert_eq!(meter.rate_text().unwrap(), "120 items/s");
        // 640 entries at 120 a second: 5.3 s.
        assert_eq!(meter.remaining_text().unwrap(), "about 5 s left");
    }

    #[test]
    fn nothing_is_measured_while_no_total_is_known() {
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&Progress::default(), start);
        meter.advance(start + SAMPLE * 5);
        assert_eq!(meter.rate(), None);
        assert_eq!(meter.samples().count(), 0);
    }

    #[test]
    fn measuring_starts_again_when_the_unit_changes() {
        // Entries were counted while the plan was made; then sizes arrive.
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&items(0, 10), start);
        meter.observe(&items(5, 10), start + SAMPLE * 2);
        assert_eq!(meter.rate().unwrap().0, Unit::Items);

        let switched = start + SAMPLE * 2 + SAMPLE / 2;
        meter.observe(&bytes(0, 100 * MIB), switched);
        assert_eq!(meter.rate(), None);
        assert_eq!(meter.samples().count(), 0);

        meter.observe(&bytes(20 * MIB, 100 * MIB), switched + SAMPLE);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 20 * MIB)));
    }

    #[test]
    fn the_graph_keeps_the_last_minute_scaled_to_its_peak() {
        let (mut meter, at) = steady(10 * MIB, GRAPH_SAMPLES as u64 + 5, 100_000 * MIB);
        assert_eq!(meter.samples().count(), GRAPH_SAMPLES);
        let done = 10 * MIB * (GRAPH_SAMPLES as u64 + 5);
        meter.observe(&bytes(done + 40 * MIB, 100_000 * MIB), at + SAMPLE);

        let graph = meter.graph();
        assert_eq!(graph.points.len(), GRAPH_SAMPLES);
        assert_eq!(graph.points.last(), Some(&1.));
        assert_eq!(graph.points.first(), Some(&0.25));
        assert_eq!(graph.peak, "Peak 40.0 MiB/s");
    }

    #[test]
    fn a_graph_of_seconds_in_which_nothing_moved_is_flat_and_names_no_peak() {
        let (meter, _) = steady(0, 4, 100 * MIB);
        assert_eq!(
            meter.graph(),
            Graph {
                points: vec![0.; 4],
                peak: String::new(),
                span: GRAPH_SAMPLES,
            }
        );
        assert_eq!(Meter::new().graph().points, Vec::<f32>::new());
    }

    #[test]
    fn a_very_long_silence_leaves_one_minute_of_empty_seconds() {
        let (mut meter, at) = steady(10 * MIB, 3, 100 * MIB);
        meter.observe(&bytes(40 * MIB, 100 * MIB), at + SAMPLE / 2);
        meter.advance(at + Duration::from_secs(30 * 24 * 3600));
        assert_eq!(meter.samples().collect::<Vec<_>>(), vec![0; GRAPH_SAMPLES]);
        assert_eq!(meter.rate(), Some((Unit::Bytes, 0)));
    }

    #[test]
    fn a_silence_of_exactly_a_minute_keeps_what_moved_before_it() {
        let (mut meter, at) = steady(10 * MIB, 3, 100 * MIB);
        meter.observe(&bytes(40 * MIB, 100 * MIB), at + SAMPLE / 2);
        meter.advance(at + SAMPLE * GRAPH_SAMPLES as u32);
        let samples: Vec<u64> = meter.samples().collect();
        assert_eq!(samples.len(), GRAPH_SAMPLES);
        assert_eq!(samples[0], 10 * MIB);
        assert_eq!(samples[1..], vec![0; GRAPH_SAMPLES - 1]);
    }

    #[test]
    fn the_slowest_speed_that_still_tells_a_time_is_one_unit_a_second() {
        let start = Instant::now();
        let mut meter = Meter::new();
        meter.observe(&items(0, 10), start);
        for second in 1..=3 {
            meter.observe(&items(second, 10), start + SAMPLE * second as u32);
        }
        assert_eq!(meter.remaining(), Some(Duration::from_secs(7)));
    }

    #[test]
    fn progress_that_goes_back_counts_as_nothing_moved() {
        // An entry tried again starts its bytes over.
        let (mut meter, at) = steady(10 * MIB, 3, 100 * MIB);
        meter.observe(&bytes(20 * MIB, 100 * MIB), at + SAMPLE);
        assert_eq!(meter.samples().last(), Some(0));
    }

    #[test]
    fn the_time_remaining_is_rounded_so_it_does_not_flicker() {
        let left = |seconds: f64| time_left(Duration::from_secs_f64(seconds));
        assert_eq!(left(0.2), "about 1 s left");
        assert_eq!(left(2.4), "about 2 s left");
        assert_eq!(left(9.4), "about 9 s left");
        assert_eq!(left(9.6), "about 10 s left");
        assert_eq!(left(12.), "about 10 s left");
        assert_eq!(left(13.), "about 15 s left");
        assert_eq!(left(57.), "about 55 s left");
        assert_eq!(left(58.), "about 1 min left");
        assert_eq!(left(89.), "about 1 min left");
        assert_eq!(left(90.), "about 2 min left");
        assert_eq!(left(59. * 60. + 20.), "about 59 min left");
        assert_eq!(left(59. * 60. + 40.), "about 1 h left");
        assert_eq!(left(3600. + 17. * 60.), "about 1 h 15 min left");
        assert_eq!(left(3600. + 58. * 60.), "about 2 h left");
        assert_eq!(left(26. * 3600.), "about 26 h left");
    }

    #[test]
    fn the_figures_name_each_part_only_when_it_is_known() {
        // 12 MiB a second for 4 s of 150.4 MiB: 102.4 MiB left, 8.5 s.
        let total = 150 * MIB + 2 * MIB / 5;
        let (meter, _) = steady(12 * MIB, 4, total);
        assert_eq!(
            figures(&bytes(48 * MIB, total), &meter),
            "4 of 12 · 48.0 MiB of 150.4 MiB · 12.0 MiB/s · about 9 s left"
        );
        assert_eq!(
            figures(&bytes(0, total), &Meter::new()),
            "4 of 12 · 0 B of 150.4 MiB"
        );
        assert_eq!(figures(&items(1, 4), &Meter::new()), "1 of 4");
    }

    #[test]
    fn a_card_says_what_is_done_and_how_fast() {
        let total = 100 * MIB;
        let (meter, _) = steady(10 * MIB, 4, total);
        assert_eq!(
            done_text(&bytes(40 * MIB, total)),
            "4 of 12 items · 40.0 MiB of 100.0 MiB"
        );
        assert_eq!(done_text(&items(0, 1)), "0 of 1 item");
        assert_eq!(pace_text(&meter), "10.0 MiB/s · about 6 s left");
        assert_eq!(pace_text(&Meter::new()), "");
    }

    #[test]
    fn a_card_names_the_entry_being_worked_on() {
        assert_eq!(current_text(&bytes(0, 1)), "/a/b.bin");
        assert_eq!(current_text(&Progress::default()), "Preparing…");
    }
}
