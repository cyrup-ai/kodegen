//! Control protocol message types
//!
//! This module defines all message types used in the control protocol for
//! bidirectional communication between the SDK and CLI.

use serde::{Deserialize, Serialize};

use crate::types::hooks::HookEvent;
use crate::types::identifiers::RequestId;
use crate::types::permissions::{PermissionRequest, PermissionResult};

use super::capabilities::{ClientCapabilities, ServerCapabilities};

/// Control message envelope for all protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    /// Request from SDK to CLI
    #[serde(rename = "request")]
    Request(ControlRequest),
    /// Response from CLI to SDK
    #[serde(rename = "response")]
    Response(ControlResponse),
    /// Initialization request
    #[serde(rename = "init")]
    Init(InitRequest),
    /// Initialization response
    #[serde(rename = "init_response")]
    InitResponse(InitResponse),
}

/// Request from SDK to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum ControlRequest {
    /// Interrupt the current operation
    #[serde(rename = "interrupt")]
    Interrupt {
        /// Unique request identifier
        id: RequestId,
    },
    /// Send a message to Claude
    #[serde(rename = "send_message")]
    SendMessage {
        /// Unique request identifier
        id: RequestId,
        /// Message content to send
        content: String,
    },
    /// Respond to a hook invocation
    #[serde(rename = "hook_response")]
    HookResponse {
        /// Unique request identifier
        id: RequestId,
        /// Hook event ID being responded to
        hook_id: String,
        /// Hook response data
        response: serde_json::Value,
    },
    /// Respond to a permission request
    #[serde(rename = "permission_response")]
    PermissionResponse {
        /// Unique request identifier
        id: RequestId,
        /// Permission request ID being responded to
        request_id: RequestId,
        /// Permission result (Allow/Deny)
        result: PermissionResult,
    },
}

/// Response from CLI to SDK
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ControlResponse {
    /// Successful response
    #[serde(rename = "success")]
    Success {
        /// Request ID this responds to
        id: RequestId,
        /// Optional response data
        data: Option<serde_json::Value>,
    },
    /// Error response
    #[serde(rename = "error")]
    Error {
        /// Request ID this responds to
        id: RequestId,
        /// Error message
        message: String,
        /// Error code
        code: Option<String>,
    },
    /// Hook invocation from CLI
    #[serde(rename = "hook")]
    Hook {
        /// Hook invocation ID
        id: String,
        /// Hook event details
        event: HookEvent,
        /// Event-specific data payload
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_data: Option<serde_json::Value>,
    },
    /// Permission request from CLI
    #[serde(rename = "permission")]
    Permission {
        /// Permission request ID
        id: RequestId,
        /// Permission request details
        request: PermissionRequest,
    },
}

/// Initialization request sent from SDK to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRequest {
    /// Protocol version
    pub protocol_version: String,
    /// SDK version
    pub sdk_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
}

/// Initialization response from CLI to SDK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResponse {
    /// Protocol version accepted
    pub protocol_version: String,
    /// CLI version
    pub cli_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Session ID for this connection
    pub session_id: String,
}
