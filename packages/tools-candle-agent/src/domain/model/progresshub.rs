//! `ProgressHub` model trait for enforcing model download patterns
//!
//! This trait ensures all models (text generation and embedding) follow
//! consistent patterns for model discovery, downloading, and initialization.

use cyrup_sugars::OneOrMany;
use std::error::Error;

/// Model format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// GGUF quantized format
    Gguf,
    /// `SafeTensors` format
    SafeTensors,
    /// `PyTorch` format
    PyTorch,
}

/// Base trait for all models that use `ProgressHub` for downloading
///
/// This trait enforces:
/// - `HuggingFace` model ID declaration
/// - Required files specification
/// - Model format declaration
/// - Consistent initialization patterns
pub trait ProgressHubModel: Send + Sync + 'static {
    /// `HuggingFace` model identifier (e.g., "unsloth/Kimi-K2-Instruct-GGUF")
    fn model_id() -> &'static str
    where
        Self: Sized;

    /// Required files from the model - at least one file required
    /// Examples: "*.gguf", "tokenizer.json", "config.json"
    fn required_files() -> OneOrMany<&'static str>
    where
        Self: Sized;

    /// Model format (GGUF, `SafeTensors`, etc.)
    fn model_format() -> ModelFormat
    where
        Self: Sized;

    /// Download and initialize using `ProgressHub`
    ///
    /// # Errors
    /// Returns error if download fails or model initialization fails
    fn new() -> impl std::future::Future<Output = Result<Self, Box<dyn Error + Send + Sync>>> + Send
    where
        Self: Sized,
    {
        async {
            let results = progresshub::ProgressHub::builder()
                .model(Self::model_id())
                .with_cli_progress()
                .download()
                .await?;

            Self::from_downloaded_model(results)
        }
    }

    /// Initialize from downloaded `ProgressHub` results
    ///
    /// # Arguments
    /// * `results` - `ProgressHub` download results containing model files
    ///
    /// # Errors
    /// Returns error if required files are missing or model initialization fails
    fn from_downloaded_model(
        results: progresshub::types::OneOrMany<progresshub::DownloadResult>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        Self: Sized;
}
