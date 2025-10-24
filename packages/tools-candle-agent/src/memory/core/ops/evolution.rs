// src/memory/evolution.rs
//! Memory evolution tracking implementation for Rust-mem0.
//!
//! This module provides tracking of memory evolution over time,
//! including transitions, relationships between versions, and
//! analysis of memory development patterns.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::Value;
use surrealdb::value::{from_value, to_value};

use crate::memory::constants::{EMPTY_STRING, METADATA_PREFIX};
use crate::memory::graph::entity::{BaseEntity, Entity};
use crate::memory::utils::Result;
use crate::memory::utils::error::Error;

/// Evolution transition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionType {
    /// Linear progression (simple update)
    Linear,
    /// Branching (creating multiple versions from one)
    Branching,
    /// Merging (combining multiple versions)
    Merging,
    /// Restoration (bringing back a deleted or previous version)
    Restoration,
    /// Deletion (removing a version)
    Deletion,
    /// Custom transition
    Custom(u8),
}

impl TransitionType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            TransitionType::Linear => "linear",
            TransitionType::Branching => "branching",
            TransitionType::Merging => "merging",
            TransitionType::Restoration => "restoration",
            TransitionType::Deletion => "deletion",
            TransitionType::Custom(_) => "custom",
        }
    }
}

impl std::str::FromStr for TransitionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(TransitionType::Linear),
            "branching" => Ok(TransitionType::Branching),
            "merging" => Ok(TransitionType::Merging),
            "restoration" => Ok(TransitionType::Restoration),
            "deletion" => Ok(TransitionType::Deletion),
            s if s.starts_with("custom") => {
                if let Some(code_str) = s.strip_prefix("custom") {
                    if let Ok(code) = code_str.trim().parse::<u8>() {
                        Ok(TransitionType::Custom(code))
                    } else {
                        Ok(TransitionType::Custom(0))
                    }
                } else {
                    Ok(TransitionType::Custom(0))
                }
            }
            _ => Err(Error::ValidationError(format!(
                "Invalid transition type: {s}"
            ))),
        }
    }
}

impl TransitionType {
    /// Convert to value
    pub fn to_value(&self) -> Value {
        use surrealdb::value::to_value;

        match self {
            TransitionType::Linear => to_value("linear").unwrap_or_default(),
            TransitionType::Branching => to_value("branching").unwrap_or_default(),
            TransitionType::Merging => to_value("merging").unwrap_or_default(),
            TransitionType::Restoration => to_value("restoration").unwrap_or_default(),
            TransitionType::Deletion => to_value("deletion").unwrap_or_default(),
            TransitionType::Custom(code) => to_value(format!("custom{code}")).unwrap_or_default(),
        }
    }

    /// Create from value
    pub fn from_value(value: &Value) -> Result<Self> {
        use surrealdb::value::from_value;

        // Convert SurrealDB Value to string and parse
        if let Ok(s) = from_value::<String>(value.clone()) {
            s.parse()
        } else {
            Err(Error::ConversionError(
                "Invalid transition type value".to_string(),
            ))
        }
    }
}

/// Memory evolution transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTransition {
    /// Transition ID
    pub id: String,
    /// Memory ID
    pub memory_id: String,
    /// Transition type
    pub transition_type: TransitionType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Primary source version ID (for simple transitions)
    pub from_version_id: String,
    /// Primary target version ID (for simple transitions)
    pub to_version_id: String,
    /// Source version IDs (for complex transitions)
    pub source_version_ids: Vec<String>,
    /// Target version IDs (for complex transitions)
    pub target_version_ids: Vec<String>,
    /// Transition metadata
    pub metadata: HashMap<String, Value>,
    /// Transition description
    pub description: Option<String>,
}

impl EvolutionTransition {
    /// Create a new evolution transition
    pub fn new(
        id: &str,
        memory_id: &str,
        transition_type: TransitionType,
        source_version_ids: Vec<String>,
        target_version_ids: Vec<String>,
    ) -> Self {
        let now = Utc::now();

        // Set primary version IDs from first elements of vectors
        let from_version_id = source_version_ids
            .first()
            .cloned()
            .unwrap_or_else(|| EMPTY_STRING.to_string());
        let to_version_id = target_version_ids
            .first()
            .cloned()
            .unwrap_or_else(|| EMPTY_STRING.to_string());

        Self {
            id: id.to_string(),
            memory_id: memory_id.to_string(),
            transition_type,
            timestamp: now,
            from_version_id,
            to_version_id,
            source_version_ids,
            target_version_ids,
            metadata: HashMap::new(),
            description: None,
        }
    }

