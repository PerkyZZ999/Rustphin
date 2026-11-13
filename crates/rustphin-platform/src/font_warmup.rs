//! Reads the system's font files in parallel just before the UI toolkit
//! scans them (W-109b).
//!
//! GPUI builds its text system with fontdb's `load_system_fonts`, which
//! opens every font file one after another on one thread to read its
//! headers. With thousands of fonts on a compressed file system and a cold
//! page cache, that scan takes seconds. Rustphin cannot change the scan
//! (D-010: no fork), but it can read the same parts of the same files on
//! several threads first, in the order fontdb walks them, so the scan
//! finds them in memory. Every font stays available; nothing is changed or
//! hidden, and nothing is kept: the pages stay in the system's cache.

use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

/// The most threads used: more barely helped in W-109's measurement.
const MAX_THREADS: usize = 16;

/// How many font files are sampled to tell whether the fonts are already
/// in memory, and the average time to open one and read its first 4 KB
/// below which they are taken to be: microseconds from memory, a large
/// fraction of a millisecond from a cold compressed disk.
const PROBE_FILES: usize = 32;
const CACHED_READ: Duration = Duration::from_micros(50);

/// The folders fontdb scans, in its order: the `<dir>` entries of the
/// fontconfig files it reads (`FONTCONFIG_FILE`, or the user's and the
/// system's), `~` meaning `HOME`; without any, its fixed fallback folders.
/// `var` reads an environment variable.
pub fn font_dirs(var: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let home = var("HOME").map(PathBuf::from);
    let mut config = fontconfig_parser::FontConfig::default();
    if let Some(file) = var("FONTCONFIG_FILE") {
        let _ = config.merge_config(Path::new(&file));
    } else {
        let config_home = var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|home| home.join(".config")));
        let read_global = match config_home {
            Some(dir) => config
                .merge_config(&dir.join("fontconfig/fonts.conf"))
                .is_err(),
            None => true,
        };
        if read_global {
            let _ = config.merge_config(Path::new("/etc/fonts/local.conf"));
        }
        let _ = config.merge_config(Path::new("/etc/fonts/fonts.conf"));
    }
    if config.dirs.is_empty() {
        let mut dirs = vec![
            PathBuf::from("/usr/share/fonts/"),
            PathBuf::from("/usr/local/share/fonts/"),
        ];
        if let Some(home) = home {
            dirs.push(home.join(".fonts"));
            dirs.push(home.join(".local/share/fonts"));
        }
        return dirs;
    }
    config
        .dirs
        .into_iter()
        .filter_map(|dir| match dir.path.strip_prefix("~") {
            Ok(rest) => home.as_ref().map(|home| home.join(rest)),
            Err(_) => Some(dir.path),
        })
        .collect()
}

/// Puts the calling thread at the lowest CPU priority, so that when the
/// fonts are already in memory the reading gives way to the app starting.
/// On Linux a thread's priority is set through its own thread ID.
fn lower_priority() {
    let _ = rustix::process::setpriority_process(Some(rustix::thread::gettid()), 19);
}

/// Whether fontdb reads a file with this name.
fn is_font(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("ttf" | "ttc" | "TTF" | "TTC" | "otf" | "otc" | "OTF" | "OTC")
    )
}

/// Calls `found` with every font file under `dirs`, depth first in
/// directory order as fontdb walks them, until it returns `false`. A
/// folder reached twice (through a link) is walked once.
pub fn walk(dirs: &[PathBuf], mut found: impl FnMut(PathBuf) -> bool) {
    let mut seen = HashSet::new();
    for dir in dirs {
        if !walk_dir(dir, &mut seen, &mut found) {
            return;
        }
    }
}

/// Walks one folder; `false` once `found` asked to stop.
fn walk_dir(
    dir: &Path,
    seen: &mut HashSet<PathBuf>,
    found: &mut impl FnMut(PathBuf) -> bool,
) -> bool {
    let Ok(canonical) = std::fs::canonicalize(dir) else {
        return true;
    };
    if !seen.insert(canonical) {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        // A link counts as what it points to, as in fontdb.
        let (is_file, is_dir) = if kind.is_symlink() {
            match std::fs::metadata(&path) {
                Ok(meta) => (meta.is_file(), meta.is_dir()),
                Err(_) => continue,
            }
        } else {
            (kind.is_file(), kind.is_dir())
        };
        let more = if is_file && is_font(&path) {
            found(path)
        } else if is_dir {
            walk_dir(&path, seen, found)
        } else {
            true
        };
        if !more {
            return false;
        }
    }
    true
}

