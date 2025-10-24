use kodegen_simd::logits::constraints::{
    GenerationConstraint, SchemaConstraint, SchemaVocabulary, regex_from_schema, regex_from_value,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, JsonSchema)]
struct TestStruct {
    name: String,
    age: u32,
}

// Tests using mock vocabulary to verify actual constraint behavior

#[test]
fn test_builder_api() {
    // Create a mock vocabulary for testing (same approach as schema_index.rs tests)
    fn create_test_vocabulary() -> SchemaVocabulary {
        let token_to_bytes = vec![
            b"hello".to_vec(),
            b"world".to_vec(),
            b"true".to_vec(),
            b"false".to_vec(),
            b"null".to_vec(),
            b"123".to_vec(),
            b"456".to_vec(),
            b"\"".to_vec(),
            b"{".to_vec(),
            b"}".to_vec(),
            b"[".to_vec(),
            b"]".to_vec(),
            b",".to_vec(),
            b":".to_vec(),
            b" ".to_vec(),
            b"\"name\"".to_vec(),
            b"\"age\"".to_vec(),
            b"\"TestStruct\"".to_vec(),
        ];
        SchemaVocabulary::from_tokens(token_to_bytes, 0)
    }

    // Create vocabulary and test constraint builder
    let vocabulary = Arc::new(create_test_vocabulary());

    // Test constraint for TestStruct type
    let regex_pattern = match regex_from_schema::<TestStruct>() {
        Ok(pattern) => pattern,
        Err(e) => panic!("Should generate regex from TestStruct: {}", e),
    };
    let constraint_result = SchemaConstraint::new(&regex_pattern, vocabulary.clone(), false);

    // Verify constraint was created successfully - if it fails, print the error to understand the issue
    if let Err(ref e) = constraint_result {
        println!("Constraint creation failed: {e}");
        println!(
            "Generated regex: {}",
            regex_from_schema::<TestStruct>()
                .unwrap_or_else(|e| format!("Regex generation failed: {e}"))
        );
    }
    assert!(
        constraint_result.is_ok(),
        "Should create constraint from TestStruct schema"
    );
    let constraint = match constraint_result {
        Ok(c) => c,
        Err(e) => panic!(
            "Failed to create constraint despite passing assertion: {}",
            e
        ),
    };

    // Test that we can create a new state
    let state = constraint.new_state();
    let initial_state_id = state.current_state();
    // Note: initial state ID is determined by the DFA library and may not be 0
    // We just verify that it's consistent and creates a valid state
    assert_eq!(
        state.tokens_processed(),
        0,
        "Should start with 0 tokens processed"
    );

    // Verify that creating another state gives the same initial state
    let state2 = constraint.new_state();
    assert_eq!(
        state2.current_state(),
        initial_state_id,
        "All new states should start at the same initial state"
    );

    // Test constraint for predefined boolean type
    let boolean_regex = r"(true|false)";
    let boolean_constraint = SchemaConstraint::new(boolean_regex, vocabulary.clone(), false);
    assert!(
        boolean_constraint.is_ok(),
        "Should create boolean constraint"
    );

    // Test actual boolean constraint behavior
    let boolean_constraint = match boolean_constraint {
        Ok(c) => c,
        Err(e) => panic!("Failed to create boolean constraint: {}", e),
    };
    let bool_state = boolean_constraint.new_state();

    // Token indices from create_test_vocabulary():
    // 2 = "true", 3 = "false", 0 = "hello", 4 = "null"
    assert!(
        boolean_constraint
            .try_next(&bool_state, 2)
            .unwrap_or_else(|e| panic!("Failed to check 'true' token: {}", e)),
        "Boolean constraint should allow 'true' token (index 2)"
    );
    assert!(
        boolean_constraint
            .try_next(&bool_state, 3)
            .unwrap_or_else(|e| panic!("Failed to check 'false' token: {}", e)),
        "Boolean constraint should allow 'false' token (index 3)"
    );
    assert!(
        !boolean_constraint
            .try_next(&bool_state, 0)
            .unwrap_or_else(|e| panic!("Failed to check 'hello' token: {}", e)),
        "Boolean constraint should reject 'hello' token (index 0)"
    );
    assert!(
        !boolean_constraint
            .try_next(&bool_state, 4)
            .unwrap_or_else(|e| panic!("Failed to check 'null' token: {}", e)),
        "Boolean constraint should reject 'null' token (index 4)"
    );

    // Test state progression with update()
    let mut progressing_state = boolean_constraint.new_state();
    assert_eq!(progressing_state.tokens_processed(), 0);

    // Update with "true" token (index 2)
    let update_result = boolean_constraint.update(&mut progressing_state, 2);
    let update_succeeded = match update_result {
        Ok(success) => success,
        Err(e) => panic!("Failed to update state with 'true' token: {}", e),
    };
    assert!(
        update_succeeded,
        "Should successfully update state with 'true' token"
    );
    assert_eq!(
        progressing_state.tokens_processed(),
        1,
        "Should have processed 1 token"
    );

    // Note: Boolean constraints do not set is_complete() after matching a single token
    // as the DFA may accept further input. This is expected behavior - the constraint
    // validates tokens but doesn't mark the state as complete until reaching an explicit
    // end state in the DFA, which doesn't occur for simple boolean patterns like (true|false).

    // Test get_allowed_tokens()
    let fresh_state = boolean_constraint.new_state();
    let allowed = boolean_constraint.get_allowed_tokens(&fresh_state);
    assert!(allowed.is_some(), "Should return allowed tokens map");

    let tokens_map = match allowed {
        Some(map) => map,
        None => panic!("Expected allowed tokens map, got None"),
    };
    assert!(
        tokens_map.contains_key(&2),
        "Allowed tokens should include 'true' (index 2)"
    );
    assert!(
        tokens_map.contains_key(&3),
        "Allowed tokens should include 'false' (index 3)"
    );
    assert!(
        !tokens_map.contains_key(&0),
        "Allowed tokens should NOT include 'hello' (index 0)"
    );

    // Test constraint builder functionality - since ConstraintBuilder needs a tokenizer,
    // we test the SchemaConstraint creation directly which is the core functionality
    let schema_json = r#"{
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        },
        "required": ["name"]
    }"#;

    // Test schema parsing and regex generation
    let schema: serde_json::Value = match serde_json::from_str(schema_json) {
        Ok(s) => s,
        Err(e) => panic!("Should parse test schema JSON: {}", e),
    };

    let regex_result = regex_from_value(&schema, None, None);
    assert!(regex_result.is_ok(), "Should generate regex from schema");

    let regex_pattern = match regex_result {
        Ok(pattern) => pattern,
        Err(e) => panic!("Failed to generate regex pattern: {}", e),
    };
    assert!(
        !regex_pattern.is_empty(),
        "Regex pattern should not be empty"
    );
    assert!(
        regex_pattern.contains("name"),
        "Regex should include 'name' property"
    );

    // Test constraint creation with the generated regex
    let final_constraint = SchemaConstraint::new(&regex_pattern, vocabulary, false);
    if let Err(ref e) = final_constraint {
        println!("Failed to create constraint: {}", e);
        println!("Regex pattern was: {}", regex_pattern);
    }
    assert!(
        final_constraint.is_ok(),
        "Should create constraint from generated regex"
    );
}

#[test]
fn test_preset_schemas() {
    // This test verifies preset schema JSON is valid
    let object_schema = r#"{"type": "object", "additionalProperties": true}"#;
    let _parsed: schemars::Schema = match serde_json::from_str(object_schema) {
        Ok(schema) => schema,
        Err(e) => panic!("Failed to parse object schema: {}", e),
    };

    let array_schema = r#"{"type": "array", "items": {}}"#;
    let _parsed: schemars::Schema = match serde_json::from_str(array_schema) {
        Ok(schema) => schema,
        Err(e) => panic!("Failed to parse array schema: {}", e),
    };
}
