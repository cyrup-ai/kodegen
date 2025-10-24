//! TextToImageCapable trait implementation for TextToImageModel

use super::pool::capabilities::text_to_image_pool;
use super::pool::core::ensure_workers_spawned_adaptive;
use crate::capability::traits::TextToImageCapable;
use crate::domain::image_generation::{ImageGenerationChunk, ImageGenerationConfig};
use crate::domain::model::traits::CandleModel;
use candle_core::Device;
use std::pin::Pin;
use tokio_stream::Stream;

use super::enums::TextToImageModel;

impl TextToImageCapable for TextToImageModel {
    fn generate_image(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>> {
        match self {
            Self::FluxSchnell(m) => {
                let registry_key = m.info().registry_key;
                let per_worker_mb = m.info().est_memory_allocation_mb;
                let pool = text_to_image_pool();
                let m_clone = (**m).clone();
                let prompt = prompt.to_string();
                let config = config.clone();
                let device = device.clone();

                Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                    // Cold start: spawn workers if needed
                    if let Err(e) = ensure_workers_spawned_adaptive(
                        pool,
                        registry_key,
                        per_worker_mb,
                        pool.config().max_workers_per_model,
                        |_, allocation_guard| {
                            let m_clone = m_clone.clone();
                            pool.spawn_text_to_image_worker(
                                registry_key,
                                move || async move { Ok(m_clone) },
                                per_worker_mb,
                                allocation_guard,
                            )
                        },
                    )
                    .await
                    {
                        let _ = tx.send(ImageGenerationChunk::Error(e.to_string()));
                        return;
                    }

                    // Route through pool
                    let mut stream = pool.generate_image(registry_key, &prompt, &config, &device);
                    use tokio_stream::StreamExt;
                    while let Some(chunk) = stream.next().await {
                        if tx.send(chunk).is_err() {
                            break;
                        }
                    }
                }))
            }
            Self::StableDiffusion35Turbo(m) => {
                let registry_key = m.info().registry_key;
                let per_worker_mb = m.info().est_memory_allocation_mb;
                let pool = text_to_image_pool();
                let m_clone = (**m).clone();
                let prompt = prompt.to_string();
                let config = config.clone();
                let device = device.clone();

                Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                    // Cold start: spawn workers if needed
                    if let Err(e) = ensure_workers_spawned_adaptive(
                        pool,
                        registry_key,
                        per_worker_mb,
                        pool.config().max_workers_per_model,
                        |_, allocation_guard| {
                            let m_clone = m_clone.clone();
                            pool.spawn_text_to_image_worker(
                                registry_key,
                                move || async move { Ok(m_clone) },
                                per_worker_mb,
                                allocation_guard,
                            )
                        },
                    )
                    .await
                    {
                        let _ = tx.send(ImageGenerationChunk::Error(e.to_string()));
                        return;
                    }

                    // Route through pool
                    let mut stream = pool.generate_image(registry_key, &prompt, &config, &device);
                    use tokio_stream::StreamExt;
                    while let Some(chunk) = stream.next().await {
                        if tx.send(chunk).is_err() {
                            break;
                        }
                    }
                }))
            }
        }
    }

    fn registry_key(&self) -> &str {
        match self {
            Self::FluxSchnell(m) => m.registry_key(),
            Self::StableDiffusion35Turbo(m) => m.registry_key(),
        }
    }
}
