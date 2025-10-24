//! High-level schema constraint API
//!
//! This module provides the main API for creating and using schema-based constraints
//! for structured text generation. It combines JSON schema parsing with DFA-based
//! token validation for high-performance constrained generation.

use anyhow::{Context, Result as AnyResult};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokenizers::Tokenizer;

pub use super::schema_index::{
    IndexStats, SchemaConstraint, SchemaConstraintState, SchemaIndex, SchemaVocabulary, StateId,
    TokenId,
};
pub use super::schema_parser::{SchemaParser, regex_from_schema, regex_from_value};

/// Schema constraint types supported by the system
#[derive(Debug, Clone)]
pub enum SchemaType {
    /// JSON schema constraint from a schema value
    JsonSchema(Value),
    /// Rust type constraint using schemars
    RustType {
        /// JSON schema definition
        schema: Value,
        /// Name of the Rust type
        type_name: String,
    },
    /// Direct regex pattern constraint
    RegexPattern(String),
    /// Predefined constraint type
    Predefined(PredefinedSchema),
}

/// Predefined schema types for common use cases
#[derive(Debug, Clone)]
pub enum PredefinedSchema {
    /// Boolean values (true/false)
    Boolean,
    /// Null value
    Null,
    /// Integer with optional range
    Integer {
        /// Minimum allowed value
        min: Option<i64>,
        /// Maximum allowed value
        max: Option<i64>,
    },
    /// Number with optional range
    Number {
        /// Minimum allowed value
        min: Option<f64>,
        /// Maximum allowed value
        max: Option<f64>,
    },
    /// String with optional pattern and length constraints
    String {
        /// Regex pattern to match
        pattern: Option<String>,
        /// Minimum string length
        min_length: Option<usize>,
        /// Maximum string length
        max_length: Option<usize>,
    },
    /// Enum of string values
    StringEnum(Vec<String>),
    /// Array of items
    Array {
        /// Schema for array items
        items: Box<SchemaType>,
        /// Minimum array length
        min_items: Option<usize>,
        /// Maximum array length
        max_items: Option<usize>,
    },
    /// Object with properties
    Object {
        /// Object property definitions (name, schema)
        properties: Vec<(String, SchemaType)>,
        /// List of required property names
        required: Vec<String>,
        /// Whether additional properties are allowed
        additional_properties: bool,
    },
}

/// Schema constraint state tracking
#[derive(Debug, Clone)]
pub struct SchemaState {
    /// Internal DFA state
    inner: SchemaConstraintState,
    /// Schema type being validated
    schema_type: SchemaType,
    /// Optional debugging name
    name: Option<String>,
}

impl SchemaState {
    /// Create new schema state
    #[must_use]
    pub fn new(inner: SchemaConstraintState, schema_type: SchemaType) -> Self {
        Self {
            inner,
            schema_type,
            name: None,
        }
    }

    /// Create schema state with name
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Get inner constraint state
    #[must_use]
    pub const fn inner(&self) -> &SchemaConstraintState {
        &self.inner
    }

    /// Get mutable inner constraint state
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut SchemaConstraintState {
        &mut self.inner
    }

    /// Get schema type
    #[must_use]
    pub const fn schema_type(&self) -> &SchemaType {
        &self.schema_type
    }

    /// Get state name
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Check if generation is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Get current DFA state
    #[must_use]
    pub fn current_state(&self) -> StateId {
        self.inner.current_state()
    }

    /// Get number of tokens processed
    #[must_use]
    pub fn tokens_processed(&self) -> usize {
        self.inner.tokens_processed()
    }
}

/// High-level schema constraint builder and factory
#[derive(Debug)]
pub struct SchemaConstraintBuilder {
    /// Vocabulary for token validation
    vocabulary: Arc<SchemaVocabulary>,
    /// Optional whitespace pattern override
    whitespace_pattern: Option<String>,
    /// Maximum recursion depth for schema parsing
    max_recursion_depth: Option<usize>,
}

impl SchemaConstraintBuilder {
    /// Create new builder with vocabulary
    pub fn new(vocabulary: Arc<SchemaVocabulary>) -> Self {
        Self {
            vocabulary,
            whitespace_pattern: None,
            max_recursion_depth: None,
        }
    }

    /// Set custom whitespace pattern
    pub fn with_whitespace_pattern(mut self, pattern: String) -> Self {
        self.whitespace_pattern = Some(pattern);
        self
    }

