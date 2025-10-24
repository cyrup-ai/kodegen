//! RFC 6.9 Node Properties Compliance Tests
//!
//! Tests anchor, alias, and tag properties
//! References: ../../../docs/ch06-structural-productions/6.9-node-properties.md

use yyaml::YamlLoader;

/// Test anchor and alias properties
#[test]
fn test_anchor_and_alias() {
    let yaml = r#"
default: &default
  name: "default"
  value: 100

override:
  <<: *default
  value: 200
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["default"]["name"].as_str().unwrap(), "default");
    assert_eq!(docs[0]["override"]["name"].as_str().unwrap(), "default");
    assert_eq!(docs[0]["override"]["value"].as_i64().unwrap(), 200);
}

/// Test explicit tag properties
#[test]
fn test_explicit_tags() {
    let yaml = r#"
string_as_int: !!str 123
int_as_string: !!int "456"
boolean: !!bool "yes"
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["string_as_int"].as_str().unwrap(), "123");
    assert_eq!(docs[0]["int_as_string"].as_i64().unwrap(), 456);
    assert_eq!(docs[0]["boolean"].as_bool().unwrap(), true);
}

/// Test node property ordering
#[test]
fn test_property_ordering() {
    // Anchor then tag
    let yaml = r#"
item: &anchor !!str "value"
ref: *anchor
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["item"].as_str().unwrap(), "value");
    assert_eq!(docs[0]["ref"].as_str().unwrap(), "value");
}