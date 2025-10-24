//! Document Types and Loading
//!
//! This module provides document types, content formats, and basic document loading
//! functionality. Originally from document.rs.

// Removed cyrup_http3::async_task import - not needed for streams-only architecture
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Document structure for storing document data and metadata
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

/// Content format enum specifying how document data is encoded
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
}

/// Document media type enum for classifying document formats
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
    /// OpenDocument Text format
    ODT,
}

// Builder implementations moved to cyrup/src/builders/document.rs

impl CandleDocument {
    /// Create a new document from file path (simplified version for domain use)
    /// Full builder functionality is in cyrup/src/builders/document.rs
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> CandleDocumentLoader {
        CandleDocumentLoader {
            path: path.as_ref().display().to_string()}
    }

    /// Extract the text content from the document
    pub fn content(&self) -> String {
        match self.format {
            Some(CandleContentFormat::Base64) => "[Base64 Document]".to_string(),
            _ => self.data.clone()}
    }
}

/// Simple document loader for domain use
pub struct CandleDocumentLoader {
    path: String}

impl CandleDocumentLoader {
    /// Load the document (simplified version)
    pub fn load(self) -> CandleDocument {
        // For domain use, return a simple text document
        // Full loading logic is in cyrup builders
        CandleDocument {
            data: format!("Document from: {}", self.path),
            format: Some(CandleContentFormat::Text),
            media_type: Some(CandleDocumentMediaType::TXT),
            additional_props: HashMap::new()}
    }
}

// All builder implementations moved to cyrup/src/builders/document.rs

// Removed BadTraitImpl implementation - not needed for streams-only architecture
// impl BadTraitImpl for Document {
//     fn bad_impl(error: &str) -> Self {
//         eprintln!("Document BadTraitImpl: {}", error);
//         Document {
//             data: format!("Error loading document: {}", error),
//             format: Some(ContentFormat::Text),
//             media_type: Some(DocumentMediaType::TXT),
//             additional_props: HashMap::new(),
//         }
//     }
// }
