use kodegen_daemon::service::sse::events::{EventType, SseEvent};

#[test]
fn test_event_type_display() {
    assert_eq!(EventType::Endpoint.to_string(), "endpoint");
    assert_eq!(EventType::Message.to_string(), "message");
    assert_eq!(EventType::Ping.to_string(), "ping");
    assert_eq!(EventType::Error.to_string(), "error");
}

#[test]
fn test_sse_event_creation() {
    let event = SseEvent::new(EventType::Message, "test data");
    assert_eq!(event.event_type, Some(EventType::Message));
    assert_eq!(event.data, "test data");
    assert_eq!(event.id, None);
}

#[test]
fn test_endpoint_event() {
    let event = SseEvent::endpoint("session123", "http://localhost:8080");
    assert_eq!(event.event_type, Some(EventType::Endpoint));
    assert_eq!(
        event.data,
        "http://localhost:8080/messages?session_id=session123"
    );
}

#[test]
fn test_event_with_id() {
    let event = SseEvent::ping("2025-01-07T12:00:00Z").with_id("ping-1");
    assert_eq!(event.id, Some("ping-1".to_string()));
}

#[test]
fn test_event_type_checks() {
    let ping_event = SseEvent::ping("timestamp");
    assert!(ping_event.is_ping());
    assert!(!ping_event.is_error());

    let error_event = SseEvent::error("Something went wrong");
    assert!(error_event.is_error());
    assert!(!error_event.is_ping());
}
