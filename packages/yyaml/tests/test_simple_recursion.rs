use serde::Deserialize;
use yyaml::value::{Tag, TaggedValue};
use yyaml::{Mapping, Number, Value};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SimpleStruct {
    x: usize,
}

#[test]
fn test_simple_tagged_deserialization() {
    println!("Testing simple tagged struct deserialization...");

    // Create a simple tagged value manually
    let tag = Tag::new("wat".to_string());
    let inner_value = Value::Mapping({
        let mut map = Mapping::new();
        map.insert(
            Value::String("x".to_string()),
            Value::Number(Number::from(42)),
        );
        map
    });
    let tagged_value = TaggedValue::new(tag, inner_value);
    let value = Value::Tagged(Box::new(tagged_value));

    println!("Created tagged value: {value:?}");

    println!("Attempting to deserialize...");
    match SimpleStruct::deserialize(&value) {
        Ok(result) => println!("SUCCESS: {result:?}"),
        Err(e) => {
            println!("ERROR: {e}");
            panic!("Deserialization should not fail for simple tagged value");
        }
    }
}
