use kodegen_daemon::service::sse::encoder::SseEncoder;
use kodegen_daemon::service::sse::events::{SseEvent, EventType};

#[test]
fn test_encode_simple_event() {
    let encoder = SseEncoder::new();
    let event = SseEvent::new(EventType::Message, "Hello, World!");

    let encoded = encoder.encode(&event);
    let expected = "event: message\ndata: Hello, World!\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_data_only_event() {
    let encoder = SseEncoder::new();
    let event = SseEvent::data_only("Just data");

    let encoded = encoder.encode(&event);
    let expected = "data: Just data\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_event_with_id() {
    let encoder = SseEncoder::new();
    let event = SseEvent::new(EventType::Ping, "timestamp").with_id("ping-123");

    let encoded = encoder.encode(&event);
    let expected = "event: ping\ndata: timestamp\nid: ping-123\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_multiline_data() {
    let encoder = SseEncoder::new();
    let event = SseEvent::new(EventType::Message, "Line 1\nLine 2\nLine 3");

    let encoded = encoder.encode(&event);
    let expected = "event: message\ndata: Line 1\ndata: Line 2\ndata: Line 3\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_unicode_data() {
    let encoder = SseEncoder::new();
    let event = SseEvent::new(EventType::Message, "Hello 世界! 🌍");

    let encoded = encoder.encode(&event);
    let expected = "event: message\ndata: Hello 世界! 🌍\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_multiple_events() {
    let encoder = SseEncoder::new();
    let events = vec![
        SseEvent::ping("2025-01-07T12:00:00Z"),
        SseEvent::message(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#),
    ];

    let encoded = encoder.encode_multiple(&events);
    let expected = "event: ping\ndata: 2025-01-07T12:00:00Z\n\nevent: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_comment_encoding() {
    let comment = SseEncoder::comment("This is a comment");
    assert_eq!(comment, ": This is a comment\n\n");

    let keep_alive = SseEncoder::keep_alive();
    assert_eq!(keep_alive, ": keep-alive\n\n");
}

#[test]
fn test_endpoint_event_encoding() {
    let encoder = SseEncoder::new();
    let event = SseEvent::endpoint("abc123", "http://localhost:8080");

    let encoded = encoder.encode(&event);
    let expected =
        "event: endpoint\ndata: http://localhost:8080/messages?session_id=abc123\n\n";

    assert_eq!(encoded, expected);
}

#[test]
fn test_json_rpc_message_encoding() {
    let encoder = SseEncoder::new();
    let json_rpc = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"echo"}]}}"#;
    let event = SseEvent::message(json_rpc);

    let encoded = encoder.encode(&event);

    assert!(encoded.contains("event: message"));
    assert!(encoded.contains(&format!("data: {json_rpc}")));
    assert!(encoded.ends_with("\n\n"));
}
