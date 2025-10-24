//! Worker pool infrastructure for keeping models loaded in memory
//!
//! # Architecture
//!
//! The pool system provides transparent request routing to pre-loaded model workers,
//! eliminating per-request model loading overhead. The system consists of:
//!
//! - **5 global pool instances** (one per capability trait)
//! - **Worker threads** that own models exclusively (no locking)
//! - **Maintenance thread** that evicts idle workers to free memory
//! - **Tokio mpsc channels** for zero-allocation request routing
//!
//! ## The 5 Global Pools
//!
//! Each capability trait has a dedicated pool:
//!
//! 1. **TEXT_EMBEDDING_POOL**: [`TextEmbeddingCapable`](crate::capability::traits::TextEmbeddingCapable) models
//!    - Stella (1 model)
//!    - Operations: `embed()`, `batch_embed()`
//!
//! 2. **TEXT_TO_TEXT_POOL**: [`TextToTextCapable`](crate::capability::traits::TextToTextCapable) models
//!    - Qwen3Quantized (1 model)
//!    - Operations: `prompt()`
//!
//! 3. **IMAGE_EMBEDDING_POOL**: [`ImageEmbeddingCapable`](crate::capability::traits::ImageEmbeddingCapable) models
//!    - ClipVision
//!    - Operations: `embed_image()`
//!
//! 4. **VISION_POOL**: [`VisionCapable`](crate::capability::traits::VisionCapable) models
//!    - LLaVA
//!    - Operations: `process_vision()`
//!
//! 5. **TEXT_TO_IMAGE_POOL**: [`TextToImageCapable`](crate::capability::traits::TextToImageCapable) models
//!    - FLUX Schnell, Stable Diffusion 3.5 Turbo
//!    - Operations: `generate_image()`
//!
//! ## Usage
//!
//! Pool integration is **transparent** to users. Simply call registry methods:
//!
//! ```rust,no_run
//! use kodegen_candle_agent::capability::registry::{self, TextEmbeddingModel};
//!
//! // Get model from registry (pool integration happens automatically)
//! let model = registry::get::<TextEmbeddingModel>("dunzhang/stella_en_1.5B_v5")?;
//!
//! // First call: pool spawns 2 workers (cold start)
//! let embedding = model.embed("hello world", None)?;
//!
//! // Subsequent calls: routed to existing workers (no loading overhead)
//! let embedding2 = model.embed("goodbye", None)?;
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Initialization
//!//! Call [`init_maintenance()`] once at application startup to start the maintenance thread:
//!
//! ```rust
//! // In main.rs or CLI runner
//! kodegen_candle_agent::pool::init_maintenance();
//! ```
//!
//! This is done automatically in the CLI runner ([`crate::cli::runner::CliRunner::run`]).
//!
//! ## Worker Lifecycle
//!
//! ### Cold Start (0 → 2 workers)
//!
//! When a model is accessed for the first time:
//!
//! 1. Pool checks: `has_workers(registry_key)?` → No
//! 2. Calculate memory: `current_mb + (2 * per_worker_mb) <= 80% system`?
//! 3. If yes: Spawn 2 workers (parallel request processing)
//! 4. If degraded: Spawn 1 worker (memory constrained)
//! 5. If no memory: Return `PoolError::MemoryExhausted`
//!
//! ### Active Processing
//!
//! When a request arrives:
//!
//! 1. Select LRU worker (least recently used)
//! 2. Send request via tokio mpsc channel
//! 3. Worker processes request (exclusive model ownership)
//! 4. Update metrics: `pending_requests--`, `last_used = now()`
//!
//! ### Idle Eviction (maintenance thread)
//!
//! Every 60 seconds, the maintenance thread:
//!
//! 1. Scans all 5 pools for idle models
//! 2. For each idle model:
//!    - Check: **ALL workers** idle? (pending == 0, idle >= 5 minutes)
//!    - If yes: Find LRU worker, send shutdown signal
//!    - Worker loop receives signal and exits
//!    - Update memory tracking: `total_memory_mb -= per_worker_mb`
//! 3. Log memory usage across all pools
//!
//! ## Memory Management
//!
//! ### 80% System Memory Limit
//!
//! The pool enforces an 80% system memory limit to prevent OOM:
//!
//! ```text
//! System: 16384 MB
//! Limit:  13107 MB (80%)
//! Available: 13107 - current_usage
//! ```
//!
//! ### Per-Worker Memory Tracking
//!
//! Each model declares memory usage in `MODEL_INFO`:
//!
//! ```rust,ignore
//! MODEL_INFO
//!     // ... other fields
//!     est_memory_allocation_mb: 2048,  // Stella: 2GB per worker
//! };
//! ```
//!//! Pool tracks memory atomically:
//!
//! - **Spawn**: `total_memory_mb += per_worker_mb`
//! - **Evict**: `total_memory_mb -= per_worker_mb`
//!
//! ## Logging
//!
//! Pool logs important events at different levels:
//!
//! ### Info Level
//!
//! ```text
//! Pool maintenance thread initialized
//! Spawned worker 1 for dunzhang/stella_en_1.5B_v5 (2048 MB)
//! Spawned worker 2 for dunzhang/stella_en_1.5B_v5 (2048 MB)
//! Evicted worker 1 from gte-large-en-v1.5 (idle cooldown), 1 workers remain
//! Maintenance thread shutting down
//! ```
//!
//! ### Debug Level
//!
//! ```text
//! TextEmbedding pool: All workers idle for gte-large-en-v1.5, evicting LRU worker at index 0 (1024 MB)
//! Pool memory usage: 6144 MB (TextEmbedding: 4096, TextToText: 2048, ImageEmbedding: 0, Vision: 0, TextToImage: 0)
//! ```
//!
//! ### Error Level
//!
//! ```text
//! Failed to spawn worker: model loading failed
//! Failed to evict worker from TextEmbedding pool: worker index out of bounds
//! ```
//!//! Enable pool logging:
//!
//! ```bash
//! # Info only
//! RUST_LOG=kodegen_candle_agent::pool=info cargo run
//!
//! # Debug (includes memory usage)
//! RUST_LOG=kodegen_candle_agent::pool=debug cargo run
//!
//! # All logs
//! RUST_LOG=debug cargo run
//! ```
//!
//! ## Known Limitations (Phase 1)
//!
//! Current implementation status:
//!
//! | Capability | Pool Integration | Status |
//! |------------|------------------|--------|
//! | TextEmbedding | ✅ Full integration (5 models) | Production |
//! | TextToText | ⚠️ Infrastructure ready, direct call | Phase 2 |
//! | ImageEmbedding | ⚠️ Infrastructure ready, direct call | Phase 2 |
//! | Vision | ⚠️ Infrastructure ready, direct call | Phase 2 |
//! | TextToImage | ⚠️ Infrastructure ready, direct call | Phase 2 |
//!
//! **Why TextEmbedding first?**
//!
//! - Small models (~1-2GB each)
//! - Stateless operations (perfect for pooling)
//! - High request frequency in production
//!
//! **Why TextToText not pooled yet?**
//!
//! - Large models (7-14GB each)
//! - Already have internal state management
//! - Lower priority for Phase 1
//!
//! ## Future Enhancements (Phase 2+)
//!
//! - **Dynamic timeout adjustment** based on queue depth
//! - **Worker health monitoring** with automatic restart
//! - **Metrics dashboard** (request counts, latencies, hit rates)
//! - **Adaptive worker scaling** (spawn 1-4 workers based on load)
//! - **Pool integration for TextToText models**
//! - **Pool integration for Vision models**
//!
//! See [`task/MODEL_POOL.md`](../../task/MODEL_POOL.md) for roadmap details.
//!
//! ## Module Structure
//!
//! ```text
//! pool/
//!   mod.rs                    - Public API, exports, init_maintenance()
//!   core/
//!     mod.rs                  - Core exports
//!     pool.rs                 - Generic Pool<T> implementation
//!     worker.rs               - Generic worker helpers
//!     types.rs                - WorkerHandle, PoolConfig, PoolMetrics
//!     error.rs                - PoolError enum
//!   capabilities/
//!     mod.rs                  - Capability exports
//!     text_embedding.rs       - TextEmbedding pool + worker loop
//!     text_to_text.rs         - TextToText pool + worker loop
//!     image_embedding.rs      - ImageEmbedding pool + worker loop
//!     vision.rs               - Vision pool + worker loop
//!     text_to_image.rs        - TextToImage pool + worker loop
//!   maintenance.rs            - Background eviction thread
//! ```

