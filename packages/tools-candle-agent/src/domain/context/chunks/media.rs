//! Media-related chunk types for streaming operations
//!
//! This module contains chunk types for various media formats including:
//! - Document/text content
//! - Images (PNG, JPEG, GIF, WebP, BMP, TIFF)
//! - Audio/Voice (MP3, WAV, FLAC, OGG, M4A, OPUS)
//! - Transcription (speech-to-text)
//! - Speech synthesis (text-to-speech)

use std::collections::HashMap;
use std::path::PathBuf;

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Candle chunk of document content for streaming file operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleDocumentChunk {
    /// Optional path to the source file
    pub path: Option<PathBuf>,

    /// The content of this chunk
    pub content: String,

    /// Byte range in the original file
    pub byte_range: Option<(usize, usize)>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

// Convenience constructors
impl CandleDocumentChunk {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            path: None,
            content: content.into(),
            byte_range: None,
            metadata: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.byte_range = Some((start, end));
        self
    }
}

impl MessageChunk for CandleDocumentChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            path: None,
            content: String::new(),
            byte_range: None,
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

/// Candle image format types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CandleImageFormat {
    /// Portable Network Graphics format
    PNG,
    /// Joint Photographic Experts Group format
    JPEG,
    /// Graphics Interchange Format
    GIF,
    /// WebP image format by Google
    WebP,
    /// Bitmap image format
    BMP,
    /// Tagged Image File Format
    TIFF,
}

/// Candle chunk of image data for streaming image operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleImageChunk {
    /// Raw image data
    pub data: Vec<u8>,

    /// Image format
    pub format: CandleImageFormat,

    /// Optional dimensions (width, height)
    pub dimensions: Option<(u32, u32)>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl MessageChunk for CandleImageChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            data: Vec::new(),
            format: CandleImageFormat::PNG,
            dimensions: None,
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

/// Audio format types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioFormat {
    /// MPEG-1 Audio Layer III compressed audio format
    MP3,
    /// Waveform Audio File Format (uncompressed)
    WAV,
    /// Free Lossless Audio Codec
    FLAC,
    /// Ogg Vorbis compressed audio format
    OGG,
    /// MPEG-4 Audio compressed format (AAC)
    M4A,
    /// Opus low-latency audio codec
    OPUS,
}

/// Chunk of audio/voice data for streaming audio operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChunk {
    /// Raw audio data
    pub audio_data: Vec<u8>,

    /// Audio format
    pub format: AudioFormat,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Sample rate in Hz
    pub sample_rate: Option<u32>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl MessageChunk for VoiceChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            audio_data: Vec::new(),
            format: AudioFormat::WAV,
            duration_ms: None,
            sample_rate: None,
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

/// Chunk of transcribed text from speech-to-text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionChunk {
    /// The transcribed text for this chunk
    pub text: String,

    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f32>,

    /// Start time in milliseconds
    pub start_time_ms: Option<u64>,

    /// End time in milliseconds  
    pub end_time_ms: Option<u64>,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl MessageChunk for TranscriptionChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            text: String::new(),
            confidence: None,
            start_time_ms: None,
            end_time_ms: None,
            is_final: false,
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

/// Chunk of synthesized speech for text-to-speech
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechChunk {
    /// Raw audio data
    pub audio_data: Vec<u8>,

    /// Audio format
    pub format: AudioFormat,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Sample rate in Hz
    pub sample_rate: Option<u32>,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl MessageChunk for SpeechChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), Value::String(error));
        Self {
            audio_data: Vec::new(),
            format: AudioFormat::WAV,
            duration_ms: None,
            sample_rate: None,
            is_final: false,
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
