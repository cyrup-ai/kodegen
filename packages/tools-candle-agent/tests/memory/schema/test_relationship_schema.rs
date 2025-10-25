// Tests extracted from src/memory/schema/relationship_schema.rs

use kodegen_candle_agent::memory::schema::relationship_schema::Relationship;

#[test]
fn test_new_relationship() {
    let relationship = Relationship::new("source-id", "target-id", "related_to");

    // RecordId doesn't have is_empty(), just verify it exists by converting to string
    assert!(!relationship.id.to_string().is_empty());
    assert_eq!(relationship.source_id, "source-id");
    assert_eq!(relationship.target_id, "target-id");
    assert_eq!(relationship.relationship_type, "related_to");
    assert_eq!(relationship.strength, 1.0);
    assert_eq!(relationship.metadata, serde_json::json!({}));
}

#[test]
fn test_relationship_builder_pattern() {
    let relationship = Relationship::new("source-id", "target-id", "related_to")
        .with_strength(0.75)
        .with_metadata(serde_json::json!({"key": "value"}));

    assert_eq!(relationship.strength, 0.75);
    assert_eq!(relationship.metadata, serde_json::json!({"key": "value"}));
}

#[test]
fn test_relationship_metadata_operations() {
    let mut relationship = Relationship::new("source-id", "target-id", "related_to");

    relationship.set_metadata_value("string", "value").unwrap();
    relationship.set_metadata_value("number", 42).unwrap();
    relationship.set_metadata_value("bool", true).unwrap();

    assert_eq!(
        relationship.get_metadata_value::<String>("string"),
        Some("value".to_string())
    );
    assert_eq!(relationship.get_metadata_value::<i32>("number"), Some(42));
    assert_eq!(relationship.get_metadata_value::<bool>("bool"), Some(true));
    assert_eq!(relationship.get_metadata_value::<String>("nonexistent"), None);

    relationship.remove_metadata_value("number");
    assert_eq!(relationship.get_metadata_value::<i32>("number"), None);
}

#[test]
fn test_relationship_additional_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut relationship = Relationship::new("source-id", "target-id", "related_to");

    relationship.set_additional_field("field1", 123)?;
    relationship.set_additional_field("field2", "value")?;

    assert_eq!(
        relationship.get_additional_field::<i32>("field1"),
        Some(123)
    );
    assert_eq!(
        relationship.get_additional_field::<String>("field2"),
        Some("value".to_string())
    );
    assert_eq!(
        relationship.get_additional_field::<bool>("nonexistent"),
        None
    );

    relationship.remove_additional_field("field1");
    assert_eq!(relationship.get_additional_field::<i32>("field1"), None);
    Ok(())
}

#[test]
fn test_relationship_timestamp_iso8601() {
    let relationship = Relationship::new("source-id", "target-id", "related_to");

    let created_iso = relationship.created_at_iso8601();
    let updated_iso = relationship.updated_at_iso8601();

    assert!(!created_iso.is_empty());
    assert!(!updated_iso.is_empty());
    assert_eq!(created_iso, updated_iso);
}

#[test]
fn test_relationship_between_and_involves() {
    let relationship = Relationship::new("memory1", "memory2", "related_to");

    assert!(relationship.is_between("memory1", "memory2"));
    assert!(relationship.is_between("memory2", "memory1"));
    assert!(!relationship.is_between("memory1", "memory3"));

    assert!(relationship.involves("memory1"));
    assert!(relationship.involves("memory2"));
    assert!(!relationship.involves("memory3"));

    assert_eq!(relationship.get_other_memory_id("memory1"), Some("memory2"));
    assert_eq!(relationship.get_other_memory_id("memory2"), Some("memory1"));
    assert_eq!(relationship.get_other_memory_id("memory3"), None);
}

#[test]
fn test_relationship_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let relationship = Relationship::new("source-id", "target-id", "related_to")
        .with_metadata(serde_json::json!({"key": "value"}))
        .with_strength(0.75);

    let json = serde_json::to_string(&relationship)?;
    let deserialized: Relationship = serde_json::from_str(&json)?;

    assert_eq!(deserialized.id, relationship.id);
    assert_eq!(deserialized.source_id, relationship.source_id);
    assert_eq!(deserialized.target_id, relationship.target_id);
    assert_eq!(
        deserialized.relationship_type,
        relationship.relationship_type
    );
    assert_eq!(deserialized.metadata, relationship.metadata);
    assert_eq!(deserialized.strength, relationship.strength);
    Ok(())
}
