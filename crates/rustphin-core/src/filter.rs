//! The name filter of the filter bar, matching as Dolphin's does.

/// Shows only entries whose name matches a pattern, as Dolphin's filter bar
/// does by default (`KFileItemModelFilter` in glob mode): case-insensitive,
/// anywhere in the name, with `*` for any run of characters and `?` for one.
/// "txt" matches `notes.txt` and `TXT-files`; "a*z" matches `abz` and `xaz.1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameFilter {
    text: String,
    /// Lower-cased, wrapped in `*` so a match may start and end anywhere.
    pattern: Vec<char>,
}

impl NameFilter {
    /// A filter for `text`, or `None` when it is empty and filters nothing.
    pub fn new(text: &str) -> Option<Self> {
        if text.is_empty() {
            return None;
        }
        let mut pattern = vec!['*'];
        pattern.extend(text.to_lowercase().chars());
        pattern.push('*');
        Some(Self {
            text: text.to_owned(),
            pattern,
        })
    }

    /// The text as typed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether `name` is shown. Bytes that are not UTF-8 are compared as
    /// replacement characters, so they never match a typed character.
    pub fn matches(&self, name: &[u8]) -> bool {
        let name: Vec<char> = String::from_utf8_lossy(name)
            .to_lowercase()
            .chars()
            .collect();
        glob(&self.pattern, &name)
    }
}

/// Matches `text` against a pattern of `*`, `?` and literal characters,
/// backtracking only to the last `*` (linear in practice, never exponential).
fn glob(pattern: &[char], text: &[char]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p, t));
                p += 1;
            }
            Some('?') => {
                p += 1;
                t += 1;
            }
            Some(&c) if c == text[t] => {
                p += 1;
                t += 1;
            }
            _ => match star {
                Some((star_p, star_t)) => {
                    p = star_p + 1;
                    t = star_t + 1;
                    star = Some((star_p, star_t + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn matches(filter: &str, name: &str) -> bool {
        NameFilter::new(filter).unwrap().matches(name.as_bytes())
    }

    #[test]
    fn plain_text_matches_anywhere_ignoring_case() {
        assert!(matches("txt", "notes.txt"));
        assert!(matches("TXT", "notes.txt"));
        assert!(matches("note", "My NOTES"));
        assert!(matches("été", "Été 2026"));
        assert!(!matches("txt", "notes.md"));
    }

    #[test]
    fn wildcards_match_runs_and_single_characters_anywhere() {
        assert!(matches("*.txt", "a.txt"));
        assert!(matches("*.txt", "a.txt.bak"), "unanchored, as Dolphin");
        assert!(matches("a*z", "xx-abz-yy"));
        assert!(!matches("a*z", "za"));
        assert!(matches("file?.txt", "file2.txt"));
        assert!(!matches("file?.txt", "file10.txt"));
        assert!(matches("**", "anything"));
    }

    #[test]
    fn an_empty_filter_filters_nothing_and_raw_bytes_never_panic() {
        assert!(NameFilter::new("").is_none());
        assert!(!matches("a", "\u{fffd}"));
        assert!(NameFilter::new("x").unwrap().matches(b"\xffx\xfe"));
        assert!(!NameFilter::new("y").unwrap().matches(b"\xff\xfe"));
    }
}
