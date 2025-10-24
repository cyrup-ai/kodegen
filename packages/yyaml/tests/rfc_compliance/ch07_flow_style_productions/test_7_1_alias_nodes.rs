//! RFC 7.1 Alias Nodes Compliance Tests
//!
//! Tests alias node handling in flow context
//! References: ../../../docs/ch07-flow-style-productions/7.1-alias-nodes.md

use yyaml::YamlLoader;

/// Test basic alias node functionality
#[test]
fn test_alias_nodes() {
    let yaml = r#"
anchor: &ref value
alias: *ref
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["anchor"].as_str().unwrap(), "value");
    assert_eq!(docs[0]["alias"].as_str().unwrap(), "value");
}

/// Test alias in flow context
#[test]
fn test_alias_in_flow_context() {
    let yaml = r#"
default: &default { name: test, value: 100 }
items: [*default, { name: override, value: 200 }]
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let items = docs[0]["items"].as_vec().unwrap();
    assert_eq!(items[0]["name"].as_str().unwrap(), "test");
    assert_eq!(items[1]["value"].as_i64().unwrap(), 200);
}

/// Test nested alias references
#[test]
fn test_nested_alias_references() {
    let yaml = r#"
base: &base
  config: &config
    debug: true
    port: 8080

server: { <<: *base, extra: *config }
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["server"]["config"]["debug"].as_bool().unwrap(), true);
}