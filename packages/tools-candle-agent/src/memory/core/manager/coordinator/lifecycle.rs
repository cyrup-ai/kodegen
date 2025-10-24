//! Memory coordinator lifecycle management

use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;
use tokio::sync::RwLock;

use crate::capability::registry::TextEmbeddingModel;
use crate::domain::memory::cognitive::types::CognitiveState;
use crate::memory::cognitive::committee::ModelCommitteeEvaluator;
use crate::memory::cognitive::quantum::{QuantumRouter, QuantumState};
use crate::memory::core::cognitive_queue::CognitiveProcessingQueue;
use crate::memory::core::manager::surreal::SurrealDBMemoryManager;
use crate::memory::repository::MemoryRepository;
use crate::memory::utils::{Error, Result};

use super::types::LazyEvalStrategy;

/// High-level memory manager that uses SurrealDB's native capabilities directly
///
/// Note: cognitive_queue, committee_evaluator, quantum_router, and quantum_state
/// are wired in but not used until COGMEM_4 worker implementation
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MemoryCoordinator {
    pub(in crate::memory::core) surreal_manager: Arc<SurrealDBMemoryManager>,
    pub(super) repository: Arc<RwLock<MemoryRepository>>,
    pub(super) embedding_model: TextEmbeddingModel,
    // NEW COGNITIVE FIELDS:
    pub(super) cognitive_queue: Arc<CognitiveProcessingQueue>,
    pub(super) committee_evaluator: Arc<ModelCommitteeEvaluator>,
    pub(super) quantum_router: Arc<QuantumRouter>,
    pub(in crate::memory::core) quantum_state: Arc<RwLock<QuantumState>>,
    pub(super) cognitive_state: Arc<RwLock<CognitiveState>>,
    pub(super) cognitive_workers: Arc<tokio::sync::RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    // LAZY EVALUATION FIELDS:
    pub(super) lazy_eval_strategy: LazyEvalStrategy,
    pub(super) evaluation_cache: Cache<String, f64>,
    // TEMPORAL DECAY:
    pub(in crate::memory::core) decay_rate: f64,
    pub(super) decay_shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl MemoryCoordinator {
    /// Create a new memory coordinator with SurrealDB and embedding model
    pub async fn new(
        surreal_manager: Arc<SurrealDBMemoryManager>,
        embedding_model: TextEmbeddingModel,
    ) -> Result<Self> {
        // Initialize committee evaluator with error handling
        // Note: ModelCommitteeEvaluator::new() is async and returns Result<Self, CognitiveError>
        let committee_evaluator = Arc::new(
            ModelCommitteeEvaluator::new()
                .await
                .map_err(|e| Error::Internal(format!("Failed to init committee: {:?}", e)))?,
        );

        let cognitive_queue = Arc::new(CognitiveProcessingQueue::new());
        let quantum_router = Arc::new(QuantumRouter::default());

        // Spawn cognitive workers as async tasks (now Send-compatible)
        let num_workers = 2;

        for worker_id in 0..num_workers {
            let queue = cognitive_queue.clone();
            let manager = surreal_manager.clone();
            let evaluator = committee_evaluator.clone();

            let worker = crate::memory::core::cognitive_worker::CognitiveWorker::new(
                queue, manager, evaluator,
            );

            // Spawn on main tokio runtime (workers are Send now)
            tokio::spawn(async move {
                log::info!("Cognitive worker {} started", worker_id);
                worker.run().await;
                log::info!("Cognitive worker {} stopped", worker_id);
            });
        }

        log::info!("Started {} cognitive worker tasks", num_workers);

        // Load entanglement graph from database into memory (prebuilt graph pattern)
        // This enables entanglement boost during search without query overhead
        let entanglement_links = match surreal_manager.load_all_entanglement_edges().await {
            Ok(links) => {
                log::info!(
                    "Loaded {} entanglement edges into quantum graph ({} total)",
                    links.len(),
                    links.len()
                );
                links
            }
            Err(e) => {
                log::error!(
                    "Failed to load entanglement graph from database: {:?}. Starting with empty graph - entanglement boost will be disabled.",
                    e
                );
                Vec::new()
            }
        };

        // Populate quantum state with the prebuilt graph
        let quantum_state_instance = QuantumState {
            coherence_level: 1.0,
            entanglement_links,
            measurement_count: 0,
        };

        // Create shutdown channel for decay worker
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let coordinator = Self {
            surreal_manager,
            repository: Arc::new(RwLock::new(MemoryRepository::new())),
            embedding_model,
            cognitive_queue,
            committee_evaluator,
            quantum_router,
            quantum_state: Arc::new(RwLock::new(quantum_state_instance)),
            cognitive_state: Arc::new(RwLock::new(CognitiveState::new())),
            cognitive_workers: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            lazy_eval_strategy: LazyEvalStrategy::default(),
            evaluation_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            decay_rate: 0.1,
            decay_shutdown_tx: Some(shutdown_tx),
        };

        // Spawn decay worker for background temporal decay processing
        let coordinator_arc = Arc::new(coordinator);
        let decay_config = crate::memory::core::decay_worker::DecayWorkerConfig::default();

        let decay_worker = crate::memory::core::decay_worker::DecayWorker::new(
            coordinator_arc.clone(),
            decay_config,
            shutdown_rx,
        );

        tokio::spawn(async move {
            log::info!("Decay worker started");
            decay_worker.run().await;
        });

        // Return Arc-wrapped coordinator to match spawn pattern
        Ok(Arc::try_unwrap(coordinator_arc).unwrap_or_else(|arc| (*arc).clone()))
    }

    /// Configure lazy evaluation strategy
    pub fn set_lazy_eval_strategy(&mut self, strategy: LazyEvalStrategy) {
        self.lazy_eval_strategy = strategy;
    }

    /// Set the decay rate for temporal importance decay
    ///
    /// # Arguments
    /// * `rate` - Decay rate (recommended: 0.01 to 0.5)
    ///   - 0.01: Very slow decay (memories stay relevant longer)
    ///   - 0.1: Default balanced decay
    ///   - 0.5: Fast decay (strong recency bias)
    pub fn set_decay_rate(&mut self, rate: f64) -> Result<()> {
        if rate <= 0.0 || rate > 1.0 {
            return Err(Error::InvalidInput(
                "Decay rate must be between 0.0 and 1.0".into(),
            ));
        }
        self.decay_rate = rate;
        log::info!("Temporal decay rate updated to {}", rate);
        Ok(())
    }

    /// Get current decay rate
    pub fn get_decay_rate(&self) -> f64 {
        self.decay_rate
    }

    /// Shutdown all cognitive worker tasks gracefully
    pub fn shutdown_workers(&mut self) {
        // Flush any pending batches before shutdown
        if let Err(e) = self.cognitive_queue.flush_batches() {
            log::warn!("Failed to flush batches during shutdown: {}", e);
        }

        // Signal decay worker to shutdown
        if let Some(shutdown_tx) = &self.decay_shutdown_tx {
            if let Err(e) = shutdown_tx.send(true) {
                log::warn!("Failed to send shutdown signal to decay worker: {}", e);
            } else {
                log::info!("Decay worker shutdown signal sent");
            }
        }

        // Note: Tokio tasks will be cancelled when runtime shuts down
        // We don't await them here since this method is sync
        // The queue channel will be dropped, causing workers to exit their loops
        log::info!("Cognitive workers will shut down when queue is closed");
    }
}
