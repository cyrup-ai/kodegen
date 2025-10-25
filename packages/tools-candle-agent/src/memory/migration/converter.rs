// src/migration/converter.rs
//! Data conversion utilities for import/export.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json;

use crate::memory::migration::Result;

/// Type alias for conversion rules
pub type ConversionRule = Arc<dyn Fn(&ImportData) -> Result<ImportData> + Send + Sync>;

/// Type alias for conversion rules map
pub type ConversionRulesMap = HashMap<(String, String), ConversionRule>;

/// Import data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportData {
    pub version: String,
    pub metadata: ImportMetadata,
    pub data: serde_json::Value,
    /// Memory records
    pub memories: Vec<serde_json::Value>,
    /// Relationship records
    pub relationships: Vec<serde_json::Value>,
}

/// Import metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportMetadata {
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub format: String,
    /// Format version
    pub format_version: String,
}

/// Data converter for migrating between different format versions
pub struct DataConverter {
    /// Source version
    source_version: String,
    /// Target version
    target_version: String,
    /// Custom conversion rules
    custom_rules: ConversionRulesMap,
}

impl DataConverter {
    /// Create a new data converter
    pub fn new(source_version: impl Into<String>, target_version: impl Into<String>) -> Self {
        Self {
            source_version: source_version.into(),
            target_version: target_version.into(),
            custom_rules: HashMap::new(),
        }
    }

    /// Convert data from source version to target version
    pub fn convert(&self, data: &ImportData) -> Result<ImportData> {
        // Check if we have a custom rule for this conversion
        let key_lookup = (&self.source_version, &self.target_version);

        // Find rule by comparing with existing keys
        if let Some(rule) = self.custom_rules.iter().find_map(|(key, rule)| {
            if (&key.0, &key.1) == key_lookup {
                Some(rule)
            } else {
                None
            }
        }) {
            return rule(data);
        }

        // Otherwise, use built-in conversion logic
        match (self.source_version.as_str(), self.target_version.as_str()) {
            ("0.1.0", "0.2.0") => self.convert_0_1_0_to_0_2_0(data),
            ("0.2.0", "0.1.0") => self.convert_0_2_0_to_0_1_0(data),
            _ => self.apply_generic_upgrade(data),
        }
    }

