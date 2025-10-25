// Tests extracted from src/memory/vector/vector_repository.rs

use kodegen_candle_agent::memory::utils::Result;
use kodegen_candle_agent::memory::vector::{DistanceMetric, VectorRepository};

#[tokio::test]
async fn test_vector_repository() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let repo = VectorRepository::new(128);

    // Create collection
    let collection = repo
        .create_collection("test_collection".to_string(), 3, DistanceMetric::Cosine)
        .await?;

    assert_eq!(collection.name, "test_collection");
    assert_eq!(collection.dimensions, 3);

    // Add vector
    let id = uuid::Uuid::new_v4().to_string();
    repo.add_vector("test_collection", id.clone(), vec![1.0, 0.0, 0.0])
        .await?;

    // Search
    let results = repo.search("test_collection", &[1.0, 0.0, 0.0], 1).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, id);

    // Delete collection
    repo.delete_collection("test_collection").await?;

    // Verify deletion
    assert!(repo.get_collection("test_collection").await.is_err());
    Ok(())
}
