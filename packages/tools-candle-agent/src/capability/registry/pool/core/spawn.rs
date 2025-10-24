//! Generic cold-start helper for eliminating duplication in registry.rs
//!
//! This module provides `ensure_workers_spawned()` which encapsulates the
//! decision logic for spawning workers that was previously duplicated 42+ times.

use super::memory_governor::{AllocationGuard, MemoryGovernor};
use super::{Pool, PoolError, SpawnGuard};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, instrument};

/// Ensure workers are spawned for a model (cold-start helper with race protection)
///
/// Encapsulates the decision logic:
/// 1. Try to acquire spawn lock (prevents race conditions)
/// 2. If lock acquired: spawn workers and release lock
/// 3. If lock busy: wait for other thread to complete spawning
///
/// # Parameters
/// - `pool`: Pool instance (must have has_workers and memory_governor)
/// - `registry_key`: Model registry key
/// - `per_worker_mb`: Memory per worker (from model.info().est_memory_allocation_mb)
/// - `spawn_fn`: Closure that spawns ONE worker with AllocationGuard, called N times
///
/// # Returns
/// - `Ok(())` if workers already exist or spawned successfully
/// - `Err(PoolError)` if memory exhausted, spawn failed, or timeout
///
/// # Example Usage
/// ```rust,ignore
/// ensure_workers_spawned(
///     pool,
///     registry_key,
///     per_worker_mb,
///     |_worker_idx, allocation_guard| {
///         let m_clone = m.clone();
///         pool.spawn_text_embedding_worker(
///             registry_key,
///             move || LoadedGteQwenModel::load(&m_clone)
///                 .map_err(|e| PoolError::SpawnFailed(e.to_string())),
///             per_worker_mb,
///             allocation_guard,
///         )
///     }
/// )?;
/// ```
#[instrument(skip(pool, spawn_fn), fields(registry_key = %registry_key, per_worker_mb = per_worker_mb))]
pub async fn ensure_workers_spawned<P, F>(
    pool: &P,
    registry_key: &str,
    per_worker_mb: usize,
    spawn_fn: F,
) -> Result<(), PoolError>
where
    P: HasWorkers + MemoryGovernorAccess + SpawnLock,
    F: Fn(usize, AllocationGuard) -> Result<(), PoolError>,
{
    // 1. Try to acquire spawn lock (prevents race conditions)
    if let Some(_guard) = pool.try_acquire_spawn_lock(registry_key) {
        // Double-check workers don't exist (another thread may have spawned before lock)
        if pool.has_workers(registry_key) {
            return Ok(());
        }

        let governor = pool.memory_governor();

        // 2. Decide worker count based on memory governor allocation
        let workers_to_spawn = if let Ok(_guard1) = governor.try_allocate(per_worker_mb).await {
            // First worker fits
            if let Ok(_guard2) = governor.try_allocate(per_worker_mb).await {
                // Second worker also fits - release both guards, will re-allocate in spawn
                drop(_guard1);
                drop(_guard2);
                2
            } else {
                // Only first fits
                drop(_guard1);
                1
            }
        } else {
            return Err(PoolError::MemoryExhausted(format!(
                "Memory governor rejected allocation for {}",
                registry_key
            )));
        };

        // 3. Spawn N workers with allocation guards
        for worker_idx in 0..workers_to_spawn {
            // Allocate with guard - will auto-release on panic/error
            let allocation_guard = governor
                .try_allocate(per_worker_mb)
                .await
                .map_err(|e| PoolError::MemoryExhausted(e.to_string()))?;

            spawn_fn(worker_idx, allocation_guard)?;

            // Guard transferred to worker thread, will drop when worker exits
        }

        info!(
            workers_spawned = workers_to_spawn,
            "Workers spawned successfully"
        );

        // _guard drops here, releasing spawn lock
        Ok(())
    } else {
        // Another thread is spawning - wait for it to complete
        // 6 hour timeout allows for large model downloads (e.g., Llama 70B ~40GB)
        // on slow connections without premature failures
        pool.wait_for_workers(registry_key, Duration::from_secs(6 * 3600))
            .await
    }
}

