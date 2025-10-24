//! RFC 7.4 Flow Collection Styles Compliance Tests
//!
//! Tests production rules [136-150] for flow collection parsing
//! References: ../../../docs/ch07-flow-style-productions/7.4-flow-collection-styles.md

use yyaml::YamlLoader;

/// Test flow sequence syntax
#[test]
fn test_flow_sequences() {
    // Basic flow sequence
    let yaml = r#"[item1, item2, item3]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str().unwrap(), "item1");
    
    // Nested flow sequences
    let yaml = r#"[[1, 2], [3, 4]]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let outer = docs[0].as_vec().unwrap();
    assert_eq!(outer.len(), 2);
    assert_eq!(outer[0].as_vec().unwrap().len(), 2);
    
    // Multi-line flow sequence
    let yaml = r#"[
      item1,
      item2,
      item3
    ]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_vec().unwrap().len(), 3);
}

/// Test flow mapping syntax
#[test]
fn test_flow_mappings() {
    // Basic flow mapping
    let yaml = r#"{key1: value1, key2: value2}"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
    
    // Nested flow mappings
    let yaml = r#"{outer: {inner: value}}"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["outer"]["inner"].as_str().unwrap(), "value");
}