//! Core text generation engine with SIMD acceleration
//!
//! This module contains the TextGenerator implementation with tokio stream token streaming,
//! SIMD-optimized sampling methods, and pure SIMD delegation without scalar fallbacks.

use crate::async_stream;
use candle_core::{Device, Tensor};
use tokenizers::Tokenizer;
use tokio_stream::Stream;

use crate::domain::context::chunks::{CandleStringChunk, GenerationStats};
use kodegen_simd::logits::LogitsProcessor as LogitsProcessorTrait;
use kodegen_simd::logits::constraints::GenerationConstraint;

use super::{
    config::SamplingConfig,
    metrics::SimdMetrics,
    models::CandleModel,
    stats::GenerationStatistics,
    tokens::{SpecialTokens, TokenHistory},
    types::CandleResult,
};

// Import constraint types for schema-based generation
use kodegen_simd::logits::constraints::{JsonConstraint, json::JsonState};

/// Core text generation engine with SIMD acceleration
///
/// Provides tokio stream-based token generation with comprehensive SIMD optimization,
/// statistics tracking, and configurable sampling parameters. Uses pure SIMD
/// delegation without scalar fallbacks for maximum performance.
pub struct TextGenerator {
    /// The model for text generation
    pub model: Box<dyn CandleModel>,

    /// Tokenizer for encoding/decoding
    pub tokenizer: Tokenizer,

    /// Device for tensor operations
    pub device: Device,

    /// Sampling configuration
    pub config: SamplingConfig,

    /// Token history for repetition penalties
    pub token_history: TokenHistory,

    /// Generation statistics tracking
    pub stats: GenerationStatistics,

    /// SIMD performance metrics
    pub simd_metrics: SimdMetrics,

    /// Optional JSON constraint for structured generation
    pub constraint: Option<JsonConstraint<'static>>,

    /// Current JSON constraint state
    pub constraint_state: Option<JsonState>,
}
impl TextGenerator {
    /// Create new TextGenerator
    pub fn new(
        model: Box<dyn CandleModel>,
        tokenizer: Tokenizer,
        device: Device,
        config: SamplingConfig,
    ) -> Self {
        let max_history = config.repetition_context_length;

        Self {
            model,
            tokenizer,
            device,
            config,
            token_history: TokenHistory::new(max_history),
            stats: GenerationStatistics::new(),
            simd_metrics: SimdMetrics::new(),
            constraint: None,
            constraint_state: None,
        }
    }

    /// Emit final chunk with generation statistics
    ///
    /// Safe to call from any error path. Idempotent.
    /// Ignores send errors if receiver has dropped.
    fn emit_final_stats(&mut self, tx: &tokio::sync::mpsc::UnboundedSender<CandleStringChunk>) {
        // Stop timer (idempotent - does nothing if already stopped)
        self.stats.stop_generation();

        let stats = GenerationStats {
            tokens_generated: self.stats.total_tokens as u32,
            elapsed_secs: self.stats.total_duration.as_secs_f64(),
            tokens_per_sec: self.stats.tokens_per_second(),
        };

        log::debug!(
            "Generation complete: {} tokens in {:.2}s ({:.2} tokens/sec)",
            stats.tokens_generated,
            stats.elapsed_secs,
            stats.tokens_per_sec
        );

        let _ = tx.send(CandleStringChunk::final_with_stats(stats));
    }

