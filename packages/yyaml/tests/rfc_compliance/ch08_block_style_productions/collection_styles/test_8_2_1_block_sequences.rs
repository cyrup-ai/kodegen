//! RFC 8.2.1 Block Sequences Compliance Tests
//!
//! Tests block sequence production rules
//! References: ../../../../../docs/ch08-block-style-productions/collection-styles/

use yyaml::YamlLoader;

/// Test basic block sequence syntax
#[test]
fn test_basic_block_sequences() {
    let yaml = r#"
- item1
- item2
- item3
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str().unwrap(), "item1");
    assert_eq!(seq[1].as_str().unwrap(), "item2");
    assert_eq!(seq[2].as_str().unwrap(), "item3");
}

/// Test nested block sequences
#[test]
fn test_nested_block_sequences() {
    let yaml = r#"
- outer1
- - inner1
  - inner2
- outer3
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str().unwrap(), "outer1");
    assert!(seq[1].as_vec().is_some());
    assert_eq!(seq[1].as_vec().unwrap().len(), 2);
    assert_eq!(seq[2].as_str().unwrap(), "outer3");
}

/// Test block sequence with mappings
#[test]
fn test_block_sequence_with_mappings() {
    let yaml = r#"
- name: item1
  value: 100
- name: item2
  value: 200
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[0]["name"].as_str().unwrap(), "item1");
    assert_eq!(seq[0]["value"].as_i64().unwrap(), 100);
    assert_eq!(seq[1]["name"].as_str().unwrap(), "item2");
    assert_eq!(seq[1]["value"].as_i64().unwrap(), 200);
}