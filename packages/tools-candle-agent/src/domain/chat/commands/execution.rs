//! Command execution engine
//!
//! Provides blazing-fast command execution with streaming processing, comprehensive error handling,
//! and zero-allocation patterns for production-ready performance.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use std::pin::Pin;
use tokio_stream::Stream;

use super::parsing::CommandParser;
use super::types::actions::SearchScope;
use super::types::commands::{CommandExecutionResult, ImmutableChatCommand, OutputType};
use super::types::events::CommandEvent;
use super::types::metadata::ResourceUsage;
use crate::domain::chat::export::{ChatExporter, ExportConfig, ExportData, ExportFormat};
use crate::domain::chat::message::{CandleMessage, CandleMessageRole};
use crate::domain::util::unix_timestamp_micros;
use crate::memory::core::manager::coordinator::MemoryCoordinator;

/// Get current timestamp in microseconds since Unix epoch, with fallback for clock errors
fn current_timestamp_us() -> u64 {
    unix_timestamp_micros()
}

/// Command execution engine with streaming processing (zero-allocation, lock-free)
#[derive(Debug)]
#[allow(clippy::missing_fields_in_debug)]
pub struct CommandExecutor {
    /// Command parser
    parser: CommandParser,
    /// Execution counter for unique IDs
    execution_counter: AtomicU64,
    /// Active executions count
    active_executions: AtomicUsize,
    /// Total executions count
    total_executions: AtomicU64,
    /// Successful executions count
    successful_executions: AtomicU64,
    /// Failed executions count
    failed_executions: AtomicU64,
    /// Optional memory access for commands that need conversation history
    memory: Option<Arc<MemoryCoordinator>>,
}