pub mod capabilities;
pub mod core;
pub mod maintenance;
pub mod shutdown;

pub use capabilities::{
    image_embedding_pool, text_embedding_pool, text_to_image_pool, text_to_text_pool, vision_pool,
};
pub use core::{Pool, PoolConfig, PoolError, WorkerHandle, WorkerState};
pub use maintenance::start_maintenance_thread;
pub use shutdown::begin_shutdown;

use once_cell::sync::Lazy;

/// Global maintenance thread handle
///
/// Lazily initialized on first access via [`init_maintenance()`].
/// The maintenance thread runs every minute to evict idle workers and monitor memory usage.
///
/// Thread lifecycle:
/// - **Start**: On first call to `init_maintenance()`
/// - **Run**: Every 60 seconds (configurable via `PoolConfig.maintenance_interval_secs`)
/// - **Stop**: When all pools signal shutdown via `begin_shutdown()`
static MAINTENANCE_THREAD: Lazy<Option<tokio::task::JoinHandle<()>>> =
    Lazy::new(|| match start_maintenance_thread() {
        Ok(handle) => {
            log::info!("Pool maintenance thread started");
            Some(handle)
        }
        Err(e) => {
            log::error!("Failed to start maintenance thread: {}", e);
            None
        }
    });

/// Initialize pool maintenance thread
///
/// Call once at application startup. Subsequent calls are no-ops.
/// Thread starts immediately and runs until shutdown.
///
/// # Example
///
/// ```rust
/// // In CLI runner or main.rs
/// kodegen_candle_agent::pool::init_maintenance();
/// ```
///
/// The maintenance thread:
/// - Runs every 60 seconds (configurable)
/// - Evicts 1 LRU worker per idle model
/// - Logs memory usage at debug level
/// - Stops when all pools shutdown
pub fn init_maintenance() {
    // Force lazy initialization
    let _ = &*MAINTENANCE_THREAD;
    log::info!("Pool maintenance thread initialized");
}
