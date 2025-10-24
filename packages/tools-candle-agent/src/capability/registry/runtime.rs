//! Runtime registration for models that require async initialization

use super::api::has_model;
use super::enums::{ImageEmbeddingModel, TextToImageModel, TextToTextModel};
use super::storage::*;
use std::fmt;

/// Error type for model registration operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    /// The registry key already exists in any capability registry
    KeyAlreadyExists(String),
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyAlreadyExists(key) => {
                write!(f, "Registry key '{}' already exists", key)
            }
        }
    }
}

impl std::error::Error for RegistrationError {}

/// Register an image embedding model at runtime
///
/// Use this for models that require explicit configuration (e.g., ClipVision)
/// and cannot be statically initialized.
///
/// After registry unification, this registers directly into the unified storage,
/// making the model immediately accessible via all APIs (get<T>(), get_image_embedding(), etc.).
///
/// # Errors
///
/// Returns `RegistrationError::KeyAlreadyExists` if the registry key is already registered
/// in ANY capability registry (prevents both same-capability and cross-capability duplicates).
///
/// # Example
///
/// **Note**: This function is for internal use only. External code cannot
/// construct model instances directly. Use `registry::get()` to access models.
///
/// ```rust,no_run
/// // Internal use only - external code cannot import concrete types
/// use kodegen_candle_agent::capability::registry;
///
/// // Model registration happens internally during initialization
/// // External code should use: registry::get::<ImageEmbeddingModel>("model-key")
/// ```
pub async fn register_image_embedding(
    key: impl Into<String>,
    model: ImageEmbeddingModel,
) -> Result<(), RegistrationError> {
    let key = key.into();

    // Check if key exists in ANY registry (prevents cross-capability duplicates)
    if has_model(&key) {
        return Err(RegistrationError::KeyAlreadyExists(key));
    }

    let mut registry = IMAGE_EMBEDDING_UNIFIED.write();
    registry.insert(key, model);
    Ok(())
}

/// Register a text-to-image model at runtime
///
/// Use this for models that require weight downloads (e.g., FluxSchnell)
/// and cannot be statically initialized.
///
/// After registry unification, this registers directly into the unified storage,
/// making the model immediately accessible via all APIs (get<T>(), get_text_to_image(), etc.).
///
/// # Errors
///
/// Returns `RegistrationError::KeyAlreadyExists` if the registry key is already registered
/// in ANY capability registry (prevents both same-capability and cross-capability duplicates).
///
/// # Example
///
/// **Note**: This function is for internal use only. External code cannot
/// construct model instances directly. Use `registry::get()` to access models.
///
/// ```rust,no_run
/// // Internal use only - external code cannot import concrete types
/// use kodegen_candle_agent::capability::registry;
///
/// // Model registration happens internally during initialization
/// // External code should use: registry::get::<TextToImageModel>("model-key")
/// ```
pub async fn register_text_to_image(
    key: impl Into<String>,
    model: TextToImageModel,
) -> Result<(), RegistrationError> {
    let key = key.into();

    // Check if key exists in ANY registry (prevents cross-capability duplicates)
    if has_model(&key) {
        return Err(RegistrationError::KeyAlreadyExists(key));
    }

    let mut registry = TEXT_TO_IMAGE_UNIFIED.write();
    registry.insert(key, model);
    Ok(())
}

/// Register a text-to-text model at runtime
///
/// Use this for models that require async initialization (e.g., Qwen3Coder with HF downloads)
/// and cannot be statically initialized with Default trait.
///
/// After registry unification, this registers directly into the unified storage,
/// making the model immediately accessible via all APIs (get<T>(), get_text_to_text(), etc.).
///
/// # Errors
///
/// Returns `RegistrationError::KeyAlreadyExists` if the registry key is already registered
/// in ANY capability registry (prevents both same-capability and cross-capability duplicates).
///
/// # Example
///
/// **Note**: This function is for internal use only. External code cannot
/// construct model instances directly. Use `registry::get()` to access models.
///
/// ```rust,no_run
/// // Internal use only - external code cannot import concrete types  
/// use kodegen_candle_agent::capability::registry;
///
/// // Model registration happens internally during initialization
/// // External code should use: registry::get::<TextToTextModel>("model-key")
/// ```
pub async fn register_text_to_text(
    key: impl Into<String>,
    model: TextToTextModel,
) -> Result<(), RegistrationError> {
    let key = key.into();

    // Check if key exists in ANY registry (prevents cross-capability duplicates)
    if has_model(&key) {
        return Err(RegistrationError::KeyAlreadyExists(key));
    }

    let mut registry = TEXT_TO_TEXT_UNIFIED.write();
    registry.insert(key, model);
    Ok(())
}

