//! SSE HTTP server implementation
//!
//! Implements the dual-endpoint SSE server with /sse and /messages endpoints
//! as specified in the MCP SSE transport protocol.

use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Sse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::oneshot;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use log::{debug, error, info, warn};

use crate::service::sse::{
    bridge::{create_invalid_request_response, validate_json_rpc_request, McpBridge, McpBridgeBuilder},
    encoder::SseEncoder,
    events::SseEvent,
    session::{ClientInfo, SessionManager},
    SseConfig,
};

/// SSE server state shared across handlers
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Session manager for tracking SSE connections
    pub session_manager: Arc<SessionManager>,
    /// Bridge for communicating with MCP server
    pub mcp_bridge: Arc<McpBridge>,
    /// SSE encoder for formatting events
    pub encoder: SseEncoder,
    /// Server configuration
    pub config: SseConfig,
}

/// Query parameters for the messages endpoint
#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    pub session_id: String,
}

/// SSE server implementation
#[derive(Debug)]
pub struct SseServer {
    pub config: SseConfig,
}

impl SseServer {
    /// Create a new SSE server with given configuration
    #[must_use] 
    pub fn new(config: SseConfig) -> Self {
        Self { config }
    }

    /// Start serving on the given address
    pub async fn serve(self, addr: SocketAddr, shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
        // Initialize components
        let session_manager = Arc::new(SessionManager::new(
            self.config.max_connections,
            Duration::from_secs(self.config.session_timeout),
        ));

        let mcp_bridge = Arc::new(
            McpBridgeBuilder::new(self.config.mcp_server_url.clone())
                .timeout(Duration::from_secs(self.config.mcp_timeout))
                .keepalive_timeout(Duration::from_secs(self.config.mcp_keepalive_timeout))
                .max_idle_connections(self.config.mcp_max_idle_connections)
                .user_agent(&self.config.mcp_user_agent)
                .build()
                .context("Failed to create MCP bridge")?,
        );

        let encoder = SseEncoder::new();

        let state = ServerState {
            session_manager: session_manager.clone(),
            mcp_bridge,
            encoder,
            config: self.config.clone(),
        };

        // Start background cleanup task
        let _cleanup_task = session_manager.start_cleanup_task(Duration::from_secs(60));

        // Build the router
        let app = self.build_router(state);

        // Start the server
        info!("Starting SSE server on {addr}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("Failed to bind to address")?;

        // Run server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
                info!("SSE server shutting down gracefully");
            })
            .await
            .context("SSE server error")?;

        info!("SSE server stopped");
        Ok(())
    }

    /// Build the axum router with all endpoints
    fn build_router(&self, state: ServerState) -> Router {
        Router::new()
            .route("/sse", get(handle_sse_endpoint))
            .route("/messages", post(handle_messages_endpoint))
            .route("/messages/batch", post(handle_batch_messages_endpoint))
            .route("/messages/stream", post(handle_streaming_messages_endpoint))
            .route("/health", get(handle_health_endpoint))
            .route("/metrics", get(handle_metrics_endpoint))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(self.build_cors_layer())
                    .into_inner(),
            )
            .with_state(state)
    }

    /// Build CORS layer based on configuration
    pub fn build_cors_layer(&self) -> CorsLayer {
        let mut cors = CorsLayer::new()
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ]);

        // Configure allowed origins
        if self.config.cors_origins.contains(&"*".to_string()) {
            cors = cors.allow_origin(Any);
        } else {
            for origin in &self.config.cors_origins {
                if let Ok(origin_header) = origin.parse::<axum::http::HeaderValue>() {
                    cors = cors.allow_origin(origin_header);
                }
            }
        }

        cors
    }
}

