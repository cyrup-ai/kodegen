//! Background tasks for `ClaudeSDKClient`
//!
//! This module contains the async task implementations that run in the background
//! to handle message reading, control writing, hooks, and permissions.

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::control::{ControlMessage, ControlRequest, ProtocolHandler};
use crate::error::Result;
use crate::hooks::HookManager;
use crate::message::parse_message;
use crate::permissions::PermissionManager;
use crate::transport::{SubprocessTransport, Transport};
use crate::types::hooks::{HookContext, HookEvent};
use crate::types::identifiers::RequestId;
use crate::types::messages::Message;
use crate::types::permissions::PermissionRequest;

impl super::ClaudeSDKClient {
    /// Message reader task - reads from transport and processes messages
    pub(super) async fn message_reader_task(
        transport: Arc<Mutex<SubprocessTransport>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        message_tx: mpsc::UnboundedSender<Result<Message>>,
    ) {
        // Get the message receiver from the transport without holding the lock
        let mut msg_stream = {
            let mut transport_guard = transport.lock().await;
            transport_guard.read_messages()
        };

        while let Some(result) = msg_stream.recv().await {
            match result {
                Ok(value) => {
                    // Try to parse as control message first
                    let protocol_guard = protocol.lock().await;
                    if let Ok(control_msg) = protocol_guard
                        .deserialize_message(&serde_json::to_string(&value).unwrap_or_default())
                    {
                        match control_msg {
                            ControlMessage::InitResponse(init_response) => {
                                if let Err(e) = protocol_guard.handle_init_response(&init_response)
                                {
                                    let _ = message_tx.send(Err(e));
                                    break;
                                }
                            }
                            ControlMessage::Response(response) => {
                                if let Err(e) = protocol_guard.handle_response(response).await {
                                    let _ = message_tx.send(Err(e));
                                }
                            }
                            ControlMessage::Request(_) | ControlMessage::Init(_) => {
                                // Ignore requests and init in client mode
                            }
                        }
                        drop(protocol_guard);
                        continue;
                    }
                    drop(protocol_guard);

                    // Otherwise parse as regular message
                    match parse_message(value) {
                        Ok(msg) => {
                            if message_tx.send(Ok(msg)).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = message_tx.send(Err(e));
                        }
                    }
                }
                Err(e) => {
                    let _ = message_tx.send(Err(e));
                    break;
                }
            }
        }
    }

    /// Control message writer task - writes control requests to transport
    pub(super) async fn control_writer_task(
        transport: Arc<Mutex<SubprocessTransport>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        mut control_rx: mpsc::UnboundedReceiver<ControlRequest>,
    ) {
        while let Some(request) = control_rx.recv().await {
            // Determine message format based on request type
            let message_line = match &request {
                // Simple stream-json format for these
                ControlRequest::Interrupt { .. } => {
                    let control_json = serde_json::json!({
                        "type": "control",
                        "method": "interrupt"
                    });
                    serde_json::to_string(&control_json).ok()
                }
                ControlRequest::SendMessage { content, .. } => {
                    let control_json = serde_json::json!({
                        "type": "user",
                        "message": {
                            "role": "user",
                            "content": content
                        }
                    });
                    serde_json::to_string(&control_json).ok()
                }

                // Full control protocol for bidirectional messages
                ControlRequest::HookResponse { .. } | ControlRequest::PermissionResponse { .. } => {
                    let protocol_guard = protocol.lock().await;
                    let message = ControlMessage::Request(request.clone());
                    let result = protocol_guard.serialize_message(&message).ok();
                    drop(protocol_guard);
                    result
                }
            };

            if let Some(json_str) = message_line {
                let formatted = if json_str.ends_with('\n') {
                    json_str
                } else {
                    format!("{json_str}\n")
                };

                let mut transport_guard = transport.lock().await;
                if transport_guard.write(&formatted).await.is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Hook handler task - automatically processes hook events
    pub(super) async fn hook_handler_task(
        manager: Arc<Mutex<HookManager>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        mut hook_rx: mpsc::UnboundedReceiver<(String, HookEvent, serde_json::Value)>,
        control_tx: mpsc::UnboundedSender<ControlRequest>,
    ) {
        while let Some((hook_id, event, event_data)) = hook_rx.recv().await {
            // Extract tool name from event data for tool-related events
            let tool_name = match event {
                HookEvent::PreToolUse | HookEvent::PostToolUse => event_data
                    .get("toolName")
                    .or_else(|| event_data.get("tool_name"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                _ => None,
            };

            // Validate event_data is an object for tool events
            if matches!(event, HookEvent::PreToolUse | HookEvent::PostToolUse)
                && !event_data.is_object()
            {
                log::warn!(
                    "Hook {hook_id} received non-object event_data for {event:?}: {event_data:?}"
                );
                // Continue with data anyway - hooks can handle invalid data
            }

            let manager_guard = manager.lock().await;
            let context = HookContext {};

            match manager_guard
                .invoke(event_data.clone(), tool_name, context)
                .await
            {
                Ok(output) => {
                    drop(manager_guard);

                    // Send hook response
                    let protocol_guard = protocol.lock().await;
                    let response = serde_json::to_value(&output).unwrap_or_default();
                    let request = protocol_guard.create_hook_response(hook_id, response);
                    drop(protocol_guard);

                    if let Err(e) = control_tx.send(request) {
                        log::error!("Failed to send hook response: {e}");
                    }
                    log::debug!("Hook processed for event {event:?}");
                }
                Err(e) => {
                    log::error!("Hook processing error: {e}");
                }
            }
        }
    }

    /// Permission handler task - automatically processes permission requests
    pub(super) async fn permission_handler_task(
        manager: Arc<Mutex<PermissionManager>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        mut permission_rx: mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>,
        control_tx: mpsc::UnboundedSender<ControlRequest>,
    ) {
        while let Some((request_id, request)) = permission_rx.recv().await {
            let manager_guard = manager.lock().await;

            match manager_guard
                .can_use_tool(
                    request.tool_name.clone(),
                    request.tool_input.clone(),
                    request.context.clone(),
                )
                .await
            {
                Ok(result) => {
                    drop(manager_guard);

                    // Send permission response
                    let protocol_guard = protocol.lock().await;
                    let request = protocol_guard
                        .create_permission_response(request_id.clone(), result.clone());
                    drop(protocol_guard);

                    if let Err(e) = control_tx.send(request) {
                        log::error!("Failed to send permission response: {e}");
                    }
                    log::debug!("Permission {} processed: {:?}", request_id.as_str(), result);
                }
                Err(e) => {
                    log::error!("Permission processing error: {e}");
                }
            }
        }
    }
}
