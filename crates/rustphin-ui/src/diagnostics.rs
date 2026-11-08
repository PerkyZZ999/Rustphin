//! Opt-in frame timing, for checking the responsiveness targets.
//!
//! With `RUSTPHIN_FRAME_LOG` set, the view records when each frame started and
//! how long the view took to describe itself, and logs a summary every
//! [`WINDOW`] frames. The interval between frames is the useful figure: it
//! includes building the visible rows, layout and painting, which happen after
//! the view returns. The build time covers only the view's own description.
//!
//! With `RUSTPHIN_SCROLL_BENCH` set as well as a folder open, the view scrolls
//! itself a few rows every frame for a fixed number of frames, so the timing
//! can be taken without an input device.
//!
//! A frame interval longer than 1.5 × the display's refresh period is a
//! dropped frame (D-019). Each summary counts them, and when the self-driven
//! scroll ends a verdict for the whole run says whether at most 5% of its
//! intervals were dropped, as R-101 asks.

use std::time::{Duration, Instant};

/// Frames per logged summary.
const WINDOW: usize = 120;
/// Rows the self-driven scroll advances per frame.
const BENCH_STEP: usize = 3;
/// Frames the self-driven scroll runs for.
const BENCH_FRAMES: usize = WINDOW * 10;
/// Long gaps are treated as idle only when no scroll benchmark is driving frames.
const IDLE_GAP: Duration = Duration::from_millis(250);
/// The refresh period assumed when the platform does not report one (60 Hz).
const ASSUMED_REFRESH: Duration = Duration::from_nanos(16_666_667);
/// The share of dropped frames sustained scrolling may have (D-019), as a
/// fraction: one in twenty.
const DROPPED_LIMIT_PER_TWENTY: usize = 1;

/// Collects frame timings when enabled.
#[derive(Default)]
pub struct FrameLog {
    enabled: bool,
    last_frame: Option<Instant>,
    intervals: Vec<Duration>,
    builds: Vec<Duration>,
    excluded_gaps: usize,
    longest_gap: Duration,
    /// The display's refresh period, when the platform reports it.
    refresh: Option<Duration>,
    /// Intervals and dropped frames over the whole self-driven scroll.
    bench_intervals: usize,
    bench_dropped: usize,
    /// Frames left in the self-driven scroll, and the row it has reached.
    bench: Option<(usize, usize)>,
    #[cfg(feature = "diagnostics")]
    toolkit: Option<gpui_kit::profiler::journal::ForegroundJournalCollector>,
}

impl FrameLog {
    /// A log that records only if `RUSTPHIN_FRAME_LOG` is set to something but `0`.
    pub fn from_env(_cx: &gpui_kit::App) -> Self {
        let set = |name| std::env::var_os(name).is_some_and(|value| value != "0");
        let bench = set("RUSTPHIN_SCROLL_BENCH");
        let enabled = bench || set("RUSTPHIN_FRAME_LOG");
        Self {
            enabled,
            bench: bench.then_some((BENCH_FRAMES, 0)),
            #[cfg(feature = "diagnostics")]
            toolkit: enabled.then(|| {
                gpui_kit::profiler::set_trace_enabled(true);
                _cx.foreground_journal().collector()
            }),
            ..Self::default()
        }
    }

    /// Marks the start of building a frame on a display that refreshes every
    /// `refresh` (`None` when unknown). Pass the result to [`Self::end`].
    pub fn begin(&mut self, refresh: Option<Duration>) -> Option<Instant> {
        if !self.enabled {
            return None;
        }
        #[cfg(feature = "diagnostics")]
        self.record_toolkit();
        self.refresh = refresh.filter(|period| !period.is_zero());
        self.begin_at(Instant::now())
    }

    /// The refresh period dropped frames are measured against.
    fn period(&self) -> Duration {
        self.refresh.unwrap_or(ASSUMED_REFRESH)
    }