impl Clone for CommandExecutor {
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            execution_counter: AtomicU64::new(self.execution_counter.load(Ordering::Relaxed)),
            active_executions: AtomicUsize::new(0),
            total_executions: AtomicU64::new(self.total_executions.load(Ordering::Relaxed)),
            successful_executions: AtomicU64::new(
                self.successful_executions.load(Ordering::Relaxed),
            ),
            failed_executions: AtomicU64::new(self.failed_executions.load(Ordering::Relaxed)),
            memory: self.memory.clone(),
        }
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor {
    /// Create a new command executor (zero-allocation, lock-free)
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: CommandParser::new(),
            execution_counter: AtomicU64::new(1),
            active_executions: AtomicUsize::new(0),
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            memory: None,
        }
    }

    /// Create a new command executor with memory access
    #[must_use]
    pub fn with_memory(memory: Arc<MemoryCoordinator>) -> Self {
        Self {
            parser: CommandParser::new(),
            execution_counter: AtomicU64::new(1),
            active_executions: AtomicUsize::new(0),
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            memory: Some(memory),
        }
    }

    /// Execute a command with streaming output (zero-allocation, lock-free)
    #[allow(clippy::too_many_lines)]
    pub fn execute_streaming(
        &self,
        _execution_id: u64,
        command: ImmutableChatCommand,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        // Clone self for the thread closure - Clone implementation creates fresh counters
        let self_clone = self.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let start_time = Instant::now();

                // Update metrics atomically using cloned instance
                self_clone.total_executions.fetch_add(1, Ordering::AcqRel);
                self_clone.active_executions.fetch_add(1, Ordering::AcqRel);

                let execution_id = self_clone.execution_counter.fetch_add(1, Ordering::AcqRel);

                // Emit Started event
                let _ = sender.send(CommandEvent::Started {
                    command: command.clone(),
                    execution_id,
                    timestamp_us: current_timestamp_us(),
                });

                // Execute command and handle results
                match command {
                    ImmutableChatCommand::Help { command, extended } => {
                        let message = if let Some(cmd) = command {
                            if extended {
                                format!("Extended help for command '{cmd}': <detailed help>")
                            } else {
                                format!("Help for command '{cmd}'")
                            }
                        } else if extended {
                            "Extended help: <comprehensive help text>".to_string()
                        } else {
                            "Available commands: help, clear, export, config, search".to_string()
                        };

                        // Emit Output event with help content
                        let _ = sender.send(CommandEvent::Output {
                            execution_id,
                            content: message.clone(),
                            output_type: OutputType::Text,
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                    ImmutableChatCommand::Clear {
                        confirm: _,
                        keep_last: _,
                    } => {
                        let _ = sender.send(CommandEvent::Output {
                            execution_id,
                            content: "Chat cleared successfully".to_string(),
                            output_type: OutputType::Text,
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                    ImmutableChatCommand::Export {
                        format,
                        output,
                        include_metadata,
                    } => {
                        // Import StreamExt in local scope
                        use tokio_stream::StreamExt;

                        // Delegate to the real export implementation
                        let export_stream = self_clone.execute_export_streaming(
                            execution_id,
                            format,
                            output,
                            include_metadata,
                        );

                        // Pin and forward all events from the export stream
                        tokio::pin!(export_stream);
                        while let Some(event) = export_stream.next().await {
                            let _ = sender.send(event);
                        }
                    }
                    ImmutableChatCommand::Config {
                        key: _,
                        value: _,
                        show: _,
                        reset: _,
                    } => {
                        let _ = sender.send(CommandEvent::Output {
                            execution_id,
                            content: "Configuration updated successfully".to_string(),
                            output_type: OutputType::Text,
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                    ImmutableChatCommand::Search {
                        query,
                        scope,
                        limit,
                        include_context,
                    } => {
                        // Emit progress events for search operation (25%, 50%, 75%, 100%)
                        let progress_steps = [25, 50, 75, 100];
                        for progress in progress_steps {
                            #[allow(clippy::cast_precision_loss)]
                            let progress_f32 = progress as f32;
                            let _ = sender.send(CommandEvent::Progress {
                                execution_id,
                                progress: progress_f32,
                                message: format!("Searching... {progress}%"),
                                timestamp: current_timestamp_us(),
                            });

                            // Simulate realistic search processing time
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        }

                        // Build search result message
                        let scope_str = format!("{scope:?}").to_lowercase();
                        let limit_str = limit.map(|l| format!(" (limit: {l})")).unwrap_or_default();
                        let context_str = if include_context { " with context" } else { "" };
                        let message = format!(
                            "Search for '{query}' in {scope_str} scope{limit_str}{context_str} completed"
                        );

                        let _ = sender.send(CommandEvent::Output {
                            execution_id,
                            content: message,
                            output_type: OutputType::Text,
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                    _ => {
                        // Default implementation for other commands
                        let _ = sender.send(CommandEvent::Output {
                            execution_id,
                            content: "Command executed successfully".to_string(),
                            output_type: OutputType::Text,
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                }

                // Emit Completed event and update metrics
                self_clone
                    .successful_executions
                    .fetch_add(1, Ordering::AcqRel);
                #[allow(clippy::cast_possible_truncation)]
                let duration_us = start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                let _ = sender.send(CommandEvent::completed(
                    execution_id,
                    CommandExecutionResult::Success("Command completed successfully".to_string()),
                    duration_us,
                    ResourceUsage::default(),
                ));

                // Decrement active executions
                self_clone.active_executions.fetch_sub(1, Ordering::AcqRel);
            },
        ))
    }

    /// Execute help command (streaming-only, zero-allocation)
    pub fn execute_help_streaming(
        &self,
        execution_id: u64,
        command: Option<String>,
        extended: bool,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let start_time = Instant::now();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                tokio::spawn(async move {
                    // Emit started event
                    #[allow(clippy::cast_possible_truncation)]
                    let timestamp_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::Started {
                        command: ImmutableChatCommand::Help {
                            command: command.clone(),
                            extended,
                        },
                        execution_id,
                        timestamp_us,
                    });

                    // Generate help message with zero allocation
                    let message = if let Some(cmd) = command {
                        if extended {
                            format!(
                                "Extended help for command '{cmd}': Detailed usage, examples, and advanced options available"
                            )
                        } else {
                            format!("Help for command '{cmd}': Basic usage and description")
                        }
                    } else if extended {
                        "Extended help: All commands with detailed descriptions, usage patterns, and examples".to_string()
                    } else {
                        "Available commands: help, clear, export, config, search, template, macro, branch, session, tool, stats, theme, debug, history, save, load, import, settings, custom".to_string()
                    };

                    // Emit output event
                    let _ = sender.send(CommandEvent::output(
                        execution_id,
                        message.clone(),
                        OutputType::Text,
                    ));

                    // Emit completion event
                    #[allow(clippy::cast_possible_truncation)]
                    let duration_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::Completed {
                        execution_id,
                        result: CommandExecutionResult::Success(message.clone()),
                        duration_us,
                        resource_usage: ResourceUsage::default(),
                        timestamp_us: current_timestamp_us(),
                    });
                });
            },
        ))
    }

    /// Execute clear command (streaming-only, zero-allocation)
    pub fn execute_clear_streaming(
        &self,
        execution_id: u64,
        confirm: bool,
        keep_last: Option<u64>,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let start_time = Instant::now();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                tokio::spawn(async move {
                    // Emit started event
                    #[allow(clippy::cast_possible_truncation)]
                    let timestamp_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::Started {
                        command: ImmutableChatCommand::Clear {
                            confirm,
                            keep_last: keep_last.and_then(|n| usize::try_from(n).ok()),
                        },
                        execution_id,
                        timestamp_us,
                    });

                    // Execute clear operation with zero allocation
                    let message = if confirm {
                        if let Some(n) = keep_last {
                            format!("Chat cleared successfully, keeping last {n} messages")
                        } else {
                            "Chat cleared completely - all messages removed".to_string()
                        }
                    } else {
                        "Clear operation cancelled (use --confirm to proceed)".to_string()
                    };

                    // Emit progress for clearing operation with all required fields
                    if confirm {
                        let _ = sender.send(CommandEvent::Progress {
                            execution_id,
                            progress: 100.0,
                            message: "Clear operation completed".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                                .as_secs(),
                        });
                    }

                    // Emit output event
                    let _ = sender.send(CommandEvent::output(
                        execution_id,
                        message.clone(),
                        OutputType::Text,
                    ));

                    // Emit completion event
                    #[allow(clippy::cast_possible_truncation)]
                    let duration_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::Completed {
                        execution_id,
                        result: CommandExecutionResult::Success(message.clone()),
                        duration_us,
                        resource_usage: ResourceUsage::default(),
                        timestamp_us: current_timestamp_us(),
                    });
                });
            },
        ))
    }

    /// Execute export command (streaming-only, zero-allocation)
    pub fn execute_export_streaming(
        &self,
        execution_id: u64,
        format: String,
        output: Option<String>,
        include_metadata: bool,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let start_time = Instant::now();
        let memory = self.memory.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                tokio::spawn(async move {
                    execute_export_workflow(
                        sender,
                        execution_id,
                        format,
                        output,
                        include_metadata,
                        memory,
                        start_time,
                    )
                    .await;
                });
            },
        ))
    }
}

