use serde::Deserialize;
use yyaml::{Mapping, Sequence, from_str};

#[derive(Debug, Deserialize, PartialEq)]
struct TestStruct {
    thing: Sequence,
}

#[derive(Debug, Deserialize, PartialEq)]
struct TestMapping {
    thing: Mapping,
}

#[test]
fn test_null_to_sequence() {
    let yaml = "thing:\n"; // This parses as { thing: null }
    let result: TestStruct = from_str(yaml).expect("Failed to deserialize null to sequence");
    assert_eq!(result.thing.len(), 0);
    println!("✅ Successfully deserialized null to empty sequence");
}

#[test]
fn test_null_to_mapping() {
    let yaml = "thing:\n"; // This parses as { thing: null }
    let result: TestMapping = from_str(yaml).expect("Failed to deserialize null to mapping");
    assert_eq!(result.thing.len(), 0);
    println!("✅ Successfully deserialized null to empty mapping");
}

#[test]
fn test_explicit_empty_sequence() {
    let yaml = "thing: []\n";
    let result: TestStruct = from_str(yaml).expect("Failed to deserialize empty array");
    assert_eq!(result.thing.len(), 0);
}

#[test]
fn test_explicit_empty_mapping() {
    let yaml = "thing: {}\n";
    let result: TestMapping = from_str(yaml).expect("Failed to deserialize empty object");
    assert_eq!(result.thing.len(), 0);
}
