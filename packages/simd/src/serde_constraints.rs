//! Serde Type Constraint API for Structured Generation
//!
//! This module provides user-friendly functions for creating JSON schema constraints
//! from Rust serde types, enabling structured generation that guarantees output
//! matches specific type definitions.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use serde::{Deserialize, Serialize};
//! use schemars::JsonSchema;
//! use kodegen_simd::serde_constraints::constraint_for_type;
//!
//! #[derive(Serialize, Deserialize, JsonSchema)]
//! struct User {
//!     name: String,
//!     age: u32,
//!     email: Option<String>,
//! }
//!
//! let constraint = constraint_for_type::<User>(tokenizer)?;
//! // Use constraint in generation to guarantee valid User JSON
//! ```

use anyhow::{Context, Result as AnyResult};
use schemars::{JsonSchema, schema_for};
use std::sync::Arc;
use tokenizers::Tokenizer;

use crate::logits::constraints::{
    JsonConstraint, SchemaConstraint, SchemaVocabulary, regex_from_schema, regex_from_value,
};

/// Create a JSON constraint from a serde type with `JsonSchema` derive
///
/// This function generates a JSON schema from the given type and creates
/// a constraint that ensures generated JSON conforms to that type structure.
///
/// # Type Parameters
/// * `T` - Type implementing `Serialize`, `Deserialize`, and `JsonSchema`
///
/// # Arguments
/// * `tokenizer` - Tokenizer for token-to-text conversion
///
/// # Returns
/// * `Ok(SchemaConstraint)` - Constraint that validates against the type schema
/// * `Err(anyhow::Error)` - If schema generation or constraint creation fails
///
/// # Example
/// ```rust,no_run
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
/// use kodegen_simd::serde_constraints::constraint_for_type;
///
/// #[derive(Serialize, Deserialize, JsonSchema)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let constraint = constraint_for_type::<Person>(&tokenizer)?;
/// ```
pub fn constraint_for_type<T>(tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint>
where
    T: JsonSchema + serde::Serialize,
{
    let vocabulary = Arc::new(SchemaVocabulary::from_tokenizer(tokenizer)?);
    let regex_pattern = regex_from_schema::<T>().context("Failed to generate regex from type")?;

    SchemaConstraint::new(&regex_pattern, vocabulary, false)
        .context("Failed to create schema constraint from type")
}

/// Create a JSON constraint from a JSON schema string
///
/// This function parses a JSON schema string and creates a constraint
/// that validates generated tokens against the schema structure.
///
/// # Arguments
/// * `schema_json` - JSON schema as a string
/// * `tokenizer` - Tokenizer for token-to-text conversion
///
/// # Returns
/// * `Ok(SchemaConstraint)` - Constraint that validates against the schema
/// * `Err(anyhow::Error)` - If schema parsing or constraint creation fails
///
/// # Example
/// ```rust,no_run
/// use kodegen_simd::serde_constraints::constraint_for_schema;
///
/// let schema = r#"{
///     "type": "object",
///     "properties": {
///         "name": { "type": "string" },
///         "age": { "type": "integer" }
///     },
///     "required": ["name", "age"]
/// }"#;
///
/// let constraint = constraint_for_schema(schema, &tokenizer)?;
/// ```
pub fn constraint_for_schema(
    schema_json: &str,
    tokenizer: &Tokenizer,
) -> AnyResult<SchemaConstraint> {
    let schema: serde_json::Value =
        serde_json::from_str(schema_json).context("Failed to parse JSON schema")?;

    let vocabulary = Arc::new(SchemaVocabulary::from_tokenizer(tokenizer)?);
    let regex_pattern =
        regex_from_value(&schema, None, None).context("Failed to generate regex from schema")?;

    SchemaConstraint::new(&regex_pattern, vocabulary, false)
        .context("Failed to create schema constraint from JSON")
}

/// Create a basic JSON syntax constraint (no schema validation)
///
/// This function creates a constraint that only validates JSON syntax
/// without enforcing any specific structure. Useful for generating
/// valid JSON without type constraints.
///
/// # Arguments
/// * `tokenizer` - Tokenizer for token-to-text conversion
///
/// # Returns
/// * `Ok(JsonConstraint)` - Constraint that validates JSON syntax
/// * `Err(anyhow::Error)` - If constraint creation fails
///
/// # Example
/// ```rust,no_run
/// use kodegen_simd::serde_constraints::basic_json_constraint;
///
/// let constraint = basic_json_constraint(&tokenizer)?;
/// // Ensures valid JSON syntax but allows any structure
/// ```
pub fn basic_json_constraint(tokenizer: &Tokenizer) -> AnyResult<JsonConstraint<'_>> {
    JsonConstraint::new(tokenizer).context("Failed to create basic JSON constraint")
}

/// Builder for creating complex constraints with multiple options
///
/// Provides a fluent API for configuring constraint behavior, including
/// validation strictness, error handling, and performance options.
///
/// # Example
/// ```rust,no_run
/// use kodegen_simd::serde_constraints::ConstraintBuilder;
///
/// let constraint = ConstraintBuilder::new(&tokenizer)
///     .with_type::<MyType>()
///     .with_strict_validation(true)
///     .with_partial_generation(false)
///     .build()?;
/// ```
pub struct ConstraintBuilder<'a> {
    tokenizer: &'a Tokenizer,
    schema: Option<serde_json::Value>,
    strict_validation: bool,
    allow_partial: bool,
}

