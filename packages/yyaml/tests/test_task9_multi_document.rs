use yyaml::parser::loader::YamlLoader;

#[test]
fn test_multi_document_with_yaml_directive() {
    let multi_doc_yaml = r#"%YAML 1.2
---
doc1: first document
---
doc2: second document
..."#;

    let result = YamlLoader::load_from_str(multi_doc_yaml);
    match result {
        Ok(docs) => {
            println!("✅ Successfully parsed {} documents", docs.len());
            assert_eq!(docs.len(), 2, "Should parse exactly 2 documents");
            for (i, doc) in docs.iter().enumerate() {
                println!("  Document {}: {:?}", i + 1, doc);
            }
        }
        Err(e) => {
            panic!("❌ Failed to parse multi-document YAML with directive: {}", e);
        }
    }
}

#[test]
fn test_multi_document_with_tag_directive() {
    let tag_directive_yaml = r#"%TAG ! tag:example.com,2000:app/
---
key: "Tagged document"
---
second: value
..."#;

    let result = YamlLoader::load_from_str(tag_directive_yaml);
    match result {
        Ok(docs) => {
            println!("✅ Successfully parsed {} documents with TAG directive", docs.len());
            assert_eq!(docs.len(), 2, "Should parse exactly 2 documents");
            for (i, doc) in docs.iter().enumerate() {
                println!("  Document {}: {:?}", i + 1, doc);
            }
        }
        Err(e) => {
            panic!("❌ Failed to parse multi-document YAML with TAG directive: {}", e);
        }
    }
}

#[test]
fn test_simple_multi_document_baseline() {
    let simple_multi_doc = r#"---
first: document
---
second: document
..."#;

    let result = YamlLoader::load_from_str(simple_multi_doc);
    match result {
        Ok(docs) => {
            println!("✅ Successfully parsed {} simple multi-documents", docs.len());
            assert_eq!(docs.len(), 2, "Should parse exactly 2 documents");
            for (i, doc) in docs.iter().enumerate() {
                println!("  Document {}: {:?}", i + 1, doc);
            }
        }
        Err(e) => {
            panic!("❌ Failed to parse simple multi-document YAML: {}", e);
        }
    }
}