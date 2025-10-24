//! Document builder API - factory methods for creating DocumentBuilders

use super::trait_def::DocumentBuilder;
use super::types::{DocumentBuilderData, DocumentBuilderImpl};
use crate::domain::context::{
    CandleContentFormat as ContentFormat, CandleDocument as Document,
    CandleDocumentMediaType as DocumentMediaType,
};
use std::path::Path;

impl Document {
    /// Create document from file path - EXACT syntax: Document::from_file("path/to/file.txt")
    #[inline]
    pub fn from_file<P: AsRef<Path>>(path: P) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::File(path.as_ref().to_path_buf()))
    }

    /// Create document from URL - EXACT syntax: Document::from_url("https://example.com/doc.pdf")
    #[inline]
    pub fn from_url(url: impl Into<String>) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::Url(url.into()))
            .max_size(10 * 1024 * 1024) // 10MB default
            .timeout(30000) // 30s default
    }

    /// Create document from GitHub - EXACT syntax: Document::from_github("owner/repo", "path/to/file.md")
    #[inline]
    pub fn from_github(repo: impl Into<String>, path: impl Into<String>) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::Github {
            repo: repo.into(),
            path: path.into(),
            branch: None,
        })
        .max_size(1024 * 1024) // 1MB default for GitHub files
        .timeout(15000) // 15s default
    }

    /// Create document from glob pattern - EXACT syntax: Document::from_glob("**/*.md")
    #[inline]
    pub fn from_glob(pattern: impl Into<String>) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::Glob(pattern.into()))
            .retry(1)
            .cache(false)
    }

    /// Create document from text - EXACT syntax: Document::from_text("content")
    #[inline]
    pub fn from_text(text: impl Into<String>) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::Text(text.into()))
            .format(ContentFormat::Text)
            .media_type(DocumentMediaType::TXT)
            .encoding("utf-8")
            .retry(0)
            .cache(false)
    }

    /// Create document from base64 data - EXACT syntax: Document::from_base64("base64data")
    #[inline]
    pub fn from_base64(data: impl Into<String>) -> impl DocumentBuilder {
        DocumentBuilderImpl::new(DocumentBuilderData::Text(data.into()))
            .format(ContentFormat::Base64)
            .media_type(DocumentMediaType::PDF)
            .retry(0)
            .cache(false)
    }

    /// Create document from data - EXACT syntax: Document::from_data("content")
    #[inline]
    pub fn from_data(data: impl Into<String>) -> impl DocumentBuilder {
        Self::from_text(data)
    }
}

/// Convenience function for fluent document creation
#[inline]
pub fn document() -> impl DocumentBuilder {
    Document::from_text("")
}
