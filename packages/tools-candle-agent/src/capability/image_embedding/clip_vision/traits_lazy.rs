//! ImageEmbeddingCapable trait implementation for ClipVisionModel
//!
//! This module implements the ImageEmbeddingCapable trait for ClipVisionModel,
//! which uses the lazy loading pattern (model loaded on-demand).

use super::models::ClipVisionModel;

impl crate::capability::traits::ImageEmbeddingCapable for ClipVisionModel {
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
        Box::pin(async move {
            // Encode image to tensor (1, embed_dim)
            let tensor = self.encode_image(&image_path).await.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            // Convert to Vec<f32>
            let embedding = tensor
                .to_vec1::<f32>()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

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
        let url = url.to_string();
        Box::pin(async move {
            // Encode image from URL
            let tensor = self.encode_url(&url).await.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            // Convert to Vec<f32>
            let embedding = tensor
                .to_vec1::<f32>()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

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
        let base64_data = base64_data.to_string();
        Box::pin(async move {
            // Encode image from base64
            let tensor = self.encode_base64(&base64_data).await.map_err(|e| {
                Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error + Send + Sync>
            })?;

            // Convert to Vec<f32>
            let embedding = tensor
                .to_vec1::<f32>()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

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
        let paths: Vec<String> = image_paths.iter().map(|s| s.to_string()).collect();
        Box::pin(async move {
            let mut embeddings = Vec::with_capacity(paths.len());
            for path in &paths {
                let embedding = self.embed_image(path).await?;
                embeddings.push(embedding);
            }
            Ok(embeddings)
        })
    }

    fn embedding_dimension(&self) -> usize {
        use crate::domain::model::traits::CandleModel;
        self.info().embedding_dimension.unwrap_or(512) as usize
    }

    fn supported_dimensions(&self) -> Vec<usize> {
        // CLIP Vision supports both Base (512D) and Large (768D) variants
        vec![512, 768]
    }
}
