//! Generation constraints for reliable JSON output and other structured generation
//!
//! This module provides constraint-based generation that ensures the model produces
//! valid, structured output by masking invalid tokens during logits processing.

use std::fmt::Debug;

use anyhow::Result as AnyResult;

pub mod json;
pub mod processor;
pub mod schema;
pub mod schema_index;
pub mod schema_parser;
pub mod types;

pub use json::JsonConstraint;
pub use processor::{ConstrainedLogitsProcessor, ConstraintStats};
pub use schema::{
    PredefinedSchema, SchemaConstraintBuilder, SchemaFactory, SchemaState, SchemaType, presets,
};
pub use schema_index::{
    IndexStats, SchemaConstraint, SchemaConstraintState, SchemaIndex, SchemaVocabulary, StateId,
    TokenId,
};
pub use schema_parser::{SchemaParser, regex_from_schema, regex_from_value};
pub use types::*;

/// Core trait for generation constraints that can be applied during logits processing
///
/// Constraints maintain state across token generation and can:
/// - Validate if a token is allowed at the current position
/// - Update internal state when a token is accepted
/// - Force deterministic sequences when only one valid choice exists
/// - Detect when generation is complete
pub trait GenerationConstraint: Debug + Send + Sync {
    /// The state type maintained by this constraint
    type State: Clone + Send + Sync + Debug;

    /// Create a new initial state for this constraint
    fn new_state(&self) -> Self::State;

    /// Update the constraint state after a token is generated
    ///
    /// # Arguments
    /// * `state` - The current constraint state to update
    /// * `token` - The token that was just generated
    ///
    /// # Returns
    /// * `Ok(true)` if the token was accepted and state updated
    /// * `Ok(false)` if the token was rejected
    /// * `Err(_)` if there was an error processing the token
    fn update(&self, state: &mut Self::State, token: u32) -> AnyResult<bool>;

    /// Check if a token would be valid at the current state without updating state
    ///
    /// # Arguments
    /// * `state` - The current constraint state
    /// * `token` - The token to validate
    ///
    /// # Returns
    /// * `Ok(true)` if the token would be valid
    /// * `Ok(false)` if the token would be invalid
    /// * `Err(_)` if there was an error checking the token
    fn try_next(&self, state: &Self::State, token: u32) -> AnyResult<bool>;

    /// Check if generation is complete according to this constraint
    ///
    /// # Arguments
    /// * `state` - The current constraint state
    ///
    /// # Returns
    /// * `true` if generation should stop
    /// * `false` if generation should continue
    fn is_done(&self, state: &Self::State) -> bool;

    /// Get a deterministic token sequence if only one valid path exists
    ///
    /// This is used for optimization when the constraint forces a specific
    /// sequence of tokens (e.g., completing `"true"` when parsing a boolean).
    ///
    /// # Arguments
    /// * `state` - The current constraint state
    ///
    /// # Returns
    /// * `Ok(tokens)` - A sequence of tokens that must be generated
    /// * `Err(_)` if there was an error determining the sequence
    fn get_deterministic_sequence(&self, state: &Self::State) -> AnyResult<Vec<u32>>;
}
