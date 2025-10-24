//! Command response formatting and serialization
//!
//! Provides blazing-fast response formatting with streaming support and zero-allocation patterns
//! for production-ready performance and ergonomic APIs.

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use serde_json::{Map, Value};
use tokio::sync::mpsc;

use super::types::{CandleCommandError, CommandInfo, CommandOutput, OutputType};
use crate::domain::util::unix_timestamp_nanos;

/// Response formatter with streaming support
#[derive(Debug, Clone)]
pub struct ResponseFormatter {
    /// Output format
    format: ResponseFormat,
    /// Include timestamps
    include_timestamps: bool,
    /// Include execution metrics
    include_metrics: bool,
    /// Pretty print JSON
    pretty_json: bool,
}

/// Response format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    /// Plain text format
    Text,
    /// JSON format
    Json,
    /// Structured format with metadata
    Structured,
    /// Streaming format for real-time updates
    Streaming,
}

impl Default for ResponseFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseFormatter {
    /// Create a new response formatter
    #[must_use]
    pub fn new() -> Self {
        Self {
            format: ResponseFormat::Text,
            include_timestamps: true,
            include_metrics: false,
            pretty_json: true,
        }
    }

    /// Set response format
    #[must_use]
    pub fn with_format(mut self, format: ResponseFormat) -> Self {
        self.format = format;
        self
    }

    /// Include timestamps in responses
    #[must_use]
    pub fn with_timestamps(mut self, include: bool) -> Self {
        self.include_timestamps = include;
        self
    }

    /// Include execution metrics in responses
    #[must_use]
    pub fn with_metrics(mut self, include: bool) -> Self {
        self.include_metrics = include;
        self
    }

    /// Pretty print JSON responses
    #[must_use]
    pub fn with_pretty_json(mut self, pretty: bool) -> Self {
        self.pretty_json = pretty;
        self
    }

