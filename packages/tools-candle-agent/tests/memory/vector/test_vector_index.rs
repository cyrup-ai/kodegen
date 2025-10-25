// Tests extracted from src/memory/vector/vector_index.rs

use kodegen_candle_agent::memory::vector::vector_index::*;
use kodegen_candle_agent::memory::vector::DistanceMetric;
use std::collections::HashMap;

#[test]
fn test_hnsw_index() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut config = VectorIndexConfig {
        metric: DistanceMetric::Cosine,
        dimensions: 4,
        index_type: IndexType::HNSW,
        parameters: HashMap::new(),
    };

    config.parameters.insert(
        "m".to_string(),
        serde_json::Value::Number(serde_json::Number::from(8)),
    );
    config.parameters.insert(
        "ef_construction".to_string(),
        serde_json::Value::Number(serde_json::Number::from(50)),
    );

    let mut index = HNSWIndex::new(config);

    let id1 = "test1".to_string();
    let id2 = "test2".to_string();
    let id3 = "test3".to_string();

    index.add(id1.clone(), vec![1.0, 0.0, 0.0, 0.0])?;
    index.add(id2.clone(), vec![0.0, 1.0, 0.0, 0.0])?;
    index.add(id3.clone(), vec![0.0, 0.0, 1.0, 0.0])?;

    index.build()?;

    let results = index.search(&[1.0, 0.0, 0.0, 0.0], 2)?;

    assert!(!results.is_empty());
    let best_distance = results
        .iter()
        .map(|(_, distance)| *distance)
        .fold(f32::INFINITY, f32::min);
    assert!(
        best_distance < 2.0,
        "Expected at least one result with reasonable distance, best was: {}",
        best_distance
    );

    let close_matches = results
        .iter()
        .filter(|(_, distance)| *distance < 0.1)
        .count();
    assert!(
        close_matches >= 1,
        "Expected at least one close match in results: {:?}",
        results
    );

    index.remove(&id2)?;
    assert_eq!(index.len(), 2);
    Ok(())
}

#[test]
fn test_distance_functions() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let c = vec![1.0, 0.0, 0.0];

    let dist_ab = euclidean_distance(&a, &b);
    let dist_ac = euclidean_distance(&a, &c);

    assert!(dist_ab > dist_ac);
    assert!((dist_ac - 0.0).abs() < f32::EPSILON);

    let cos_dist_ab = cosine_distance(&a, &b);
    let cos_dist_ac = cosine_distance(&a, &c);

    assert!(cos_dist_ab > cos_dist_ac);
    assert!((cos_dist_ac - 0.0).abs() < f32::EPSILON);
}

#[test]
fn test_vector_index_factory() {
    let config = VectorIndexConfig {
        metric: DistanceMetric::Cosine,
        dimensions: 3,
        index_type: IndexType::HNSW,
        parameters: HashMap::new(),
    };

    let index = VectorIndexFactory::create(config);
    assert_eq!(index.len(), 0);
    assert!(index.is_empty());
}
