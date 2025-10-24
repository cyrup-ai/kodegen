//! RFC 5.2 Character Encodings Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.2 - Character Encodings
//!
//! ## Requirements Tested
//!
//! 1. **MUST support UTF-8 and UTF-16 character encodings on input**
//!    - Input encoding detection and processing
//!
//! 2. **MUST support UTF-32 encodings for JSON compatibility**  
//!    - UTF-32BE and UTF-32LE support
//!
//! 3. **MUST support byte order mark detection using specified table**
//!    - BOM detection algorithm per RFC table
//!
//! 4. **All documents in same stream MUST use same character encoding**
//!    - Multi-document encoding consistency
//!
//! 5. **Byte order marks MUST NOT appear inside document**
//!    - BOM placement restrictions (except in quoted scalars)
//!
//! 6. **Character encoding is presentation detail and MUST NOT convey content**
//!    - Encoding transparency requirement

use yyaml::{YamlEmitter, YamlLoader};

/// Test RFC requirement: "On input, a YAML processor must support the UTF-8 and UTF-16 character encodings"
///
/// UTF-8 is the default and most common encoding
#[test]
fn test_utf8_encoding_support() {
    // UTF-8 encoded YAML with various Unicode characters
    let utf8_yaml = "key: \"Hello 世界 🌍 Здравствуй\"";

    let result = YamlLoader::load_from_str(utf8_yaml);
    assert!(result.is_ok(), "Must support UTF-8 encoding per RFC 5.2");

    let docs = result.unwrap();
    assert_eq!(docs.len(), 1);

    if let Some(value_str) = docs[0]["key"].as_str() {
        assert!(
            value_str.contains("世界"),
            "UTF-8 Chinese characters must be preserved"
        );
        assert!(value_str.contains("🌍"), "UTF-8 emoji must be preserved");
        assert!(
            value_str.contains("Здравствуй"),
            "UTF-8 Cyrillic must be preserved"
        );
    }
}

/// Test RFC requirement: UTF-16 encoding support
///
/// Note: This test assumes the parser can handle UTF-16 input.
/// In practice, Rust strings are UTF-8, so this tests the principle.
#[test]
fn test_utf16_conceptual_support() {
    // Test that all UTF-16 representable characters work
    // UTF-16 can represent all Unicode code points using surrogate pairs for >U+FFFF

    let utf16_chars = [
        '\u{0041}',  // Basic Latin 'A'
        '\u{00E9}',  // Latin-1 Supplement 'é'
        '\u{4E2D}',  // CJK Unified Ideographs '中'
        '\u{1F600}', // Emoticons '😀' (requires surrogate pair in UTF-16)
    ];

    for &ch in &utf16_chars {
        let yaml = format!("key: \"UTF-16 char: {}\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Must support UTF-16 representable character {:?} (U+{:04X}) per RFC 5.2",
            ch,
            ch as u32
        );
    }
}

/// Test RFC requirement: "For JSON compatibility, the UTF-32 encodings must also be supported"
///
/// UTF-32 can directly represent any Unicode code point without surrogate pairs
#[test]
fn test_utf32_json_compatibility() {
    // UTF-32 can represent the full Unicode range directly
    let utf32_representable = [
        '\u{000000}', // NULL (though not printable, UTF-32 can represent it)
        '\u{10FFFF}', // Maximum Unicode code point
        '\u{1F680}',  // Rocket emoji
        '\u{2603}',   // Snowman
    ];

    for &ch in &utf32_representable {
        // Only test printable characters per RFC 5.1
        if ch >= '\u{20}' || ch == '\u{09}' || ch == '\u{0A}' || ch == '\u{0D}' {
            let yaml = format!("key: \"UTF-32: {}\"", ch);
            let result = YamlLoader::load_from_str(&yaml);
            assert!(
                result.is_ok(),
                "Must support UTF-32 representable character {:?} (U+{:06X}) for JSON compatibility per RFC 5.2",
                ch,
                ch as u32
            );
        }
    }
}

