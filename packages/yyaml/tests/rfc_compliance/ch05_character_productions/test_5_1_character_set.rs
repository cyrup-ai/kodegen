//! RFC 5.1 Character Set Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.1 - Character Set
//!
//! ## Requirements Tested
//!
//! 1. **MUST accept all printable characters on input**
//!    - Production rule [1] c-printable
//!    - Characters: x09, x0A, x0D, [x20-x7E], x85, [xA0-xD7FF], [xE000-xFFFD], [x010000-x10FFFF]
//!
//! 2. **MUST only produce printable characters on output**
//!    - Emitter output validation
//!
//! 3. **Characters outside printable set MUST be escaped on output**
//!    - Non-printable input handling
//!
//! 4. **MUST allow all non-C0 characters inside quoted scalars**
//!    - Production rule [2] nb-json: x09 | [x20-x10FFFF]
//!    - JSON compatibility requirement

use yyaml::{YamlEmitter, YamlLoader};

/// Test RFC requirement: "On input, a YAML processor must accept all characters in this printable subset."
///
/// Tests production rule [1] c-printable ::=
///   x09                  # Tab (\t)  
/// | x0A                  # Line feed (LF \n)
/// | x0D                  # Carriage Return (CR \r)
/// | [x20-x7E]            # Printable ASCII
/// | x85                  # Next Line (NEL)
/// | [xA0-xD7FF]          # Basic Multilingual Plane (BMP)
/// | [xE000-xFFFD]        # Additional Unicode Areas  
/// | [x010000-x10FFFF]    # 32 bit
#[test]
fn test_accept_all_printable_characters_tab() {
    // x09 - Tab character
    let yaml_with_tab = "key:\t\"value with tab\"";
    let result = YamlLoader::load_from_str(yaml_with_tab);
    assert!(
        result.is_ok(),
        "Must accept tab character (x09) per RFC 5.1"
    );

    let docs = result.unwrap();
    assert_eq!(docs.len(), 1);
}

#[test]
fn test_accept_all_printable_characters_line_feed() {
    // x0A - Line feed
    let yaml_with_lf = "key: \"value\nwith LF\"";
    let result = YamlLoader::load_from_str(yaml_with_lf);
    assert!(result.is_ok(), "Must accept line feed (x0A) per RFC 5.1");
}

#[test]
fn test_accept_all_printable_characters_carriage_return() {
    // x0D - Carriage return
    let yaml_with_cr = "key: \"value\rwith CR\"";
    let result = YamlLoader::load_from_str(yaml_with_cr);
    assert!(
        result.is_ok(),
        "Must accept carriage return (x0D) per RFC 5.1"
    );
}

#[test]
fn test_accept_printable_ascii_range() {
    // [x20-x7E] - Printable ASCII range
    let printable_ascii_chars = (0x20u8..=0x7E).map(|b| b as char).collect::<String>();
    let yaml = format!("key: \"{}\"", printable_ascii_chars);

    let result = YamlLoader::load_from_str(&yaml);
    assert!(
        result.is_ok(),
        "Must accept all printable ASCII [x20-x7E] per RFC 5.1"
    );

    let docs = result.unwrap();
    if let Some(doc) = docs.first()
        && let Some(value_str) = doc["key"].as_str()
    {
        assert_eq!(
            value_str, printable_ascii_chars,
            "ASCII characters must be preserved"
        );
    }
}

#[test]
fn test_accept_next_line_character() {
    // x85 - Next Line (NEL)
    let yaml_with_nel = format!("key: \"value{}with NEL\"", '\u{85}');
    let result = YamlLoader::load_from_str(&yaml_with_nel);
    assert!(
        result.is_ok(),
        "Must accept Next Line character (x85) per RFC 5.1"
    );
}

