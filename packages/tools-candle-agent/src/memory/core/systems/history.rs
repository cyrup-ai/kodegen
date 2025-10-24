// src/memory/history.rs
//! History tracking implementation for Rust-mem0.
//!
//! This module provides versioning and history tracking for memory nodes,
//! with support for evolution tracking, history queries, and diff/merge operations.

use std::collections::HashMap;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::Value;

use crate::memory::graph::entity::{BaseEntity, Entity};
use crate::memory::utils::Result;
use crate::memory::utils::error::Error;

/// Change type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Creation of a new memory
    Creation,
    /// Update of an existing memory
    Update,
    /// Deletion of a memory
    Deletion,
    /// Restoration of a deleted memory
    Restoration,
    /// Merge of multiple memories
    Merge,
    /// Split of a memory into multiple parts
    Split,
    /// Custom change type
    Custom(u8),
}

impl ChangeType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Creation => "creation",
            ChangeType::Update => "update",
            ChangeType::Deletion => "deletion",
            ChangeType::Restoration => "restoration",
            ChangeType::Merge => "merge",
            ChangeType::Split => "split",
            ChangeType::Custom(_) => "custom",
        }
    }

    /// Parse from string
    pub fn parse_from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "creation" => Ok(ChangeType::Creation),
            "update" => Ok(ChangeType::Update),
            "deletion" => Ok(ChangeType::Deletion),
            "restoration" => Ok(ChangeType::Restoration),
            "merge" => Ok(ChangeType::Merge),
            "split" => Ok(ChangeType::Split),
            s if s.starts_with("custom") => {
                if let Some(code_str) = s.strip_prefix("custom") {
                    if let Ok(code) = code_str.trim().parse::<u8>() {
                        Ok(ChangeType::Custom(code))
                    } else {
                        Ok(ChangeType::Custom(0))
                    }
                } else {
                    Ok(ChangeType::Custom(0))
                }
            }
            _ => Err(Error::ValidationError(format!("Invalid change type: {s}"))),
        }
    }

    /// Convert to value
    pub fn to_value(&self) -> Value {
        use surrealdb::value::to_value;
        match self {
            ChangeType::Creation => to_value("creation").unwrap_or_default(),
            ChangeType::Update => to_value("update").unwrap_or_default(),
            ChangeType::Deletion => to_value("deletion").unwrap_or_default(),
            ChangeType::Restoration => to_value("restoration").unwrap_or_default(),
            ChangeType::Merge => to_value("merge").unwrap_or_default(),
            ChangeType::Split => to_value("split").unwrap_or_default(),
            ChangeType::Custom(code) => to_value(format!("custom{code}")).unwrap_or_default(),
        }
    }

    /// Create from value
    pub fn from_value(value: &Value) -> Result<Self> {
        use surrealdb::value::from_value;

        if let Ok(s) = from_value::<String>(value.clone()) {
            Self::parse_from_str(&s)
        } else {
            Err(Error::ConversionError(
                "Invalid change type value".to_string(),
            ))
        }
    }
}

/// Memory version struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersion {
    /// Version ID
    pub id: String,
    /// Memory ID
    pub memory_id: String,
    /// Version number
    pub version: u32,
    /// Change type
    pub change_type: ChangeType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User ID
    pub user_id: Option<String>,
    /// Content
    pub content: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, Value>,
    /// Previous version ID
    pub previous_version_id: Option<String>,
    /// Related version IDs (for merge/split)
    pub related_version_ids: Vec<String>,
    /// Change summary
    pub change_summary: Option<String>,
    /// Diff from previous version
    pub diff: Option<String>,
}