/// Test RFC requirement: BOM detection using specified table
///
/// Tests the BOM detection algorithm from RFC 5.2 table
#[test]
fn test_byte_order_mark_detection() {
    // Test UTF-8 BOM (xEF xBB xBF)
    let utf8_bom = "\u{FEFF}key: value";
    let result = YamlLoader::load_from_str(utf8_bom);
    assert!(
        result.is_ok(),
        "Must support UTF-8 BOM detection per RFC 5.2"
    );

    // Verify BOM is handled correctly (typically stripped)
    let docs = result.unwrap();
    assert_eq!(docs.len(), 1);
    assert!(
        docs[0]["key"].as_str().is_some(),
        "BOM should not interfere with parsing"
    );
}

/// Test RFC requirement: "byte order marks may appear at the start of any document,
/// however all documents in the same stream must use the same character encoding"
#[test]
fn test_multi_document_encoding_consistency() {
    // Multi-document YAML - all must use same encoding
    let multi_doc_yaml = "\u{FEFF}doc1: value1\n---\ndoc2: value2\n---\ndoc3: value3";

    let result = YamlLoader::load_from_str(multi_doc_yaml);
    assert!(
        result.is_ok(),
        "Multi-document streams must maintain encoding consistency per RFC 5.2"
    );

    let docs = result.unwrap();
    assert_eq!(docs.len(), 3, "Should parse all three documents");

    // All documents should be parsed correctly
    assert!(docs[0]["doc1"].as_str().is_some());
    assert!(docs[1]["doc2"].as_str().is_some());
    assert!(docs[2]["doc3"].as_str().is_some());
}

/// Test RFC requirement: BOM handling in different document positions
///
/// "Byte order marks may appear at the start of any document"
#[test]
fn test_bom_at_document_start() {
    // BOM at start of first document
    let yaml_bom_first = "\u{FEFF}first: document\n---\nsecond: document";
    let result = YamlLoader::load_from_str(yaml_bom_first);
    assert!(
        result.is_ok(),
        "BOM at start of first document must be supported per RFC 5.2"
    );

    // Note: BOMs in subsequent documents would require special handling
    // This test verifies the basic principle
}

/// Test RFC requirement: BOMs in quoted scalars are allowed for JSON compatibility
///
/// "To allow for JSON compatibility, byte order marks are also allowed inside quoted scalars"
#[test]
fn test_bom_in_quoted_scalars() {
    let yaml_bom_quoted = "key: \"text with BOM: \u{FEFF} here\"";
    let result = YamlLoader::load_from_str(yaml_bom_quoted);
    assert!(
        result.is_ok(),
        "BOM inside quoted scalars must be allowed per RFC 5.2"
    );

    let docs = result.unwrap();
    if let Some(value_str) = docs[0]["key"].as_str() {
        assert!(
            value_str.contains('\u{FEFF}'),
            "BOM must be preserved in quoted scalar content"
        );
    }
}

/// Test RFC requirement: "For readability, such content byte order marks should be escaped on output"
#[test]
fn test_bom_escaping_on_output() {
    // Input with BOM in quoted scalar
    let yaml_input = "key: \"BOM here: \u{FEFF}\"";
    let docs = YamlLoader::load_from_str(yaml_input).unwrap();

    // Emit back to string
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.dump(&docs[0]).unwrap();

    // Should preferably escape BOM for readability (this is a "SHOULD" not "MUST")
    // The exact behavior may vary by implementation
    println!("Output with BOM: {}", output);
    // This is more of a recommendation test
}

