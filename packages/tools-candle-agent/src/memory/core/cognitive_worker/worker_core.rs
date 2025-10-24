//! Core cognitive worker implementation
//!
//! Contains the main CognitiveWorker struct, constructor, run loop, and task dispatcher.
//! Delegates specialized processing to helper modules.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::memory::cognitive::types::{CognitiveMemory, CognitiveMemoryConfig};
use crate::memory::cognitive::committee::ModelCommitteeEvaluator;
use crate::memory::core::cognitive_queue::{
    CognitiveProcessingQueue, CognitiveTask, CognitiveTaskType,
};
use crate::memory::core::manager::surreal::SurrealDBMemoryManager;
use crate::memory::monitoring::operations::OperationTracker;

use super::{committee_evaluation, entanglement_discovery};

/// Background worker for processing cognitive tasks
pub struct CognitiveWorker {
    queue: Arc<CognitiveProcessingQueue>,
    memory_manager: Arc<SurrealDBMemoryManager>,
    committee_evaluator: Arc<ModelCommitteeEvaluator>,
    /// Cognitive memory for quantum features and pattern storage
    _cognitive_memory: Arc<RwLock<CognitiveMemory>>,
    /// Operation tracker for metrics
    operation_tracker: Arc<OperationTracker>,
}

impl CognitiveWorker {
    /// Create new cognitive worker
    pub fn new(
        queue: Arc<CognitiveProcessingQueue>,
        memory_manager: Arc<SurrealDBMemoryManager>,
        committee_evaluator: Arc<ModelCommitteeEvaluator>,
    ) -> Self {
        Self {
            queue,
            memory_manager,
            committee_evaluator,
            _cognitive_memory: Arc::new(RwLock::new(CognitiveMemory::new(
                CognitiveMemoryConfig::default(),
            ))),
            operation_tracker: Arc::new(OperationTracker::new(10_000)),
        }
    }

    /// Main worker loop - processes tasks from queue (async, yields)
    pub async fn run(&self) {
        log::info!("Cognitive worker started, waiting for tasks...");

        loop {
            // Async dequeue - yields until work arrives
            match self.queue.dequeue().await {
                Ok(task) => {
                    log::debug!(
                        "Task dequeued: type={:?}, memory_id={}, priority={}",
                        task.task_type,
                        task.memory_id,
                        task.priority
                    );

                    // Dispatch to handler
                    self.process_task(task).await;
                }
                Err(e) => {
                    log::error!("Worker dequeue error: {}", e);
                    // Channel disconnected - exit worker
                    break;
                }
            }
        }

        log::info!("Cognitive worker stopped");
    }

    /// Get current operation metrics for monitoring
    pub fn metrics(&self) -> Arc<OperationTracker> {
        self.operation_tracker.clone()
    }

    /// Get queue depth for monitoring
    pub fn queue_depth(&self) -> usize {
        self.queue.get_depth()
    }

    /// Route task to appropriate handler module
    async fn process_task(&self, task: CognitiveTask) {
        match task.task_type {
            CognitiveTaskType::CommitteeEvaluation => {
                committee_evaluation::process_committee_evaluation(
                    &self.memory_manager,
                    &self.committee_evaluator,
                    &self.operation_tracker,
                    &task.memory_id,
                )
                .await;
            }
            CognitiveTaskType::EntanglementDiscovery => {
                entanglement_discovery::process_entanglement_discovery(
                    &self.memory_manager,
                    &self.operation_tracker,
                    &task.memory_id,
                )
                .await;
            }
            CognitiveTaskType::QuantumRouting => {
                log::debug!("QuantumRouting task deferred to COGMEM_6");
            }
            CognitiveTaskType::BatchProcessing(memory_ids) => {
                committee_evaluation::process_batch_evaluation(
                    &self.memory_manager,
                    &self.committee_evaluator,
                    memory_ids,
                )
                .await;
            }
        }
    }
}
