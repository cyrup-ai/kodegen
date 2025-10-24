//! Document Types and Loading
//!
//! This module provides document types, content formats, and basic document loading
//! functionality. Originally from document.rs.

// Removed cyrup_http3::async_task import - not needed for streams-only architecture
use std::collections::HashMap;

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Use the local DocumentMediaType instead of importing from non-existent module
// DocumentMediaType is defined locally as CandleDocumentMediaType

/// Candle document structure for storing document data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleDocument {
    /// The document content data as a string
    pub data: String,
    /// Optional format specification for the document content
    pub format: Option<CandleContentFormat>,
    /// Optional media type classification for the document
    pub media_type: Option<CandleDocumentMediaType>,
    /// Additional properties stored as key-value pairs
    #[serde(flatten)]
    pub additional_props: HashMap<String, Value>,
}

impl MessageChunk for CandleDocument {
    fn bad_chunk(error: String) -> Self {
        CandleDocument {
            data: format!("ERROR: {error}"),
            format: Some(CandleContentFormat::Text),
            media_type: Some(CandleDocumentMediaType::TXT),
            additional_props: HashMap::new(),
        }
    }

    fn error(&self) -> Option<&str> {
        if self.data.starts_with("ERROR: ") {
            Some(&self.data)
        } else {
            None
        }
    }
}

impl Default for CandleDocument {
    fn default() -> Self {
        CandleDocument {
            data: String::new(),
            format: Some(CandleContentFormat::Text),
            media_type: Some(CandleDocumentMediaType::TXT),
            additional_props: HashMap::new(),
        }
    }
}

/// Candle content format enum specifying how document data is encoded
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CandleContentFormat {
    /// Base64 encoded binary data
    Base64,
    /// Plain text content
    Text,
    /// HTML formatted content
    Html,
    /// Markdown formatted content
    Markdown,
    /// JSON formatted content
    Json,
    /// XML formatted content
    Xml,
    /// YAML formatted content
    Yaml,
    /// CSV formatted content
    Csv,
}

/// Candle document media type enum for classifying document formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CandleDocumentMediaType {
    /// PDF document format
    PDF,
    /// Microsoft Word document format
    DOCX,
    /// Plain text file format
    TXT,
    /// Rich Text Format
    RTF,
    /// `OpenDocument` Text format
    ODT,
    /// JSON document format
    Json,
    /// HTML document format
    Html,
    /// Markdown document format
    Markdown,
    /// XML document format
    Xml,
    /// YAML document format
    Yaml,
    /// CSV document format
    Csv,
    /// Plain text document format
    PlainText,
    /// Generic document format
    Document,
    /// Image file format
    Image,
    /// Binary file format
    Binary,
}

// Builder implementations moved to cyrup/src/builders/document.rs

impl CandleDocument {
    /// Extract the text content from the document
    #[must_use]
    pub fn content(&self) -> String {
        match self.format {
            Some(CandleContentFormat::Base64) => "[Base64 Document]".to_string(),
            _ => self.data.clone(),
        }
    }
}

/// Simple document loader for domain use
pub struct DocumentLoader {
    path: String,
}

impl DocumentLoader {
    /// Load the document (simplified version)
    #[must_use]
    pub fn load(self) -> CandleDocument {
        // For domain use, return a simple text document
        // Full loading logic is in cyrup builders
        CandleDocument {
            data: format!("Document from: {}", self.path),
            format: Some(CandleContentFormat::Text),
            media_type: Some(CandleDocumentMediaType::TXT),
            additional_props: HashMap::new(),
        }
    }
}

// All builder implementations moved to cyrup/src/builders/document.rs

// Removed BadTraitImpl implementation - not needed for streams-only architecture
// impl BadTraitImpl for Document {
//     fn bad_impl(error: &str) -> Self {
//         eprintln!("Document BadTraitImpl: {}", error);
//         Document {
//             data: format!("Error loading document: {}", error),
//             format: Some(CandleContentFormat::Text),
//             media_type: Some(CandleDocumentMediaType::TXT),
//             additional_props: HashMap::new(),
//         }
//     }
// }
