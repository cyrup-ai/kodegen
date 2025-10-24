//! RFC 6.7 Separation Lines Compliance Tests
//!
//! Tests production rules [80-81] for separation line handling
//! References: ../../../docs/ch06-structural-productions/6.7-separation-lines.md

use yyaml::YamlLoader;

/// Test [80] l-directive-end - directive end line
#[test]
fn test_production_80_directive_end() {
    let yaml = r#"%YAML 1.2
---
content: value"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["content"].as_str().unwrap(), "value");
}

/// Test [81] l-document-suffix - document suffix
#[test]
fn test_production_81_document_suffix() {
    let yaml = r#"first: document
...
second: document"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["first"].as_str().unwrap(), "document");
    assert_eq!(docs[1]["second"].as_str().unwrap(), "document");
}

/// Test separation between documents
#[test]
fn test_document_separation() {
    let yaml = r#"
%YAML 1.2
---
doc1: content
...
%YAML 1.2  
---
doc2: content
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["doc1"].as_str().unwrap(), "content");
    assert_eq!(docs[1]["doc2"].as_str().unwrap(), "content");
}