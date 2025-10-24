//! RFC 6.3 Line Prefixes Compliance Tests
//!
//! Tests production rules [67-69] for line prefix handling
//! References: ../../../docs/ch06-structural-productions/6.3-line-prefixes.md

use yyaml::YamlLoader;

/// Test [67] s-line-prefix(n,c) - line prefix production
#[test]
fn test_production_67_line_prefix() {
    // Block context: indentation only
    let yaml = r#"
root:
  child:
    nested: value
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["root"]["child"]["nested"].as_str().unwrap(), "value");
}

/// Test [68] s-block-line-prefix(n) - block line prefix
#[test]
fn test_production_68_block_line_prefix() {
    let yaml = r#"
sequence:
  - first: item
  - second: item
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["sequence"].as_vec().unwrap().len(), 2);
}

/// Test [69] s-flow-line-prefix(n) - flow line prefix
#[test]
fn test_production_69_flow_line_prefix() {
    let yaml = r#"
flow: [
  item1,
  item2
]
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["flow"].as_vec().unwrap().len(), 2);
}