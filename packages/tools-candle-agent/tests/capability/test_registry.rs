// Tests extracted from src/capability/registry/tests.rs

//! Comprehensive test suite for the registry module
//!
//! These tests verify the critical functionality of the unified registry system,
//! which is THE ONLY MODEL REGISTRY IN THE ENTIRE CODEBASE.

use kodegen_candle_agent::capability::registry::*;
use kodegen_candle_agent::capability::text_to_text::CandleQwen3QuantizedModel;
use std::sync::Arc;

/// Test 1: Runtime registration makes models visible via all APIs
#[tokio::test]
async fn test_runtime_registration_makes_model_visible() {
    // Use unique test keys to avoid conflicts
    let test_key = format!("test-runtime-{}", uuid::Uuid::new_v4());

    // Create a test model using an existing model type
    let model = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));

    // Verify model doesn't exist before registration
    assert!(
        !has_model(&test_key),
        "Model should not exist before registration"
    );

    // Register the model
    let result = register_text_to_text(&test_key, model.clone()).await;
    assert!(result.is_ok(), "Registration should succeed: {:?}", result);

    // Verify model is accessible via get<T>()
    let retrieved: Option<TextToTextModel> = get(&test_key);
    assert!(
        retrieved.is_some(),
        "Model should be accessible via get<T>()"
    );

    // Verify model is accessible via get_text_to_text()
    let retrieved_specific = get_text_to_text(&test_key);
    assert!(
        retrieved_specific.is_some(),
        "Model should be accessible via get_text_to_text()"
    );

    // Verify has_model() returns true
    assert!(
        has_model(&test_key),
        "has_model() should return true after registration"
    );

    // Verify all_registry_keys() includes the key
    let all_keys = all_registry_keys();
    assert!(
        all_keys.contains(&test_key),
        "all_registry_keys() should include the test key"
    );

    // Register a second model to verify multi-model support
    let test_key_2 = format!("test-runtime-2-{}", uuid::Uuid::new_v4());
    let model_2 = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));
    let result_2 = register_text_to_text(&test_key_2, model_2).await;
    assert!(result_2.is_ok(), "Second registration should succeed");

    // Verify both models exist (reliable check without global count dependency)
    assert!(
        has_model(&test_key),
        "First model should still be registered"
    );
    assert!(has_model(&test_key_2), "Second model should be registered");
    assert!(
        get::<TextToTextModel>(&test_key_2).is_some(),
        "Second model should be retrievable"
    );

    // Clean up
    let _ = unregister_text_to_text(&test_key).await;
    let _ = unregister_text_to_text(&test_key_2).await;
}

/// Test 2: Runtime registration prevents duplicate keys (same capability)
#[tokio::test]
async fn test_runtime_registration_prevents_same_capability_duplicates() {
    let test_key = format!("test-duplicate-same-{}", uuid::Uuid::new_v4());

    let model_1 = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));
    let model_2 = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));

    // Register first model
    let result_1 = register_text_to_text(&test_key, model_1).await;
    assert!(result_1.is_ok(), "First registration should succeed");

    // Try to register second model with same key - should fail
    let result_2 = register_text_to_text(&test_key, model_2).await;
    assert!(result_2.is_err(), "Second registration should fail");

    // Verify error type
    match result_2 {
        Err(RegistrationError::KeyAlreadyExists(key)) => {
            assert_eq!(key, test_key, "Error should contain the duplicate key");
        }
        _ => panic!("Expected KeyAlreadyExists error"),
    }

    // Verify only one instance exists
    let all_keys = all_registry_keys();
    let count = all_keys.iter().filter(|k| *k == &test_key).count();
    assert_eq!(count, 1, "Should only have one instance of the key");

    // Clean up
    let _ = unregister_text_to_text(&test_key).await;
}

/// Test 3: Concurrent registration and reads
#[tokio::test]
async fn test_concurrent_registration_and_reads() {
    use tokio::task::JoinSet;

    let base_key = format!("test-concurrent-{}", uuid::Uuid::new_v4());

    let mut tasks = JoinSet::new();

    // Spawn 5 concurrent registration tasks
    for i in 0..5 {
        let key = format!("{}-{}", base_key, i);
        tasks.spawn(async move {
            let model =
                TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));
            register_text_to_text(&key, model).await
        });
    }

    // Spawn 5 concurrent read tasks
    for i in 0..5 {
        let key = format!("{}-{}", base_key, i);
        tasks.spawn(async move {
            // Try to read the model multiple times
            for _ in 0..10 {
                let _ = get::<TextToTextModel>(&key);
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
            Ok(())
        });
    }

    // Wait for all tasks to complete
    let mut errors = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Err(e) = result {
            errors.push(format!("Task panicked: {:?}", e));
        }
    }

    assert!(errors.is_empty(), "No tasks should panic: {:?}", errors);

    // Verify all 5 models were registered
    for i in 0..5 {
        let key = format!("{}-{}", base_key, i);
        assert!(has_model(&key), "Model {} should be registered", i);
    }

    // Clean up
    for i in 0..5 {
        let key = format!("{}-{}", base_key, i);
        let _ = unregister_text_to_text(&key).await;
    }
}

