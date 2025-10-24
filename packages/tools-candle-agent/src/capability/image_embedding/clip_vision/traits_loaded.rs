//! ImageEmbeddingCapable trait implementation for LoadedClipVisionModel
//!
//! This module implements the ImageEmbeddingCapable trait for LoadedClipVisionModel,
//! which uses the pre-loaded pattern (model loaded once and reused with spawn_blocking).

use super::loaded_encoding::encode_image_sync;
use super::models::LoadedClipVisionModel;
use super::preprocessing::PreprocessingConfig;
use crate::domain::image::Image;

impl crate::capability::traits::ImageEmbeddingCapable for LoadedClipVisionModel {
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
        use crate::domain::model::traits::CandleModel;

        let image_path = image_path.to_string();
        // Clone for move into spawn_blocking
        let model = self.model.clone();
        let device = self.device.clone();
        let image_size = self.config.image_size;
        let image_mean = self
            .info()
            .image_mean
            .ok_or("image_mean missing from ModelInfo")
            .map_err(|e| e.to_string());
        let image_std = self
            .info()
            .image_std
            .ok_or("image_std missing from ModelInfo")
            .map_err(|e| e.to_string());

        Box::pin(async move {
            // Handle config extraction errors before spawn_blocking
            let image_mean = image_mean.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let image_std = image_std.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            let config = PreprocessingConfig {
                image_size,
                image_mean,
                image_std,
            };

            // Wrap ALL CPU-intensive operations in spawn_blocking
            let embedding = tokio::task::spawn_blocking(move || {
                encode_image_sync(Image::from_path(&image_path), &config, &device, &model)
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "Spawn blocking failed: {}",
                    e
                ))) as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|e: String| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            Ok(embedding)
        })
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
        use crate::domain::model::traits::CandleModel;

        let url = url.to_string();
        // Clone for move into spawn_blocking
        let model = self.model.clone();
        let device = self.device.clone();
        let image_size = self.config.image_size;
        let image_mean = self
            .info()
            .image_mean
            .ok_or("image_mean missing from ModelInfo")
            .map_err(|e| e.to_string());
        let image_std = self
            .info()
            .image_std
            .ok_or("image_std missing from ModelInfo")
            .map_err(|e| e.to_string());

        Box::pin(async move {
            // Handle config extraction errors before spawn_blocking
            let image_mean = image_mean.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let image_std = image_std.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            let config = PreprocessingConfig {
                image_size,
                image_mean,
                image_std,
            };

            // Wrap ALL CPU-intensive operations in spawn_blocking
            let embedding = tokio::task::spawn_blocking(move || {
                encode_image_sync(Image::from_url(&url), &config, &device, &model)
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "Spawn blocking failed: {}",
                    e
                ))) as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|e: String| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            Ok(embedding)
        })
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
        use crate::domain::model::traits::CandleModel;

        let base64_data = base64_data.to_string();
        // Clone for move into spawn_blocking
        let model = self.model.clone();
        let device = self.device.clone();
        let image_size = self.config.image_size;
        let image_mean = self
            .info()
            .image_mean
            .ok_or("image_mean missing from ModelInfo")
            .map_err(|e| e.to_string());
        let image_std = self
            .info()
            .image_std
            .ok_or("image_std missing from ModelInfo")
            .map_err(|e| e.to_string());

        Box::pin(async move {
            // Handle config extraction errors before spawn_blocking
            let image_mean = image_mean.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let image_std = image_std.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            let config = PreprocessingConfig {
                image_size,
                image_mean,
                image_std,
            };

            // Wrap ALL CPU-intensive operations in spawn_blocking
            let embedding = tokio::task::spawn_blocking(move || {
                encode_image_sync(Image::from_base64(&base64_data), &config, &device, &model)
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "Spawn blocking failed: {}",
                    e
                ))) as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|e: String| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            Ok(embedding)
        })
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
        use crate::domain::model::traits::CandleModel;

        let paths: Vec<String> = image_paths.iter().map(|s| s.to_string()).collect();
        // Clone for move into spawn_blocking
        let model = self.model.clone();
        let device = self.device.clone();
        let image_size = self.config.image_size;
        let image_mean = self
            .info()
            .image_mean
            .ok_or("image_mean missing from ModelInfo")
            .map_err(|e| e.to_string());
        let image_std = self
            .info()
            .image_std
            .ok_or("image_std missing from ModelInfo")
            .map_err(|e| e.to_string());

        Box::pin(async move {
            // Handle config extraction errors before spawn_blocking
            let image_mean = image_mean.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;
            let image_std = image_std.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            let config = PreprocessingConfig {
                image_size,
                image_mean,
                image_std,
            };

            // Wrap ALL CPU-intensive operations in spawn_blocking
            let embeddings = tokio::task::spawn_blocking(move || {
                super::loaded_encoding::encode_batch_sync(&paths, &config, &device, &model)
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(format!(
                    "Spawn blocking failed: {}",
                    e
                ))) as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|e: String| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            Ok(embeddings)
        })
    }

    fn embedding_dimension(&self) -> usize {
        use crate::domain::model::traits::CandleModel;
        self.info().embedding_dimension.unwrap_or(512) as usize
    }

    fn supported_dimensions(&self) -> Vec<usize> {
        vec![512, 768]
    }
}
