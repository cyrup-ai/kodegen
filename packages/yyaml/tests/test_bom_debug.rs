use yyaml::YamlLoader;

#[test]
fn test_bom_loader_debug() {
    println!("=== Testing YamlLoader directly ===");

    // Test without BOM
    let yaml_no_bom = "- 0";
    println!("Without BOM: {:?}", yaml_no_bom);
    match YamlLoader::load_from_str(yaml_no_bom) {
        Ok(docs) => {
            println!("  Success: {} documents", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("  Doc {}: {:?}", i, doc);
            }
            assert_eq!(docs.len(), 1);
        }
        Err(e) => {
            println!("  Error: {:?}", e);
            panic!("Expected success for non-BOM case");
        }
    }

    // Test with BOM
    let yaml_with_bom = "\u{feff}- 0";
    println!("With BOM: {:?}", yaml_with_bom);
    match YamlLoader::load_from_str(yaml_with_bom) {
        Ok(docs) => {
            println!("  Success: {} documents", docs.len());
            for (i, doc) in docs.iter().enumerate() {
                println!("  Doc {}: {:?}", i, doc);
            }
            assert_eq!(docs.len(), 1);
        }
        Err(e) => {
            println!("  Error: {:?}", e);
            panic!("BOM case should succeed after fix: {:?}", e);
        }
    }
}

#[test]
fn test_bom_serde_debug() {
    println!("=== Testing serde deserialization ===");

    // Test without BOM
    let yaml_no_bom = "- 0";
    println!("Without BOM serde test: {:?}", yaml_no_bom);
    match yyaml::parse_str::<Vec<i32>>(yaml_no_bom) {
        Ok(result) => {
            println!("  Success: {:?}", result);
            assert_eq!(result, vec![0]);
        }
        Err(e) => {
            println!("  Error: {:?}", e);
            panic!("Expected success for non-BOM serde case");
        }
    }

    // Test with BOM
    let yaml_with_bom = "\u{feff}- 0";
    println!("With BOM serde test: {:?}", yaml_with_bom);

    // First check what YamlLoader produces
    match YamlLoader::load_from_str(yaml_with_bom) {
        Ok(docs) => {
            println!("  YamlLoader produced: {} docs", docs.len());
            if !docs.is_empty() {
                println!("  First doc: {:?}", docs[0]);
            }
        }
        Err(e) => println!("  YamlLoader error: {:?}", e),
    }

    // Now test serde deserialization
    match yyaml::parse_str::<Vec<i32>>(yaml_with_bom) {
        Ok(result) => {
            println!("  Success: {:?}", result);
            assert_eq!(result, vec![0]);
        }
        Err(e) => {
            println!("  Serde Error: {:?}", e);
            panic!("BOM serde case should succeed after fix: {:?}", e);
        }
    }
}