    fn begin_at(&mut self, now: Instant) -> Option<Instant> {
        if !self.enabled {
            return None;
        }
        if let Some(interval) = self.last_frame.map(|last| now.duration_since(last)) {
            self.longest_gap = self.longest_gap.max(interval);
            let driving = self
                .bench
                .is_some_and(|(left, _)| left < BENCH_FRAMES && left > 0);
            if driving {
                self.bench_intervals += 1;
                self.bench_dropped += usize::from(is_dropped(interval, self.period()));
            }
            if interval <= IDLE_GAP || driving {
                self.intervals.push(interval);
            } else {
                self.excluded_gaps += 1;
            }
        }
        self.last_frame = Some(now);
        Some(now)
    }

    /// The row to scroll to this frame while the self-driven scroll is
    /// running over a list of `rows` rows, wrapping to the top at the end.
    /// Returns `None` once it has finished or was never requested.
    pub fn next_bench_row(&mut self, rows: usize) -> Option<usize> {
        let (frames_left, row) = self.bench.as_mut()?;
        if *frames_left == 0 || rows == 0 {
            if *frames_left == 0 {
                self.bench = None;
            }
            return None;
        }
        *frames_left -= 1;
        *row = (*row + BENCH_STEP) % rows;
        let row = *row;
        if *frames_left == 0 {
            self.report_bench();
        }
        Some(row)
    }

    /// Logs the dropped-frame verdict for the whole self-driven scroll.
    fn report_bench(&self) {
        let (counted, dropped) = (self.bench_intervals, self.bench_dropped);
        tracing::info!(
            counted_intervals = counted,
            dropped_frames = dropped,
            dropped_percent = percent(dropped, counted),
            refresh_ms = self.period().as_secs_f64() * 1e3,
            refresh_reported = self.refresh.is_some(),
            passes_d019 = within_limit(dropped, counted),
            "scroll benchmark result"
        );
    }

    /// Marks the end of building a frame and logs a summary when due.
    pub fn end(&mut self, started: Option<Instant>) {
        let Some(started) = started else {
            return;
        };
        self.builds.push(started.elapsed());
        if self.builds.len() >= WINDOW {
            let (intervals, builds) = (summarise(&self.intervals), summarise(&self.builds));
            let period = self.period();
            let dropped = self
                .intervals
                .iter()
                .filter(|&&interval| is_dropped(interval, period))
                .count();
            tracing::info!(
                frames = self.builds.len(),
                counted_intervals = self.intervals.len(),
                dropped_frames = dropped,
                dropped_percent = percent(dropped, self.intervals.len()),
                refresh_ms = period.as_secs_f64() * 1e3,
                refresh_reported = self.refresh.is_some(),
                excluded_gaps = self.excluded_gaps,
                longest_gap_ms = self.longest_gap.as_secs_f64() * 1e3,
                interval_p50_ms = intervals.median,
                interval_p95_ms = intervals.p95,
                interval_max_ms = intervals.max,
                build_p50_ms = builds.median,
                build_p95_ms = builds.p95,
                build_max_ms = builds.max,
                "frame timing"
            );
            self.intervals.clear();
            self.builds.clear();
            self.excluded_gaps = 0;
            self.longest_gap = Duration::ZERO;
        }
    }

    /// Times a synchronous UI task only when frame diagnostics are enabled.
    pub fn task(&self, name: &'static str) -> Option<TaskTimer> {
        self.enabled.then(|| TaskTimer {
            name,
            started: Instant::now(),
        })
    }

