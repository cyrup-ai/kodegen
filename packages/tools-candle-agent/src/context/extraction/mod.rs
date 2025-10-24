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

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::domain::agent::CandleAgent;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        age: u32}

    // Note: Actual tests would require proper mocking of the Agent and CompletionModel
    // These are placeholders to demonstrate the test structure
    #[test]
    fn test_extractor_creation() {
        // Test would create a mock agent and verify extractor creation
        assert!(true);
    }
}
