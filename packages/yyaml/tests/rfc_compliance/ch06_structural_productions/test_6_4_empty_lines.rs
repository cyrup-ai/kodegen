//! RFC 6.4 Empty Lines Compliance Tests
//!
//! Tests production rules [70-72] for empty line handling
//! References: ../../../docs/ch06-structural-productions/6.4-empty-lines.md

use yyaml::YamlLoader;

/// Test [70] l-empty(n,c) - empty line production
#[test]
fn test_production_70_empty_lines() {
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

/// Test [71] s-block-line-prefix(n) + b-non-content
#[test]
fn test_production_71_block_empty_lines() {
    let yaml = r#"
block:
  - item1
  
  - item2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["block"].as_vec().unwrap().len(), 2);
}

/// Test [72] s-flow-line-prefix(n) + b-non-content
#[test]
fn test_production_72_flow_empty_lines() {
    let yaml = r#"
flow: [
  item1,
  
  item2
]
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["flow"].as_vec().unwrap().len(), 2);
}