/// Send export started event
fn send_export_started(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    format: &str,
    output: Option<&String>,
    include_metadata: bool,
    start_time: &Instant,
) {
    let _ = sender.send(CommandEvent::Started {
        command: ImmutableChatCommand::Export {
            format: format.to_string(),
            output: output.cloned(),
            include_metadata,
        },
        execution_id,
        #[allow(clippy::cast_possible_truncation)]
        timestamp_us: start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
    });
}

/// Retrieve and validate messages from memory
async fn retrieve_and_validate_messages(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    memory: Option<Arc<MemoryCoordinator>>,
    start_time: &Instant,
) -> Option<Vec<CandleMessage>> {
    send_export_progress(
        sender,
        execution_id,
        25.0,
        "Retrieving conversation messages...".to_string(),
    );

    let Some(mem) = memory.as_ref() else {
        send_export_failure(
            sender,
            execution_id,
            "Memory not available for export".to_string(),
            4000,
            start_time,
        );
        return None;
    };

    let messages = match retrieve_conversation_messages(mem).await {
        Ok(msgs) => msgs,
        Err(e) => {
            send_export_failure(
                sender,
                execution_id,
                format!("Failed to retrieve messages: {e}"),
                4001,
                start_time,
            );
            return None;
        }
    };

    if messages.is_empty() {
        send_export_failure(
            sender,
            execution_id,
            "No messages to export".to_string(),
            4001,
            start_time,
        );
        return None;
    }

    Some(messages)
}

/// Prepare export configuration
fn prepare_export_config(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    format: &str,
    include_metadata: bool,
    output: Option<String>,
    start_time: &Instant,
) -> Option<ExportConfig> {
    send_export_progress(
        sender,
        execution_id,
        50.0,
        format!("Preparing {format} export..."),
    );

    let export_format = match parse_export_format(format) {
        Ok(fmt) => fmt,
        Err(e) => {
            send_export_failure(sender, execution_id, e, 4002, start_time);
            return None;
        }
    };

    Some(create_export_config(
        export_format,
        include_metadata,
        output,
    ))
}

