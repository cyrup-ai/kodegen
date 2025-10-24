//! RFC 7.5 Flow Nodes Compliance Tests
//!
//! Tests flow node production rules
//! References: ../../../docs/ch07-flow-style-productions/7.5-flow-nodes.md

use yyaml::YamlLoader;

/// Test flow nodes in different contexts
#[test]
fn test_flow_nodes() {
    // Flow sequence as root node
    let yaml = r#"[1, 2, 3]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_vec().unwrap().len(), 3);
    
    // Flow mapping as root node
    let yaml = r#"{key: value}"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key"].as_str().unwrap(), "value");
    
    // Flow scalar as root node
    let yaml = r#""quoted string""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "quoted string");
}

/// Test flow nodes with properties
#[test]
fn test_flow_nodes_with_properties() {
    // Flow node with anchor
    let yaml = r#"
reference: &anchor {key: value}
copy: *anchor
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["reference"]["key"].as_str().unwrap(), "value");
    assert_eq!(docs[0]["copy"]["key"].as_str().unwrap(), "value");
    
    // Flow node with explicit tag
    let yaml = r#"tagged: !!str [1, 2, 3]"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["tagged"].as_str().unwrap(), "[1, 2, 3]");
}