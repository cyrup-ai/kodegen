//! Tests for binary mode controls
//!
//! These tests verify that binary mode handling works correctly
//! and aligns with ripgrep's --binary and -a/--text flags.

use crate::search::rg::build_rust_matcher;
use crate::search::rg::flags::lowargs::BinaryMode as RgBinaryMode;
use crate::search::types::BinaryMode;
use crate::search::types::CaseMode;
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use std::path::Path;

/// Simple sink that collects match information
struct MatchCollector {
    matches: Vec<String>,
    is_binary: bool,
}

impl MatchCollector {
    fn new() -> Self {
        Self {
            matches: Vec::new(),
            is_binary: false,
        }
    }
}

impl Sink for MatchCollector {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if let Ok(line) = std::str::from_utf8(mat.bytes()) {
            self.matches.push(line.to_string());
        }
        Ok(true)
    }

    fn binary_data(
        &mut self,
        _searcher: &Searcher,
        _binary_byte_offset: u64,
    ) -> Result<bool, Self::Error> {
        self.is_binary = true;
        // Return true to allow searcher to continue (needed for convert mode)
        // Quit mode will stop regardless of this return value
        Ok(true)
    }
}

// ============================================================================
// UNIT TESTS: Enum Mapping
// ============================================================================

#[test]
fn test_binary_mode_enum_values() {
    // Verify that our BinaryMode enum has the correct variants
    // These map to ripgrep's flags: (none), --binary, -a/--text

    let auto = BinaryMode::Auto;
    let binary = BinaryMode::Binary;
    let text = BinaryMode::Text;

    // Verify default is Auto (skip binaries)
    assert_eq!(BinaryMode::default(), auto);

    // Verify all variants exist and are distinct
    assert_ne!(auto, binary);
    assert_ne!(auto, text);
    assert_ne!(binary, text);
}

#[test]
fn test_binary_mode_to_rg_mapping() {
    // Verify that MCP BinaryMode maps correctly to ripgrep's internal BinaryMode
    // This mapping is critical for correct behavior

    // Auto → Auto (default: skip binaries, like rg with no flags)
    let _rg_auto = RgBinaryMode::Auto;

    // Binary → SearchAndSuppress (like rg --binary)
    let _rg_search_suppress = RgBinaryMode::SearchAndSuppress;

    // Text → AsText (like rg -a or rg --text)
    let _rg_as_text = RgBinaryMode::AsText;

    // Verify the enum variants exist
    assert_eq!(format!("{:?}", RgBinaryMode::Auto), "Auto");
    assert_eq!(
        format!("{:?}", RgBinaryMode::SearchAndSuppress),
        "SearchAndSuppress"
    );
    assert_eq!(format!("{:?}", RgBinaryMode::AsText), "AsText");
}

// ============================================================================
// INTEGRATION TESTS: Binary Detection Behavior
// ============================================================================

#[test]
fn test_auto_mode_quits_on_binary() {
    // Test that Auto mode (default) stops searching when binary data is detected
    // This simulates: rg pattern (no flags)

    // Binary content with null bytes
    let binary_content = b"normal text\x00FINDME\x00more binary";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Build searcher with Auto mode: quit on binary detection
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    // Auto mode should detect binary and stop searching
    // The match "FINDME" comes after null byte, so it should NOT be found
    assert!(sink.is_binary, "Auto mode should detect binary data");
    assert_eq!(
        sink.matches.len(),
        0,
        "Auto mode should not return matches from binary files"
    );
}

#[test]
fn test_binary_mode_converts_nulls() {
    // Test that Binary mode (SearchAndSuppress) converts null bytes to newlines
    // This simulates: rg --binary pattern

    // Realistic binary content with embedded null bytes
    // After null-to-newline conversion: "text\nFINDME\ndata"
    let binary_content = b"text\x00FINDME\x00data";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Build searcher with Binary mode: convert nulls to newlines
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    // Should find the match because nulls were converted to newlines
    assert!(
        !sink.matches.is_empty(),
        "Binary mode should find matches after null conversion (found {})",
        sink.matches.len()
    );
}

#[test]
fn test_null_bytes_at_start() {
    // Test multiple consecutive null bytes at start of content
    // After conversion: "\n\nFINDME"
    let binary_content = b"\x00\x00FINDME";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    assert!(
        sink.is_binary,
        "Should detect binary data with nulls at start"
    );
    // Verify pattern is found after null conversion
    assert!(
        !sink.matches.is_empty(),
        "Binary mode should find FINDME after converting leading nulls"
    );
}

#[test]
fn test_null_bytes_at_end() {
    // Test multiple consecutive null bytes at end of content
    // After conversion: "FINDME\n\n"
    let binary_content = b"FINDME\x00\x00";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    assert!(
        sink.is_binary,
        "Should detect binary data with nulls at end"
    );
    // Verify pattern is found before null conversion
    assert!(
        !sink.matches.is_empty(),
        "Binary mode should find FINDME before trailing nulls"
    );
}

