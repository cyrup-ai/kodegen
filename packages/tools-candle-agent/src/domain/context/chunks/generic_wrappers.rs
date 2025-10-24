//! Generic wrapper chunk types for common operations
//!
//\! This module provides generic wrapper types that implement `MessageChunk`
//! for common data types including:
//! - Unit operations (success/error)
//! - String values
//! - JSON values
//! - Collections

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A simple unit chunk type that implements `MessageChunk` for operations that don't return data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CandleUnitChunk {
    /// Operation completed successfully
    #[default]
    Success,
    /// Operation failed with error
    Error(String),
}

impl MessageChunk for CandleUnitChunk {
    fn bad_chunk(error: String) -> Self {
        CandleUnitChunk::Error(error)
    }

    fn error(&self) -> Option<&str> {
        match self {
            CandleUnitChunk::Error(err) => Some(err),
            CandleUnitChunk::Success => None,
        }
    }
}

/// Lightweight statistics for completed text generation
///
/// Communicated via final `CandleStringChunk` to report accurate
/// generation performance measured at the `TextGenerator` layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStats {
    /// Total tokens generated (output tokens only)
    pub tokens_generated: u32,
    /// Wall-clock time spent generating tokens in seconds
    pub elapsed_secs: f64,
    /// Throughput: tokens generated per second
    pub tokens_per_sec: f64,
}

/// Streaming text chunk from `TextGenerator` to Engine layer
///
/// Non-final chunks carry generated text.
/// Final chunk (`is_final=true`) carries generation statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleStringChunk {
    /// Generated text content (empty for final chunk)
    pub text: String,
    /// True if this is the last chunk in the stream
    #[serde(default)]
    pub is_final: bool,
    /// Generation statistics (only present in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<GenerationStats>,
}

impl CandleStringChunk {
    /// Create text chunk (non-final)
    #[must_use]
    pub fn text(text: String) -> Self {
        Self {
            text,
            is_final: false,
            stats: None,
        }
    }

    /// Create final chunk with statistics
    #[must_use]
    pub fn final_with_stats(stats: GenerationStats) -> Self {
        Self {
            text: String::new(),
            is_final: true,
            stats: Some(stats),
        }
    }
}

impl MessageChunk for CandleStringChunk {
    fn bad_chunk(error: String) -> Self {
        CandleStringChunk {
            text: format!("Error: {error}"),
            is_final: false,
            stats: None,
        }
    }

    fn error(&self) -> Option<&str> {
        if self.text.starts_with("Error: ") {
            Some(&self.text)
        } else {
            None
        }
    }
}

/// Wrapper for JSON Value to implement `MessageChunk`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleJsonChunk(pub Value);

impl MessageChunk for CandleJsonChunk {
    fn bad_chunk(error: String) -> Self {
        CandleJsonChunk(Value::String(format!("Error: {error}")))
    }

    fn error(&self) -> Option<&str> {
        match &self.0 {
            Value::String(s) if s.starts_with("Error: ") => Some(s),
            _ => None,
        }
    }
}

/// Generic wrapper for collections to implement `MessageChunk`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleCollectionChunk<T> {
    pub items: T,
    pub error_message: Option<String>,
}

impl<T> Default for CandleCollectionChunk<T>
where
    T: Default,
{
    fn default() -> Self {
        CandleCollectionChunk {
            items: T::default(),
            error_message: None,
        }
    }
}

impl<T> MessageChunk for CandleCollectionChunk<T>
where
    T: Default,
{
    fn bad_chunk(error: String) -> Self {
        CandleCollectionChunk {
            items: T::default(),
            error_message: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// Chunk of embedding data for streaming embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingChunk {
    /// The embedding vector
    pub embeddings: cyrup_sugars::ZeroOneOrMany<f32>,

    /// Index in the batch
    pub index: usize,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl Default for EmbeddingChunk {
    fn default() -> Self {
        Self {
            embeddings: cyrup_sugars::ZeroOneOrMany::None,
            index: 0,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl MessageChunk for EmbeddingChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            embeddings: cyrup_sugars::ZeroOneOrMany::None,
            index: 0,
            metadata,
        }
    }

    fn error(&self) -> Option<&str> {
        if let Some(Value::String(error)) = self.metadata.get("error") {
            Some(error)
        } else {
            None
        }
    }
}

/// Workflow data chunk for streaming JSON values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDataChunk {
    /// The JSON data
    pub data: Value,
    /// Step name that produced this data
    pub step_name: Option<String>,
    /// Processing timestamp
    pub timestamp: Option<u64>,
    /// Error message if this represents an error
    pub error_message: Option<String>,
}

impl MessageChunk for WorkflowDataChunk {
    fn bad_chunk(error: String) -> Self {
        Self {
            data: Value::String(format!("Error: {error}")),
            step_name: Some("error".to_string()),
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            error_message: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl Default for WorkflowDataChunk {
    fn default() -> Self {
        Self {
            data: Value::Null,
            step_name: None,
            timestamp: None,
            error_message: None,
        }
    }
}

impl From<Value> for WorkflowDataChunk {
    fn from(data: Value) -> Self {
        Self {
            data,
            step_name: None,
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            error_message: None,
        }
    }
}
