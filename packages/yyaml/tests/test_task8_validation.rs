use yyaml::{Yaml, YamlLoader};

#[test]
fn test_block_scalar_mapping() {
    let yaml_input = r#"key: |
  value"#;

    let docs = YamlLoader::load_from_str(yaml_input).unwrap();
    let doc = &docs[0];

    match doc {
        Yaml::Hash(map) => {
            println!("Hash with {} entries:", map.len());
            for (i, (key, value)) in map.iter().enumerate() {
                println!("  Entry {}: Key={:?}, Value={:?}", i, key, value);
            }

            // Verify it's a single entry with correct key-value pair
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

#[test]
fn test_standalone_block_scalar() {
    let yaml_input = r#"|
  value"#;

    let docs = YamlLoader::load_from_str(yaml_input).unwrap();
    let doc = &docs[0];

    match doc {
        Yaml::String(s) => {
            println!("Standalone block scalar: {:?}", s);
            assert_eq!(s, "value", "Should parse as 'value'");
        }
        other => {
            panic!("Expected String, got: {:?}", other);
        }
    }
}

#[test]
fn test_folded_block_scalar() {
    let yaml_input = r#"key: >
  folded
  value"#;

    let docs = YamlLoader::load_from_str(yaml_input).unwrap();
    let doc = &docs[0];

    match doc {
        Yaml::Hash(map) => {
            assert_eq!(map.len(), 1, "Should have exactly 1 entry");

            let key = Yaml::String("key".to_string());
            let value = map.get(&key).unwrap();

            match value {
                Yaml::String(s) => {
                    println!("Folded block scalar: {:?}", s);
                    assert!(s.contains("folded"), "Should contain 'folded'");
                    assert!(s.contains("value"), "Should contain 'value'");
                }
                other => panic!("Expected String value, got: {:?}", other),
            }
        }
        other => {
            panic!("Expected Hash, got: {:?}", other);
        }
    }
}
