//! RFC 8.1.1 Block Scalar Headers Compliance Tests
//!
//! Tests block scalar header indicators
//! References: ../../../../../docs/ch08-block-style-productions/scalar-styles/

use yyaml::YamlLoader;

/// Test block scalar style indicators
#[test]
fn test_literal_style_indicator() {
    let yaml = r#"
literal: |
  Line 1
  Line 2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["literal"].as_str().unwrap();
    assert_eq!(content, "Line 1\nLine 2\n");
}

#[test]
fn test_folded_style_indicator() {
    let yaml = r#"
folded: >
  Line 1
  Line 2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["folded"].as_str().unwrap();
    assert_eq!(content, "Line 1 Line 2\n");
}

/// Test explicit indentation indicators
#[test]
fn test_explicit_indentation_indicators() {
    let yaml = r#"
explicit: |2
    Content
      More content
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["explicit"].as_str().unwrap();
    assert_eq!(content, "  Content\n    More content\n");
}