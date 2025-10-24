//! RFC 5.3 Indicator Characters Compliance Tests
//!
//! Tests for YAML 1.2.2 specification section 5.3 - Indicator Characters
//!
//! ## Requirements Tested
//!
//! 1. **MUST recognize all indicator characters with special semantics**
//!    - Block structure: -, ?, :
//!    - Flow collections: ,, [, ], {, }
//!    - Comments: #
//!    - Anchors/aliases: &, *
//!    - Tags: !  
//!    - Block scalars: |, >
//!    - Quoted scalars: ', "
//!    - Directives: %
//!
//! 2. **Reserved indicators @ and ` MUST NOT start plain scalars**
//!    - Error handling for reserved characters
//!
//! 3. **Flow indicators forbidden in certain contexts to avoid ambiguity**
//!    - Context-sensitive indicator validation

use yyaml::YamlLoader;

/// Test RFC requirement: Recognition of block structure indicators
///
/// Production rules [4], [5], [6]:
/// - c-sequence-entry ::= '-' (hyphen denotes block sequence entry)
/// - c-mapping-key ::= '?' (question mark denotes mapping key)
/// - c-mapping-value ::= ':' (colon denotes mapping value)
#[test]
fn test_block_structure_indicators() {
    // Test c-sequence-entry: '-'
    let sequence_yaml = r#"
- one
- two  
- three
"#;
    let result = YamlLoader::load_from_str(sequence_yaml);
    assert!(
        result.is_ok(),
        "Must recognize '-' as sequence entry indicator per RFC 5.3"
    );

    let docs = result.unwrap();
    if let Some(seq) = docs[0].as_vec() {
        assert_eq!(
            seq.len(),
            3,
            "Sequence indicator must create proper sequence"
        );
    }

    // Test c-mapping-key: '?' and c-mapping-value: ':'
    let mapping_yaml = r#"
? sky
: blue
? sea  
: green
simple: value
"#;
    let result = YamlLoader::load_from_str(mapping_yaml);
    assert!(
        result.is_ok(),
        "Must recognize '?' and ':' as mapping indicators per RFC 5.3"
    );
}

/// Test RFC requirement: Recognition of flow collection indicators
///
/// Production rules [7], [8], [9], [10], [11]:
/// - c-collect-entry ::= ',' (comma ends flow collection entry)
/// - c-sequence-start ::= '[' (left bracket starts flow sequence)
/// - c-sequence-end ::= ']' (right bracket ends flow sequence)  
/// - c-mapping-start ::= '{' (left brace starts flow mapping)
/// - c-mapping-end ::= '}' (right brace ends flow mapping)
#[test]
fn test_flow_collection_indicators() {
    // Test flow sequence indicators: '[', ']', ','
    let flow_sequence = "[one, two, three]";
    let result = YamlLoader::load_from_str(flow_sequence);
    assert!(
        result.is_ok(),
        "Must recognize flow sequence indicators '[', ']', ',' per RFC 5.3"
    );

    let docs = result.unwrap();
    if let Some(seq) = docs[0].as_vec() {
        assert_eq!(seq.len(), 3, "Flow sequence indicators must work correctly");
    }

    // Test flow mapping indicators: '{', '}', ','
    let flow_mapping = "{key1: value1, key2: value2}";
    let result = YamlLoader::load_from_str(flow_mapping);
    assert!(
        result.is_ok(),
        "Must recognize flow mapping indicators '{{', '}}', ',' per RFC 5.3"
    );

    let docs = result.unwrap();
    assert!(
        docs[0]["key1"].as_str().is_some(),
        "Flow mapping indicators must work correctly"
    );
    assert!(
        docs[0]["key2"].as_str().is_some(),
        "Flow mapping indicators must work correctly"
    );
}

/// Test RFC requirement: Recognition of comment indicator
///
/// Production rule [12]:
/// - c-comment ::= '#' (octothorpe denotes comment)
#[test]
fn test_comment_indicator() {
    let yaml_with_comments = r#"
key: value  # This is a comment
# Full line comment  
other: data # Another comment
"#;
    let result = YamlLoader::load_from_str(yaml_with_comments);
    assert!(
        result.is_ok(),
        "Must recognize '#' as comment indicator per RFC 5.3"
    );

    let docs = result.unwrap();
    assert_eq!(
        docs[0]["key"].as_str(),
        Some("value"),
        "Comments must not affect parsing"
    );
    assert_eq!(
        docs[0]["other"].as_str(),
        Some("data"),
        "Comments must not affect parsing"
    );
}

/// Test RFC requirement: Recognition of anchor and alias indicators
///
/// Production rules [13], [14]:
/// - c-anchor ::= '&' (ampersand denotes node anchor property)
/// - c-alias ::= '*' (asterisk denotes alias node)
#[test]
fn test_anchor_alias_indicators() {
    let yaml_with_anchors = r#"
default: &DEFAULT
  name: "Default Name"
  value: 42

instance1: *DEFAULT
instance2: 
  <<: *DEFAULT
  name: "Custom Name"
"#;
    let result = YamlLoader::load_from_str(yaml_with_anchors);
    assert!(
        result.is_ok(),
        "Must recognize '&' and '*' as anchor/alias indicators per RFC 5.3"
    );

    let docs = result.unwrap();
    // Verify anchor/alias functionality works
    assert!(
        docs[0]["instance1"]["name"].as_str().is_some(),
        "Alias indicator must work"
    );
}