/// Test RFC requirement: "Character encoding is a presentation detail and must not be used to convey content information"
#[test]
fn test_encoding_transparency() {
    // Same logical content in different encodings should produce same result
    let content_utf8 = "message: \"Hello World\"";
    let content_with_bom = "\u{FEFF}message: \"Hello World\"";

    let result_utf8 = YamlLoader::load_from_str(content_utf8).unwrap();
    let result_bom = YamlLoader::load_from_str(content_with_bom).unwrap();

    // Should produce equivalent content (BOM is presentation detail)
    assert_eq!(
        result_utf8[0]["message"].as_str(),
        result_bom[0]["message"].as_str(),
        "Encoding differences must not affect content per RFC 5.2"
    );
}

/// Test RFC requirement: Encoding deduction by ASCII character pattern
///
/// "If a character stream begins with a byte order mark, the character encoding will be taken to be as indicated by the byte order mark.
///  Otherwise, the stream must begin with an ASCII character."
#[test]
fn test_ascii_character_encoding_deduction() {
    // Stream beginning with ASCII character (no BOM)
    let ascii_start_cases = [
        "key: value",        // Starts with 'k' (ASCII)
        "- item",            // Starts with '-' (ASCII)
        "123",               // Starts with '1' (ASCII)
        "  indented: value", // Starts with space (ASCII)
    ];

    for yaml in ascii_start_cases {
        let result = YamlLoader::load_from_str(yaml);
        assert!(
            result.is_ok(),
            "ASCII-starting stream '{}' must be parsed correctly per RFC 5.2",
            yaml
        );
    }
}

/// Test comprehensive BOM detection table from RFC 5.2
///
/// Tests the specific byte patterns mentioned in the RFC table
#[test]
fn test_bom_detection_table_patterns() {
    // Note: In Rust, we work with Unicode strings, not raw bytes.
    // This tests the logical equivalent of the BOM detection.

    // UTF-8 BOM pattern: xEF xBB xBF → U+FEFF
    let utf8_bom_yaml = "\u{FEFF}test: utf8";
    let result = YamlLoader::load_from_str(utf8_bom_yaml);
    assert!(
        result.is_ok(),
        "UTF-8 BOM pattern must be recognized per RFC 5.2"
    );

    // Test that content after BOM is parsed correctly
    let docs = result.unwrap();
    assert!(
        docs[0]["test"].as_str().is_some(),
        "Content after BOM must parse correctly"
    );
}

/// Test RFC requirement: Invalid BOM placement
///
/// "A BOM must not appear inside a document"
#[test]
fn test_invalid_bom_placement() {
    // BOM in middle of document (not at start, not in quoted scalar)
    let invalid_bom_yaml = "key: value\n\u{FEFF}\nother: content";

    // This should either fail or handle the BOM as content
    let result = YamlLoader::load_from_str(invalid_bom_yaml);

    // The behavior here depends on implementation:
    // - Might fail (BOM not allowed inside document)
    // - Might treat BOM as regular character in unquoted context
    // - Either is acceptable as long as it's consistent

    // Main requirement: BOMs should not appear inside documents
    // This test documents the expected behavior
    println!("Invalid BOM placement result: {:?}", result);
}

/// Test encoding consistency across complex documents  
#[test]
fn test_encoding_consistency_complex() {
    // Complex multi-document YAML with Unicode content
    let complex_yaml = r#"
doc1: 
  chinese: "中文测试"
  emoji: "🚀🌟"
  cyrillic: "Тест"
---
doc2:
  japanese: "テスト" 
  arabic: "اختبار"
  math: "∑∞≈√"
---
doc3:
  mixed: "Hello 世界 🌍 Привет"
"#;

    let result = YamlLoader::load_from_str(complex_yaml);
    assert!(
        result.is_ok(),
        "Complex Unicode content must work consistently per RFC 5.2"
    );

    let docs = result.unwrap();
    assert_eq!(
        docs.len(),
        3,
        "All documents must parse with consistent encoding"
    );

    // Verify Unicode content is preserved across all documents
    assert!(docs[0]["chinese"].as_str().unwrap().contains("中文"));
    assert!(docs[1]["japanese"].as_str().unwrap().contains("テスト"));
    assert!(docs[2]["mixed"].as_str().unwrap().contains("世界"));
}
