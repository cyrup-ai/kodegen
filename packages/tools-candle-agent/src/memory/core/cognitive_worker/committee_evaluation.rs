//! Committee-based memory quality evaluation
//!
//! This module handles LLM-based quality scoring of memories using a committee
//! evaluation approach. Supports both single and batch processing with retry
//! logic and timeout handling.

use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::memory::cognitive::committee::ModelCommitteeEvaluator;
use crate::memory::core::manager::surreal::{MemoryManager, SurrealDBMemoryManager};
use crate::memory::monitoring::operations::{OperationTracker, OperationType};

/// Process committee evaluation using real LLM with timeout, retry, and metrics
pub(crate) async fn process_committee_evaluation(
    memory_manager: &SurrealDBMemoryManager,
    committee_evaluator: &ModelCommitteeEvaluator,
    operation_tracker: &OperationTracker,
    memory_id: &str,
) {
    let memory_id = memory_id.to_string();
    let manager = memory_manager;
    let evaluator = committee_evaluator;
    let tracker = operation_tracker;

    // Start operation tracking
    let op_id = tracker.start_operation(OperationType::CommitteeEvaluation, None);

    let start_time = Instant::now();

    // Get memory from database
    let mut memory = match manager.get_memory(&memory_id).await {
        Ok(Some(mem)) => mem,
        Ok(None) => {
            log::warn!("Memory {} not found", memory_id);
            tracker.fail_operation(op_id, "Memory not found".to_string());
            return;
        }
        Err(e) => {
            log::error!("Failed to fetch memory {}: {:?}", memory_id, e);
            tracker.fail_operation(op_id, format!("Fetch error: {:?}", e));
            return;
        }
    };

    // Evaluate with timeout + retry
    match evaluate_with_timeout_and_retry(evaluator, &memory.content.text, 2).await {
        Ok(score) => {
            // Store quality score
            memory.metadata.set_custom("quality_score", score).ok();
            memory
                .metadata
                .set_custom("evaluation_status", "Success")
                .ok();

            // Update memory in database
            if let Err(e) = manager.update_memory(memory).await {
                log::error!("Failed to update memory {}: {:?}", memory_id, e);
                tracker.fail_operation(op_id, format!("Update error: {:?}", e));
            } else {
                log::info!(
                    "Committee evaluation completed: {} (score: {:.2})",
                    memory_id,
                    score
                );
                tracker.complete_operation(op_id);
            }
        }
        Err(e) => {
            // Evaluation failed after retries
            log::error!(
                "Committee evaluation exhausted retries for {}: {}",
                memory_id,
                e
            );

            memory.metadata.set_custom("quality_score", 0.5).ok();
            memory
                .metadata
                .set_custom("evaluation_status", "Failed")
                .ok();
            memory.metadata.set_custom("error_message", e.clone()).ok();

            manager.update_memory(memory).await.ok();
            tracker.fail_operation(op_id, e);
        }
    }

    let duration = start_time.elapsed();
    log::debug!("Committee evaluation took {:?} for {}", duration, memory_id);
}

/// Evaluate with timeout and retry for transient failures
async fn evaluate_with_timeout_and_retry(
    evaluator: &ModelCommitteeEvaluator,
    content: &str,
    max_retries: u32,
) -> Result<f64, String> {
    let mut attempt = 0;
    let mut backoff_ms = 100u64;

    loop {
        // Wrap evaluation in 10-second timeout
        let eval_future = evaluator.evaluate(content);
        match timeout(Duration::from_secs(10), eval_future).await {
            Ok(Ok(score)) => {
                log::debug!("Evaluation succeeded on attempt {}", attempt + 1);
                return Ok(score);
            }
            Ok(Err(e)) if attempt < max_retries => {
                // Cognitive error - retry with backoff
                attempt += 1;
                log::warn!(
                    "Evaluation attempt {}/{} failed: {:?}, retrying in {}ms",
                    attempt,
                    max_retries + 1,
                    e,
                    backoff_ms
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2; // Exponential backoff
            }
            Ok(Err(e)) => {
                // Max retries exceeded
                return Err(format!(
                    "Evaluation failed after {} attempts: {:?}",
                    attempt + 1,
                    e
                ));
            }
            Err(_) if attempt < max_retries => {
                // Timeout - retry
                attempt += 1;
                log::warn!(
                    "Evaluation timeout on attempt {}/{}, retrying in {}ms",
                    attempt,
                    max_retries + 1,
                    backoff_ms
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2;
            }
            Err(_) => {
                return Err(format!(
                    "Evaluation timed out after {} attempts",
                    attempt + 1
                ));
            }
        }
    }
}

/// Process batch of memories for committee evaluation
pub(crate) async fn process_batch_evaluation(
    memory_manager: &SurrealDBMemoryManager,
    committee_evaluator: &ModelCommitteeEvaluator,
    memory_ids: Vec<String>,
) {
    let manager = memory_manager;
    let evaluator = committee_evaluator;

    log::info!(
        "Processing batch evaluation for {} memories",
        memory_ids.len()
    );

    // Collect memory contents
    let mut memories = Vec::new();
    for id in &memory_ids {
        match manager.get_memory(id).await {
            Ok(Some(memory)) => {
                memories.push((id.clone(), memory.content.text.clone()));
            }
            Ok(None) => {
                log::warn!("Memory {} not found for batch evaluation", id);
            }
            Err(e) => {
                log::error!("Failed to retrieve memory {}: {:?}", id, e);
            }
        }
    }

    if memories.is_empty() {
        log::warn!("No memories to evaluate in batch");
        return;
    }

    // Evaluate batch
    match evaluator.evaluate_batch(&memories).await {
        Ok(results) => {
            log::info!(
                "Batch evaluation successful: {} scores returned",
                results.len()
            );

            // Update each memory with its score
            for (id, score) in results {
                match manager.get_memory(&id).await {
                    Ok(Some(mut memory)) => {
                        // Update quality score in metadata
                        memory.metadata.set_custom("quality_score", score).ok();
                        memory
                            .metadata
                            .set_custom("evaluation_status", "Success")
                            .ok();
                        memory
                            .metadata
                            .set_custom("evaluated_at", chrono::Utc::now().to_rfc3339())
                            .ok();
                        memory
                            .metadata
                            .set_custom("evaluation_method", "batch_committee")
                            .ok();

                        // Update memory
                        match manager.update_memory(memory).await {
                            Ok(_) => {
                                log::debug!("Updated memory {} with score {:.3}", id, score);
                            }
                            Err(e) => {
                                log::error!("Failed to update memory {}: {:?}", id, e);
                            }
                        }
                    }
                    Ok(None) => {
                        log::warn!("Memory {} disappeared during batch processing", id);
                    }
                    Err(e) => {
                        log::error!("Failed to retrieve memory {} for update: {:?}", id, e);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Batch evaluation failed: {:?}", e);

            // Mark all memories as failed
            for id in &memory_ids {
                if let Ok(Some(mut memory)) = manager.get_memory(id).await {
                    memory
                        .metadata
                        .set_custom("evaluation_status", "Failed")
                        .ok();
                    memory
                        .metadata
                        .set_custom("error_message", e.to_string())
                        .ok();
                    let _ = manager.update_memory(memory).await;
                }
            }
        }
    }
}
