//! Embedding builder implementations - Zero Box<dyn> trait-based architecture
//!
//! All embedding construction logic and builder patterns with zero allocation.
//! Integrates with the registry system to access text embedding models.

use crate::capability::registry::{self, TextEmbeddingModel};
use crate::capability::traits::TextEmbeddingCapable;
use crate::domain::embedding_result::Embedding;
use cylo::{AsyncTask, async_task::AsyncTaskBuilder};

/// Embedding builder trait - elegant zero-allocation builder pattern
pub trait EmbeddingBuilder: Sized {
    /// Set the model to use for embedding - EXACT syntax: .model("registry_key")
    fn model(self, registry_key: &str) -> impl EmbeddingBuilder;

    /// Set task instruction for embedding - EXACT syntax: .with_task("query")
    fn with_task(self, task: impl Into<String>) -> impl EmbeddingBuilder;

    /// Set dimensions (for validation only) - EXACT syntax: .with_dims(512)
    fn with_dims(self, dims: usize) -> impl EmbeddingBuilder;

    /// Generate embedding - EXACT syntax: .embed()
    fn embed(self) -> AsyncTask<Result<Embedding, Box<dyn std::error::Error + Send + Sync>>>;
}

/// Hidden implementation struct - zero-allocation builder state
struct EmbeddingBuilderImpl {
    document: String,
    model_key: Option<String>,
    task: Option<String>,
    expected_dims: Option<usize>,
}

impl Embedding {
    /// Semantic entry point - EXACT syntax: Embedding::from_document("text")
    pub fn from_document(document: impl Into<String>) -> impl EmbeddingBuilder {
        EmbeddingBuilderImpl {
            document: document.into(),
            model_key: None,
            task: None,
            expected_dims: None,
        }
    }
}

impl EmbeddingBuilder for EmbeddingBuilderImpl {
    /// Set the model to use for embedding
    fn model(mut self, registry_key: &str) -> impl EmbeddingBuilder {
        self.model_key = Some(registry_key.to_string());
        self
    }

    /// Set task instruction for embedding
    fn with_task(mut self, task: impl Into<String>) -> impl EmbeddingBuilder {
        self.task = Some(task.into());
        self
    }

    /// Set expected dimensions (for validation only)
    fn with_dims(mut self, dims: usize) -> impl EmbeddingBuilder {
        self.expected_dims = Some(dims);
        self
    }

    /// Generate embedding - EXACT syntax: .embed()
    fn embed(self) -> AsyncTask<Result<Embedding, Box<dyn std::error::Error + Send + Sync>>> {
        AsyncTaskBuilder::new(async move {
            // Get model from registry (defaults to Stella if not specified)
            let model_key = self
                .model_key
                .unwrap_or_else(|| "dunzhang/stella_en_400M_v5".to_string());

            let model: TextEmbeddingModel = registry::get(&model_key)
                .ok_or_else(|| format!("Model not found in registry: {}", model_key))?;

            // Validate dimensions if specified
            if let Some(expected) = self.expected_dims {
                let actual = model.embedding_dimension();
                if expected != actual {
                    return Err(format!(
                        "Dimension mismatch: expected {}, model provides {}",
                        expected, actual
                    )
                    .into());
                }
            }

            // Generate embedding via capability trait
            let vec = model.embed(&self.document, self.task).await?;

            Ok(Embedding::new(self.document, vec))
        })
        .spawn()
    }
}