    /// Format command output
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::SerializationError` if JSON serialization fails
    pub fn format_output(&self, output: &CommandOutput) -> Result<String, ResponseError> {
        match self.format {
            ResponseFormat::Text => self.format_text(output),
            ResponseFormat::Json => self.format_json(output),
            ResponseFormat::Structured => self.format_structured(output),
            ResponseFormat::Streaming => self.format_streaming(output),
        }
    }

    /// Format as plain text
    fn format_text(&self, output: &CommandOutput) -> Result<String, ResponseError> {
        let mut result = String::new();

        // Add status indicator based on output type
        if matches!(output.output_type, OutputType::Text) {
            result.push_str("✓ ");
        } else {
            result.push_str("✗ ");
        }

        // Add main message
        result.push_str(&output.content);

        // Add execution time if metrics are enabled
        if self.include_metrics && output.execution_time > 0 {
            write!(&mut result, " ({}μs)", output.execution_time).map_err(|e| {
                ResponseError::FormatError {
                    detail: format!("Failed to write metrics: {e}"),
                }
            })?;
        }

        // Add timestamp if enabled
        if self.include_timestamps {
            let timestamp = chrono::Utc::now().format("%H:%M:%S");
            write!(&mut result, " [{timestamp}]").map_err(|e| ResponseError::FormatError {
                detail: format!("Failed to write timestamp: {e}"),
            })?;
        }

        Ok(result)
    }

    /// Format as JSON
    fn format_json(&self, output: &CommandOutput) -> Result<String, ResponseError> {
        let mut json_output = Map::new();

        json_output.insert(
            "success".to_string(),
            Value::Bool(matches!(output.output_type, OutputType::Text)),
        );
        json_output.insert("message".to_string(), Value::String(output.content.clone()));

        if let Some(ref data) = output.data {
            json_output.insert("data".to_string(), data.clone());
        }

        if self.include_metrics {
            let mut metrics = Map::new();
            metrics.insert(
                "execution_time_us".to_string(),
                Value::Number(output.execution_time.into()),
            );
            if let Some(ref usage) = output.resource_usage {
                metrics.insert(
                    "memory_bytes".to_string(),
                    Value::Number(usage.memory_bytes.into()),
                );
                metrics.insert(
                    "cpu_time_us".to_string(),
                    Value::Number(usage.cpu_time_us.into()),
                );
                metrics.insert(
                    "network_requests".to_string(),
                    Value::Number(usage.network_requests.into()),
                );
                metrics.insert(
                    "disk_operations".to_string(),
                    Value::Number(usage.disk_operations.into()),
                );
            }
            json_output.insert("metrics".to_string(), Value::Object(metrics));
        }

        if self.include_timestamps {
            let timestamp = chrono::Utc::now().to_rfc3339();
            json_output.insert("timestamp".to_string(), Value::String(timestamp));
        }

        let json_value = Value::Object(json_output);

        if self.pretty_json {
            serde_json::to_string_pretty(&json_value)
        } else {
            serde_json::to_string(&json_value)
        }
        .map_err(|e| ResponseError::SerializationError {
            detail: e.to_string(),
        })
    }

    /// Format as structured response
    fn format_structured(&self, output: &CommandOutput) -> Result<String, ResponseError> {
        let mut result = String::new();

        // Header
        result.push_str("=== Command Response ===\n");

        // Status
        writeln!(
            &mut result,
            "Status: {}",
            if output.success { "SUCCESS" } else { "FAILED" }
        )
        .map_err(|e| ResponseError::FormatError {
            detail: format!("Failed to write status: {e}"),
        })?;

        // Message
        writeln!(&mut result, "Message: {}", output.message).map_err(|e| {
            ResponseError::FormatError {
                detail: format!("Failed to write message: {e}"),
            }
        })?;

        // Data section
        if let Some(data) = &output.data {
            result.push_str("Data:\n");
            let data_str = serde_json::to_string_pretty(data).map_err(|e| {
                ResponseError::SerializationError {
                    detail: e.to_string(),
                }
            })?;
            for line in data_str.lines() {
                writeln!(&mut result, "  {line}").map_err(|e| ResponseError::FormatError {
                    detail: format!("Failed to write data line: {e}"),
                })?;
            }
        }

        // Metrics section
        if self.include_metrics {
            result.push_str("Metrics:\n");
            writeln!(&mut result, "  Execution Time: {}μs", output.execution_time).map_err(
                |e| ResponseError::FormatError {
                    detail: format!("Failed to write execution time: {e}"),
                },
            )?;
            if let Some(ref usage) = output.resource_usage {
                writeln!(&mut result, "  Memory Usage: {} bytes", usage.memory_bytes).map_err(
                    |e| ResponseError::FormatError {
                        detail: format!("Failed to write memory usage: {e}"),
                    },
                )?;
                writeln!(&mut result, "  CPU Time: {}μs", usage.cpu_time_us).map_err(|e| {
                    ResponseError::FormatError {
                        detail: format!("Failed to write CPU time: {e}"),
                    }
                })?;
                writeln!(
                    &mut result,
                    "  Network Requests: {}",
                    usage.network_requests
                )
                .map_err(|e| ResponseError::FormatError {
                    detail: format!("Failed to write network requests: {e}"),
                })?;
                writeln!(&mut result, "  Disk Operations: {}", usage.disk_operations).map_err(
                    |e| ResponseError::FormatError {
                        detail: format!("Failed to write disk operations: {e}"),
                    },
                )?;
            }
        }

        // Timestamp
        if self.include_timestamps {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            writeln!(&mut result, "Timestamp: {timestamp}").map_err(|e| {
                ResponseError::FormatError {
                    detail: format!("Failed to write timestamp: {e}"),
                }
            })?;
        }

        result.push_str("========================\n");

        Ok(result)
    }

    /// Format for streaming
    fn format_streaming(&self, output: &CommandOutput) -> Result<String, ResponseError> {
        // For streaming, we use a compact JSON format
        let mut json_output = Map::new();
        json_output.insert(
            "type".to_string(),
            Value::String("command_response".to_string()),
        );
        json_output.insert("success".to_string(), Value::Bool(output.success));
        json_output.insert("message".to_string(), Value::String(output.message.clone()));

        if let Some(data) = &output.data {
            json_output.insert("data".to_string(), data.clone());
        }

        if self.include_timestamps {
            let timestamp = chrono::Utc::now().timestamp_millis();
            json_output.insert("timestamp".to_string(), Value::Number(timestamp.into()));
        }

        let json_value = Value::Object(json_output);
        serde_json::to_string(&json_value).map_err(|e| ResponseError::SerializationError {
            detail: e.to_string(),
        })
    }

    /// Format error response
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::SerializationError` if JSON serialization fails
    pub fn format_error(&self, error: &CandleCommandError) -> Result<String, ResponseError> {
        let output = CommandOutput {
            execution_id: 0,
            content: error.to_string(),
            output_type: OutputType::Text,
            timestamp_nanos: unix_timestamp_nanos(),
            is_final: true,
            execution_time: 0,
            success: false,
            message: error.to_string(),
            data: None,
            resource_usage: None,
        };

        self.format_output(&output)
    }

