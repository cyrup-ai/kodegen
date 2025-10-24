//! Base Stella embedding model implementation

use super::config::STELLA_400M_MODEL_INFO;
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;

/// Stella embedding provider - registry holder only
///
/// This struct serves as a registry holder and provides model metadata.
/// It is NOT meant for direct inference - use LoadedStellaModel via the worker pool.
///
/// # Usage
/// ```rust,ignore
/// // CORRECT: Via worker pool (automatic)
/// let model = TextEmbeddingModel::Stella(Arc::new(StellaEmbeddingModel::new()));
/// model.embed("text", None).await?;  // Routes through pool → LoadedStellaModel
///
/// // WRONG: Direct usage (now prevented)
/// let model = StellaEmbeddingModel::new();
/// model.embed("text", None).await?;  // ← Compile error!
/// ```
#[derive(Debug, Clone)]
pub struct StellaEmbeddingModel {}

impl Default for StellaEmbeddingModel {
    fn default() -> Self {
        Self::new()
    }
}

impl StellaEmbeddingModel {
    /// Create new Stella embedding provider
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    /// Get the embedding output dimension from model info
    pub fn embedding_dimension(&self) -> usize {
        self.info().embedding_dimension.unwrap_or(1024) as usize
    }
}

impl CandleModel for StellaEmbeddingModel {
    fn info(&self) -> &'static CandleModelInfo {
        // Default to 400M variant
        // Note: This is only used for registry lookup. Actual variant is detected
        // from registry_key during model loading.
        &STELLA_400M_MODEL_INFO
    }
}

// TextEmbeddingCapable implementation REMOVED
// Use LoadedStellaModel via worker pool instead
