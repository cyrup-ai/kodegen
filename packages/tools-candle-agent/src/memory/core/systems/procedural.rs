// src/memory/procedural.rs
//! Procedural memory implementation for Rust-mem0.
//!
//! This module provides a specialized memory type for storing action sequences
//! and procedural knowledge, with support for steps, conditions, and execution.

use std::collections::HashMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use surrealdb::{
    Value,
    value::{from_value, to_value},
};

use crate::memory::graph::graph_db::{GraphError, Result};
use crate::memory::primitives::types::{BaseMemory, MemoryContent, MemoryTypeEnum};

/// Step status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently executing
    Executing,
    /// Step has completed successfully
    Completed,
    /// Step has failed
    Failed,
    /// Step has been skipped
    Skipped,
}

impl StepStatus {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::Executing => "executing",
            StepStatus::Completed => "completed",
            StepStatus::Failed => "failed",
            StepStatus::Skipped => "skipped",
        }
    }

    /// Parse from string
    pub fn parse_from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(StepStatus::Pending),
            "executing" => Ok(StepStatus::Executing),
            "completed" => Ok(StepStatus::Completed),
            "failed" => Ok(StepStatus::Failed),
            "skipped" => Ok(StepStatus::Skipped),
            _ => Err(GraphError::ValidationError(format!(
                "Invalid step status: {s}"
            ))),
        }
    }
}

/// Condition type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    /// Condition is a prerequisite
    Prerequisite,
    /// Condition is a postcondition
    Postcondition,
    /// Condition is an invariant
    Invariant,
}

impl ConditionType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionType::Prerequisite => "prerequisite",
            ConditionType::Postcondition => "postcondition",
            ConditionType::Invariant => "invariant",
        }
    }

    /// Parse from string
    pub fn parse_from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "prerequisite" => Ok(ConditionType::Prerequisite),
            "postcondition" => Ok(ConditionType::Postcondition),
            "invariant" => Ok(ConditionType::Invariant),
            _ => Err(GraphError::ValidationError(format!(
                "Invalid condition type: {s}"
            ))),
        }
    }
}

/// Condition struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Condition ID
    pub id: String,

    /// Condition type
    pub condition_type: ConditionType,

    /// Condition description
    pub description: String,

    /// Condition expression
    pub expression: Value,

    /// Is the condition required
    pub required: bool,
}

impl Condition {
    /// Create a new condition
    pub fn new(
        id: &str,
        condition_type: ConditionType,
        description: &str,
        expression: Value,
        required: bool,
    ) -> Self {
        Self {
            id: id.to_string(),
            condition_type,
            description: description.to_string(),
            expression,
            required,
        }
    }

    /// Create a new prerequisite
    pub fn prerequisite(id: &str, description: &str, expression: Value, required: bool) -> Self {
        Self::new(
            id,
            ConditionType::Prerequisite,
            description,
            expression,
            required,
        )
    }

    /// Create a new postcondition
    pub fn postcondition(id: &str, description: &str, expression: Value, required: bool) -> Self {
        Self::new(
            id,
            ConditionType::Postcondition,
            description,
            expression,
            required,
        )
    }

    /// Create a new invariant
    pub fn invariant(id: &str, description: &str, expression: Value, required: bool) -> Self {
        Self::new(
            id,
            ConditionType::Invariant,
            description,
            expression,
            required,
        )
    }

    /// Convert to Value
    pub fn to_value(&self) -> Value {
        let mut obj = BTreeMap::new();
        obj.insert(
            "id".to_string(),
            to_value(self.id.clone()).unwrap_or_default(),
        );
        obj.insert(
            "condition_type".to_string(),
            to_value(self.condition_type.as_str().to_string()).unwrap_or_default(),
        );
        obj.insert(
            "description".to_string(),
            to_value(self.description.clone()).unwrap_or_default(),
        );
        obj.insert("expression".to_string(), self.expression.clone());
        obj.insert(
            "required".to_string(),
            to_value(self.required).unwrap_or_default(),
        );
        to_value(obj).unwrap_or_default()
    }

