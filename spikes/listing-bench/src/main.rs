//! DISPOSABLE SPIKE — times listing, stat and natural sort for one large directory.
//! Usage: rustphin-spike-listing <dir> [create <count>]

use std::{cmp::Ordering, fs, os::unix::fs::MetadataExt, path::Path, time::Instant};

use rayon::prelude::*;

/// Case-insensitive natural comparison: digit runs compare by numeric value.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let x = &a[si..i][a[si..i].iter().take_while(|c| **c == b'0').count()..];
            let y = &b[sj..j][b[sj..j].iter().take_while(|c| **c == b'0').count()..];
            match x.len().cmp(&y.len()).then_with(|| x.cmp(y)) {
                Ordering::Equal => {}
                other => return other,
            }
        } else {
            match a[i].to_ascii_lowercase().cmp(&b[j].to_ascii_lowercase()) {
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
                other => return other,
            }
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    println!("{label:<44} {:>8.1} ms", start.elapsed().as_secs_f64() * 1e3);
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = Path::new(&args[1]);
    if args.get(2).map(String::as_str) == Some("create") {
        let count: usize = args[3].parse().unwrap();
        fs::create_dir_all(dir).unwrap();
        let prefixes = ["IMG_", "report-", "Track ", "data.", "Zeta", "alpha_"];
        timed("create files", || {
            for n in 0..count {
                let name = format!("{}{}.{}", prefixes[n % 6], (n * 7919) % count, ["jpg", "txt"][n % 2]);
                fs::write(dir.join(name), b"").unwrap();
            }
        });
        return;
    }

    let names: Vec<String> = timed("read_dir: names + d_type", || {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                let _ = e.file_type();
                e.file_name().into_string().unwrap()
            })
            .collect()
    });
    println!("entries: {}", names.len());

    let total: u64 = timed("stat every entry, 1 thread", || {
        names.iter().map(|n| fs::symlink_metadata(dir.join(n)).unwrap().size()).sum()
    });
    let total_par: u64 = timed("stat every entry, rayon", || {
        names.par_iter().map(|n| fs::symlink_metadata(dir.join(n)).unwrap().size()).sum()
    });
    assert_eq!(total, total_par);

    let mut a = names.clone();
    timed("natural sort, 1 thread", || a.sort_unstable_by(|x, y| natural_cmp(x, y)));
    let mut b = names.clone();
    timed("natural sort, rayon", || b.par_sort_unstable_by(|x, y| natural_cmp(x, y)));
    assert_eq!(a, b);
    println!("first: {:?}  last: {:?}", a[0], a[a.len() - 1]);
    println!("name bytes: {} (avg {:.1})", names.iter().map(String::len).sum::<usize>(), names.iter().map(String::len).sum::<usize>() as f64 / names.len() as f64);
}
