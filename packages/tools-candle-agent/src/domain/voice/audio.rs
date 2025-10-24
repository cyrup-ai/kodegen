//! Audio domain types
//!
//! Contains pure data structures for audio processing.
//! Builder implementations are in the cyrup package.

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};

/// Represents audio data with format and media type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    /// The audio data (can be base64-encoded, raw bytes, or a URL)
    pub data: String,

    /// The format of the audio data
    pub format: Option<ContentFormat>,

    /// The media type of the audio
    pub media_type: Option<AudioMediaType>,
}

/// Supported audio content formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContentFormat {
    /// Base64-encoded audio data
    Base64,

    /// Raw binary audio data
    Raw,

    /// URL pointing to audio resource
    Url,
}

/// Supported audio media types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AudioMediaType {
    /// MP3 audio format
    MP3,

    /// WAV audio format
    WAV,

    /// OGG audio format
    OGG,

    /// M4A audio format
    M4A,

    /// FLAC audio format
    FLAC,
}

impl Audio {
    /// Create a new audio instance with basic data
    ///
    /// # Arguments
    /// * `data` - The audio data (base64, raw, or URL)
    ///
    /// # Returns
    /// A new Audio instance with the provided data
    #[inline]
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            format: None,
            media_type: None,
        }
    }

    /// Set the format of the audio data
    #[must_use]
    pub fn with_format(mut self, format: ContentFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the media type of the audio
    #[must_use]
    pub fn with_media_type(mut self, media_type: AudioMediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }

    /// Check if the audio is in base64 format
    #[must_use]
    pub fn is_base64(&self) -> bool {
        self.format == Some(ContentFormat::Base64)
    }

    /// Check if the audio is raw binary data
    #[must_use]
    pub fn is_raw(&self) -> bool {
        self.format == Some(ContentFormat::Raw)
    }

    /// Check if the audio is a URL
    #[must_use]
    pub fn is_url(&self) -> bool {
        self.format == Some(ContentFormat::Url)
    }

    /// Get the audio data as bytes
    ///
    /// # Errors
    /// Returns an error if the data is not in base64 format
    pub fn as_bytes(&self) -> Result<Vec<u8>, String> {
        if self.is_base64() {
            general_purpose::STANDARD
                .decode(&self.data)
                .map_err(|e| e.to_string())
        } else if self.is_raw() {
            Ok(self.data.as_bytes().to_vec())
        } else {
            Err("Cannot get bytes from URL-based audio".to_string())
        }
    }
}
