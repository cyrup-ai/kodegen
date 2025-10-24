//! Logits processor implementation

use crate::config::ProcessorConfig;
use crate::context::ProcessingContext;
use crate::logits::{LogitsError, LogitsResult};

/// Default implementation of the logits processor
#[derive(Debug, Clone, Default)]
pub struct DefaultLogitsProcessor {
    config: ProcessorConfig,
}

impl DefaultLogitsProcessor {
    /// Create a new processor with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new processor with the given configuration
    #[must_use]
    pub fn with_config(config: ProcessorConfig) -> Self {
        Self { config }
    }

    /// Process logits with the given context
    pub fn process(&mut self, logits: &mut [f32], context: &ProcessingContext) -> LogitsResult<()> {
        if logits.is_empty() {
            return Ok(());
        }

        // Apply temperature scaling
        let temperature = context.temperature;
        if temperature != 1.0 && temperature > 0.0 {
            let inv_temp = 1.0 / temperature;
            for x in logits.iter_mut() {
                *x *= inv_temp;
            }
        }

        // Apply penalties
        super::penalties::apply_penalties_simd(logits, context, &self.config)?;

        // Apply top-k filtering if enabled
        if let Some(k) = context.top_k.or(self.config.top_k)
            && k > 0
            && k < logits.len()
        {
            super::topk::topk_filtering_simd(logits, k)?;
        }

        // Apply nucleus sampling if enabled
        if let Some(top_p) = context.top_p.or(self.config.top_p)
            && top_p > 0.0
            && top_p < 1.0
        {
            super::nucleus::prepare_nucleus_sampling_simd(logits, top_p as f64)?;
        }

        // Ensure we have valid probabilities
        if logits.iter().all(|&x| x == f32::NEG_INFINITY) {
            return Err(LogitsError::SamplingError(
                "All logits are negative infinity".to_string(),
            ));
        }

        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut ProcessorConfig {
        &mut self.config
    }
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;

    #[test]
    fn test_temperature_scaling() {
        let mut processor = DefaultLogitsProcessor::new();
        let mut logits = vec![1.0, 2.0, 3.0];
        let context = ProcessingContext::new().with_temperature(0.5);

        if let Err(e) = processor.process(&mut logits, &context) {
            panic!("Temperature scaling failed: {}", e);
        }

        assert_float_eq!(logits[0], 2.0, abs <= 1e-6);
        assert_float_eq!(logits[1], 4.0, abs <= 1e-6);
        assert_float_eq!(logits[2], 6.0, abs <= 1e-6);
    }

    #[test]
    fn test_topk_processing() {
        let mut processor = DefaultLogitsProcessor::new();
        let mut logits = vec![1.0, 5.0, 2.0, 4.0, 3.0];
        let context = ProcessingContext::new().with_top_k(Some(2));

        if let Err(e) = processor.process(&mut logits, &context) {
            panic!("Top-k processing failed: {}", e);
        }

        // Only top 2 values should remain non-negative infinity
        let non_inf = logits.iter().filter(|&&x| x > f32::NEG_INFINITY).count();
        assert_eq!(non_inf, 2);
        assert!(logits[1] > f32::NEG_INFINITY); // 5.0
        assert!(logits[3] > f32::NEG_INFINITY); // 4.0
    }

    #[test]
    fn test_nucleus_processing() {
        let mut processor = DefaultLogitsProcessor::new();
        let mut logits = vec![1.0, 2.0, 3.0, 4.0];
        let context = ProcessingContext::new().with_top_p(Some(0.7));

        if let Err(e) = processor.process(&mut logits, &context) {
            panic!("Nucleus processing failed: {}", e);
        }

        // At least one value should be set to negative infinity
        let has_inf = logits.contains(&f32::NEG_INFINITY);
        assert!(has_inf);
    }

    #[test]
    fn test_penalties() {
        let mut processor = DefaultLogitsProcessor::with_config(ProcessorConfig {
            repetition_penalty: 2.0,
            frequency_penalty: 0.5,
            presence_penalty: 0.1,
            ..Default::default()
        });

        let mut logits = vec![1.0, 2.0, 3.0];
        let context = ProcessingContext::new().with_token_history(vec![0, 0, 1]); // Token 0 appears twice, token 1 once

        if let Err(e) = processor.process(&mut logits, &context) {
            panic!("Penalty processing failed: {}", e);
        }

        // Verify penalties were applied
        // Original: [1.0, 2.0, 3.0]
        // After repetition penalty (divide by 2.0 for tokens 0 and 1): [0.5, 1.0, 3.0]
        // After frequency penalty (subtract 1.0 for token 0, 0.5 for token 1): [-0.5, 0.5, 3.0]
        // After presence penalty (subtract 0.1 for tokens 0 and 1): [-0.6, 0.4, 3.0]
        assert_float_eq!(logits[0], -0.6, abs <= 1e-6);
        assert_float_eq!(logits[1], 0.4, abs <= 1e-6);
        assert_float_eq!(logits[2], 3.0, abs <= 1e-6);
    }
}
