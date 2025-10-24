//! RFC 6.8 Directives Compliance Tests
//!
//! Tests directive handling and processing
//! References: ../../../docs/ch06-structural-productions/6.8-directives.md

use yyaml::YamlLoader;

/// Test YAML directive processing
#[test]
fn test_yaml_directive() {
    let yaml = r#"%YAML 1.2
---
content: value"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["content"].as_str().unwrap(), "value");
}

/// Test TAG directive processing
#[test]
fn test_tag_directive() {
    let yaml = r#"%TAG ! tag:example.com,2000:app/
---
!shape Circle"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    // Tag directive should be processed but content preserved
    assert_eq!(docs[0].as_str().unwrap(), "Circle");
}

/// Test multiple directives
#[test]
fn test_multiple_directives() {
    let yaml = r#"%YAML 1.2
%TAG ! tag:example.com,2000:app/
%TAG !! tag:yaml.org,2002:
---
content: value"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["content"].as_str().unwrap(), "value");
}

/// Test reserved directive handling
#[test]
fn test_reserved_directive() {
    // Reserved directives should be ignored
    let yaml = r#"%FUTURE 1.0
---
content: value"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["content"].as_str().unwrap(), "value");
}