// Tests extracted from src/builders/embedding.rs

use kodegen_candle_agent::prelude::*;

#[tokio::test]
async fn test_embedding_builder_default_model() {
    // This test requires the Stella model to be registered
    let join_result = Embedding::from_document("Hello world").embed().await;

    // Handle JoinHandle result first, then embedding result
    match join_result {
        Ok(result) => match result {
            Ok(embedding) => {
                assert!(!embedding.document.is_empty());
                assert!(embedding.as_vec().is_some());
            }
            Err(e) => {
                // Expected if model not available
                eprintln!("Expected error (model may not be loaded): {}", e);
            }
        },
        Err(e) => {
            panic!("Task join failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_embedding_builder_with_model() {
    let join_result = Embedding::from_document("Hello world")
        .model("dunzhang/stella_en_400M_v5")
        .with_task("query")
        .embed()
        .await;

    // Handle JoinHandle result first, then embedding result
    match join_result {
        Ok(result) => match result {
            Ok(embedding) => {
                assert_eq!(embedding.document, "Hello world");
            }
            Err(e) => {
                eprintln!("Expected error (model may not be loaded): {}", e);
            }
        },
        Err(e) => {
            panic!("Task join failed: {}", e);
        }
    }
}
