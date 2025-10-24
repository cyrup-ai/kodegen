//! DocumentBuilder trait implementation for DocumentBuilderImpl

use super::trait_def::DocumentBuilder;
use super::types::{DocumentBuilderData, DocumentBuilderImpl};
use crate::async_stream;
use crate::domain::context::{
    CandleContentFormat as ContentFormat, CandleDocument as Document,
    CandleDocumentChunk as DocumentChunk, CandleDocumentMediaType as DocumentMediaType,
};
use cyrup_sugars::ZeroOneOrMany;
use serde_json::Value;
use std::collections::BTreeMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

impl<F1, F2> DocumentBuilder for DocumentBuilderImpl<F1, F2>
where
    F1: Fn(String) + Send + Sync + 'static + Clone,
    F2: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone,
{
    /// Set content format - EXACT syntax: .format(ContentFormat::Markdown)
    fn format(mut self, format: ContentFormat) -> impl DocumentBuilder {
        self.format = Some(format);
        self
    }

    /// Set media type - EXACT syntax: .media_type(DocumentMediaType::PDF)
    fn media_type(mut self, media_type: DocumentMediaType) -> impl DocumentBuilder {
        self.media_type = Some(media_type);
        self
    }

    /// Set encoding - EXACT syntax: .encoding("utf-8")
    fn encoding(mut self, encoding: impl Into<String>) -> impl DocumentBuilder {
        self.encoding = Some(encoding.into());
        self
    }

    /// Set maximum file size - EXACT syntax: .max_size(1024 * 1024)
    fn max_size(mut self, size: usize) -> impl DocumentBuilder {
        self.max_size = Some(size);
        self
    }

    /// Set request timeout - EXACT syntax: .timeout(30000)
    fn timeout(mut self, timeout_ms: u64) -> impl DocumentBuilder {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set retry attempts - EXACT syntax: .retry(5)
    fn retry(mut self, attempts: u8) -> impl DocumentBuilder {
        self.retry_attempts = attempts;
        self
    }

    /// Enable/disable caching - EXACT syntax: .cache(true)
    fn cache(mut self, enabled: bool) -> impl DocumentBuilder {
        self.cache_enabled = enabled;
        self
    }

    /// Set GitHub branch - EXACT syntax: .branch("main")
    fn branch(mut self, branch: impl Into<String>) -> impl DocumentBuilder {
        if let DocumentBuilderData::Github {
            repo: _,
            path: _,
            branch: branch_field,
        } = &mut self.data
        {
            *branch_field = Some(branch.into());
        }
        self
    }

    /// Add metadata property - EXACT syntax: .property("key", "value")
    fn property(mut self, key: impl Into<String>, value: impl Into<Value>) -> impl DocumentBuilder {
        self.additional_props.insert(key.into(), value.into());
        self
    }

    /// Add multiple properties - EXACT syntax: .properties([("key", "value")])
    fn properties<F>(mut self, f: F) -> impl DocumentBuilder
    where
        F: FnOnce() -> BTreeMap<String, Value>,
    {
        let props = f();
        for (key, value) in props {
            self.additional_props.insert(key, value);
        }
        self
    }

    /// Add error handler - EXACT syntax: .on_error(|error| { ... })
    fn on_error<F>(self, handler: F) -> impl DocumentBuilder
    where
        F: Fn(String) + Send + Sync + 'static + Clone,
    {
        DocumentBuilderImpl {
            data: self.data,
            format: self.format,
            media_type: self.media_type,
            additional_props: self.additional_props,
            encoding: self.encoding,
            max_size: self.max_size,
            timeout_ms: self.timeout_ms,
            retry_attempts: self.retry_attempts,
            cache_enabled: self.cache_enabled,
            error_handler: Some(handler),
            chunk_handler: self.chunk_handler,
            _marker: PhantomData,
        }
    }

    /// Add chunk handler - EXACT syntax: .on_chunk(|chunk| { ... })
    fn on_chunk<F>(self, handler: F) -> impl DocumentBuilder
    where
        F: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone,
    {
        DocumentBuilderImpl {
            data: self.data,
            format: self.format,
            media_type: self.media_type,
            additional_props: self.additional_props,
            encoding: self.encoding,
            max_size: self.max_size,
            timeout_ms: self.timeout_ms,
            retry_attempts: self.retry_attempts,
            cache_enabled: self.cache_enabled,
            error_handler: self.error_handler,
            chunk_handler: Some(handler),
            _marker: PhantomData,
        }
    }

    /// Synchronous load for immediate data only - EXACT syntax: .load()
    fn load(self) -> Document {
        match self.data {
            DocumentBuilderData::Text(data) => Document {
                data,
                format: self.format,
                media_type: self.media_type,
                additional_props: self.additional_props.into_iter().collect(),
            },
            _ => {
                // Return error document instead of panicking
                Document {
                    data: "Error: load() can only be used with immediate data. Use on_error().load_async() for file/url/glob operations.".to_string(),
                    format: Some(ContentFormat::Text),
                    media_type: Some(DocumentMediaType::TXT),
                    additional_props: std::collections::HashMap::new()
                }
            }
        }
    }

    /// Load document asynchronously - EXACT syntax: .load_async()
    fn load_async(self) -> Pin<Box<dyn Future<Output = Document> + Send>> {
        Box::pin(async move {
            if let Some(handler) = self.error_handler.clone() {
                let mut stream = Self::load_document_data(self, handler);
                match stream.next().await {
                    Some(document) => document,
                    None => {
                        // Return empty document on error
                        Document {
                            data: String::new(),
                            format: Some(ContentFormat::Text),
                            media_type: Some(DocumentMediaType::TXT),
                            additional_props: std::collections::HashMap::new(),
                        }
                    }
                }
            } else {
                // No error handler - use default no-op handler
                let default_handler = |_: String| {};
                let mut stream = Self::load_document_data(self, default_handler);
                match stream.next().await {
                    Some(document) => document,
                    None => {
                        // Return empty document on error
                        Document {
                            data: String::new(),
                            format: Some(ContentFormat::Text),
                            media_type: Some(DocumentMediaType::TXT),
                            additional_props: std::collections::HashMap::new(),
                        }
                    }
                }
            }
        })
    }

    /// Load multiple documents - EXACT syntax: .load_all()
    fn load_all(self) -> Pin<Box<dyn Future<Output = ZeroOneOrMany<Document>> + Send>> {
        Box::pin(async move {
            match self.data.clone() {
                DocumentBuilderData::Glob(pattern) => {
                    if let Some(handler) = self.error_handler.clone() {
                        let mut stream = Self::load_glob_documents(pattern, self, handler);
                        match stream.next().await {
                            Some(chunk) => chunk.0,
                            None => ZeroOneOrMany::None,
                        }
                    } else {
                        let default_handler = |_: String| {};
                        let mut stream = Self::load_glob_documents(pattern, self, default_handler);
                        match stream.next().await {
                            Some(chunk) => chunk.0,
                            None => ZeroOneOrMany::None,
                        }
                    }
                }
                _ => {
                    // Single document
                    if let Some(handler) = self.error_handler.clone() {
                        let mut stream = Self::load_document_data(self, handler);
                        match stream.next().await {
                            Some(doc) => ZeroOneOrMany::One(doc),
                            None => ZeroOneOrMany::None,
                        }
                    } else {
                        let default_handler = |_: String| {};
                        let mut stream = Self::load_document_data(self, default_handler);
                        match stream.next().await {
                            Some(doc) => ZeroOneOrMany::One(doc),
                            None => ZeroOneOrMany::None,
                        }
                    }
                }
            }
        })
    }

    /// Stream documents one by one - EXACT syntax: .stream()
    fn stream(self) -> impl Stream<Item = Document> {
        async_stream::spawn_stream(move |tx| async move {
            match self.data {
                DocumentBuilderData::Glob(pattern) => {
                    if let Ok(paths) = glob::glob(&pattern) {
                        for entry in paths.filter_map(Result::ok) {
                            let doc_builder = DocumentBuilderImpl {
                                data: DocumentBuilderData::File(entry),
                                format: self.format,
                                media_type: self.media_type,
                                additional_props: self.additional_props.clone(),
                                encoding: self.encoding.clone(),
                                max_size: self.max_size,
                                timeout_ms: self.timeout_ms,
                                retry_attempts: self.retry_attempts,
                                cache_enabled: self.cache_enabled,
                                error_handler: None,
                                chunk_handler: None,
                                _marker: PhantomData,
                            };

                            if let Some(handler) = self.error_handler.clone() {
                                let handler_clone = handler.clone();
                                let mut doc_stream = Self::load_document_data(doc_builder, handler);
                                match doc_stream.next().await {
                                    Some(doc) => {
                                        let _ = tx.send(doc);
                                    }
                                    None => {
                                        handler_clone("Failed to load document".to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    if let Some(handler) = self.error_handler.clone() {
                        let handler_clone = handler.clone();
                        let mut doc_stream = Self::load_document_data(self, handler);
                        match doc_stream.next().await {
                            Some(doc) => {
                                let _ = tx.send(doc);
                            }
                            None => {
                                handler_clone("Failed to load document".to_string());
                            }
                        }
                    }
                }
            }
        })
    }

    /// Stream document content in chunks - EXACT syntax: .stream_chunks(1024)
    fn stream_chunks(self, chunk_size: usize) -> impl Stream<Item = DocumentChunk> {
        async_stream::spawn_stream(move |tx| async move {
            let chunk_handler = self.chunk_handler.clone();
            let doc = self.load_async().await;

            let content = &doc.data;
            let mut offset = 0;

            while offset < content.len() {
                let end = (offset + chunk_size).min(content.len());
                let mut chunk = DocumentChunk::new(&content[offset..end]).with_range(offset, end);

                // Apply chunk handler if present
                if let Some(ref handler) = chunk_handler {
                    chunk = handler(chunk);
                }

                let _ = tx.send(chunk);
                offset = end;
            }
        })
    }

    /// Stream document content line by line - EXACT syntax: .stream_lines()
    fn stream_lines(self) -> impl Stream<Item = DocumentChunk> {
        async_stream::spawn_stream(move |tx| async move {
            let chunk_handler = self.chunk_handler.clone();
            let doc = self.load_async().await;

            let mut offset = 0;
            for line in doc.data.lines() {
                let mut chunk = DocumentChunk::new(line).with_range(offset, offset + line.len());

                // Apply chunk handler if present
                if let Some(ref handler) = chunk_handler {
                    chunk = handler(chunk);
                }

                let _ = tx.send(chunk);
                offset += line.len() + 1; // +1 for newline
            }
        })
    }
}
