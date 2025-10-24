//! RFC 9.2 Streams Compliance Tests
//!
//! Tests production rule [211] l-yaml-stream
//! References: ../../../docs/ch09-document-stream-productions/9.2-streams.md

use yyaml::YamlLoader;

/// Test [211] l-yaml-stream production
#[test]
fn test_yaml_stream_production() {
    // Multiple documents with explicit markers
    let yaml = r#"---
first: document
---
second: document
...
%YAML 1.2
---
third: document
"#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0]["first"].as_str().unwrap(), "document");
    assert_eq!(docs[1]["second"].as_str().unwrap(), "document");
    assert_eq!(docs[2]["third"].as_str().unwrap(), "document");
}

/// Test stream with various document separators
#[test]
fn test_document_separators() {
    // Document end followed by document start
    let yaml = r#"first
...
---
second"#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].as_str().unwrap(), "first");
    assert_eq!(docs[1].as_str().unwrap(), "second");
}

/// Test Example 9.6 from specification
#[test]
fn test_spec_example_9_6_stream() {
    let yaml = r#"Document
---
# Empty
...
%YAML 1.2
---
matches %: 20"#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].as_str().unwrap(), "Document");
    assert!(docs[1].is_null()); // Empty document
    assert_eq!(docs[2]["matches %"].as_i64().unwrap(), 20);
}/// Test well-formed stream requirement
#[test]
fn test_well_formed_stream_compliance() {
    // Empty stream
    let docs = YamlLoader::load_from_str("").unwrap();
    assert_eq!(docs.len(), 1);
    assert!(docs[0].is_null());
    
    // Stream with only comments
    let docs = YamlLoader::load_from_str("# Just a comment").unwrap();
    assert_eq!(docs.len(), 1);
    assert!(docs[0].is_null());
    
    // Stream with BOM
    let docs = YamlLoader::load_from_str("\u{FEFF}content").unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].as_str().unwrap(), "content");
}