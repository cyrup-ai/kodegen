//! # Capability Model Registry
//!
//! **THE ONLY MODEL REGISTRY IN THE ENTIRE CODEBASE**
//!
//! This is the single source of truth for all model registrations.
//! Every model in the system is registered here, keyed by its `registry_key`.
//!
//! ## Architecture
//!
//! Uses enum-based storage with unified registries to eliminate type erasure:
//! - Each capability has its own enum (TextToTextModel, TextEmbeddingModel, etc.)
//! - Each enum wraps Arc<ConcreteModel> for cheap cloning
//! - Enums implement CandleModel + their capability trait via match delegation
//! - Returns `impl Trait` instead of trait objects for zero-cost abstraction
//! - All registries use `LazyLock<RwLock<HashMap<String, T>>>` for unified storage
//!
//! ## Unified Registry Architecture
//!
//! After registry unification, all models (static and runtime-registered) live in
//! a single storage system per capability:
//! - `TEXT_TO_TEXT_UNIFIED` - All text-to-text models
//! - `TEXT_EMBEDDING_UNIFIED` - All embedding models
//! - `IMAGE_EMBEDDING_UNIFIED` - All image embedding models
//! - `TEXT_TO_IMAGE_UNIFIED` - All text-to-image models
//! - `VISION_UNIFIED` - All vision/multimodal models
//!
//! Benefits:
//! - `get<T>()` works for ALL models (static + runtime)
//! - No more API confusion about which getter to use
//! - Simpler mental model: register once, available everywhere
//!
//! ## Pool Integration
//!
//! All capability types route through capability-specific pools for performance:
//!
//! **Worker Management:**
//! - First request: Adaptive worker spawning (typically 0→2 cold start)
//! - Subsequent requests: Route to least-busy available worker
//! - Workers keep models loaded in memory (no reload between requests)
//!
//! **Capability-Specific Pools:**
//! - `TextToTextModel` → `text_to_text_pool()`
//! - `TextEmbeddingModel` → `text_embedding_pool()`
//! - `ImageEmbeddingModel` → `image_embedding_pool()`
//! - `TextToImageModel` → `text_to_image_pool()`
//! - `VisionModel` → `vision_pool()`
//!
//! **Benefits:**
//! - Amortizes model loading cost across many requests
//! - Enables concurrent request handling with multiple workers
//! - Automatic resource management (memory allocation tracking)
//! - Fault isolation (worker failures don't crash main process)
//!
//! ## User Transparency
//!
//! Users call:
//! ```rust
//! let model = registry::get::<TextEmbeddingModel>("dunzhang/stella_en_1.5B_v5")?;
//! let embedding = model.embed("hello world", None).await?;
//! // ↑ Pool manages workers transparently within embed()
//! ```
//!
//! Pool integration is invisible - user code unchanged.
//!
//! ## To Add a New Model:
//!
//! 1. Implement `CandleModel` trait with static `MODEL_INFO`
//! 2. Implement capability trait(s): `TextToTextCapable`, `TextEmbeddingCapable`, etc.
//! 3. Add variant to the appropriate enum in `enums.rs`
//! 4. Add static initialization in `storage.rs` OR use runtime registration
//!
//! ## Thread Safety
//!
//! Uses `parking_lot::RwLock` for unified registry storage:
//! - Multiple readers can access concurrently (no contention)
//! - Single writer blocks readers momentarily during registration
//! - `LazyLock` ensures one-time initialization of static models
//! - Fully sync API - no `.await` needed for reads
//!
//! ## Runtime Registration
//!
//! Some models require async initialization (HF downloads, local files, etc.) and
//! cannot be statically initialized. Use the registration functions:
//! - `register_text_to_text()` for models like Qwen3Coder
//! - `register_image_embedding()` for ClipVision models
//! - `register_text_to_image()` for Flux/StableDiffusion models
//!
//! After registration, models are immediately available via ALL APIs:
//! ```rust
//! // Register at runtime
//! registry::register_text_to_text("my-key", model).await;
//!
//! // Available via primary API
//! let model = registry::get<TextToTextModel>("my-key").unwrap();
//!
//! // Also available via capability-specific API
//! let model = registry::get_text_to_text("my-key").unwrap();
//!
//! // Backward compat: get_*_runtime() functions still work but are now redundant
//! let model = registry::get_text_to_text_runtime("my-key").await.unwrap();
//! ```

mod api;
mod enums;
mod image_embedding;
mod runtime;
pub(crate) mod storage;
mod text_embedding;
mod text_to_image;
mod text_to_text;
mod vision;

// Pool is an integral part of registry - registry IS ALWAYS POOLED
pub mod pool;

// Re-export enums
pub use enums::{
    AnyModel, ImageEmbeddingModel, TextEmbeddingModel, TextToImageModel, TextToTextModel,
    VisionModel,
};

// Re-export API functions
pub use api::{
    FromRegistry, all_registry_keys, count_models_by_provider, get, get_by_provider_and_name,
    get_image_embedding, get_model, get_text_embedding, get_text_to_image, get_text_to_text,
    get_vision, has_model, model_count,
};

// Re-export runtime registration functions and types
pub use runtime::{
    RegistrationError, get_image_embedding_runtime, get_text_to_image_runtime,
    get_text_to_text_runtime, register_image_embedding, register_text_to_image,
    register_text_to_text, unregister_image_embedding, unregister_text_to_image,
    unregister_text_to_text,
};

// Test module