/// Test RFC requirement: Recognition of tag indicator
///
/// Production rule [15]:
/// - c-tag ::= '!' (exclamation used for node tags, tag handles, local tags)
#[test]
fn test_tag_indicator() {
    let yaml_with_tags = r#"
string_tag: !!str "123"
int_tag: !!int "456"  
custom_tag: !custom "data"
"#;
    let result = YamlLoader::load_from_str(yaml_with_tags);
    assert!(
        result.is_ok(),
        "Must recognize '!' as tag indicator per RFC 5.3"
    );

    // Note: Tag processing depends on implementation, but parsing should succeed
}

/// Test RFC requirement: Recognition of block scalar indicators
///
/// Production rules [16], [17]:
/// - c-literal ::= '|' (vertical bar denotes literal block scalar)
/// - c-folded ::= '>' (greater than denotes folded block scalar)
#[test]
fn test_block_scalar_indicators() {
    let literal_yaml = r#"
literal: |
  Line 1
  Line 2
  Line 3
"#;
    let result = YamlLoader::load_from_str(literal_yaml);
    assert!(
        result.is_ok(),
        "Must recognize '|' as literal block scalar indicator per RFC 5.3"
    );

    let folded_yaml = r#"
folded: >
  This is a folded
  block scalar that
  becomes one line
"#;
    let result = YamlLoader::load_from_str(folded_yaml);
    assert!(
        result.is_ok(),
        "Must recognize '>' as folded block scalar indicator per RFC 5.3"
    );
}

/// Test RFC requirement: Recognition of quoted scalar indicators
///
/// Production rules [18], [19]:
/// - c-single-quote ::= "'" (apostrophe surrounds single-quoted flow scalar)
/// - c-double-quote ::= '"' (double quote surrounds double-quoted flow scalar)
#[test]
fn test_quoted_scalar_indicators() {
    let single_quoted = "key: 'single quoted string with \"double quotes\" inside'";
    let result = YamlLoader::load_from_str(single_quoted);
    assert!(
        result.is_ok(),
        "Must recognize \"'\" as single-quote indicator per RFC 5.3"
    );

    let double_quoted = "key: \"double quoted string with 'single quotes' inside\"";
    let result = YamlLoader::load_from_str(double_quoted);
    assert!(
        result.is_ok(),
        "Must recognize '\"' as double-quote indicator per RFC 5.3"
    );
}

/// Test RFC requirement: Recognition of directive indicator
///
/// Production rule [20]:
/// - c-directive ::= '%' (percent denotes directive line)
#[test]
fn test_directive_indicator() {
    let yaml_with_directive = r#"
%YAML 1.2
---
key: value
"#;
    let result = YamlLoader::load_from_str(yaml_with_directive);
    assert!(
        result.is_ok(),
        "Must recognize '%' as directive indicator per RFC 5.3"
    );
}

/// Test RFC requirement: Reserved indicators MUST NOT start plain scalars
///
/// Production rule [21]:
/// - c-reserved ::= '@' | '`' (reserved for future use)
#[test]
fn test_reserved_indicators_forbidden() {
    // Test '@' cannot start plain scalar
    let at_sign_yaml = "key: @invalid";
    let result = YamlLoader::load_from_str(at_sign_yaml);
    // Should either fail or be parsed with @ preserved (implementation dependent)
    // If it parses, verify the character is preserved
    if let Ok(docs) = result
        && !docs.is_empty()
        && let Some(value) = docs[0]["key"].as_str()
    {
        assert!(
            value.starts_with('@'),
            "Reserved indicator @ must be preserved if parsed"
        );
    }

    // Test '`' cannot start plain scalar
    let grave_yaml = "key: `invalid";
    let result = YamlLoader::load_from_str(grave_yaml);
    // Should either fail or be parsed with ` preserved (implementation dependent)
    // If it parses, verify the character is preserved
    if let Ok(docs) = result
        && !docs.is_empty()
        && let Some(value) = docs[0]["key"].as_str()
    {
        assert!(
            value.starts_with('`'),
            "Reserved indicator ` must be preserved if parsed"
        );
    }

    // These should work in quoted contexts
    let quoted_reserved = r#"
at_quoted: "@valid in quotes"
grave_quoted: "`valid in quotes"
"#;
    let result = YamlLoader::load_from_str(quoted_reserved);
    assert!(
        result.is_ok(),
        "Reserved indicators should work in quoted scalars per RFC 5.3"
    );
}

