use yyaml::{Yaml, YamlLoader};

#[test]
fn test_multi_document_with_directives() {
    let input = r#"%YAML 1.2
---
doc1: value
...
%TAG ! tag:example.com,2000:app/
---
doc2: simple_value"#;

    let docs = YamlLoader::load_from_str(input).expect("Should parse multi-document stream");
    assert_eq!(docs.len(), 2, "Expected 2 documents");

    // Validate first document
    if let Yaml::Hash(map) = &docs[0] {
        let key = Yaml::String("doc1".to_string());
        let expected = Yaml::String("value".to_string());
        assert_eq!(map.get(&key), Some(&expected), "First document should have correct content");
    } else {
        panic!("First document should be a mapping");
    }

    // Validate second document
    if let Yaml::Hash(map) = &docs[1] {
        let key = Yaml::String("doc2".to_string());
        let expected = Yaml::String("simple_value".to_string());
        assert_eq!(map.get(&key), Some(&expected), "Second document should have correct content");
    } else {
        panic!("Second document should be a mapping");
    }
}

#[test]
fn test_bom_handling() {
    let bom_input = "\u{feff}%YAML 1.2\n---\nkey: value";
    let docs = YamlLoader::load_from_str(bom_input).expect("Should handle BOM correctly");
    assert_eq!(docs.len(), 1, "Expected 1 document with BOM");
}

#[test]
fn test_multiple_explicit_documents() {
    let multi_explicit = r#"---
first: document
...
---
second: document
---
third: document"#;

    let docs = YamlLoader::load_from_str(multi_explicit).expect("Should parse multiple explicit documents");
    assert_eq!(docs.len(), 3, "Expected 3 documents");
}

#[test]
fn test_empty_stream_handling() {
    let empty_input = "";
    let docs = YamlLoader::load_from_str(empty_input).expect("Should handle empty stream");
    assert_eq!(docs.len(), 1, "Expected 1 null document for empty stream");
}

#[test]
fn test_directive_only_document() {
    let directive_only = r#"%YAML 1.2
%TAG ! tag:example.com,2000:app/
---"#;

    let docs = YamlLoader::load_from_str(directive_only).expect("Should parse directive-only document");
    assert_eq!(docs.len(), 1, "Expected 1 document");
    assert_eq!(docs[0], Yaml::Null, "Expected null document");
}

#[test]
fn test_complex_tagged_nested_structures() {
    let complex_tagged = r#"%TAG ! tag:example.com,2000:app/
---
tagged: !shape
  value: circle
---
simple: scalar"#;

    let docs = YamlLoader::load_from_str(complex_tagged).unwrap();
    println!("Parsed {} documents:", docs.len());
    for (i, doc) in docs.iter().enumerate() {
        println!("Document {}: {:?}", i, doc);
    }
    assert_eq!(docs.len(), 2); // Should pass after fix
}