    /// Generate text using tokio stream with SIMD acceleration
    pub fn generate(
        mut self,
        prompt: String,
        max_tokens: u32,
        special_tokens: SpecialTokens,
    ) -> impl Stream<Item = crate::domain::context::chunks::CandleStringChunk> {
        async_stream::spawn_stream(move |tx| async move {
            log::info!(
                ">>> GENERATE STARTED: max_tokens={}, prompt_len={}, eos={:?}",
                max_tokens,
                prompt.len(),
                special_tokens.eos_token_id
            );
            self.stats.start_generation();

            // Encode prompt to tokens using tokenizer (fast CPU operation)
            log::info!(">>> Encoding prompt...");
            let tokens = match self.tokenizer.encode(prompt.as_str(), true) {
                Ok(encoded) => {
                    let ids = encoded.get_ids().to_vec();
                    log::info!(">>> Prompt encoded to {} tokens", ids.len());
                    ids
                }
                Err(e) => {
                    log::error!("Prompt encoding error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };

            self.stats.set_input_tokens(tokens.len() as u64);
            let mut all_tokens = tokens.clone();
            let mut position = 0;

            // Initial forward pass - fast tensor creation
            log::info!(">>> Creating initial input tensor...");
            let initial_input = match Tensor::new(tokens.as_slice(), &self.device) {
                Ok(tensor) => match tensor.unsqueeze(0) {
                    Ok(unsqueezed) => {
                        log::info!(">>> Initial tensor created");
                        unsqueezed
                    }
                    Err(e) => {
                        log::error!("Initial tensor unsqueeze error: {}", e);
                        self.emit_final_stats(&tx);
                        return;
                    }
                },
                Err(e) => {
                    log::error!("Initial tensor creation error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };

            log::info!(">>> Running initial forward pass...");
            let initial_logits = match self.model.forward(&initial_input, position).await {
                Ok(logits) => {
                    log::info!(">>> Forward pass completed, squeezing logits...");
                    match logits.squeeze(0) {
                        Ok(squeezed) => {
                            log::info!(">>> Logits squeezed successfully");
                            squeezed
                        }
                        Err(e) => {
                            log::error!("Initial logits squeeze error: {}", e);
                            self.emit_final_stats(&tx);
                            return;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Initial forward pass error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };

            self.stats.record_forward_pass();
            // Convert logits to vec - fast CPU operation
            log::info!(">>> Converting logits to vector...");
            let logits_vec = match initial_logits.to_vec1::<f32>() {
                Ok(v) => {
                    log::info!(">>> Logits converted, vocab_size={}", v.len());
                    v
                }
                Err(e) => {
                    log::error!("Converting initial logits to vector error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };
            log::info!(">>> Sampling first token...");
            let mut next_token = match self.sample_token(&logits_vec, &tokens).await {
                Ok(token) => {
                    log::info!(">>> Sampled token: {}", token);
                    token
                }
                Err(e) => {
                    log::error!("Initial SIMD sampling error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };
            all_tokens.push(next_token);
            self.token_history.push(next_token);

            // Update constraint state after token sampling
            if let Err(e) = self.update_constraint_state(next_token) {
                log::warn!("Failed to update constraint state: {}", e);
            }

            position += 1;

            // Decode and emit initial token - fast CPU operation
            // Note: We decode and send the first token even if it's EOS to ensure at least one chunk is emitted
            log::info!("🎯 First token generated: {}", next_token);
            match self.tokenizer.decode(&[next_token], false) {
                Ok(token_str) => {
                    log::info!("✅ Sending first token: '{}'", token_str);
                    let _ = tx.send(CandleStringChunk::text(token_str));
                }
                Err(e) => {
                    log::error!("Initial token decoding error: {}", e);
                    self.emit_final_stats(&tx);
                    return;
                }
            };

            // Check termination AFTER sending first token
            if self.should_stop(next_token, &special_tokens) {
                log::info!(
                    "STOP: First token was EOS ({}), stopping generation",
                    next_token
                );
                self.emit_final_stats(&tx);
                return; // Graceful EOS termination after at least one token sent
            }

            // Generation loop - stream each token as generated
            for _index in 1..max_tokens {
                // Prepare input tensor for next forward pass - fast CPU operation
                let input = match Tensor::new(&[next_token], &self.device) {
                    Ok(tensor) => match tensor.unsqueeze(0) {
                        Ok(unsqueezed) => unsqueezed,
                        Err(e) => {
                            log::error!("Tensor unsqueeze in loop error: {}", e);
                            break;
                        }
                    },
                    Err(e) => {
                        log::error!("Tensor creation in loop error: {}", e);
                        break;
                    }
                };

                // Forward pass using model
                let logits = match self.model.forward(&input, position).await {
                    Ok(logits) => match logits.squeeze(0) {
                        Ok(squeezed) => squeezed,
                        Err(e) => {
                            log::error!("Logits squeeze in loop error: {}", e);
                            break;
                        }
                    },
                    Err(e) => {
                        log::error!("Forward pass in loop error: {}", e);
                        break;
                    }
                };

                self.stats.record_forward_pass();
                // SIMD sampling using existing infrastructure - fast CPU operation
                let logits_vec = match logits.to_vec1::<f32>() {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("Converting logits to vector in loop error: {}", e);
                        break;
                    }
                };
                next_token = match self.sample_token(&logits_vec, &all_tokens).await {
                    Ok(token) => token,
                    Err(e) => {
                        log::error!("SIMD sampling in loop error: {}", e);
                        break;
                    }
                };
                all_tokens.push(next_token);
                self.token_history.push(next_token);

                // Update constraint state after token sampling
                if let Err(e) = self.update_constraint_state(next_token) {
                    log::warn!("Failed to update constraint state: {}", e);
                }

                position += 1;

                // Check termination before decoding
                if self.should_stop(next_token, &special_tokens) {
                    break; // Graceful EOS termination
                }

                // Decode and emit individual token - fast CPU operation
                match self.tokenizer.decode(&[next_token], false) {
                    Ok(token_str) => {
                        let _ = tx.send(CandleStringChunk::text(token_str)); // Individual token streaming
                    }
                    Err(e) => {
                        log::error!("Token decoding in loop error: {}", e);
                        break;
                    }
                };

                self.stats.add_tokens(1);
            }

            self.emit_final_stats(&tx);
        })
    }
    /// SIMD-optimized token sampling with comprehensive acceleration (async)
    pub async fn sample_token(&mut self, logits: &[f32], _context: &[u32]) -> CandleResult<u32> {
        use kodegen_simd::{
            argmax, prepare_nucleus_sampling_simd, scale_temperature, softmax, topk_filtering_simd,
        };

        // Clone necessary data before moving into spawn_blocking
        let logits_owned = logits.to_vec();
        let config = self.config.clone();
        let token_history = self.token_history.clone();
        let constraint = self.constraint.clone();
        let constraint_state = self.constraint_state.clone();

        // Wrap all SIMD operations in spawn_blocking for CPU-intensive work
        let result = tokio::task::spawn_blocking(move || -> CandleResult<u32> {
            let mut logits = logits_owned;

            // Apply temperature scaling with SIMD
            if config.temperature != 1.0 {
                scale_temperature(&mut logits, config.temperature).map_err(|e| {
                    crate::domain::model::error::CandleModelError::OperationNotSupported(
                        e.to_string().into(),
                    )
                })?;
            }

            // Apply top-k filtering with SIMD
            if let Some(k) = config.top_k {
                topk_filtering_simd(&mut logits, k).map_err(|e| {
                    crate::domain::model::error::CandleModelError::OperationNotSupported(
                        e.to_string().into(),
                    )
                })?;
            }

            // Apply nucleus sampling with SIMD
            if let Some(p) = config.top_p {
                prepare_nucleus_sampling_simd(&mut logits, p).map_err(|e| {
                    crate::domain::model::error::CandleModelError::OperationNotSupported(
                        e.to_string().into(),
                    )
                })?;
            }

            // Create processing context with all fields including constraints
            let context = kodegen_simd::context::ProcessingContext {
                temperature: config.temperature,
                top_k: config.top_k,
                top_p: config.top_p.map(|p| p as f32),
                token_history: token_history.as_slice(),
                start_time: None,
                max_new_tokens: None,
                stop_tokens: Vec::new(),
                json_constraint: constraint,
                json_constraint_state: constraint_state,
                schema_constraint: None,
                schema_constraint_state: None,
            };

            // Use ConstrainedLogitsProcessor for all processing including constraints
            let processor_config = kodegen_simd::config::ProcessorConfig {
                temperature: config.temperature,
                top_k: config.top_k,
                top_p: config.top_p.map(|p| p as f32),
                repetition_penalty: config.repetition_penalty,
                frequency_penalty: config.frequency_penalty,
                presence_penalty: config.presence_penalty,
            };

            let mut processor = kodegen_simd::logits::constraints::ConstrainedLogitsProcessor::new(
                processor_config,
            );
            processor.process(&mut logits, &context).map_err(|e| {
                crate::domain::model::error::CandleModelError::OperationNotSupported(
                    e.to_string().into(),
                )
            })?;

            // Convert to probabilities with SIMD softmax
            let probs = softmax(&logits).map_err(|e| {
                crate::domain::model::error::CandleModelError::OperationNotSupported(
                    e.to_string().into(),
                )
            })?;

            // Sample token with SIMD argmax (for deterministic) or weighted sampling
            let token = if config.is_deterministic() {
                argmax(&probs).map_err(|e| {
                    crate::domain::model::error::CandleModelError::OperationNotSupported(
                        e.to_string().into(),
                    )
                })?
            } else {
                // Weighted sampling implementation would go here
                argmax(&probs).map_err(|e| {
                    crate::domain::model::error::CandleModelError::OperationNotSupported(
                        e.to_string().into(),
                    )
                })?
            };

            Ok(token as u32)
        })
        .await
        .map_err(|e| {
            crate::domain::model::error::CandleModelError::Internal(
                format!("SIMD sampling spawn_blocking failed: {}", e).into(),
            )
        })??;

        // Record metrics after spawn_blocking completes
        if self.config.temperature != 1.0 {
            self.simd_metrics.record_temperature_op();
        }
        if self.config.top_k.is_some() {
            self.simd_metrics.record_topk_op();
        }
        if self.config.top_p.is_some() {
            self.simd_metrics.record_nucleus_op();
        }
        self.simd_metrics.record_penalty_op();
        self.simd_metrics.record_softmax_op();
        self.simd_metrics.record_argmax_op();
        self.stats.record_simd_operation();

        Ok(result)
    }

    /// Check if generation should stop
    pub fn should_stop(&self, token: u32, special_tokens: &SpecialTokens) -> bool {
        special_tokens.is_eos(token)
    }

    /// Get generation statistics
    pub fn stats(&self) -> &GenerationStatistics {
        &self.stats
    }

    /// Get SIMD metrics
    pub fn simd_metrics(&self) -> &SimdMetrics {
        &self.simd_metrics
    }

    /// Reset all statistics and metrics
    pub fn reset_stats(&mut self) {
        self.stats.reset();
        self.simd_metrics.reset();
        self.token_history.clear();
    }

    /// Set JSON constraint for structured generation
    pub fn set_json_constraint(
        &mut self,
        constraint: JsonConstraint<'static>,
    ) -> anyhow::Result<()> {
        let initial_state = constraint.new_state();
        self.constraint = Some(constraint);
        self.constraint_state = Some(initial_state);
        Ok(())
    }

    /// Remove JSON constraint
    pub fn clear_constraint(&mut self) {
        self.constraint = None;
        self.constraint_state = None;
    }

    /// Check if generation has active constraints
    pub fn has_constraints(&self) -> bool {
        self.constraint.is_some() && self.constraint_state.is_some()
    }

    /// Update constraint state after token generation
    pub fn update_constraint_state(&mut self, token: u32) -> anyhow::Result<bool> {
        if let (Some(constraint), Some(state)) = (&self.constraint, &mut self.constraint_state) {
            constraint.update(state, token)
        } else {
            Ok(true) // No constraints, continue generation
        }
    }

    /// Check if constraint-based generation is complete
    pub fn is_constraint_done(&self) -> bool {
        if let (Some(constraint), Some(state)) = (&self.constraint, &self.constraint_state) {
            constraint.is_done(state)
        } else {
            false // No constraints, not constraint-complete
        }
    }
}

impl std::fmt::Debug for TextGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextGenerator")
            .field("device", &self.device)
            .field("config", &self.config)
            .field("token_history", &self.token_history)
            .field("stats", &self.stats)
            .field("simd_metrics", &self.simd_metrics)
            .field("has_constraint", &self.constraint.is_some())
            .finish()
    }
}