    /// Format help response
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::SerializationError` if JSON serialization fails
    pub fn format_help(&self, commands: &[CommandInfo]) -> Result<String, ResponseError> {
        match self.format {
            ResponseFormat::Json => self.format_help_json(commands),
            _ => Ok(Self::format_help_text(commands)),
        }
    }

    /// Format help as text
    fn format_help_text(commands: &[CommandInfo]) -> String {
        let mut result = String::new();
        result.push_str("Available Commands:\n\n");

        // Group commands by category
        let mut categories: HashMap<String, Vec<&CommandInfo>> = HashMap::new();
        for command in commands {
            categories
                .entry(command.category.clone())
                .or_default()
                .push(command);
        }

        // Format each category
        for (category, category_commands) in categories {
            let _ = writeln!(&mut result, "{category}:");

            for command in category_commands {
                let _ = writeln!(
                    &mut result,
                    "  /{:<12} - {}",
                    command.name, command.description
                );

                // Add aliases if any
                if !command.aliases.is_empty() {
                    let aliases = command
                        .aliases
                        .iter()
                        .map(|a| format!("/{a}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = writeln!(&mut result, "               (aliases: {aliases})");
                }
            }
            result.push('\n');
        }

        result
    }

    /// Format help as JSON
    fn format_help_json(&self, commands: &[CommandInfo]) -> Result<String, ResponseError> {
        let json_commands: Vec<Value> = commands
            .iter()
            .map(|cmd| {
                let mut command_obj = Map::new();
                command_obj.insert("name".to_string(), Value::String(cmd.name.clone()));
                command_obj.insert(
                    "description".to_string(),
                    Value::String(cmd.description.clone()),
                );
                command_obj.insert("usage".to_string(), Value::String(cmd.usage.clone()));
                command_obj.insert("category".to_string(), Value::String(cmd.category.clone()));

                let aliases: Vec<Value> = cmd
                    .aliases
                    .iter()
                    .map(|a| Value::String(a.clone()))
                    .collect();
                command_obj.insert("aliases".to_string(), Value::Array(aliases));

                let examples: Vec<Value> = cmd
                    .examples
                    .iter()
                    .map(|e| Value::String(e.clone()))
                    .collect();
                command_obj.insert("examples".to_string(), Value::Array(examples));

                Value::Object(command_obj)
            })
            .collect();

        let result = Value::Array(json_commands);

        if self.pretty_json {
            serde_json::to_string_pretty(&result)
        } else {
            serde_json::to_string(&result)
        }
        .map_err(|e| ResponseError::SerializationError {
            detail: e.to_string(),
        })
    }

    /// Create streaming response channel
    #[must_use]
    pub fn create_streaming_channel(&self) -> (StreamingSender, StreamingReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (StreamingSender::new(tx), StreamingReceiver::new(rx))
    }
}

/// Streaming response sender
#[derive(Debug)]
pub struct StreamingSender {
    /// The underlying sender for streaming messages
    sender: mpsc::UnboundedSender<StreamingMessage>,
}

impl StreamingSender {
    fn new(sender: mpsc::UnboundedSender<StreamingMessage>) -> Self {
        Self { sender }
    }

    /// Send a streaming message
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::StreamingError` if the message cannot be sent
    pub fn send(&self, message: StreamingMessage) -> Result<(), ResponseError> {
        self.sender
            .send(message)
            .map_err(|_| ResponseError::StreamingError {
                detail: "Failed to send streaming message".to_string(),
            })
    }