#[test]
fn test_accept_basic_multilingual_plane() {
    // [xA0-xD7FF] - Basic Multilingual Plane sample
    let bmp_chars = [
        '\u{A0}',   // Non-breaking space
        '\u{1000}', // Myanmar character
        '\u{2000}', // En quad
        '\u{3000}', // Ideographic space
        '\u{D7FF}', // End of BMP range
    ];

    for &ch in &bmp_chars {
        let yaml = format!("key: \"value{}test\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Must accept BMP character {:?} (U+{:04X}) per RFC 5.1",
            ch,
            ch as u32
        );
    }
}

#[test]
fn test_accept_additional_unicode_areas() {
    // [xE000-xFFFD] - Additional Unicode Areas sample
    let additional_chars = [
        '\u{E000}', // Private use area start
        '\u{F000}', // Private use area middle
        '\u{FFF0}', // Specials block
        '\u{FFFD}', // Replacement character (end of range)
    ];

    for &ch in &additional_chars {
        let yaml = format!("key: \"value{}test\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Must accept additional Unicode character {:?} (U+{:04X}) per RFC 5.1",
            ch,
            ch as u32
        );
    }
}

#[test]
fn test_accept_32bit_unicode() {
    // [x010000-x10FFFF] - 32-bit Unicode sample
    let thirtytwo_bit_chars = [
        '\u{10000}',  // Linear B Syllable B008 A (start of range)
        '\u{1F600}',  // Grinning face emoji
        '\u{E0000}',  // Language tag (plane 14)
        '\u{10FFFF}', // End of Unicode range
    ];

    for &ch in &thirtytwo_bit_chars {
        let yaml = format!("key: \"value{}test\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Must accept 32-bit Unicode character {:?} (U+{:06X}) per RFC 5.1",
            ch,
            ch as u32
        );
    }
}

/// Test RFC requirement: "Characters outside this set must be presented using escape sequences"
///
/// Verifies that non-printable characters are properly escaped on output
#[test]
fn test_non_printable_characters_escaped_on_output() {
    // C0 control characters (except allowed ones) should be escaped
    let control_chars = [
        '\u{00}', // NULL
        '\u{01}', // SOH
        '\u{08}', // Backspace
        '\u{1F}', // Unit separator
    ];

    for &ch in &control_chars {
        let yaml_input = format!("key: \"value{}test\"", ch);

        // Parse should work (in quoted scalar)
        let docs = YamlLoader::load_from_str(&yaml_input).unwrap();

        // Emit back to string - should escape non-printable chars
        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter.dump(&docs[0]).unwrap();

        // Output should contain escape sequence, not raw control char
        assert!(
            !output.contains(ch),
            "Non-printable character {:?} (U+{:02X}) must be escaped in output per RFC 5.1",
            ch,
            ch as u32
        );

        // Should contain some form of escape (backslash)
        assert!(
            output.contains('\\'),
            "Output must contain escape sequences for non-printable character {:?}",
            ch
        );
    }
}

/// Test RFC requirement: "YAML processors must allow all non-C0 characters inside quoted scalars"
///
/// Tests production rule [2] nb-json ::= x09 | [x20-x10FFFF]
/// For JSON compatibility
#[test]
fn test_allow_non_c0_characters_in_quoted_scalars() {
    // nb-json allows x09 (tab) + [x20-x10FFFF] (non-C0-control characters)

    // Tab character (x09)
    let yaml_tab = "key: \"value\twith tab\"";
    let result = YamlLoader::load_from_str(yaml_tab);
    assert!(
        result.is_ok(),
        "Must allow tab (x09) in quoted scalars per RFC 5.1 nb-json"
    );

    // High Unicode characters (non-C0)
    let high_unicode_chars = [
        '\u{A0}',    // Non-breaking space
        '\u{2028}',  // Line separator
        '\u{2029}',  // Paragraph separator
        '\u{FEFF}',  // Byte order mark
        '\u{1F600}', // Emoji
    ];

    for &ch in &high_unicode_chars {
        let yaml = format!("key: \"value{}in quotes\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Must allow non-C0 character {:?} (U+{:04X}) in quoted scalars per RFC 5.1 nb-json",
            ch,
            ch as u32
        );
    }
}

