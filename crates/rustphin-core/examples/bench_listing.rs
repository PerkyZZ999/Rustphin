//! Measures how fast a very large directory becomes visible and fully sorted.
//!
//! Usage: `bench_listing <directory> [entries]`. The directory is created and
//! filled on first use. Exits with status 1 when a threshold of requirement
//! R-101 is missed: first entries within 200 ms, sorted listing within 500 ms.

use std::{
    error::Error,
    fs,
    path::Path,
    process::ExitCode,
    sync::Arc,
    time::{Duration, Instant},
};

use rustphin_core::{DirectoryModel, ModelEvent, RealFs};

const DEFAULT_ENTRIES: usize = 500_000;
const RUNS: usize = 5;
const FIRST_ENTRIES_LIMIT: Duration = Duration::from_millis(200);
const SORTED_LIMIT: Duration = Duration::from_millis(500);

fn populate(dir: &Path, entries: usize) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let existing = fs::read_dir(dir)?.count();
    if existing == entries {
        return Ok(());
    }
    if existing != 0 {
        return Err(format!(
            "{} holds {existing} entries, expected 0 or {entries}",
            dir.display()
        )
        .into());
    }
    println!("creating {entries} files in {}", dir.display());
    let prefixes = ["IMG_", "report-", "Track ", "data.", "Zeta", "alpha_"];
    for n in 0..entries {
        let name = format!(
            "{}{}.{}",
            prefixes[n % 6],
            (n * 7919) % entries,
            ["jpg", "txt"][n % 2]
        );
        fs::write(dir.join(name), b"")?;
    }
    Ok(())
}

/// Returns the time to the first snapshot and to the complete one.
fn measure(dir: &Path, entries: usize) -> Result<(Duration, Duration), Box<dyn Error>> {
    let (mut model, events) = DirectoryModel::new(Arc::new(RealFs))?;
    let start = Instant::now();
    let generation = model.open(dir);
    let mut first = None;
    loop {
        match events.recv()? {
            ModelEvent::Snapshot(snapshot) if snapshot.generation() == generation => {
                first.get_or_insert(start.elapsed());
                if !snapshot.is_loading() {
                    if snapshot.len() != entries {
                        return Err(
                            format!("listed {} of {entries} entries", snapshot.len()).into()
                        );
                    }
                    return Ok((first.unwrap_or_default(), start.elapsed()));
                }
            }
            ModelEvent::Failed { error, .. } => return Err(error.into()),
            ModelEvent::Snapshot(_) => {}
        }
    }
}

fn run() -> Result<bool, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .ok_or("usage: bench_listing <directory> [entries]")?;
    let entries = args
        .next()
        .map(|n| n.parse())
        .transpose()?
        .unwrap_or(DEFAULT_ENTRIES);
    let dir = Path::new(&dir);
    populate(dir, entries)?;

    let mut worst = (Duration::ZERO, Duration::ZERO);
    for run in 1..=RUNS {
        let (first, sorted) = measure(dir, entries)?;
        println!(
            "run {run}: first entries {:>6.1} ms, sorted listing {:>6.1} ms",
            first.as_secs_f64() * 1e3,
            sorted.as_secs_f64() * 1e3
        );
        worst = (worst.0.max(first), worst.1.max(sorted));
    }
    let pass = worst.0 <= FIRST_ENTRIES_LIMIT && worst.1 <= SORTED_LIMIT;
    println!(
        "{entries} entries, worst of {RUNS}: first entries {:.1} ms (limit {}), sorted listing {:.1} ms (limit {}) => {}",
        worst.0.as_secs_f64() * 1e3,
        FIRST_ENTRIES_LIMIT.as_millis(),
        worst.1.as_secs_f64() * 1e3,
        SORTED_LIMIT.as_millis(),
        if pass { "PASS" } else { "FAIL" }
    );
    Ok(pass)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("bench_listing: {error}");
            ExitCode::FAILURE
        }
    }
}
