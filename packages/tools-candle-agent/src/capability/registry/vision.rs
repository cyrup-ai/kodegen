//! VisionCapable trait implementation for VisionModel

use super::pool::capabilities::vision_pool;
use super::pool::core::{PoolError, ensure_workers_spawned_adaptive};
use crate::capability::traits::VisionCapable;
use crate::capability::vision::llava::LoadedLLaVAModel;
use crate::domain::context::chunks::CandleStringChunk;
use crate::domain::model::traits::CandleModel;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

use super::enums::VisionModel;

impl VisionCapable for VisionModel {
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        match self {
            Self::LLaVA(m) => {
                spawn_describe_image_llava(m.clone(), image_path.to_string(), query.to_string())
            }
        }
    }

    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        match self {
            Self::LLaVA(m) => {
                spawn_describe_url_llava(m.clone(), url.to_string(), query.to_string())
            }
        }
    }
}

// Helper function for LLaVA describe_image
fn spawn_describe_image_llava(
    model: Arc<crate::capability::vision::llava::LLaVAModel>,
    image_path: String,
    query: String,
) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = vision_pool();

    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Err(e) = ensure_workers_spawned_adaptive(
            pool,
            registry_key,
            per_worker_mb,
            pool.config().max_workers_per_model,
            |_, allocation_guard| {
                let m_clone = model.clone();
                pool.spawn_vision_worker(
                    registry_key,
                    move || async move {
                        LoadedLLaVAModel::load(&m_clone)
                            .map_err(|e| PoolError::SpawnFailed(e.to_string()))
                    },
                    per_worker_mb,
                    allocation_guard,
                )
            },
        )
        .await
        {
            let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            return;
        }

        let mut stream = pool.describe_image(registry_key, &image_path, &query);
        use tokio_stream::StreamExt;
        while let Some(chunk) = stream.next().await {
            if tx.send(chunk).is_err() {
                break;
            }
        }
    }))
}

// Helper function for LLaVA describe_url
fn spawn_describe_url_llava(
    model: Arc<crate::capability::vision::llava::LLaVAModel>,
    url: String,
    query: String,
) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
    let registry_key = model.info().registry_key;
    let per_worker_mb = model.info().est_memory_allocation_mb;
    let pool = vision_pool();

    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Err(e) = ensure_workers_spawned_adaptive(
            pool,
            registry_key,
            per_worker_mb,
            pool.config().max_workers_per_model,
            |_, allocation_guard| {
                let m_clone = model.clone();
                pool.spawn_vision_worker(
                    registry_key,
                    move || async move {
                        LoadedLLaVAModel::load(&m_clone)
                            .map_err(|e| PoolError::SpawnFailed(e.to_string()))
                    },
                    per_worker_mb,
                    allocation_guard,
                )
            },
        )
        .await
        {
            let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            return;
        }

        let mut stream = pool.describe_url(registry_key, &url, &query);
        use tokio_stream::StreamExt;
        while let Some(chunk) = stream.next().await {
            if tx.send(chunk).is_err() {
                break;
            }
        }
    }))
}
