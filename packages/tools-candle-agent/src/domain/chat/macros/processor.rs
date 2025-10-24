//\! `MacroProcessor` implementation with advanced features

use super::context::{ChatMacro, LoopContext, MacroExecutionContext, send_message_to_conversation};
use super::errors::{ActionExecutionResult, MacroSystemError};
use super::parser::{CondParser, tokenize_condition};
use super::types::MacroAction;
use crate::domain::chat::commands::{CommandEvent, ImmutableChatCommand, execute_candle_command};
use crate::domain::chat::conversation::CandleStreamingConversation;
use crossbeam_skiplist::SkipMap;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use uuid::Uuid;

/// Macro processor for executing and managing chat macros
///
/// This processor provides comprehensive macro execution capabilities with:
/// - Recording and playback of chat interactions
/// - Variable substitution and conditional logic
/// - Performance monitoring and error handling
/// - Concurrent execution with lock-free data structures
/// - Macro validation and optimization
#[derive(Debug, Clone)]
pub struct MacroProcessor {
    /// Macro storage with lock-free access
    macros: Arc<SkipMap<Uuid, ChatMacro>>,
    /// Execution statistics
    stats: Arc<MacroProcessorStats>,
    /// Variable context for macro execution
    variables: Arc<RwLock<HashMap<String, String>>>,
    /// Configuration settings
    config: MacroProcessorConfig,
}

/// Macro processor statistics (internal atomic counters)
#[derive(Debug, Default)]
pub struct MacroProcessorStats {
    /// Total macros executed
    pub total_executions: AtomicUsize,
    /// Successful executions
    pub successful_executions: AtomicUsize,
    /// Failed executions
    pub failed_executions: AtomicUsize,
    /// Total execution time in microseconds
    pub total_execution_time_us: AtomicUsize,
    /// Active executions
    pub active_executions: AtomicUsize,
}

/// Macro processor statistics snapshot (for external API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroProcessorStatsSnapshot {
    /// Total macros executed
    pub total_executions: usize,
    /// Successful executions
    pub successful_executions: usize,
    /// Failed executions
    pub failed_executions: usize,
    /// Total execution time in microseconds
    pub total_execution_time_us: usize,
    /// Active executions
    pub active_executions: usize,
}

bitflags::bitflags! {
    /// Macro processor feature flags for zero-allocation enable/disable checks
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct MacroFeatureFlags: u8 {
        /// Enable variable substitution in macros
        const VARIABLE_SUBSTITUTION = 1 << 0;
        /// Enable conditional execution (if/else)
        const CONDITIONAL_EXECUTION = 1 << 1;
        /// Enable loop execution (for/while)
        const LOOP_EXECUTION = 1 << 2;
        /// Enable performance monitoring
        const MONITORING = 1 << 3;
        /// Auto-save macro changes
        const AUTO_SAVE = 1 << 4;
        /// All macro features enabled
        const ALL = Self::VARIABLE_SUBSTITUTION.bits() | Self::CONDITIONAL_EXECUTION.bits() | Self::LOOP_EXECUTION.bits() | Self::MONITORING.bits() | Self::AUTO_SAVE.bits();
    }
}

/// Macro processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroProcessorConfig {
    /// Maximum concurrent executions
    pub max_concurrent_executions: usize,
    /// Default execution timeout in seconds
    pub default_timeout_seconds: u64,
    /// Feature flags
    pub flags: MacroFeatureFlags,
    /// Maximum recursion depth for nested macros
    pub max_recursion_depth: usize,
    /// Enable performance metrics collection
    pub enable_metrics: bool,
}

impl Default for MacroProcessorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            default_timeout_seconds: 300,
            flags: MacroFeatureFlags::ALL,
            max_recursion_depth: 10,
            enable_metrics: true,
        }
    }
}