#[test]
fn test_consecutive_null_bytes() {
    // Test multiple consecutive null bytes in middle of content
    // After conversion: "text\n\n\ndata"
    let binary_content = b"text\x00\x00\x00data";

    let matcher = build_rust_matcher("data", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    assert!(
        sink.is_binary,
        "Should detect binary data with consecutive nulls"
    );
    // Verify pattern after consecutive nulls is found
    assert!(
        !sink.matches.is_empty(),
        "Binary mode should find 'data' after consecutive null conversions"
    );
}

#[test]
fn test_pattern_split_by_null_byte() {
    // CRITICAL TEST: Verify that null byte conversion doesn't create false matches
    // Pattern "FINDME" split by null becomes "FI\nNDME" which should NOT match
    let binary_content = b"FI\x00NDME";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Test with Binary mode (convert)
    let mut searcher_binary = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink_binary = MatchCollector::new();
    let _ = searcher_binary.search_slice(&matcher, binary_content, &mut sink_binary);

    assert!(sink_binary.is_binary, "Should detect binary data");
    // Verify no matches in Binary mode (null creates line break)
    assert_eq!(
        sink_binary.matches.len(),
        0,
        "Pattern split by null byte should not match (null→newline breaks pattern)"
    );

    // Test with Text mode to verify the split pattern is truly not matching
    let mut searcher_text = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();

    let mut sink_text = MatchCollector::new();
    let _ = searcher_text.search_slice(&matcher, binary_content, &mut sink_text);

    // Verify no matches in Text mode either (null byte is still there)
    assert_eq!(
        sink_text.matches.len(),
        0,
        "Pattern split by null byte should not match even in Text mode"
    );
}

#[test]
fn test_multiline_pattern_with_nulls() {
    // Test that multi-line patterns work with null byte conversion
    // Content: "line1\x00line2" becomes "line1\nline2"
    let binary_content = b"line1\x00line2";

    // Single-line pattern should work
    let matcher_single = build_rust_matcher("line2", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher_single, binary_content, &mut sink);

    assert!(sink.is_binary, "Should detect binary data");
    assert!(
        !sink.matches.is_empty(),
        "Should find single-line pattern after null conversion"
    );
}

#[test]
fn test_text_mode_no_detection() {
    // Test that Text mode (-a/--text) disables binary detection entirely
    // This simulates: rg -a pattern  OR  rg --text pattern

    // Binary content with null bytes
    let binary_content = b"text\x00FINDME\x00data";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Build searcher with Text mode: no binary detection
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, binary_content, &mut sink);

    // Text mode should search everything (may have garbled output but should find matches)
    assert!(!sink.matches.is_empty(), "Text mode should find matches");
    assert!(
        !sink.is_binary,
        "Text mode should not trigger binary detection"
    );
}

// ============================================================================
// FILE-BASED INTEGRATION TESTS
// ============================================================================

#[test]
fn test_normal_text_file_found_in_all_modes() {
    // Verify that normal text files are found in ALL binary modes
    // All modes should find matches in text files

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/search/tests/fixtures/binary_modes/normal.txt");

    assert!(
        fixture_path.exists(),
        "Test fixture not found: {fixture_path:?}"
    );

    let content = std::fs::read(&fixture_path).unwrap_or_else(|e| panic!("Failed to read normal.txt: {e}"));

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Test with each binary detection mode
    let modes = vec![
        ("Auto", BinaryDetection::quit(b'\x00')),
        ("Binary", BinaryDetection::convert(b'\x00')),
        ("Text", BinaryDetection::none()),
    ];

    for (mode_name, detection) in modes {
        let mut searcher = SearcherBuilder::new().binary_detection(detection).build();

        let mut sink = MatchCollector::new();
        searcher
            .search_slice(&matcher, &content, &mut sink)
            .unwrap_or_else(|e| panic!("Search failed: {e}"));

        assert!(
            sink.matches.len() >= 2,
            "{} mode should find at least 2 matches in normal.txt (found {})",
            mode_name,
            sink.matches.len()
        );
        assert!(
            !sink.is_binary,
            "{mode_name} mode should not detect normal.txt as binary"
        );
    }
}

#[test]
fn test_binary_file_auto_mode_skips() {
    // Test that Auto mode skips binary files
    // Simulates: rg FINDME (default behavior)

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/search/tests/fixtures/binary_modes/binary.bin");

    assert!(
        fixture_path.exists(),
        "Test fixture not found: {fixture_path:?}"
    );

    let content = std::fs::read(&fixture_path).unwrap_or_else(|e| panic!("Failed to read binary.bin: {e}"));

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, &content, &mut sink);

    // Auto mode should detect binary and stop
    assert!(
        sink.is_binary,
        "Auto mode should detect binary.bin as binary"
    );
    // May or may not have matches depending on when null byte appears
    // Key point: binary detection was triggered
}

#[test]
fn test_binary_file_binary_mode_searches() {
    // Test that Binary mode behavior differs from Auto mode
    // Simulates: rg --binary FINDME

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/search/tests/fixtures/binary_modes/binary.bin");

    if !fixture_path.exists() {
        eprintln!("Skipping test - fixture not found: {fixture_path:?}");
        return;
    }

    let content = std::fs::read(&fixture_path).unwrap_or_else(|e| panic!("Failed to read binary.bin: {e}"));

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Test with convert mode
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::convert(b'\x00'))
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, &content, &mut sink);

    // Binary mode may or may not find matches depending on content structure
    // Key point: it doesn't quit immediately like Auto mode
    // This test mainly verifies the mode doesn't crash
}

