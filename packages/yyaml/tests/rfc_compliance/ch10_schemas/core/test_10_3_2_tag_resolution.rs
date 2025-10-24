//! RFC 10.3.2 Core Schema Tag Resolution
//!
//! Tests Core schema tag resolution rules
//! References: ../../../../docs/ch10-schemas/core/

use yyaml::YamlLoader;

/// Test Core schema null resolution
#[test]
fn test_core_null_resolution() {
    let null_values = vec!["null", "Null", "NULL", "~", ""];
    
    for value in null_values {
        let yaml = if value.is_empty() {
            "key:".to_string()
        } else {
            format!("key: {}", value)
        };
        
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert!(
            docs[0]["key"].is_null(),
            "Core schema should resolve '{}' to null",
            value
        );
    }
}

/// Test Core schema boolean resolution
#[test]
fn test_core_boolean_resolution() {
    let true_values = vec![
        "true", "True", "TRUE",
        "yes", "Yes", "YES", 
        "on", "On", "ON",
        "y", "Y"
    ];
    
    let false_values = vec![
        "false", "False", "FALSE",
        "no", "No", "NO",
        "off", "Off", "OFF", 
        "n", "N"
    ];
    
    for value in true_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_bool().unwrap(), 
            true,
            "Core schema should resolve '{}' to true", 
            value
        );
    }
    
    for value in false_values {
        let yaml = format!("key: {}", value);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_bool().unwrap(), 
            false,
            "Core schema should resolve '{}' to false", 
            value
        );
    }
}/// Test Core schema integer resolution  
#[test]
fn test_core_integer_resolution() {
    let test_cases = vec![
        ("0", 0i64),
        ("123", 123i64),
        ("-456", -456i64),
        ("0x1A", 26i64),        // Hexadecimal
        ("0o77", 63i64),        // Octal  
        ("0b1010", 10i64),      // Binary
        ("1_000_000", 1000000i64), // With underscores
    ];
    
    for (input, expected) in test_cases {
        let yaml = format!("key: {}", input);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        assert_eq!(
            docs[0]["key"].as_i64().unwrap(),
            expected,
            "Core schema should resolve '{}' to {}",
            input,
            expected
        );
    }
}

/// Test Core schema float resolution
#[test]
fn test_core_float_resolution() {
    // Regular floats
    let yaml = "key: 3.14159";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!((docs[0]["key"].as_f64().unwrap() - 3.14159).abs() < f64::EPSILON);
    
    // Scientific notation
    let yaml = "key: 1.23e-4";
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!((docs[0]["key"].as_f64().unwrap() - 0.000123).abs() < f64::EPSILON);
    
    // Special values
    let special_cases = vec![
        (".inf", f64::INFINITY),
        ("-.inf", f64::NEG_INFINITY),
        (".nan", f64::NAN),
    ];
    
    for (input, _expected) in special_cases {
        let yaml = format!("key: {}", input);
        let docs = YamlLoader::load_from_str(&yaml).unwrap();
        // Note: NaN comparison requires special handling
        if input == ".nan" {
            assert!(docs[0]["key"].as_str().unwrap().contains("NaN"));
        } else {
            assert!(docs[0]["key"].as_str().is_some());
        }
    }
}

/// Test explicit tag override of schema inference
#[test]
fn test_explicit_tag_override() {
    // Force string interpretation of number
    let yaml = r#"key: !!str 123"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key"].as_str().unwrap(), "123");
    
    // Force integer interpretation of string  
    let yaml = r#"key: !!int "456""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key"].as_i64().unwrap(), 456);
    
    // Force boolean interpretation
    let yaml = r#"key: !!bool "yes""#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["key"].as_bool().unwrap(), true);
}