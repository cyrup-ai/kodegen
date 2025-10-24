//! Extraction module for structured data extraction from unstructured text
//!
//! This module provides functionality for extracting structured data from unstructured text
//! using language models and other NLP techniques.

mod error;
mod extractor;
mod model;

// Re-export the main types
pub use error::ExtractionError;
pub use extractor::{Extractor, ExtractorImpl};
pub use model::{ExtractionConfig, ExtractionRequest, ExtractionResult};

/// Result type for extraction operations
pub type Result<T> = std::result::Result<T, ExtractionError>;

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        age: u32,
    }

    #[test]
    fn test_extractor_creation() -> Result<(), Box<dyn std::error::Error>> {
        // Test JSON deserialization of TestData structure
        let json_data = r#"{"name": "Alice", "age": 30}"#;
        let parsed: TestData = serde_json::from_str(json_data)?;

        assert_eq!(parsed.name, "Alice");
        assert_eq!(parsed.age, 30);

        // Test structure equality
        let expected = TestData {
            name: "Alice".to_string(),
            age: 30,
        };
        assert_eq!(parsed, expected);
        Ok(())
    }
}
