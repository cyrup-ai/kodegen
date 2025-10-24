//! Constraint-aware logits processor for structured generation
//!
//! This module provides `ConstrainedLogitsProcessor` that applies both standard
//! logits processing (temperature, top-k, nucleus) and constraint masking to
//! ensure generated tokens conform to specified constraints.

use anyhow::Context;

use crate::config::ProcessorConfig;
use crate::context::ProcessingContext;
use crate::logits::processor::DefaultLogitsProcessor;
use crate::logits::{LogitsError, LogitsProcessor, LogitsResult};

use super::GenerationConstraint;

/// Logits processor that applies constraints after standard processing
///
/// This processor first applies standard logits processing operations
/// (temperature scaling, penalties, top-k, nucleus sampling), then
/// applies constraint masking to ensure only valid tokens can be selected.
///
/// ## Processing Order
/// 1. Temperature scaling
/// 2. Repetition/frequency/presence penalties  
/// 3. Top-k filtering
/// 4. Nucleus (top-p) sampling
/// 5. **Constraint masking** (sets invalid tokens to `f32::NEG_INFINITY`)
///
/// ## Performance
/// - Zero allocation when no constraints are active
/// - Efficient constraint validation through `try_next()` calls
/// - SIMD-optimized masking for large vocabularies
/// - Maintains all existing SIMD optimizations for standard processing
pub struct ConstrainedLogitsProcessor {
    /// Inner processor for standard logits operations
    inner: DefaultLogitsProcessor,
    /// Whether constraint processing is enabled
    constraints_enabled: bool,
    /// Track tokens masked in last processing step
    last_tokens_masked: usize,
}

impl ConstrainedLogitsProcessor {
    /// Create a new constrained logits processor
    ///
    /// # Arguments
    /// * `config` - Configuration for standard logits processing
    ///
    /// # Returns
    /// A new processor that can apply both standard and constraint processing
    #[must_use]
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            inner: DefaultLogitsProcessor::with_config(config),
            constraints_enabled: true,
            last_tokens_masked: 0,
        }
    }

    /// Create a processor with constraints disabled
    ///
    /// This creates a processor that only does standard processing,
    /// effectively equivalent to `DefaultLogitsProcessor` but with
    /// the same API for seamless switching.
    #[must_use]
    pub fn new_unconstrained(config: ProcessorConfig) -> Self {
        Self {
            inner: DefaultLogitsProcessor::with_config(config),
            constraints_enabled: false,
            last_tokens_masked: 0,
        }
    }

    /// Enable or disable constraint processing
    ///
    /// When disabled, this processor behaves identically to `DefaultLogitsProcessor`.
    /// When enabled, constraint masking is applied after standard processing.
    pub fn set_constraints_enabled(&mut self, enabled: bool) {
        self.constraints_enabled = enabled;
    }

    /// Check if constraint processing is enabled
    #[must_use]
    pub fn constraints_enabled(&self) -> bool {
        self.constraints_enabled
    }

    /// Apply constraint masking to logits
    ///
    /// This method iterates through all tokens in the vocabulary and masks
    /// (sets to `f32::NEG_INFINITY`) any tokens that would be invalid according
    /// to the current constraint state.
    ///
    /// # Arguments
    /// * `logits` - Mutable slice of logits to mask
    /// * `context` - Processing context containing constraint state
    ///
    /// # Returns
    /// * `Ok(masked_count)` - Number of tokens that were masked
    /// * `Err(LogitsError)` - If constraint validation fails
    fn apply_constraint_masking(
        &self,
        logits: &mut [f32],
        context: &ProcessingContext,
    ) -> LogitsResult<usize> {
        let mut masked_count = 0;

        // Check JSON constraints first
        if let Some(ref constraint) = context.json_constraint
            && let Some(ref state) = context.json_constraint_state
        {
            // Validate each token against the constraint
            for (token_id, logit) in logits.iter_mut().enumerate() {
                // Skip tokens already masked by previous processing
                if *logit == f32::NEG_INFINITY {
                    continue;
                }

                // Check if token is valid for current constraint state
                match constraint.try_next(state, token_id as u32) {
                    Ok(is_valid) => {
                        if !is_valid {
                            *logit = f32::NEG_INFINITY;
                            masked_count += 1;
                        }
                    }
                    Err(e) => {
                        return Err(LogitsError::ConstraintError(format!(
                            "JSON constraint validation failed for token {token_id}: {e}"
                        )));
                    }
                }
            }
        }

        // Check schema constraints
        if let Some(ref constraint) = context.schema_constraint
            && let Some(ref state) = context.schema_constraint_state
        {
            // Validate each token against the schema constraint
            for (token_id, logit) in logits.iter_mut().enumerate() {
                // Skip tokens already masked by previous processing
                if *logit == f32::NEG_INFINITY {
                    continue;
                }

                // Check if token is valid for current constraint state
                match constraint.try_next(state, token_id as u32) {
                    Ok(is_valid) => {
                        if !is_valid {
                            *logit = f32::NEG_INFINITY;
                            masked_count += 1;
                        }
                    }
                    Err(e) => {
                        return Err(LogitsError::ConstraintError(format!(
                            "Schema constraint validation failed for token {token_id}: {e}"
                        )));
                    }
                }
            }
        }

        Ok(masked_count)
    }

    /// Get statistics about constraint masking
    ///
    /// Returns information about how many tokens were masked and
    /// constraint validation performance for debugging/monitoring.
    #[must_use]
    pub fn get_constraint_stats(&self, context: &ProcessingContext) -> ConstraintStats {
        let has_json_constraints =
            context.json_constraint.is_some() && context.json_constraint_state.is_some();
        let has_schema_constraints =
            context.schema_constraint.is_some() && context.schema_constraint_state.is_some();
        let has_constraints = has_json_constraints || has_schema_constraints;

        let constraint_type = if has_json_constraints && has_schema_constraints {
            Some("JSON+Schema".to_string())
        } else if has_json_constraints {
            Some("JSON".to_string())
        } else if has_schema_constraints {
            Some("Schema".to_string())
        } else {
            None
        };

        ConstraintStats {
            constraints_active: has_constraints,
            constraint_type,
            tokens_masked_last_step: self.last_tokens_masked,
        }
    }
}

