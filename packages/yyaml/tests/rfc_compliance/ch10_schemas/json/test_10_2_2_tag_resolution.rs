//! RFC 10.2.2 JSON Schema Tag Resolution
//!
//! Tests JSON schema tag resolution rules
//! References: ../../../../docs/ch10-schemas/json/

use yyaml::YamlLoader;

/// Test JSON schema null resolution
#[test]
fn test_json_null_resolution() {
    let null_values = vec!["null", "Null", "NULL", "~"];
    
    for value in null_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert!(
            docs[0]["key"].is_null(),
            "JSON schema should resolve '{}' to null",
            value
        );
    }
    
    // Empty value should also resolve to null
    let yaml = "key:";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["key"].is_null());
}

/// Test JSON schema boolean resolution
#[test]
fn test_json_boolean_resolution() {
    let true_values = vec!["true", "True", "TRUE"];
    let false_values = vec!["false", "False", "FALSE"];
    
    for value in true_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_bool().unwrap(), 
            true,
            "JSON schema should resolve '{}' to true", 
            value
        );
    }
    
    for value in false_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_bool().unwrap(), 
            false,
            "JSON schema should resolve '{}' to false", 
            value
        );
    }
}/// Test JSON schema integer resolution
#[test]
fn test_json_integer_resolution() {
    let test_cases = vec![
        ("0", 0i64),
        ("123", 123i64),
        ("-456", -456i64),
        ("+789", 789i64),
    ];
    
    for (input, expected) in test_cases {
        let yaml = format!("key: {}", input);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_i64().unwrap(),
            expected,
            "JSON schema should resolve '{}' to {}",
            input,
            expected
        );
    }
}

/// Test JSON schema float resolution
#[test]
fn test_json_float_resolution() {
    // Regular floats
    let yaml = "key: 3.14159";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!((docs[0]["key"].as_f64().unwrap() - 3.14159).abs() < f64::EPSILON);
    
    // Scientific notation
    let yaml = "key: 1.23e-4";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!((docs[0]["key"].as_f64().unwrap() - 0.000123).abs() < f64::EPSILON);
    
    // Negative floats
    let yaml = "key: -2.718";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!((docs[0]["key"].as_f64().unwrap() + 2.718).abs() < f64::EPSILON);
}

/// Test JSON schema string fallback
#[test]
fn test_json_string_fallback() {
    // Values that don't match other types should be strings
    let string_values = vec![
        "hello",
        "123abc",
        "true_but_not_bool",
        "null_but_not_null",
    ];
    
    for value in string_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(docs[0]["key"].as_str().unwrap(), value);
    }
}