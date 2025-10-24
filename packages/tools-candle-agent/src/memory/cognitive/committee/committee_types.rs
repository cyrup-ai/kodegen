//! Committee Types with Registry-Based Model Usage
//!
//! Committee evaluation using the registry pattern for proper model caching.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_stream::StreamExt;

use crate::capability::registry::TextToTextModel;
use crate::capability::text_to_text::CandleQwen3QuantizedModel;
use crate::capability::traits::TextToTextCapable;
use crate::domain::{
    completion::CandleCompletionParams, context::chunks::CandleCompletionChunk,
    prompt::CandlePrompt,
};
use crate::memory::cognitive::types::CognitiveError;

/// Committee configuration for provider-based evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeConfig {
    pub member_count: usize,
    pub consensus_threshold: f64,
    pub use_multiple_providers: bool,
}

impl Default for CommitteeConfig {
    fn default() -> Self {
        Self {
            member_count: 3,
            consensus_threshold: 0.7,
            use_multiple_providers: true,
        }
    }
}

// ModelType enum removed - use provider instances directly

// CommitteeModelPool removed - use models directly

/// Committee for memory evaluation using registry-based models
#[derive(Debug)]
pub struct Committee {
    pub config: CommitteeConfig,
    pub qwen_model: TextToTextModel,
}

impl Committee {
    pub async fn new(config: CommitteeConfig) -> Result<Self, CognitiveError> {
        let qwen_base = Arc::new(
            CandleQwen3QuantizedModel::new()
                .map_err(|e| CognitiveError::InitializationError(e.to_string()))?,
        );

        Ok(Self {
            config,
            qwen_model: TextToTextModel::Qwen3Quantized(qwen_base),
        })
    }

    /// Evaluate content using Qwen3Quantized model via registry
    pub async fn evaluate(&self, content: &str) -> Result<f64, CognitiveError> {
        let evaluation_prompt = format!(
            "Evaluate the quality of this content on a scale from 0.0 to 1.0, considering factors like clarity, relevance, and completeness. Return only a decimal number between 0.0 and 1.0.\n\nContent:\n{}",
            content
        );

        let prompt = CandlePrompt::new(&evaluation_prompt);
        let params = CandleCompletionParams::default();

        // Use registry model which properly uses LoadedModel pattern
        let mut response = String::new();
        let mut stream = Box::pin(self.qwen_model.prompt(prompt, &params));

        // Consume stream asynchronously
        while let Some(chunk) = stream.next().await {
            match chunk {
                CandleCompletionChunk::Text(text) => response.push_str(&text),
                CandleCompletionChunk::Complete { text, .. } => {
                    response.push_str(&text);
                    break;
                }
                _ => {}
            }
        }

        let score = parse_score_from_response(&response).unwrap_or(0.5);
        Ok(score)
    }
}

/// Parse numerical score from AI response
pub fn parse_score_from_response(response: &str) -> Option<f64> {
    // Look for decimal numbers in the response
    use regex::Regex;
    let re = Regex::new(r"([01]?\.\d+|[01]\.?0*)").ok()?;

    for cap in re.captures_iter(response) {
        if let Ok(score) = cap[1].parse::<f64>()
            && (0.0..=1.0).contains(&score)
        {
            return Some(score);
        }
    }
    None
}
