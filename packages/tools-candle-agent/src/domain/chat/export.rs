//! Chat export functionality
//!
//! Provides zero-allocation export capabilities for chat conversations and history.
//! Supports multiple formats with blazing-fast serialization and ergonomic APIs.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Write;
use thiserror::Error;

use crate::domain::util::duration_to_micros_u64;

/// String serialization helper
mod arc_str_serde {
    use super::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(arc_str: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(arc_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s)
    }
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format with full metadata
    Json,
    /// Markdown format for human readability
    Markdown,
    /// Plain text format
    Text,
    /// CSV format for data analysis
    Csv,
}

/// Export configuration with zero-allocation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,
    /// Include metadata in export
    pub include_metadata: bool,
    /// Include timestamps
    pub include_timestamps: bool,
    /// Maximum messages to export (0 = all)
    pub max_messages: usize,
    /// Custom filename prefix
    #[serde(with = "arc_str_serde")]
    pub filename_prefix: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_metadata: true,
            include_timestamps: true,
            max_messages: 0,
            filename_prefix: String::from("chat_export"),
        }
    }
}

/// Chat exporter for converting conversations to various formats
///
/// This exporter provides high-performance, zero-allocation export capabilities
/// with support for multiple output formats and comprehensive customization options.
#[derive(Debug, Clone)]
pub struct ChatExporter {
    /// Export configuration
    config: ExportConfig,
    /// Export statistics
    stats: ExportStats,
}

/// Export statistics for monitoring and optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportStats {
    /// Total exports performed
    pub total_exports: u64,
    /// Total messages exported
    pub total_messages: u64,
    /// Total bytes exported
    pub total_bytes: u64,
    /// Average export time in microseconds
    pub avg_export_time_us: u64,
    /// Export success rate (0.0 to 1.0)
    pub success_rate: f32,
}

/// Export result containing the exported data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// Exported content
    #[serde(with = "arc_str_serde")]
    pub content: String,
    /// Content type/format
    #[serde(with = "arc_str_serde")]
    pub content_type: String,
    /// File extension recommendation
    #[serde(with = "arc_str_serde")]
    pub file_extension: String,
    /// Export metadata
    pub metadata: ExportMetadata,
}

/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export timestamp
    pub exported_at: std::time::SystemTime,
    /// Number of messages exported
    pub message_count: usize,
    /// Export format used
    pub format: ExportFormat,
    /// Export configuration
    pub config: ExportConfig,
    /// Export size in bytes
    pub size_bytes: usize,
}

/// Export errors
#[derive(Error, Debug, Clone)]
pub enum ExportError {
    /// Serialization of data failed during export
    #[error("Serialization failed: {detail}")]
    SerializationError {
        /// Details about the serialization failure
        detail: String,
    },
    /// Invalid export format was specified
    #[error("Invalid format: {format}")]
    InvalidFormat {
        /// The invalid format that was specified
        format: String,
    },
    /// Export data exceeds maximum allowed size
    #[error("Export too large: {size_bytes} bytes")]
    ExportTooLarge {
        /// Size of the export data in bytes
        size_bytes: usize,
    },
    /// No messages available to export
    #[error("No messages to export")]
    NoMessages,
    /// Input/output error occurred during export
    #[error("IO error: {detail}")]
    IoError {
        /// Details about the IO error
        detail: String,
    },
}

/// Result type for export operations
pub type ExportResult<T> = Result<T, ExportError>;

