use kodegen_tools_citescrape::content_saver::markdown_converter::{
    clean_html_content, extract_main_content,
};
use std::time::Instant;

/// Test that regex patterns are resistant to `ReDoS` (Regular Expression Denial of Service) attacks
///
/// The Rust `regex` crate uses bounded execution and finite automata, which makes it
/// naturally resistant to catastrophic backtracking. However, we still test with
/// adversarial inputs to ensure reasonable performance.
#[test]
fn test_redos_resistance_script_tag() {
    let adversarial = "<script ".to_string() + &"a".repeat(10000);
    let start = Instant::now();
    let _ = clean_html_content(&adversarial);
    let elapsed = start.elapsed();

    println!("Script tag test: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 100,
        "ReDoS vulnerability detected: took {elapsed:?}"
    );
}

#[test]
fn test_redos_resistance_style_attribute() {
    let adversarial = "<div style=\"".to_string() + &"a".repeat(10000);
    let start = Instant::now();
    let _ = clean_html_content(&adversarial);
    let elapsed = start.elapsed();

    println!("Style attribute test: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 100,
        "ReDoS vulnerability detected: took {elapsed:?}"
    );
}

#[test]
fn test_redos_resistance_class_attribute() {
    let adversarial = "<div class=\"".to_string() + &"a".repeat(10000);
    let start = Instant::now();
    let _ = clean_html_content(&adversarial);
    let elapsed = start.elapsed();

    println!("Class attribute test: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 100,
        "ReDoS vulnerability detected: took {elapsed:?}"
    );
}

#[test]
fn test_redos_resistance_partial_match() {
    let adversarial = "<div style=\"display:none".to_string() + &"a".repeat(10000);
    let start = Instant::now();
    let _ = clean_html_content(&adversarial);
    let elapsed = start.elapsed();

    println!("Partial match test: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 100,
        "ReDoS vulnerability detected: took {elapsed:?}"
    );
}

#[test]
fn test_input_size_limit() {
    // Test that MAX_HTML_SIZE (10 MB) is enforced
    let too_large = "a".repeat(11 * 1024 * 1024); // 11 MB
    let result = clean_html_content(&too_large);

    assert!(result.is_err(), "Should reject HTML larger than 10 MB");
    assert!(
        result.unwrap_err().to_string().contains("too large"),
        "Error message should mention size limit"
    );
}

#[test]
fn test_extract_main_content_size_limit() {
    // Test that MAX_HTML_SIZE is also enforced in extract_main_content
    let too_large = "a".repeat(11 * 1024 * 1024); // 11 MB
    let result = extract_main_content(&too_large);

    assert!(result.is_err(), "Should reject HTML larger than 10 MB");
    assert!(
        result.unwrap_err().to_string().contains("too large"),
        "Error message should mention size limit"
    );
}