    #[cfg(feature = "diagnostics")]
    fn record_toolkit(&mut self) {
        use gpui_kit::profiler::journal::{
            ForegroundEvent, ForegroundJournalEntry, IntervalBoundary,
        };
        let Some(collector) = &mut self.toolkit else {
            return;
        };
        let ms = |duration: Duration| duration.as_secs_f64() * 1e3;
        let drained = collector.collect_unseen();
        if drained.lost > 0 {
            tracing::warn!(lost = drained.lost, "toolkit timing discontinuity");
        }
        for entry in drained.entries {
            let event = match entry {
                ForegroundJournalEntry::Event(event) => event,
                ForegroundJournalEntry::Boundary(IntervalBoundary::Presented(frame)) => {
                    ForegroundEvent::Present(frame.presentation)
                }
                _ => continue,
            };
            match event {
                ForegroundEvent::Draw(frame) => tracing::info!(
                    draw_ms = ms(frame.draw_duration()),
                    layout_ms = ms(frame.phases.request_layout),
                    prepaint_ms = ms(frame.phases.prepaint),
                    paint_ms = ms(frame.phases.paint),
                    "toolkit draw timing"
                ),
                ForegroundEvent::Present(frame) => tracing::info!(
                    present_ms = ms(frame.present_duration()),
                    animation_interval_ms = frame.animation_interval.map_or(0., ms),
                    "toolkit present timing"
                ),
                ForegroundEvent::View(view) => tracing::info!(
                    view = view.name,
                    phase = view.phase.as_str(),
                    elapsed_ms = ms(view.self_duration),
                    "toolkit view timing"
                ),
                ForegroundEvent::TaskPoll(task) => tracing::info!(
                    source = %task.location, elapsed_ms = ms(task.poll_duration()),
                    "toolkit foreground task timing"
                ),
                _ => {}
            }
        }
    }
}

/// An opt-in task timer; dropping it reports wall time on the UI thread.
pub struct TaskTimer {
    name: &'static str,
    started: Instant,
}

impl Drop for TaskTimer {
    fn drop(&mut self) {
        tracing::info!(
            task = self.name,
            elapsed_ms = self.started.elapsed().as_secs_f64() * 1e3,
            "UI task timing"
        );
    }
}

/// Whether a frame interval means a frame was dropped: longer than 1.5 ×
/// the refresh period (D-019).
fn is_dropped(interval: Duration, period: Duration) -> bool {
    interval.as_nanos() * 2 > period.as_nanos() * 3
}

/// Whether `dropped` of `counted` intervals is within D-019's 5%. A run with
/// no intervals measured nothing and does not pass.
fn within_limit(dropped: usize, counted: usize) -> bool {
    counted > 0 && dropped * 20 <= counted * DROPPED_LIMIT_PER_TWENTY
}

/// `part` as a percentage of `whole`; zero when there is no whole.
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.;
    }
    part as f64 / whole as f64 * 100.
}

/// Median, 95th percentile and maximum of a set of durations, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Summary {
    median: f64,
    p95: f64,
    max: f64,
}

