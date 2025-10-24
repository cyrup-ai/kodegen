//! RFC 8.1.3 Folded Style Block Scalars
//!
//! Tests folded style (>) block scalar production rules
//! References: ../../../../../docs/ch08-block-style-productions/scalar-styles/

use yyaml::YamlLoader;

/// Test basic folded style line folding
#[test]
fn test_folded_style_line_folding() {
    let yaml = r#"
folded: >
  This is a long line
  that should be folded
  into a single line.
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["folded"].as_str().unwrap();
    
    // Folded style should join lines with spaces
    assert_eq!(content, "This is a long line that should be folded into a single line.\n");
}

/// Test folded style with blank lines
#[test]
fn test_folded_style_blank_lines() {
    let yaml = r#"
folded: >
  Line 1
  Line 2

  Line 4 after blank
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["folded"].as_str().unwrap();
    
    // Blank lines should be preserved
    assert_eq!(content, "Line 1 Line 2\n\nLine 4 after blank\n");
}

/// Test folded style with chomping indicators
#[test]
fn test_folded_chomping_indicators() {
    // Strip chomping (-)
    let yaml = r#"
strip: >-
  Content line
  
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["strip"].as_str().unwrap(), "Content line");
    
    // Keep chomping (+)
    let yaml = r#"
keep: >+
  Content line
  
  
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["keep"].as_str().unwrap(), "Content line\n\n\n");
}

/// Test folded style with more indented lines
#[test]
fn test_folded_style_more_indented() {
    let yaml = r#"
folded: >
  Normal line
    More indented line
  Back to normal
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["folded"].as_str().unwrap();
    
    // More indented lines should be preserved
    assert!(content.contains("  More indented line"));
}