/// Unregister an image embedding model from runtime registry
///
/// Removes the model from the unified registry. After unregistration, the model
/// will no longer be accessible via `get<ImageEmbeddingModel>()` or related APIs.
///
/// The model memory is freed when all remaining Arc references are dropped.
///
/// Returns `Some(model)` if a model was registered under this key, `None` otherwise.
///
/// # Example
/// ```rust
/// use kodegen_candle_agent::capability::registry;
///
/// // Register a model
/// registry::register_image_embedding("temp-model", model).await;
///
/// // Later, unregister it
/// if let Some(old_model) = registry::unregister_image_embedding("temp-model").await {
///     println!("Removed model, freeing ~{} MB", old_model.info().est_memory_allocation_mb);
/// }
/// ```
pub async fn unregister_image_embedding(key: &str) -> Option<ImageEmbeddingModel> {
    let mut registry = IMAGE_EMBEDDING_UNIFIED.write();
    registry.remove(key)
}

/// Unregister a text-to-image model from runtime registry
///
/// Removes the model from the unified registry. After unregistration, the model
/// will no longer be accessible via `get<TextToImageModel>()` or related APIs.
///
/// The model memory is freed when all remaining Arc references are dropped.
///
/// Returns `Some(model)` if a model was registered under this key, `None` otherwise.
///
/// See `unregister_image_embedding` for usage example.
pub async fn unregister_text_to_image(key: &str) -> Option<TextToImageModel> {
    let mut registry = TEXT_TO_IMAGE_UNIFIED.write();
    registry.remove(key)
}

/// Unregister a text-to-text model from runtime registry
///
/// Removes the model from the unified registry. After unregistration, the model
/// will no longer be accessible via `get<TextToTextModel>()` or related APIs.
///
/// The model memory is freed when all remaining Arc references are dropped.
///
/// Returns `Some(model)` if a model was registered under this key, `None` otherwise.
///
/// See `unregister_image_embedding` for usage example.
pub async fn unregister_text_to_text(key: &str) -> Option<TextToTextModel> {
    let mut registry = TEXT_TO_TEXT_UNIFIED.write();
    registry.remove(key)
}

/// Get an image embedding model from unified registry
///
/// After registry unification, this is now equivalent to using `registry::get<ImageEmbeddingModel>()`.
/// Both static and runtime-registered models are accessible.
///
/// This function is kept for backward compatibility but users should prefer `registry::get<T>()`.
pub async fn get_image_embedding_runtime(key: &str) -> Option<ImageEmbeddingModel> {
    let registry = IMAGE_EMBEDDING_UNIFIED.read();
    registry.get(key).cloned()
}

/// Get a text-to-image model from unified registry
///
/// After registry unification, this is now equivalent to using `registry::get<TextToImageModel>()`.
/// Both static and runtime-registered models are accessible.
///
/// This function is kept for backward compatibility but users should prefer `registry::get<T>()`.
pub async fn get_text_to_image_runtime(key: &str) -> Option<TextToImageModel> {
    let registry = TEXT_TO_IMAGE_UNIFIED.read();
    registry.get(key).cloned()
}

/// Get a text-to-text model from unified registry
///
/// After registry unification, this is now equivalent to using `registry::get<TextToTextModel>()`.
/// Both static and runtime-registered models are accessible.
///
/// This function is kept for backward compatibility but users should prefer `registry::get<T>()`.
pub async fn get_text_to_text_runtime(key: &str) -> Option<TextToTextModel> {
    let registry = TEXT_TO_TEXT_UNIFIED.read();
    registry.get(key).cloned()
}
