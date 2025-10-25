// Tests extracted from src/util/json_util.rs

use serde::{Deserialize, Serialize};
use kodegen_candle_agent::util::json_util::*;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Dummy {
    #[serde(with = "stringified_json")]
    data: serde_json::Value,
}

// ----- merge -----------------------------------------------------------
#[test]
fn merge_by_value() {
    let a = serde_json::json!({"k1":"v1"});
    let b = serde_json::json!({"k2":"v2"});
    assert_eq!(merge(a, b), serde_json::json!({"k1":"v1","k2":"v2"}));
}

#[test]
fn merge_in_place() {
    let mut a = serde_json::json!({"k1":"v1"});
    merge_inplace(&mut a, serde_json::json!({"k2":"v2"}));
    assert_eq!(a, serde_json::json!({"k1":"v1","k2":"v2"}));
}

// ----- stringified JSON -----------------------------------------------
#[test]
fn stringified_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let original = Dummy {
        data: serde_json::json!({"k":"v"}),
    };
    let s = serde_json::to_string(&original)?;
    assert_eq!(s, r#"{"data":"{\"k\":\"v\"}"}"#);
    let parsed: Dummy = serde_json::from_str(&s)?;
    assert_eq!(parsed, original);
    Ok(())
}

// ----- string_or_vec ---------------------------------------------------
#[test]
fn str_or_array_deserialise() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Wrapper {
        #[serde(deserialize_with = "string_or_vec")]
        v: Vec<u32>,
    }

    let w1: Wrapper = serde_json::from_str(r#"{"v":"3"}"#)?;
    assert_eq!(w1.v, vec![3]);

    let w2: Wrapper = serde_json::from_str(r#"{"v":[1,2,3]}"#)?;
    assert_eq!(w2.v, vec![1, 2, 3]);

    let w3: Wrapper = serde_json::from_str(r#"{"v":null}"#)?;
    assert!(w3.v.is_empty());
    Ok(())
}

// ----- null_or_vec -----------------------------------------------------
#[test]
fn null_or_array_deserialise() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Wrapper {
        #[serde(deserialize_with = "null_or_vec")]
        v: Vec<bool>,
    }

    let w1: Wrapper = serde_json::from_str(r#"{"v":[true,false]}"#)?;
    assert_eq!(w1.v, vec![true, false]);

    let w2: Wrapper = serde_json::from_str(r#"{"v":null}"#)?;
    assert!(w2.v.is_empty());
    Ok(())
}

// ----- utility functions -----------------------------------------------
#[test]
fn test_ensure_object_and_merge() {
    let mut target = serde_json::json!("not an object");
    let source = serde_json::json!({"key": "value"});

    ensure_object_and_merge(&mut target, source);
    assert_eq!(target, serde_json::json!({"key": "value"}));
}

#[test]
fn test_is_empty_value() {
    assert!(is_empty_value(&serde_json::json!(null)));
    assert!(is_empty_value(&serde_json::json!({})));
    assert!(is_empty_value(&serde_json::json!([])));
    assert!(is_empty_value(&serde_json::json!("")));
    assert!(!is_empty_value(&serde_json::json!({"key": "value"})));
    assert!(!is_empty_value(&serde_json::json!([1, 2, 3])));
    assert!(!is_empty_value(&serde_json::json!("content")));
}

#[test]
fn test_merge_multiple() {
    let values = vec![
        serde_json::json!({"a": 1}),
        serde_json::json!({"b": 2}),
        serde_json::json!({"c": 3}),
    ];

    let result = merge_multiple(values);
    assert_eq!(result, serde_json::json!({"a": 1, "b": 2, "c": 3}));
}
