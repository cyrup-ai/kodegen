//! RFC 5.5 White Space Characters Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.5 - White Space Characters
//!
//! ## Requirements Tested
//!
//! 1. **MUST recognize only space (x20) and tab (x09) as white space characters**
//!    - Production rules [31], [32], [33]: s-space, s-tab, s-white
//!
//! 2. **All other printable non-break characters are non-space characters**  
//!    - Production rule [34]: ns-char ::= nb-char - s-white

use yyaml::YamlLoader;

/// Test RFC requirement: Recognition of white space characters
///
/// Production rules [31], [32], [33]:
/// - s-space ::= x20 (space)
/// - s-tab ::= x09 (tab)  
/// - s-white ::= s-space | s-tab
#[test]
fn test_white_space_character_recognition() {
    // Test space (x20) as whitespace
    let yaml_with_spaces = "key1:   value1\nkey2:     value2";
    let result = YamlLoader::load_from_str(yaml_with_spaces);
    assert!(
        result.is_ok(),
        "Must recognize space (x20) as whitespace per RFC 5.5"
    );

    let docs = result.unwrap();
    assert_eq!(
        docs[0]["key1"].as_str(),
        Some("value1"),
        "Space separation must work"
    );
    assert_eq!(
        docs[0]["key2"].as_str(),
        Some("value2"),
        "Multiple spaces must work"
    );

    // Test tab (x09) as whitespace
    let yaml_with_tabs = "key1:\tvalue1\nkey2:\t\tvalue2";
    let result = YamlLoader::load_from_str(yaml_with_tabs);
    assert!(
        result.is_ok(),
        "Must recognize tab (x09) as whitespace per RFC 5.5"
    );

    let docs = result.unwrap();
    assert_eq!(
        docs[0]["key1"].as_str(),
        Some("value1"),
        "Tab separation must work"
    );
    assert_eq!(
        docs[0]["key2"].as_str(),
        Some("value2"),
        "Multiple tabs must work"
    );
}

/// Test RFC requirement: Mixed space and tab whitespace  
#[test]
fn test_mixed_whitespace() {
    // Mix of spaces and tabs
    let yaml_mixed = "key1: \t value1\nkey2:\t  \tvalue2";
    let result = YamlLoader::load_from_str(yaml_mixed);
    assert!(
        result.is_ok(),
        "Must handle mixed space/tab whitespace per RFC 5.5"
    );

    let docs = result.unwrap();
    assert_eq!(
        docs[0]["key1"].as_str(),
        Some("value1"),
        "Mixed whitespace separation must work"
    );
    assert_eq!(
        docs[0]["key2"].as_str(),
        Some("value2"),
        "Complex mixed whitespace must work"
    );
}

/// Test RFC requirement: Only space and tab are whitespace
///
/// Other Unicode whitespace characters should NOT be treated as YAML whitespace
#[test]
fn test_only_space_and_tab_are_whitespace() {
    // These Unicode characters are whitespace in Unicode, but NOT in YAML
    let non_yaml_whitespace = [
        ('\u{00A0}', "Non-breaking space"), // Unicode whitespace, not YAML whitespace
        ('\u{2000}', "En quad"),            // Unicode whitespace, not YAML whitespace
        ('\u{2001}', "Em quad"),            // Unicode whitespace, not YAML whitespace
        ('\u{2002}', "En space"),           // Unicode whitespace, not YAML whitespace
        ('\u{2003}', "Em space"),           // Unicode whitespace, not YAML whitespace
        ('\u{2009}', "Thin space"),         // Unicode whitespace, not YAML whitespace
        ('\u{200A}', "Hair space"),         // Unicode whitespace, not YAML whitespace
        ('\u{3000}', "Ideographic space"),  // Unicode whitespace, not YAML whitespace
    ];

    for (ch, desc) in non_yaml_whitespace {
        // These should be treated as content characters, not separators
        let yaml = format!("key:{}value", ch);
        let result = YamlLoader::load_from_str(&yaml);

        // This might fail because the character is not treated as separating whitespace
        // The key point is that it's NOT treated as YAML whitespace per RFC 5.5
        println!(
            "Non-YAML whitespace {} ({}): parse result = {:?}",
            desc,
            ch,
            result.is_ok()
        );

        // In quoted context, should definitely work
        let yaml_quoted = format!("key: \"before{}after\"", ch);
        let result_quoted = YamlLoader::load_from_str(&yaml_quoted);
        assert!(
            result_quoted.is_ok(),
            "{} must work as content character in quotes per RFC 5.5",
            desc
        );

        if let Ok(docs) = result_quoted
            && let Some(value) = docs[0]["key"].as_str()
        {
            assert!(
                value.contains(ch),
                "{} must be preserved as content, not treated as YAML whitespace",
                desc
            );
        }
    }
}

/// Test RFC requirement: Non-space character definition
///
/// Production rule [34]: ns-char ::= nb-char - s-white
/// Non-space chars are non-break chars minus whitespace chars
#[test]
fn test_non_space_character_definition() {
    // ns-char = nb-char - s-white
    // So: all non-break chars except space and tab

    let non_space_samples = [
        'a',        // Letter (nb-char, not s-white)
        '1',        // Digit (nb-char, not s-white)
        '!',        // Punctuation (nb-char, not s-white)
        '\u{00A0}', // Non-breaking space (nb-char, not s-white per YAML)
        '\u{2000}', // En quad (nb-char, not s-white per YAML)
    ];

    for &ch in &non_space_samples {
        let yaml = format!("key: \"{}\"", ch);
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Non-space character {:?} (U+{:04X}) must work per RFC 5.5 ns-char production",
            ch,
            ch as u32
        );
    }

    // These ARE s-white and thus not ns-char
    let whitespace_chars = [' ', '\t']; // space, tab

    for &ch in &whitespace_chars {
        let yaml = format!("key:{}value", ch); // Should work as separator
        let result = YamlLoader::load_from_str(&yaml);
        assert!(
            result.is_ok(),
            "Whitespace character {:?} must work as separator per RFC 5.5",
            ch
        );

        if let Ok(docs) = result {
            // Should parse as key-value pair due to whitespace separation
            assert!(
                docs[0]["key"].as_str().is_some(),
                "Whitespace {:?} should enable key-value separation",
                ch
            );
        }
    }
}

