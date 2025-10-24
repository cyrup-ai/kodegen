//! Processing context for SIMD-accelerated operations

use std::time::Instant;

use crate::logits::constraints::{
    GenerationConstraint, JsonConstraint, PredefinedSchema, SchemaConstraint,
    SchemaConstraintBuilder, SchemaConstraintState, SchemaVocabulary, json::JsonState,
};

/// Context for processing logits with token history and configuration
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    /// Current temperature for sampling (can be overridden per-call)
    pub temperature: f32,

    /// Current top-k value (can be overridden per-call)
    pub top_k: Option<usize>,

    /// Current top-p value (can be overridden per-call)
    pub top_p: Option<f32>,

    /// History of token IDs in the current sequence
    pub token_history: Vec<u32>,

    /// Optional timestamp when processing started
    pub start_time: Option<Instant>,

    /// Optional maximum number of new tokens to generate
    pub max_new_tokens: Option<usize>,

    /// Optional stop tokens that will halt generation if encountered
    pub stop_tokens: Vec<u32>,

    /// Optional JSON constraint for structured output
    pub json_constraint: Option<JsonConstraint<'static>>,

    /// Current JSON constraint state
    pub json_constraint_state: Option<JsonState>,

    /// Optional schema constraint for typed structured output
    pub schema_constraint: Option<SchemaConstraint>,

    /// Current schema constraint state
    pub schema_constraint_state: Option<SchemaConstraintState>,
}

impl Default for ProcessingContext {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            top_k: None,
            top_p: None,
            token_history: Vec::new(),
            start_time: None,
            max_new_tokens: None,
            stop_tokens: Vec::new(),
            json_constraint: None,
            json_constraint_state: None,
            schema_constraint: None,
            schema_constraint_state: None,
        }
    }
}

impl ProcessingContext {
    /// Create a new processing context with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the token history as a slice
    #[must_use]
    pub fn token_history(&self) -> &[u32] {
        &self.token_history
    }

    /// Set the temperature
    #[must_use]
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the top-k value
    #[must_use]
    pub const fn with_top_k(mut self, top_k: Option<usize>) -> Self {
        self.top_k = top_k;
        self
    }

    /// Set the top-p value
    #[must_use]
    pub const fn with_top_p(mut self, top_p: Option<f32>) -> Self {
        self.top_p = top_p;
        self
    }

    /// Set the token history
    #[must_use]
    pub fn with_token_history(mut self, history: Vec<u32>) -> Self {
        self.token_history = history;
        self
    }

    /// Add tokens to the history
    pub fn extend_history(&mut self, tokens: &[u32]) {
        self.token_history.extend_from_slice(tokens);
    }

    /// Get the number of tokens in history
    #[must_use]
    pub const fn history_len(&self) -> usize {
        self.token_history.len()
    }

    /// Check if processing should stop based on stop tokens or max tokens
    #[must_use]
    pub fn should_stop(&self, new_token: u32, generated_count: usize) -> bool {
        // Check for stop tokens
        if self.stop_tokens.contains(&new_token) {
            return true;
        }

        // Check max new tokens
        if let Some(max_tokens) = self.max_new_tokens
            && generated_count >= max_tokens
        {
            return true;
        }

        // Check for max sequence length (safety check)
        if self.token_history.len() > 4096 {
            // Arbitrary large number
            return true;
        }

        false
    }

