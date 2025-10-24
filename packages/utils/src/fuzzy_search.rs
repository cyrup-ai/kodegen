//! Fuzzy string matching utilities using Levenshtein distance
//!
//! Provides algorithms for finding approximate string matches within text,
//! useful for error correction, search features, and text comparison.

use std::cmp;

// ============================================================================
// PUBLIC TYPES
// ============================================================================

/// Result of a fuzzy search operation
#[derive(Debug, Clone)]
pub struct FuzzySearchResult {
    /// Start index of the match in the text
    pub start: usize,

    /// End index of the match in the text
    pub end: usize,

    /// The matched substring
    pub value: String,

    /// Levenshtein distance (number of edits needed)
    pub distance: f64,
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Safe substring that mimics JavaScript's substring behavior
/// - Swaps start and end if start > end
/// - Clamps indices to valid bounds
/// - Returns empty string for invalid ranges
fn safe_substring(text: &str, start: usize, end: usize) -> &str {
    let len = text.len();
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };
    let start = cmp::min(start, len);
    let end = cmp::min(end, len);
    if start >= end { "" } else { &text[start..end] }
}

// ============================================================================
// CORE ALGORITHMS
// ============================================================================

/// Calculate Levenshtein distance between two strings
///
/// Returns the minimum number of single-character edits (insertions,
/// deletions, or substitutions) needed to transform string `a` into string `b`.
///
/// # Examples
///
/// ```
/// use kodegen_utils::fuzzy_search::levenshtein_distance;
///
/// assert!((levenshtein_distance("hello", "hello") - 0.0).abs() < f64::EPSILON);
/// assert!((levenshtein_distance("hello", "hallo") - 1.0).abs() < f64::EPSILON);
/// assert!((levenshtein_distance("", "hello") - 5.0).abs() < f64::EPSILON);
/// ```
#[must_use]
pub fn levenshtein_distance(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    // Return early for empty strings
    // Cast string lengths to f64: exact for lengths < 2^52, acceptable loss for impossible sizes
    if a_len == 0 {
        return b_len as f64;
    }
    if b_len == 0 {
        return a_len as f64;
    }

    // Dynamic programming matrix using f64 to avoid precision loss warnings
    // f64 can exactly represent integers up to 2^52 (sufficient for realistic string lengths)
    let mut matrix = vec![vec![0.0; b_len + 1]; a_len + 1];

    // Initialize first row and column with distances
    // Cast indices to f64: exact for all realistic string lengths < 2^52 chars
    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i as f64;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *cell = j as f64;
    }

    // Fill matrix
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0.0
            } else {
                1.0
            };
            matrix[i][j] = (matrix[i - 1][j] + 1.0) // deletion
                .min(matrix[i][j - 1] + 1.0) // insertion
                .min(matrix[i - 1][j - 1] + cost); // substitution
        }
    }

    matrix[a_len][b_len]
}

/// Iteratively refines the best match by reducing the search area
fn iterative_reduction(
    text: &str,
    query: &str,
    start: usize,
    end: usize,
    parent_distance: f64,
) -> FuzzySearchResult {
    let mut best_distance = parent_distance;
    let mut best_start = start;
    let mut best_end = end;

    // Improve start position
    let next_text = safe_substring(text, best_start + 1, best_end);
    let mut next_distance = levenshtein_distance(next_text, query);

    while next_distance < best_distance {
        best_distance = next_distance;
        best_start += 1;

        let smaller_text = safe_substring(text, best_start + 1, best_end);
        next_distance = levenshtein_distance(smaller_text, query);
    }

    // Improve end position
    let next_text = safe_substring(text, best_start, best_end.saturating_sub(1));
    let mut next_distance = levenshtein_distance(next_text, query);

    while next_distance < best_distance {
        best_distance = next_distance;
        best_end = best_end.saturating_sub(1);

        let smaller_text = safe_substring(text, best_start, best_end.saturating_sub(1));
        next_distance = levenshtein_distance(smaller_text, query);
    }

    FuzzySearchResult {
        start: best_start,
        end: best_end,
        value: safe_substring(text, best_start, best_end).to_string(),
        distance: best_distance,
    }
}

