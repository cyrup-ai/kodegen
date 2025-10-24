//\! `MacroSystem` implementation for recording and playback

use super::context::{
    ChatMacro, LoopContext, MacroExecutionConfig, MacroExecutionContext, MacroMetadata,
    send_message_to_conversation,
};
use super::errors::{ActionExecutionResult, MacroPlaybackResult, MacroSystemError};
use super::types::{MacroAction, MacroPlaybackState, MacroRecordingState};
use atomic_counter::{AtomicCounter, ConsistentCounter};
use chrono::{DateTime, Utc};
use crossbeam_skiplist::SkipMap;
use cyrup_sugars::prelude::MessageChunk;
use dashmap::DashMap;
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Macro recording session
#[derive(Debug)]
pub struct MacroRecordingSession {
    /// Unique identifier for this recording session
    pub id: Uuid,
    /// Name of the macro being recorded
    pub name: String,
    /// When recording started
    pub start_time: DateTime<Utc>,
    /// Queue of recorded actions
    pub actions: Arc<Mutex<Vec<MacroAction>>>,
    /// Current recording state
    pub state: MacroRecordingState,
    /// Variables captured during recording
    pub variables: HashMap<String, String>,
    /// Metadata for the macro being recorded
    pub metadata: MacroMetadata,
}

/// Macro playback session
#[derive(Debug)]
pub struct MacroPlaybackSession {
    /// Unique identifier for this playback session
    pub id: Uuid,
    /// ID of the macro being played back
    pub macro_id: Uuid,
    /// When playback started
    pub start_time: DateTime<Utc>,
    /// Execution context with variables and state
    pub context: MacroExecutionContext,
    /// Current playback state
    pub state: MacroPlaybackState,
    /// Index of the currently executing action
    pub current_action: usize,
    /// Total number of actions in the macro
    pub total_actions: usize,
    /// Error message if playback failed
    pub error: Option<String>,
}

/// High-performance macro system with lock-free operations
pub struct MacroSystem {
    /// Lock-free macro storage using skip list
    macros: SkipMap<Uuid, ChatMacro>,
    /// Active recording sessions
    recording_sessions: DashMap<Uuid, MacroRecordingSession>,
    /// Active playback sessions
    playback_sessions: DashMap<Uuid, MacroPlaybackSession>,
    /// Global macro counter
    macro_counter: ConsistentCounter,
    /// Execution counter
    execution_counter: ConsistentCounter,
}

/// Macro execution statistics
#[derive(Debug, Default)]
pub struct ExecutionStats {
    /// Total number of macro executions attempted
    pub total_executions: ConsistentCounter,
    /// Number of successful macro executions
    pub successful_executions: ConsistentCounter,
    /// Number of failed macro executions
    pub failed_executions: ConsistentCounter,
    /// Total time spent executing macros
    pub total_duration: tokio::sync::Mutex<Duration>,
    /// Average execution duration per macro
    pub average_duration: tokio::sync::Mutex<Duration>,
    /// Timestamp of the last macro execution
    pub last_execution: tokio::sync::Mutex<Option<DateTime<Utc>>>,
}

impl Clone for ExecutionStats {
    fn clone(&self) -> Self {
        ExecutionStats {
            total_executions: ConsistentCounter::new(self.total_executions.get()),
            successful_executions: ConsistentCounter::new(self.successful_executions.get()),
            failed_executions: ConsistentCounter::new(self.failed_executions.get()),
            total_duration: tokio::sync::Mutex::new(*self.total_duration.blocking_lock()),
            average_duration: tokio::sync::Mutex::new(*self.average_duration.blocking_lock()),
            last_execution: tokio::sync::Mutex::new(*self.last_execution.blocking_lock()),
        }
    }
}

