//! RFC 8.1.4 Chomping Indicators Compliance Tests
//!
//! Tests chomping behavior for block scalars
//! References: ../../../../../docs/ch08-block-style-productions/scalar-styles/

use yyaml::YamlLoader;

/// Test strip chomping indicator (-)
#[test]
fn test_strip_chomping() {
    // Literal with strip
    let yaml = r#"
literal: |-
  Content
  

"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["literal"].as_str().unwrap(), "Content");
    
    // Folded with strip
    let yaml = r#"
folded: >-
  Content line
  

"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["folded"].as_str().unwrap(), "Content line");
}

/// Test clip chomping (default behavior)
#[test]
fn test_clip_chomping() {
    // Literal with clip (default)
    let yaml = r#"
literal: |
  Content
  

"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["literal"].as_str().unwrap(), "Content\n");
    
    // Folded with clip (default)
    let yaml = r#"
folded: >
  Content line
  

"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["folded"].as_str().unwrap(), "Content line\n");
}

/// Test keep chomping indicator (+)
#[test]
fn test_keep_chomping() {
    // Literal with keep
    let yaml = r#"
literal: |+
  Content
  


"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["literal"].as_str().unwrap(), "Content\n\n\n\n");
    
    // Folded with keep
    let yaml = r#"
folded: >+
  Content line
  


"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["folded"].as_str().unwrap(), "Content line\n\n\n\n");
}