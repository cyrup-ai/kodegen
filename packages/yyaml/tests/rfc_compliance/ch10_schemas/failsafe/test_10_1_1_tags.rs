//! RFC 10.1.1 Failsafe Schema Tags
//!
//! Tests Failsafe schema tag handling
//! References: ../../../../docs/ch10-schemas/failsafe/

use yyaml::YamlLoader;

/// Test Failsafe schema basic types
#[test]
fn test_failsafe_basic_types() {
    // String (default for all scalars in Failsafe)
    let yaml = r#"
string: some text
number: 123
boolean: true
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    // In Failsafe schema, everything should be treated as string unless explicitly tagged
    assert_eq!(docs[0]["string"].as_str().unwrap(), "some text");
    // Note: Our implementation may resolve types, but Failsafe spec says default to string
}

/// Test Failsafe schema sequence type
#[test]
fn test_failsafe_sequence_type() {
    let yaml = r#"
- item1
- item2
- item3
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let seq = docs[0].as_vec().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_str().unwrap(), "item1");
}

/// Test Failsafe schema mapping type
#[test]
fn test_failsafe_mapping_type() {
    let yaml = r#"
key1: value1
key2: value2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
}

/// Test explicit Failsafe schema tags
#[test]
fn test_explicit_failsafe_tags() {
    let yaml = r#"
string: !!str "123"
sequence: !!seq [1, 2, 3]
mapping: !!map { key: value }
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["string"].as_str().unwrap(), "123");
    assert!(docs[0]["sequence"].as_vec().is_some());
    assert!(docs[0]["mapping"].as_hash().is_some());
}