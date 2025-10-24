//! RFC 8.3 Block Nodes Compliance Tests
//!
//! Tests block node production rules
//! References: ../../../docs/ch08-block-style-productions/8.3-block-nodes.md

use yyaml::YamlLoader;

/// Test block nodes in different contexts
#[test]
fn test_block_nodes() {
    // Block scalar as root node
    let yaml = r#"|
  This is a literal
  block scalar
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0].as_str().unwrap().contains("This is a literal"));
    
    // Block sequence as root node
    let yaml = r#"
- item1
- item2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_vec().unwrap().len(), 2);
    
    // Block mapping as root node
    let yaml = r#"
key1: value1
key2: value2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
}

/// Test block nodes with properties
#[test]
fn test_block_nodes_with_properties() {
    // Block node with anchor
    let yaml = r#"
reference: &anchor
  key: value
copy: *anchor
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["reference"]["key"].as_str().unwrap(), "value");
    assert_eq!(docs[0]["copy"]["key"].as_str().unwrap(), "value");
    
    // Block node with explicit tag
    let yaml = r#"tagged: !!str
- item1
- item2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["tagged"].as_str().unwrap().contains("item1"));
}