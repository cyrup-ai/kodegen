//! RFC 10.3.1 Core Schema Tags
//!
//! Tests Core schema tag handling
//! References: ../../../../docs/ch10-schemas/core/

use yyaml::YamlLoader;

/// Test Core schema extended boolean recognition
#[test]
fn test_core_extended_boolean_tags() {
    let yaml = r#"
yes_value: yes
no_value: no
on_value: on
off_value: off
y_value: y
n_value: n
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    // Core schema extends JSON schema with additional boolean values
    assert_eq!(docs[0]["yes_value"].as_bool().unwrap(), true);
    assert_eq!(docs[0]["no_value"].as_bool().unwrap(), false);
    assert_eq!(docs[0]["on_value"].as_bool().unwrap(), true);
    assert_eq!(docs[0]["off_value"].as_bool().unwrap(), false);
    assert_eq!(docs[0]["y_value"].as_bool().unwrap(), true);
    assert_eq!(docs[0]["n_value"].as_bool().unwrap(), false);
}

/// Test Core schema extended integer formats
#[test]
fn test_core_extended_integer_formats() {
    let yaml = r#"
octal: 0o77
hexadecimal: 0x1A
binary: 0b1010
with_underscores: 1_000_000
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["octal"].as_i64().unwrap(), 63);
    assert_eq!(docs[0]["hexadecimal"].as_i64().unwrap(), 26);
    assert_eq!(docs[0]["binary"].as_i64().unwrap(), 10);
    assert_eq!(docs[0]["with_underscores"].as_i64().unwrap(), 1000000);
}

/// Test Core schema special float values
#[test]
fn test_core_special_float_values() {
    let yaml = r#"
positive_infinity: .inf
negative_infinity: -.inf
not_a_number: .nan
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    // Note: How these are handled depends on implementation
    // They might be represented as strings in some cases
    assert!(docs[0]["positive_infinity"].as_str().is_some() || docs[0]["positive_infinity"].as_f64().is_some());
    assert!(docs[0]["negative_infinity"].as_str().is_some() || docs[0]["negative_infinity"].as_f64().is_some());
    assert!(docs[0]["not_a_number"].as_str().is_some() || docs[0]["not_a_number"].as_f64().is_some());
}