    /// Create a linear transition
    pub fn linear(
        id: &str,
        memory_id: &str,
        source_version_id: &str,
        target_version_id: &str,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            TransitionType::Linear,
            vec![source_version_id.to_string()],
            vec![target_version_id.to_string()],
        )
    }

    /// Create a branching transition
    pub fn branching(
        id: &str,
        memory_id: &str,
        source_version_id: &str,
        target_version_ids: Vec<String>,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            TransitionType::Branching,
            vec![source_version_id.to_string()],
            target_version_ids,
        )
    }

    /// Create a merging transition
    pub fn merging(
        id: &str,
        memory_id: &str,
        source_version_ids: Vec<String>,
        target_version_id: &str,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            TransitionType::Merging,
            source_version_ids,
            vec![target_version_id.to_string()],
        )
    }

    /// Create a restoration transition
    pub fn restoration(
        id: &str,
        memory_id: &str,
        source_version_id: &str,
        target_version_id: &str,
    ) -> Self {
        Self::new(
            id,
            memory_id,
            TransitionType::Restoration,
            vec![source_version_id.to_string()],
            vec![target_version_id.to_string()],
        )
    }

    /// Create a deletion transition
    pub fn deletion(id: &str, memory_id: &str, source_version_id: &str) -> Self {
        Self::new(
            id,
            memory_id,
            TransitionType::Deletion,
            vec![source_version_id.to_string()],
            vec![],
        )
    }

    /// Add metadata
    pub fn with_metadata<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Convert to entity
    pub fn to_entity(&self) -> BaseEntity {
        let mut entity =
            BaseEntity::new(self.id.clone(), "memory_evolution_transition".to_string());

        // Add basic attributes using to_value
        entity = entity.with_attribute(
            "memory_id",
            to_value(self.memory_id.clone()).unwrap_or_default(),
        );
        entity = entity.with_attribute("transition_type", self.transition_type.to_value());
        entity = entity.with_attribute("timestamp", to_value(self.timestamp).unwrap_or_default());

        // Add source version IDs
        if !self.source_version_ids.is_empty() {
            entity = entity.with_attribute(
                "source_version_ids",
                to_value(self.source_version_ids.clone()).unwrap_or_default(),
            );
        }

        // Add target version IDs
        if !self.target_version_ids.is_empty() {
            entity = entity.with_attribute(
                "target_version_ids",
                to_value(self.target_version_ids.clone()).unwrap_or_default(),
            );
        }

        // Add description if present
        if let Some(ref description) = self.description {
            entity = entity.with_attribute(
                "description",
                to_value(description.clone()).unwrap_or_default(),
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
            from_value::<String>(value.clone()).map_err(|_| {
                Error::ConversionError("Invalid memory_id attribute type".to_string())
            })?
        } else {
            return Err(Error::ConversionError(
                "Missing memory_id attribute".to_string(),
            ));
        };

        let transition_type = if let Some(value) = entity.get_attribute("transition_type") {
            TransitionType::from_value(value)?
        } else {
            return Err(Error::ConversionError(
                "Missing transition_type attribute".to_string(),
            ));
        };

        let timestamp = if let Some(value) = entity.get_attribute("timestamp") {
            from_value::<DateTime<Utc>>(value.clone()).map_err(|_| {
                Error::ConversionError("Invalid timestamp attribute type".to_string())
            })?
        } else {
            return Err(Error::ConversionError(
                "Missing timestamp attribute".to_string(),
            ));
        };

        let source_version_ids = if let Some(value) = entity.get_attribute("source_version_ids") {
            from_value::<Vec<String>>(value.clone()).unwrap_or_default()
        } else {
            Vec::new()
        };

        let target_version_ids = if let Some(value) = entity.get_attribute("target_version_ids") {
            from_value::<Vec<String>>(value.clone()).unwrap_or_default()
        } else {
            Vec::new()
        };

        let description = if let Some(value) = entity.get_attribute("description") {
            from_value::<String>(value.clone()).ok()
        } else {
            None
        };

        // Extract metadata
        let mut metadata = HashMap::new();
        for (key, value) in entity.attributes() {
            if key.starts_with(METADATA_PREFIX)
                && let Some(metadata_key) = key.strip_prefix(METADATA_PREFIX)
            {
                metadata.insert(metadata_key.to_string(), value.clone());
            }
        }

        // Set primary version IDs from first elements of vectors
        let from_version_id = source_version_ids.first().cloned().unwrap_or_default();
        let to_version_id = target_version_ids.first().cloned().unwrap_or_default();

        Ok(Self {
            id,
            memory_id,
            transition_type,
            timestamp,
            source_version_ids,
            target_version_ids,
            from_version_id,
            to_version_id,
            metadata,
            description,
        })
    }
}

/// Memory evolution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionGraph {
    /// Memory ID
    pub memory_id: String,
    /// Transitions
    pub transitions: Vec<EvolutionTransition>,
    /// Version IDs
    pub version_ids: HashSet<String>,
}

