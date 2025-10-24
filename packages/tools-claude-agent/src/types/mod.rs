//! Type definitions for Claude Agent SDK
//!
//! This module contains all the type definitions used throughout the SDK,
//! organized into logical submodules:
//!
//! - [`identifiers`] - Type-safe ID wrappers (`SessionId`, `ToolName`, `RequestId`)
//! - [`permissions`] - Permission modes, rules, and callbacks
//! - [`hooks`] - Hook system types and callbacks
//! - [`mcp`] - MCP server configuration
//! - [`messages`] - Message and content block types
//! - [`agent`] - Agent definitions and system prompts
//! - [`options`] - Main configuration options
//! - [`prompt_input`] - Prompt input types supporting both plain strings and templates

pub mod agent;
pub mod hooks;
pub mod identifiers;
pub mod mcp;
pub mod messages;
pub mod options;
pub mod permissions;
/// Prompt input types for Claude agents
///
/// Supports both plain string prompts and template-based prompts with parameters.
pub mod prompt_input;

// Re-export commonly used types
pub use identifiers::{RequestId, SessionId, ToolName};
pub use permissions::{
    CanUseToolCallback, PermissionBehavior, PermissionMode, PermissionRequest, PermissionResult,
    PermissionResultAllow, PermissionResultDeny, PermissionRuleValue, PermissionUpdate,
    PermissionUpdateDestination, SettingSource, ToolPermissionContext,
};

// Re-export session management types from agent module
pub use agent::{
    AgentInfo, GetOutputResponse, ListSessionsResponse, SerializedMessage, TerminateResponse,
};

// Re-export prompt input types
pub use prompt_input::{PromptInput, PromptTemplateInput};