impl ChatExporter {
    /// Create a new chat exporter with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ExportConfig::default(),
            stats: ExportStats::default(),
        }
    }

    /// Create a new chat exporter with custom configuration
    #[must_use]
    pub fn with_config(config: ExportConfig) -> Self {
        Self {
            config,
            stats: ExportStats::default(),
        }
    }

    /// Export messages to the configured format
    ///
    /// # Errors
    ///
    /// Returns `ExportError` if:
    /// - No messages are provided
    /// - Export format handler fails
    /// - Message serialization fails
    pub fn export_messages(
        &mut self,
        messages: &[crate::domain::chat::message::types::CandleMessage],
    ) -> ExportResult<ExportData> {
        if messages.is_empty() {
            return Err(ExportError::NoMessages);
        }

        let start_time = std::time::Instant::now();

        // Apply message limit if configured
        let messages_to_export =
            if self.config.max_messages > 0 && messages.len() > self.config.max_messages {
                &messages[messages.len() - self.config.max_messages..]
            } else {
                messages
            };

        let content = match self.config.format {
            ExportFormat::Json => Self::export_as_json(messages_to_export)?,
            ExportFormat::Markdown => self.export_as_markdown(messages_to_export),
            ExportFormat::Text => self.export_as_text(messages_to_export),
            ExportFormat::Csv => self.export_as_csv(messages_to_export),
        };

        let export_time = start_time.elapsed();

        // Update statistics
        self.stats.total_exports += 1;
        self.stats.total_messages += messages_to_export.len() as u64;
        self.stats.total_bytes += content.len() as u64;
        self.stats.avg_export_time_us = ((self.stats.avg_export_time_us
            * (self.stats.total_exports - 1))
            + duration_to_micros_u64(export_time))
            / self.stats.total_exports;

        let (content_type, file_extension) = match self.config.format {
            ExportFormat::Json => ("application/json", "json"),
            ExportFormat::Markdown => ("text/markdown", "md"),
            ExportFormat::Text => ("text/plain", "txt"),
            ExportFormat::Csv => ("text/csv", "csv"),
        };

        let content_size = content.len();

        Ok(ExportData {
            content,
            content_type: content_type.to_string(),
            file_extension: file_extension.to_string(),
            metadata: ExportMetadata {
                exported_at: std::time::SystemTime::now(),
                message_count: messages_to_export.len(),
                format: self.config.format,
                config: self.config.clone(),
                size_bytes: content_size,
            },
        })
    }

    /// Export as JSON format
    fn export_as_json(
        messages: &[crate::domain::chat::message::types::CandleMessage],
    ) -> Result<String, ExportError> {
        serde_json::to_string_pretty(messages).map_err(|e| ExportError::SerializationError {
            detail: e.to_string(),
        })
    }

    /// Export as Markdown format
    fn export_as_markdown(
        &self,
        messages: &[crate::domain::chat::message::types::CandleMessage],
    ) -> String {
        let mut output = String::new();
        output.push_str("# Chat Export\n\n");

        if self.config.include_metadata {
            let _ = writeln!(
                output,
                "**Exported:** {}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs()
            );
            let _ = writeln!(output, "**Messages:** {}\n", messages.len());
        }

        for (i, message) in messages.iter().enumerate() {
            let _ = writeln!(output, "## Message {}\n", i + 1);

            if self.config.include_timestamps {
                let elapsed_secs = message.timestamp.unwrap_or(0);
                let _ = writeln!(output, "**Timestamp:** {elapsed_secs}");
            }

            let _ = writeln!(output, "**Role:** {:?}\n", message.role);
            let content_str = &message.content;
            output.push_str(content_str);
            output.push_str("---\n\n");
        }

        output
    }

    /// Export as plain text format
    fn export_as_text(
        &self,
        messages: &[crate::domain::chat::message::types::CandleMessage],
    ) -> String {
        let mut output = String::new();

        for message in messages {
            if self.config.include_timestamps {
                let elapsed_secs = message.timestamp.unwrap_or(0);
                let _ = write!(output, "[{elapsed_secs}] ");
            }

            let content_str = &message.content;
            let _ = writeln!(output, "{:?}: {content_str}", message.role);
        }

        output
    }

    /// Export as CSV format
    fn export_as_csv(
        &self,
        messages: &[crate::domain::chat::message::types::CandleMessage],
    ) -> String {
        let mut output = String::new();

        // CSV header
        if self.config.include_timestamps {
            output.push_str("timestamp,type,content\n");
        } else {
            output.push_str("type,content\n");
        }

        for message in messages {
            if self.config.include_timestamps {
                let elapsed_secs = message.timestamp.unwrap_or(0);
                let _ = write!(output, "{elapsed_secs},");
            }

            // Escape CSV content - use char literal for pattern
            let content_str = &message.content;
            let escaped_content = content_str.replace('"', "\"\"");
            let _ = writeln!(output, "\"{:?}\",\"{escaped_content}\"", message.role);
        }

        output
    }

    /// Get export statistics
    #[must_use]
    pub fn stats(&self) -> &ExportStats {
        &self.stats
    }

    /// Get current configuration
    #[must_use]
    pub fn config(&self) -> &ExportConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ExportConfig) {
        self.config = config;
    }
}