/// Result of a macro execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExecutionResult {
    /// Execution ID
    pub execution_id: Uuid,
    /// Macro ID that was executed
    pub macro_id: Uuid,
    /// Whether execution succeeded
    pub success: bool,
    /// Number of actions executed
    pub actions_executed: usize,
    /// Execution metadata
    pub metadata: MacroExecutionMetadata,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Metadata about macro execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExecutionMetadata {
    /// When execution started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When execution completed
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Duration of execution
    pub duration_ms: u64,
    /// Variables at execution start
    pub initial_variables: HashMap<String, String>,
    /// Variables at execution end
    pub final_variables: HashMap<String, String>,
    /// Performance metrics
    pub performance: MacroPerformanceMetrics,
}

/// Performance metrics for macro execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacroPerformanceMetrics {
    /// Number of messages sent
    pub messages_sent: usize,
    /// Number of commands executed
    pub commands_executed: usize,
    /// Number of variables set
    pub variables_set: usize,
    /// Number of conditionals evaluated
    pub conditionals_evaluated: usize,
    /// Number of loops executed
    pub loops_executed: usize,
    /// Total wait time in milliseconds
    pub total_wait_ms: u64,
    /// Number of disk operations
    pub disk_operations: usize,
}

impl MacroProcessor {
    /// Create a new macro processor with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            macros: Arc::new(SkipMap::new()),
            stats: Arc::new(MacroProcessorStats::default()),
            variables: Arc::new(RwLock::new(HashMap::new())),
            config: MacroProcessorConfig::default(),
        }
    }

    /// Create a new macro processor with custom configuration
    #[must_use]
    pub fn with_config(config: MacroProcessorConfig) -> Self {
        Self {
            macros: Arc::new(SkipMap::new()),
            stats: Arc::new(MacroProcessorStats::default()),
            variables: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a new macro
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if macro validation fails
    pub fn register_macro(&self, macro_def: ChatMacro) -> Result<(), MacroSystemError> {
        self.validate_macro(&macro_def)?;
        self.macros.insert(macro_def.metadata.id, macro_def);
        Ok(())
    }

    /// Unregister a macro by ID
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if macro with the given ID does not exist
    pub fn unregister_macro(&self, macro_id: &Uuid) -> Result<(), MacroSystemError> {
        if self.macros.remove(macro_id).is_some() {
            Ok(())
        } else {
            Err(MacroSystemError::MacroNotFound)
        }
    }

    /// Get a macro by ID
    #[must_use]
    pub fn get_macro(&self, macro_id: &Uuid) -> Option<ChatMacro> {
        self.macros.get(macro_id).map(|entry| entry.value().clone())
    }

    /// List all registered macros
    #[must_use]
    pub fn list_macros(&self) -> Vec<ChatMacro> {
        self.macros
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Execute a macro with given context variables
    ///
    /// # Errors
    ///
    /// Returns error stream if macro not found or execution fails
    #[must_use]
    pub fn execute_macro(
        &self,
        macro_id: &Uuid,
        context_variables: HashMap<String, String>,
        conversation: Option<Arc<RwLock<CandleStreamingConversation>>>,
    ) -> Pin<Box<dyn tokio_stream::Stream<Item = MacroExecutionResult> + Send>> {
        let macro_def = if let Some(entry) = self.macros.get(macro_id) {
            entry.value().clone()
        } else {
            // Macro not found - return error stream
            let error_result = MacroExecutionResult {
                execution_id: Uuid::new_v4(),
                macro_id: *macro_id,
                success: false,
                actions_executed: 0,
                metadata: MacroExecutionMetadata {
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                    duration_ms: 0,
                    initial_variables: HashMap::new(),
                    final_variables: HashMap::new(),
                    performance: MacroPerformanceMetrics::default(),
                },
                error: Some("Macro not found".to_string()),
            };
            return Box::pin(tokio_stream::once(error_result));
        };

        // Create execution context
        let execution_id = Uuid::new_v4();
        let started_at = chrono::Utc::now();

        let stats = Arc::clone(&self.stats);
        stats.total_executions.fetch_add(1, Ordering::Relaxed);
        stats.active_executions.fetch_add(1, Ordering::Relaxed);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut context = MacroExecutionContext {
                variables: context_variables.clone(),
                execution_id,
                start_time: started_at,
                current_action: 0,
                loop_stack: Vec::new(),
                conversation,
            };

            let mut performance = MacroPerformanceMetrics::default();
            let (success, actions_executed, error_message) =
                Self::execute_macro_impl(&macro_def, &mut context, &mut performance).await;

            let completed_at = chrono::Utc::now();
            let duration_ms = (completed_at - started_at)
                .num_milliseconds()
                .cast_unsigned();

            if success {
                stats.successful_executions.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.failed_executions.fetch_add(1, Ordering::Relaxed);
            }
            stats.total_execution_time_us.fetch_add(
                usize::try_from(duration_ms * 1000).unwrap_or(usize::MAX),
                Ordering::Relaxed,
            );
            stats.active_executions.fetch_sub(1, Ordering::Relaxed);

            let result = MacroExecutionResult {
                execution_id,
                macro_id: macro_def.metadata.id,
                success,
                actions_executed,
                metadata: MacroExecutionMetadata {
                    started_at,
                    completed_at: Some(completed_at),
                    duration_ms,
                    initial_variables: context_variables,
                    final_variables: context.variables.clone(),
                    performance,
                },
                error: error_message,
            };

            let _ = tx.send(result);
        });

        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    /// Internal implementation of macro execution
    async fn execute_macro_impl(
        macro_def: &ChatMacro,
        context: &mut MacroExecutionContext,
        performance: &mut MacroPerformanceMetrics,
    ) -> (bool, usize, Option<String>) {
        let mut actions_executed = 0;
        let mut error_message = None;

        let success = loop {
            if context.current_action >= macro_def.actions.len() {
                break true;
            }

            let action = &macro_def.actions[context.current_action];
            context.current_action += 1;
            actions_executed += 1;

            match execute_action_sync(action, context).await {
                Ok(ActionExecutionResult::Success) => {
                    // Track performance metrics
                    match action {
                        MacroAction::SendMessage { .. } => performance.messages_sent += 1,
                        MacroAction::ExecuteCommand { .. } => performance.commands_executed += 1,
                        MacroAction::SetVariable { .. } => performance.variables_set += 1,
                        MacroAction::Conditional { .. } => performance.conditionals_evaluated += 1,
                        MacroAction::Loop { .. } => performance.loops_executed += 1,
                        MacroAction::Wait { duration, .. } => {
                            performance.total_wait_ms +=
                                u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                        }
                    }
                }
                Ok(ActionExecutionResult::Wait(duration)) => {
                    tokio::time::sleep(duration).await;
                    performance.total_wait_ms +=
                        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                }
                Ok(ActionExecutionResult::SkipToAction(index)) => {
                    context.current_action = index;
                }
                Ok(ActionExecutionResult::Error(error)) => {
                    error_message = Some(format!(
                        "Action {} failed: {}",
                        context.current_action, error
                    ));
                    if let Some(ref msg) = error_message {
                        error!("Macro execution error: {msg}");
                    }
                    break false;
                }
                Err(e) => {
                    error_message = Some(format!(
                        "System error at action {}: {}",
                        context.current_action, e
                    ));
                    if let Some(ref msg) = error_message {
                        error!("Macro system error: {msg}");
                    }
                    break false;
                }
            }

            performance.disk_operations += 1;
        };

        (success, actions_executed, error_message)
    }

    /// Validate a macro
    fn validate_macro(&self, macro_def: &ChatMacro) -> Result<(), MacroSystemError> {
        if macro_def.actions.is_empty() {
            return Err(MacroSystemError::ValidationError(
                "Macro has no actions".to_string(),
            ));
        }

        if macro_def.metadata.name.is_empty() {
            return Err(MacroSystemError::ValidationError(
                "Macro name is required".to_string(),
            ));
        }

        // Additional validation - recursion depth, etc.
        let mut depth = 0;
        for action in &macro_def.actions {
            if let MacroAction::Loop { actions: _, .. } = action {
                depth += 1;
                if depth > self.config.max_recursion_depth {
                    return Err(MacroSystemError::ValidationError(
                        "Maximum recursion depth exceeded".to_string(),
                    ));
                }
                // Recursive validation
            }
        }

        Ok(())
    }

    /// Get macro processor statistics snapshot
    #[must_use]
    pub fn get_stats_snapshot(&self) -> MacroProcessorStatsSnapshot {
        MacroProcessorStatsSnapshot {
            total_executions: self.stats.total_executions.load(Ordering::Relaxed),
            successful_executions: self.stats.successful_executions.load(Ordering::Relaxed),
            failed_executions: self.stats.failed_executions.load(Ordering::Relaxed),
            total_execution_time_us: self.stats.total_execution_time_us.load(Ordering::Relaxed),
            active_executions: self.stats.active_executions.load(Ordering::Relaxed),
        }
    }

    /// Get active executions count
    #[must_use]
    pub fn get_active_executions(&self) -> usize {
        self.stats.active_executions.load(Ordering::Relaxed)
    }

    /// Set a global variable that persists across macro executions
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if lock on variables cannot be acquired
    pub async fn set_global_variable(
        &self,
        name: String,
        value: String,
    ) -> Result<(), MacroSystemError> {
        let mut vars = self.variables.write().await;
        vars.insert(name, value);
        Ok(())
    }

    /// Get a global variable value by name
    #[must_use]
    pub async fn get_global_variable(&self, name: &str) -> Option<String> {
        let vars = self.variables.read().await;
        vars.get(name).cloned()
    }

    /// Get all global variables as a snapshot
    #[must_use]
    pub async fn get_global_variables_snapshot(&self) -> HashMap<String, String> {
        let vars = self.variables.read().await;
        vars.clone()
    }

    /// Clear all global variables
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if lock acquisition fails
    pub async fn clear_global_variables(&self) -> Result<(), MacroSystemError> {
        let mut vars = self.variables.write().await;
        vars.clear();
        Ok(())
    }
}

impl Default for MacroProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro action execution
pub(crate) fn execute_action_sync<'a>(
    action: &'a MacroAction,
    context: &'a mut MacroExecutionContext,
) -> Pin<Box<dyn Future<Output = Result<ActionExecutionResult, MacroSystemError>> + Send + 'a>> {
    Box::pin(async move {
        match action {
            MacroAction::SendMessage {
                content,
                message_type,
                ..
            } => execute_send_message(content, message_type, context).await,
            MacroAction::ExecuteCommand { command, .. } => execute_command_action(command).await,
            MacroAction::Wait { duration, .. } => Ok(ActionExecutionResult::Wait(*duration)),
            MacroAction::SetVariable { name, value, .. } => {
                Ok(execute_set_variable(name, value, context))
            }
            MacroAction::Conditional {
                condition,
                then_actions,
                else_actions,
                ..
            } => execute_conditional(condition, then_actions, else_actions.as_ref(), context).await,
            MacroAction::Loop {
                iterations,
                actions,
                ..
            } => execute_loop(*iterations, actions, context).await,
        }
    })
}

