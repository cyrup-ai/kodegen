//! RFC 8.2.2 Block Mappings Compliance Tests
//!
//! Tests block mapping production rules
//! References: ../../../../../docs/ch08-block-style-productions/collection-styles/

use yyaml::YamlLoader;

/// Test basic block mapping syntax
#[test]
fn test_basic_block_mappings() {
    let yaml = r#"
key1: value1
key2: value2
key3: value3
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
    assert_eq!(docs[0]["key3"].as_str().unwrap(), "value3");
}

/// Test nested block mappings
#[test]
fn test_nested_block_mappings() {
    let yaml = r#"
outer:
  inner1: value1
  inner2: value2
another: value3
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["outer"]["inner1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["outer"]["inner2"].as_str().unwrap(), "value2");
    assert_eq!(docs[0]["another"].as_str().unwrap(), "value3");
}

/// Test block mapping with sequences
#[test]
fn test_block_mapping_with_sequences() {
    let yaml = r#"
items:
  - first
  - second
  - third
config:
  debug: true
  port: 8080
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["items"].as_vec().unwrap().len(), 3);
    assert_eq!(docs[0]["config"]["debug"].as_bool().unwrap(), true);
    assert_eq!(docs[0]["config"]["port"].as_i64().unwrap(), 8080);
}