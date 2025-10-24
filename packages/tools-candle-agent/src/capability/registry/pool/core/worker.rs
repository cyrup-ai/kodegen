//! Generic worker spawn and lifecycle helpers
//!
//! # Worker Pattern
//!
//! Workers are generic over capability traits. Each worker:
//! 1. Owns loaded model exclusively (no Arc<Mutex<>>)
//! 2. Processes requests from tokio mpsc channels
//! 3. Self-schedules via channel recv() (first available gets request)
//! 4. Runs infinite loop until shutdown signal
//!
//! # Capability-Specific Implementation
//!
//! Worker loops are implemented in `pool/capabilities/{trait_name}.rs`:
//! - TextEmbedding: embed_rx, batch_embed_rx channels
//! - TextToText: prompt_rx channel
//! - ImageEmbedding: embed_image_rx, embed_image_url_rx, etc.
//! - Vision: describe_image_rx, describe_url_rx
//! - TextToImage: generate_image_rx channel
//!
//! # Memory Management
//!
//! Before spawning worker, check memory with `check_memory_available()`.
//! After spawn: pool.add_memory(per_worker_mb)
//! After eviction: pool.remove_memory(per_worker_mb)
//!
//! # References
//!
//! - MODEL_POOL.md Scenario 1: Dynamic worker limits
//! - MODEL_POOL.md Scenario 4: Memory footprint calculation
//! - MODEL_POOL.md Scenario 5: Worker lifecycle

use super::PoolError;
use crate::domain::model::traits::CandleModel;
use std::thread;

/// Spawn worker thread with model loader closure
///
/// Generic pattern for all capability types:
/// 1. Worker thread spawned
/// 2. Model loaded via closure (happens in worker thread)
/// 3. Worker loop starts, processes requests until shutdown
/// 4. Worker owns model exclusively (no Arc<Mutex<>>)
pub fn spawn_worker_thread<T, F>(
    model_loader: F,
    worker_name: String,
) -> Result<thread::JoinHandle<()>, PoolError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PoolError> + Send + 'static,
{
    thread::Builder::new()
        .name(worker_name)
        .spawn(move || {
            // Load model in worker thread
            let _model = match model_loader() {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Worker model loading failed: {}", e);
                    return;
                }
            };

            // Worker loop implemented by capability-specific modules
            // (model passed to capability-specific worker loop)
        })
        .map_err(|e| PoolError::SpawnFailed(format!("Thread spawn failed: {}", e)))
}

/// Check if spawning N workers would exceed 80% memory limit
///
/// Formula (from Scenario 4):
/// ```
/// current_usage_mb + (num_workers * per_worker_mb) <= 0.80 * total_system_mb
/// ```
pub fn check_memory_available<T: CandleModel>(
    model: &T,
    current_usage_mb: usize,
    num_workers: usize,
) -> Result<(), PoolError> {
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let required_mb = num_workers * per_worker_mb;

    // Query system memory
    let total_system_mb = query_system_memory_mb();
    let memory_limit_mb = (total_system_mb as f64 * 0.80) as usize;

    if current_usage_mb + required_mb > memory_limit_mb {
        return Err(PoolError::MemoryExhausted(format!(
            "Cannot spawn {} workers ({} MB). Current: {} MB, Limit: {} MB (80% of {})",
            num_workers, required_mb, current_usage_mb, memory_limit_mb, total_system_mb
        )));
    }

    Ok(())
}

/// Query total system memory in MB
fn query_system_memory_mb() -> usize {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_memory();
    (sys.total_memory() / 1024 / 1024) as usize
}
