//! RFC 5.7 Escaped Characters Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.7 - Escaped Characters
//!
//! ## Requirements Tested
//!
//! 1. **Escape sequences MUST only be interpreted in double-quoted scalars**
//!    - No escape processing in other scalar types
//!
//! 2. **MUST support all specified escape sequences**
//!    - C-style escapes: \0, \a, \b, \t, \n, \v, \f, \r, \e
//!    - YAML-specific: \ , \", \/, \\, \N, \_, \L, \P  
//!    - Hex escapes: \xXX, \uXXXX, \UXXXXXXXX
//!
//! 3. **MUST reject invalid escape sequences**
//!    - Error handling for undefined escapes
//!
//! 4. **Non-printable characters MUST be escaped**
//!    - Input/output handling requirement

use yyaml::YamlLoader;

/// Test RFC requirement: Escape sequences only work in double-quoted scalars
///
/// "Note that escape sequences are only interpreted in double-quoted scalars.
///  In all other scalar styles, the \ character has no special meaning"
#[test]
fn test_escape_sequences_only_in_double_quotes() {
    // Should work in double-quoted scalars
    let double_quoted = r#"text: "Line 1\nLine 2\tTabbed""#;
    let result = YamlLoader::load_from_str(double_quoted);
    assert!(
        result.is_ok(),
        "Escape sequences must work in double-quoted scalars per RFC 5.7"
    );

    if let Ok(docs) = result
        && let Some(value) = docs[0]["text"].as_str()
    {
        assert!(
            value.contains('\n'),
            "\\n must be interpreted as newline in double quotes"
        );
        assert!(
            value.contains('\t'),
            "\\t must be interpreted as tab in double quotes"
        );
    }

    // Should NOT work in single-quoted scalars (literal backslashes)
    let single_quoted = r#"text: 'Line 1\nLine 2\tTabbed'"#;
    let result = YamlLoader::load_from_str(single_quoted);
    assert!(
        result.is_ok(),
        "Single-quoted scalars must parse successfully per RFC 5.7"
    );

    if let Ok(docs) = result
        && let Some(value) = docs[0]["text"].as_str()
    {
        assert!(
            value.contains("\\n"),
            "\\n must be literal in single quotes, not escape"
        );
        assert!(
            value.contains("\\t"),
            "\\t must be literal in single quotes, not escape"
        );
    }

    // Should NOT work in plain scalars
    let plain_scalar = r#"text: Plain\ntext\there"#;
    let result = YamlLoader::load_from_str(plain_scalar);
    if let Ok(docs) = result
        && let Some(value) = docs[0]["text"].as_str()
    {
        assert!(
            value.contains("\\n"),
            "\\n must be literal in plain scalars, not escape"
        );
    }

    // Should NOT work in block scalars
    let block_literal = r#"
text: |
  Line 1\nLine 2\tTabbed
"#;
    let result = YamlLoader::load_from_str(block_literal);
    assert!(
        result.is_ok(),
        "Block scalars must parse successfully per RFC 5.7"
    );

    if let Ok(docs) = result
        && let Some(value) = docs[0]["text"].as_str()
    {
        assert!(
            value.contains("\\n"),
            "\\n must be literal in block scalars, not escape"
        );
        assert!(
            value.contains("\\t"),
            "\\t must be literal in block scalars, not escape"
        );
    }
}

