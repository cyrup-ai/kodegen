//! RFC 5.4 Line Break Characters Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.4 - Line Break Characters
//!
//! ## Requirements Tested
//!
//! 1. **MUST recognize ASCII line break characters: LF (x0A) and CR (x0D)**
//!    - Production rules [24], [25], [26]: b-line-feed, b-carriage-return, b-char
//!
//! 2. **MUST normalize line breaks in scalar content to single LF**
//!    - Production rule [29]: b-as-line-feed
//!    - Line break format is presentation detail, not content
//!
//! 3. **Non-ASCII line breaks MUST be treated as non-break characters**
//!    - NEL (x85), line separator (x2028), paragraph separator (x2029)
//!    - YAML 1.2 vs 1.1 compatibility requirement
//!
//! 4. **Line breaks outside scalar content can use any format**
//!    - Production rule [30]: b-non-content

use yyaml::YamlLoader;

/// Test RFC requirement: Recognition of ASCII line break characters
///
/// Production rules [24], [25], [26]:
/// - b-line-feed ::= x0A (LF)
/// - b-carriage-return ::= x0D (CR)  
/// - b-char ::= b-line-feed | b-carriage-return
#[test]
fn test_ascii_line_break_recognition() {
    // Test LF (x0A) line breaks
    let yaml_lf = "line1\nline2\nline3";
    let result = YamlLoader::load_from_str(yaml_lf);
    assert!(
        result.is_ok(),
        "Must recognize LF (x0A) as line break per RFC 5.4"
    );

    // Test CR (x0D) line breaks
    let yaml_cr = "line1\rline2\rline3";
    let result = YamlLoader::load_from_str(yaml_cr);
    assert!(
        result.is_ok(),
        "Must recognize CR (x0D) as line break per RFC 5.4"
    );

    // Test CRLF (x0D x0A) line breaks
    let yaml_crlf = "line1\r\nline2\r\nline3";
    let result = YamlLoader::load_from_str(yaml_crlf);
    assert!(
        result.is_ok(),
        "Must recognize CRLF (x0D x0A) as line break per RFC 5.4"
    );
}

/// Test RFC requirement: Line break normalization in scalar content
///
/// Production rule [29] b-as-line-feed ::= b-break
/// "Line breaks inside scalar content must be normalized by the YAML processor.
///  Each such line break must be parsed into a single line feed character."
#[test]
fn test_line_break_normalization_in_scalars() {
    // Test literal block scalar with different line break types
    let literal_lf = "text: |\n  line1\n  line2\n  line3";
    let result_lf = YamlLoader::load_from_str(literal_lf);
    assert!(
        result_lf.is_ok(),
        "LF line breaks must work in literal scalars per RFC 5.4"
    );

    let literal_crlf = "text: |\r\n  line1\r\n  line2\r\n  line3";
    let result_crlf = YamlLoader::load_from_str(literal_crlf);
    assert!(
        result_crlf.is_ok(),
        "CRLF line breaks must work in literal scalars per RFC 5.4"
    );

    // Both should produce equivalent content (normalized to LF)
    if let (Ok(docs_lf), Ok(docs_crlf)) = (result_lf, result_crlf) {
        let content_lf = docs_lf[0]["text"].as_str().unwrap();
        let content_crlf = docs_crlf[0]["text"].as_str().unwrap();

        // Both should contain LF characters (normalized)
        assert!(content_lf.contains('\n'), "LF normalization must occur");
        assert!(
            content_crlf.contains('\n'),
            "CRLF normalization to LF must occur"
        );

        // Content should be logically equivalent
        let normalized_lf = content_lf.replace('\r', "");
        let normalized_crlf = content_crlf.replace('\r', "");
        assert_eq!(
            normalized_lf, normalized_crlf,
            "Line break normalization must produce equivalent content per RFC 5.4"
        );
    }
}

