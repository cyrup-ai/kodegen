use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, instrument, warn};

use super::capabilities::{
    image_embedding_pool, text_embedding_pool, text_to_image_pool, text_to_text_pool, vision_pool,
};
use super::core::Pool;

/// Check if all workers for a model are idle
///
/// A worker is considered idle if:
/// - It has no pending requests (pending_requests == 0)
/// - It hasn't been used for at least idle_threshold_secs seconds
/// - It's in an evictable state (Ready or Idle, not Loading or Processing)
fn all_workers_idle<W: super::core::types::PoolWorkerHandle>(
    workers: &[W],
    idle_threshold_secs: u64,
) -> bool {
    use super::core::worker_state::WorkerState;

    if workers.is_empty() {
        return false;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    workers.iter().all(|w| {
        let core = w.core();
        // CRITICAL: Don't evict workers that are Loading or Processing!
        let state = core.get_state();
        if matches!(
            state,
            WorkerState::Loading | WorkerState::Processing | WorkerState::Spawning
        ) {
            return false; // Not evictable
        }

        let pending = core.pending_requests.load(Ordering::Acquire);
        let last_used = core.last_used.load(Ordering::Acquire);
        let idle_duration = now.saturating_sub(last_used);

        pending == 0 && idle_duration >= idle_threshold_secs
    })
}

/// Find least recently used (LRU) worker
///
/// Returns the index of the worker with the oldest last_used timestamp.
/// Returns None if workers vector is empty.
fn find_lru_worker<W: super::core::types::PoolWorkerHandle>(workers: &[W]) -> Option<usize> {
    workers
        .iter()
        .enumerate()
        .min_by_key(|(_, w)| w.core().last_used.load(Ordering::Acquire))
        .map(|(idx, _)| idx)
}

/// Remove dead and failed workers from pool
#[instrument(skip(pool))]
fn cleanup_dead_workers<W: super::core::types::PoolWorkerHandle>(pool: &Pool<W>) {
    use super::core::worker_state::WorkerState;

    for entry in pool.workers().iter() {
        let registry_key = entry.key();
        let mut removed_count = 0;

        if let Some(mut workers) = pool.workers().get_mut(registry_key) {
            workers.retain(|worker| {
                let state = worker.core().get_state();

                if matches!(state, WorkerState::Dead | WorkerState::Failed) {
                    warn!(
                        worker_id = worker.core().worker_id,
                        registry_key = %registry_key,
                        state = ?state,
                        "Removing dead worker"
                    );

                    // Clean up memory
                    pool.remove_memory(worker.core().per_worker_mb);

                    removed_count += 1;
                    false // Remove from vector
                } else {
                    true // Keep in vector
                }
            });
        }

        if removed_count > 0 {
            pool.metrics()
                .workers_evicted
                .fetch_add(removed_count, Ordering::Release);
        }
    }
}

/// Evict worker from pool
///
/// Removes the worker at the specified index, sends shutdown signal,
/// and updates memory tracking.
///
/// # Arguments
/// * `pool` - The pool to evict from
/// * `registry_key` - Model registry key
/// * `worker_idx` - Index of worker to evict
/// * `per_worker_mb` - Memory footprint of worker in MB
///
/// # Returns
/// Ok(()) on success, Err with description on failure
#[instrument(skip(pool), fields(registry_key = %registry_key, worker_idx = worker_idx))]
fn evict_worker<W: super::core::types::PoolWorkerHandle>(
    pool: &Pool<W>,
    registry_key: &str,
    worker_idx: usize,
    per_worker_mb: usize,
) -> Result<(), String> {
    // Get mutable access to workers vector
    let mut workers_guard = pool
        .workers()
        .get_mut(registry_key)
        .ok_or_else(|| format!("No workers for {}", registry_key))?;

    if worker_idx >= workers_guard.len() {
        return Err(format!(
            "Worker index {} out of bounds (len: {})",
            worker_idx,
            workers_guard.len()
        ));
    }

    // Remove worker from vector
    let worker = workers_guard.remove(worker_idx);
    let remaining_count = workers_guard.len();
    drop(workers_guard); // Release lock

    // Send shutdown signal to worker thread
    // Worker loop will receive signal and break
    if let Err(e) = worker.core().shutdown_tx.send(()) {
        warn!(
            worker_id = worker.core().worker_id,
            error = %e,
            "Failed to send shutdown signal"
        );
    }

    // Update memory tracking
    pool.remove_memory(per_worker_mb);

    // Update metrics
    pool.metrics()
        .workers_evicted
        .fetch_add(1, Ordering::Release);

    info!(
        worker_id = worker.core().worker_id,
        remaining_count = remaining_count,
        "Evicted worker (idle cooldown)"
    );

    Ok(())
}

/// Validate health of all workers in a pool
///
/// Checks each worker's health and removes dead workers.
/// Should be called before idle eviction to ensure accurate state.
#[instrument(skip(pool))]
fn validate_pool_health<T: super::core::types::PoolWorkerHandle>(
    pool: &'static Pool<T>,
    pool_name: &str,
) {
    for entry in pool.workers().iter() {
        let registry_key = entry.key();
        let removed = pool.validate_workers(registry_key);

        if removed > 0 {
            warn!(
                pool_name = %pool_name,
                removed = removed,
                registry_key = %registry_key,
                "Removed dead workers"
            );
        }
    }
}

/// Process maintenance for one pool
///
/// Iterates over all models in the pool and evicts one LRU worker
/// per idle model.
fn process_pool_maintenance<W: super::core::types::PoolWorkerHandle>(
    pool: &'static Pool<W>,
    idle_threshold_secs: u64,
    pool_name: &str,
) {
    // FIRST: Clean up dead/failed workers
    cleanup_dead_workers(pool);

    // Collect models that need eviction (to avoid holding locks)
    let mut models_to_evict = Vec::new();

    // Scan all models in pool
    for entry in pool.workers().iter() {
        let registry_key = entry.key().clone();
        let workers = entry.value();

        // Check if all workers are idle and there's at least one worker
        if all_workers_idle(workers, idle_threshold_secs) && !workers.is_empty() {
            // Find LRU worker index
            if let Some(lru_idx) = find_lru_worker(workers) {
                models_to_evict.push((registry_key, lru_idx));
            }
        }
    }

    // Perform evictions (after releasing iterator locks)
    for (registry_key, lru_idx) in models_to_evict {
        // Get per_worker_mb from the worker handle
        let per_worker_mb = pool
            .workers()
            .get(&registry_key)
            .and_then(|workers| workers.get(lru_idx).map(|w| w.core().per_worker_mb))
            .unwrap_or(1024); // Default fallback if worker not found

        debug!(
            pool_name = %pool_name,
            registry_key = %registry_key,
            lru_idx = lru_idx,
            per_worker_mb = per_worker_mb,
            "All workers idle, evicting LRU worker"
        );

        if let Err(e) = evict_worker(pool, &registry_key, lru_idx, per_worker_mb) {
            warn!(
                pool_name = %pool_name,
                registry_key = %registry_key,
                error = %e,
                "Failed to evict worker"
            );
        }
    }
}

/// Log current memory usage across all pools
fn log_memory_usage() {
    let text_embedding_mb = text_embedding_pool().total_memory_mb();
    let text_to_text_mb = text_to_text_pool().total_memory_mb();
    let image_embedding_mb = image_embedding_pool().total_memory_mb();
    let vision_mb = vision_pool().total_memory_mb();
    let text_to_image_mb = text_to_image_pool().total_memory_mb();

    let total_mb =
        text_embedding_mb + text_to_text_mb + image_embedding_mb + vision_mb + text_to_image_mb;

    debug!(
        total_mb = total_mb,
        text_embedding_mb = text_embedding_mb,
        text_to_text_mb = text_to_text_mb,
        image_embedding_mb = image_embedding_mb,
        vision_mb = vision_mb,
        text_to_image_mb = text_to_image_mb,
        "Pool memory usage"
    );
}

/// Start maintenance thread for all pools
///
/// Runs every 1 minute (configurable via pool config):
/// - Check each pool for idle workers
/// - Evict 1 LRU worker per idle model
/// - Monitor system memory pressure
/// - Log eviction events
///
/// The thread continues until all pools signal shutdown.
pub fn start_maintenance_thread() -> Result<tokio::task::JoinHandle<()>, String> {
    // Get interval from config (default 60 seconds)
    let config = text_embedding_pool().config();
    let interval = Duration::from_secs(config.maintenance_interval_secs);
    let idle_threshold = config.cooldown_idle_minutes * 60; // Convert minutes to seconds

    info!(
        interval_secs = interval.as_secs(),
        idle_threshold_secs = idle_threshold,
        "Maintenance thread started"
    );

    Ok(tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;

            // Check if shutting down (check all pools, exit if any shutting down)
            if text_embedding_pool().is_shutting_down()
                || text_to_text_pool().is_shutting_down()
                || image_embedding_pool().is_shutting_down()
                || vision_pool().is_shutting_down()
                || text_to_image_pool().is_shutting_down()
            {
                info!("Maintenance thread shutting down");
                break;
            }

            // Validate worker health (remove dead workers)
            validate_pool_health(text_embedding_pool(), "TextEmbedding");
            validate_pool_health(text_to_text_pool(), "TextToText");
            validate_pool_health(image_embedding_pool(), "ImageEmbedding");
            validate_pool_health(vision_pool(), "Vision");
            validate_pool_health(text_to_image_pool(), "TextToImage");

            // Process each pool (evict idle workers)
            process_pool_maintenance(text_embedding_pool(), idle_threshold, "TextEmbedding");
            process_pool_maintenance(text_to_text_pool(), idle_threshold, "TextToText");
            process_pool_maintenance(image_embedding_pool(), idle_threshold, "ImageEmbedding");
            process_pool_maintenance(vision_pool(), idle_threshold, "Vision");
            process_pool_maintenance(text_to_image_pool(), idle_threshold, "TextToImage");

            // Log memory usage
            log_memory_usage();
        }

        info!("Maintenance thread exited");
    }))
}