/// Whether the first font files under `dirs` read as fast as from memory,
/// so the fonts are probably cached and reading them ahead would only
/// compete with the app starting.
fn already_in_memory(dirs: &[PathBuf]) -> bool {
    let mut sample = Vec::with_capacity(PROBE_FILES);
    walk(dirs, |path| {
        sample.push(path);
        sample.len() < PROBE_FILES
    });
    let started = Instant::now();
    for path in &sample {
        let mut head = [0; 4096];
        if let Ok(mut file) = std::fs::File::open(path) {
            let _ = std::io::Read::read(&mut file, &mut head);
        }
    }
    let count = u32::try_from(sample.len()).unwrap_or(u32::MAX);
    started.elapsed() < CACHED_READ * count.max(1)
}

/// What a pre-read did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Font files read.
    pub files: usize,
    /// Faces found in them.
    pub faces: usize,
}

/// Reads the font files under `dirs` with up to `threads` threads, each
/// parsing them as fontdb does so the same parts of each file are read.
/// Returns when every file has been read.
pub fn read_all(dirs: &[PathBuf], threads: usize) -> Report {
    let (send, receive) = flume::unbounded::<PathBuf>();
    let workers: Vec<_> = (0..threads.max(1))
        .filter_map(|index| {
            let receive = receive.clone();
            thread::Builder::new()
                .name(format!("font-warmup-{index}"))
                .spawn(move || {
                    lower_priority();
                    let mut files = 0;
                    let mut faces = 0;
                    for path in receive.iter() {
                        // A fresh database each time keeps nothing between
                        // files; only the count is wanted.
                        let mut database = fontdb::Database::new();
                        if database.load_font_file(&path).is_ok() {
                            faces += database.len();
                        }
                        files += 1;
                    }
                    Report { files, faces }
                })
                .ok()
        })
        .collect();
    drop(receive);
    if workers.is_empty() {
        // No thread could be started: nothing is read, and the toolkit's
        // own scan still works.
        return Report { files: 0, faces: 0 };
    }
    walk(dirs, |path| send.send(path).is_ok());
    drop(send);
    workers
        .into_iter()
        .filter_map(|worker| worker.join().ok())
        .fold(Report { files: 0, faces: 0 }, |total, part| Report {
            files: total.files + part.files,
            faces: total.faces + part.faces,
        })
}

/// What [`warm`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The fonts read as already in memory; nothing more was read.
    AlreadyInMemory,
    /// The font files were read ahead.
    Read(Report),
}

/// Reads the font files under `dirs` ahead with up to `threads` threads,
/// unless a sample shows they are already in memory.
pub fn warm(dirs: &[PathBuf], threads: usize) -> Outcome {
    if already_in_memory(dirs) {
        Outcome::AlreadyInMemory
    } else {
        Outcome::Read(read_all(dirs, threads))
    }
}

