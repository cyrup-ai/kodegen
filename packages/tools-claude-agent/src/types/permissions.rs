//! Permission-related type definitions
//!
//! This module contains types for managing permissions, including permission modes,
//! permission rules, permission requests, and permission results.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::identifiers::ToolName;
use crate::error::Result;

// ============================================================================
// Permission Types
// ============================================================================

/// Permission modes for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - CLI prompts for dangerous tools
    Default,
    /// Auto-accept file edits
    AcceptEdits,
    /// Plan mode
    Plan,
    /// Allow all tools (use with caution)
    BypassPermissions,
}

/// Setting source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    /// User-level settings
    User,
    /// Project-level settings
    Project,
    /// Local settings
    Local,
}

/// Permission update destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// Save to user settings
    UserSettings,
    /// Save to project settings
    ProjectSettings,
    /// Save to local settings
    LocalSettings,
    /// Save to session only (temporary)
    Session,
}

/// Permission behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
    /// Ask the user
    Ask,
}

/// Permission rule value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleValue {
    /// Name of the tool
    pub tool_name: String,
    /// Optional rule content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// Permission update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PermissionUpdate {
    /// Add permission rules
    AddRules {
        /// Rules to add
        #[serde(skip_serializing_if = "Option::is_none")]
        rules: Option<Vec<PermissionRuleValue>>,
        /// Where to save the rules
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
    /// Replace existing permission rules
    ReplaceRules {
        /// New rules
        #[serde(skip_serializing_if = "Option::is_none")]
        rules: Option<Vec<PermissionRuleValue>>,
        /// Where to save the rules
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
    /// Remove permission rules
    RemoveRules {
        /// Rules to remove
        #[serde(skip_serializing_if = "Option::is_none")]
        rules: Option<Vec<PermissionRuleValue>>,
        /// Where to remove from
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
    /// Set permission mode
    SetMode {
        /// New permission mode
        mode: PermissionMode,
        /// Where to save the mode
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
    /// Add directories to allowed list
    AddDirectories {
        /// Directories to add
        #[serde(skip_serializing_if = "Option::is_none")]
        directories: Option<Vec<String>>,
        /// Where to save
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
    /// Remove directories from allowed list
    RemoveDirectories {
        /// Directories to remove
        #[serde(skip_serializing_if = "Option::is_none")]
        directories: Option<Vec<String>>,
        /// Where to remove from
        #[serde(skip_serializing_if = "Option::is_none")]
        destination: Option<PermissionUpdateDestination>,
    },
}

/// Context for tool permission callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionContext {
    /// Permission suggestions from CLI
    pub suggestions: Vec<PermissionUpdate>,
}

/// Permission request from CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// Tool name being requested
    pub tool_name: ToolName,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Permission context
    pub context: ToolPermissionContext,
}

/// Permission result for allowing tool use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResultAllow {
    /// Modified input for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    /// Permission updates to apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

/// Permission result for denying tool use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResultDeny {
    /// Reason for denying
    pub message: String,
    /// Whether to interrupt the conversation
    #[serde(default)]
    pub interrupt: bool,
}

/// Permission result enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PermissionResult {
    /// Allow the tool use
    Allow(PermissionResultAllow),
    /// Deny the tool use
    Deny(PermissionResultDeny),
}

/// Callback type for tool permission checks
pub type CanUseToolCallback = Arc<
    dyn Fn(
            ToolName,
            serde_json::Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
>;