/// Summarises `samples`; all zero when there are none. Percentiles use the
/// nearest-rank method.
fn summarise(samples: &[Duration]) -> Summary {
    let mut sorted: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1e3).collect();
    sorted.sort_by(f64::total_cmp);
    let rank = |fraction: f64| -> f64 {
        let index = ((fraction * sorted.len() as f64).ceil() as usize).saturating_sub(1);
        sorted.get(index).copied().unwrap_or(0.)
    };
    Summary {
        median: rank(0.5),
        p95: rank(0.95),
        max: sorted.last().copied().unwrap_or(0.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(values: &[u64]) -> Vec<Duration> {
        values.iter().map(|&ms| Duration::from_millis(ms)).collect()
    }

    #[test]
    fn summary_reports_median_p95_and_max_by_nearest_rank() {
        let samples: Vec<u64> = (1..=100).collect();
        assert_eq!(
            summarise(&millis(&samples)),
            Summary {
                median: 50.,
                p95: 95.,
                max: 100.
            }
        );
    }

    #[test]
    fn summary_does_not_depend_on_sample_order() {
        assert_eq!(
            summarise(&millis(&[30, 10, 20])),
            Summary {
                median: 20.,
                p95: 30.,
                max: 30.
            }
        );
    }

    #[test]
    fn an_empty_summary_is_all_zero() {
        assert_eq!(
            summarise(&[]),
            Summary {
                median: 0.,
                p95: 0.,
                max: 0.
            }
        );
    }

    #[test]
    fn the_self_driven_scroll_steps_wraps_and_then_stops() {
        let mut log = FrameLog {
            bench: Some((3, 0)),
            ..FrameLog::default()
        };
        assert_eq!(log.next_bench_row(0), None);
        assert_eq!(log.next_bench_row(100), Some(BENCH_STEP));
        assert_eq!(log.next_bench_row(BENCH_STEP + 2), Some(BENCH_STEP - 2));
        assert_eq!(log.next_bench_row(100), Some(2 * BENCH_STEP - 2));
        assert_eq!(log.next_bench_row(100), None);
        assert_eq!(FrameLog::default().next_bench_row(100), None);
    }

    #[test]
    fn a_disabled_log_records_nothing() {
        let mut log = FrameLog::default();
        let started = log.begin(None);
        log.end(started);
        assert_eq!(started, None);
        assert!(log.builds.is_empty() && log.intervals.is_empty());
    }

    #[test]
    fn an_enabled_log_counts_builds_and_only_gaps_between_consecutive_frames() {
        let mut log = FrameLog {
            enabled: true,
            ..FrameLog::default()
        };
        let first = log.begin(None);
        log.end(first);
        let second = log.begin(None);
        log.end(second);
        assert_eq!(log.builds.len(), 2);
        assert_eq!(log.intervals.len(), 1);
    }

    #[test]
    fn an_active_scroll_benchmark_counts_even_a_one_second_stall() {
        let start = Instant::now();
        let mut log = FrameLog {
            enabled: true,
            bench: Some((BENCH_FRAMES - 1, 0)),
            ..FrameLog::default()
        };
        log.begin_at(start);
        log.begin_at(start + Duration::from_secs(1));
        assert_eq!(summarise(&log.intervals).max, 1000.);
    }

    #[test]
    fn a_frame_is_dropped_only_beyond_one_and_a_half_refresh_periods() {
        let period = Duration::from_micros(16_667);
        assert!(!is_dropped(Duration::from_micros(16_980), period), "jitter");
        let even = Duration::from_millis(16);
        assert!(
            !is_dropped(Duration::from_micros(24_000), even),
            "1.5 x is not"
        );
        assert!(is_dropped(Duration::from_micros(24_001), even));
        assert!(
            is_dropped(Duration::from_micros(33_333), period),
            "a missed frame"
        );
        let fast = Duration::from_micros(6_944); // 144 Hz
        assert!(is_dropped(Duration::from_micros(16_667), fast));
    }

    #[test]
    fn five_percent_dropped_passes_and_more_or_nothing_measured_fails() {
        assert!(within_limit(0, 1_199));
        assert!(within_limit(59, 1_199));
        assert!(within_limit(60, 1_200));
        assert!(!within_limit(61, 1_200));
        assert!(!within_limit(0, 0));
        assert_eq!(percent(30, 1_200), 2.5);
        assert_eq!(percent(1, 0), 0.);
    }

    #[test]
    fn the_benchmark_counts_dropped_frames_against_the_reported_refresh() {
        let start = Instant::now();
        let mut log = FrameLog {
            enabled: true,
            bench: Some((BENCH_FRAMES - 1, 0)),
            refresh: Some(Duration::from_millis(10)),
            ..FrameLog::default()
        };
        let mut at = start;
        for gap in [10, 10, 16, 14, 30] {
            log.begin_at(at);
            at += Duration::from_millis(gap);
        }
        log.begin_at(at);
        assert_eq!((log.bench_intervals, log.bench_dropped), (5, 2));
        // Unknown refresh: 60 Hz is assumed, so only the 30 ms gap drops.
        log.refresh = None;
        log.bench_dropped = 0;
        log.bench_intervals = 0;
        log.begin_at(at + Duration::from_millis(30));
        assert_eq!((log.bench_intervals, log.bench_dropped), (1, 1));
    }
}
