//! Sorting functionality for search results
//!
//! This module provides sorting capabilities for search results based on various criteria
//! such as file path, modification time, access time, and creation time.

use std::cmp::Ordering;
use std::time::SystemTime;

use super::types::SearchResult;

/// Sort criterion for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort alphabetically by file path
    Path,
    /// Sort by last modified time
    Modified,
    /// Sort by last accessed time
    Accessed,
    /// Sort by creation time
    Created,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order (oldest first for time, A-Z for path)
    Ascending,
    /// Descending order (newest first for time, Z-A for path)
    Descending,
}

/// Sort search results in place based on specified criterion and direction
///
/// # Arguments
/// * `results` - Mutable slice of search results to sort
/// * `sort_by` - The criterion to sort by
/// * `direction` - The sort direction (ascending or descending)
///
/// # Platform Notes
/// - `modified` timestamp is available on all platforms
/// - `accessed` timestamp is available on most platforms but may be None
/// - `created` timestamp is available on Windows, may be None on some Unix systems
///
/// Results with missing timestamps are sorted to the end (ascending) or beginning (descending)
pub fn sort_results(results: &mut [SearchResult], sort_by: SortBy, direction: SortDirection) {
    results.sort_by(|a, b| {
        let ordering = match sort_by {
            SortBy::Path => compare_paths(&a.file, &b.file),
            SortBy::Modified => compare_optional_times(&a.modified, &b.modified),
            SortBy::Accessed => compare_optional_times(&a.accessed, &b.accessed),
            SortBy::Created => compare_optional_times(&a.created, &b.created),
        };

        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

/// Compare two file paths alphabetically
fn compare_paths(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

/// Compare two optional timestamps
///
/// Ordering logic:
/// - Both Some: Compare normally
/// - a is Some, b is None: a comes first (has timestamp is "less")
/// - a is None, b is Some: b comes first
/// - Both None: Equal (maintain original order)
fn compare_optional_times(a: &Option<SystemTime>, b: &Option<SystemTime>) -> Ordering {
    match (a, b) {
        (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
        (Some(_), None) => Ordering::Less, // Has timestamp comes first
        (None, Some(_)) => Ordering::Greater, // Missing timestamp goes last
        (None, None) => Ordering::Equal,   // Both missing, maintain order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn create_test_result(file: &str, modified: Option<SystemTime>) -> SearchResult {
        SearchResult {
            file: file.to_string(),
            line: None,
            r#match: None,
            r#type: super::super::types::SearchResultType::File,
            is_context: false,
            is_binary: None,
            binary_suppressed: None,
            modified,
            accessed: None,
            created: None,
        }
    }

    #[test]
    fn test_sort_by_path_ascending() {
        let mut results = vec![
            create_test_result("c.rs", None),
            create_test_result("a.rs", None),
            create_test_result("b.rs", None),
        ];

        sort_results(&mut results, SortBy::Path, SortDirection::Ascending);

        assert_eq!(results[0].file, "a.rs");
        assert_eq!(results[1].file, "b.rs");
        assert_eq!(results[2].file, "c.rs");
    }

    #[test]
    fn test_sort_by_path_descending() {
        let mut results = vec![
            create_test_result("a.rs", None),
            create_test_result("c.rs", None),
            create_test_result("b.rs", None),
        ];

        sort_results(&mut results, SortBy::Path, SortDirection::Descending);

        assert_eq!(results[0].file, "c.rs");
        assert_eq!(results[1].file, "b.rs");
        assert_eq!(results[2].file, "a.rs");
    }

    #[test]
    fn test_sort_by_modified_ascending() {
        let now = SystemTime::now();
        let old = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);
        let mid = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);

        let mut results = vec![
            create_test_result("new.rs", Some(now)),
            create_test_result("old.rs", Some(old)),
            create_test_result("mid.rs", Some(mid)),
        ];

        sort_results(&mut results, SortBy::Modified, SortDirection::Ascending);

        assert_eq!(results[0].file, "old.rs");
        assert_eq!(results[1].file, "mid.rs");
        assert_eq!(results[2].file, "new.rs");
    }

    #[test]
    fn test_sort_by_modified_descending() {
        let now = SystemTime::now();
        let old = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);
        let mid = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);

        let mut results = vec![
            create_test_result("old.rs", Some(old)),
            create_test_result("new.rs", Some(now)),
            create_test_result("mid.rs", Some(mid)),
        ];

        sort_results(&mut results, SortBy::Modified, SortDirection::Descending);

        assert_eq!(results[0].file, "new.rs");
        assert_eq!(results[1].file, "mid.rs");
        assert_eq!(results[2].file, "old.rs");
    }

    #[test]
    fn test_sort_handles_missing_metadata() {
        let now = SystemTime::now();

        let mut results = vec![
            create_test_result("has_time.rs", Some(now)),
            create_test_result("no_time.rs", None),
            create_test_result("also_has_time.rs", Some(now)),
        ];

        // Ascending: files with timestamps should come before files without
        sort_results(&mut results, SortBy::Modified, SortDirection::Ascending);

        assert!(results[0].modified.is_some());
        assert!(results[1].modified.is_some());
        assert!(results[2].modified.is_none());
        assert_eq!(results[2].file, "no_time.rs");
    }

    #[test]
    fn test_sort_respects_direction() {
        let now = SystemTime::now();
        let old = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);

        let mut results_asc = vec![
            create_test_result("new.rs", Some(now)),
            create_test_result("old.rs", Some(old)),
        ];

        let mut results_desc = results_asc.clone();

        sort_results(&mut results_asc, SortBy::Modified, SortDirection::Ascending);
        sort_results(
            &mut results_desc,
            SortBy::Modified,
            SortDirection::Descending,
        );

        // Verify reversed order
        assert_eq!(results_asc[0].file, "old.rs");
        assert_eq!(results_asc[1].file, "new.rs");
        assert_eq!(results_desc[0].file, "new.rs");
        assert_eq!(results_desc[1].file, "old.rs");
    }
}