impl MemoryVersion {
    /// Create a new memory version
    pub fn new(
        id: &str,
        memory_id: &str,
        version: u32,
        change_type: ChangeType,
        content: Option<&str>,
        previous_version_id: Option<&str>,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: id.to_string(),
            memory_id: memory_id.to_string(),
            version,
            change_type,
            timestamp: now,
            user_id: None,
            content: content.map(|s| s.to_string()),
            metadata: HashMap::new(),
            previous_version_id: previous_version_id.map(|s| s.to_string()),
            related_version_ids: Vec::new(),
            change_summary: None,
            diff: None,
        }
    }

    /// Create a new creation version
    pub fn creation(id: &str, memory_id: &str, content: &str) -> Self {
        Self::new(id, memory_id, 1, ChangeType::Creation, Some(content), None)
    }

    /// Create a new update version
    pub fn update(
        id: &str,
        memory_id: &str,
        version: u32,
        content: &str,
        previous_version_id: &str,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            version,
            ChangeType::Update,
            Some(content),
            Some(previous_version_id),
        )
    }

    /// Create a new deletion version
    pub fn deletion(id: &str, memory_id: &str, version: u32, previous_version_id: &str) -> Self {
        Self::new(
            id,
            memory_id,
            version,
            ChangeType::Deletion,
            None,
            Some(previous_version_id),
        )
    }

    /// Create a new restoration version
    pub fn restoration(
        id: &str,
        memory_id: &str,
        version: u32,
        content: &str,
        previous_version_id: &str,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            version,
            ChangeType::Restoration,
            Some(content),
            Some(previous_version_id),
        )
    }

    /// Set user ID
    #[must_use]
    pub fn with_user_id(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Add related version ID
    #[must_use]
    pub fn with_related_version_id(mut self, related_version_id: &str) -> Self {
        self.related_version_ids
            .push(related_version_id.to_string());
        self
    }

    /// Set change summary
    #[must_use]
    pub fn with_change_summary(mut self, change_summary: &str) -> Self {
        self.change_summary = Some(change_summary.to_string());
        self
    }

    /// Set diff
    #[must_use]
    pub fn with_diff(mut self, diff: &str) -> Self {
        self.diff = Some(diff.to_string());
        self
    }

    /// Convert to entity
    pub fn to_entity(&self) -> BaseEntity {
        let mut entity = BaseEntity::new(self.id.clone(), "memory_version".to_string());

        // Add basic attributes
        entity = entity.with_attribute(
            "memory_id",
            surrealdb::value::to_value(self.memory_id.clone()).unwrap_or_default(),
        );
        entity = entity.with_attribute(
            "version",
            surrealdb::value::to_value(self.version).unwrap_or_default(),
        );
        entity = entity.with_attribute("change_type", self.change_type.to_value());
        entity = entity.with_attribute(
            "timestamp",
            surrealdb::value::to_value(self.timestamp).unwrap_or_default(),
        );

        // Add optional attributes
        if let Some(ref user_id) = self.user_id {
            entity = entity.with_attribute(
                "user_id",
                surrealdb::value::to_value(user_id.clone()).unwrap_or_default(),
            );
        }

        if let Some(ref content) = self.content {
            entity = entity.with_attribute(
                "content",
                surrealdb::value::to_value(content.clone()).unwrap_or_default(),
            );
        }

        if let Some(ref previous_version_id) = self.previous_version_id {
            entity = entity.with_attribute(
                "previous_version_id",
                surrealdb::value::to_value(previous_version_id.clone()).unwrap_or_default(),
            );
        }

        if !self.related_version_ids.is_empty() {
            entity = entity.with_attribute(
                "related_version_ids",
                surrealdb::value::to_value(self.related_version_ids.clone()).unwrap_or_default(),
            );
        }

        if let Some(ref change_summary) = self.change_summary {
            entity = entity.with_attribute(
                "change_summary",
                surrealdb::value::to_value(change_summary.clone()).unwrap_or_default(),
            );
        }

        if let Some(ref diff) = self.diff {
            entity = entity.with_attribute(
                "diff",
                surrealdb::value::to_value(diff.clone()).unwrap_or_default(),
            );
        }

        // Add metadata
        for (key, value) in &self.metadata {
            entity = entity.with_attribute(&format!("metadata_{key}"), value.clone());
        }

        entity
    }

    /// Create from entity
    pub fn from_entity(entity: &dyn Entity) -> Result<Self> {
        let id = entity.id().to_string();

        let memory_id = if let Some(value) = entity.get_attribute("memory_id") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone())
                .map_err(|_| Error::ConversionError("Invalid memory_id attribute".to_string()))?
        } else {
            return Err(Error::ConversionError(
                "Missing memory_id attribute".to_string(),
            ));
        };

        let version = if let Some(value) = entity.get_attribute("version") {
            use surrealdb::value::from_value;
            from_value::<u32>(value.clone())
                .map_err(|_| Error::ConversionError("Invalid version attribute".to_string()))?
        } else {
            return Err(Error::ConversionError(
                "Missing version attribute".to_string(),
            ));
        };

        let change_type = if let Some(value) = entity.get_attribute("change_type") {
            ChangeType::from_value(value)?
        } else {
            return Err(Error::ConversionError(
                "Missing change_type attribute".to_string(),
            ));
        };

        let timestamp = if let Some(value) = entity.get_attribute("timestamp") {
            use surrealdb::value::from_value;
            from_value::<DateTime<Utc>>(value.clone()).map_err(|e| {
                Error::ConversionError(format!("Invalid timestamp attribute: {}", e))
            })?
        } else {
            return Err(Error::ConversionError(
                "Missing timestamp attribute".to_string(),
            ));
        };

        let user_id = if let Some(value) = entity.get_attribute("user_id") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        let content = if let Some(value) = entity.get_attribute("content") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        let previous_version_id = if let Some(value) = entity.get_attribute("previous_version_id") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        let related_version_ids = if let Some(value) = entity.get_attribute("related_version_ids") {
            use surrealdb::value::from_value;
            from_value::<Vec<String>>(value.clone()).unwrap_or_default()
        } else {
            Vec::new()
        };

        let change_summary = if let Some(value) = entity.get_attribute("change_summary") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        let diff = if let Some(value) = entity.get_attribute("diff") {
            use surrealdb::value::from_value;
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        // Extract metadata
        let mut metadata = HashMap::new();
        for (key, value) in entity.attributes() {
            if let Some(stripped_key) = key.strip_prefix("metadata_") {
                metadata.insert(stripped_key.to_string(), value.clone());
            }
        }

        Ok(Self {
            id,
            memory_id,
            version,
            change_type,
            timestamp,
            user_id,
            content,
            metadata,
            previous_version_id,
            related_version_ids,
            change_summary,
            diff,
        })
    }
}

