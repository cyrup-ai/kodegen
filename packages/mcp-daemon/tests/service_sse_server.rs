use kodegen_daemon::service::sse::{SseServer, SseConfig};
use kodegen_daemon::service::sse::session::SessionManager;
use kodegen_daemon::service::sse::bridge::McpBridgeBuilder;
use kodegen_daemon::service::sse::encoder::SseEncoder;
use kodegen_daemon::service::sse::server::MessagesQuery;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_server_creation() {
    let config = SseConfig::default();
    let server = SseServer::new(config);
    assert_eq!(server.config.port, 8080);
}

#[test]
fn test_cors_configuration() {
    let config = SseConfig {
        cors_origins: vec!["*".to_string()],
        ..Default::default()
    };
    let server = SseServer::new(config);
    let _cors_layer = server.build_cors_layer();
    // CORS layer creation should not panic
}

#[test]
fn test_messages_query_parsing() {
    use serde_urlencoded;

    let query_str = "session_id=abc123";
    let query: MessagesQuery = serde_urlencoded::from_str(query_str).unwrap();
    assert_eq!(query.session_id, "abc123");
}

#[tokio::test]
async fn test_server_state_creation() {
    let config = SseConfig::default();
    let session_manager = Arc::new(SessionManager::default());
    let mcp_bridge = Arc::new(
        McpBridgeBuilder::new("http://localhost:3000".to_string())
            .timeout(Duration::from_secs(config.mcp_timeout))
            .build()
            .unwrap(),
    );
    let encoder = SseEncoder::new();

    let state = kodegen_daemon::service::sse::server::ServerState {
        session_manager,
        mcp_bridge,
        encoder,
        config,
    };

    assert_eq!(state.config.port, 8080);
}
