//! RFC 9.1 Documents Compliance Tests
//!
//! Tests production rules [200-210] for document handling
//! References: ../../../docs/ch09-document-stream-productions/9.1-documents.md

use yyaml::YamlLoader;

/// Test [202] l-directive-document - document with directives
#[test]
fn test_directive_document() {
    let yaml = r#"%YAML 1.2
%TAG ! tag:example.com,2000:app/
---
content: value"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["content"].as_str().unwrap(), "value");
}

/// Test [203] l-explicit-document - document with explicit start
#[test]
fn test_explicit_document() {
    let yaml = r#"---
explicit: document"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["explicit"].as_str().unwrap(), "document");
}

/// Test [204] l-bare-document - document without markers
#[test]
fn test_bare_document() {
    let yaml = r#"bare: document"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["bare"].as_str().unwrap(), "document");
}

/// Test document start markers
#[test]
fn test_document_start_markers() {
    let yaml = r#"---
first: document
---
second: document"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["first"].as_str().unwrap(), "document");
    assert_eq!(docs[1]["second"].as_str().unwrap(), "document");
}