/// Memory history struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHistory {
    /// Memory ID
    pub memory_id: String,
    /// Current version
    pub current_version: u32,
    /// Versions
    pub versions: Vec<MemoryVersion>,
}

impl MemoryHistory {
    /// Create a new memory history
    pub fn new(memory_id: &str) -> Self {
        Self {
            memory_id: memory_id.to_string(),
            current_version: 0,
            versions: Vec::new(),
        }
    }

    /// Add version
    pub fn add_version(&mut self, version: MemoryVersion) -> Result<()> {
        // Validate version
        if version.memory_id != self.memory_id {
            return Err(Error::ValidationError(format!(
                "Version memory ID {} does not match history memory ID {}",
                version.memory_id, self.memory_id
            )));
        }

        // Update current version if needed
        if version.version > self.current_version {
            self.current_version = version.version;
        }

        // Add version
        self.versions.push(version);

        // Sort versions by version number
        self.versions.sort_by(|a, b| a.version.cmp(&b.version));

        Ok(())
    }

    /// Get version by number
    pub fn get_version(&self, version: u32) -> Option<&MemoryVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// Get version by ID
    pub fn get_version_by_id(&self, id: &str) -> Option<&MemoryVersion> {
        self.versions.iter().find(|v| v.id == id)
    }

    /// Get current version
    pub fn get_current_version(&self) -> Option<&MemoryVersion> {
        self.get_version(self.current_version)
    }

    /// Get versions by change type
    pub fn get_versions_by_change_type(&self, change_type: ChangeType) -> Vec<&MemoryVersion> {
        self.versions
            .iter()
            .filter(|v| v.change_type == change_type)
            .collect()
    }

    /// Get versions by time range
    pub fn get_versions_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&MemoryVersion> {
        self.versions
            .iter()
            .filter(|v| v.timestamp >= start && v.timestamp <= end)
            .collect()
    }

    /// Get version history
    pub fn get_version_history(&self) -> Vec<(u32, ChangeType, DateTime<Utc>)> {
        self.versions
            .iter()
            .map(|v| (v.version, v.change_type, v.timestamp))
            .collect()
    }

    /// Get version path
    pub fn get_version_path(
        &self,
        from_version: u32,
        to_version: u32,
    ) -> Result<Vec<&MemoryVersion>> {
        // Validate versions
        if from_version > to_version {
            return Err(Error::ValidationError(format!(
                "From version {from_version} is greater than to version {to_version}"
            )));
        }

        // Get versions in range
        let mut path = Vec::new();
        for version in from_version..=to_version {
            if let Some(v) = self.get_version(version) {
                path.push(v);
            } else {
                return Err(Error::NotFound(format!("Version {version} not found")));
            }
        }

        Ok(path)
    }

    /// Calculate diff between versions
    pub fn diff_versions(&self, from_version: u32, to_version: u32) -> Result<String> {
        // Get versions
        let from = self
            .get_version(from_version)
            .ok_or_else(|| Error::NotFound(format!("Version {from_version} not found")))?;

        let to = self
            .get_version(to_version)
            .ok_or_else(|| Error::NotFound(format!("Version {to_version} not found")))?;

        // Get content
        let from_content = from.content.as_deref().unwrap_or("");
        let to_content = to.content.as_deref().unwrap_or("");

        // Calculate diff
        let diff = self.calculate_diff(from_content, to_content);

        Ok(diff)
    }

    /// Calculate text diff between two strings
    fn calculate_diff(&self, old: &str, new: &str) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        // Simple line-based diff
        // Safe: writeln! to String never fails as String always has sufficient capacity
        let _ = writeln!(&mut output, "--- Version A");
        let _ = writeln!(&mut output, "+++ Version B");

        let max_lines = old_lines.len().max(new_lines.len());
        for i in 0..max_lines {
            match (old_lines.get(i), new_lines.get(i)) {
                (Some(old_line), Some(new_line)) if old_line != new_line => {
                    let _ = writeln!(&mut output, "-{old_line}");
                    let _ = writeln!(&mut output, "+{new_line}");
                }
                (Some(old_line), None) => {
                    let _ = writeln!(&mut output, "-{old_line}");
                }
                (None, Some(new_line)) => {
                    let _ = writeln!(&mut output, "+{new_line}");
                }
                (Some(line), Some(_)) => {
                    let _ = writeln!(&mut output, " {line}");
                }
                _ => {}
            }
        }

        output
    }
}