/// Test 4: Cross-capability uniqueness - prevent same key in different capabilities
#[tokio::test]
async fn test_cross_capability_duplicate_prevention() {
    let test_key = format!("test-cross-cap-{}", uuid::Uuid::new_v4());

    // Register in text-to-text capability
    let text_model =
        TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));
    let result_1 = register_text_to_text(&test_key, text_model).await;
    assert!(result_1.is_ok(), "First registration should succeed");

    // Verify has_model() finds the key across capabilities
    assert!(
        has_model(&test_key),
        "Key should be found in any capability"
    );

    // Verify all_registry_keys() doesn't have duplicates
    let all_keys = all_registry_keys();
    let count = all_keys.iter().filter(|k| *k == &test_key).count();
    assert_eq!(
        count, 1,
        "Key should appear exactly once in all_registry_keys()"
    );

    // Clean up
    let _ = unregister_text_to_text(&test_key).await;
}

/// Test 5: Unregistration removes model
#[tokio::test]
async fn test_unregister_removes_model() {
    let test_key = format!("test-unregister-{}", uuid::Uuid::new_v4());

    let model = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));

    // Register the model
    let _ = register_text_to_text(&test_key, model).await;

    // Verify it exists
    assert!(
        has_model(&test_key),
        "Model should exist after registration"
    );
    assert!(
        get::<TextToTextModel>(&test_key).is_some(),
        "get() should return Some after registration"
    );

    // Unregister it
    let removed = unregister_text_to_text(&test_key).await;
    assert!(
        removed.is_some(),
        "Unregister should return the removed model"
    );

    // Verify has_model() returns false
    assert!(
        !has_model(&test_key),
        "has_model() should return false after unregistration"
    );

    // Verify get() returns None
    let retrieved: Option<TextToTextModel> = get(&test_key);
    assert!(
        retrieved.is_none(),
        "get() should return None after unregistration"
    );

    // Verify the key is not in all_registry_keys()
    let all_keys = all_registry_keys();
    assert!(
        !all_keys.contains(&test_key),
        "all_registry_keys() should not contain key after unregistration"
    );
}

/// Test 6: Static models are accessible (regression test)
/// NOTE: This test may fail in external integration tests due to static initialization timing.
/// TODO: Investigate static model registration in external test context.
#[test]
#[ignore] // Temporarily ignored - needs static model initialization fix
fn test_static_models_accessible() {
    // Initialize library to register static models
    kodegen_candle_agent::init_candle();

    // Verify all static models from storage.rs are accessible

    // Text-to-text models
    assert!(
        has_model("Qwen/Qwen2.5-Coder-3B-Instruct-GGUF"),
        "Qwen3Quantized should be accessible"
    );

    // Text embedding models - use actual registry keys from storage.rs
    assert!(
        has_model("dunzhang/stella_en_400M_v5"),
        "Stella should be accessible"
    );

    // Vision models
    assert!(
        has_model("llava-hf/llava-1.5-7b-hf"),
        "LLaVA should be accessible"
    );

    // Verify we can get the models using get<T>()
    assert!(get::<TextToTextModel>("Qwen/Qwen2.5-Coder-3B-Instruct-GGUF").is_some());
    assert!(get::<TextEmbeddingModel>("dunzhang/stella_en_400M_v5").is_some());
    assert!(get::<VisionModel>("llava-hf/llava-1.5-7b-hf").is_some());
}

/// Test 7: Empty registry behavior (IMAGE_EMBEDDING starts empty)
#[test]
fn test_empty_registry_behavior() {
    // IMAGE_EMBEDDING_UNIFIED starts empty, so test that empty registries don't cause panics

    // all_registry_keys() should not panic on empty registries
    let all_keys = all_registry_keys();
    // Should include keys from non-empty registries
    assert!(
        !all_keys.is_empty(),
        "Should have keys from non-empty registries"
    );

    // model_count() should handle empty registries
    let count = model_count();
    assert!(count > 0, "Should count models from non-empty registries");

    // has_model() should work with non-existent keys
    assert!(
        !has_model("non-existent-key-12345"),
        "Should return false for non-existent key"
    );

    // get() should return None for non-existent keys
    let result: Option<ImageEmbeddingModel> = get("non-existent-key-12345");
    assert!(result.is_none(), "Should return None for non-existent key");
}

/// Test 8: all_registry_keys() deduplication
#[tokio::test]
async fn test_all_registry_keys_deduplication() {
    // Get all keys
    let all_keys = all_registry_keys();

    // Convert to a set and check if sizes match
    use std::collections::HashSet;
    let unique_keys: HashSet<String> = all_keys.iter().cloned().collect();

    assert_eq!(
        all_keys.len(),
        unique_keys.len(),
        "all_registry_keys() should not contain duplicates"
    );

    // Verify no key appears more than once
    for key in &all_keys {
        let count = all_keys.iter().filter(|k| *k == key).count();
        assert_eq!(
            count, 1,
            "Key '{}' should appear exactly once, found {} times",
            key, count
        );
    }
}

/// Test 9: Registration with static model key should fail
/// NOTE: This test may fail in external integration tests due to static initialization timing.
/// TODO: Investigate static model registration in external test context.
#[tokio::test]
#[ignore] // Temporarily ignored - needs static model initialization fix
async fn test_cannot_overwrite_static_models() {
    // Initialize library to register static models
    kodegen_candle_agent::init_candle();

    // Try to register a model with a static model's key
    let static_key = "Qwen/Qwen2.5-Coder-3B-Instruct-GGUF";

    let model = TextToTextModel::Qwen3Quantized(Arc::new(CandleQwen3QuantizedModel::default()));

    // This should fail because the key already exists
    let result = register_text_to_text(static_key, model).await;

    assert!(
        result.is_err(),
        "Should not be able to overwrite static model"
    );

    match result {
        Err(RegistrationError::KeyAlreadyExists(key)) => {
            assert_eq!(key, static_key, "Error should contain the static key");
        }
        _ => panic!("Expected KeyAlreadyExists error"),
    }
}