    /// Send progress update
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::StreamingError` if the message cannot be sent
    pub fn send_progress(
        &self,
        current: u64,
        total: u64,
        message: &str,
    ) -> Result<(), ResponseError> {
        self.send(StreamingMessage::Progress {
            current,
            total,
            message: message.to_string(),
        })
    }

    /// Send partial result
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::StreamingError` if the message cannot be sent
    pub fn send_partial(&self, data: Value) -> Result<(), ResponseError> {
        self.send(StreamingMessage::PartialResult { data })
    }

    /// Send completion
    ///
    /// # Errors
    ///
    /// Returns `ResponseError::StreamingError` if the message cannot be sent
    pub fn send_complete(&self, output: CommandOutput) -> Result<(), ResponseError> {
        self.send(StreamingMessage::Complete { output })
    }
}

/// Streaming response receiver
#[derive(Debug)]
pub struct StreamingReceiver {
    /// The underlying receiver for streaming messages
    receiver: mpsc::UnboundedReceiver<StreamingMessage>,
}

impl StreamingReceiver {
    fn new(receiver: mpsc::UnboundedReceiver<StreamingMessage>) -> Self {
        Self { receiver }
    }

    /// Receive next streaming message
    pub async fn recv(&mut self) -> Option<StreamingMessage> {
        self.receiver.recv().await
    }
}

/// Streaming message types
#[derive(Debug, Clone)]
pub enum StreamingMessage {
    /// Progress update
    Progress {
        /// Current progress count
        current: u64,
        /// Total progress count
        total: u64,
        /// Progress message
        message: String,
    },
    /// Partial result
    PartialResult {
        /// Partial result data
        data: Value,
    },
    /// Command completion
    Complete {
        /// Final command output
        output: CommandOutput,
    },
}

/// Response formatting errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResponseError {
    /// Serialization error occurred
    #[error("Serialization error: {detail}")]
    SerializationError {
        /// Error detail message
        detail: String,
    },

    /// Streaming error occurred
    #[error("Streaming error: {detail}")]
    StreamingError {
        /// Error detail message
        detail: String,
    },

    /// Format error occurred
    #[error("Format error: {detail}")]
    FormatError {
        /// Error detail message
        detail: String,
    },
}

/// Global response formatter
static GLOBAL_FORMATTER: LazyLock<ResponseFormatter> = LazyLock::new(ResponseFormatter::new);

/// Get global response formatter
#[must_use]
pub fn get_global_formatter() -> &'static ResponseFormatter {
    &GLOBAL_FORMATTER
}

/// Format command output using default formatter
///
/// # Errors
///
/// Returns `ResponseError::SerializationError` if JSON serialization fails
pub fn format_global_output(output: &CommandOutput) -> Result<String, ResponseError> {
    get_global_formatter().format_output(output)
}

/// Format command error using default formatter
///
/// # Errors
///
/// Returns `ResponseError::SerializationError` if JSON serialization fails
pub fn format_global_error(error: &CandleCommandError) -> Result<String, ResponseError> {
    get_global_formatter().format_error(error)
}

/// Format help information using default formatter
///
/// # Errors
///
/// Returns `ResponseError::SerializationError` if JSON serialization fails
pub fn format_global_help(commands: &[CommandInfo]) -> Result<String, ResponseError> {
    get_global_formatter().format_help(commands)
}
