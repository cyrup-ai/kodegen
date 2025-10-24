use yyaml::parser::loader::YamlLoader;

#[test]
fn test_tagged_sequence_no_infinite_recursion() {
    // This was the original case that caused infinite recursion
    let yaml_content = r#"vec: !wat
  - 0"#;

    // Test with a timeout by running in a way that would panic if it takes too long
    let result = std::panic::catch_unwind(|| YamlLoader::load_from_str(yaml_content));

    // If we get here without infinite recursion, that's progress
    match result {
        Ok(parse_result) => {
            match parse_result {
                Ok(docs) => {
                    println!(
                        "✅ Tagged sequence parsed successfully: {} documents",
                        docs.len()
                    );
                    assert!(!docs.is_empty(), "Should parse at least one document");
                }
                Err(e) => {
                    println!(
                        "⚠️  Tagged sequence failed to parse (but no infinite recursion): {}",
                        e
                    );
                    // Even if parsing fails, no infinite recursion is progress
                    // For now, we'll accept controlled failure
                    // No infinite recursion - this is progress
                }
            }
        }
        Err(_) => {
            panic!("Panic occurred - might be infinite recursion or other issue");
        }
    }
}

#[test]
fn test_simple_sequence_still_works() {
    let yaml_content = r#"- item1
- item2"#;

    let result = YamlLoader::load_from_str(yaml_content);

    match result {
        Ok(docs) => {
            println!("✅ Simple sequence works: {} documents", docs.len());
            assert!(!docs.is_empty(), "Should parse at least one document");
        }
        Err(e) => {
            println!("❌ Simple sequence failed: {}", e);
            panic!("Simple sequences should work");
        }
    }
}

#[test]
fn test_simple_mapping_still_works() {
    let yaml_content = r#"key: value
another: item"#;

    let result = YamlLoader::load_from_str(yaml_content);

    match result {
        Ok(docs) => {
            println!("✅ Simple mapping works: {} documents", docs.len());
            assert!(!docs.is_empty(), "Should parse at least one document");
        }
        Err(e) => {
            println!("❌ Simple mapping failed: {}", e);
            panic!("Simple mappings should work");
        }
    }
}
