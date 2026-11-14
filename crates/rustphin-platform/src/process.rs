//! Other running copies of Rustphin, and this one's memory, read from `/proc`.

use std::{fs, path::Path};

/// Whether a live process other than this one runs the program `name`,
/// judged by `/proc/<pid>/stat`. Zombie and dead processes do not count: a
/// killed Rustphin may wait there for its parent. Unreadable entries are
/// skipped, so processes this user may not inspect are missed.
pub fn others_running(name: &str) -> bool {
    let own = std::process::id();
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    let processes = entries.flatten().filter_map(|entry| {
        let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
        let stat =
            fs::read_to_string(Path::new("/proc").join(pid.to_string()).join("stat")).ok()?;
        let (comm, state) = parse_stat(&stat)?;
        Some((pid, comm.to_owned(), state))
    });
    any_other(processes, own, name)
}

/// This process's own memory in bytes: `RssAnon` in `/proc/self/status`,
/// the memory nobody else shares and the kernel cannot simply drop.
pub fn own_memory() -> Option<u64> {
    rss_anon(&fs::read_to_string("/proc/self/status").ok()?)
}

/// `RssAnon` of a `/proc/<pid>/status` text, in bytes.
fn rss_anon(status: &str) -> Option<u64> {
    let line = status.lines().find(|line| line.starts_with("RssAnon:"))?;
    let kilobytes: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kilobytes.checked_mul(1024)
}

/// The program name and state letter of a `/proc/<pid>/stat` line:
/// `pid (comm) S …`, where the name may itself hold spaces or parentheses.
fn parse_stat(stat: &str) -> Option<(&str, char)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let comm = stat.get(open + 1..close)?;
    let state = stat.get(close + 1..)?.trim_start().chars().next()?;
    Some((comm, state))
}

/// Whether `processes` (pid, name, state) hold a live `name` under another pid.
fn any_other(
    processes: impl IntoIterator<Item = (u32, String, char)>,
    own: u32,
    name: &str,
) -> bool {
    processes
        .into_iter()
        .any(|(pid, comm, state)| pid != own && comm == name && !matches!(state, 'Z' | 'X' | 'x'))
}

#[cfg(test)]
mod tests {

    #[test]
    fn own_memory_is_the_anonymous_resident_size() {
        let status =
            "Name:\trustphin\nVmRSS:\t  139504 kB\nRssAnon:\t   40120 kB\nRssFile:\t   99384 kB\n";
        assert_eq!(rss_anon(status), Some(40_120 * 1024));
        assert_eq!(rss_anon("Name:\trustphin\n"), None);
        assert!(own_memory().is_some_and(|bytes| bytes > 0));
    }
    use super::*;

    #[test]
    fn only_another_live_process_with_the_exact_name_counts() {
        let list = |items: &[(u32, &str, char)]| {
            items
                .iter()
                .map(|(pid, comm, state)| (*pid, (*comm).to_owned(), *state))
                .collect::<Vec<_>>()
        };
        assert!(!any_other(list(&[(7, "rustphin", 'S')]), 7, "rustphin"));
        assert!(any_other(
            list(&[(7, "rustphin", 'R'), (9, "rustphin", 'S')]),
            7,
            "rustphin"
        ));
        assert!(!any_other(list(&[(9, "rustphin", 'Z')]), 7, "rustphin"));
        assert!(!any_other(
            list(&[(9, "rustphin-ui", 'S'), (10, "dolphin", 'S')]),
            7,
            "rustphin"
        ));
    }

    #[test]
    fn stat_lines_give_the_name_and_state_even_with_odd_names() {
        assert_eq!(parse_stat("42 (rustphin) S 1 42"), Some(("rustphin", 'S')));
        assert_eq!(parse_stat("43 (a) b (c)) Z 1"), Some(("a) b (c)", 'Z')));
        assert_eq!(parse_stat("garbage"), None);
    }
}