/// Execute send message action
async fn execute_send_message(
    msg_content: &str,
    message_type: &str,
    exec_context: &MacroExecutionContext,
) -> Result<ActionExecutionResult, MacroSystemError> {
    let resolved_content = resolve_variables_sync(msg_content, &exec_context.variables);

    if let Some(ref conversation) = exec_context.conversation {
        match send_message_to_conversation(conversation, resolved_content.clone(), message_type)
            .await
        {
            Ok(()) => Ok(ActionExecutionResult::Success),
            Err(e) => Ok(ActionExecutionResult::Error(format!(
                "Message send failed: {e}"
            ))),
        }
    } else {
        warn!("No conversation available for SendMessage action. Message: {resolved_content}");
        Ok(ActionExecutionResult::Success)
    }
}

/// Execute command action
async fn execute_command_action(
    command: &ImmutableChatCommand,
) -> Result<ActionExecutionResult, MacroSystemError> {
    let mut event_stream = execute_candle_command(command.clone());
    let mut command_output = String::new();
    let mut result = ActionExecutionResult::Success;

    while let Some(event) = event_stream.next().await {
        match event {
            CommandEvent::Output { content, .. } => {
                command_output.push_str(&content);
            }
            CommandEvent::Completed { .. } => {
                result = ActionExecutionResult::Success;
            }
            CommandEvent::Failed { error, .. } => {
                result = ActionExecutionResult::Error(format!("Command execution failed: {error}"));
                break;
            }
            CommandEvent::Cancelled { reason, .. } => {
                result = ActionExecutionResult::Error(format!("Command cancelled: {reason}"));
                break;
            }
            _ => {}
        }
    }

    if !command_output.is_empty() && matches!(result, ActionExecutionResult::Success) {
        log::debug!("Command output: {command_output}");
    }

    Ok(result)
}

