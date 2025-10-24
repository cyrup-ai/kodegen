//! RFC 6.2 Separation Spaces Compliance Tests
//!
//! Tests production rules [66] for separation space handling
//! References: ../../../docs/ch06-structural-productions/6.2-separation-spaces.md

use yyaml::YamlLoader;

/// Test [66] s-separate(n,c) - separation spaces production
#[test]
fn test_production_66_separation_spaces() {
    // Block context: requires line break
    let yaml = r#"
key1: value1
key2: value2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
    
    // Flow context: allows spaces
    let yaml = r#"{ key1: value1, key2: value2 }"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key1"].as_str().unwrap(), "value1");
    assert_eq!(docs[0]["key2"].as_str().unwrap(), "value2");
}

/// Test separation in different contexts
#[test]
fn test_context_dependent_separation() {
    // BLOCK-OUT context
    let yaml = r#"
- item1
- item2
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_vec().unwrap().len(), 2);
    
    // FLOW-IN context
    let yaml = r#"[item1, item2]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_vec().unwrap().len(), 2);
}