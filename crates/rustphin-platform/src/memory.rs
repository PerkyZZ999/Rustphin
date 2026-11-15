//! How the C allocator keeps memory (W-115).
//!
//! glibc gives threads their own allocation arenas, up to eight per core.
//! A large listing is built on the listing and CPU threads and freed on
//! another, and the freed space stays in the arena that built it: reloading
//! a 500,000-entry folder grew Rustphin by about 11 MB each time, up to
//! about 175 MB after 20 reloads on 16 cores, although nothing was still in
//! use. With two arenas the same reloads stay at about 42 MB, and listing
//! and sorting are as fast (VALIDATION.md, W-115).

/// The arenas the allocator may use.
pub const ARENAS: i32 = 2;

/// Limits the C allocator to [`ARENAS`] arenas. Call it first in `main`,
/// before any thread starts; it does nothing outside glibc. `false` when
/// the allocator refused.
pub fn limit_arenas() -> bool {
    #[cfg(target_env = "gnu")]
    {
        // SAFETY: `mallopt` takes two integers and only changes the
        // allocator's tuning; M_ARENA_MAX is a parameter glibc documents.
        unsafe { libc::mallopt(libc::M_ARENA_MAX, ARENAS) == 1 }
    }
    #[cfg(not(target_env = "gnu"))]
    {
        true
    }
}

#[cfg(all(test, target_env = "gnu"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        ptr,
        sync::{Arc, Barrier},
        thread,
    };

    use super::*;

    /// The arenas the allocator has made so far, from its own report.
    fn arenas() -> usize {
        let mut buffer: *mut libc::c_char = ptr::null_mut();
        let mut size: libc::size_t = 0;
        // SAFETY: open_memstream writes a buffer pointer and its size into
        // the two locals, which outlive the stream; the stream is closed
        // before the buffer is read and the buffer freed once copied.
        let report = unsafe {
            let stream = libc::open_memstream(&raw mut buffer, &raw mut size);
            assert!(!stream.is_null());
            assert_eq!(libc::malloc_info(0, stream), 0);
            libc::fclose(stream);
            let bytes = std::slice::from_raw_parts(buffer.cast::<u8>(), size).to_vec();
            libc::free(buffer.cast());
            bytes
        };
        String::from_utf8_lossy(&report)
            .matches("<heap nr=")
            .count()
    }

    // nextest runs each test in its own process, so the limit set here
    // reaches no other test.
    #[test]
    fn many_threads_allocating_at_once_share_two_arenas() {
        assert!(limit_arenas());
        let threads = 16;
        let all_allocated = Arc::new(Barrier::new(threads));
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                let all_allocated = Arc::clone(&all_allocated);
                thread::spawn(move || {
                    let held: Vec<Vec<u8>> = (0..256).map(|n| vec![1; 64 + n]).collect();
                    // Every thread holds its memory until all have allocated,
                    // so none can hand its arena to the next.
                    all_allocated.wait();
                    held.len()
                })
            })
            .collect();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), 256);
        }
        assert!(arenas() <= ARENAS as usize, "{} arenas", arenas());
    }
}
