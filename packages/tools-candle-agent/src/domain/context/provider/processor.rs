//! Processor module for Candle context provider system
//!
//\! This module contains the ``CandleStreamingContextProcessor`` which handles
//! file loading, format detection, and document creation with streaming operations.

use base64::{Engine as _, engine::general_purpose};
use mime_guess;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};
use tokio_stream::Stream;
use uuid::Uuid;

use super::types::{
    CandleContextError, CandleContextEvent, CandleImmutableFileContext, CandleValidationError,
};
use crate::domain::context::CandleDocument as Document;
use cyrup_sugars::prelude::MessageChunk;

/// Streaming context processor with atomic state tracking for Candle
pub struct CandleStreamingContextProcessor {
    /// Unique processor identifier
    processor_id: String,

    /// Atomic performance counters
    context_requests: AtomicU64,
    active_contexts: AtomicUsize,
    total_contexts_processed: AtomicU64,
    successful_contexts: AtomicU64,
    failed_contexts: AtomicU64,
    total_documents_loaded: AtomicU64,
    total_processing_time_nanos: AtomicU64,

    /// Event streaming
    event_sender: Option<tokio::sync::mpsc::UnboundedSender<CandleContextEvent>>,

    /// Performance thresholds
    max_processing_time_ms: u64,
    max_documents_per_context: usize,
    max_concurrent_contexts: usize,
}

impl std::fmt::Debug for CandleStreamingContextProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("`CandleStreamingContextProcessor`")
            .field("processor_id", &self.processor_id)
            .field(
                "context_requests",
                &self
                    .context_requests
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "active_contexts",
                &self
                    .active_contexts
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "total_contexts_processed",
                &self
                    .total_contexts_processed
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "successful_contexts",
                &self
                    .successful_contexts
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "failed_contexts",
                &self
                    .failed_contexts
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "total_documents_loaded",
                &self
                    .total_documents_loaded
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field(
                "total_processing_time_nanos",
                &self
                    .total_processing_time_nanos
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("event_sender", &self.event_sender.is_some())
            .field("max_processing_time_ms", &self.max_processing_time_ms)
            .field("max_documents_per_context", &self.max_documents_per_context)
            .field("max_concurrent_contexts", &self.max_concurrent_contexts)
            .finish()
    }
}

impl CandleStreamingContextProcessor {
    /// Create new streaming context processor
    #[inline]
    #[must_use]
    pub fn new(processor_id: String) -> Self {
        Self {
            processor_id,
            context_requests: AtomicU64::new(0),
            active_contexts: AtomicUsize::new(0),
            total_contexts_processed: AtomicU64::new(0),
            successful_contexts: AtomicU64::new(0),
            failed_contexts: AtomicU64::new(0),
            total_documents_loaded: AtomicU64::new(0),
            total_processing_time_nanos: AtomicU64::new(0),
            event_sender: None,
            max_processing_time_ms: 30000, // 30 seconds default
            max_documents_per_context: 10000,
            max_concurrent_contexts: 100,
        }
    }

    /// Create processor with event streaming
    #[inline]
    #[must_use]
    pub fn with_streaming(
        processor_id: String,
    ) -> (Self, Pin<Box<dyn Stream<Item = CandleContextEvent> + Send>>) {
        let stream = Box::pin(crate::async_stream::spawn_stream(|_tx| async move {
            // Stream created for event processing
        }));
        let mut processor = Self::new(processor_id);
        processor.event_sender = None; // Will be set up separately if needed
        (processor, stream)
    }

