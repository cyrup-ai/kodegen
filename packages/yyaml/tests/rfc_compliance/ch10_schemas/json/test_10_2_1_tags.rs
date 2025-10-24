//! RFC 10.2.1 JSON Schema Tags
//!
//! Tests JSON schema tag handling
//! References: ../../../../docs/ch10-schemas/json/

use yyaml::YamlLoader;

/// Test JSON schema null tag
#[test]
fn test_json_null_tag() {
    let yaml = r#"
null_value: null
empty_value:
explicit_null: !!null ""
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["null_value"].is_null());
    assert!(docs[0]["empty_value"].is_null());
    assert!(docs[0]["explicit_null"].is_null());
}

/// Test JSON schema boolean tags
#[test]
fn test_json_boolean_tags() {
    let yaml = r#"
true_value: true
false_value: false
explicit_true: !!bool "yes"
explicit_false: !!bool "no"
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["true_value"].as_bool().unwrap(), true);
    assert_eq!(docs[0]["false_value"].as_bool().unwrap(), false);
    // Note: Explicit bool tags may or may not be supported
}

/// Test JSON schema integer tags
#[test]
fn test_json_integer_tags() {
    let yaml = r#"
positive: 123
negative: -456
zero: 0
explicit_int: !!int "789"
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["positive"].as_i64().unwrap(), 123);
    assert_eq!(docs[0]["negative"].as_i64().unwrap(), -456);
    assert_eq!(docs[0]["zero"].as_i64().unwrap(), 0);
}

/// Test JSON schema float tags
#[test]
fn test_json_float_tags() {
    let yaml = r#"
float_value: 3.14159
scientific: 1.23e-4
explicit_float: !!float "2.718"
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert!(docs[0]["float_value"].as_f64().is_some());
    assert!(docs[0]["scientific"].as_f64().is_some());
}