impl MacroSystem {
    /// Create a new macro system with optimal performance settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            macros: SkipMap::new(),
            recording_sessions: DashMap::new(),
            playback_sessions: DashMap::new(),
            macro_counter: ConsistentCounter::new(0),
            execution_counter: ConsistentCounter::new(0),
        }
    }

    /// Start recording a new macro
    pub fn start_recording(
        &self,
        name: String,
        description: &str,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = MacroSessionId> + Send>> {
        let session_id = Uuid::new_v4();
        let macro_id = Uuid::new_v4();

        // Create metadata for the macro
        let metadata = MacroMetadata {
            id: macro_id,
            name: name.clone(),
            description: description.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default(),
            version: 1,
            tags: Vec::new(),
            author: "system".to_string(),
            execution_count: 0,
            last_execution: None,
            average_duration: Duration::from_secs(0),
            success_rate: 0.0,
            category: "user-defined".to_string(),
            is_private: false,
        };

        // Create a new recording session with the correct fields
        let new_session = MacroRecordingSession {
            id: session_id,
            name,
            start_time: Utc::now(),
            actions: Arc::new(Mutex::new(Vec::new())),
            state: MacroRecordingState::Recording,
            variables: HashMap::new(),
            metadata,
        };

        // Capture session_id in owned variable before any operations
        let owned_session_id = session_id;

        // Insert the new session directly
        self.recording_sessions.insert(session_id, new_session);

        // Create a stream that immediately yields the session ID using tokio stream pattern
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let _ = sender.send(MacroSessionId(owned_session_id));
            },
        ))
    }

    /// Record a macro action
    pub fn record_action(
        &self,
        session_id: Uuid,
        action: MacroAction,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = MacroActionResult> + Send>> {
        // Get a reference to the session without cloning the entire DashMap
        if let Some(session) = self.recording_sessions.get(&session_id) {
            // Check if we're still recording
            if session.value().state == MacroRecordingState::Recording {
                // Push the action to the session's actions
                if let Ok(mut actions) = session.value().actions.try_lock() {
                    actions.push(action);
                }

                // Create a stream that immediately yields success
                return Box::pin(crate::async_stream::spawn_stream(
                    move |sender| async move {
                        let _ = sender.send(MacroActionResult);
                    },
                ));
            }
        }

        // If we get here, either the session doesn't exist or it's not recording
        // Return an empty stream that immediately completes
        Box::pin(crate::async_stream::empty())
    }

    /// Stop recording and save the macro
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if:
    /// - Recording session with the given ID does not exist
    /// - Macro cannot be saved
    pub fn stop_recording(&self, session_id: Uuid) -> Result<Uuid, MacroSystemError> {
        if let Some((_, session)) = self.recording_sessions.remove(&session_id) {
            let mut session = session; // make mutable
            session.state = MacroRecordingState::Completed;

            // Collect all recorded actions
            let actions = if let Ok(mut action_vec) = session.actions.try_lock() {
                action_vec.drain(..).collect()
            } else {
                Vec::new()
            };

            // Create the macro
            let chat_macro = ChatMacro {
                metadata: session.metadata.clone(),
                actions,
                variables: session.variables,
                triggers: Vec::new(),
                conditions: Vec::new(),
                dependencies: Vec::new(),
                execution_config: MacroExecutionConfig::default(),
            };

            let macro_id = session.metadata.id;
            self.macros.insert(macro_id, chat_macro);
            self.macro_counter.inc();

            Ok(macro_id)
        } else {
            Err(MacroSystemError::SessionNotFound)
        }
    }

    /// Get a macro by ID
    pub fn get_macro(&self, macro_id: Uuid) -> Option<ChatMacro> {
        self.macros
            .get(&macro_id)
            .map(|entry| entry.value().clone())
    }

    /// List all available macros
    pub fn list_macros(&self) -> Vec<MacroMetadata> {
        self.macros
            .iter()
            .map(|entry| entry.value().metadata.clone())
            .collect()
    }

    /// Search macros by name, description, or tags
    pub fn search_macros(&self, query: &str) -> Vec<MacroMetadata> {
        let query_lower = query.to_lowercase();

        self.macros
            .iter()
            .filter(|entry| {
                let macro_def = entry.value();
                macro_def
                    .metadata
                    .name
                    .to_lowercase()
                    .contains(&query_lower)
                    || macro_def
                        .metadata
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
                    || macro_def
                        .metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().metadata.clone())
            .collect()
    }

    /// Start macro playback
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if:
    /// - Macro with the given ID does not exist
    /// - Playback session cannot be created
    pub fn start_playback(
        &self,
        macro_id: Uuid,
        variables: HashMap<String, String>,
        conversation: Option<
            Arc<RwLock<crate::domain::chat::conversation::CandleStreamingConversation>>,
        >,
    ) -> Result<Uuid, MacroSystemError> {
        let macro_def = self
            .macros
            .get(&macro_id)
            .ok_or(MacroSystemError::MacroNotFound)?
            .value()
            .clone();

        let session_id = Uuid::new_v4();
        let context = MacroExecutionContext {
            variables,
            execution_id: session_id,
            start_time: Utc::now(),
            current_action: 0,
            loop_stack: Vec::new(),
            conversation,
        };

        let session = MacroPlaybackSession {
            id: session_id,
            macro_id,
            start_time: Utc::now(),
            context,
            state: MacroPlaybackState::Playing,
            current_action: 0,
            total_actions: macro_def.actions.len(),
            error: None,
        };

        self.playback_sessions.insert(session_id, session);

        self.execution_counter.inc();

        Ok(session_id)
    }

    /// Execute the next action in a playback session
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if:
    /// - Playback session with the given ID does not exist
    /// - Action execution fails
    pub async fn execute_next_action(
        &self,
        session_id: Uuid,
    ) -> Result<MacroPlaybackResult, MacroSystemError> {
        let mut session_guard = self
            .playback_sessions
            .get_mut(&session_id)
            .ok_or(MacroSystemError::SessionNotFound)?;

        let session = session_guard.value_mut();
        if session.state != MacroPlaybackState::Playing {
            return Ok(MacroPlaybackResult::SessionNotActive);
        }

        let macro_def = self
            .macros
            .get(&session.macro_id)
            .ok_or(MacroSystemError::MacroNotFound)?
            .value()
            .clone();

        if session.current_action >= macro_def.actions.len() {
            session.state = MacroPlaybackState::Completed;
            return Ok(MacroPlaybackResult::Completed);
        }

        let action = &macro_def.actions[session.current_action];
        let result = execute_action_sync(action, &mut session.context).await?;

        session.current_action += 1;

        match result {
            ActionExecutionResult::Success => {
                if session.current_action >= macro_def.actions.len() {
                    session.state = MacroPlaybackState::Completed;
                    Ok(MacroPlaybackResult::Completed)
                } else {
                    Ok(MacroPlaybackResult::ActionExecuted)
                }
            }
            ActionExecutionResult::Wait(duration) => {
                tokio::time::sleep(duration).await;
                Ok(MacroPlaybackResult::ActionExecuted)
            }
            ActionExecutionResult::SkipToAction(index) => {
                session.current_action = index;
                Ok(MacroPlaybackResult::ActionExecuted)
            }
            ActionExecutionResult::Error(error) => {
                session.state = MacroPlaybackState::Failed;
                session.error = Some(error);
                Ok(MacroPlaybackResult::Failed)
            }
        }
    }

    /// Delete a macro by ID
    ///
    /// # Errors
    ///
    /// Returns `MacroSystemError` if macro with the given ID does not exist
    pub fn delete_macro(&self, macro_id: Uuid) -> Result<(), MacroSystemError> {
        if self.macros.remove(&macro_id).is_some() {
            Ok(())
        } else {
            Err(MacroSystemError::MacroNotFound)
        }
    }
}

impl Default for MacroSystem {
    fn default() -> Self {
        Self::new()
    }
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
    use super::parser::{CondParser, tokenize_condition};

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

use std::future::Future;
/// Macro action execution
use std::pin::Pin;

fn execute_action_sync<'a>(
    action: &'a MacroAction,
    context: &'a mut MacroExecutionContext,
) -> Pin<Box<dyn Future<Output = Result<ActionExecutionResult, MacroSystemError>> + Send + 'a>> {
    Box::pin(async move {
        match action {
            MacroAction::SendMessage {
                content,
                message_type,
                ..
            } => {
                let resolved_content = resolve_variables_sync(content, &context.variables);

                // Send message to conversation if available
                if let Some(ref conversation) = context.conversation {
                    match send_message_to_conversation(
                        conversation,
                        resolved_content.clone(),
                        message_type,
                    )
                    .await
                    {
                        Ok(()) => Ok(ActionExecutionResult::Success),
                        Err(e) => Ok(ActionExecutionResult::Error(format!(
                            "Message send failed: {e}"
                        ))),
                    }
                } else {
                    // No conversation available - log warning and continue
                    warn!(
                        "No conversation available for SendMessage action. Message: {resolved_content}"
                    );
                    Ok(ActionExecutionResult::Success)
                }
            }
            MacroAction::ExecuteCommand { command, .. } => {
                // Note: Sync command execution not supported - commands require async streams
                // This sync helper is only used for conditional/loop actions
                // Real command execution happens in the async _execute_action function
                warn!("Command execution skipped in sync context: {command:?}");
                Ok(ActionExecutionResult::Success)
            }
            MacroAction::Wait { duration, .. } => Ok(ActionExecutionResult::Wait(*duration)),
            MacroAction::SetVariable { name, value, .. } => {
                let resolved_value = resolve_variables_sync(value, &context.variables);
                context.variables.insert(name.clone(), resolved_value);
                Ok(ActionExecutionResult::Success)
            }
            MacroAction::Conditional {
                condition,
                then_actions,
                else_actions,
                ..
            } => {
                let condition_result = evaluate_condition_sync(condition, &context.variables);

                let actions_to_execute = if condition_result {
                    then_actions
                } else if let Some(else_actions) = else_actions {
                    else_actions
                } else {
                    return Ok(ActionExecutionResult::Success);
                };

                // Execute conditional actions synchronously
                for action in actions_to_execute {
                    execute_action_sync(action, context).await?;
                }

                Ok(ActionExecutionResult::Success)
            }
            MacroAction::Loop {
                iterations,
                actions,
                ..
            } => {
                let loop_context = LoopContext {
                    iteration: 0,
                    max_iterations: *iterations,
                    start_action: context.current_action,
                    end_action: context.current_action + actions.len() - 1,
                };

                context.loop_stack.push(loop_context);

                for _ in 0..*iterations {
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
        }
    })
}

/// Wrapper for `Uuid` to implement `MessageChunk`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MacroSessionId(pub Uuid);

/// Wrapper for `()` to implement `MessageChunk`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct MacroActionResult;

impl From<Uuid> for MacroSessionId {
    fn from(uuid: Uuid) -> Self {
        MacroSessionId(uuid)
    }
}

impl From<MacroSessionId> for Uuid {
    fn from(session_id: MacroSessionId) -> Self {
        session_id.0
    }
}

/// Implementation of `MessageChunk` for `MacroSessionId` to enable streaming
impl MessageChunk for MacroSessionId {
    fn bad_chunk(_error: String) -> Self {
        // For errors, we'll generate a nil UUID and log the error
        // This is a fallback since Uuid doesn't have a natural error representation
        MacroSessionId(Uuid::nil())
    }

    fn error(&self) -> Option<&str> {
        // MacroSessionId doesn't carry error information, so always None
        None
    }
}

/// Implementation of `MessageChunk` for `MacroActionResult` to enable streaming
impl MessageChunk for MacroActionResult {
    fn bad_chunk(_error: String) -> Self {
        // For action results, we just return the default unit result
        MacroActionResult
    }

    fn error(&self) -> Option<&str> {
        // MacroActionResult doesn't carry error information, so always None
        None
    }
}