/// Adaptive worker spawning with load-based scaling
///
/// Extends ensure_workers_spawned with adaptive scaling:
/// - Cold start (0 workers): spawn 1-2 workers as before
/// - All workers busy: spawn 1 additional worker (up to max_workers)
///
/// # Parameters
/// - `pool`: Pool instance
/// - `registry_key`: Model registry key
/// - `per_worker_mb`: Memory per worker
/// - `max_workers`: Maximum workers to spawn (from config.max_workers_per_model)
/// - `spawn_fn`: Closure that spawns ONE worker with AllocationGuard
///
/// # Returns
/// - `Ok(())` if workers exist, spawned, or at max capacity
/// - `Err(PoolError)` if memory exhausted or spawn failed
#[instrument(skip(pool, spawn_fn), fields(registry_key = %registry_key, max_workers = max_workers))]
pub async fn ensure_workers_spawned_adaptive<P, F>(
    pool: &P,
    registry_key: &str,
    per_worker_mb: usize,
    max_workers: usize,
    spawn_fn: F,
) -> Result<(), PoolError>
where
    P: HasWorkers + MemoryGovernorAccess + SpawnLock + WorkerMetrics,
    F: Fn(usize, AllocationGuard) -> Result<(), PoolError>,
{
    let worker_count = pool.worker_count(registry_key);

    debug!(worker_count = worker_count, "Adaptive scaling check");

    // Cold start: spawn 1-2 workers
    if worker_count == 0 {
        if let Some(_guard) = pool.try_acquire_spawn_lock(registry_key) {
            // Double-check after lock
            if pool.has_workers(registry_key) {
                return Ok(());
            }

            let governor = pool.memory_governor();

            // Decide worker count based on memory governor allocation
            let workers_to_spawn = if let Ok(_guard1) = governor.try_allocate(per_worker_mb).await {
                // First worker fits
                if let Ok(_guard2) = governor.try_allocate(per_worker_mb).await {
                    // Second worker also fits - release both guards, will re-allocate in spawn
                    drop(_guard1);
                    drop(_guard2);
                    2
                } else {
                    // Only first fits
                    drop(_guard1);
                    1
                }
            } else {
                return Err(PoolError::MemoryExhausted(format!(
                    "Memory governor rejected allocation for {}",
                    registry_key
                )));
            };

            // Spawn N workers with allocation guards
            for worker_idx in 0..workers_to_spawn {
                let allocation_guard = governor
                    .try_allocate(per_worker_mb)
                    .await
                    .map_err(|e| PoolError::MemoryExhausted(e.to_string()))?;

                spawn_fn(worker_idx, allocation_guard)?;
            }

            info!(
                workers_spawned = workers_to_spawn,
                "Workers spawned successfully for {}", registry_key
            );

            // Wait for at least one worker to be ready before returning
            // This prevents race condition where prompt request arrives before workers finish loading
            // 6 hour timeout allows for large model downloads on slow connections
            debug!(
                "Waiting for {} workers to become ready...",
                workers_to_spawn
            );
            return pool
                .wait_for_workers(registry_key, Duration::from_secs(6 * 3600))
                .await;
        } else {
            return pool
                .wait_for_workers(registry_key, Duration::from_secs(6 * 3600))
                .await;
        }
    }

    // Adaptive scaling: spawn +1 if all workers are busy
    if worker_count < max_workers {
        let busy_count = pool.busy_worker_count(registry_key);

        // If all workers are busy, try to spawn one more
        if busy_count >= worker_count
            && let Some(_guard) = pool.try_acquire_spawn_lock(registry_key)
        {
            // Double-check after acquiring lock
            let current_count = pool.worker_count(registry_key);
            let current_busy = pool.busy_worker_count(registry_key);

            if current_busy >= current_count && current_count < max_workers {
                let governor = pool.memory_governor();

                // Try to allocate memory for one more worker
                match governor.try_allocate(per_worker_mb).await {
                    Ok(allocation_guard) => {
                        info!(
                            current_count = current_count,
                            max_workers = max_workers,
                            "All workers busy, spawning 1 more"
                        );
                        spawn_fn(current_count, allocation_guard)?;
                    }
                    Err(_) => {
                        // Memory exhausted, can't spawn more workers (not an error, just at capacity)
                        debug!("Cannot spawn additional worker - memory limit reached");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Trait for pools that can check worker existence
pub trait HasWorkers {
    fn has_workers(&self, registry_key: &str) -> bool;
}

/// Trait for pools with memory governor access
pub trait MemoryGovernorAccess {
    fn memory_governor(&self) -> Arc<MemoryGovernor>;
}

/// Trait for pools that support spawn locking
pub trait SpawnLock {
    fn try_acquire_spawn_lock(&self, registry_key: &str) -> Option<SpawnGuard>;
    fn wait_for_workers(
        &self,
        registry_key: &str,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<(), PoolError>> + Send;
}

/// Trait for pools that provide worker metrics
pub trait WorkerMetrics {
    fn worker_count(&self, registry_key: &str) -> usize;
    fn busy_worker_count(&self, registry_key: &str) -> usize;
}

// Implement traits for Pool<W>
impl<W: super::types::PoolWorkerHandle> HasWorkers for Pool<W> {
    fn has_workers(&self, registry_key: &str) -> bool {
        Pool::has_workers(self, registry_key)
    }
}

impl<W: super::types::PoolWorkerHandle> MemoryGovernorAccess for Pool<W> {
    fn memory_governor(&self) -> Arc<MemoryGovernor> {
        self.memory_governor.clone()
    }
}

impl<W: super::types::PoolWorkerHandle> SpawnLock for Pool<W> {
    fn try_acquire_spawn_lock(&self, registry_key: &str) -> Option<SpawnGuard> {
        Pool::try_acquire_spawn_lock(self, registry_key)
    }

    async fn wait_for_workers(
        &self,
        registry_key: &str,
        timeout: Duration,
    ) -> Result<(), PoolError> {
        Pool::wait_for_workers(self, registry_key, timeout).await
    }
}

impl<W: super::types::PoolWorkerHandle> WorkerMetrics for Pool<W> {
    fn worker_count(&self, registry_key: &str) -> usize {
        self.workers()
            .get(registry_key)
            .map(|workers| workers.len())
            .unwrap_or(0)
    }

    fn busy_worker_count(&self, registry_key: &str) -> usize {
        self.workers()
            .get(registry_key)
            .map(|workers| {
                workers
                    .iter()
                    .filter(|w| {
                        w.core()
                            .pending_requests
                            .load(std::sync::atomic::Ordering::Acquire)
                            > 0
                    })
                    .count()
            })
            .unwrap_or(0)
    }
}
