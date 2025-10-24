//! Protocol handler for managing control protocol communication

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{ClaudeError, Result};
use crate::types::hooks::HookEvent;
use crate::types::identifiers::RequestId;
use crate::types::permissions::{PermissionRequest, PermissionResult};

use super::capabilities::ClientCapabilities;
use super::messages::{ControlMessage, ControlRequest, ControlResponse, InitRequest, InitResponse};

/// Pending request awaiting response
struct PendingRequest {
    /// Response channel
    response_tx: oneshot::Sender<ControlResponse>,
}

/// Protocol handler for managing control protocol communication
pub struct ProtocolHandler {
    /// Request ID counter
    next_request_id: Arc<AtomicU64>,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    /// Initialized flag
    initialized: Arc<AtomicBool>,
    /// Hook callback channel
    hook_tx: Option<mpsc::UnboundedSender<(String, HookEvent, serde_json::Value)>>,
    /// Permission callback channel
    permission_tx: Option<mpsc::UnboundedSender<(RequestId, PermissionRequest)>>,
}

impl ProtocolHandler {
    /// Create a new protocol handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_request_id: Arc::new(AtomicU64::new(1)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            initialized: Arc::new(AtomicBool::new(false)),
            hook_tx: None,
            permission_tx: None,
        }
    }

    /// Set hook callback channel
    pub fn set_hook_channel(
        &mut self,
        tx: mpsc::UnboundedSender<(String, HookEvent, serde_json::Value)>,
    ) {
        self.hook_tx = Some(tx);
    }

    /// Set permission callback channel
    pub fn set_permission_channel(
        &mut self,
        tx: mpsc::UnboundedSender<(RequestId, PermissionRequest)>,
    ) {
        self.permission_tx = Some(tx);
    }

    /// Check if protocol is initialized
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Set protocol as initialized (for cases where no handshake is needed)
    pub fn set_initialized(&self, value: bool) {
        self.initialized.store(value, Ordering::SeqCst);
    }

    /// Generate next request ID
    ///
    /// Generates a unique request ID for each request. Used internally and in tests.
    #[must_use]
    pub fn next_id(&self) -> RequestId {
        let id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        RequestId::new(format!("req-{id}"))
    }

    /// Create initialization request
    #[must_use]
    pub fn create_init_request(&self) -> InitRequest {
        InitRequest {
            protocol_version: "1.0".to_string(),
            sdk_version: crate::VERSION.to_string(),
            capabilities: ClientCapabilities::all_features(),
        }
    }

    /// Handle initialization response
    ///
    /// # Errors
    /// Returns error if protocol version is unsupported
    pub fn handle_init_response(&self, response: &InitResponse) -> Result<()> {
        // Validate protocol version
        if response.protocol_version != "1.0" {
            return Err(ClaudeError::protocol_error(format!(
                "Unsupported protocol version: {}",
                response.protocol_version
            )));
        }

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Send a request and wait for response
    ///
    /// # Errors
    /// Returns error if protocol is not initialized
    pub async fn send_request(
        &self,
        request: ControlRequest,
    ) -> Result<oneshot::Receiver<ControlResponse>> {
        if !self.is_initialized() {
            return Err(ClaudeError::protocol_error(
                "Protocol not initialized - call init first",
            ));
        }

        let id = Self::get_request_id(&request);
        let (response_tx, response_rx) = oneshot::channel();

        let pending = PendingRequest { response_tx };

        {
            let mut pending_requests = self.pending_requests.lock().await;
            pending_requests.insert(id, pending);
        }

        Ok(response_rx)
    }

    /// Extract request ID from a control request
    ///
    /// # Examples
    /// ```
    /// use kodegen_claude_agent::control::protocol::{ProtocolHandler, ControlRequest};
    /// use kodegen_claude_agent::types::RequestId;
    ///
    /// let interrupt = ControlRequest::Interrupt {
    ///     id: RequestId::new("test-id"),
    /// };
    /// let id = ProtocolHandler::get_request_id(&interrupt);
    /// assert_eq!(id.as_str(), "test-id");
    /// ```
    #[must_use]
    pub fn get_request_id(request: &ControlRequest) -> RequestId {
        match request {
            ControlRequest::Interrupt { id }
            | ControlRequest::SendMessage { id, .. }
            | ControlRequest::HookResponse { id, .. }
            | ControlRequest::PermissionResponse { id, .. } => id.clone(),
        }
    }

    /// Handle incoming control response
    ///
    /// # Errors
    /// Currently returns Ok in all cases, but is Result for API consistency
    pub async fn handle_response(&self, response: ControlResponse) -> Result<()> {
        match &response {
            ControlResponse::Success { id, .. } | ControlResponse::Error { id, .. } => {
                let pending = self.pending_requests.lock().await.remove(id);
                if let Some(pending) = pending {
                    let _ = pending.response_tx.send(response);
                }
                Ok(())
            }
            ControlResponse::Hook {
                id,
                event,
                event_data,
            } => {
                if let Some(ref tx) = self.hook_tx {
                    // Default to empty object if no event data provided
                    let data = event_data.clone().unwrap_or_else(|| serde_json::json!({}));
                    tx.send((id.clone(), *event, data))
                        .map_err(|_| ClaudeError::protocol_error("Hook channel closed"))?;
                }
                Ok(())
            }
            ControlResponse::Permission { id, request } => {
                if let Some(ref tx) = self.permission_tx {
                    tx.send((id.clone(), request.clone()))
                        .map_err(|_| ClaudeError::protocol_error("Permission channel closed"))?;
                }
                Ok(())
            }
        }
    }

    /// Create interrupt request
    #[must_use]
    pub fn create_interrupt_request(&self) -> ControlRequest {
        ControlRequest::Interrupt { id: self.next_id() }
    }

    /// Create send message request
    #[must_use]
    pub fn create_send_message_request(&self, content: String) -> ControlRequest {
        ControlRequest::SendMessage {
            id: self.next_id(),
            content,
        }
    }

    /// Create hook response
    #[must_use]
    pub fn create_hook_response(
        &self,
        hook_id: String,
        response: serde_json::Value,
    ) -> ControlRequest {
        ControlRequest::HookResponse {
            id: self.next_id(),
            hook_id,
            response,
        }
    }

    /// Create permission response
    #[must_use]
    pub fn create_permission_response(
        &self,
        request_id: RequestId,
        result: PermissionResult,
    ) -> ControlRequest {
        ControlRequest::PermissionResponse {
            id: self.next_id(),
            request_id,
            result,
        }
    }

    /// Serialize control message to JSON
    ///
    /// # Errors
    /// Returns error if JSON serialization fails
    pub fn serialize_message(&self, message: &ControlMessage) -> Result<String> {
        serde_json::to_string(message)
            .map(|s| format!("{s}\n"))
            .map_err(|e| ClaudeError::json_encode(format!("Failed to serialize message: {e}")))
    }

    /// Deserialize control message from JSON
    ///
    /// # Errors
    /// Returns error if JSON deserialization fails
    pub fn deserialize_message(&self, json: &str) -> Result<ControlMessage> {
        serde_json::from_str(json)
            .map_err(|e| ClaudeError::json_decode(format!("Failed to deserialize message: {e}")))
    }
}

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}
