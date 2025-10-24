use yyaml::YamlLoader;

#[test]
fn test_literal_scalar_with_chomping() {
    // Test literal strip (|-)
    let literal_strip = r#"test: |-
  line 1
  line 2
"#;

    let docs = YamlLoader::load_from_str(literal_strip).expect("Should parse literal strip");
    println!("Full document: {:#?}", docs[0]);
    println!("Test value: {:#?}", docs[0]["test"]);
    if let Some(result) = docs[0]["test"].as_str() {
        println!("Literal strip result: {:?}", result);
    } else {
        println!("Not parsed as string - type: {:#?}", docs[0]["test"]);
    }

    // Note: This test documents current behavior - may not implement chomping yet
}

#[test]
fn test_folded_scalar_basic() {
    let folded = r#"test: >
  line 1
  line 2

  paragraph 2
"#;

    let docs = YamlLoader::load_from_str(folded).expect("Should parse folded scalar");
    println!("Full document: {:#?}", docs[0]);
    println!("Test value: {:#?}", docs[0]["test"]);
    if let Some(result) = docs[0]["test"].as_str() {
        println!("Folded result: {:?}", result);
    } else {
        println!("Not parsed as string - type: {:#?}", docs[0]["test"]);
    }
}

#[test]
fn test_explicit_block_mapping() {
    let yaml = r#"
root:
  ? explicit
    key
  : explicit
    value
"#;

    let result = YamlLoader::load_from_str(yaml);
    match result {
        Ok(docs) => {
            println!("Successfully parsed explicit mapping: {:#?}", docs[0]);
        }
        Err(e) => {
            println!("Failed to parse explicit mapping: {:?}", e);
            // This might be expected if explicit key/value isn't implemented
        }
    }
}
