//! ImageEmbeddingCapable trait implementation for ImageEmbeddingModel

use super::pool::capabilities::image_embedding_pool;
use super::pool::core::ensure_workers_spawned_adaptive;
use crate::capability::traits::ImageEmbeddingCapable;
use crate::domain::model::traits::CandleModel;
use std::sync::Arc;

use super::enums::ImageEmbeddingModel;

impl ImageEmbeddingCapable for ImageEmbeddingModel {
    fn embed_image(
        &self,
        image_path: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        let image_path = image_path.to_string();
        match self {
            Self::ClipVision(m) => Box::pin(spawn_embed_image_clip(m.clone(), image_path)),
        }
    }

    fn embed_image_url(
        &self,
        url: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        let url = url.to_string();
        match self {
            Self::ClipVision(m) => Box::pin(spawn_embed_image_url_clip(m.clone(), url)),
        }
    }

    fn embed_image_base64(
        &self,
        base64_data: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<f32>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        let base64_data = base64_data.to_string();
        match self {
            Self::ClipVision(m) => Box::pin(spawn_embed_image_base64_clip(m.clone(), base64_data)),
        }
    }

    fn batch_embed_images(
        &self,
        image_paths: Vec<&str>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Vec<Vec<f32>>,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send
                + '_,
        >,
    > {
        let paths: Vec<String> = image_paths.iter().map(|s| s.to_string()).collect();
        match self {
            Self::ClipVision(m) => Box::pin(spawn_batch_embed_images_clip(m.clone(), paths)),
        }
    }

    fn embedding_dimension(&self) -> usize {
        match self {
            Self::ClipVision(m) => m.embedding_dimension(),
        }
    }
}

// Helper functions for ClipVision model
async fn spawn_embed_image_clip(
    model: Arc<crate::capability::image_embedding::clip_vision_embedding::ClipVisionEmbeddingModel>,
    image_path: String,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = image_embedding_pool();

    ensure_workers_spawned_adaptive(
        pool,
        registry_key,
        per_worker_mb,
        pool.config().max_workers_per_model,
        |_, allocation_guard| {
            let model_clone = (*model).clone();
            pool.spawn_image_embedding_worker(
                registry_key,
                move || async move { Ok(model_clone) },
                per_worker_mb,
                allocation_guard,
            )
        },
    )
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    pool.embed_image(registry_key, &image_path)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

async fn spawn_embed_image_url_clip(
    model: Arc<crate::capability::image_embedding::clip_vision_embedding::ClipVisionEmbeddingModel>,
    url: String,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = image_embedding_pool();

    ensure_workers_spawned_adaptive(
        pool,
        registry_key,
        per_worker_mb,
        pool.config().max_workers_per_model,
        |_, allocation_guard| {
            let model_clone = (*model).clone();
            pool.spawn_image_embedding_worker(
                registry_key,
                move || async move { Ok(model_clone) },
                per_worker_mb,
                allocation_guard,
            )
        },
    )
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    pool.embed_image_url(registry_key, &url)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

async fn spawn_embed_image_base64_clip(
    model: Arc<crate::capability::image_embedding::clip_vision_embedding::ClipVisionEmbeddingModel>,
    base64_data: String,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = image_embedding_pool();

    ensure_workers_spawned_adaptive(
        pool,
        registry_key,
        per_worker_mb,
        pool.config().max_workers_per_model,
        |_, allocation_guard| {
            let model_clone = (*model).clone();
            pool.spawn_image_embedding_worker(
                registry_key,
                move || async move { Ok(model_clone) },
                per_worker_mb,
                allocation_guard,
            )
        },
    )
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    pool.embed_image_base64(registry_key, &base64_data)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

async fn spawn_batch_embed_images_clip(
    model: Arc<crate::capability::image_embedding::clip_vision_embedding::ClipVisionEmbeddingModel>,
    paths: Vec<String>,
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = image_embedding_pool();

    ensure_workers_spawned_adaptive(
        pool,
        registry_key,
        per_worker_mb,
        pool.config().max_workers_per_model,
        |_, allocation_guard| {
            let model_clone = (*model).clone();
            pool.spawn_image_embedding_worker(
                registry_key,
                move || async move { Ok(model_clone) },
                per_worker_mb,
                allocation_guard,
            )
        },
    )
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    pool.batch_embed_images(registry_key, &paths)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}
