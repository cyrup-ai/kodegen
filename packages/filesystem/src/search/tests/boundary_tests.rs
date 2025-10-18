//! Unit tests for word boundary mode implementation
//!
//! These tests verify that word boundary matching works correctly
//! for both literal and regex patterns.

use crate::search::rg::build_rust_matcher;
use crate::search::types::CaseMode;
use grep::matcher::Matcher;

#[test]
fn test_word_boundary_literal_search() {
    // Test literal string with word boundary
    // Pattern "test" with literal_search=true, word_boundary=true
    // Should be wrapped as \btest\b with special chars escaped
    
    let matcher = build_rust_matcher("test", CaseMode::Sensitive, true, true)
        .expect("Failed to build matcher");
    
    // Should match: "test()", "test ", "test."
    assert!(matcher.is_match(b"test()").expect("Test operation should succeed"), "Should match 'test()'");
    assert!(matcher.is_match(b"test ").expect("Match check failed"), "Should match 'test '");
    assert!(matcher.is_match(b"test.").expect("Match check failed"), "Should match 'test.'");
    assert!(matcher.is_match(b"test").expect("Match check failed"), "Should match 'test'");
    
    // Should NOT match: "testing", "attest", "fastest"
    assert!(!matcher.is_match(b"testing").expect("Match check failed"), "Should not match 'testing'");
    assert!(!matcher.is_match(b"attest").expect("Match check failed"), "Should not match 'attest'");
    assert!(!matcher.is_match(b"fastest").expect("Match check failed"), "Should not match 'fastest'");
    assert!(!matcher.is_match(b"libtest").expect("Match check failed"), "Should not match 'libtest'");
}

#[test]
fn test_word_boundary_regex_search() {
    // Test regex pattern with word boundary
    // Pattern "test.*" with literal_search=false, word_boundary=true
    // Should be wrapped as \b(?:test.*)\b
    
    let matcher = build_rust_matcher("test.*", CaseMode::Sensitive, false, true)
        .expect("Failed to build matcher");
    
    // Should match: "test", "testing", "tester"
    assert!(matcher.is_match(b"test").expect("Match check failed"), "Should match 'test'");
    assert!(matcher.is_match(b"testing word").expect("Match check failed"), "Should match 'testing' at word boundary");
    assert!(matcher.is_match(b"tester ").expect("Match check failed"), "Should match 'tester'");
    
    // Should NOT match: "attest" (test not at word boundary)
    assert!(!matcher.is_match(b"attest").expect("Match check failed"), "Should not match 'attest'");
    assert!(!matcher.is_match(b"libtest").expect("Match check failed"), "Should not match 'libtest'");
}

#[test]
fn test_word_boundary_preserves_existing_boundaries() {
    // Test that existing \b in pattern is not double-wrapped
    // Pattern "\btest\b" with word_boundary=true
    // Should NOT become \b(?:\btest\b)\b
    
    let matcher = build_rust_matcher(r"\btest\b", CaseMode::Sensitive, false, true)
        .expect("Failed to build matcher");
    
    // Should match: "test" at word boundaries
    assert!(matcher.is_match(b"test").expect("Match check failed"), "Should match 'test'");
    assert!(matcher.is_match(b"test ").expect("Match check failed"), "Should match 'test '");
    
    // Should NOT match: "testing", "attest"
    assert!(!matcher.is_match(b"testing").expect("Match check failed"), "Should not match 'testing'");
    assert!(!matcher.is_match(b"attest").expect("Match check failed"), "Should not match 'attest'");
}

#[test]
fn test_substring_mode_default() {
    // Test that without word_boundary, substring matching works
    // Pattern "test" with literal_search=false, word_boundary=false
    // Should match anywhere (substring mode)
    
    let matcher = build_rust_matcher("test", CaseMode::Sensitive, false, false)
        .expect("Failed to build matcher");
    
    // Should match ALL occurrences including substrings
    assert!(matcher.is_match(b"test").expect("Match check failed"), "Should match 'test'");
    assert!(matcher.is_match(b"testing").expect("Match check failed"), "Should match 'testing'");
    assert!(matcher.is_match(b"attest").expect("Match check failed"), "Should match 'attest'");
    assert!(matcher.is_match(b"fastest").expect("Match check failed"), "Should match 'fastest'");
    assert!(matcher.is_match(b"libtest").expect("Match check failed"), "Should match 'libtest'");
}

#[test]
fn test_word_boundary_with_special_chars() {
    // Test literal string with special regex chars and word boundary
    // Pattern "test.log" with literal_search=true, word_boundary=true
    // Should be escaped as \btest\.log\b (dot escaped)
    
    let matcher = build_rust_matcher("test.log", CaseMode::Sensitive, true, true)
        .expect("Failed to build matcher");
    
    // Should match: "test.log" exactly
    assert!(matcher.is_match(b"test.log").expect("Match check failed"), "Should match 'test.log'");
    assert!(matcher.is_match(b"test.log ").expect("Match check failed"), "Should match 'test.log '");
    
    // Should NOT match: "testXlog" (dot not escaped would match this)
    assert!(!matcher.is_match(b"testXlog").expect("Match check failed"), "Should not match 'testXlog'");
    
    // Should NOT match when pattern is part of longer word
    assert!(!matcher.is_match(b"mytest.log").expect("Match check failed"), "Should not match 'mytest.log'");
    assert!(!matcher.is_match(b"test.logger").expect("Match check failed"), "Should not match 'test.logger'");
}

#[test]
fn test_word_boundary_case_insensitive() {
    // Test word boundary with case-insensitive mode
    let matcher = build_rust_matcher("Test", CaseMode::Insensitive, true, true)
        .expect("Failed to build matcher");
    
    // Should match both cases at word boundaries
    assert!(matcher.is_match(b"test").expect("Match check failed"), "Should match lowercase 'test'");
    assert!(matcher.is_match(b"TEST").expect("Match check failed"), "Should match uppercase 'TEST'");
    assert!(matcher.is_match(b"Test").expect("Match check failed"), "Should match mixed 'Test'");
    
    // Should NOT match when not at word boundary
    assert!(!matcher.is_match(b"testing").expect("Match check failed"), "Should not match 'testing'");
    assert!(!matcher.is_match(b"TESTING").expect("Match check failed"), "Should not match 'TESTING'");
}

#[test]
fn test_word_boundary_with_numbers() {
    // Test word boundary behavior with numbers
    // Note: Numbers are word characters (\w), so no boundary between letters and numbers
    let matcher = build_rust_matcher("test", CaseMode::Sensitive, false, true)
        .expect("Failed to build matcher");
    
    // Numbers are word characters - NO boundary between letters and numbers
    assert!(!matcher.is_match(b"test123").expect("Match check failed"), "Should NOT match 'test123' (no boundary)");
    assert!(!matcher.is_match(b"123test").expect("Match check failed"), "Should NOT match '123test' (no boundary)");
    
    // But boundaries exist with non-word characters
    assert!(matcher.is_match(b"test-123").expect("Match check failed"), "Should match 'test-123' (hyphen is boundary)");
    assert!(matcher.is_match(b"test.123").expect("Match check failed"), "Should match 'test.123' (dot is boundary)");
    assert!(matcher.is_match(b"test 123").expect("Match check failed"), "Should match 'test 123' (space is boundary)");
}

// Note: PCRE2 support is available via grep-pcre2 but not currently exposed in our API
// If needed in the future, implement build_pcre2_matcher in the rg module