/// Execute set variable action
fn execute_set_variable(
    name: &str,
    value: &str,
    context: &mut MacroExecutionContext,
) -> ActionExecutionResult {
    let resolved_value = resolve_variables_sync(value, &context.variables);
    context.variables.insert(name.to_string(), resolved_value);
    ActionExecutionResult::Success
}

/// Execute conditional action
async fn execute_conditional<'a>(
    condition: &str,
    then_actions: &'a [MacroAction],
    else_actions: Option<&'a Vec<MacroAction>>,
    context: &'a mut MacroExecutionContext,
) -> Result<ActionExecutionResult, MacroSystemError> {
    let condition_result = evaluate_condition_sync(condition, &context.variables);

    let actions_to_execute = if condition_result {
        then_actions
    } else if let Some(else_actions) = else_actions {
        else_actions.as_slice()
    } else {
        return Ok(ActionExecutionResult::Success);
    };

    for action in actions_to_execute {
        execute_action_sync(action, context).await?;
    }

    Ok(ActionExecutionResult::Success)
}

/// Execute loop action
async fn execute_loop<'a>(
    iterations: u32,
    actions: &'a [MacroAction],
    context: &'a mut MacroExecutionContext,
) -> Result<ActionExecutionResult, MacroSystemError> {
    let loop_context = LoopContext {
        iteration: 0,
        max_iterations: iterations,
        start_action: context.current_action,
        end_action: context.current_action + actions.len() - 1,
    };

    context.loop_stack.push(loop_context);

    for _ in 0..iterations {
        for action in actions {
            if let Err(e) = execute_action_sync(action, context).await {
                context.loop_stack.pop();
                return Err(e);
            }
        }
    }

    context.loop_stack.pop();
    Ok(ActionExecutionResult::Success)
}

/// Synchronous variable resolution
fn resolve_variables_sync(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    for (key, value) in variables {
        let placeholder = format!("{{{key}}}");
        result = result.replace(&placeholder, value);
    }

    result
}

/// Synchronous condition evaluation with full expression support
fn evaluate_condition_sync(condition: &str, variables: &HashMap<String, String>) -> bool {
    // Handle empty condition
    if condition.trim().is_empty() {
        return false;
    }

    // Resolve variables in condition
    let resolved = resolve_variables_sync(condition, variables);

    // Tokenize the resolved condition
    let tokens = tokenize_condition(&resolved);

    // Handle empty token stream
    if tokens.is_empty() {
        return false;
    }

    // Parse and evaluate expression
    let mut parser = CondParser::new(tokens);
    let result = parser.parse_or();

    // Convert result to boolean
    result.as_bool()
}