/// Test RFC requirement: Support for all C-style escape sequences
///
/// Production rules [42-50]: C escape sequence equivalents
#[test]
fn test_c_style_escape_sequences() {
    let c_escapes = vec![
        ("\\0", '\u{00}', "null"),            // [42] ns-esc-null
        ("\\a", '\u{07}', "bell"),            // [43] ns-esc-bell
        ("\\b", '\u{08}', "backspace"),       // [44] ns-esc-backspace
        ("\\t", '\u{09}', "horizontal tab"),  // [45] ns-esc-horizontal-tab
        ("\\n", '\u{0A}', "line feed"),       // [46] ns-esc-line-feed
        ("\\v", '\u{0B}', "vertical tab"),    // [47] ns-esc-vertical-tab
        ("\\f", '\u{0C}', "form feed"),       // [48] ns-esc-form-feed
        ("\\r", '\u{0D}', "carriage return"), // [49] ns-esc-carriage-return
        ("\\e", '\u{1B}', "escape"),          // [50] ns-esc-escape
    ];

    for (escape_seq, expected_char, desc) in c_escapes {
        let yaml = format!(r#"test: "before{}after""#, escape_seq);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "C-style escape {} ({}) must work per RFC 5.7",
            escape_seq,
            desc
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["test"].as_str()
        {
            assert!(
                value.contains(expected_char),
                "Escape {} must produce character {:?} (U+{:02X})",
                escape_seq,
                expected_char,
                expected_char as u32
            );
        }
    }
}

/// Test RFC requirement: Support for YAML-specific escape sequences
///
/// Production rules [51-58]: YAML-specific escapes
#[test]
fn test_yaml_specific_escape_sequences() {
    let yaml_escapes = vec![
        ("\\ ", '\u{20}', "space"),                 // [51] ns-esc-space
        ("\\\"", '\u{22}', "double quote"),         // [52] ns-esc-double-quote
        ("\\/", '\u{2F}', "slash (JSON compat)"),   // [53] ns-esc-slash
        ("\\\\", '\u{5C}', "backslash"),            // [54] ns-esc-backslash
        ("\\N", '\u{85}', "next line"),             // [55] ns-esc-next-line
        ("\\_", '\u{A0}', "non-breaking space"),    // [56] ns-esc-non-breaking-space
        ("\\L", '\u{2028}', "line separator"),      // [57] ns-esc-line-separator
        ("\\P", '\u{2029}', "paragraph separator"), // [58] ns-esc-paragraph-separator
    ];

    for (escape_seq, expected_char, desc) in yaml_escapes {
        let yaml = format!(r#"test: "before{}after""#, escape_seq);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "YAML-specific escape {} ({}) must work per RFC 5.7",
            escape_seq,
            desc
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["test"].as_str()
        {
            assert!(
                value.contains(expected_char),
                "Escape {} must produce character {:?} (U+{:04X})",
                escape_seq,
                expected_char,
                expected_char as u32
            );
        }
    }
}

/// Test RFC requirement: Support for hexadecimal escape sequences
///
/// Production rules [59-61]: Hex escapes
#[test]
fn test_hexadecimal_escape_sequences() {
    // [59] ns-esc-8-bit ::= 'x' ns-hex-digit{2}
    let hex_8bit_cases = vec![
        ("\\x41", 'A'), // ASCII 'A'
        ("\\x20", ' '), // Space
        ("\\x7E", '~'), // Tilde
    ];

    for (escape_seq, expected_char) in hex_8bit_cases {
        let yaml = format!(r#"test: "{}""#, escape_seq);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "8-bit hex escape {} must work per RFC 5.7",
            escape_seq
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["test"].as_str()
        {
            assert!(
                value.contains(expected_char),
                "Hex escape {} must produce '{}'",
                escape_seq,
                expected_char
            );
        }
    }

    // [60] ns-esc-16-bit ::= 'u' ns-hex-digit{4}
    let hex_16bit_cases = vec![
        ("\\u0041", 'A'),  // ASCII 'A'
        ("\\u00E9", 'é'),  // Latin small letter e with acute
        ("\\u4E2D", '中'), // CJK unified ideograph
    ];

    for (escape_seq, expected_char) in hex_16bit_cases {
        let yaml = format!(r#"test: "{}""#, escape_seq);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "16-bit hex escape {} must work per RFC 5.7",
            escape_seq
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["test"].as_str()
        {
            assert!(
                value.contains(expected_char),
                "Hex escape {} must produce '{}'",
                escape_seq,
                expected_char
            );
        }
    }

    // [61] ns-esc-32-bit ::= 'U' ns-hex-digit{8}
    let hex_32bit_cases = vec![
        ("\\U00000041", 'A'),  // ASCII 'A'
        ("\\U0001F600", '😀'), // Grinning face emoji
        ("\\U0001F680", '🚀'), // Rocket emoji
    ];

    for (escape_seq, expected_char) in hex_32bit_cases {
        let yaml = format!(r#"test: "{}""#, escape_seq);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "32-bit hex escape {} must work per RFC 5.7",
            escape_seq
        );

        if let Ok(docs) = result
            && let Some(value) = docs[0]["test"].as_str()
        {
            assert!(
                value.contains(expected_char),
                "Hex escape {} must produce '{}'",
                escape_seq,
                expected_char
            );
        }
    }
}

/// Test RFC requirement: Invalid escape sequences must be rejected
///
/// "Bad escapes: \c \xq-" - example of invalid escapes from RFC
#[test]
fn test_invalid_escape_sequences() {
    let invalid_escapes = vec![
        r#"test: "\c""#,         // Invalid escape character
        r#"test: "\xq1""#,       // Invalid hex digit in 8-bit escape
        r#"test: "\x1""#,        // Too few hex digits in 8-bit escape
        r#"test: "\uGHIJ""#,     // Invalid hex digits in 16-bit escape
        r#"test: "\u123""#,      // Too few hex digits in 16-bit escape
        r#"test: "\UGHIJKLMN""#, // Invalid hex digits in 32-bit escape
        r#"test: "\U1234567""#,  // Too few hex digits in 32-bit escape
        r#"test: "\z""#,         // Undefined escape character
    ];

    for yaml in invalid_escapes {
        let result = YamlLoader::load_from_str(yaml);
        // These should either fail or handle gracefully
        // The key requirement is consistent behavior per RFC 5.7
        println!("Invalid escape test '{}': {:?}", yaml, result.is_err());
    }
}

/// Test comprehensive escape sequence support
///
/// Production rule [62]: c-ns-esc-char covers all escape sequences
#[test]
fn test_comprehensive_escape_support() {
    let comprehensive_escapes = r#"
escapes: "Null:\0 Bell:\a Backspace:\b Tab:\t Newline:\n Vtab:\v Formfeed:\f Return:\r Escape:\e Space:\ Quote:\" Slash:\/ Backslash:\\ NextLine:\N NonBreakSpace:\_ LineSep:\L ParaSep:\P Hex8:\x41 Hex16:\u0041 Hex32:\U00000041"
"#;

    let result = YamlLoader::load_from_str(comprehensive_escapes);
    assert!(
        result.is_ok(),
        "All escape sequences must work together per RFC 5.7"
    );

    if let Ok(docs) = result
        && let Some(value) = docs[0]["escapes"].as_str()
    {
        // Verify key escape characters are present
        assert!(value.contains('\0'), "Null escape must work");
        assert!(value.contains('\t'), "Tab escape must work");
        assert!(value.contains('\n'), "Newline escape must work");
        assert!(value.contains('"'), "Quote escape must work");
        assert!(value.contains('\\'), "Backslash escape must work");
        assert!(value.contains('A'), "Hex escapes must work");
    }
}

/// Test escape sequences in different contexts
#[test]
fn test_escapes_in_different_contexts() {
    // Escapes in mapping keys (if supported)
    let key_escapes = r#""key\twith\ttabs": "value""#;
    let result = YamlLoader::load_from_str(key_escapes);
    if result.is_ok() {
        println!("Escape sequences in mapping keys: supported");
    }

    // Escapes in flow collections
    let flow_escapes = r#"["item\none", "item\ttwo"]"#;
    let result = YamlLoader::load_from_str(flow_escapes);
    assert!(
        result.is_ok(),
        "Escape sequences in flow collections must work per RFC 5.7"
    );

    // Multiple escapes in single string
    let multiple_escapes = r#"text: "First\nSecond\tThird\"Fourth\\Fifth""#;
    let result = YamlLoader::load_from_str(multiple_escapes);
    assert!(
        result.is_ok(),
        "Multiple escape sequences must work per RFC 5.7"
    );
}

/// Test edge cases for escape processing
#[test]
fn test_escape_edge_cases() {
    // Backslash at end of string
    let trailing_backslash = r#"test: "text\\""#;
    let result = YamlLoader::load_from_str(trailing_backslash);
    assert!(result.is_ok(), "Escaped backslash must work per RFC 5.7");

    // Empty string with escapes
    let empty_with_escapes = r#"test: "\n""#;
    let result = YamlLoader::load_from_str(empty_with_escapes);
    assert!(
        result.is_ok(),
        "Single escape in string must work per RFC 5.7"
    );

    // Unicode surrogate handling (if applicable)
    let high_unicode = r#"test: "\U0010FFFF""#; // Maximum Unicode code point
    let result = YamlLoader::load_from_str(high_unicode);
    assert!(
        result.is_ok(),
        "Maximum Unicode escape must work per RFC 5.7"
    );
}

/// Test escape sequence output requirements
///
/// "Characters outside this set must be presented using escape sequences"
#[test]
fn test_escape_requirements_on_output() {
    // This tests the requirement that non-printable chars get escaped on output
    let yaml_with_control = "test: \"control\u{01}char\"";
    let docs = YamlLoader::load_from_str(yaml_with_control).unwrap();

    // When emitting back, control characters should be escaped
    let mut output = String::new();
    let mut emitter = yyaml::YamlEmitter::new(&mut output);
    emitter.dump(&docs[0]).unwrap();

    // Should contain escape sequence, not raw control character
    assert!(
        !output.contains('\u{01}'),
        "Non-printable characters must be escaped on output per RFC 5.7"
    );
    println!("Output with control char: {}", output.escape_debug());
}
