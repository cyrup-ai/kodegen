//! RFC 8.1.2 Literal Style Block Scalars
//!
//! Tests literal style (|) block scalar production rules
//! References: ../../../../../docs/ch08-block-style-productions/scalar-styles/

use yyaml::YamlLoader;

/// Test basic literal style preservation
#[test]
fn test_literal_style_preserves_newlines() {
    let yaml = r#"
literal: |
  Line 1
  Line 2
  
  Line 4 after blank
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["literal"].as_str().unwrap();
    
    // Literal style MUST preserve all newlines
    assert_eq!(content, "Line 1\nLine 2\n\nLine 4 after blank\n");
    assert_eq!(content.matches('\n').count(), 4);
}

/// Test literal style with chomping indicators
#[test]
fn test_literal_chomping_indicators() {
    // Strip chomping (-)
    let yaml = r#"
strip: |-
  Content
  
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["strip"].as_str().unwrap(), "Content");
    
    // Clip chomping (default)
    let yaml = r#"
clip: |
  Content
  
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["clip"].as_str().unwrap(), "Content\n");
    
    // Keep chomping (+)
    let yaml = r#"
keep: |+
  Content
  
  
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["keep"].as_str().unwrap(), "Content\n\n\n");
}/// Test explicit indentation indicator
#[test]
fn test_explicit_indentation_indicator() {
    let yaml = r#"
explicit: |2
    Two spaces
      Four spaces
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let content = docs[0]["explicit"].as_str().unwrap();
    
    // With explicit indent of 2, content beyond that is preserved
    assert_eq!(content, "  Two spaces\n    Four spaces\n");
}

/// Test combined indicators (order independence)
#[test]
fn test_combined_indicators() {
    // Chomping then indentation
    let yaml1 = r#"
test: |-2
    Content
"#;
    
    // Indentation then chomping  
    let yaml2 = r#"
test: |2-
    Content
"#;
    
    let docs1 = YamlLoader::load_from_str(yaml1).unwrap();
    let docs2 = YamlLoader::load_from_str(yaml2).unwrap();
    
    // Both should produce same result
    assert_eq!(
        docs1[0]["test"].as_str().unwrap(),
        docs2[0]["test"].as_str().unwrap()
    );
    assert_eq!(docs1[0]["test"].as_str().unwrap(), "  Content");
}