//! DocumentBuilder trait definition - public API contract

use crate::domain::context::{
    CandleContentFormat as ContentFormat, CandleDocument as Document,
    CandleDocumentChunk as DocumentChunk, CandleDocumentMediaType as DocumentMediaType,
};
use cyrup_sugars::ZeroOneOrMany;
use serde_json::Value;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::Stream;

/// Document builder trait - elegant zero-allocation builder pattern
pub trait DocumentBuilder: Sized {
    /// Set content format - EXACT syntax: .format(ContentFormat::Markdown)
    fn format(self, format: ContentFormat) -> impl DocumentBuilder;

    /// Set media type - EXACT syntax: .media_type(DocumentMediaType::PDF)
    fn media_type(self, media_type: DocumentMediaType) -> impl DocumentBuilder;

    /// Set encoding - EXACT syntax: .encoding("utf-8")
    fn encoding(self, encoding: impl Into<String>) -> impl DocumentBuilder;

    /// Set maximum file size - EXACT syntax: .max_size(1024 * 1024)
    fn max_size(self, size: usize) -> impl DocumentBuilder;

    /// Set request timeout - EXACT syntax: .timeout(30000)
    fn timeout(self, timeout_ms: u64) -> impl DocumentBuilder;

    /// Set retry attempts - EXACT syntax: .retry(5)
    fn retry(self, attempts: u8) -> impl DocumentBuilder;

    /// Enable/disable caching - EXACT syntax: .cache(true)
    fn cache(self, enabled: bool) -> impl DocumentBuilder;

    /// Set GitHub branch - EXACT syntax: .branch("main")
    fn branch(self, branch: impl Into<String>) -> impl DocumentBuilder;

    /// Add metadata property - EXACT syntax: .property("key", "value")
    fn property(self, key: impl Into<String>, value: impl Into<Value>) -> impl DocumentBuilder;

    /// Add multiple properties - EXACT syntax: .properties([("key", "value")])
    fn properties<F>(self, f: F) -> impl DocumentBuilder
    where
        F: FnOnce() -> BTreeMap<String, Value>;

    /// Add error handler - EXACT syntax: .on_error(|error| { ... })
    fn on_error<F>(self, handler: F) -> impl DocumentBuilder
    where
        F: Fn(String) + Send + Sync + 'static + Clone;

    /// Add chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    fn on_chunk<F>(self, handler: F) -> impl DocumentBuilder
    where
        F: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone;

    /// Synchronous load for immediate data only - EXACT syntax: .load()
    fn load(self) -> Document;

    /// Load document asynchronously - EXACT syntax: .load_async()
    fn load_async(self) -> Pin<Box<dyn Future<Output = Document> + Send>>;

    /// Load multiple documents - EXACT syntax: .load_all()
    fn load_all(self) -> Pin<Box<dyn Future<Output = ZeroOneOrMany<Document>> + Send>>;

    /// Stream documents one by one - EXACT syntax: .stream()
    fn stream(self) -> impl Stream<Item = Document>;

    /// Stream document content in chunks - EXACT syntax: .stream_chunks(1024)
    fn stream_chunks(self, chunk_size: usize) -> impl Stream<Item = DocumentChunk>;

    /// Stream document content line by line - EXACT syntax: .stream_lines()
    fn stream_lines(self) -> impl Stream<Item = DocumentChunk>;
}
