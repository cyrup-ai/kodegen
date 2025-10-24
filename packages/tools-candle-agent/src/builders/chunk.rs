//! Chunk builder implementations with zero-allocation, lock-free design
//!
//! Provides EXACT API syntax for chunk construction and streaming operations.

use std::path::PathBuf;
use std::collections::HashMap;

use crate::domain::{
    context::chunks::{CandleChatMessageChunk as ChatMessageChunk, CandleCompletionChunk as CompletionChunk, CandleDocumentChunk as DocumentChunk, CandleFinishReason as FinishReason, CandleUsage as Usage},
    chat::message::types::CandleMessageRole as MessageRole};
use serde_json::Value;

/// Zero-allocation document chunk builder with blazing-fast construction
pub struct DocumentChunkBuilder {
    content: String,
    path: Option<PathBuf>,
    byte_range: Option<(usize, usize)>,
    metadata: HashMap<String, Value>}

impl DocumentChunkBuilder {
    /// Create new document chunk builder - EXACT syntax: DocumentChunkBuilder::new(content)
    #[inline]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            path: None,
            byte_range: None,
            metadata: HashMap::new()}
    }

    /// Set file path - EXACT syntax: .with_path(path)
    #[inline]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Set byte range - EXACT syntax: .with_range(start, end)
    #[inline]
    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.byte_range = Some((start, end));
        self
    }

    /// Add metadata - EXACT syntax: .with_metadata(key, value)
    #[inline]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build final document chunk - EXACT syntax: .build()
    #[inline]
    pub fn build(self) -> DocumentChunk {
        DocumentChunk {
            path: self.path,
            content: self.content,
            byte_range: self.byte_range,
            metadata: self.metadata}
    }
}

/// Zero-allocation chat message chunk builder with blazing-fast construction
pub struct ChatMessageChunkBuilder {
    content: String,
    role: MessageRole,
    is_final: bool,
    metadata: HashMap<String, Value>}

impl ChatMessageChunkBuilder {
    /// Create new chat message chunk builder - EXACT syntax: ChatMessageChunkBuilder::new(content, role)
    #[inline]
    pub fn new(content: impl Into<String>, role: MessageRole) -> Self {
        Self {
            content: content.into(),
            role,
            is_final: false,
            metadata: HashMap::new()}
    }

    /// Mark as final chunk - EXACT syntax: .final_chunk()
    #[inline]
    pub fn final_chunk(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Build final chat message chunk - EXACT syntax: .build()
    #[inline]
    pub fn build(self) -> ChatMessageChunk {
        ChatMessageChunk {
            content: self.content,
            role: self.role,
            is_final: self.is_final,
            metadata: self.metadata}
    }
}

/// Zero-allocation completion chunk builder with blazing-fast construction
pub struct CompletionChunkBuilder {
    text: String,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>}

impl CompletionChunkBuilder {
    /// Create new completion chunk builder - EXACT syntax: CompletionChunkBuilder::new(text)
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            finish_reason: None,
            usage: None}
    }

    /// Set finish reason - EXACT syntax: .finished(reason)
    #[inline]
    pub fn finished(mut self, reason: FinishReason) -> Self {
        self.finish_reason = Some(reason);
        self
    }

    /// Set usage information - EXACT syntax: .with_usage(usage)
    #[inline]
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Build final completion chunk - EXACT syntax: .build()
    #[inline]
    pub fn build(self) -> CompletionChunk {
        CompletionChunk {
            text: self.text,
            finish_reason: self.finish_reason,
            usage: self.usage}
    }
}

impl DocumentChunk {
    /// Create document chunk from content - EXACT syntax: DocumentChunk::from_content(content)
    #[inline]
    pub fn from_content(content: impl Into<String>) -> DocumentChunkBuilder {
        DocumentChunkBuilder::new(content)
    }
}

impl ChatMessageChunk {
    /// Create chat message chunk - EXACT syntax: ChatMessageChunk::from_message(content, role)
    #[inline]
    pub fn from_message(content: impl Into<String>, role: MessageRole) -> ChatMessageChunkBuilder {
        ChatMessageChunkBuilder::new(content, role)
    }
}

impl CompletionChunk {
    /// Create completion chunk - EXACT syntax: CompletionChunk::from_text(text)
    #[inline]
    pub fn from_text(text: impl Into<String>) -> CompletionChunkBuilder {
        CompletionChunkBuilder::new(text)
    }
}