    /// Set maximum recursion depth
    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = Some(depth);
        self
    }

    /// Build constraint from Rust type using schemars
    pub fn from_type<T>(&self) -> AnyResult<SchemaConstraint>
    where
        T: JsonSchema + Serialize,
    {
        let schema = schemars::schema_for!(T);
        let schema_value = serde_json::to_value(schema).context("Failed to serialize schema")?;

        self.from_schema_value(&schema_value)
            .context("Failed to create constraint from type")
    }

    /// Build constraint from JSON schema value
    pub fn from_schema_value(&self, schema: &Value) -> AnyResult<SchemaConstraint> {
        let regex_pattern = regex_from_value(
            schema,
            self.whitespace_pattern.as_deref(),
            self.max_recursion_depth,
        )
        .context("Failed to convert schema to regex")?;

        SchemaConstraint::new(&regex_pattern, self.vocabulary.clone(), false)
            .context("Failed to create schema constraint")
    }

    /// Build constraint from predefined schema type
    pub fn from_predefined(&self, predefined: &PredefinedSchema) -> AnyResult<SchemaConstraint> {
        let schema_value = self.predefined_to_schema(predefined)?;
        self.from_schema_value(&schema_value)
    }

    /// Build constraint directly from regex pattern
    pub fn from_regex(&self, pattern: &str) -> AnyResult<SchemaConstraint> {
        SchemaConstraint::new(pattern, self.vocabulary.clone(), false)
            .context("Failed to create regex constraint")
    }

    /// Convert predefined schema to JSON schema value
    fn predefined_to_schema(&self, predefined: &PredefinedSchema) -> AnyResult<Value> {
        let schema = match predefined {
            PredefinedSchema::Boolean => {
                serde_json::json!({"type": "boolean"})
            }
            PredefinedSchema::Null => {
                serde_json::json!({"type": "null"})
            }
            PredefinedSchema::Integer { min, max } => {
                let mut schema = serde_json::json!({"type": "integer"});
                if let Some(min_val) = min {
                    schema["minimum"] = serde_json::json!(min_val);
                }
                if let Some(max_val) = max {
                    schema["maximum"] = serde_json::json!(max_val);
                }
                schema
            }
            PredefinedSchema::Number { min, max } => {
                let mut schema = serde_json::json!({"type": "number"});
                if let Some(min_val) = min {
                    schema["minimum"] = serde_json::json!(min_val);
                }
                if let Some(max_val) = max {
                    schema["maximum"] = serde_json::json!(max_val);
                }
                schema
            }
            PredefinedSchema::String {
                pattern,
                min_length,
                max_length,
            } => {
                let mut schema = serde_json::json!({"type": "string"});
                if let Some(pat) = pattern {
                    schema["pattern"] = serde_json::json!(pat);
                }
                if let Some(min_len) = min_length {
                    schema["minLength"] = serde_json::json!(min_len);
                }
                if let Some(max_len) = max_length {
                    schema["maxLength"] = serde_json::json!(max_len);
                }
                schema
            }
            PredefinedSchema::StringEnum(values) => {
                serde_json::json!({"enum": values})
            }
            PredefinedSchema::Array {
                items,
                min_items,
                max_items,
            } => {
                let items_schema = self.schema_type_to_value(items)?;
                let mut schema = serde_json::json!({
                    "type": "array",
                    "items": items_schema
                });
                if let Some(min_items_val) = min_items {
                    schema["minItems"] = serde_json::json!(min_items_val);
                }
                if let Some(max_items_val) = max_items {
                    schema["maxItems"] = serde_json::json!(max_items_val);
                }
                schema
            }
            PredefinedSchema::Object {
                properties,
                required,
                additional_properties,
            } => {
                let mut props = serde_json::Map::new();
                for (prop_name, prop_type) in properties {
                    let prop_schema = self.schema_type_to_value(prop_type)?;
                    props.insert(prop_name.clone(), prop_schema);
                }

                let mut schema = serde_json::json!({
                    "type": "object",
                    "properties": props,
                    "additionalProperties": additional_properties
                });

                if !required.is_empty() {
                    schema["required"] = serde_json::json!(required);
                }

                schema
            }
        };

        Ok(schema)
    }

    /// Convert `SchemaType` to JSON schema value
    fn schema_type_to_value(&self, schema_type: &SchemaType) -> AnyResult<Value> {
        match schema_type {
            SchemaType::JsonSchema(value) => Ok(value.clone()),
            SchemaType::RustType { schema, .. } => Ok(schema.clone()),
            SchemaType::RegexPattern(pattern) => {
                // Create a string schema with the pattern
                Ok(serde_json::json!({
                    "type": "string",
                    "pattern": pattern
                }))
            }
            SchemaType::Predefined(predefined) => self.predefined_to_schema(predefined),
        }
    }
}

