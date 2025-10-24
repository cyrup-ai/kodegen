use kodegen_tools_claude_agent::control::protocol::{
    ClientCapabilities, ControlMessage, ControlRequest, ControlResponse, InitResponse,
    ProtocolHandler, ServerCapabilities,
};
use kodegen_tools_claude_agent::{
    HookEvent, PermissionRequest, PermissionResult, PermissionResultAllow, RequestId, ToolName,
    ToolPermissionContext,
};

#[test]
fn test_request_id_generation() {
    let handler = ProtocolHandler::new();
    let id1 = handler.next_id();
    let id2 = handler.next_id();
    assert_ne!(id1, id2);
}

#[test]
fn test_init_request_creation() {
    let handler = ProtocolHandler::new();
    let init_req = handler.create_init_request();
    assert_eq!(init_req.protocol_version, "1.0");
    assert!(
        init_req
            .capabilities
            .contains(ClientCapabilities::BIDIRECTIONAL)
    );
    assert_eq!(init_req.capabilities, ClientCapabilities::all_features());
}

#[test]
fn test_serialize_deserialize() {
    let handler = ProtocolHandler::new();
    let request = handler.create_interrupt_request();
    let message = ControlMessage::Request(request);

    let serialized = handler.serialize_message(&message).unwrap();
    let deserialized = handler.deserialize_message(serialized.trim()).unwrap();

    match deserialized {
        ControlMessage::Request(ControlRequest::Interrupt { .. }) => {}
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_deserialize_invalid_json() {
    let handler = ProtocolHandler::new();
    let result = handler.deserialize_message("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_deserialize_invalid_message_structure() {
    let handler = ProtocolHandler::new();
    let invalid = r#"{"type":"unknown_type"}"#;
    let result = handler.deserialize_message(invalid);
    assert!(result.is_err());
}

#[test]
fn test_deserialize_missing_fields() {
    let handler = ProtocolHandler::new();
    let missing = r#"{"type":"request"}"#;
    let result = handler.deserialize_message(missing);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handle_response_with_missing_pending_request() {
    let handler = ProtocolHandler::new();
    handler.set_initialized(true);

    // Create a response for a request that was never sent
    let response = ControlResponse::Success {
        id: RequestId::new("non-existent-req"),
        data: None,
    };

    // Should not error, just ignore
    let result = handler.handle_response(response).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_hook_response_without_channel() {
    let handler = ProtocolHandler::new();

    // Try to handle hook response without setting up channel
    let response = ControlResponse::Hook {
        id: "hook-1".to_string(),
        event: HookEvent::PreToolUse,
        event_data: Some(serde_json::json!({
            "toolName": "Read",
            "toolInput": { "file_path": "test.txt" }
        })),
    };

    // Should not error, just no-op
    let result = handler.handle_response(response).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_permission_response_without_channel() {
    let handler = ProtocolHandler::new();

    // Try to handle permission response without setting up channel
    let response = ControlResponse::Permission {
        id: RequestId::new("perm-1"),
        request: PermissionRequest {
            tool_name: ToolName::new("test"),
            tool_input: serde_json::json!({}),
            context: ToolPermissionContext {
                suggestions: vec![],
            },
        },
    };

    // Should not error, just no-op
    let result = handler.handle_response(response).await;
    assert!(result.is_ok());
}

#[test]
fn test_init_response_with_wrong_version() {
    let handler = ProtocolHandler::new();

    let init_response = InitResponse {
        protocol_version: "999.0".to_string(),
        cli_version: "1.0.0".to_string(),
        capabilities: ServerCapabilities::all_features(),
        session_id: "test".to_string(),
    };

    let result = handler.handle_init_response(&init_response);
    assert!(result.is_err());
    assert!(!handler.is_initialized());
}

#[tokio::test]
async fn test_send_request_without_init() {
    let handler = ProtocolHandler::new();
    assert!(!handler.is_initialized());

    let request = handler.create_interrupt_request();
    let result = handler.send_request(request).await;
    assert!(result.is_err());
}

#[test]
fn test_serialize_all_request_types() {
    let handler = ProtocolHandler::new();

    // Test Interrupt
    let req = handler.create_interrupt_request();
    let msg = ControlMessage::Request(req);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test SendMessage
    let req = handler.create_send_message_request("test".to_string());
    let msg = ControlMessage::Request(req);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test HookResponse
    let req = handler.create_hook_response("hook-1".to_string(), serde_json::json!({}));
    let msg = ControlMessage::Request(req);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test PermissionResponse
    let req = handler.create_permission_response(
        RequestId::new("req-1"),
        PermissionResult::Allow(PermissionResultAllow {
            updated_input: None,
            updated_permissions: None,
        }),
    );
    let msg = ControlMessage::Request(req);
    assert!(handler.serialize_message(&msg).is_ok());
}

#[test]
fn test_serialize_all_response_types() {
    let handler = ProtocolHandler::new();

    // Test Success
    let resp = ControlResponse::Success {
        id: RequestId::new("req-1"),
        data: Some(serde_json::json!({"result": "ok"})),
    };
    let msg = ControlMessage::Response(resp);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test Error
    let resp = ControlResponse::Error {
        id: RequestId::new("req-1"),
        message: "test error".to_string(),
        code: Some("ERR_TEST".to_string()),
    };
    let msg = ControlMessage::Response(resp);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test Hook
    let resp = ControlResponse::Hook {
        id: "hook-1".to_string(),
        event: HookEvent::PreToolUse,
        event_data: Some(serde_json::json!({
            "toolName": "Read",
            "toolInput": { "file_path": "test.txt" }
        })),
    };
    let msg = ControlMessage::Response(resp);
    assert!(handler.serialize_message(&msg).is_ok());

    // Test Permission
    let resp = ControlResponse::Permission {
        id: RequestId::new("perm-1"),
        request: PermissionRequest {
            tool_name: ToolName::new("test"),
            tool_input: serde_json::json!({}),
            context: ToolPermissionContext {
                suggestions: vec![],
            },
        },
    };
    let msg = ControlMessage::Response(resp);
    assert!(handler.serialize_message(&msg).is_ok());
}

#[test]
fn test_get_request_id() {
    let interrupt = ControlRequest::Interrupt {
        id: RequestId::new("id1"),
    };
    assert_eq!(ProtocolHandler::get_request_id(&interrupt).as_str(), "id1");

    let send_msg = ControlRequest::SendMessage {
        id: RequestId::new("id2"),
        content: "test".to_string(),
    };
    assert_eq!(ProtocolHandler::get_request_id(&send_msg).as_str(), "id2");

    let hook_resp = ControlRequest::HookResponse {
        id: RequestId::new("id3"),
        hook_id: "hook".to_string(),
        response: serde_json::json!({}),
    };
    assert_eq!(ProtocolHandler::get_request_id(&hook_resp).as_str(), "id3");

    let perm_resp = ControlRequest::PermissionResponse {
        id: RequestId::new("id4"),
        request_id: RequestId::new("perm"),
        result: PermissionResult::Allow(PermissionResultAllow {
            updated_input: None,
            updated_permissions: None,
        }),
    };
    assert_eq!(ProtocolHandler::get_request_id(&perm_resp).as_str(), "id4");
}
