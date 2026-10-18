//! Measures fuzzy ranking of 10,000 recent locations (R-015).
//! Exits unsuccessfully when any measured query takes more than 50 ms.

use std::{
    hint::black_box,
    process::ExitCode,
    time::{Duration, Instant},
};

use rustphin_core::{RECENT_LOCATION_LIMIT, SearchQuery};

const RUNS: usize = 20;
const RESULT_LIMIT: usize = 100;
const LATENCY_LIMIT: Duration = Duration::from_millis(50);

fn main() -> ExitCode {
    let locations: Vec<String> = (0..RECENT_LOCATION_LIMIT)
        .map(|index| format!("/home/example/Documents/Projects/Été-{index:05}/source and pictures"))
        .collect();
    let mut passed = true;
    for text in ["", "projects", "psp", "ÉTÉ", "9999", "not-present"] {
        let mut worst = Duration::ZERO;
        let mut count = 0;
        for _ in 0..RUNS {
            let start = Instant::now();
            let query = SearchQuery::new(black_box(text));
            let results = query.rank(locations.iter().map(String::as_str), RESULT_LIMIT);
            count = black_box(results).len();
            worst = worst.max(start.elapsed());
        }
        println!(
            "{text:?}: worst {:.3} ms across {RUNS} runs, {count} results",
            worst.as_secs_f64() * 1e3
        );
        passed &= worst <= LATENCY_LIMIT;
    }
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
