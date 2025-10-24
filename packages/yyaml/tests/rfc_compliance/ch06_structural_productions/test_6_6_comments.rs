//! RFC 6.6 Comments Compliance Tests
//!
//! Tests production rules [75-79] for comment handling
//! References: ../../../docs/ch06-structural-productions/6.6-comments.md

use yyaml::YamlLoader;

/// Test [75] s-b-comment - comment production
#[test]
fn test_production_75_comments() {
    let yaml = r#"
key: value # This is a comment
# Full line comment
another: value
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key"].as_str().unwrap(), "value");
    assert_eq!(docs[0]["another"].as_str().unwrap(), "value");
}

/// Test [76] s-l-comments - line comments
#[test]
fn test_production_76_line_comments() {
    let yaml = r#"
# Header comment
data:
  # Nested comment
  item: value
# Footer comment
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["data"]["item"].as_str().unwrap(), "value");
}

/// Test [77] s-separate(n,c) with comments
#[test]
fn test_production_77_separation_with_comments() {
    let yaml = r#"
key1: value1 # Comment 1
# Intermediate comment
key2: value2 # Comment 2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
}

/// Test comments in different contexts
#[test]
fn test_comments_in_flow_context() {
    let yaml = r#"
flow: [
  item1, # Comment in flow
  item2  # Another comment
]
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["flow"].as_vec().unwrap().len(), 2);
}