    /// Start timing the processing
    pub fn start_timer(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Get elapsed time since timer was started
    #[must_use]
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Set a JSON constraint for structured output
    #[must_use]
    pub fn with_json_constraint(mut self, constraint: JsonConstraint<'static>) -> Self {
        let initial_state = constraint.new_state();
        self.json_constraint = Some(constraint);
        self.json_constraint_state = Some(initial_state);
        self
    }

    /// Check if a token is valid according to JSON constraints
    ///
    /// # Errors
    ///
    /// Returns an error if constraint validation fails
    pub fn is_token_valid(&self, token: u32) -> anyhow::Result<bool> {
        if let (Some(constraint), Some(state)) =
            (&self.json_constraint, &self.json_constraint_state)
        {
            constraint.try_next(state, token)
        } else {
            Ok(true) // No constraints, all tokens valid
        }
    }

    /// Update JSON constraint state after token generation
    ///
    /// # Errors
    ///
    /// Returns an error if constraint state update fails
    pub fn update_constraint_state(&mut self, token: u32) -> anyhow::Result<bool> {
        if let (Some(constraint), Some(state)) =
            (&self.json_constraint, &mut self.json_constraint_state)
        {
            constraint.update(state, token)
        } else {
            Ok(true) // No constraints to update
        }
    }

    /// Check if JSON constraint-based generation is complete
    #[must_use]
    pub fn is_constraint_done(&self) -> bool {
        if let (Some(constraint), Some(state)) =
            (&self.json_constraint, &self.json_constraint_state)
        {
            constraint.is_done(state)
        } else {
            false // No constraints, generation not complete
        }
    }

    /// Get deterministic token sequence from JSON constraints
    ///
    /// # Errors
    ///
    /// Returns an error if sequence generation fails
    pub fn get_deterministic_sequence(&self) -> anyhow::Result<Vec<u32>> {
        if let (Some(constraint), Some(state)) =
            (&self.json_constraint, &self.json_constraint_state)
        {
            constraint.get_deterministic_sequence(state)
        } else {
            Ok(Vec::new()) // No constraints, no forced sequence
        }
    }

    /// Set a schema constraint from constraint instance
    #[must_use]
    pub fn with_schema_constraint(mut self, constraint: SchemaConstraint) -> Self {
        let initial_state = constraint.new_state();
        self.schema_constraint = Some(constraint);
        self.schema_constraint_state = Some(initial_state);
        self
    }

    /// Set a schema constraint from JSON schema value
    ///
    /// # Errors
    ///
    /// Returns an error if schema constraint creation fails
    pub fn with_schema_constraint_from_value(
        mut self,
        schema: &serde_json::Value,
        vocabulary: std::sync::Arc<SchemaVocabulary>,
    ) -> anyhow::Result<Self> {
        let builder = SchemaConstraintBuilder::new(vocabulary);
        let constraint = builder.from_schema_value(schema)?;
        let initial_state = constraint.new_state();
        self.schema_constraint = Some(constraint);
        self.schema_constraint_state = Some(initial_state);
        Ok(self)
    }

    /// Set a schema constraint from Serde type
    ///
    /// # Errors
    ///
    /// Returns an error if schema constraint creation fails
    pub fn with_schema_constraint_from_type<T>(
        mut self,
        vocabulary: std::sync::Arc<SchemaVocabulary>,
    ) -> anyhow::Result<Self>
    where
        T: schemars::JsonSchema + serde::Serialize,
    {
        let builder = SchemaConstraintBuilder::new(vocabulary);
        let constraint = builder.from_type::<T>()?;
        let initial_state = constraint.new_state();
        self.schema_constraint = Some(constraint);
        self.schema_constraint_state = Some(initial_state);
        Ok(self)
    }

    /// Set a schema constraint from predefined schema type
    pub fn with_schema_constraint_from_predefined(
        mut self,
        predefined: &PredefinedSchema,
        vocabulary: std::sync::Arc<SchemaVocabulary>,
    ) -> anyhow::Result<Self> {
        let builder = SchemaConstraintBuilder::new(vocabulary);
        let constraint = builder.from_predefined(predefined)?;
        let initial_state = constraint.new_state();
        self.schema_constraint = Some(constraint);
        self.schema_constraint_state = Some(initial_state);
        Ok(self)
    }

    /// Check if a token is valid according to schema constraints
    pub fn is_token_valid_schema(&self, token: u32) -> anyhow::Result<bool> {
        if let (Some(constraint), Some(state)) =
            (&self.schema_constraint, &self.schema_constraint_state)
        {
            constraint.try_next(state, token)
        } else {
            Ok(true) // No schema constraints, all tokens valid
        }
    }

    /// Update schema constraint state after token generation
    pub fn update_schema_constraint_state(&mut self, token: u32) -> anyhow::Result<bool> {
        if let (Some(constraint), Some(state)) =
            (&self.schema_constraint, &mut self.schema_constraint_state)
        {
            constraint.update(state, token)
        } else {
            Ok(true) // No constraints to update
        }
    }

    /// Check if schema constraint-based generation is complete
    pub fn is_schema_constraint_done(&self) -> bool {
        if let (Some(constraint), Some(state)) =
            (&self.schema_constraint, &self.schema_constraint_state)
        {
            constraint.is_done(state)
        } else {
            false // No constraints, generation not complete
        }
    }

    /// Get deterministic token sequence from schema constraints
    pub fn get_schema_deterministic_sequence(&self) -> anyhow::Result<Vec<u32>> {
        if let (Some(constraint), Some(state)) =
            (&self.schema_constraint, &self.schema_constraint_state)
        {
            constraint.get_deterministic_sequence(state)
        } else {
            Ok(Vec::new()) // No constraints, no forced sequence
        }
    }

    /// Get allowed tokens for current schema constraint state
    pub fn get_schema_allowed_tokens(&self) -> Option<&rustc_hash::FxHashMap<u32, u32>> {
        if let (Some(constraint), Some(state)) =
            (&self.schema_constraint, &self.schema_constraint_state)
        {
            constraint.get_allowed_tokens(state)
        } else {
            None
        }
    }

    /// Check if we have any active constraints (JSON or schema)
    pub fn has_constraints(&self) -> bool {
        self.json_constraint.is_some() || self.schema_constraint.is_some()
    }

    /// Get comprehensive constraint validation for a token
    pub fn is_token_valid_any_constraint(&self, token: u32) -> anyhow::Result<bool> {
        // Check JSON constraints first
        let json_valid = if self.json_constraint.is_some() {
            self.is_token_valid(token)?
        } else {
            true
        };

        // Check schema constraints
        let schema_valid = if self.schema_constraint.is_some() {
            self.is_token_valid_schema(token)?
        } else {
            true
        };

        // Token is valid if it passes all active constraints
        Ok(json_valid && schema_valid)
    }

    /// Update all active constraint states after token generation
    pub fn update_all_constraint_states(&mut self, token: u32) -> anyhow::Result<bool> {
        let mut all_accepted = true;

        // Update JSON constraints
        if self.json_constraint.is_some() {
            let json_accepted = self.update_constraint_state(token)?;
            all_accepted &= json_accepted;
        }

        // Update schema constraints
        if self.schema_constraint.is_some() {
            let schema_accepted = self.update_schema_constraint_state(token)?;
            all_accepted &= schema_accepted;
        }

        Ok(all_accepted)
    }

    /// Check if any constraint indicates generation is complete
    pub fn is_any_constraint_done(&self) -> bool {
        let json_done = if self.json_constraint.is_some() {
            self.is_constraint_done()
        } else {
            false
        };

        let schema_done = if self.schema_constraint.is_some() {
            self.is_schema_constraint_done()
        } else {
            false
        };

        json_done || schema_done
    }
}
