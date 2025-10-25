//! Extraction module for structured data extraction from unstructured text
//!
//! This module provides functionality for extracting structured data from unstructured text
//! using language models and other NLP techniques.

mod error;
mod extractor;
mod model;

// Re-export the main types
pub use error::ExtractionError;
pub use extractor::{Extractor, ExtractorImpl, DocumentExtractor, BatchExtractor};
pub use model::{ExtractionConfig, ExtractionRequest, ExtractionResult};

/// Result type for extraction operations
pub type Result<T> = std::result::Result<T, ExtractionError>;

