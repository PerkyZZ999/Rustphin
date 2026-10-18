//! Fuzzy ranking for the command palette, independent of its views and actions.

/// A query normalised once and reused across all candidates.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    text: String,
}

/// A match's rank. Smaller values are better; equal values keep input order.
/// Exact names precede prefixes, word substrings, other substrings and subsequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatchScore {
    kind: u8,
    gaps: usize,
    start: usize,
    length: usize,
}

impl SearchQuery {
    /// Trims surrounding whitespace and matches without regard to case.
    /// Unicode lowercasing is supported; accents are not stripped.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.trim().to_lowercase(),
        }
    }

    /// Scores one display label. An empty query matches every label equally.
    /// Matching is linear in the candidate for subsequences and uses no matrix.
    pub fn score(&self, candidate: &str) -> Option<MatchScore> {
        if self.text.is_empty() {
            return Some(MatchScore {
                kind: 0,
                gaps: 0,
                start: 0,
                length: 0,
            });
        }
        let candidate = candidate.to_lowercase();
        let length = candidate.len();
        if let Some(start) = candidate.find(&self.text) {
            let kind = if candidate == self.text {
                0
            } else if start == 0 {
                1
            } else if candidate[..start]
                .chars()
                .next_back()
                .is_some_and(|ch| !ch.is_alphanumeric())
            {
                2
            } else {
                3
            };
            return Some(MatchScore {
                kind,
                gaps: 0,
                start,
                length,
            });
        }

        let mut query = self.text.chars().peekable();
        let mut first = None;
        let mut matched = 0;
        for (index, ch) in candidate.chars().enumerate() {
            if query.peek() == Some(&ch) {
                query.next();
                let start = *first.get_or_insert(index);
                matched += 1;
                if query.peek().is_none() {
                    return Some(MatchScore {
                        kind: 4,
                        gaps: index - start + 1 - matched,
                        start,
                        length,
                    });
                }
            }
        }
        None
    }

    /// Returns indices of the best matching labels, up to `limit`.
    /// Equal scores preserve candidate order, so callers can supply recency
    /// or category order without embedding UI concerns in the matcher.
    pub fn rank<'a>(
        &self,
        candidates: impl IntoIterator<Item = &'a str>,
        limit: usize,
    ) -> Vec<usize> {
        if limit == 0 {
            return Vec::new();
        }
        let mut matches: Vec<_> = candidates
            .into_iter()
            .enumerate()
            .filter_map(|(index, text)| self.score(text).map(|score| (score, index)))
            .collect();
        matches.sort_unstable();
        matches.truncate(limit);
        matches.into_iter().map(|(_, index)| index).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_exact_prefix_word_substring_and_subsequence_in_that_order() {
        let labels = [
            "r-e-n-a-m-e",
            "prerename",
            "Batch rename",
            "Rename file",
            "Rename",
        ];
        assert_eq!(SearchQuery::new("rename").rank(labels, 10), [4, 3, 2, 1, 0]);
    }

    #[test]
    fn matches_unicode_case_and_ignores_surrounding_query_whitespace() {
        assert_eq!(
            SearchQuery::new("  ÉTÉ  ").rank(["notes", "Été", "été photos"], 10),
            [1, 2]
        );
    }

    #[test]
    fn requires_every_character_in_order_including_repeated_characters() {
        assert_eq!(
            SearchQuery::new("bkk").rank(["bk", "kkb", "bookkeeper"], 10),
            [2]
        );
    }

    #[test]
    fn prefers_a_compact_subsequence_to_a_scattered_one() {
        assert_eq!(
            SearchQuery::new("abc").rank(["a--b--c", "a-bc very long label"], 10),
            [1, 0]
        );
    }

    #[test]
    fn preserves_input_order_for_ties_and_empty_queries() {
        assert_eq!(SearchQuery::new(" ").rank(["Z", "A", "B"], 2), [0, 1]);
        assert_eq!(SearchQuery::new("a").rank(["Alpha", "Apple"], 2), [0, 1]);
    }

    #[test]
    fn excludes_nonmatches_and_honours_the_result_limit() {
        assert_eq!(
            SearchQuery::new("file").rank(["File", "Folder", "Files"], 1),
            [0]
        );
        assert!(SearchQuery::new("absent").rank([""], 5).is_empty());
        assert!(SearchQuery::new("").rank(["anything"], 0).is_empty());
    }
}