impl EvolutionGraph {
    /// Create a new evolution graph
    pub fn new(memory_id: &str) -> Self {
        Self {
            memory_id: memory_id.to_string(),
            transitions: Vec::new(),
            version_ids: HashSet::new(),
        }
    }

    /// Add transition
    pub fn add_transition(&mut self, transition: EvolutionTransition) -> Result<()> {
        // Validate transition
        if transition.memory_id != self.memory_id {
            return Err(Error::ValidationError(format!(
                "Transition memory ID {} does not match graph memory ID {}",
                transition.memory_id, self.memory_id
            )));
        }

        // Add transition
        self.transitions.push(transition.clone());

        // Add version IDs
        for id in &transition.source_version_ids {
            self.version_ids.insert(id.clone());
        }

        for id in &transition.target_version_ids {
            self.version_ids.insert(id.clone());
        }

        // Sort transitions by timestamp
        self.transitions
            .sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(())
    }

    /// Get transition by ID
    pub fn get_transition(&self, id: &str) -> Option<&EvolutionTransition> {
        self.transitions.iter().find(|t| t.id == id)
    }

    /// Get transitions by type
    pub fn get_transitions_by_type(
        &self,
        transition_type: TransitionType,
    ) -> Vec<&EvolutionTransition> {
        self.transitions
            .iter()
            .filter(|t| t.transition_type == transition_type)
            .collect()
    }

    /// Get transitions by time range
    pub fn get_transitions_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&EvolutionTransition> {
        self.transitions
            .iter()
            .filter(|t| t.timestamp >= start && t.timestamp <= end)
            .collect()
    }

    /// Get transitions for version
    pub fn get_transitions_for_version(&self, version_id: &str) -> Vec<&EvolutionTransition> {
        self.transitions
            .iter()
            .filter(|t| {
                t.source_version_ids.contains(&version_id.to_string())
                    || t.target_version_ids.contains(&version_id.to_string())
            })
            .collect()
    }

    /// Get incoming transitions for version
    pub fn get_incoming_transitions(&self, version_id: &str) -> Vec<&EvolutionTransition> {
        self.transitions
            .iter()
            .filter(|t| t.target_version_ids.contains(&version_id.to_string()))
            .collect()
    }

    /// Get outgoing transitions for version
    pub fn get_outgoing_transitions(&self, version_id: &str) -> Vec<&EvolutionTransition> {
        self.transitions
            .iter()
            .filter(|t| t.source_version_ids.contains(&version_id.to_string()))
            .collect()
    }

    /// Get evolution path between versions
    pub fn get_evolution_path(
        &self,
        from_version_id: &str,
        to_version_id: &str,
    ) -> Result<Vec<&EvolutionTransition>> {
        // Check if versions exist
        if !self.version_ids.contains(from_version_id) {
            return Err(Error::NotFound(format!(
                "Version {from_version_id} not found"
            )));
        }

        if !self.version_ids.contains(to_version_id) {
            return Err(Error::NotFound(format!(
                "Version {to_version_id} not found"
            )));
        }

        // If same version, return empty path
        if from_version_id == to_version_id {
            return Ok(Vec::new());
        }

        // Build graph representation for path finding
        let mut graph: HashMap<String, Vec<(String, &EvolutionTransition)>> = HashMap::new();

        for transition in &self.transitions {
            for source_id in &transition.source_version_ids {
                for target_id in &transition.target_version_ids {
                    graph
                        .entry(source_id.clone())
                        .or_default()
                        .push((target_id.clone(), transition));
                }
            }
        }

        // Perform breadth-first search
        let mut queue = std::collections::VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map: HashMap<String, String> = HashMap::new();

        queue.push_back(from_version_id.to_string());
        visited.insert(from_version_id.to_string());

        while let Some(current) = queue.pop_front() {
            if current == to_version_id {
                // Found path, reconstruct it
                let mut path = Vec::new();
                let mut current_id = to_version_id;

                while let Some(parent_id) = parent_map.get(current_id) {
                    if let Some(transition) = self
                        .transitions
                        .iter()
                        .find(|t| &t.from_version_id == parent_id && t.to_version_id == current_id)
                    {
                        path.push(transition);
                    }
                    current_id = parent_id;
                    if current_id == from_version_id {
                        break;
                    }
                }

                path.reverse();
                return Ok(path);
            }

            // Explore neighbors
            for transition in &self.transitions {
                if transition.from_version_id == current
                    && !visited.contains(&transition.to_version_id)
                {
                    visited.insert(transition.to_version_id.clone());
                    parent_map.insert(transition.to_version_id.clone(), current.clone());
                    queue.push_back(transition.to_version_id.clone());
                }
            }
        }

        Err(Error::NotFound(format!(
            "No evolution path found from version {from_version_id} to {to_version_id}"
        )))
    }
}
