//! Token sampling utilities

use candle_core::Tensor;
use candle_transformers::generation::{LogitsProcessor, Sampling};

/// Sample next token from logits using LogitsProcessor (static version for thread)
pub(crate) fn sample_token_static(temperature: f64, logits: &Tensor) -> Result<u32, String> {
    let logits = logits
        .squeeze(0)
        .map_err(|e| format!("Logits squeeze failed: {}", e))?;

    // Use LogitsProcessor for sampling
    let sampling = if temperature <= 0.0 {
        Sampling::ArgMax
    } else {
        Sampling::All { temperature }
    };

    let mut processor = LogitsProcessor::from_sampling(299792458, sampling);
    processor
        .sample(&logits)
        .map_err(|e| format!("Sampling failed: {}", e))
}