    /// Process file context with streaming results - returns unwrapped values
    #[inline]
    pub fn process_file_context(
        &self,
        context: CandleImmutableFileContext,
    ) -> Pin<Box<dyn Stream<Item = Document> + Send>> {
        let _processor_id = self.processor_id.clone();
        let event_sender = self.event_sender.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let start_time = SystemTime::now();

            // Emit context load started event
            if let Some(ref events) = event_sender {
                let _ = events.send(CandleContextEvent::ContextLoadStarted {
                    context_type: "File".to_string(),
                    source: context.path.clone(),
                    timestamp: start_time,
                });
            }

            // Validate input
            if let Err(validation_error) = Self::validate_file_context(&context) {
                let error = CandleContextError::ValidationError(validation_error.to_string());

                // Emit validation failed event before terminating
                if let Some(ref events) = event_sender {
                    let _ = events.send(CandleContextEvent::ValidationFailed {
                        validation_type: "FileContext".to_string(),
                        error: error.to_string(),
                        timestamp: SystemTime::now(),
                    });
                }

                log::error!("File context validation failed: {error}");
                return;
            }

            // Process file context
            let document = Self::load_file_document(&context).await;
            let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
            let _ = tx.send(document);

            // Emit context load completed event
            if let Some(ref events) = event_sender {
                let _ = events.send(CandleContextEvent::ContextLoadCompleted {
                    context_type: "File".to_string(),
                    source: context.path.clone(),
                    documents_loaded: 1,
                    duration,
                    timestamp: SystemTime::now(),
                });
            }
        }))
    }

    /// Validate file context
    fn validate_file_context(
        context: &CandleImmutableFileContext,
    ) -> Result<(), CandleValidationError> {
        if context.path.is_empty() {
            return Err(CandleValidationError::PathValidation(
                "Empty file path".to_string(),
            ));
        }

        if context.size_bytes > 100 * 1024 * 1024 {
            // 100MB limit
            return Err(CandleValidationError::SizeLimitExceeded(format!(
                "File size {} bytes exceeds 100MB limit",
                context.size_bytes
            )));
        }

        Ok(())
    }

    /// Load file document with production-quality file reading
    #[inline]
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn load_file_document(context: &CandleImmutableFileContext) -> Document {
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB default
        const LARGE_FILE_WARNING_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB

        let file_path = Path::new(&context.path);

        // Validate path exists and is a file
        let metadata = match tokio::fs::metadata(file_path).await {
            Ok(meta) => {
                if !meta.is_file() {
                    log::error!(
                        "File context validation failed: Path is not a file: {}",
                        context.path
                    );
                    return Document::bad_chunk(format!("Path is not a file: {}", context.path));
                }
                meta
            }
            Err(e) => {
                log::error!("Failed to read file metadata: {e}");
                return Document::bad_chunk(format!("Failed to access file: {e}"));
            }
        };

        // Check file size limits to prevent OOM
        if metadata.len() > MAX_FILE_SIZE {
            log::error!(
                "File exceeds size limit: {} bytes (max: {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            );
            return Document::bad_chunk(format!(
                "File too large: {} bytes (max {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            ));
        }

        // Warn for large files
        if metadata.len() > LARGE_FILE_WARNING_THRESHOLD {
            log::warn!("Reading large file: {} bytes", metadata.len());
        }

        // Detect MIME type and derive format/media type
        let mime_guess = mime_guess::from_path(file_path);
        let mime_type = mime_guess.first();

        // Determine format and media type based on MIME or extension
        let (format, media_type) = match mime_type {
            Some(mime) => {
                let mime_str = mime.as_ref();
                match mime_str {
                    "text/html" => (
                        crate::domain::context::CandleContentFormat::Html,
                        crate::domain::context::CandleDocumentMediaType::Html,
                    ),
                    "text/markdown" | "text/x-markdown" => (
                        crate::domain::context::CandleContentFormat::Markdown,
                        crate::domain::context::CandleDocumentMediaType::Markdown,
                    ),
                    "application/json" => (
                        crate::domain::context::CandleContentFormat::Json,
                        crate::domain::context::CandleDocumentMediaType::Json,
                    ),
                    "application/xml" | "text/xml" => (
                        crate::domain::context::CandleContentFormat::Xml,
                        crate::domain::context::CandleDocumentMediaType::Xml,
                    ),
                    "application/x-yaml" | "text/yaml" | "text/x-yaml" => (
                        crate::domain::context::CandleContentFormat::Yaml,
                        crate::domain::context::CandleDocumentMediaType::Yaml,
                    ),
                    "text/csv" => (
                        crate::domain::context::CandleContentFormat::Csv,
                        crate::domain::context::CandleDocumentMediaType::Csv,
                    ),
                    "application/pdf" => (
                        crate::domain::context::CandleContentFormat::Base64,
                        crate::domain::context::CandleDocumentMediaType::PDF,
                    ),
                    "text/plain" => {
                        // For text/plain, use extension-based detection for better accuracy
                        Self::detect_format_from_extension(file_path)
                    }
                    _ if mime_str.starts_with("text/") => (
                        crate::domain::context::CandleContentFormat::Text,
                        crate::domain::context::CandleDocumentMediaType::TXT,
                    ),
                    _ if mime_str.starts_with("image/") => (
                        crate::domain::context::CandleContentFormat::Base64,
                        crate::domain::context::CandleDocumentMediaType::Image,
                    ),
                    _ => {
                        // Fall back to extension-based detection
                        Self::detect_format_from_extension(file_path)
                    }
                }
            }
            None => Self::detect_format_from_extension(file_path),
        };

        // Read file content - try UTF-8 first for text formats
        let data = match format {
            crate::domain::context::CandleContentFormat::Base64 => {
                // Binary file - read as bytes and encode
                match tokio::fs::read(file_path).await {
                    Ok(bytes) => general_purpose::STANDARD.encode(&bytes),
                    Err(e) => {
                        log::error!("Failed to read binary file: {e}");
                        return Document::bad_chunk(format!("Failed to read file: {e}"));
                    }
                }
            }
            _ => {
                // Try to read as UTF-8 text first
                match tokio::fs::read_to_string(file_path).await {
                    Ok(text) => text,
                    Err(_) => {
                        // If UTF-8 fails, try as binary
                        match tokio::fs::read(file_path).await {
                            Ok(bytes) => {
                                // Successfully read as binary, encode it
                                log::warn!(
                                    "File is not valid UTF-8, encoding as base64: {}",
                                    context.path
                                );
                                return Document {
                                    data: general_purpose::STANDARD.encode(&bytes),
                                    format: Some(
                                        crate::domain::context::CandleContentFormat::Base64,
                                    ),
                                    media_type: Some(media_type),
                                    additional_props: Self::build_metadata_props(context),
                                };
                            }
                            Err(read_err) => {
                                log::error!("Failed to read file as text or binary: {read_err}");
                                return Document::bad_chunk(format!(
                                    "Failed to read file: {read_err}"
                                ));
                            }
                        }
                    }
                }
            }
        };

        // Create the document with actual content
        Document {
            data,
            format: Some(format),
            media_type: Some(media_type),
            additional_props: Self::build_metadata_props(context),
        }
    }

    /// Helper function to detect format from file extension (case-insensitive)
    #[inline]
    fn detect_format_from_extension(
        path: &Path,
    ) -> (
        crate::domain::context::CandleContentFormat,
        crate::domain::context::CandleDocumentMediaType,
    ) {
        // Convert extension to lowercase for case-insensitive matching
        let ext_lower = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase);

        match ext_lower.as_deref() {
            Some("html" | "htm") => (
                crate::domain::context::CandleContentFormat::Html,
                crate::domain::context::CandleDocumentMediaType::Html,
            ),
            Some("md" | "markdown") => (
                crate::domain::context::CandleContentFormat::Markdown,
                crate::domain::context::CandleDocumentMediaType::Markdown,
            ),
            Some("json") => (
                crate::domain::context::CandleContentFormat::Json,
                crate::domain::context::CandleDocumentMediaType::Json,
            ),
            Some("xml") => (
                crate::domain::context::CandleContentFormat::Xml,
                crate::domain::context::CandleDocumentMediaType::Xml,
            ),
            Some("yaml" | "yml") => (
                crate::domain::context::CandleContentFormat::Yaml,
                crate::domain::context::CandleDocumentMediaType::Yaml,
            ),
            Some("csv") => (
                crate::domain::context::CandleContentFormat::Csv,
                crate::domain::context::CandleDocumentMediaType::Csv,
            ),
            Some("pdf") => (
                crate::domain::context::CandleContentFormat::Base64,
                crate::domain::context::CandleDocumentMediaType::PDF,
            ),
            Some("doc" | "docx") => (
                crate::domain::context::CandleContentFormat::Base64,
                crate::domain::context::CandleDocumentMediaType::DOCX,
            ),
            Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp") => (
                crate::domain::context::CandleContentFormat::Base64,
                crate::domain::context::CandleDocumentMediaType::Image,
            ),
            Some("txt" | "text" | "log") => (
                crate::domain::context::CandleContentFormat::Text,
                crate::domain::context::CandleDocumentMediaType::TXT,
            ),
            _ => (
                crate::domain::context::CandleContentFormat::Text,
                crate::domain::context::CandleDocumentMediaType::PlainText,
            ),
        }
    }

    /// Build metadata properties `HashMap`
    #[inline]
    fn build_metadata_props(
        context: &CandleImmutableFileContext,
    ) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::with_capacity(4);
        props.insert(
            "id".to_string(),
            serde_json::Value::String(Uuid::new_v4().to_string()),
        );
        props.insert(
            "path".to_string(),
            serde_json::Value::String(context.path.clone()),
        );
        props.insert(
            "size".to_string(),
            serde_json::Value::String(context.size_bytes.to_string()),
        );
        props.insert(
            "hash".to_string(),
            serde_json::Value::String(context.content_hash.clone()),
        );
        props
    }

    /// Get processor statistics
    #[inline]
    pub fn get_statistics(&self) -> CandleContextProcessorStatistics {
        CandleContextProcessorStatistics {
            processor_id: self.processor_id.clone(),
            context_requests: self.context_requests.load(Ordering::Relaxed),
            active_contexts: self.active_contexts.load(Ordering::Relaxed),
            total_contexts_processed: self.total_contexts_processed.load(Ordering::Relaxed),
            successful_contexts: self.successful_contexts.load(Ordering::Relaxed),
            failed_contexts: self.failed_contexts.load(Ordering::Relaxed),
            total_documents_loaded: self.total_documents_loaded.load(Ordering::Relaxed),
            success_rate: self.success_rate(),
            average_processing_time_nanos: self.average_processing_time_nanos(),
        }
    }

    /// Calculate success rate
    #[inline]
    #[allow(clippy::cast_precision_loss)] // Acceptable for rate calculations
    fn success_rate(&self) -> f64 {
        let successful = self.successful_contexts.load(Ordering::Relaxed);
        let failed = self.failed_contexts.load(Ordering::Relaxed);
        let total = successful + failed;
        if total == 0 {
            1.0
        } else {
            successful as f64 / total as f64
        }
    }

    /// Calculate average processing time
    #[inline]
    fn average_processing_time_nanos(&self) -> u64 {
        let total_time = self.total_processing_time_nanos.load(Ordering::Relaxed);
        let processed = self.total_contexts_processed.load(Ordering::Relaxed);
        if processed == 0 {
            0
        } else {
            total_time / processed
        }
    }
}

/// Context processor statistics with owned strings for Candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleContextProcessorStatistics {
    pub processor_id: String,
    pub context_requests: u64,
    pub active_contexts: usize,
    pub total_contexts_processed: u64,
    pub successful_contexts: u64,
    pub failed_contexts: u64,
    pub total_documents_loaded: u64,
    pub success_rate: f64,
    pub average_processing_time_nanos: u64,
}