/// Handle GET /sse endpoint - establish SSE connection
async fn handle_sse_endpoint(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<
    Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>>,
    StatusCode,
> {
    // Extract client information
    let remote_addr = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);

    let client_info = ClientInfo {
        remote_addr: remote_addr.clone(),
        user_agent,
        connection_id: None,
    };

    // Create new session
    let session = if let Some(session) = state.session_manager.create_session(client_info).await { session } else {
        warn!(
            "Rejected SSE connection from {remote_addr} (session limit reached)"
        );
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    info!("Established SSE connection for session {}", session.id);

    // Create event stream using futures stream
    let session_id = session.id.clone();
    let session_manager = state.session_manager.clone();
    let encoder = state.encoder.clone();
    let config = state.config.clone();

    use futures_util::stream::{self, StreamExt};

    // Send initial endpoint event
    let base_url = format!("http://127.0.0.1:{}", config.port);
    let endpoint_event = SseEvent::endpoint(&session_id, &base_url);
    let initial_data = encoder.encode(&endpoint_event);
    let initial_event = axum::response::sse::Event::default().data(initial_data.trim_end());

    // Create ping stream
    let ping_stream = {
        let session_id = session_id.clone();
        let session_manager = session_manager.clone();
        let encoder = encoder.clone();
        let ping_interval = config.ping_interval;

        stream::unfold(0u64, move |event_counter| {
            let session_id = session_id.clone();
            let session_manager = session_manager.clone();
            let encoder = encoder.clone();

            async move {
                // Wait for ping interval
                tokio::time::sleep(Duration::from_secs(ping_interval)).await;

                // Create ping event
                let timestamp = chrono::Utc::now().to_rfc3339();
                let ping_event =
                    SseEvent::ping(timestamp).with_id(format!("ping-{event_counter}"));
                let encoded = encoder.encode(&ping_event);
                let event = axum::response::sse::Event::default().data(encoded.trim_end());

                // Touch session to keep it alive
                session_manager.touch_session(&session_id).await;

                Some((Ok::<_, Infallible>(event), event_counter + 1))
            }
        })
    };

    // Combine initial event with ping stream
    let combined_stream =
        stream::once(async { Ok::<_, Infallible>(initial_event) }).chain(ping_stream);

    Ok(Sse::new(combined_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(config.ping_interval))
            .text("keep-alive"),
    ))
}

/// Handle POST /messages endpoint - process JSON-RPC requests
async fn handle_messages_endpoint(
    State(state): State<ServerState>,
    Query(query): Query<MessagesQuery>,
    body: String,  // Accept raw string instead of Json<Value>
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Validate session exists
    let session = if let Some(session) = state.session_manager.get_session(&query.session_id).await { session } else {
        warn!("Message for unknown session: {}", query.session_id);
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Session not found"
        }))));
    };

    // Touch session to update activity
    state.session_manager.touch_session(&session.id).await;

    // Validate request size (DoS protection)
    if let Err(size_error) = 
        crate::service::sse::bridge::validation::validate_request_size(&body) 
    {
        warn!("Request size validation failed: {size_error}");
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": "Request exceeds maximum size of 1MB"
            }))
        ));
    }

    // Try to parse JSON
    let request = match serde_json::from_str::<Value>(&body) {
        Ok(req) => req,
        Err(parse_error) => {
            warn!("Failed to parse JSON: {parse_error}");
            
            // Try to extract ID from malformed request
            let id = crate::service::sse::bridge::validation::extract_request_id(&body);
            
            let error_response = 
                crate::service::sse::bridge::validation::create_parse_error_response(id);
            
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
    };

    log::debug!(
        "Received message for session {}: {}",
        query.session_id, request
    );

    // Validate JSON-RPC format
    if let Err(error) = validate_json_rpc_request(&request) {
        warn!("Invalid JSON-RPC request: {error}");
        let error_response = create_invalid_request_response(request.get("id").cloned());
        return Ok(Json(error_response));
    }

    // Determine if this is a critical operation requiring retry
    let is_critical = is_critical_operation(&request);

    // Forward request with appropriate handling
    let response = if is_critical {
        state.mcp_bridge.forward_request_with_retry(
            request,
            state.config.mcp_max_retries,
            Duration::from_millis(state.config.mcp_retry_delay_ms)
        ).await
    } else {
        state.mcp_bridge.forward_request(request).await
    };

    debug!("Returning response for session {}", session.id);
    Ok(Json(response))
}

/// Handle POST /messages/stream endpoint - SSE streaming for long operations
async fn handle_streaming_messages_endpoint(
    State(state): State<ServerState>,
    Query(query): Query<MessagesQuery>,
    Json(request): Json<Value>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    
    log::debug!("Received streaming request for session {}", query.session_id);
    
    // Create channel for streaming SSE events (not raw Values)
    let (tx, rx) = mpsc::unbounded_channel::<Result<axum::response::sse::Event, Infallible>>();
    
    // Validate session (same as regular messages)
    if state.session_manager.get_session(&query.session_id).await.is_none() {
        warn!("Streaming request for unknown session: {}", query.session_id);
        // Return empty stream on invalid session - channel already closed
        return Sse::new(UnboundedReceiverStream::new(rx));
    }
    
    // Touch session
    state.session_manager.touch_session(&query.session_id).await;
    
    // Spawn forwarding task
    let bridge = state.mcp_bridge.clone();
    tokio::spawn(async move {
        let callback = move |response: Value| {
            let data = serde_json::to_string(&response).unwrap_or_default();
            let event = Ok(axum::response::sse::Event::default().data(data));
            let _ = tx.send(event);
        };
        
        if let Err(e) = bridge.forward_streaming_request(request, callback).await {
            error!("Streaming request failed: {e}");
        }
    });
    
    Sse::new(UnboundedReceiverStream::new(rx))
}