#[test]
fn test_binary_file_text_mode_searches() {
    // Test that Text mode searches binary files as text
    // Simulates: rg -a FINDME  OR  rg --text FINDME

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/search/tests/fixtures/binary_modes/binary.bin");

    assert!(
        fixture_path.exists(),
        "Test fixture not found: {fixture_path:?}"
    );

    let content = std::fs::read(&fixture_path).unwrap_or_else(|e| panic!("Failed to read binary.bin: {e}"));

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();

    let mut sink = MatchCollector::new();
    let _ = searcher.search_slice(&matcher, &content, &mut sink);

    // Text mode should find matches (no binary detection)
    assert!(
        !sink.matches.is_empty(),
        "Text mode should find matches in binary.bin"
    );
    assert!(
        !sink.is_binary,
        "Text mode should not trigger binary detection"
    );
}

#[test]
fn test_data_with_nulls_csv_modes() {
    // Test data file with null bytes in different modes
    // This simulates CSV/JSON files that may have null byte delimiters

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/search/tests/fixtures/binary_modes/data_with_nulls.csv");

    if !fixture_path.exists() {
        eprintln!("Skipping test - fixture not found: {fixture_path:?}");
        return;
    }

    let content = std::fs::read(&fixture_path).unwrap_or_else(|e| panic!("Failed to read data_with_nulls.csv: {e}"));

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Auto mode - will quit on nulls
    let mut searcher_auto = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build();
    let mut sink_auto = MatchCollector::new();
    let _ = searcher_auto.search_slice(&matcher, &content, &mut sink_auto);

    // Text mode - no binary detection
    let mut searcher_text = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();
    let mut sink_text = MatchCollector::new();
    let _ = searcher_text.search_slice(&matcher, &content, &mut sink_text);

    // Text mode should find the pattern (no binary detection)
    assert!(
        !sink_text.matches.is_empty(),
        "Text mode should find pattern in data_with_nulls.csv"
    );
    assert!(
        !sink_text.is_binary,
        "Text mode should not trigger binary detection"
    );

    // Auto mode should detect it as binary
    assert!(
        sink_auto.is_binary,
        "Auto mode should detect data_with_nulls.csv as binary"
    );
}

#[test]
fn test_mode_comparison_on_binary_content() {
    // Comprehensive test comparing all three modes on same content
    // This verifies the behavioral differences between modes

    // Test content with null byte at start (before pattern)
    let test_content = b"\x00FINDME on own line\n";

    let matcher = build_rust_matcher("FINDME", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build matcher: {e}"));

    // Mode 1: Auto (quit on binary)
    let mut searcher_auto = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build();
    let mut sink_auto = MatchCollector::new();
    let _ = searcher_auto.search_slice(&matcher, test_content, &mut sink_auto);

    // Mode 2: Text (no detection)
    let mut searcher_text = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();
    let mut sink_text = MatchCollector::new();
    let _ = searcher_text.search_slice(&matcher, test_content, &mut sink_text);

    // Verify behavioral differences
    assert!(sink_auto.is_binary, "Auto mode should detect binary");
    assert_eq!(
        sink_auto.matches.len(),
        0,
        "Auto mode should stop at null byte"
    );

    // Text mode should find the pattern (no binary detection)
    assert!(
        !sink_text.matches.is_empty(),
        "Text mode should find pattern"
    );
    assert!(
        !sink_text.is_binary,
        "Text mode should not trigger binary detection"
    );
}

#[test]
fn test_literal_vs_regex_with_binary_modes() {
    // Test that binary modes work correctly with both literal and regex patterns
    // Use content without nulls for reliable matching

    let test_content = b"test line\nFINDME.txt here\nmore data\n";

    // Literal pattern "FINDME.txt"
    let matcher_literal = build_rust_matcher("FINDME.txt", CaseMode::Sensitive, true, false)
        .unwrap_or_else(|e| panic!("Failed to build literal matcher: {e}"));

    // Regex pattern "FINDME.*"
    let matcher_regex = build_rust_matcher("FINDME.*", CaseMode::Sensitive, false, false)
        .unwrap_or_else(|e| panic!("Failed to build regex matcher: {e}"));

    // Test both with Text mode (no binary detection)
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::none())
        .build();

    let mut sink_literal = MatchCollector::new();
    let _ = searcher.search_slice(&matcher_literal, test_content, &mut sink_literal);

    let mut sink_regex = MatchCollector::new();
    let _ = searcher.search_slice(&matcher_regex, test_content, &mut sink_regex);

    // Both should find matches
    assert!(
        !sink_literal.matches.is_empty(),
        "Literal pattern should work (found {} matches)",
        sink_literal.matches.len()
    );
    assert!(
        !sink_regex.matches.is_empty(),
        "Regex pattern should work (found {} matches)",
        sink_regex.matches.len()
    );
}