impl<'a> ConstraintBuilder<'a> {
    /// Create a new constraint builder
    ///
    /// # Arguments
    /// * `tokenizer` - Tokenizer for token-to-text conversion
    pub fn new(tokenizer: &'a Tokenizer) -> Self {
        Self {
            tokenizer,
            schema: None,
            strict_validation: true,
            allow_partial: false,
        }
    }

    /// Set the constraint to validate against a specific serde type
    ///
    /// # Type Parameters
    /// * `T` - Type implementing `JsonSchema`
    pub fn with_type<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        let schema = schema_for!(T);
        if let Ok(schema_value) = serde_json::to_value(&schema) {
            self.schema = Some(schema_value);
        }
        self
    }

    /// Set the constraint to validate against a JSON schema string
    ///
    /// # Arguments
    /// * `schema_json` - JSON schema as a string
    pub fn with_schema(mut self, schema_json: &str) -> AnyResult<Self> {
        let schema: serde_json::Value =
            serde_json::from_str(schema_json).context("Failed to parse JSON schema in builder")?;
        self.schema = Some(schema);
        Ok(self)
    }

    /// Enable or disable strict validation
    ///
    /// When enabled (default), all schema constraints are enforced strictly.
    /// When disabled, some validation errors may be treated as warnings.
    ///
    /// # Arguments
    /// * `strict` - Whether to enable strict validation
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.strict_validation = strict;
        self
    }

    /// Enable or disable partial generation
    ///
    /// When enabled, allows generation to complete even if the schema
    /// is not fully satisfied. When disabled (default), generation
    /// continues until schema requirements are met.
    ///
    /// # Arguments
    /// * `allow_partial` - Whether to allow partial generation
    pub fn with_partial_generation(mut self, allow_partial: bool) -> Self {
        self.allow_partial = allow_partial;
        self
    }

    /// Build the constraint with current configuration
    ///
    /// # Returns
    /// * `Ok(SchemaConstraint)` - Configured constraint ready for use
    /// * `Err(anyhow::Error)` - If constraint creation fails
    pub fn build(self) -> AnyResult<SchemaConstraint> {
        let schema = self
            .schema
            .ok_or_else(|| anyhow::anyhow!("No schema specified in constraint builder"))?;

        let vocabulary = Arc::new(SchemaVocabulary::from_tokenizer(self.tokenizer)?);

        // Apply configuration options to regex generation
        let max_recursion = if self.strict_validation { 5 } else { 3 };
        let regex_pattern = regex_from_value(&schema, None, Some(max_recursion))
            .context("Failed to generate regex from schema")?;

        let constraint = SchemaConstraint::new(&regex_pattern, vocabulary, self.allow_partial)
            .context("Failed to build schema constraint")?;

        Ok(constraint)
    }

    /// Build a basic JSON constraint instead of schema constraint
    ///
    /// This creates a constraint that only validates JSON syntax,
    /// ignoring any schema that may have been set.
    ///
    /// # Returns
    /// * `Ok(JsonConstraint)` - JSON syntax constraint
    /// * `Err(anyhow::Error)` - If constraint creation fails
    pub fn build_json_only(self) -> AnyResult<JsonConstraint<'a>> {
        JsonConstraint::new(self.tokenizer).context("Failed to build JSON constraint")
    }
}

/// Convenience function for common constraint patterns
pub mod presets {
    use super::*;

    /// Create constraint for generating JSON objects with string keys and any values
    ///
    /// Useful for generating generic JSON objects without specific structure requirements.
    ///
    /// # Arguments
    /// * `tokenizer` - Tokenizer for token-to-text conversion
    pub fn object_with_string_keys(tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint> {
        let schema_json = r#"{
            "type": "object",
            "additionalProperties": true
        }"#;
        constraint_for_schema(schema_json, tokenizer)
    }

    /// Create constraint for generating JSON arrays of any values
    ///
    /// Useful for generating JSON arrays without specific item type requirements.
    ///
    /// # Arguments
    /// * `tokenizer` - Tokenizer for token-to-text conversion
    pub fn array_of_any(tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint> {
        let schema_json = r#"{
            "type": "array",
            "items": {}
        }"#;
        constraint_for_schema(schema_json, tokenizer)
    }

    /// Create constraint for generating JSON arrays of strings
    ///
    /// # Arguments
    /// * `tokenizer` - Tokenizer for token-to-text conversion
    pub fn array_of_strings(tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint> {
        let schema_json = r#"{
            "type": "array",
            "items": { "type": "string" }
        }"#;
        constraint_for_schema(schema_json, tokenizer)
    }

    /// Create constraint for generating JSON arrays of integers
    ///
    /// # Arguments
    /// * `tokenizer` - Tokenizer for token-to-text conversion
    pub fn array_of_integers(tokenizer: &Tokenizer) -> AnyResult<SchemaConstraint> {
        let schema_json = r#"{
            "type": "array", 
            "items": { "type": "integer" }
        }"#;
        constraint_for_schema(schema_json, tokenizer)
    }
}
