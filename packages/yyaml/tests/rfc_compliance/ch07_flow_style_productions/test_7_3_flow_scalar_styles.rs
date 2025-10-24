//! RFC 7.3 Flow Scalar Styles Compliance Tests
//!
//! Tests production rules [107-135] for flow scalar parsing
//! References: ../../../docs/ch07-flow-style-productions/7.3-flow-scalar-styles.md

use yyaml::YamlLoader;

/// Test [107-116] Double-quoted flow scalars
#[test]
fn test_double_quoted_flow_scalars() {
    // Basic double-quoted scalar
    let yaml = r#""Hello, World!""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "Hello, World!");
    
    // Multi-line double-quoted with folding
    let yaml = r#""First line
Second line""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "First line Second line");
    
    // With escape sequences
    let yaml = r#""Tab:\t Newline:\n Quote:\" Backslash:\\""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let result = docs[0].as_str().unwrap();
    assert!(result.contains('\t'));
    assert!(result.contains('\n'));
    assert!(result.contains('"'));
    assert!(result.contains('\\'));
}

/// Test [117-125] Single-quoted flow scalars
#[test]
fn test_single_quoted_flow_scalars() {
    // Basic single-quoted scalar
    let yaml = r#"'Hello, World!'"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "Hello, World!");
    
    // Quote doubling for literal quote
    let yaml = r#"'It''s quoted'"#;    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "It's quoted");
    
    // No escape sequences in single quotes
    let yaml = r#"'Literal \n and \t'"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "Literal \\n and \\t");
}

/// Test [126-135] Plain flow scalars with context restrictions
#[test]
fn test_plain_flow_scalars() {
    // Valid plain scalars
    let valid_cases = vec![
        "simple",
        "with spaces",
        "123",
        "true",
        "null",
        "with:colon",
        "with#hash",
    ];
    
    for case in valid_cases {
        let docs = YamlLoader::load_from_str(case).unwrap();
        assert_eq!(docs[0].as_str().unwrap(), case);
    }
    
    // Invalid in flow context - these should be parsed differently
    let flow_yaml = r#"[key: value]"#; // : followed by space in flow context
    let docs = YamlLoader::load_from_str(flow_yaml).unwrap();
    // Should parse as mapping inside sequence, not as plain scalar
    assert!(docs[0].as_vec().unwrap()[0].as_hash().is_some());
}

/// Test context-dependent plain scalar restrictions
#[test]
fn test_plain_scalar_context_restrictions() {
    // In FLOW-KEY context: stricter restrictions
    let yaml = r#"{simple_key: value}"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["simple_key"].as_str().unwrap(), "value");
    
    // Flow indicators not allowed to start plain scalars in flow context  
    let invalid_flow = r#"[[], {}]"#; // Should parse as nested collections
    let docs = YamlLoader::load_from_str(invalid_flow).unwrap();
    let arr = docs[0].as_vec().unwrap();
    assert!(arr[0].as_vec().unwrap().is_empty());
    assert!(arr[1].as_hash().unwrap().is_empty());
}