//! Tests for fuzzy string matching utilities
//!
//! Tests extracted from `src/utils/fuzzy_search.rs`

use kodegen_utils::fuzzy_search::{
    get_similarity_ratio, levenshtein_distance, recursive_fuzzy_index_of_with_defaults,
};

#[test]
fn test_levenshtein_distance() {
    assert!((levenshtein_distance("hello", "hello") - 0.0).abs() < f64::EPSILON);
    assert!((levenshtein_distance("hello", "hallo") - 1.0).abs() < f64::EPSILON);
    assert!((levenshtein_distance("", "hello") - 5.0).abs() < f64::EPSILON);
    assert!((levenshtein_distance("hello", "") - 5.0).abs() < f64::EPSILON);
}

#[test]
fn test_get_similarity_ratio() {
    assert!((get_similarity_ratio("hello", "hello") - 1.0).abs() < f64::EPSILON);
    assert!((get_similarity_ratio("", "") - 1.0).abs() < f64::EPSILON);
    assert!(get_similarity_ratio("hello", "hallo") >= 0.8);
    assert!(get_similarity_ratio("hello", "world") < 0.5);
}

#[test]
fn test_fuzzy_search_exact_match() {
    let text = "The quick brown fox jumps over the lazy dog";
    let query = "quick";

    let result = recursive_fuzzy_index_of_with_defaults(text, query);

    assert_eq!(result.value, "quick");
    assert!((result.distance - 0.0).abs() < f64::EPSILON);
    assert!(result.start <= result.end);
}

#[test]
fn test_fuzzy_search_partial_match() {
    let text = "The qwick brown fox";
    let query = "quick";

    let result = recursive_fuzzy_index_of_with_defaults(text, query);

    assert!(result.distance > 0.0);
    assert!(result.value.contains("qwick") || result.value.contains("quick"));
}

#[test]
fn test_empty_strings() {
    let result = recursive_fuzzy_index_of_with_defaults("", "");
    assert_eq!(result.value, "");
    assert!((result.distance - 0.0).abs() < f64::EPSILON);
}