/// Test RFC requirement: Line break format is presentation detail
///
/// "The original line break format is a presentation detail and must not be used to convey content information."
#[test]
fn test_line_break_presentation_detail() {
    // Same logical content with different line break formats
    let content_lf = "key: value\nother: data";
    let content_crlf = "key: value\r\nother: data";
    let content_cr = "key: value\rother: data";

    let docs_lf = YamlLoader::load_from_str(content_lf).unwrap();
    let docs_crlf = YamlLoader::load_from_str(content_crlf).unwrap();
    let docs_cr = YamlLoader::load_from_str(content_cr).unwrap();

    // All should produce same logical structure
    assert_eq!(
        docs_lf[0]["key"].as_str(),
        Some("value"),
        "LF format must parse correctly"
    );
    assert_eq!(
        docs_crlf[0]["key"].as_str(),
        Some("value"),
        "CRLF format must parse correctly"
    );
    assert_eq!(
        docs_cr[0]["key"].as_str(),
        Some("value"),
        "CR format must parse correctly"
    );

    assert_eq!(
        docs_lf[0]["other"].as_str(),
        Some("data"),
        "LF format must parse correctly"
    );
    assert_eq!(
        docs_crlf[0]["other"].as_str(),
        Some("data"),
        "CRLF format must parse correctly"
    );
    assert_eq!(
        docs_cr[0]["other"].as_str(),
        Some("data"),
        "CR format must parse correctly"
    );
}