/// Starts reading the system's fonts in the background and returns at
/// once; call it before the UI toolkit starts. `done` is called on the
/// background thread with what happened, how long it took and with how
/// many threads. Fails only if no thread could be started.
pub fn start(done: impl FnOnce(Outcome, Duration, usize) + Send + 'static) -> std::io::Result<()> {
    let threads = thread::available_parallelism()
        .map_or(4, usize::from)
        .min(MAX_THREADS);
    thread::Builder::new()
        .name("font-warmup".into())
        .spawn(move || {
            lower_priority();
            let started = Instant::now();
            let dirs = font_dirs(|name| std::env::var_os(name));
            done(warm(&dirs, threads), started.elapsed(), threads);
        })
        .map(drop)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A real font: the UI's bundled Inter.
    fn font_bytes() -> Vec<u8> {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rustphin-ui/assets/fonts");
        let path = std::fs::read_dir(&fonts)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| is_font(path))
            .unwrap();
        std::fs::read(path).unwrap()
    }

    #[test]
    fn walks_the_files_fontdb_loads_in_its_order() {
        let root = tempfile::tempdir().unwrap();
        let font = font_bytes();
        for name in ["a.ttf", "sub/b.OTF", "sub/deeper/c.otc", "z.ttc"] {
            let path = root.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, &font).unwrap();
        }
        std::fs::write(root.path().join("readme.txt"), b"not a font").unwrap();
        std::fs::write(root.path().join("sub/fonts.dir"), b"not a font").unwrap();

        let mut walked = Vec::new();
        walk(&[root.path().to_path_buf()], |path| {
            walked.push(path);
            true
        });

        let mut database = fontdb::Database::new();
        database.load_fonts_dir(root.path());
        let loaded: Vec<PathBuf> = database
            .faces()
            .filter_map(|face| match &face.source {
                fontdb::Source::File(path) => Some(path.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(walked.len(), 4);
        assert_eq!(walked, loaded);
    }

    #[test]
    fn a_folder_linked_into_itself_is_walked_once() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.ttf"), font_bytes()).unwrap();
        std::os::unix::fs::symlink(root.path(), root.path().join("loop")).unwrap();
        let mut walked = Vec::new();
        walk(
            &[root.path().to_path_buf(), root.path().join("loop")],
            |path| {
                walked.push(path);
                true
            },
        );
        assert_eq!(walked, [root.path().join("a.ttf")]);
    }

    #[test]
    fn the_walk_stops_when_asked() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(root.path().join(format!("{index}.ttf")), b"x").unwrap();
        }
        let mut walked = 0;
        walk(&[root.path().to_path_buf()], |_| {
            walked += 1;
            walked < 2
        });
        assert_eq!(walked, 2);
    }

    #[test]
    fn fonts_just_written_are_in_memory_and_not_read_again() {
        let root = tempfile::tempdir().unwrap();
        let font = font_bytes();
        for index in 0..PROBE_FILES {
            std::fs::write(root.path().join(format!("{index}.ttf")), &font).unwrap();
        }
        assert_eq!(
            warm(&[root.path().to_path_buf()], 4),
            Outcome::AlreadyInMemory
        );
    }

    #[test]
    fn reads_every_font_file_across_threads() {
        let root = tempfile::tempdir().unwrap();
        let font = font_bytes();
        for index in 0..20 {
            std::fs::write(root.path().join(format!("{index}.ttf")), &font).unwrap();
        }
        std::fs::write(root.path().join("broken.ttf"), b"not a font").unwrap();
        let report = read_all(&[root.path().to_path_buf()], 4);
        assert_eq!(
            report,
            Report {
                files: 21,
                faces: 20
            }
        );
    }

    #[test]
    fn fontconfig_dirs_are_used_in_their_order() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("fonts.conf");
        std::fs::write(
            &config,
            "<?xml version=\"1.0\"?>\n<fontconfig>\n<dir>/opt/fonts</dir>\n<dir>/srv/more-fonts</dir>\n</fontconfig>\n",
        )
        .unwrap();
        let dirs = font_dirs(|name| match name {
            "FONTCONFIG_FILE" => Some(config.clone().into()),
            "HOME" => Some("/home/someone".into()),
            _ => None,
        });
        assert_eq!(
            dirs,
            [
                PathBuf::from("/opt/fonts"),
                PathBuf::from("/srv/more-fonts")
            ]
        );
    }

    #[test]
    fn without_fontconfig_dirs_the_known_folders_are_used() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("empty.conf");
        std::fs::write(&config, "<?xml version=\"1.0\"?>\n<fontconfig/>\n").unwrap();
        let dirs = font_dirs(|name| match name {
            "FONTCONFIG_FILE" => Some(config.clone().into()),
            "HOME" => Some("/home/someone".into()),
            _ => None,
        });
        assert_eq!(
            dirs,
            [
                PathBuf::from("/usr/share/fonts/"),
                PathBuf::from("/usr/local/share/fonts/"),
                PathBuf::from("/home/someone/.fonts"),
                PathBuf::from("/home/someone/.local/share/fonts"),
            ]
        );
    }
}