    /// Create from Value
    pub fn from_value(value: &Value) -> Result<Self> {
        let obj_map: BTreeMap<String, Value> = from_value(value.clone()).map_err(|e| {
            GraphError::ConversionError(format!("Failed to deserialize as object: {}", e))
        })?;

        let id: String = obj_map
            .get("id")
            .ok_or_else(|| GraphError::ConversionError("Missing id in condition".to_string()))
            .and_then(|v| {
                from_value(v.clone())
                    .map_err(|e| GraphError::ConversionError(format!("Invalid id format: {}", e)))
            })?;

        let condition_type_str: String = obj_map
            .get("condition_type")
            .ok_or_else(|| {
                GraphError::ConversionError("Missing condition_type in condition".to_string())
            })
            .and_then(|v| {
                from_value(v.clone()).map_err(|e| {
                    GraphError::ConversionError(format!("Invalid condition_type format: {}", e))
                })
            })?;

        let condition_type = ConditionType::parse_from_str(&condition_type_str)?;

        let description: String = obj_map
            .get("description")
            .ok_or_else(|| {
                GraphError::ConversionError("Missing description in condition".to_string())
            })
            .and_then(|v| {
                from_value(v.clone()).map_err(|e| {
                    GraphError::ConversionError(format!("Invalid description format: {}", e))
                })
            })?;

        let expression = obj_map
            .get("expression")
            .ok_or_else(|| {
                GraphError::ConversionError("Missing expression in condition".to_string())
            })?
            .clone();

        let required: bool = obj_map
            .get("required")
            .and_then(|v| from_value(v.clone()).ok())
            .unwrap_or(false);

        Ok(Self {
            id,
            condition_type,
            description,
            expression,
            required,
        })
    }
}

/// Step struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step ID
    pub id: String,

    /// Step name
    pub name: String,

    /// Step description
    pub description: String,

    /// Step order
    pub order: u32,

    /// Step action
    pub action: Value,

    /// Step status
    pub status: StepStatus,

    /// Step conditions
    pub conditions: Vec<Condition>,

    /// Step dependencies
    pub dependencies: Vec<String>,

    /// Step result
    pub result: Option<Value>,

    /// Step error
    pub error: Option<String>,

    /// Step metadata
    pub metadata: HashMap<String, Value>,
}

impl Step {
    /// Create a new step
    pub fn new(id: &str, name: &str, description: &str, order: u32, action: Value) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            order,
            action,
            status: StepStatus::Pending,
            conditions: Vec::new(),
            dependencies: Vec::new(),
            result: None,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a condition
    #[must_use]
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add a dependency
    #[must_use]
    pub fn with_dependency(mut self, dependency_id: &str) -> Self {
        self.dependencies.push(dependency_id.to_string());
        self
    }

    /// Set status
    #[must_use]
    pub fn with_status(mut self, status: StepStatus) -> Self {
        self.status = status;
        self
    }

    /// Set result
    #[must_use]
    pub fn with_result(mut self, result: Value) -> Self {
        self.result = Some(result);
        self
    }

    /// Set error
    #[must_use]
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Convert to Value
    pub fn to_value(&self) -> Value {
        let mut obj = BTreeMap::new();
        obj.insert(
            "id".to_string(),
            to_value(self.id.clone()).unwrap_or_default(),
        );
        obj.insert(
            "name".to_string(),
            to_value(self.name.clone()).unwrap_or_default(),
        );
        obj.insert(
            "description".to_string(),
            to_value(self.description.clone()).unwrap_or_default(),
        );
        obj.insert(
            "order".to_string(),
            to_value(self.order).unwrap_or_default(),
        );
        obj.insert("action".to_string(), self.action.clone());
        obj.insert(
            "status".to_string(),
            to_value(self.status.as_str().to_string()).unwrap_or_default(),
        );

        let conditions: Vec<Value> = self.conditions.iter().map(|c| c.to_value()).collect();
        obj.insert(
            "conditions".to_string(),
            to_value(conditions).unwrap_or_default(),
        );

        let dependencies: Vec<String> = self.dependencies.clone();
        obj.insert(
            "dependencies".to_string(),
            to_value(dependencies).unwrap_or_default(),
        );

        if let Some(result) = &self.result {
            obj.insert("result".to_string(), result.clone());
        }

        if let Some(error) = &self.error {
            obj.insert(
                "error".to_string(),
                to_value(error.clone()).unwrap_or_default(),
            );
        }

        obj.insert(
            "metadata".to_string(),
            to_value(self.metadata.clone()).unwrap_or_default(),
        );

        to_value(obj).unwrap_or_default()
    }