/// Handle POST /messages/batch endpoint - process batch JSON-RPC requests
async fn handle_batch_messages_endpoint(
    State(state): State<ServerState>,
    Query(query): Query<MessagesQuery>,
    body: String,
) -> Result<Json<Vec<Value>>, (StatusCode, String)> {
    // Validate session exists
    let session = if let Some(session) = state.session_manager.get_session(&query.session_id).await { session } else {
        warn!("Batch message for unknown session: {}", query.session_id);
        return Err((
            StatusCode::BAD_REQUEST,
            "Session not found".to_string()
        ));
    };

    // Touch session to update activity
    state.session_manager.touch_session(&session.id).await;

    // Validate request size
    if let Err(size_error) = 
        crate::service::sse::bridge::validation::validate_request_size(&body) 
    {
        warn!("Batch request size validation failed: {size_error}");
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Batch request exceeds maximum size of 1MB".to_string()
        ));
    }

    // Parse batch requests
    let requests: Vec<Value> = match serde_json::from_str(&body) {
        Ok(reqs) => reqs,
        Err(parse_error) => {
            warn!("Failed to parse batch JSON: {parse_error}");
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON in batch request: {parse_error}")
            ));
        }
    };

    debug!(
        "Received batch of {} requests for session {}",
        requests.len(), query.session_id
    );

    // Validate batch
    let validation_results = 
        crate::service::sse::bridge::validation::validate_batch_requests(&requests);
    
    // Return first validation error if any exist
    for result in validation_results {
        if let Err(validation_error) = result {
            warn!("Batch validation failed: {validation_error}");
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Batch validation failed: {validation_error}")
            ));
        }
    }

    // Forward batch to MCP bridge
    let responses = state.mcp_bridge.forward_batch_requests(requests).await;

    debug!(
        "Returning {} responses for session {}",
        responses.len(), query.session_id
    );
    Ok(Json(responses))
}

/// Handle GET /health endpoint - health check
async fn handle_health_endpoint(
    State(state): State<ServerState>,
) -> Result<(StatusCode, Json<HealthResponse>), StatusCode> {
    let session_count = state.session_manager.session_count().await;
    let mcp_healthy = state.mcp_bridge.health_check().await.unwrap_or(false);
    let stats = state.mcp_bridge.get_forwarding_stats();
    
    // Combined health: MCP reachable AND stats indicate healthy performance
    let is_healthy = mcp_healthy && stats.is_healthy();

    let response = HealthResponse {
        status: if is_healthy { "healthy" } else { "degraded" }.to_string(),
        session_count,
        mcp_server_url: state.mcp_bridge.server_url().to_string(),
        mcp_server_healthy: mcp_healthy,
        success_rate: stats.success_rate(),
        average_response_time_ms: stats.average_response_time_ms,
    };

    let status_code = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(response)))
}

/// Handle GET /metrics endpoint - performance metrics
async fn handle_metrics_endpoint(
    State(state): State<ServerState>,
) -> Json<MetricsResponse> {
    let conn_stats = state.mcp_bridge.get_connection_stats();
    let fwd_stats = state.mcp_bridge.get_forwarding_stats();
    let session_count = state.session_manager.session_count().await;
    
    Json(MetricsResponse {
        bridge_stats: BridgeStats {
            total_requests: conn_stats.total_requests,
            successful_requests: conn_stats.successful_requests,
            failed_requests: conn_stats.failed_requests,
            last_request_time: conn_stats.last_request_time.map(|t| t.to_rfc3339()),
        },
        forwarding_stats: ForwardingStatsView {
            total_requests: fwd_stats.total_requests,
            successful_requests: fwd_stats.successful_requests,
            failed_requests: fwd_stats.failed_requests,
            average_response_time_ms: fwd_stats.average_response_time_ms,
            success_rate: fwd_stats.success_rate(),
            failure_rate: fwd_stats.failure_rate(),
            is_healthy: fwd_stats.is_healthy(),
            last_request_time: fwd_stats.last_request_time.map(|t| t.to_rfc3339()),
        },
        session_count,
    })
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    session_count: usize,
    mcp_server_url: String,
    mcp_server_healthy: bool,
    success_rate: f64,
    average_response_time_ms: f64,
}

/// Metrics response structure
#[derive(Debug, Serialize)]
struct MetricsResponse {
    bridge_stats: BridgeStats,
    forwarding_stats: ForwardingStatsView,
    session_count: usize,
}

#[derive(Debug, Serialize)]
struct BridgeStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    last_request_time: Option<String>,
}

#[derive(Debug, Serialize)]
struct ForwardingStatsView {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    average_response_time_ms: f64,
    success_rate: f64,
    failure_rate: f64,
    is_healthy: bool,
    last_request_time: Option<String>,
}

/// Determine if a JSON-RPC method should use retry logic
fn is_critical_operation(request: &Value) -> bool {
    matches!(
        request.get("method").and_then(|m| m.as_str()),
        Some("tools/call_tool" | "resources/read" | "prompts/get")
    )
}


