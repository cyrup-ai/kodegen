//! Committee Evaluators Using Existing Models
//!
//! Committee evaluation implementation using CandleQwen3QuantizedModel.

use crate::capability::traits::TextToTextCapable;
use crate::domain::{
    completion::CandleCompletionParams, context::chunks::CandleCompletionChunk,
    prompt::CandlePrompt,
};
use crate::memory::cognitive::committee::committee_types::{Committee, CommitteeConfig};
use crate::memory::cognitive::types::CognitiveError;
use std::sync::Arc;
use tokio_stream::StreamExt;

/// Committee evaluator using Qwen3Quantized model
#[derive(Debug)]
pub struct ModelCommitteeEvaluator {
    committee: Arc<Committee>,
}

impl ModelCommitteeEvaluator {
    /// Create new committee evaluator using existing models
    pub async fn new() -> Result<Self, CognitiveError> {
        let config = CommitteeConfig::default();
        let committee = Arc::new(Committee::new(config).await?);

        Ok(Self { committee })
    }

    /// Create evaluator with custom configuration
    pub async fn with_config(config: CommitteeConfig) -> Result<Self, CognitiveError> {
        let committee = Arc::new(Committee::new(config).await?);

        Ok(Self { committee })
    }

    /// Evaluate content using existing providers for real AI evaluation
    pub async fn evaluate(&self, content: &str) -> Result<f64, CognitiveError> {
        self.committee.evaluate(content).await
    }

    /// Evaluate multiple memories in a single batch LLM call
    ///
    /// This reduces N LLM calls to 1 call for batch size N
    pub async fn evaluate_batch(
        &self,
        memories: &[(String, String)],
    ) -> Result<Vec<(String, f64)>, CognitiveError> {
        // Create batch evaluation prompt
        let mut batch_prompt =
            String::from("Evaluate the quality of each memory below on a scale from 0.0 to 1.0.\n");
        batch_prompt.push_str("Consider clarity, relevance, and completeness.\n");
        batch_prompt.push_str(
            "Return ONLY a JSON array of scores in the exact order: [0.8, 0.6, 0.9, ...]\n\n",
        );

        for (i, (_id, content)) in memories.iter().enumerate() {
            batch_prompt.push_str(&format!("Memory {}:\n{}\n\n", i + 1, content));
        }

        batch_prompt.push_str(
            "\nReturn scores as JSON array with one score per memory: [score1, score2, ...]",
        );

        let prompt = CandlePrompt::new(&batch_prompt);
        let params = CandleCompletionParams::default();

        // Get response from Qwen provider
        let mut response = String::new();
        let mut stream = Box::pin(self.committee.qwen_model.prompt(prompt, &params));

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

        // Parse JSON array of scores
        let scores = parse_batch_scores(&response, memories.len())?;

        // Pair memory IDs with scores
        let results: Vec<(String, f64)> = memories
            .iter()
            .map(|(id, _)| id.clone())
            .zip(scores.into_iter())
            .collect();

        Ok(results)
    }

    /// Generate evaluation report using AI
    pub async fn generate_report(&self, content: &str) -> Result<String, CognitiveError> {
        let score = self.evaluate(content).await?;
        Ok(format!(
            "AI evaluation score: {:.2} (using local Qwen3Quantized model)",
            score
        ))
    }

    /// Evaluate with Qwen provider
    pub async fn evaluate_with_qwen(&self, content: &str) -> Result<String, CognitiveError> {
        let evaluation_prompt = format!(
            "Provide a detailed evaluation of this content including strengths, weaknesses, and an overall quality assessment:\n\nContent:\n{}",
            content
        );

        let prompt = CandlePrompt::new(&evaluation_prompt);
        let params = CandleCompletionParams::default();

        let mut response = String::new();
        let mut stream = Box::pin(self.committee.qwen_model.prompt(prompt, &params));

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

        Ok(response)
    }

    /// Evaluate using Qwen provider (single model evaluation)
    pub async fn consensus_evaluate(&self, content: &str) -> Result<f64, CognitiveError> {
        // With only one model, consensus is just the single evaluation
        self.evaluate(content).await
    }
}

/// Parse batch scores from LLM response
fn parse_batch_scores(response: &str, expected_count: usize) -> Result<Vec<f64>, CognitiveError> {
    use regex::Regex;

    // Extract JSON array from response
    let re = Regex::new(r"\[[\d\.,\s]+\]")
        .map_err(|e| CognitiveError::ProcessingError(format!("Regex error: {}", e)))?;

    if let Some(captures) = re.find(response) {
        let json_str = captures.as_str();
        let scores: Vec<f64> = serde_json::from_str(json_str)
            .map_err(|e| CognitiveError::ProcessingError(format!("JSON parse error: {}", e)))?;

        // Validate score count
        if scores.len() != expected_count {
            return Err(CognitiveError::ProcessingError(format!(
                "Expected {} scores, got {}",
                expected_count,
                scores.len()
            )));
        }

        // Validate score range
        for (i, &score) in scores.iter().enumerate() {
            if !(0.0..=1.0).contains(&score) {
                return Err(CognitiveError::ProcessingError(format!(
                    "Score {} at index {} out of range [0.0, 1.0]",
                    score, i
                )));
            }
        }

        Ok(scores)
    } else {
        Err(CognitiveError::ProcessingError(
            "No score array found in response".to_string(),
        ))
    }
}
