//! RFC 7.2 Empty Nodes Compliance Tests
//!
//! Tests empty node handling in flow context
//! References: ../../../docs/ch07-flow-style-productions/7.2-empty-nodes.md

use yyaml::YamlLoader;

/// Test empty scalar nodes
#[test]
fn test_empty_scalar_nodes() {
    let yaml = r#"
empty_key:
explicit_null: null
empty_string: ""
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["empty_key"].is_null());
    assert!(docs[0]["explicit_null"].is_null());
    assert_eq!(docs[0]["empty_string"].as_str().unwrap(), "");
}

/// Test empty nodes in flow sequences
#[test]
fn test_empty_nodes_in_flow_sequences() {
    let yaml = r#"[item1, , item3]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str().unwrap(), "item1");
    assert!(seq[1].is_null());
    assert_eq!(seq[2].as_str().unwrap(), "item3");
}

/// Test empty nodes in flow mappings
#[test]
fn test_empty_nodes_in_flow_mappings() {
    let yaml = r#"{ key1: value1, key2: , key3: value3 }"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert!(docs[0]["key2"].is_null());
    assert_eq!(docs[0]["key3"].as_str().unwrap(), "value3");
}