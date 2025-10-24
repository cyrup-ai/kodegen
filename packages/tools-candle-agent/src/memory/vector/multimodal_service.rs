//! Multimodal embedding service bridging text and vision embeddings
//!
//! This module provides a unified interface for both text and image embeddings,
//! enabling cross-modal similarity operations between text and images.

use crate::capability::registry::{ImageEmbeddingModel, TextEmbeddingModel};
use crate::capability::traits::{ImageEmbeddingCapable, TextEmbeddingCapable};
use crate::memory::core::manager::surreal::{PendingBatchEmbedding, PendingEmbedding};
use crate::memory::utils::error::Result;
use kodegen_simd::cosine_similarity;

/// Multimodal embedding service bridging text and vision embeddings
///
/// Wraps both a text embedding model and a vision model from the registry
/// to enable unified multimodal operations.
#[derive(Clone)]
pub struct MultimodalEmbeddingService {
    /// Text embedding model from registry
    text_model: TextEmbeddingModel,

    /// Vision embedding model from registry
    vision_model: ImageEmbeddingModel,
}

impl MultimodalEmbeddingService {
    /// Create new multimodal service with text and vision models from registry
    pub fn new(text_model: TextEmbeddingModel, vision_model: ImageEmbeddingModel) -> Self {
        Self {
            text_model,
            vision_model,
        }
    }

    /// Get text embedding dimension
    pub fn text_embedding_dimension(&self) -> usize {
        self.text_model.embedding_dimension()
    }

    /// Get vision embedding dimension
    pub fn vision_embedding_dimension(&self) -> usize {
        self.vision_model.embedding_dimension()
    }

    /// Embed text using the text model
    ///
    /// Returns a Future that resolves to the embedding vector.
    pub fn embed_text(&self, text: String, task: Option<String>) -> PendingEmbedding {
        let text_model = self.text_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = text_model.embed(&text, task).await.map_err(|e| {
                crate::memory::utils::error::Error::Other(format!("Text embedding failed: {}", e))
            });
            let _ = tx.send(result);
        });

        PendingEmbedding::new(rx)
    }

    /// Batch embed multiple texts
    ///
    /// Returns a Future that resolves to a vector of embedding vectors.
    pub fn batch_embed_text(
        &self,
        texts: Vec<String>,
        task: Option<String>,
    ) -> PendingBatchEmbedding {
        let text_model = self.text_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = text_model.batch_embed(&texts, task).await.map_err(|e| {
                crate::memory::utils::error::Error::Other(format!(
                    "Batch text embedding failed: {}",
                    e
                ))
            });
            let _ = tx.send(result);
        });

        PendingBatchEmbedding::new(rx)
    }
    /// Embed image from file path
    ///
    /// Returns a Future that resolves to the embedding vector.
    pub fn embed_image(&self, image_path: String) -> PendingEmbedding {
        let vision_model = self.vision_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = vision_model.embed_image(&image_path).await.map_err(|e| {
                crate::memory::utils::error::Error::Other(format!("Failed to embed image: {}", e))
            });
            let _ = tx.send(result);
        });

        PendingEmbedding::new(rx)
    }

    /// Embed image from URL
    ///
    /// Returns a Future that resolves to the embedding vector.
    pub fn embed_image_url(&self, url: String) -> PendingEmbedding {
        let vision_model = self.vision_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = vision_model.embed_image_url(&url).await.map_err(|e| {
                crate::memory::utils::error::Error::Other(format!(
                    "Failed to embed image from URL: {}",
                    e
                ))
            });
            let _ = tx.send(result);
        });

        PendingEmbedding::new(rx)
    }

    /// Embed image from base64 data (for API usage)
    ///
    /// Returns a Future that resolves to the embedding vector.
    pub fn embed_image_base64(&self, base64_data: String) -> PendingEmbedding {
        let vision_model = self.vision_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = vision_model
                .embed_image_base64(&base64_data)
                .await
                .map_err(|e| {
                    crate::memory::utils::error::Error::Other(format!(
                        "Failed to embed base64 image: {}",
                        e
                    ))
                });
            let _ = tx.send(result);
        });

        PendingEmbedding::new(rx)
    }

    /// Batch embed multiple images from file paths
    ///
    /// Returns a Future that resolves to a vector of embedding vectors.
    pub fn batch_embed_images(&self, image_paths: Vec<String>) -> PendingBatchEmbedding {
        let vision_model = self.vision_model.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let paths: Vec<&str> = image_paths.iter().map(|s| s.as_str()).collect();
            let result = vision_model.batch_embed_images(paths).await.map_err(|e| {
                crate::memory::utils::error::Error::Other(format!(
                    "Failed to batch embed images: {}",
                    e
                ))
            });
            let _ = tx.send(result);
        });

        PendingBatchEmbedding::new(rx)
    }

    /// Compute cross-modal similarity between text and image
    ///
    /// Embeds text using text model and image using vision provider,
    /// then computes cosine similarity using kodegen_simd::cosine_similarity.
    pub async fn text_image_similarity(&self, text: String, image_path: String) -> Result<f32> {
        // Embed text
        let text_emb = self.embed_text(text, None).await?;

        // Embed image
        let img_emb = self.embed_image(image_path).await?;

        // Dimension check
        if text_emb.len() != img_emb.len() {
            return Err(crate::memory::utils::error::Error::Other(format!(
                "Dimension mismatch: text={}, image={}",
                text_emb.len(),
                img_emb.len()
            )));
        }

        // Compute cosine similarity using kodegen_simd (SIMD-optimized)
        Ok(cosine_similarity(&text_emb, &img_emb))
    }

    /// Compute image-text similarity (alias for symmetry)
    pub async fn image_text_similarity(&self, image_path: String, text: String) -> Result<f32> {
        self.text_image_similarity(text, image_path).await
    }

    /// Compute similarity between text and image URL
    pub async fn text_image_url_similarity(&self, text: String, image_url: String) -> Result<f32> {
        let text_emb = self.embed_text(text, None).await?;
        let img_emb = self.embed_image_url(image_url).await?;

        if text_emb.len() != img_emb.len() {
            return Err(crate::memory::utils::error::Error::Other(format!(
                "Dimension mismatch: text={}, image={}",
                text_emb.len(),
                img_emb.len()
            )));
        }

        Ok(cosine_similarity(&text_emb, &img_emb))
    }

    /// Batch compute cross-modal similarities
    ///
    /// For each text, compute similarity with corresponding image.
    /// Requires texts.len() == image_paths.len()
    pub async fn batch_text_image_similarity(
        &self,
        texts: Vec<String>,
        image_paths: Vec<String>,
    ) -> Result<Vec<f32>> {
        if texts.len() != image_paths.len() {
            return Err(crate::memory::utils::error::Error::Other(format!(
                "Batch size mismatch: texts={}, images={}",
                texts.len(),
                image_paths.len()
            )));
        }

        // Batch embed texts
        let text_embs = self.batch_embed_text(texts, None).await?;

        // Batch embed images
        let img_embs = self.batch_embed_images(image_paths).await?;

        // Compute similarities
        let mut similarities = Vec::with_capacity(text_embs.len());
        for (text_emb, img_emb) in text_embs.iter().zip(img_embs.iter()) {
            if text_emb.len() != img_emb.len() {
                return Err(crate::memory::utils::error::Error::Other(format!(
                    "Embedding dimension mismatch in batch: text={}, image={}",
                    text_emb.len(),
                    img_emb.len()
                )));
            }
            similarities.push(cosine_similarity(text_emb, img_emb));
        }

        Ok(similarities)
    }
}
