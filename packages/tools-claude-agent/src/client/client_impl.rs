//! `ClaudeSDKClient` implementation
//!
//! This module contains the constructor and public API methods for `ClaudeSDKClient`.

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::control::ProtocolHandler;
use crate::error::{ClaudeError, Result};
use crate::hooks::HookManager;
use crate::permissions::PermissionManager;
use crate::transport::{PromptInput, SubprocessTransport, Transport};
use crate::types::hooks::HookEvent;
use crate::types::identifiers::RequestId;
use crate::types::messages::Message;
use crate::types::options::ClaudeAgentOptions;
use crate::types::permissions::{PermissionRequest, PermissionResult};

impl super::ClaudeSDKClient {
    /// Create a new `ClaudeSDKClient`
    ///
    /// # Arguments
    /// * `options` - Configuration options
    /// * `cli_path` - Optional path to Claude Code CLI
    ///
    /// # Errors
    /// Returns error if CLI cannot be found or connection fails
    pub async fn new(
        options: ClaudeAgentOptions,
        cli_path: Option<std::path::PathBuf>,
    ) -> Result<super::ClaudeSDKClient> {
        // Initialize hook manager if hooks are configured
        let (hook_manager, hook_rx) = options.hooks.as_ref().map_or_else(
            || (None, Some(mpsc::unbounded_channel().1)),
            |hooks_config| {
                let mut manager = HookManager::new();
                for matchers in hooks_config.values() {
                    for matcher in matchers {
                        manager.register(matcher.clone());
                    }
                }
                (Some(Arc::new(Mutex::new(manager))), None)
            },
        );

        // Initialize permission manager if callback is configured
        let (permission_manager, permission_rx) = if options.can_use_tool.is_some() {
            let mut manager = PermissionManager::new();
            if let Some(callback) = options.can_use_tool.clone() {
                manager.set_callback(callback);
            }
            manager.set_allowed_tools(Some(options.allowed_tools.clone()));
            manager.set_disallowed_tools(options.disallowed_tools.clone());
            (Some(Arc::new(Mutex::new(manager))), None)
        } else {
            (None, Some(mpsc::unbounded_channel().1))
        };

        // Create transport with streaming mode
        let prompt_input = PromptInput::Stream;
        let mut transport = SubprocessTransport::new(prompt_input, options, cli_path)?;

        // Connect transport
        transport.connect().await?;

        // Create protocol handler
        let mut protocol = ProtocolHandler::new();

        // Set up channels
        let (hook_tx, hook_rx_internal) = mpsc::unbounded_channel();
        let (permission_tx, permission_rx_internal) = mpsc::unbounded_channel();
        protocol.set_hook_channel(hook_tx);
        protocol.set_permission_channel(permission_tx);

        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        // Note: Claude CLI doesn't use a separate control protocol initialization.
        // The stream-json mode expects user messages to be sent directly.
        // Mark protocol as initialized immediately.
        protocol.set_initialized(true);

        let transport = Arc::new(Mutex::new(transport));
        let protocol = Arc::new(Mutex::new(protocol));

        // Spawn message reader task
        let transport_clone = transport.clone();
        let protocol_clone = protocol.clone();
        let message_tx_clone = message_tx;
        tokio::spawn(async move {
            super::ClaudeSDKClient::message_reader_task(
                transport_clone,
                protocol_clone,
                message_tx_clone,
            )
            .await;
        });

        // Spawn control message writer task
        let transport_clone = transport.clone();
        let protocol_clone = protocol.clone();
        tokio::spawn(async move {
            super::ClaudeSDKClient::control_writer_task(
                transport_clone,
                protocol_clone,
                control_rx,
            )
            .await;
        });

        // Spawn hook handler task if hook manager is configured
        if let Some(ref manager) = hook_manager {
            let manager_clone = manager.clone();
            let protocol_clone = protocol.clone();
            let control_tx_clone = control_tx.clone();
            tokio::spawn(async move {
                super::ClaudeSDKClient::hook_handler_task(
                    manager_clone,
                    protocol_clone,
                    hook_rx_internal,
                    control_tx_clone,
                )
                .await;
            });
        }

        // Spawn permission handler task if permission manager is configured
        if let Some(ref manager) = permission_manager {
            let manager_clone = manager.clone();
            let protocol_clone = protocol.clone();
            let control_tx_clone = control_tx.clone();
            tokio::spawn(async move {
                super::ClaudeSDKClient::permission_handler_task(
                    manager_clone,
                    protocol_clone,
                    permission_rx_internal,
                    control_tx_clone,
                )
                .await;
            });
        }

        Ok(super::ClaudeSDKClient {
            transport,
            protocol,
            message_rx,
            control_tx,
            hook_rx,
            permission_rx,
            hook_manager,
            permission_manager,
        })
    }

    /// Send a message to Claude
    ///
    /// # Arguments
    /// * `content` - Message content to send
    ///
    /// # Errors
    /// Returns error if message cannot be sent
    pub async fn send_message(&mut self, content: impl Into<String>) -> Result<()> {
        // Send a user message in the format the CLI expects
        let message = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": content.into()
            }
        });
        let message_json = format!("{}\n", serde_json::to_string(&message)?);

        let mut transport = self.transport.lock().await;
        transport.write(&message_json).await
    }

    /// Send an interrupt signal
    ///
    /// **Note**: Interrupt functionality via control messages may not be fully supported
    /// in all Claude CLI versions. The method demonstrates the SDK's bidirectional
    /// capability and will send the control message without blocking, but the CLI
    /// may not process it. Check your CLI version for control message support.
    ///
    /// # Errors
    /// Returns error if interrupt cannot be sent
    pub async fn interrupt(&mut self) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_interrupt_request();
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Get the next message from the stream
    ///
    /// Returns None when the stream ends
    pub async fn next_message(&mut self) -> Option<Result<Message>> {
        self.message_rx.recv().await
    }

    /// Take the hook event receiver
    ///
    /// This allows the caller to handle hook events independently
    pub const fn take_hook_receiver(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<(String, HookEvent, serde_json::Value)>> {
        self.hook_rx.take()
    }

    /// Take the permission request receiver
    ///
    /// This allows the caller to handle permission requests independently
    pub const fn take_permission_receiver(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>> {
        self.permission_rx.take()
    }

    /// Respond to a hook event
    ///
    /// # Arguments
    /// * `hook_id` - ID of the hook event being responded to
    /// * `response` - Hook response data
    ///
    /// # Errors
    /// Returns error if response cannot be sent
    pub async fn respond_to_hook(
        &mut self,
        hook_id: String,
        response: serde_json::Value,
    ) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_hook_response(hook_id, response);
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Respond to a permission request
    ///
    /// # Arguments
    /// * `request_id` - ID of the permission request being responded to
    /// * `result` - Permission result (Allow/Deny)
    ///
    /// # Errors
    /// Returns error if response cannot be sent
    pub async fn respond_to_permission(
        &mut self,
        request_id: RequestId,
        result: PermissionResult,
    ) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_permission_response(request_id, result);
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Close the client and clean up resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    pub async fn close(&mut self) -> Result<()> {
        let mut transport = self.transport.lock().await;
        transport.close().await
    }
}

impl Drop for super::ClaudeSDKClient {
    fn drop(&mut self) {
        // Channel senders will be dropped, causing background tasks to exit
    }
}
