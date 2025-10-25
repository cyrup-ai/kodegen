// Tests extracted from src/memory/migration/converter.rs

use kodegen_candle_agent::memory::migration::converter::{DataConverter, ImportMetadata, ImportData};

#[test]
fn test_convert_0_1_0_to_0_2_0() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let converter = DataConverter::new("0.1.0", "0.2.0");

    let metadata = ImportMetadata {
        source: "test".to_string(),
        timestamp: chrono::Utc::now(),
        format: "memory_export".to_string(),
        format_version: "0.1.0".to_string(),
    };

    let mut data = ImportData {
        version: "0.1.0".to_string(),
        metadata,
        data: serde_json::Value::Object(serde_json::Map::new()),
        memories: Vec::new(),
        relationships: Vec::new(),
    };

    let memory = kodegen_candle_agent::memory::schema::memory_schema::Memory {
        id: "test-memory".to_string(),
        content: "Test".to_string(),
        memory_type: "semantic".to_string(),
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
        last_accessed_at: chrono::Utc::now().into(),
        importance: 0.5,
        embedding: None,
        tags: vec![],
        metadata: serde_json::Value::Object(serde_json::Map::new()),
    };
    data.memories.push(serde_json::to_value(memory)?);

    let relationship = kodegen_candle_agent::memory::schema::relationship_schema::Relationship::new(
        "source",
        "target",
        "related_to",
    );
    data.relationships.push(serde_json::to_value(relationship)?);

    let result = converter.convert(&data)?;
    assert_eq!(result.metadata.format_version, "0.2.0");

    assert!(!result.memories.is_empty());
    let memory = &result.memories[0];
    if let serde_json::Value::Object(memory_obj) = memory {
        if let Some(serde_json::Value::Object(metadata)) = memory_obj.get("metadata") {
            assert_eq!(
                metadata.get("schema_version").ok_or_else(|| {
                    Box::<dyn std::error::Error>::from("Missing schema_version")
                })?,
                &serde_json::json!("0.2.0")
            );
        } else {
            panic!("Expected memory metadata to be an object");
        }
    } else {
        panic!("Expected memory to be an object");
    }

    assert!(!result.relationships.is_empty());
    let relationship = &result.relationships[0];
    if let serde_json::Value::Object(rel_obj) = relationship {
        if let Some(serde_json::Value::Object(metadata)) = rel_obj.get("metadata") {
            assert_eq!(
                metadata.get("schema_version").ok_or_else(|| {
                    Box::<dyn std::error::Error>::from("Missing schema_version")
                })?,
                &serde_json::json!("0.2.0")
            );
        } else {
            panic!("Expected relationship metadata to be an object");
        }
    } else {
        panic!("Expected relationship to be an object");
    }
    Ok(())
}

#[test]
fn test_convert_0_2_0_to_0_1_0() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let converter = DataConverter::new("0.2.0", "0.1.0");

    let metadata = ImportMetadata {
        source: "test".to_string(),
        timestamp: chrono::Utc::now(),
        format: "memory_export".to_string(),
        format_version: "0.2.0".to_string(),
    };

    let mut data = ImportData {
        version: "0.2.0".to_string(),
        metadata,
        data: serde_json::Value::Object(serde_json::Map::new()),
        memories: Vec::new(),
        relationships: Vec::new(),
    };

    let mut memory_metadata = serde_json::Map::new();
    memory_metadata.insert("schema_version".to_string(), serde_json::json!("0.2.0"));
    memory_metadata.insert("advanced_features".to_string(), serde_json::json!(true));

    let memory = kodegen_candle_agent::memory::schema::memory_schema::Memory {
        id: "test-memory".to_string(),
        content: "Test".to_string(),
        memory_type: "semantic".to_string(),
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
        last_accessed_at: chrono::Utc::now().into(),
        importance: 0.5,
        embedding: None,
        tags: vec![],
        metadata: serde_json::Value::Object(memory_metadata),
    };
    data.memories.push(serde_json::to_value(memory)?);

    let mut relationship = kodegen_candle_agent::memory::schema::relationship_schema::Relationship::new(
        "source",
        "target",
        "related_to",
    );
    let mut rel_metadata = serde_json::Map::new();
    rel_metadata.insert("schema_version".to_string(), serde_json::json!("0.2.0"));
    rel_metadata.insert("advanced_features".to_string(), serde_json::json!(true));
    relationship.metadata = serde_json::Value::Object(rel_metadata);
    data.relationships.push(serde_json::to_value(relationship)?);

    let result = converter.convert(&data)?;
    assert_eq!(result.metadata.format_version, "0.1.0");

    assert!(!result.memories.is_empty());
    let memory = &result.memories[0];
    if let serde_json::Value::Object(memory_obj) = memory {
        if let Some(serde_json::Value::Object(metadata)) = memory_obj.get("metadata") {
            assert!(metadata.get("schema_version").is_none());
            assert!(metadata.get("advanced_features").is_none());
        } else {
            panic!("Expected memory metadata to be an object");
        }
    } else {
        panic!("Expected memory to be an object");
    }

    assert!(!result.relationships.is_empty());
    let relationship = &result.relationships[0];
    if let serde_json::Value::Object(rel_obj) = relationship {
        if let Some(serde_json::Value::Object(metadata)) = rel_obj.get("metadata") {
            assert!(metadata.get("schema_version").is_none());
            assert!(metadata.get("advanced_features").is_none());
        } else {
            panic!("Expected relationship metadata to be an object");
        }
    } else {
        panic!("Expected relationship to be an object");
    }
    Ok(())
}

#[test]
fn test_custom_conversion_rule() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut converter = DataConverter::new("custom", "target");

    converter.add_custom_rule("custom", "target", |data| {
        let mut new_data = data.clone();
        new_data.metadata.format_version = "custom-converted".to_string();
        Ok(new_data)
    });

    let metadata = ImportMetadata {
        source: "test".to_string(),
        timestamp: chrono::Utc::now(),
        format: "memory_export".to_string(),
        format_version: "custom".to_string(),
    };

    let data = ImportData {
        version: "custom".to_string(),
        metadata,
        data: serde_json::Value::Object(serde_json::Map::new()),
        memories: Vec::new(),
        relationships: Vec::new(),
    };

    let result = converter.convert(&data)?;
    assert_eq!(result.metadata.format_version, "custom-converted");
    Ok(())
}

