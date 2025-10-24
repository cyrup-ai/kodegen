use yyaml::{Yaml, YamlLoader};

#[test]
fn test_standalone_block_scalar_fix() {
    println!("=== Testing Standalone Block Scalar Fix ===");
    
    let standalone_input = "|\n  value";
    
    println!("Input: {:?}", standalone_input);
    
    let result = YamlLoader::load_from_str(standalone_input);
    
    match result {
        Ok(docs) => {
            println!("SUCCESS: Parsed {} documents", docs.len());
            assert_eq!(docs.len(), 1, "Should have exactly 1 document");
            
            let doc = &docs[0];
            match doc {
                Yaml::String(s) => {
                    println!("Result: String({:?})", s);
                    assert_eq!(s, "value", "Should parse as 'value'");
                }
                other => {
                    panic!("Expected String, got: {:?}", other);
                }
            }
        }
        Err(e) => {
            panic!("Parse failed: {:?}", e);
        }
    }
    
    println!("✅ Standalone block scalar fix verified!");
}

#[test]
fn test_block_scalar_mapping_still_works() {
    println!("=== Testing Block Scalar Mapping Still Works ===");
    
    let mapping_input = "key: |\n  value";
    
    println!("Input: {:?}", mapping_input);
    
    let result = YamlLoader::load_from_str(mapping_input);
    
    match result {
        Ok(docs) => {
            println!("SUCCESS: Parsed {} documents", docs.len());
            assert_eq!(docs.len(), 1, "Should have exactly 1 document");
            
            let doc = &docs[0];
            match doc {
                Yaml::Hash(map) => {
                    println!("Result: Hash with {} entries", map.len());
                    assert_eq!(map.len(), 1, "Should have exactly 1 entry");
                    
                    let key = Yaml::String("key".to_string());
                    let expected_value = Yaml::String("value".to_string());
                    
                    assert_eq!(
                        map.get(&key),
                        Some(&expected_value),
                        "Should have key='value' mapping"
                    );
                }
                other => {
                    panic!("Expected Hash, got: {:?}", other);
                }
            }
        }
        Err(e) => {
            panic!("Parse failed: {:?}", e);
        }
    }
    
    println!("✅ Block scalar mapping still works!");
}