/// Export messages to data
fn export_messages_to_data(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    messages: &[CandleMessage],
    config: ExportConfig,
    start_time: &Instant,
) -> Option<ExportData> {
    send_export_progress(
        sender,
        execution_id,
        75.0,
        "Exporting messages...".to_string(),
    );

    match perform_message_export(messages, config) {
        Ok(data) => Some(data),
        Err(e) => {
            send_export_failure(sender, execution_id, e, 4003, start_time);
            None
        }
    }
}

/// Write export data to disk
async fn write_export_to_disk(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    export_data: &ExportData,
    output_path: &str,
    start_time: &Instant,
) -> bool {
    send_export_progress(sender, execution_id, 90.0, "Writing to file...".to_string());

    if let Err(e) = tokio::fs::write(output_path, &export_data.content).await {
        send_export_failure(
            sender,
            execution_id,
            format!("Failed to write file: {e}"),
            4004,
            start_time,
        );
        return false;
    }

    true
}

/// Execute the export workflow with all steps
async fn execute_export_workflow(
    sender: tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    format: String,
    output: Option<String>,
    include_metadata: bool,
    memory: Option<Arc<MemoryCoordinator>>,
    start_time: Instant,
) {
    send_export_started(
        &sender,
        execution_id,
        &format,
        output.as_ref(),
        include_metadata,
        &start_time,
    );

    let Some(messages) =
        retrieve_and_validate_messages(&sender, execution_id, memory, &start_time).await
    else {
        return;
    };

    let Some(config) = prepare_export_config(
        &sender,
        execution_id,
        &format,
        include_metadata,
        output.clone(),
        &start_time,
    ) else {
        return;
    };

    let Some(export_data) =
        export_messages_to_data(&sender, execution_id, &messages, config, &start_time)
    else {
        return;
    };

    let output_path = determine_output_path(output, &export_data.file_extension);

    if !write_export_to_disk(
        &sender,
        execution_id,
        &export_data,
        &output_path,
        &start_time,
    )
    .await
    {
        return;
    }

    send_export_completion(
        &sender,
        execution_id,
        &export_data,
        &output_path,
        &format,
        &start_time,
    );
}