/// Convenience functions for creating common schema constraints
pub mod presets {
    use super::*;

    /// Create a boolean constraint (true/false)
    pub fn boolean(vocabulary: Arc<SchemaVocabulary>) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary).from_predefined(&PredefinedSchema::Boolean)
    }

    /// Create a null constraint
    pub fn null(vocabulary: Arc<SchemaVocabulary>) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary).from_predefined(&PredefinedSchema::Null)
    }

    /// Create an integer constraint with optional range
    pub fn integer(
        vocabulary: Arc<SchemaVocabulary>,
        min: Option<i64>,
        max: Option<i64>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary)
            .from_predefined(&PredefinedSchema::Integer { min, max })
    }

    /// Create a number constraint with optional range
    pub fn number(
        vocabulary: Arc<SchemaVocabulary>,
        min: Option<f64>,
        max: Option<f64>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary)
            .from_predefined(&PredefinedSchema::Number { min, max })
    }

    /// Create a string constraint with optional pattern and length
    pub fn string(
        vocabulary: Arc<SchemaVocabulary>,
        pattern: Option<String>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary).from_predefined(&PredefinedSchema::String {
            pattern,
            min_length,
            max_length,
        })
    }

    /// Create an enum constraint from string values
    pub fn string_enum(
        vocabulary: Arc<SchemaVocabulary>,
        values: Vec<String>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary)
            .from_predefined(&PredefinedSchema::StringEnum(values))
    }

    /// Create an array constraint
    pub fn array(
        vocabulary: Arc<SchemaVocabulary>,
        items: SchemaType,
        min_items: Option<usize>,
        max_items: Option<usize>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary).from_predefined(&PredefinedSchema::Array {
            items: Box::new(items),
            min_items,
            max_items,
        })
    }

    /// Create a simple object constraint
    pub fn simple_object(
        vocabulary: Arc<SchemaVocabulary>,
        properties: Vec<(String, SchemaType)>,
        required: Vec<String>,
    ) -> AnyResult<SchemaConstraint> {
        SchemaConstraintBuilder::new(vocabulary).from_predefined(&PredefinedSchema::Object {
            properties,
            required,
            additional_properties: false,
        })
    }

    /// Create constraint from Rust type
    pub fn from_rust_type<T>(vocabulary: Arc<SchemaVocabulary>) -> AnyResult<SchemaConstraint>
    where
        T: JsonSchema + Serialize,
    {
        SchemaConstraintBuilder::new(vocabulary).from_type::<T>()
    }
}

/// Factory for creating schema vocabularies and constraints
#[derive(Debug)]
pub struct SchemaFactory {
    /// Cached vocabulary
    vocabulary: Option<Arc<SchemaVocabulary>>,
}

impl SchemaFactory {
    /// Create new factory
    pub fn new() -> Self {
        Self { vocabulary: None }
    }

    /// Get or create vocabulary from tokenizer
    pub fn get_vocabulary(&mut self, tokenizer: &Tokenizer) -> AnyResult<Arc<SchemaVocabulary>> {
        if let Some(ref vocab) = self.vocabulary {
            Ok(vocab.clone())
        } else {
            let vocab = Arc::new(SchemaVocabulary::from_tokenizer(tokenizer)?);
            self.vocabulary = Some(vocab.clone());
            Ok(vocab)
        }
    }

    /// Create constraint builder
    pub fn builder(&mut self, tokenizer: &Tokenizer) -> AnyResult<SchemaConstraintBuilder> {
        let vocabulary = self.get_vocabulary(tokenizer)?;
        Ok(SchemaConstraintBuilder::new(vocabulary))
    }

    /// Create constraint from Rust type
    pub fn constraint_from_type<T>(&mut self, tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint>
    where
        T: JsonSchema + Serialize,
    {
        self.builder(tokenizer)?.from_type::<T>()
    }

    /// Create constraint from JSON schema
    pub fn constraint_from_schema(
        &mut self,
        tokenizer: &Tokenizer,
        schema: &Value,
    ) -> AnyResult<SchemaConstraint> {
        self.builder(tokenizer)?.from_schema_value(schema)
    }

    /// Create constraint from predefined type
    pub fn constraint_from_predefined(
        &mut self,
        tokenizer: &Tokenizer,
        predefined: &PredefinedSchema,
    ) -> AnyResult<SchemaConstraint> {
        self.builder(tokenizer)?.from_predefined(predefined)
    }
}

impl Default for SchemaFactory {
    fn default() -> Self {
        Self::new()
    }
}