/// Recursively finds the closest match to a query string within text using fuzzy matching
///
/// Uses a binary-search-like approach to efficiently find the best matching substring.
///
/// # Arguments
///
/// * `text` - The text to search within
/// * `query` - The pattern to search for
/// * `start` - Starting index in text
/// * `end` - Optional ending index (None = end of text)
/// * `parent_distance` - Best distance found so far
///
/// # Examples
///
/// ```
/// use kodegen_utils::fuzzy_search::recursive_fuzzy_index_of;
///
/// let text = "The quick brown fox";
/// let result = recursive_fuzzy_index_of(text, "qwick", 0, None, f64::INFINITY);
/// assert!(result.distance > 0.0); // Not an exact match
/// ```
#[must_use]
pub fn recursive_fuzzy_index_of(
    text: &str,
    query: &str,
    start: usize,
    end: Option<usize>,
    parent_distance: f64,
) -> FuzzySearchResult {
    let end = end.unwrap_or(text.len());

    // For small text segments, use iterative approach
    if end.saturating_sub(start) <= 2 * query.len() {
        return iterative_reduction(text, query, start, end, parent_distance);
    }

    let mid_point = start + (end - start) / 2;
    let left_end = cmp::min(end, mid_point + query.len()); // Include query length to cover overlaps
    let right_start = cmp::max(start, mid_point.saturating_sub(query.len())); // Include query length to cover overlaps

    // Calculate distance for current segments
    let left_text = safe_substring(text, start, left_end);
    let right_text = safe_substring(text, right_start, end);

    let left_distance = levenshtein_distance(left_text, query);
    let right_distance = levenshtein_distance(right_text, query);
    let best_distance = left_distance.min(parent_distance.min(right_distance));

    // If parent distance is already the best, use iterative approach
    // Use epsilon comparison for f64 to avoid precision issues
    if (parent_distance - best_distance).abs() < f64::EPSILON {
        return iterative_reduction(text, query, start, end, parent_distance);
    }

    // Recursively search the better half
    if left_distance < right_distance {
        recursive_fuzzy_index_of(text, query, start, Some(left_end), best_distance)
    } else {
        recursive_fuzzy_index_of(text, query, right_start, Some(end), best_distance)
    }
}

/// Public interface with sensible defaults
///
/// # Examples
///
/// ```
/// use kodegen_utils::fuzzy_search::recursive_fuzzy_index_of_with_defaults;
///
/// let text = "The quick brown fox jumps over the lazy dog";
/// let result = recursive_fuzzy_index_of_with_defaults(text, "quick");
/// assert_eq!(result.value, "quick");
/// assert!((result.distance - 0.0).abs() < f64::EPSILON);
/// ```
#[must_use]
pub fn recursive_fuzzy_index_of_with_defaults(text: &str, query: &str) -> FuzzySearchResult {
    recursive_fuzzy_index_of(text, query, 0, None, f64::INFINITY)
}

/// Calculate similarity ratio between two strings
///
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
///
/// # Examples
///
/// ```
/// use kodegen_utils::fuzzy_search::get_similarity_ratio;
///
/// assert_eq!(get_similarity_ratio("hello", "hello"), 1.0);
/// assert!(get_similarity_ratio("hello", "hallo") >= 0.8);
/// ```
#[must_use]
pub fn get_similarity_ratio(a: &str, b: &str) -> f64 {
    let max_length = cmp::max(a.len(), b.len());
    if max_length == 0 {
        return 1.0; // Both strings are empty
    }

    let distance = levenshtein_distance(a, b);

    // Cast string length to f64 for ratio calculation
    let max_length_f64 = max_length as f64;
    1.0 - (distance / max_length_f64)
}