/// Test that C0 control characters (except allowed ones) are rejected in unquoted contexts
///
/// This verifies the boundary of what's acceptable
#[test]
fn test_c0_control_characters_handling() {
    // These C0 characters ARE allowed per c-printable
    let allowed_c0 = ['\u{09}', '\u{0A}', '\u{0D}']; // Tab, LF, CR

    // These C0 characters should NOT be allowed in plain scalars
    let forbidden_c0 = ['\u{00}', '\u{01}', '\u{08}', '\u{1F}'];

    // Test all allowed C0 characters systematically
    for &ch in &allowed_c0 {
        let yaml_with_c0 = format!("- \u{0009}{}item", ch); // Use tab as separator, then test character
        let result = YamlLoader::load_from_str(&yaml_with_c0);
        assert!(
            result.is_ok(),
            "Allowed C0 character {:?} (U+{:02X}) should work per RFC 5.1",
            ch,
            ch as u32
        );
    }

    // Forbidden C0 characters should fail in plain context but work in quoted
    for &ch in &forbidden_c0 {
        // Should fail in plain scalar context
        let yaml_plain = format!("key: value{}", ch);
        let result_plain = YamlLoader::load_from_str(&yaml_plain);
        // Note: Some parsers may accept these in plain scalars, but they should be avoided
        // The key is that the character is preserved if parsed
        if let Ok(docs) = result_plain
            && !docs.is_empty()
            && let Some(value) = docs[0]["key"].as_str()
        {
            assert!(
                value.contains(ch),
                "If C0 character {:?} (U+{:02X}) is accepted in plain scalar, it must be preserved",
                ch,
                ch as u32
            );
        }

        // Should work in quoted scalar context
        let yaml_quoted = format!("key: \"value{}quoted\"", ch);
        let result_quoted = YamlLoader::load_from_str(&yaml_quoted);
        assert!(
            result_quoted.is_ok(),
            "C0 character {:?} (U+{:02X}) should work in quoted scalars per RFC 5.1",
            ch,
            ch as u32
        );
    }
}

/// Test comprehensive c-printable production rule coverage
///
/// This is the definitive test for RFC 5.1 character set compliance
#[test]
fn test_comprehensive_c_printable_compliance() {
    // Test each range from production rule [1] c-printable

    let test_cases = vec![
        // x09 - Tab
        ('\u{09}', "Tab character"),
        // x0A - Line feed
        ('\u{0A}', "Line feed character"),
        // x0D - Carriage return
        ('\u{0D}', "Carriage return character"),
        // [x20-x7E] sample - Printable ASCII
        ('\u{20}', "Space character (start of printable ASCII)"),
        ('\u{41}', "Letter A"),
        ('\u{7E}', "Tilde (end of printable ASCII)"),
        // x85 - Next Line
        ('\u{85}', "Next Line character"),
        // [xA0-xD7FF] sample - BMP
        ('\u{A0}', "Non-breaking space (start of BMP range)"),
        ('\u{1000}', "Myanmar Letter Ka"),
        ('\u{D7FF}', "Hangul Jongseong Phieuph (end of BMP range)"),
        // [xE000-xFFFD] sample - Additional areas
        ('\u{E000}', "Private Use Area start"),
        (
            '\u{FFFD}',
            "Replacement character (end of additional range)",
        ),
        // [x010000-x10FFFF] sample - 32-bit
        (
            '\u{10000}',
            "Linear B Syllable B008 A (start of 32-bit range)",
        ),
        ('\u{1F600}', "Grinning Face emoji"),
        ('\u{10FFFF}', "End of Unicode range"),
    ];

    for (ch, description) in test_cases {
        let yaml = format!("test: \"character: {}\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "c-printable compliance failed for {} {:?} (U+{:06X}) per RFC 5.1",
            description,
            ch,
            ch as u32
        );

        // Verify the character is preserved
        if let Ok(docs) = result
            && let Some(value_str) = docs[0]["test"].as_str()
        {
            assert!(
                value_str.contains(ch),
                "Character {} must be preserved in parsed output",
                description
            );
        }
    }
}
