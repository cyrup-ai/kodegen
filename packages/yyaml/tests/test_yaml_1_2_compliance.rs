use yyaml::{Value, YamlLoader, parse_str};

/// Test YAML 1.2 specification compliance for block sequences
/// Based on examples from YAML 1.2 spec section 8.2.1

#[test]
fn test_yaml_1_2_spec_example_8_14_block_sequence() {
    // Example 8.14 from YAML 1.2 specification
    let yaml = r#"
block sequence:
  - one
  - two : three
"#;

    // Test YamlLoader (should work)
    let docs = YamlLoader::load_from_str(yaml).expect("YAML 1.2 spec example should parse");
    assert_eq!(docs.len(), 1);

    // Test serde deserialization
    let result: Value = parse_str(yaml).expect("YAML 1.2 spec example should deserialize");
    println!("YAML 1.2 Example 8.14 result: {:?}", result);
}

#[test]
fn test_yaml_1_2_simple_block_sequence() {
    // Simple block sequence: "- 0" should parse to [0]
    let yaml = "- 0";
    let result: Vec<i32> = parse_str(yaml).expect("Simple sequence should parse per YAML 1.2");
    assert_eq!(result, vec![0]);
    println!("Simple sequence result: {:?}", result);
}

#[test]
fn test_yaml_1_2_multi_item_sequence() {
    // Multi-item sequence per YAML 1.2 grammar
    let yaml = "- 1\n- 2\n- 3";
    let result: Vec<i32> = parse_str(yaml).expect("Multi-item sequence should parse per YAML 1.2");
    assert_eq!(result, vec![1, 2, 3]);
    println!("Multi-item sequence result: {:?}", result);
}

#[test]
fn test_yaml_1_2_bom_with_sequences() {
    // BOM with block sequences per YAML 1.2 BOM specification
    let yaml_with_bom = "\u{feff}- 42";
    let result: Vec<i32> =
        parse_str(yaml_with_bom).expect("BOM + sequence should parse per YAML 1.2");
    assert_eq!(result, vec![42]);
    println!("BOM + sequence result: {:?}", result);
}

#[test]
fn test_yaml_1_2_whitespace_separation() {
    // Test YAML 1.2 requirement: "- " must be separated from node by whitespace
    let valid_cases = vec![
        ("- 123", vec![123]),   // Normal case
        ("-   456", vec![456]), // Multiple spaces
        ("- \t789", vec![789]), // Tab separation
    ];

    for (yaml, expected) in valid_cases {
        let result: Vec<i32> = parse_str(yaml)
            .unwrap_or_else(|_| panic!("Should parse valid whitespace case: {}", yaml));
        assert_eq!(result, expected, "Failed for case: {}", yaml);
        println!("Whitespace case '{}' result: {:?}", yaml, result);
    }
}

#[test]
fn test_yaml_1_2_spec_example_8_15_types() {
    // Example 8.15 from YAML 1.2 specification - Block Sequence Entry Types
    let yaml = r#"
- # Empty
- |
  block node
- - one # Compact
  - two # sequence  
- one: two # Compact mapping
"#;

    // Test with YamlLoader first (should work)
    let docs = YamlLoader::load_from_str(yaml).expect("YAML 1.2 spec example 8.15 should parse");
    assert_eq!(docs.len(), 1);
    println!("YAML 1.2 Example 8.15 parsed successfully with YamlLoader");

    // Test serde deserialization
    let result: Value = parse_str(yaml).expect("YAML 1.2 spec example 8.15 should deserialize");
    println!("YAML 1.2 Example 8.15 result: {:?}", result);

    // Verify structure matches specification - using yyaml Value types
    match &result {
        Value::Sequence(seq) => {
            assert_eq!(seq.len(), 4, "Should have 4 sequence entries per spec");
            println!("Successfully verified YAML 1.2 Example 8.15 structure compliance");
        }
        _ => {
            panic!(
                "Result should be sequence per YAML 1.2 specification, got: {:?}",
                result
            );
        }
    }
}

#[test]
fn test_yaml_1_2_grammar_rule_184_compliance() {
    // Test direct compliance with grammar rule [184]:
    // c-l-block-seq-entry(n) ::= c-sequence-entry [ lookahead ≠ ns-char ] s-l+block-indented(n,BLOCK-IN)

    let test_cases = vec![
        ("- scalar", vec!["scalar".to_string()]),
        ("- 42", vec!["42".to_string()]),
        ("- true", vec!["true".to_string()]),
        ("- null", vec!["null".to_string()]),
    ];

    for (yaml, expected) in test_cases {
        let result: Vec<String> = parse_str(yaml)
            .unwrap_or_else(|_| panic!("Grammar rule [184] compliance failed for: {}", yaml));
        assert_eq!(result, expected, "Grammar rule [184] failed for: {}", yaml);
        println!("Grammar rule [184] case '{}' result: {:?}", yaml, result);
    }
}