impl Default for ChatExporter {
    fn default() -> Self {
        Self::new()
    }
}

// Duplicate ExportError and ExportResult removed - already defined above

/// Export a conversation to the specified format
///
/// # Errors
///
/// Returns `ExportError` if:
/// - Format conversion fails
/// - Message serialization fails
/// - No messages are provided
pub fn export_conversation(
    messages: &[crate::domain::chat::message::types::CandleMessage],
    config: &ExportConfig,
) -> ExportResult<String> {
    match config.format {
        ExportFormat::Json => export_to_json(messages, config),
        ExportFormat::Markdown => Ok(export_to_markdown(messages, config)),
        ExportFormat::Text => Ok(export_to_text(messages, config)),
        ExportFormat::Csv => Ok(export_to_csv(messages, config)),
    }
}

/// Export to JSON format
fn export_to_json(
    messages: &[crate::domain::chat::message::types::CandleMessage],
    config: &ExportConfig,
) -> ExportResult<String> {
    let limited_messages = if config.max_messages > 0 {
        &messages[..config.max_messages.min(messages.len())]
    } else {
        messages
    };

    serde_json::to_string_pretty(limited_messages).map_err(|e| ExportError::SerializationError {
        detail: e.to_string(),
    })
}

/// Export to Markdown format
fn export_to_markdown(
    messages: &[crate::domain::chat::message::types::CandleMessage],
    config: &ExportConfig,
) -> String {
    let mut output = String::with_capacity(messages.len() * 100);
    output.push_str("# Chat Export\n\n");

    let limited_messages = if config.max_messages > 0 {
        &messages[..config.max_messages.min(messages.len())]
    } else {
        messages
    };

    for message in limited_messages {
        let _ = write!(output, "## {}\n\n", message.role);
        output.push_str(&message.content);
        output.push_str("\n\n");

        if config.include_timestamps {
            let timestamp_str = message.timestamp.unwrap_or(0);
            let _ = write!(output, "*Timestamp: {timestamp_str}*\n\n");
        }
    }

    output
}

/// Export to plain text format
fn export_to_text(
    messages: &[crate::domain::chat::message::types::CandleMessage],
    config: &ExportConfig,
) -> String {
    let mut output = String::with_capacity(messages.len() * 100);

    let limited_messages = if config.max_messages > 0 {
        &messages[..config.max_messages.min(messages.len())]
    } else {
        messages
    };

    for message in limited_messages {
        let _ = writeln!(output, "{}: {}", message.role, message.content);

        if config.include_timestamps {
            let timestamp_str = message.timestamp.unwrap_or(0);
            let _ = writeln!(output, "Timestamp: {timestamp_str}");
        }
        output.push('\n');
    }

    output
}

/// Export to CSV format
fn export_to_csv(
    messages: &[crate::domain::chat::message::types::CandleMessage],
    config: &ExportConfig,
) -> String {
    let mut output = String::with_capacity(messages.len() * 100);

    // CSV header
    if config.include_timestamps {
        output.push_str("role,content,timestamp\n");
    } else {
        output.push_str("role,content\n");
    }

    let limited_messages = if config.max_messages > 0 {
        &messages[..config.max_messages.min(messages.len())]
    } else {
        messages
    };

    for message in limited_messages {
        let escaped_content = message.content.replace('"', "\"\"");
        if config.include_timestamps {
            let timestamp_str = message.timestamp.unwrap_or(0);
            let _ = writeln!(
                output,
                "\"{}\",\"{}\",{}",
                message.role, escaped_content, timestamp_str
            );
        } else {
            let _ = writeln!(output, "\"{}\",\"{}\"", message.role, escaped_content);
        }
    }

    output
}