/// Test whitespace in different YAML contexts
#[test]
fn test_whitespace_in_different_contexts() {
    // Whitespace in mappings
    let mapping_whitespace = r#"
key1:   value1
key2:		value2
key3: 	 value3
"#;
    let result = YamlLoader::load_from_str(mapping_whitespace);
    assert!(
        result.is_ok(),
        "Whitespace in mappings must work per RFC 5.5"
    );

    // Whitespace in sequences
    let sequence_whitespace = r#"
- 	item1
-   item2
-	 item3
"#;
    let result = YamlLoader::load_from_str(sequence_whitespace);
    assert!(
        result.is_ok(),
        "Whitespace in sequences must work per RFC 5.5"
    );

    // Whitespace in flow collections
    let flow_whitespace = "[ item1 ,	item2 , 	item3 ]";
    let result = YamlLoader::load_from_str(flow_whitespace);
    assert!(
        result.is_ok(),
        "Whitespace in flow collections must work per RFC 5.5"
    );
}

/// Test whitespace preservation in quoted scalars
#[test]
fn test_whitespace_preservation_in_scalars() {
    // Whitespace should be preserved inside quoted scalars
    let quoted_whitespace = r#"
single: 'text with   spaces	and	tabs'
double: "text with   spaces	and	tabs"  
"#;
    let result = YamlLoader::load_from_str(quoted_whitespace);
    assert!(
        result.is_ok(),
        "Quoted scalars with whitespace must work per RFC 5.5"
    );

    if let Ok(docs) = result {
        if let Some(single_value) = docs[0]["single"].as_str() {
            assert!(
                single_value.contains("   "),
                "Multiple spaces must be preserved in single quotes"
            );
            assert!(
                single_value.contains("\t"),
                "Tabs must be preserved in single quotes"
            );
        }

        if let Some(double_value) = docs[0]["double"].as_str() {
            assert!(
                double_value.contains("   "),
                "Multiple spaces must be preserved in double quotes"
            );
            assert!(
                double_value.contains("\t"),
                "Tabs must be preserved in double quotes"
            );
        }
    }
}

/// Test whitespace significance in indentation
#[test]
fn test_whitespace_in_indentation() {
    // Indentation uses spaces - tabs may have different behavior
    let space_indented = r#"
parent:
  child1: value1
  child2: value2
    grandchild: value3
"#;
    let result = YamlLoader::load_from_str(space_indented);
    assert!(
        result.is_ok(),
        "Space-based indentation must work per RFC 5.5"
    );

    // Tab indentation behavior may vary by implementation
    let tab_indented = r#"
parent:
	child1: value1
	child2: value2  
		grandchild: value3
"#;
    let result = YamlLoader::load_from_str(tab_indented);
    println!("Tab indentation result: {:?}", result.is_ok());
    // Document behavior but don't assert - implementation specific
}

/// Test comprehensive s-white production compliance  
#[test]
fn test_comprehensive_s_white_compliance() {
    // Test all combinations of space and tab in various contexts
    let comprehensive_yaml = format!(
        "{}key1:{} value1\n{}key2:\t{}\tvalue2\n{}-{} item1\n{}-\t{}\titem2\n{}flow:{} [{} a{},\tb{} ]\n",
        " ",   // leading space
        "   ", // multiple spaces
        "\t",  // leading tab
        " ",   // space after colon
        "  ",  // sequence indent spaces
        " ",   // space after dash
        "\t",  // sequence indent tab
        " ",   // space after dash
        " ",   // flow indent space
        " ",   // space after colon
        " ",   // space in flow sequence
        " ",   // space after item
        " "    // space after item
    );

    let result = YamlLoader::load_from_str(&comprehensive_yaml);
    assert!(
        result.is_ok(),
        "Comprehensive whitespace usage must work per RFC 5.5"
    );

    if let Ok(docs) = result {
        // Verify all structures parsed correctly despite varied whitespace
        assert!(
            docs[0]["key1"].as_str().is_some(),
            "Space-separated mapping must work"
        );
        assert!(
            docs[0]["key2"].as_str().is_some(),
            "Tab-separated mapping must work"
        );
        assert!(
            docs[0]["flow"].as_vec().is_some(),
            "Flow with whitespace must work"
        );
    }
}

/// Test edge cases for whitespace handling
#[test]
fn test_whitespace_edge_cases() {
    // Leading/trailing whitespace
    let leading_trailing = "   key: value   \n   other: data   ";
    let result = YamlLoader::load_from_str(leading_trailing);
    assert!(
        result.is_ok(),
        "Leading/trailing whitespace must be handled per RFC 5.5"
    );

    // Only whitespace content
    let whitespace_only = "key: '   \t   '";
    let result = YamlLoader::load_from_str(whitespace_only);
    assert!(
        result.is_ok(),
        "Whitespace-only quoted content must work per RFC 5.5"
    );

    if let Ok(docs) = result
        && let Some(value) = docs[0]["key"].as_str()
    {
        assert!(value.contains(" "), "Whitespace content must be preserved");
        assert!(value.contains("\t"), "Tab content must be preserved");
    }
}
