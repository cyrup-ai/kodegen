//! Character-level diff for showing precise differences between strings
//!
//! Provides visual diff in format: `prefix{-removed-}{+added+}suffix`

/// Character-level diff result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharDiff {
    pub common_prefix: String,
    pub expected_part: String,
    pub actual_part: String,
    pub common_suffix: String,
}

impl CharDiff {
    /// Find character-level diff between expected and actual strings
    ///
    /// # Examples
    ///
    /// ```
    /// use kodegen_utils::char_diff::CharDiff;
    ///
    /// let diff = CharDiff::new("function getUserData()", "function  getUserData()");
    /// assert_eq!(diff.format(), "function {--}{+ +}getUserData()");
    /// ```
    #[must_use]
    pub fn new(expected: &str, actual: &str) -> Self {
        // 1. Find common prefix length (char-by-char comparison)
        let prefix_len = Self::find_common_prefix(expected, actual);

        // 2. Find common suffix length (excluding prefix chars)
        let suffix_len = Self::find_common_suffix(expected, actual, prefix_len);

        // 3. Extract parts
        let common_prefix = expected[..prefix_len].to_string();
        let common_suffix = expected[expected.len() - suffix_len..].to_string();
        let expected_part = expected[prefix_len..expected.len() - suffix_len].to_string();
        let actual_part = actual[prefix_len..actual.len() - suffix_len].to_string();

        Self {
            common_prefix,
            expected_part,
            actual_part,
            common_suffix,
        }
    }

    /// Find length of common prefix between two strings
    fn find_common_prefix(a: &str, b: &str) -> usize {
        a.char_indices()
            .zip(b.chars())
            .take_while(|((_, ca), cb)| ca == cb)
            .last()
            .map_or(0, |((idx, c), _)| idx + c.len_utf8())
    }

    /// Find length of common suffix (excluding `prefix_len` chars from start)
    fn find_common_suffix(a: &str, b: &str, prefix_len: usize) -> usize {
        let a_suffix = &a[prefix_len..];
        let b_suffix = &b[prefix_len..];

        // Collect (byte_idx, char) pairs from the END
        let a_chars_rev: Vec<_> = a_suffix.char_indices().collect();
        let b_chars_rev: Vec<_> = b_suffix.char_indices().collect();

        // Compare from the end
        let matching_chars = a_chars_rev
            .iter()
            .rev()
            .zip(b_chars_rev.iter().rev())
            .take_while(|((_, ca), (_, cb))| ca == cb)
            .count();

        if matching_chars == 0 {
            return 0;
        }

        // Return BYTE length of matching suffix
        if matching_chars == a_chars_rev.len() {
            a_suffix.len() // All chars match
        } else {
            let first_non_matching_idx = a_chars_rev.len() - matching_chars;
            let (byte_idx, _) = a_chars_rev[first_non_matching_idx];
            a_suffix.len() - byte_idx
        }
    }

    /// Format as standard diff: `prefix{-old-}{+new+}suffix`
    #[must_use]
    pub fn format(&self) -> String {
        format!(
            "{}{{-{}-}}{{+{}+}}{}",
            self.common_prefix, self.expected_part, self.actual_part, self.common_suffix
        )
    }

    /// Check if diff is whitespace-only
    #[must_use]
    pub fn is_whitespace_only(&self) -> bool {
        self.expected_part.trim() == self.actual_part.trim()
    }
}