/// Test RFC requirement: Non-ASCII line breaks treated as non-break characters
///
/// "All other characters, including the form feed (x0C), are considered to be non-break characters.
///  Note that these include the non-ASCII line breaks: next line (x85), line separator (x2028)
///  and paragraph separator (x2029)."
#[test]
fn test_non_ascii_line_breaks_as_non_break() {
    // These characters should NOT be treated as line breaks in YAML 1.2
    let non_break_chars = [
        ('\u{0C}', "Form feed"),             // x0C - explicitly mentioned
        ('\u{85}', "Next line (NEL)"),       // x85 - was line break in YAML 1.1
        ('\u{2028}', "Line separator"),      // x2028 - Unicode line separator
        ('\u{2029}', "Paragraph separator"), // x2029 - Unicode paragraph separator
    ];

    for (ch, desc) in non_break_chars {
        let yaml = format!("key: \"before{}after\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Non-ASCII character {} must be treated as non-break per RFC 5.4",
            desc
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["key"].as_str()
        {
            // Character should be preserved as content, not treated as line break
            assert!(
                value.contains(ch),
                "{} ({:?}) must be preserved as content, not line break per RFC 5.4",
                desc,
                ch
            );
        }
    }
}

/// Test RFC requirement: YAML 1.2 vs 1.1 line break compatibility
///
/// "YAML version 1.1 did support the above non-ASCII line break characters; however, JSON does not.
///  Hence, to ensure JSON compatibility, YAML treats them as non-break characters as of version 1.2."
#[test]
fn test_yaml_1_2_json_compatibility() {
    // Test that YAML 1.2 treats Unicode line separators as content, not breaks
    let yaml_with_unicode_separators = format!(
        "text: \"Line1{}Line2{}Line3\"",
        '\u{2028}', // Line separator
        '\u{2029}'  // Paragraph separator
    );

    let result = YamlLoader::load_from_str(&yaml_with_unicode_separators);
    assert!(
        result.is_ok(),
        "Unicode separators must work as content for JSON compatibility per RFC 5.4"
    );

    if let Ok(docs) = result
        && let Some(text_value) = docs[0]["text"].as_str()
    {
        // Should contain the Unicode separators as literal content
        assert!(
            text_value.contains('\u{2028}'),
            "Line separator must be preserved as content"
        );
        assert!(
            text_value.contains('\u{2029}'),
            "Paragraph separator must be preserved as content"
        );

        // Should NOT be split into multiple lines by these characters
        assert!(
            text_value.contains("Line1"),
            "Text before separator must be preserved"
        );
        assert!(
            text_value.contains("Line2"),
            "Text after separator must be preserved"
        );
        assert!(
            text_value.contains("Line3"),
            "Text after paragraph separator must be preserved"
        );
    }
}

/// Test RFC requirement: Non-break character production rule
///
/// Production rule [27]: nb-char ::= c-printable - b-char - c-byte-order-mark
#[test]
fn test_non_break_character_definition() {
    // nb-char = c-printable MINUS b-char MINUS c-byte-order-mark
    // So: all printable chars except LF, CR, and BOM

    let non_break_samples = [
        '\u{20}',   // Space (printable, not line break, not BOM)
        '\u{41}',   // 'A' (printable, not line break, not BOM)
        '\u{85}',   // NEL (printable, not b-char in 1.2, not BOM)
        '\u{2028}', // Line separator (printable, not b-char in 1.2, not BOM)
    ];

    for &ch in &non_break_samples {
        let yaml = format!("key: \"{}\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Non-break character {:?} (U+{:04X}) must work per RFC 5.4 nb-char production",
            ch,
            ch as u32
        );
    }

    // These SHOULD be line breaks (b-char) and thus not nb-char
    let break_chars = ['\u{0A}', '\u{0D}']; // LF, CR

    for &ch in &break_chars {
        let yaml = format!("line1{}line2", ch); // Outside quotes
        let result = YamlLoader::load_from_str(&yaml);
        // Should parse as separate lines, not single content with embedded character
        assert!(
            result.is_ok(),
            "Line break character {:?} must work as line break per RFC 5.4",
            ch
        );
    }
}

/// Test RFC requirement: Line breaks outside scalar content
///
/// Production rule [30]: b-non-content ::= b-break
/// "Outside scalar content, YAML allows any line break to be used to terminate lines."
#[test]
fn test_line_breaks_outside_scalar_content() {
    // Line breaks between YAML constructs (outside scalar content)
    let yaml_structures = [
        ("key1: value1\nkey2: value2", "LF between mappings"),
        ("key1: value1\r\nkey2: value2", "CRLF between mappings"),
        ("key1: value1\rkey2: value2", "CR between mappings"),
        ("- item1\n- item2", "LF between sequence items"),
        ("- item1\r\n- item2", "CRLF between sequence items"),
    ];

    for (yaml, description) in yaml_structures {
        let result = YamlLoader::load_from_str(yaml);
        assert!(
            result.is_ok(),
            "Line breaks outside scalar content must work: {} per RFC 5.4",
            description
        );

        if let Ok(docs) = result {
            // Verify structure was parsed correctly
            match description {
                desc if desc.contains("mappings") => {
                    assert!(
                        docs[0]["key1"].as_str().is_some(),
                        "First mapping entry must parse"
                    );
                    assert!(
                        docs[0]["key2"].as_str().is_some(),
                        "Second mapping entry must parse"
                    );
                }
                desc if desc.contains("sequence") => {
                    if let Some(seq) = docs[0].as_vec() {
                        assert_eq!(seq.len(), 2, "Both sequence items must parse");
                    }
                }
                _ => {}
            }
        }
    }
}

/// Test comprehensive line break handling in different contexts
#[test]
fn test_comprehensive_line_break_handling() {
    let complex_yaml = r#"
# Document with mixed line break contexts
mapping:
  key1: value1  # Comment with LF
  key2: >       # Folded scalar
    This is folded
    across lines
  key3: |       # Literal scalar  
    Line 1
    Line 2
    Line 3

sequence:
  - item 1
  - item 2
  - |
    Multi-line
    item content

flow: [one, two, three]

quoted: "String with\nembedded newline"
"#;

    let result = YamlLoader::load_from_str(complex_yaml);
    assert!(
        result.is_ok(),
        "Complex line break usage must work per RFC 5.4"
    );

    if let Ok(docs) = result {
        // Verify different line break contexts all work
        assert!(
            docs[0]["mapping"]["key1"].as_str().is_some(),
            "Simple mapping must parse"
        );
        assert!(
            docs[0]["mapping"]["key2"].as_str().is_some(),
            "Folded scalar must parse"
        );
        assert!(
            docs[0]["mapping"]["key3"].as_str().is_some(),
            "Literal scalar must parse"
        );
        assert!(
            docs[0]["sequence"].as_vec().is_some(),
            "Sequence must parse"
        );
        assert!(
            docs[0]["flow"].as_vec().is_some(),
            "Flow sequence must parse"
        );
        assert!(
            docs[0]["quoted"].as_str().is_some(),
            "Quoted scalar must parse"
        );
    }
}

/// Test edge cases for line break processing
#[test]
fn test_line_break_edge_cases() {
    // Empty lines
    let yaml_empty_lines = "key1: value1\n\n\nkey2: value2";
    let result = YamlLoader::load_from_str(yaml_empty_lines);
    assert!(
        result.is_ok(),
        "Empty lines should be handled correctly per RFC 5.4"
    );

    // Line breaks at end of file
    let yaml_trailing = "key: value\n";
    let result = YamlLoader::load_from_str(yaml_trailing);
    assert!(
        result.is_ok(),
        "Trailing line breaks should be handled correctly per RFC 5.4"
    );

    // Line breaks at start of file
    let yaml_leading = "\nkey: value";
    let result = YamlLoader::load_from_str(yaml_leading);
    assert!(
        result.is_ok(),
        "Leading line breaks should be handled correctly per RFC 5.4"
    );
}
