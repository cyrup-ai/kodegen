//! RFC 5.6 Miscellaneous Characters Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.6 - Miscellaneous Characters
//!
//! Note: This section primarily defines helper productions for character classification
//! and typically doesn't contain standalone MUST/MUST NOT requirements.
//! Tests here verify the character classification productions work correctly.

use yyaml::YamlLoader;

/// Test miscellaneous character productions and classifications
///
/// This section usually contains productions like:
/// - Character range definitions
/// - Helper classifications  
/// - Utility character sets
#[test]
fn test_miscellaneous_character_handling() {
    // Test various character types work correctly in different contexts
    let misc_yaml = r#"
ascii: "ASCII text"
unicode: "Unicode: ñáéíóú"  
symbols: "Symbols: @#$%^&*()"
numbers: "123456789"
mixed: "Mixed 123 Unicode ñ Symbols @#$"
"#;

    let result = YamlLoader::load_from_str(misc_yaml);
    assert!(
        result.is_ok(),
        "Miscellaneous characters must be handled correctly per RFC 5.6"
    );

    if let Ok(docs) = result {
        assert!(
            docs[0]["ascii"].as_str().is_some(),
            "ASCII characters must work"
        );
        assert!(
            docs[0]["unicode"].as_str().is_some(),
            "Unicode characters must work"
        );
        assert!(
            docs[0]["symbols"].as_str().is_some(),
            "Symbol characters must work"
        );
        assert!(
            docs[0]["numbers"].as_str().is_some(),
            "Number characters must work"
        );
        assert!(
            docs[0]["mixed"].as_str().is_some(),
            "Mixed characters must work"
        );
    }
}

/// Test character boundary conditions
#[test]
fn test_character_boundaries() {
    // Test characters at boundaries of defined ranges
    let boundary_chars = [
        '\u{0020}', // Start of printable ASCII
        '\u{007E}', // End of printable ASCII
        '\u{00A0}', // Start of extended range
        '\u{FFFD}', // Unicode replacement character
    ];

    for &ch in &boundary_chars {
        let yaml = format!("char: \"{}\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Boundary character {:?} (U+{:04X}) must work per RFC 5.6",
            ch,
            ch as u32
        );
    }
}

/// Test that character classifications are consistent
#[test]
fn test_character_classification_consistency() {
    // Verify different character types behave consistently
    let test_cases = vec![
        ("letter: a", "Single letter"),
        ("digit: 1", "Single digit"),
        ("space: \" \"", "Single space"),
        ("tab: \"\t\"", "Single tab"),
        ("unicode: \"ñ\"", "Unicode letter"),
    ];

    for (yaml, description) in test_cases {
        let result = YamlLoader::load_from_str(yaml);
        assert!(
            result.is_ok(),
            "Character classification test '{}' must work per RFC 5.6",
            description
        );
    }
}