impl LogitsProcessor for ConstrainedLogitsProcessor {
    /// Process logits with both standard and constraint operations
    ///
    /// This is the main entry point that applies the complete processing pipeline:
    /// 1. Standard logits processing (temperature, penalties, sampling)
    /// 2. Constraint masking (if constraints are active and enabled)
    ///
    /// # Arguments
    /// * `logits` - Mutable slice of logits to process
    /// * `context` - Processing context with generation state and constraints
    ///
    /// # Returns
    /// * `Ok(())` - If processing completed successfully
    /// * `Err(LogitsError)` - If any processing step failed
    ///
    /// # Performance Notes
    /// - Constraint masking only applied if constraints are present
    /// - Zero overhead when constraints are disabled
    /// - All existing SIMD optimizations preserved
    fn process(&mut self, logits: &mut [f32], context: &ProcessingContext) -> LogitsResult<()> {
        // First, apply standard logits processing
        self.inner
            .process(logits, context)
            .context("Standard logits processing failed")
            .map_err(|e| LogitsError::NumericalError(e.to_string()))?;

        // Then apply constraint masking if enabled and constraints are present
        if self.constraints_enabled
            && (context.json_constraint.is_some()
                || context.json_constraint_state.is_some()
                || context.schema_constraint.is_some()
                || context.schema_constraint_state.is_some())
        {
            // Apply constraint masking
            let masked_count = self.apply_constraint_masking(logits, context)?;

            // Store for statistics
            self.last_tokens_masked = masked_count;

            // Log constraint application for debugging
            if masked_count > 0 {
                log::debug!(
                    "Constraint masking applied: {} tokens masked out of {} total",
                    masked_count,
                    logits.len()
                );
            }

            // Verify we haven't masked all tokens (would cause sampling failure)
            let valid_tokens = logits
                .iter()
                .filter(|&&logit| logit > f32::NEG_INFINITY)
                .count();

            if valid_tokens == 0 {
                return Err(LogitsError::SamplingError(
                    "All tokens masked by constraints - cannot sample".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get the current processor configuration
    fn config(&self) -> &ProcessorConfig {
        self.inner.config()
    }

    /// Get a mutable reference to the processor configuration
    fn config_mut(&mut self) -> &mut ProcessorConfig {
        self.inner.config_mut()
    }
}

/// Statistics about constraint processing
#[derive(Debug, Clone)]
pub struct ConstraintStats {
    /// Whether constraints are currently active
    pub constraints_active: bool,
    /// Type of constraint being applied (e.g., "JSON", "Schema")
    pub constraint_type: Option<String>,
    /// Number of tokens masked in the last processing step
    pub tokens_masked_last_step: usize,
}

/// Factory functions for creating constraint processors
impl ConstrainedLogitsProcessor {
    /// Create a processor optimized for JSON constraint generation
    ///
    /// This factory method creates a processor with configuration optimized
    /// for JSON generation tasks, including appropriate temperature and
    /// sampling parameters.
    pub fn for_json_generation() -> Self {
        let config = ProcessorConfig::default()
            .with_temperature(0.7) // Slightly lower temperature for more structured output
            .with_top_p(Some(0.9)); // Enable nucleus sampling

        Self::new(config)
    }

    /// Create a processor optimized for schema-constrained generation
    ///
    /// This factory method creates a processor with configuration optimized
    /// for generating JSON that matches specific schemas, with more
    /// conservative sampling parameters.
    pub fn for_schema_generation() -> Self {
        let config = ProcessorConfig::default()
            .with_temperature(0.5) // Lower temperature for more deterministic output
            .with_top_p(Some(0.8)) // More focused sampling
            .with_top_k(Some(50)); // Limit token choices for better structure

        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ProcessingContext;

    #[test]
    fn test_unconstrained_processing() {
        let config = ProcessorConfig::default();
        let mut processor = ConstrainedLogitsProcessor::new_unconstrained(config);
        let mut logits = vec![1.0, 2.0, 3.0, 4.0];
        let context = ProcessingContext::new();

        // Should process normally without constraints
        let result = processor.process(&mut logits, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_stats() {
        let config = ProcessorConfig::default();
        let processor = ConstrainedLogitsProcessor::new(config);
        let context = ProcessingContext::new();

        let stats = processor.get_constraint_stats(&context);
        assert!(!stats.constraints_active);
        assert!(stats.constraint_type.is_none());
    }

    #[test]
    fn test_factory_methods() {
        let json_processor = ConstrainedLogitsProcessor::for_json_generation();
        assert!(json_processor.constraints_enabled());
        assert_eq!(json_processor.config().temperature, 0.7);

        let schema_processor = ConstrainedLogitsProcessor::for_schema_generation();
        assert!(schema_processor.constraints_enabled());
        assert_eq!(schema_processor.config().temperature, 0.5);
    }

    #[test]
    fn test_enable_disable_constraints() {
        let config = ProcessorConfig::default();
        let mut processor = ConstrainedLogitsProcessor::new(config);

        assert!(processor.constraints_enabled());

        processor.set_constraints_enabled(false);
        assert!(!processor.constraints_enabled());

        processor.set_constraints_enabled(true);
        assert!(processor.constraints_enabled());
    }
}
