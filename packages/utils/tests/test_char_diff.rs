//! Tests for character-level diff functionality

use kodegen_utils::char_diff::CharDiff;

#[test]
fn test_unicode_suffix_no_panic() {
    // Critical test: ensure no panic with multi-byte UTF-8 suffix
    let diff = CharDiff::new("test世界", "best世界");
    let result = diff.format();
    // Should not panic - that's the critical fix
    assert!(result.contains("世界"));
}

#[test]
fn test_unicode_prefix_no_panic() {
    // Ensure no panic with multi-byte UTF-8 prefix
    let diff = CharDiff::new("世界test", "世界best");
    let result = diff.format();
    // Should not panic
    assert!(result.contains("世界"));
}

#[test]
fn test_emoji_no_panic() {
    // Ensure no panic with emoji (4-byte UTF-8)
    let diff = CharDiff::new("test🎉", "best🎉");
    let result = diff.format();
    // Should not panic
    assert!(result.contains("🎉"));
}

#[test]
fn test_mixed_unicode_no_panic() {
    // Ensure no panic with different multi-byte chars
    let diff = CharDiff::new("a世b", "a界b");
    let result = diff.format();
    // Should not panic and show the difference
    assert!(result.contains("世"));
    assert!(result.contains("界"));
}

#[test]
fn test_ascii_regression() {
    // Ensure ASCII still works (regression test)
    let diff = CharDiff::new("function getUserData()", "function  getUserData()");
    let result = diff.format();
    // Should show whitespace difference
    assert!(result.contains("{-"));
    assert!(result.contains("+}"));
}

#[test]
fn test_whitespace_detection() {
    let diff = CharDiff::new("test ", "test  ");
    assert!(diff.is_whitespace_only());
}

#[test]
fn test_identical_strings() {
    let diff = CharDiff::new("test", "test");
    assert_eq!(diff.common_prefix, "test");
    assert_eq!(diff.expected_part, "");
    assert_eq!(diff.actual_part, "");
    assert_eq!(diff.common_suffix, "");
}

#[test]
fn test_completely_different() {
    let diff = CharDiff::new("abc", "xyz");
    assert_eq!(diff.common_prefix, "");
    assert_eq!(diff.common_suffix, "");
    assert_eq!(diff.expected_part, "abc");
    assert_eq!(diff.actual_part, "xyz");
}