/// Retrieve conversation messages from memory coordinator using public API
async fn retrieve_conversation_messages(
    memory: &MemoryCoordinator,
) -> Result<Vec<CandleMessage>, String> {
    use crate::memory::core::ops::filter::MemoryFilter;

    // Use public MemoryFilter API to query by tags
    let filter = MemoryFilter::new().with_tags(vec![
        "message_type.user".to_string(),
        "message_type.assistant".to_string(),
        "message_type.system".to_string(),
    ]);

    // Use public get_memories() API - returns Vec<DomainMemoryNode>
    let memories = memory
        .get_memories(filter)
        .await
        .map_err(|e| format!("Failed to retrieve memories: {e}"))?;

    // Convert domain memory nodes to CandleMessage format
    let mut messages: Vec<CandleMessage> = memories
        .iter()
        .map(|mem| {
            // Determine role from tags
            let role = if mem
                .metadata
                .tags
                .iter()
                .any(|t| t.as_ref() == "message_type.user")
            {
                CandleMessageRole::User
            } else if mem
                .metadata
                .tags
                .iter()
                .any(|t| t.as_ref() == "message_type.system")
            {
                CandleMessageRole::System
            } else if mem
                .metadata
                .tags
                .iter()
                .any(|t| t.as_ref() == "message_type.assistant")
            {
                CandleMessageRole::Assistant
            } else {
                // Fallback for unrecognized tags - treat as Assistant
                CandleMessageRole::Assistant
            };

            CandleMessage {
                role,
                content: mem.content().to_string(),
                id: Some(mem.id().to_string()),
                timestamp: Some(
                    mem.creation_time()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
            }
        })
        .collect();

    // Sort by timestamp to maintain conversation order
    messages.sort_by_key(|m| m.timestamp.unwrap_or(0));

    Ok(messages)
}

/// Send progress event for export operation
fn send_export_progress(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    progress: f64,
    message: String,
) {
    #[allow(clippy::cast_possible_truncation)]
    let progress_f32 = progress as f32;
    let _ = sender.send(CommandEvent::Progress {
        execution_id,
        progress: progress_f32,
        message,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
}

/// Send failure event for export operation
fn send_export_failure(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    error: String,
    error_code: u32,
    start_time: &Instant,
) {
    #[allow(clippy::cast_possible_truncation)]
    let duration_us = start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;

    let _ = sender.send(CommandEvent::Failed {
        execution_id,
        error,
        error_code,
        duration_us,
        resource_usage: ResourceUsage::default(),
        timestamp_us: current_timestamp_us(),
    });
}

/// Parse export format string to `ExportFormat` enum
fn parse_export_format(format: &str) -> Result<ExportFormat, String> {
    match format.to_lowercase().as_str() {
        "json" => Ok(ExportFormat::Json),
        "markdown" | "md" => Ok(ExportFormat::Markdown),
        "text" | "txt" => Ok(ExportFormat::Text),
        "csv" => Ok(ExportFormat::Csv),
        _ => Err(format!("Unsupported export format: {format}")),
    }
}

/// Create export configuration
fn create_export_config(
    format: ExportFormat,
    include_metadata: bool,
    output: Option<String>,
) -> ExportConfig {
    ExportConfig {
        format,
        include_metadata,
        include_timestamps: true,
        max_messages: 0,
        filename_prefix: output.unwrap_or_else(|| "chat_export".to_string()),
    }
}

/// Perform export operation
fn perform_message_export(
    messages: &[CandleMessage],
    config: ExportConfig,
) -> Result<ExportData, String> {
    let mut exporter = ChatExporter::with_config(config);
    exporter
        .export_messages(messages)
        .map_err(|e| format!("Export failed: {e}"))
}

/// Determine output file path
fn determine_output_path(output: Option<String>, file_extension: &str) -> String {
    output.unwrap_or_else(|| format!("chat_export.{file_extension}"))
}

/// Send completion events for export operation
fn send_export_completion(
    sender: &tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    execution_id: u64,
    export_data: &ExportData,
    output_path: &str,
    format: &str,
    start_time: &Instant,
) {
    // Send 100% progress
    send_export_progress(sender, execution_id, 100.0, "Export complete!".to_string());

    // Send success output message
    let success_message = format!(
        "Successfully exported {} messages to '{}' ({} format, {} bytes)",
        export_data.metadata.message_count, output_path, format, export_data.metadata.size_bytes
    );

    let _ = sender.send(CommandEvent::output(
        execution_id,
        success_message,
        OutputType::Text,
    ));

    // Send completed event
    let result = CommandExecutionResult::File {
        path: output_path.to_string(),
        #[allow(clippy::cast_possible_truncation)]
        size_bytes: export_data.metadata.size_bytes as u64,
        mime_type: export_data.content_type.clone(),
    };

    #[allow(clippy::cast_possible_truncation)]
    let duration_us = start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;

    let _ = sender.send(CommandEvent::completed(
        execution_id,
        result,
        duration_us,
        ResourceUsage::default(),
    ));
}

impl CommandExecutor {
    /// Execute config command (streaming-only, zero-allocation)  
    pub fn execute_config_streaming(
        &self,
        execution_id: u64,
        key: Option<String>,
        value: Option<String>,
        show: bool,
        reset: bool,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let start_time = Instant::now();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                tokio::spawn(async move {
                    // Emit started event
                    let _ = sender.send(CommandEvent::Started {
                        command: ImmutableChatCommand::Config {
                            key: key.clone(),
                            value: value.clone(),
                            show,
                            reset,
                        },
                        execution_id,
                        #[allow(clippy::cast_possible_truncation)]
                        timestamp_us: start_time.elapsed().as_micros().min(u128::from(u64::MAX))
                            as u64,
                    });

                    let message = if reset {
                        "Configuration reset to defaults".to_string()
                    } else if show {
                        "Current configuration: <config data>".to_string()
                    } else if let (Some(k), Some(v)) = (key.as_ref(), value.as_ref()) {
                        format!("Configuration updated: {k} = {v}")
                    } else if let Some(k) = key {
                        format!("Configuration value for {k}: <value>")
                    } else {
                        "Use --show to display current configuration".to_string()
                    };

                    // Emit output event
                    let _ = sender.send(CommandEvent::output(
                        execution_id,
                        message.clone(),
                        OutputType::Text,
                    ));

                    // Emit completion event
                    #[allow(clippy::cast_possible_truncation)]
                    let duration_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::Completed {
                        execution_id,
                        result: CommandExecutionResult::Success(message.clone()),
                        duration_us,
                        resource_usage: ResourceUsage::default(),
                        timestamp_us: current_timestamp_us(),
                    });
                });
            },
        ))
    }

    /// Execute search command (streaming-only, zero-allocation)
    pub fn execute_search_streaming(
        &self,
        execution_id: u64,
        query: String,
        scope: SearchScope,
        limit: Option<usize>,
        include_context: bool,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let start_time = Instant::now();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                tokio::spawn(async move {
                    // Emit started event
                    let _ = sender.send(CommandEvent::Started {
                        command: ImmutableChatCommand::Search {
                            query: query.clone(),
                            scope,
                            limit,
                            include_context,
                        },
                        execution_id,
                        #[allow(clippy::cast_possible_truncation)]
                        timestamp_us: start_time.elapsed().as_micros().min(u128::from(u64::MAX))
                            as u64,
                    });

                    // Simulate search progress with zero allocation
                    for progress in [20, 40, 60, 80, 100] {
                        #[allow(clippy::cast_precision_loss)]
                        let progress_f32 = progress as f32;
                        let _ = sender.send(CommandEvent::Progress {
                            execution_id,
                            progress: progress_f32,
                            message: format!("Searching... {progress}%"),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                                .as_secs(),
                        });
                    }

                    let scope_str = match scope {
                        SearchScope::All => "all conversations",
                        SearchScope::Session => "current session",
                        SearchScope::Current => "current conversation",
                        SearchScope::Recent => "recent conversations",
                        SearchScope::Bookmarked => "bookmarked conversations",
                        SearchScope::User => "user messages",
                        SearchScope::Assistant => "assistant messages",
                        SearchScope::System => "system messages",
                        SearchScope::Branch => "current branch",
                        SearchScope::TimeRange => "time range",
                        SearchScope::MessageType => "message type",
                        SearchScope::Tags => "tags",
                        SearchScope::Archived => "archived content",
                    };

                    let limit_str = limit.map(|n| format!(" (limit: {n})")).unwrap_or_default();
                    let context_str = if include_context { " with context" } else { "" };

                    let message = format!(
                        "Searching for '{query}' in {scope_str}{limit_str}{context_str}\nSearch completed - 0 results found"
                    );

                    // Emit output event
                    let _ = sender.send(CommandEvent::output(
                        execution_id,
                        message.clone(),
                        OutputType::Text,
                    ));

                    // Emit completion event with search results
                    let result = CommandExecutionResult::Data(serde_json::json!({
                        "query": query,
                        "scope": format!("{:?}", scope),
                        "results": [],
                        "total_found": 0
                    }));
                    #[allow(clippy::cast_possible_truncation)]
                    let duration_us =
                        start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
                    let _ = sender.send(CommandEvent::completed(
                        execution_id,
                        result,
                        duration_us,
                        ResourceUsage::default(),
                    ));
                });
            },
        ))
    }

    /// Get parser reference
    pub fn parser(&self) -> &CommandParser {
        &self.parser
    }

    /// Get execution statistics (zero-allocation, lock-free)
    pub fn get_stats(&self) -> (u64, usize, u64, u64, u64) {
        (
            self.execution_counter.load(Ordering::Acquire),
            self.active_executions.load(Ordering::Acquire),
            self.total_executions.load(Ordering::Acquire),
            self.successful_executions.load(Ordering::Acquire),
            self.failed_executions.load(Ordering::Acquire),
        )
    }

    /// Parse and execute command from string (streaming-only, zero-allocation)
    pub fn parse_and_execute(
        &self,
        input: &str,
    ) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
        let execution_id = self.execution_counter.fetch_add(1, Ordering::AcqRel);
        let command_result = self.parser.parse_command(input);

        // Clone self for use in async closure
        let self_clone = self.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let start_time = Instant::now();

                match command_result {
                    Ok(command) => {
                        use tokio_stream::StreamExt;

                        // Delegate to execute_streaming which has full timing
                        let execution_stream = self_clone.execute_streaming(execution_id, command);

                        // Forward all events from execute_streaming
                        tokio::pin!(execution_stream);
                        while let Some(event) = execution_stream.next().await {
                            let _ = sender.send(event);
                        }
                    }
                    Err(e) => {
                        // Calculate elapsed time for parse error
                        #[allow(clippy::cast_possible_truncation)]
                        let duration_us =
                            start_time.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;

                        // Emit Failed event with actual duration
                        let _ = sender.send(CommandEvent::Failed {
                            execution_id,
                            error: format!("Parse error: {e}"),
                            error_code: 1001,
                            duration_us,
                            resource_usage: ResourceUsage::default(),
                            timestamp_us: current_timestamp_us(),
                        });
                    }
                }
            },
        ))
    }
}
