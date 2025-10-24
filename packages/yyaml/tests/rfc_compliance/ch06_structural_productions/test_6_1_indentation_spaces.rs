//! RFC 6.1 Indentation Spaces Compliance Tests
//!
//! Tests production rules [63-65] for indentation handling
//! References: ../../../docs/ch06-structural-productions/6.1-indentation-spaces.md

use yyaml::YamlLoader;

/// Test [63] s-indent(n) ::= s-space × n
#[test]
fn test_production_63_exact_indentation() {
    // Valid: exact indentation matches expected
    let yaml = r#"
root:
  child1: value1
  child2: value2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["root"]["child1"].as_str().is_some());
    
    // Invalid: mixed tab/space indentation MUST be rejected
    let invalid_yaml = "root:\n\tchild: value"; // Tab not allowed
    assert!(YamlLoader::load_from_str(invalid_yaml).is_err());
}

/// Test [64] s-indent(<n) - less than n spaces
#[test]
fn test_production_64_less_indentation() {
    let yaml = r#"
root:
  child1: value1
 # This line has less indentation - should end block
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["root"]["child1"].as_str().unwrap(), "value1");
}

/// Test [65] s-indent(≤n) - less than or equal to n spaces  
#[test]
fn test_production_65_less_equal_indentation() {
    let yaml = r#"
level1:
  level2:
    level3: value
  back_to_level2: value
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["level1"]["level2"]["level3"].as_str().is_some());
    assert!(docs[0]["level1"]["back_to_level2"].as_str().is_some());
}

/// Test Example 6.1 from specification
#[test]
fn test_spec_example_6_1_indentation_spaces() {
    let yaml = r#"
# Leading comment line spaces are
# neither content nor indentation.

Not indented:
 By one space: |
    By four
      spaces
 Flow style: [    # Leading spaces
   By two,        # in flow style
  Also by two,    # are neither
  →Still by two   # content nor
    ]             # indentation.
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    // Verify structure matches specification expectation
    assert!(docs[0]["Not indented"].as_hash().is_some());
}