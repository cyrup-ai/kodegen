//! Image constructor methods - entry points for creating ImageBuilder instances

use super::api::ImageBuilder;
use super::builder_impl::ImageBuilderImpl;
use crate::domain::context::CandleDocumentChunk as ImageChunk;
use crate::domain::image::{ContentFormat, Image};

impl Image {
    /// Semantic entry point - EXACT syntax: Image::from_base64(data)
    pub fn from_base64(data: impl Into<String>) -> impl ImageBuilder {
        ImageBuilderImpl::<fn(String), fn(ImageChunk) -> ImageChunk> {
            data: data.into(),
            format: Some(ContentFormat::Base64),
            media_type: None,
            detail: None,
            error_handler: None,
            chunk_handler: None,
            operations: Vec::new(),
        }
    }

    /// Semantic entry point - EXACT syntax: Image::from_url(url)
    pub fn from_url(url: impl Into<String>) -> impl ImageBuilder {
        ImageBuilderImpl::<fn(String), fn(ImageChunk) -> ImageChunk> {
            data: url.into(),
            format: Some(ContentFormat::Url),
            media_type: None,
            detail: None,
            error_handler: None,
            chunk_handler: None,
            operations: Vec::new(),
        }
    }

    /// Semantic entry point - EXACT syntax: Image::from_path(path)
    pub fn from_path(path: impl Into<String>) -> impl ImageBuilder {
        ImageBuilderImpl::<fn(String), fn(ImageChunk) -> ImageChunk> {
            data: path.into(),
            format: Some(ContentFormat::Url),
            media_type: None,
            detail: None,
            error_handler: None,
            chunk_handler: None,
            operations: Vec::new(),
        }
    }
}