    /// Convert from version 0.1.0 to 0.2.0
    fn convert_0_1_0_to_0_2_0(&self, data: &ImportData) -> Result<ImportData> {
        // Create a new ImportData with updated metadata
        let mut new_data = data.clone();
        new_data.metadata.format_version = "0.2.0".to_string();

        // Update memories
        new_data.memories = data
            .memories
            .iter()
            .map(|memory| {
                let mut new_memory = memory.clone();

                // Add any new fields or transform existing ones
                // For example, add a new field to metadata if it exists
                if let Some(serde_json::Value::Object(metadata_obj)) = new_memory.get("metadata") {
                    let mut new_metadata = metadata_obj.clone();
                    new_metadata.insert("schema_version".to_string(), serde_json::json!("0.2.0"));
                    if let serde_json::Value::Object(obj) = &mut new_memory {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                } else {
                    // Create new metadata object if none exists
                    let mut new_metadata = serde_json::Map::new();
                    new_metadata.insert("schema_version".to_string(), serde_json::json!("0.2.0"));
                    if let serde_json::Value::Object(obj) = &mut new_memory {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_memory
            })
            .collect();

        // Update relationships
        new_data.relationships = data
            .relationships
            .iter()
            .map(|relationship| {
                let mut new_relationship = relationship.clone();

                // Update relationship metadata with schema version
                if let Some(serde_json::Value::Object(metadata_obj)) =
                    new_relationship.get("metadata")
                {
                    // Process metadata fields
                    let mut new_metadata = metadata_obj.clone();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );

                    // Update relationship with new metadata
                    if let serde_json::Value::Object(obj) = &mut new_relationship {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                } else {
                    // Create new metadata object if none exists
                    let mut new_metadata = serde_json::Map::new();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );
                    if let serde_json::Value::Object(obj) = &mut new_relationship {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_relationship
            })
            .collect();

        Ok(new_data)
    }

    /// Convert from version 0.2.0 to 0.1.0
    fn convert_0_2_0_to_0_1_0(&self, data: &ImportData) -> Result<ImportData> {
        // Create a new ImportData with updated metadata
        let mut new_data = data.clone();
        new_data.metadata.format_version = "0.1.0".to_string();

        // Update memories
        new_data.memories = data
            .memories
            .iter()
            .map(|memory| {
                let mut new_memory = memory.clone();

                // Remove or transform fields
                if let Some(serde_json::Value::Object(metadata_obj)) = new_memory.get("metadata") {
                    let mut new_metadata = metadata_obj.clone();
                    // Remove schema version and other fields not compatible with older versions
                    new_metadata.remove("schema_version");
                    new_metadata.remove("advanced_features");
                    new_metadata.remove("version_specific_data");

                    if let serde_json::Value::Object(obj) = &mut new_memory {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_memory
            })
            .collect();

        // Update relationships
        new_data.relationships = data
            .relationships
            .iter()
            .map(|relationship| {
                let mut new_relationship = relationship.clone();

                // Remove or transform fields
                if let Some(serde_json::Value::Object(metadata_obj)) =
                    new_relationship.get("metadata")
                {
                    let mut new_metadata = metadata_obj.clone();
                    // Remove schema version and other fields not compatible with older versions
                    new_metadata.remove("schema_version");

                    // Remove any fields that might not be compatible with older versions
                    new_metadata.remove("advanced_features");

                    // Remove version specific data
                    new_metadata.remove("version_specific_data");

                    if let serde_json::Value::Object(obj) = &mut new_relationship {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_relationship
            })
            .collect();

        Ok(new_data)
    }

    /// Apply generic upgrade
    fn apply_generic_upgrade(&self, data: &ImportData) -> Result<ImportData> {
        // Create a new ImportData with updated metadata
        let mut new_data = data.clone();
        new_data.metadata.format_version = self.target_version.clone();

        // For generic upgrades, we keep all existing data and add version info to metadata
        new_data.memories = data
            .memories
            .iter()
            .map(|memory| {
                let mut new_memory = memory.clone();

                // Update metadata with schema version
                if let Some(serde_json::Value::Object(metadata_obj)) = new_memory.get("metadata") {
                    let mut new_metadata = metadata_obj.clone();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );
                    if let serde_json::Value::Object(obj) = &mut new_memory {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                } else {
                    // If metadata is not an object, create a new object
                    let mut new_metadata = serde_json::Map::new();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );
                    if let serde_json::Value::Object(obj) = &mut new_memory {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_memory
            })
            .collect();

        // Update relationships
        new_data.relationships = data
            .relationships
            .iter()
            .map(|relationship| {
                let mut new_relationship = relationship.clone();

                // Update relationship metadata with schema version
                if let Some(serde_json::Value::Object(metadata_obj)) =
                    new_relationship.get("metadata")
                {
                    // Process metadata fields
                    let mut new_metadata = metadata_obj.clone();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );

                    // Update relationship with new metadata
                    if let serde_json::Value::Object(obj) = &mut new_relationship {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                } else {
                    // Create new metadata object if none exists
                    let mut new_metadata = serde_json::Map::new();
                    new_metadata.insert(
                        "schema_version".to_string(),
                        serde_json::json!(self.target_version),
                    );
                    if let serde_json::Value::Object(obj) = &mut new_relationship {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(new_metadata),
                        );
                    }
                }

                new_relationship
            })
            .collect();

        Ok(new_data)
    }

    /// Add custom conversion rule
    pub fn add_custom_rule<F>(
        &mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        rule: F,
    ) where
        F: Fn(&ImportData) -> Result<ImportData> + Send + Sync + 'static,
    {
        let key = (source.into(), target.into());
        self.custom_rules.insert(key, Arc::new(rule));
    }
}