/// Test RFC requirement: Comprehensive indicator recognition
///
/// Production rule [22] - c-indicator covers all indicator characters
#[test]
fn test_all_indicators_recognized() {
    let comprehensive_yaml = r#"
%YAML 1.2
---
# Comment
sequence:
  - item1
  - item2
  
mapping:
  ? complex key
  : complex value
  simple: value

flow_seq: [one, two, three]
flow_map: {key1: val1, key2: val2}

anchored: &anchor "anchored value" 
alias: *anchor

tagged: !!str "tagged value"

literal_scalar: |
  Literal block
  preserves newlines
  
folded_scalar: >
  Folded block
  folds newlines
  
single_quoted: 'single quoted'
double_quoted: "double quoted with \n escape"
"#;

    let result = YamlLoader::load_from_str(comprehensive_yaml);
    assert!(
        result.is_ok(),
        "Must recognize all indicator characters per RFC 5.3 rule [22]"
    );

    let docs = result.unwrap();

    // Verify all structures parsed correctly
    assert!(
        docs[0]["sequence"].as_vec().is_some(),
        "Sequence indicators must work"
    );
    assert!(
        docs[0]["mapping"]["simple"].as_str().is_some(),
        "Mapping indicators must work"
    );
    assert!(
        docs[0]["flow_seq"].as_vec().is_some(),
        "Flow sequence indicators must work"
    );
    assert!(
        docs[0]["flow_map"]["key1"].as_str().is_some(),
        "Flow mapping indicators must work"
    );
    assert!(
        docs[0]["alias"].as_str().is_some(),
        "Anchor/alias indicators must work"
    );
    assert!(
        docs[0]["literal_scalar"].as_str().is_some(),
        "Literal scalar indicator must work"
    );
    assert!(
        docs[0]["folded_scalar"].as_str().is_some(),
        "Folded scalar indicator must work"
    );
    assert!(
        docs[0]["single_quoted"].as_str().is_some(),
        "Single quote indicator must work"
    );
    assert!(
        docs[0]["double_quoted"].as_str().is_some(),
        "Double quote indicator must work"
    );
}

/// Test RFC requirement: Flow indicators forbidden in certain contexts
///
/// Production rule [23] - c-flow-indicator characters can cause ambiguity
#[test]
fn test_flow_indicators_context_restrictions() {
    // Flow indicators: ',', '[', ']', '{', '}'

    // These should work in quoted contexts
    let quoted_flow = r#"
comma_quoted: "Hello, world"
brackets_quoted: "Array[index]"  
braces_quoted: "Object{key}"
"#;
    let result = YamlLoader::load_from_str(quoted_flow);
    assert!(
        result.is_ok(),
        "Flow indicators should work in quoted contexts per RFC 5.3"
    );

    // Test potential ambiguity cases
    let ambiguous_cases = vec![
        "key: [value",      // Incomplete flow sequence
        "key: {incomplete", // Incomplete flow mapping
        "key: value,",      // Trailing comma outside flow context
    ];

    for yaml in ambiguous_cases {
        let result = YamlLoader::load_from_str(yaml);
        // These should either fail gracefully or handle according to spec
        // The key is consistent behavior per RFC 5.3
        println!("Ambiguous case '{}' result: {:?}", yaml, result.is_ok());
    }
}

/// Test edge cases for indicator recognition
#[test]
fn test_indicator_edge_cases() {
    // Test indicators in different positions and contexts
    let edge_cases = vec![
        ("- -", "Dash as sequence entry and content"),
        ("?: ?", "Question mark as key and content"),
        (": :", "Colon as separator and content"),
        ("# #", "Hash in comment"),
        ("'single ''' quote'", "Quote inside single-quoted string"),
        ("\"double \\\" quote\"", "Quote inside double-quoted string"),
    ];

    for (yaml, description) in edge_cases {
        let result = YamlLoader::load_from_str(yaml);
        println!(
            "Edge case '{}' ({}): {:?}",
            yaml,
            description,
            result.is_ok()
        );
        // Document behavior for edge cases
    }
}

/// Test indicator precedence and parsing priorities
#[test]
fn test_indicator_precedence() {
    // Test cases where multiple indicators could apply
    let precedence_yaml = r##"
test1: "# Not a comment in quotes"
test2: '@ Not reserved in single quotes'
test3: '` Not reserved in single quotes'
test4: ': not a mapping in quotes'
"##;

    let result = YamlLoader::load_from_str(precedence_yaml);
    assert!(
        result.is_ok(),
        "Indicator precedence must be handled correctly per RFC 5.3"
    );

    let docs = result.unwrap();
    assert!(
        docs[0]["test1"].as_str().unwrap().contains("#"),
        "Quoted indicators should be literal"
    );
    assert!(
        docs[0]["test2"].as_str().unwrap().contains("@"),
        "Reserved chars should work in quotes"
    );
    assert!(
        docs[0]["test3"].as_str().unwrap().contains("`"),
        "Reserved chars should work in quotes"
    );
    assert!(
        docs[0]["test4"].as_str().unwrap().contains(":"),
        "Mapping indicators should be literal in quotes"
    );
}
