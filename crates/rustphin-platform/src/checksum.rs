//! A file's checksums for Properties › Checksums (W-170): MD5, SHA1,
//! SHA256 and SHA512, read in steps so the caller can follow the progress
//! and stop it. Call from a background thread.

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

/// Bytes read per step; progress and the stop are checked once per step.
const STEP: usize = 1024 * 1024;

/// The checksums Properties offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// MD5, 32 hexadecimal digits.
    Md5,
    /// SHA-1, 40 digits.
    Sha1,
    /// SHA-256, 64 digits.
    Sha256,
    /// SHA-512, 128 digits.
    Sha512,
}

/// Calculates `algorithm` over the file at `path` as lowercase hexadecimal.
/// `done` counts the bytes read so far. `Ok(None)` when `stop` was set.
pub fn calculate(
    path: &Path,
    algorithm: Algorithm,
    stop: &AtomicBool,
    done: &AtomicU64,
) -> io::Result<Option<String>> {
    let file = File::open(path)?;
    match algorithm {
        Algorithm::Md5 => digest::<Md5>(file, stop, done),
        Algorithm::Sha1 => digest::<Sha1>(file, stop, done),
        Algorithm::Sha256 => digest::<Sha256>(file, stop, done),
        Algorithm::Sha512 => digest::<Sha512>(file, stop, done),
    }
}

fn digest<D: Digest>(
    mut file: impl Read,
    stop: &AtomicBool,
    done: &AtomicU64,
) -> io::Result<Option<String>> {
    let mut hasher = D::new();
    let mut buffer = vec![0u8; STEP];
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let read = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        hasher.update(&buffer[..read]);
        done.fetch_add(read as u64, Ordering::Relaxed);
    }
    Ok(Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn of(content: &[u8], algorithm: Algorithm) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, content).unwrap();
        let done = AtomicU64::new(0);
        let sum = calculate(&path, algorithm, &AtomicBool::new(false), &done)
            .unwrap()
            .unwrap();
        assert_eq!(done.load(Ordering::Relaxed), content.len() as u64);
        sum
    }

    /// The published test vectors for "abc".
    #[test]
    fn each_checksum_matches_its_published_value() {
        assert_eq!(
            of(b"abc", Algorithm::Md5),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            of(b"abc", Algorithm::Sha1),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            of(b"abc", Algorithm::Sha256),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            of(b"abc", Algorithm::Sha512),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn a_file_larger_than_a_step_is_read_whole() {
        let big = vec![7u8; STEP * 2 + 5];
        assert_eq!(of(&big, Algorithm::Md5).len(), 32);
    }

    #[test]
    fn a_stop_ends_the_calculation_without_a_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"abc").unwrap();
        let stopped = calculate(
            &path,
            Algorithm::Sha256,
            &AtomicBool::new(true),
            &AtomicU64::new(0),
        );
        assert_eq!(stopped.unwrap(), None);
        let missing = calculate(
            &dir.path().join("gone"),
            Algorithm::Md5,
            &AtomicBool::new(false),
            &AtomicU64::new(0),
        );
        assert!(missing.is_err());
    }
}
