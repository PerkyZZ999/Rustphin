//! Measures how fast a very large directory becomes visible and fully sorted.
//!
//! Usage: `bench_listing <directory> [entries]`. The directory is created and
//! filled on first use. Exits with status 1 when a threshold of requirement
//! R-101 is missed: first entries within 200 ms, sorted listing within 500 ms,
//! and re-sorting by any column within 500 ms.

use std::{
    error::Error,
    fs,
    path::Path,
    process::ExitCode,
    sync::Arc,
    time::{Duration, Instant},
};

use rustphin_core::{DirectoryModel, ModelEvent, RealFs, SortKey, ViewOptions};

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

/// Waits for the complete snapshot of `generation`; returns when the first
/// snapshot of it arrived and how many rows the complete one has.
fn settle(
    events: &flume::Receiver<ModelEvent>,
    generation: u64,
    start: Instant,
) -> Result<(Duration, usize), Box<dyn Error>> {
    let mut first = None;
    loop {
        match events.recv()? {
            ModelEvent::Snapshot(snapshot) if snapshot.generation() == generation => {
                let first = *first.get_or_insert(start.elapsed());
                if !snapshot.is_loading() {
                    return Ok((first, snapshot.len()));
                }
            }
            ModelEvent::Failed { error, .. } => return Err(error.into()),
            ModelEvent::Snapshot(_) | ModelEvent::Moved { .. } => {}
        }
    }
}

/// Time to load details, then a label and time for each re-sort.
type Resorts = (Duration, Vec<(String, Duration)>);

/// Times loading every entry's size and date, then re-sorting by each column.
///
/// The details load is reported but not held to a limit: its cost depends on
/// whether the kernel still has the files' inodes cached, which this program
/// cannot control. The re-sorts that follow reuse the loaded details.
fn measure_resorts(dir: &Path) -> Result<Resorts, Box<dyn Error>> {
    let (mut model, events) = DirectoryModel::new(Arc::new(RealFs))?;
    settle(&events, model.open(dir), Instant::now())?;

    let base = ViewOptions {
        details: true,
        ..ViewOptions::default()
    };
    // Time until sizes and dates are known for the top of the list, and for all of it.
    let start = Instant::now();
    let generation = model.set_options(base).ok_or("options did not change")?;
    let mut first_details = None;
    loop {
        if let ModelEvent::Snapshot(snapshot) = events.recv()?
            && snapshot.generation() == generation
        {
            if first_details.is_none() && snapshot.metadata(0).is_some() {
                first_details = Some(start.elapsed());
            }
            if !snapshot.is_loading() {
                break;
            }
        }
    }
    let details = start.elapsed();
    println!(
        "sizes and dates at the top {:>6.1} ms",
        first_details.unwrap_or(details).as_secs_f64() * 1e3
    );
    let mut switch = |options: ViewOptions| -> Result<Duration, Box<dyn Error>> {
        let start = Instant::now();
        let generation = model.set_options(options).ok_or("options did not change")?;
        settle(&events, generation, start)?;
        Ok(start.elapsed())
    };
    let mut timings = Vec::new();
    for (label, sort, descending) in [
        ("size", SortKey::Size, false),
        ("modified", SortKey::Modified, false),
        ("type", SortKey::Type, false),
        ("name, descending", SortKey::Name, true),
    ] {
        let options = ViewOptions {
            sort,
            descending,
            ..base
        };
        timings.push((label.to_owned(), switch(options)?));
    }
    Ok((details, timings))
}

/// Returns the time to the first snapshot and to the complete one.
fn measure(dir: &Path, entries: usize) -> Result<(Duration, Duration), Box<dyn Error>> {
    let (mut model, events) = DirectoryModel::new(Arc::new(RealFs))?;
    let start = Instant::now();
    let generation = model.open(dir);
    let (first, listed) = settle(&events, generation, start)?;
    if listed != entries {
        return Err(format!("listed {listed} of {entries} entries").into());
    }
    Ok((first, start.elapsed()))
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
    let mut worst_resort = Duration::ZERO;
    let (details, resorts) = measure_resorts(dir)?;
    println!(
        "load sizes and dates       {:>6.1} ms (no limit: depends on the inode cache)",
        details.as_secs_f64() * 1e3
    );
    for (label, elapsed) in resorts {
        println!(
            "re-sort by {label:<16} {:>6.1} ms",
            elapsed.as_secs_f64() * 1e3
        );
        worst_resort = worst_resort.max(elapsed);
    }
    let pass =
        worst.0 <= FIRST_ENTRIES_LIMIT && worst.1 <= SORTED_LIMIT && worst_resort <= SORTED_LIMIT;
    println!(
        "{entries} entries, worst of {RUNS}: first entries {:.1} ms (limit {}), sorted listing {:.1} ms (limit {}), slowest re-sort {:.1} ms (limit {}) => {}",
        worst.0.as_secs_f64() * 1e3,
        FIRST_ENTRIES_LIMIT.as_millis(),
        worst.1.as_secs_f64() * 1e3,
        SORTED_LIMIT.as_millis(),
        worst_resort.as_secs_f64() * 1e3,
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