    /// Create from Value
    pub fn from_value(value: &Value) -> Result<Self> {
        let obj_map: BTreeMap<String, Value> = from_value(value.clone()).map_err(|e| {
            GraphError::ConversionError(format!("Failed to deserialize as object: {}", e))
        })?;

        let id: String = obj_map
            .get("id")
            .ok_or_else(|| GraphError::ConversionError("Missing id in step".to_string()))
            .and_then(|v| {
                from_value(v.clone())
                    .map_err(|e| GraphError::ConversionError(format!("Invalid id format: {}", e)))
            })?;

        let name: String = obj_map
            .get("name")
            .ok_or_else(|| GraphError::ConversionError("Missing name in step".to_string()))
            .and_then(|v| {
                from_value(v.clone())
                    .map_err(|e| GraphError::ConversionError(format!("Invalid name format: {}", e)))
            })?;

        let description: String = obj_map
            .get("description")
            .ok_or_else(|| GraphError::ConversionError("Missing description in step".to_string()))
            .and_then(|v| {
                from_value(v.clone()).map_err(|e| {
                    GraphError::ConversionError(format!("Invalid description format: {}", e))
                })
            })?;

        let order: u32 = obj_map
            .get("order")
            .ok_or_else(|| GraphError::ConversionError("Missing order in step".to_string()))
            .and_then(|v| {
                from_value(v.clone()).map_err(|e| {
                    GraphError::ConversionError(format!("Invalid order format: {}", e))
                })
            })?;

        let action = obj_map
            .get("action")
            .ok_or_else(|| GraphError::ConversionError("Missing action in step".to_string()))?
            .clone();

        let status_str: String = obj_map
            .get("status")
            .and_then(|v| from_value::<String>(v.clone()).ok())
            .unwrap_or_else(|| "Pending".to_string());
        let status = StepStatus::parse_from_str(&status_str)?;

        let conditions: Vec<Condition> = obj_map
            .get("conditions")
            .and_then(|v| from_value::<Vec<Value>>(v.clone()).ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| Condition::from_value(&v).ok())
            .collect();

        let dependencies: Vec<String> = obj_map
            .get("dependencies")
            .and_then(|v| from_value(v.clone()).ok())
            .unwrap_or_default();

        let result = obj_map.get("result").cloned();

        let error: Option<String> = obj_map
            .get("error")
            .and_then(|v| from_value(v.clone()).ok());

        let metadata: HashMap<String, Value> = obj_map
            .get("metadata")
            .and_then(|v| from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            id,
            name,
            description,
            order,
            action,
            status,
            conditions,
            dependencies,
            result,
            error,
            metadata,
        })
    }
}

/// Procedural memory struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralMemory {
    /// Base memory
    pub base: BaseMemory,

    /// Procedure name
    pub name: String,

    /// Procedure description
    pub description: String,

    /// Procedure steps
    pub steps: Vec<Step>,

    /// Procedure conditions
    pub conditions: Vec<Condition>,

    /// Current step index
    pub current_step: Option<usize>,

    /// Execution status
    pub status: StepStatus,

    /// Execution result
    pub result: Option<Value>,

    /// Execution error
    pub error: Option<String>,
}

impl ProceduralMemory {
    /// Create a new procedural memory
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        let content = MemoryContent::json(serde_json::Value::Object(serde_json::Map::new()));
        let base = BaseMemory::new(
            id,
            "Procedural Memory",
            "Auto-generated procedural memory",
            MemoryTypeEnum::Procedural,
            content,
        );

        Self {
            base,
            name: name.to_string(),
            description: description.to_string(),
            steps: Vec::new(),
            conditions: Vec::new(),
            current_step: None,
            status: StepStatus::Pending,
            result: None,
            error: None,
        }
    }

    /// Add a step
    pub fn add_step(&mut self, step: Step) {
        self.steps.push(step);
        // Sort steps by order
        self.steps.sort_by_key(|s| s.order);
    }

    /// Add a condition
    pub fn add_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }

    /// Get step by ID
    pub fn get_step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Get mutable step by ID
    pub fn get_step_mut(&mut self, id: &str) -> Option<&mut Step> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    /// Get condition by ID
    pub fn get_condition(&self, id: &str) -> Option<&Condition> {
        self.conditions.iter().find(|c| c.id == id)
    }

    /// Get mutable condition by ID
    pub fn get_condition_mut(&mut self, id: &str) -> Option<&mut Condition> {
        self.conditions.iter_mut().find(|c| c.id == id)
    }

    /// Get current step
    pub fn current_step(&self) -> Option<&Step> {
        self.current_step.and_then(|i| self.steps.get(i))
    }

    /// Get next step
    pub fn next_step(&self) -> Option<&Step> {
        match self.current_step {
            Some(i) if i + 1 < self.steps.len() => self.steps.get(i + 1),
            None if !self.steps.is_empty() => self.steps.first(),
            _ => None,
        }
    }
}
