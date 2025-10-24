//! Document loading implementation for all source types

use super::types::{DocumentBuilderData, DocumentBuilderImpl};
use crate::domain::context::chunks::{CandleStringChunk, CandleZeroOneOrManyChunk};
use crate::domain::context::{CandleDocument as Document, CandleDocumentChunk as DocumentChunk};
use cyrup_sugars::ZeroOneOrMany;
use cyrup_sugars::prelude::MessageChunk;
use quyc::Quyc;
use serde_json::Value;
use std::fs;
use std::marker::PhantomData;
use std::path::Path;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

impl<F1, F2> DocumentBuilderImpl<F1, F2>
where
    F1: Fn(String) + Send + Sync + 'static + Clone,
    F2: Fn(DocumentChunk) -> DocumentChunk + Send + Sync + 'static + Clone,
{
    pub(super) fn load_document_data<G>(
        builder: DocumentBuilderImpl<F1, F2>,
        error_handler: G,
    ) -> Pin<Box<dyn Stream<Item = Document> + Send>>
    where
        G: Fn(String) + Send + 'static,
    {
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let content = match builder.data {
                    DocumentBuilderData::File(ref path) => {
                        let mut file_stream = Self::load_file_content(path, &builder);
                        match file_stream.next().await {
                            Some(content_result) => content_result.text,
                            None => {
                                error_handler("Failed to load file content".to_string());
                                return;
                            }
                        }
                    }
                    DocumentBuilderData::Url(ref url) => {
                        let mut url_stream = Self::load_url_content(url, &builder);
                        match url_stream.next().await {
                            Some(content_result) => content_result.text,
                            None => {
                                error_handler("Failed to load URL content".to_string());
                                return;
                            }
                        }
                    }
                    DocumentBuilderData::Github {
                        ref repo,
                        ref path,
                        ref branch,
                    } => {
                        let mut github_stream =
                            Self::load_github_content(repo, path, branch.as_deref(), &builder);
                        match github_stream.next().await {
                            Some(content_result) => content_result.text,
                            None => {
                                error_handler("Failed to load GitHub content".to_string());
                                return;
                            }
                        }
                    }
                    DocumentBuilderData::Text(ref text) => text.clone(),
                    DocumentBuilderData::Glob(_) => {
                        error_handler("Glob patterns require load_all() or stream()".to_string());
                        return;
                    }
                };

                // Detect format if not specified
                let format = builder
                    .format
                    .unwrap_or_else(|| Self::detect_format(&content, &builder.data));

                // Detect media type if not specified
                let media_type = builder
                    .media_type
                    .unwrap_or_else(|| Self::detect_media_type(&format, &builder.data));

                // Build metadata
                let mut metadata =
                    std::collections::HashMap::with_capacity(builder.additional_props.len() + 4);
                for (key, value) in builder.additional_props {
                    metadata.insert(key, value);
                }

                if let Some(encoding) = builder.encoding {
                    metadata.insert("encoding".to_string(), Value::String(encoding));
                }

                metadata.insert("size".to_string(), Value::Number(content.len().into()));
                metadata.insert(
                    "cache_enabled".to_string(),
                    Value::Bool(builder.cache_enabled),
                );

                let document = Document {
                    data: content,
                    format: Some(format),
                    media_type: Some(media_type),
                    additional_props: metadata,
                };

                let _ = sender.send(document);
            },
        ))
    }

    pub(super) fn load_glob_documents<G>(
        pattern: String,
        builder: DocumentBuilderImpl<F1, F2>,
        error_handler: G,
    ) -> Pin<Box<dyn Stream<Item = CandleZeroOneOrManyChunk<Document>> + Send>>
    where
        G: Fn(String) + Send + Sync + Clone + 'static,
    {
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let paths = match glob::glob(&pattern) {
                    Ok(paths) => paths.filter_map(Result::ok).collect::<Vec<_>>(),
                    Err(e) => {
                        error_handler(format!("Invalid glob pattern: {}", e));
                        let _ = sender.send(CandleZeroOneOrManyChunk(ZeroOneOrMany::None));
                        return;
                    }
                };

                if paths.is_empty() {
                    let _ = sender.send(CandleZeroOneOrManyChunk(ZeroOneOrMany::None));
                    return;
                }

                let mut documents = Vec::with_capacity(paths.len());

                for path in paths {
                    let doc_builder = DocumentBuilderImpl {
                        data: DocumentBuilderData::File(path),
                        format: builder.format,
                        media_type: builder.media_type,
                        additional_props: builder.additional_props.clone(),
                        encoding: builder.encoding.clone(),
                        max_size: builder.max_size,
                        timeout_ms: builder.timeout_ms,
                        retry_attempts: builder.retry_attempts,
                        cache_enabled: builder.cache_enabled,
                        error_handler: None,
                        chunk_handler: None,
                        _marker: PhantomData,
                    };

                    let mut doc_stream =
                        Self::load_document_data(doc_builder, error_handler.clone());
                    match doc_stream.next().await {
                        Some(doc) => {
                            documents.push(doc);
                        }
                        None => {
                            error_handler("Failed to load document from glob pattern".to_string());
                        }
                    }
                }

                let result = match documents.len() {
                    0 => ZeroOneOrMany::None,
                    1 => {
                        let mut iter = documents.into_iter();
                        match iter.next() {
                            Some(doc) => ZeroOneOrMany::One(doc),
                            None => ZeroOneOrMany::None,
                        }
                    }
                    _ => ZeroOneOrMany::Many(documents),
                };

                let _ = sender.send(CandleZeroOneOrManyChunk(result));
            },
        ))
    }

    pub(super) fn load_file_content(
        path: &Path,
        builder: &DocumentBuilderImpl<F1, F2>,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let path = path.to_path_buf();
        let builder = builder.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Check file size first if max_size is set
                if let Some(max_size) = builder.max_size {
                    match fs::metadata(&path) {
                        Ok(metadata) => {
                            if metadata.len() as usize > max_size {
                                return; // Skip sending - file too large
                            }
                        }
                        Err(_) => {
                            return; // Skip sending - metadata error
                        }
                    }
                }

                // Attempt to read with retries
                let mut last_error = String::new();
                for attempt in 0..=builder.retry_attempts {
                    match fs::read_to_string(&path) {
                        Ok(content) => {
                            let _ = sender.send(CandleStringChunk::text(content));
                            return;
                        }
                        Err(e) => {
                            last_error = format!("Attempt {}: {}", attempt + 1, e);
                            if attempt < builder.retry_attempts {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    100 * (1 << attempt), // Exponential backoff
                                ))
                                .await;
                            }
                        }
                    }
                }
                // If all attempts failed, log the error and send error chunk
                log::error!(
                    "Failed to read file after {} attempts: {}",
                    builder.retry_attempts + 1,
                    last_error
                );
                let _ = sender.send(CandleStringChunk::bad_chunk(last_error));
            },
        ))
    }

    pub(super) fn load_url_content(
        url: &str,
        builder: &DocumentBuilderImpl<F1, F2>,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let url = url.to_string();
        let builder = builder.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Attempt request with retries
                for attempt in 0..=builder.retry_attempts {
                    // Use Quyc::get() directly - much simpler!
                    let mut quyc_builder = Quyc::debug();

                    // Set timeout if specified
                    if let Some(timeout_ms) = builder.timeout_ms {
                        quyc_builder =
                            quyc_builder.timeout(std::time::Duration::from_millis(timeout_ms));
                    }

                    // Simple string response type for text content
                    #[derive(serde::Deserialize, Default)]
                    struct StringResponse(String);

                    impl cyrup_sugars::prelude::MessageChunk for StringResponse {
                        fn bad_chunk(error: String) -> Self {
                            StringResponse(format!("ERROR: {}", error))
                        }

                        fn error(&self) -> Option<&str> {
                            if self.0.starts_with("ERROR: ") {
                                Some(&self.0)
                            } else {
                                None
                            }
                        }
                    }

                    // Use quyc to get the content
                    let response: StringResponse = quyc_builder.get(&url).collect_one();

                    if let Some(error) = response.error() {
                        if attempt < builder.retry_attempts {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                100 * (1 << attempt), // Exponential backoff
                            ))
                            .await;
                            continue;
                        }
                        // Final attempt failed, log error and send error chunk
                        log::error!(
                            "Failed to load URL after {} attempts: {}",
                            builder.retry_attempts + 1,
                            error
                        );
                        let _ = sender.send(CandleStringChunk::bad_chunk(error.to_string()));
                        return;
                    }

                    let content = response.0;

                    // Check size if max_size is set
                    if let Some(max_size) = builder.max_size
                        && content.len() > max_size
                    {
                        return; // Skip sending - content too large
                    }

                    let _ = sender.send(CandleStringChunk::text(content));
                    return;
                }
            },
        ))
    }

    pub(super) fn load_github_content(
        repo: &str,
        path: &str,
        branch: Option<&str>,
        builder: &DocumentBuilderImpl<F1, F2>,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let branch = branch.unwrap_or("main");
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, branch, path
        );

        Self::load_url_content(&url, builder)
    }
}
