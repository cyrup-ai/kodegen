use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use log::{info, warn};

use super::capabilities::{
    image_embedding_pool, text_embedding_pool, text_to_image_pool, text_to_text_pool, vision_pool,
};
use super::core::Pool;

/// Begin shutdown sequence for all pools
///
/// Process:
/// 1. Set shutdown flag on all pools (reject new requests)
/// 2. Wait up to `timeout_secs` for in-flight requests to complete
/// 3. Check every 100ms if all pools have drained
/// 4. Exit early if drained, or force exit after timeout
///
/// # Arguments
/// * `timeout_secs` - Maximum seconds to wait for drain (typically 5)
pub async fn begin_shutdown(timeout_secs: u64) {
    info!(
        "Shutdown signal received, draining pools (timeout: {}s)...",
        timeout_secs
    );

    // Step 1: Signal all pools to stop accepting new requests
    text_embedding_pool().begin_shutdown();
    text_to_text_pool().begin_shutdown();
    image_embedding_pool().begin_shutdown();
    vision_pool().begin_shutdown();
    text_to_image_pool().begin_shutdown();

    // Step 2: Wait for drain with periodic checks
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        // Check if drain period completed
        if all_pools_drained() {
            let elapsed = start.elapsed().as_secs_f64();
            info!(
                "Graceful shutdown complete ({:.2}s, all queues drained)",
                elapsed
            );
            return;
        }

        // Check if timeout reached
        if start.elapsed() >= timeout {
            let in_flight = count_in_flight_requests();
            warn!(
                "Shutdown timeout reached ({}s), forcing exit with {} in-flight requests",
                timeout_secs, in_flight
            );
            return;
        }

        // Brief sleep before next check
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Check if all pools have drained (no pending requests)
fn all_pools_drained() -> bool {
    count_pool_pending(text_embedding_pool()) == 0
        && count_pool_pending(text_to_text_pool()) == 0
        && count_pool_pending(image_embedding_pool()) == 0
        && count_pool_pending(vision_pool()) == 0
        && count_pool_pending(text_to_image_pool()) == 0
}

/// Count pending requests in a pool
///
/// Iterates all workers across all registry keys and sums pending_requests.
fn count_pool_pending<T: super::core::types::PoolWorkerHandle>(pool: &Pool<T>) -> usize {
    let mut total = 0;

    // Iterate DashMap entries (registry_key -> Vec<WorkerHandle>)
    for entry in pool.workers().iter() {
        let workers = entry.value();
        for worker in workers {
            total += worker.core().pending_requests.load(Ordering::Acquire);
        }
    }

    total
}

/// Count total in-flight requests across all pools
fn count_in_flight_requests() -> usize {
    count_pool_pending(text_embedding_pool())
        + count_pool_pending(text_to_text_pool())
        + count_pool_pending(image_embedding_pool())
        + count_pool_pending(vision_pool())
        + count_pool_pending(text_to_image_pool())
}
