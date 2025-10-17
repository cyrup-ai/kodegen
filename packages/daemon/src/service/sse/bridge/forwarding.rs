//! Request forwarding and response handling
//!
//! This module provides request forwarding logic and response processing
//! for the MCP bridge with zero allocation patterns and blazing-fast
//! performance.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::StreamExt;
use serde_json::Value;
use log::{debug, error, warn};

use super::core::McpBridge;

/// Maximum size for a single SSE event to prevent DoS attacks
///
/// Set to 10MB which is sufficient for:
/// - Large JSON-RPC responses with embeddings (typically < 5MB)
/// - File content transfers (reasonable chunk size)
/// - Tool outputs with extensive logs
///
/// This limit prevents malicious servers from exhausting memory by sending
/// events without boundaries.
const MAX_SSE_EVENT_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Find the earliest event boundary in the buffer and return its position and length
///
/// SSE spec allows three boundary types:
/// - \n\n (LF LF) - 2 bytes
/// - \r\n\r\n (CRLF CRLF) - 4 bytes
/// - \r\r (CR CR) - 2 bytes
///
/// Returns: Some((position, boundary_length)) or None if no boundary found
fn find_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    // Search for all possible boundary patterns
    let lf_lf = buffer.windows(2).position(|w| w == b"\n\n");
    let crlf_crlf = buffer.windows(4).position(|w| w == b"\r\n\r\n");
    let cr_cr = buffer.windows(2).position(|w| w == b"\r\r");
    
    // Find the earliest boundary
    let mut earliest: Option<(usize, usize)> = None;
    
    if let Some(pos) = lf_lf {
        earliest = Some((pos, 2));
    }
    
    if let Some(pos) = crlf_crlf {
        match earliest {
            None => earliest = Some((pos, 4)),
            Some((earliest_pos, _)) if pos < earliest_pos => earliest = Some((pos, 4)),
            _ => {}
        }
    }
    
    if let Some(pos) = cr_cr {
        match earliest {
            None => earliest = Some((pos, 2)),
            Some((earliest_pos, _)) if pos < earliest_pos => earliest = Some((pos, 2)),
            _ => {}
        }
    }
    
    earliest
}

impl McpBridge {
    /// Parse a complete SSE event from the buffer
    /// 
    /// SSE format:
    /// event: <type>
    /// data: <line1>
    /// data: <line2>
    /// id: <id>
    /// <blank line>
    fn parse_sse_event(buffer: &mut Vec<u8>) -> Result<Option<super::super::events::SseEvent>> {
        // Look for event boundary (double newline - any SSE-compliant type)
        let boundary = find_event_boundary(buffer);
        
        let (pos, boundary_len) = match boundary {
            Some((p, len)) => (p, len),
            None => return Ok(None), // Incomplete event, need more data
        };
        
        // Extract complete event
        let event_bytes: Vec<u8> = buffer.drain(..pos + boundary_len).collect();
        let event_text = String::from_utf8_lossy(&event_bytes);
        
        // Parse SSE fields
        let mut event_type = None;
        let mut data_lines = Vec::new();
        let mut id = None;
        
        for line in event_text.lines() {
            if let Some(value) = line.strip_prefix("event: ") {
                event_type = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("data: ") {
                data_lines.push(value.to_string());
            } else if let Some(value) = line.strip_prefix("id: ") {
                id = Some(value.trim().to_string());
            }
            // Ignore comment lines starting with ':'
        }
        
        // Reconstruct multiline data
        if data_lines.is_empty() {
            return Ok(None); // No data, skip this event
        }
        
        let data = data_lines.join("\n");
        
        // Convert event_type string to EventType enum if present
        let parsed_event_type = event_type.and_then(|et| {
            match et.as_str() {
                "endpoint" => Some(super::super::events::EventType::Endpoint),
                "message" => Some(super::super::events::EventType::Message),
                "ping" => Some(super::super::events::EventType::Ping),
                "error" => Some(super::super::events::EventType::Error),
                _ => None,
            }
        });
        
        Ok(Some(super::super::events::SseEvent {
            event_type: parsed_event_type,
            data,
            id,
        }))
    }

    /// Forward a JSON-RPC request to the MCP server
    ///
    /// Takes a JSON-RPC request and forwards it to the configured MCP server.
    /// Returns the JSON-RPC response or an error response on failure.
    pub async fn forward_request(&self, json_rpc_request: Value) -> Value {
        let start_time = Instant::now();

        debug!(
            "Forwarding JSON-RPC request to MCP server: {}",
            json_rpc_request
        );

        // Validate request before forwarding
        if let Err(validation_error) =
            super::validation::validate_json_rpc_request(&json_rpc_request)
        {
            error!("Invalid JSON-RPC request: {}", validation_error);
            return self.create_error_response(&json_rpc_request, validation_error);
        }

        match self.send_request(json_rpc_request.clone()).await {
            Ok(response) => {
                let duration = start_time.elapsed();
                debug!(
                    "Received successful response from MCP server in {:?}",
                    duration
                );

                // Log slow requests
                if duration > Duration::from_millis(1000) {
                    warn!(
                        "Slow MCP server response: {:?} for request: {}",
                        duration,
                        json_rpc_request
                            .get("method")
                            .unwrap_or(&serde_json::Value::Null)
                    );
                }

                response
            }
            Err(error) => {
                let duration = start_time.elapsed();
                error!(
                    "Failed to forward request to MCP server after {:?}: {}",
                    duration, error
                );
                self.create_error_response(&json_rpc_request, error)
            }
        }
    }

    /// Forward multiple requests in batch
    #[allow(dead_code)]
    pub async fn forward_batch_requests(&self, requests: Vec<Value>) -> Vec<Value> {
        if requests.is_empty() {
            return Vec::new();
        }

        debug!("Forwarding batch of {} requests", requests.len());

        let start_time = Instant::now();
        let mut responses = Vec::with_capacity(requests.len());

        // Process requests concurrently with controlled parallelism
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10)); // Limit concurrent requests
        let mut tasks = Vec::new();

        for request in requests {
            let permit = semaphore.clone().acquire_owned().await;
            let bridge = self.clone();

            let task = tokio::spawn(async move {
                let _permit = permit;
                bridge.forward_request(request).await
            });

            tasks.push(task);
        }

        // Collect results maintaining order
        for task in tasks {
            match task.await {
                Ok(response) => responses.push(response),
                Err(join_error) => {
                    error!("Task join error in batch processing: {}", join_error);
                    responses.push(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32603,
                            "message": "Internal error during batch processing"
                        }
                    }));
                }
            }
        }

        let duration = start_time.elapsed();
        debug!(
            "Completed batch of {} requests in {:?}",
            responses.len(),
            duration
        );

        responses
    }

    /// Forward request with retry logic
    #[allow(dead_code)]
    pub async fn forward_request_with_retry(
        &self,
        json_rpc_request: Value,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Value {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                debug!("Retrying request (attempt {})", attempt + 1);
                tokio::time::sleep(retry_delay).await;
            }

            match self.send_request(json_rpc_request.clone()).await {
                Ok(response) => {
                    if attempt > 0 {
                        debug!("Request succeeded on retry attempt {}", attempt + 1);
                    }
                    return response;
                }
                Err(error) => {
                    let error_string = error.to_string();

                    // Don't retry on client errors (4xx status codes)
                    if error_string.contains("400")
                        || error_string.contains("401")
                        || error_string.contains("403")
                        || error_string.contains("404")
                    {
                        last_error = Some(error);
                        break;
                    }

                    last_error = Some(error);
                }
            }
        }

        let final_error = last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Request failed after {} attempts", max_retries + 1)
        });

        error!(
            "Request failed after {} attempts: {}",
            max_retries + 1,
            final_error
        );

        self.create_error_response(&json_rpc_request, final_error)
    }

    /// Forward request with timeout override
    #[allow(dead_code)]
    pub async fn forward_request_with_timeout(
        &self,
        json_rpc_request: Value,
        timeout: Duration,
    ) -> Value {
        debug!("Forwarding request with custom timeout: {:?}", timeout);

        let request_future = self.send_request(json_rpc_request.clone());

        match tokio::time::timeout(timeout, request_future).await {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                error!("Request failed: {}", error);
                self.create_error_response(&json_rpc_request, error)
            }
            Err(_timeout_error) => {
                error!("Request timed out after {:?}", timeout);
                let timeout_error = anyhow::anyhow!("Request timed out after {:?}", timeout);
                self.create_error_response(&json_rpc_request, timeout_error)
            }
        }
    }

    /// Stream responses for long-running requests via SSE
    ///
    /// Establishes an SSE connection to the MCP server and streams JSON-RPC
    /// responses back through the callback as they arrive.
    #[allow(dead_code)]
    pub async fn forward_streaming_request(
        &self,
        json_rpc_request: Value,
        response_callback: impl Fn(Value) + Send + Sync + 'static,
    ) -> Result<()> {
        debug!("Forwarding streaming request with SSE");
        
        // Serialize the JSON-RPC request
        let request_body = serde_json::to_string(&json_rpc_request)
            .context("Failed to serialize JSON-RPC request")?;
        
        // Make SSE-enabled POST request
        let response = self
            .client
            .post(&self.mcp_server_url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream") // Request SSE stream
            .body(request_body)
            .send()
            .await
            .context("Failed to send streaming request to MCP server")?;
        
        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            return Err(anyhow::anyhow!(
                "SSE request failed with status {}: {}",
                status,
                error_body
            ));
        }
        
        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        
        // Track stream statistics
        let mut events_processed: usize = 0;
        let mut events_dropped: usize = 0;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read chunk from SSE stream")?;
            buffer.extend_from_slice(&chunk);
            
            // Check buffer size to prevent DoS attacks
            if buffer.len() > MAX_SSE_EVENT_SIZE {
                return Err(anyhow::anyhow!(
                    "SSE event exceeded maximum size of {} bytes ({} MB). \
                     This may indicate a malicious server or a malformed event stream.",
                    MAX_SSE_EVENT_SIZE,
                    MAX_SSE_EVENT_SIZE / (1024 * 1024)
                ));
            }
            
            // Parse all complete events from buffer
            while let Some(event) = Self::parse_sse_event(&mut buffer)? {
                // Skip ping and endpoint events, process message events
                match event.event_type {
                    Some(super::super::events::EventType::Message) | None => {
                        // Parse JSON-RPC response from data field
                        match serde_json::from_str::<Value>(&event.data) {
                            Ok(json_response) => {
                                response_callback(json_response);
                                events_processed += 1;
                            }
                            Err(e) => {
                                events_dropped += 1;
                                warn!(
                                    "Failed to parse JSON-RPC from SSE event: {}. Data: {}",
                                    e, event.data
                                );
                            }
                        }
                    }
                    Some(super::super::events::EventType::Ping) => {
                        debug!("Received SSE ping event");
                    }
                    Some(super::super::events::EventType::Error) => {
                        error!("Received SSE error event: {}", event.data);
                        return Err(anyhow::anyhow!(
                            "SSE stream terminated by server error: {}",
                            event.data
                        ));
                    }
                    Some(super::super::events::EventType::Endpoint) => {
                        debug!("Received SSE endpoint event: {}", event.data);
                    }
                }
            }
        }
        
        debug!(
            "SSE stream completed. Processed {} events, dropped {} events",
            events_processed,
            events_dropped
        );
        Ok(())
    }

    /// Get forwarding statistics
    #[allow(dead_code)]
    pub fn get_forwarding_stats(&self) -> ForwardingStats {
        let stats = self.get_connection_stats();
        
        // Calculate average response time (avoid division by zero)
        let average_response_time_ms = if stats.successful_requests > 0 {
            stats.total_response_time_ms as f64 / stats.successful_requests as f64
        } else {
            0.0
        };
        
        ForwardingStats {
            total_requests: stats.total_requests,
            successful_requests: stats.successful_requests,
            failed_requests: stats.failed_requests,
            average_response_time_ms,
            last_request_time: stats.last_request_time,
        }
    }

    /// Process response and apply transformations if needed
    #[allow(dead_code)]
    pub fn process_response(&self, response: Value, original_request: &Value) -> Value {
        // Apply any necessary response transformations
        let mut processed_response = response;

        // Ensure response has proper JSON-RPC structure
        if !processed_response.is_object() {
            warn!("Response is not a JSON object, wrapping in error response");
            return self.create_error_response(
                original_request,
                anyhow::anyhow!("Invalid response format from MCP server"),
            );
        }

        // Ensure jsonrpc field is present
        if processed_response.get("jsonrpc").is_none() {
            if let Some(obj) = processed_response.as_object_mut() {
                obj.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
            }
        }

        // Ensure id field matches request
        if let Some(request_id) = original_request.get("id") {
            if let Some(obj) = processed_response.as_object_mut() {
                obj.insert("id".to_string(), request_id.clone());
            }
        }

        processed_response
    }

    /// Handle connection errors with appropriate fallback
    #[allow(dead_code)]
    pub async fn handle_connection_error(
        &self,
        error: &anyhow::Error,
        request: &Value,
    ) -> Option<Value> {
        let error_msg = error.to_string();

        // Check for specific connection issues
        if error_msg.contains("Connection refused") {
            warn!("MCP server connection refused, server may be down");

            // Attempt health check to confirm server status
            if !self.health_check().await.unwrap_or(false) {
                return Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request.get("id").cloned().unwrap_or(Value::Null),
                    "error": {
                        "code": -32001,
                        "message": "MCP server is unavailable",
                        "data": {
                            "server_url": self.mcp_server_url,
                            "error_type": "connection_refused"
                        }
                    }
                }));
            }
        } else if error_msg.contains("timeout") {
            warn!("Request timed out, server may be overloaded");

            return Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(Value::Null),
                "error": {
                    "code": -32002,
                    "message": "Request timed out",
                    "data": {
                        "timeout_ms": self.timeout.as_millis(),
                        "error_type": "timeout"
                    }
                }
            }));
        }

        None
    }

    /// Validate response structure
    #[allow(dead_code)]
    pub fn validate_response(&self, response: &Value) -> Result<()> {
        if !response.is_object() {
            return Err(anyhow::anyhow!("Response must be a JSON object"));
        }

        let obj = response.as_object().unwrap();

        // Check for required jsonrpc field
        match obj.get("jsonrpc") {
            Some(Value::String(version)) if version == "2.0" => {}
            Some(_) => return Err(anyhow::anyhow!("Invalid JSON-RPC version")),
            None => return Err(anyhow::anyhow!("Missing jsonrpc field")),
        }

        // Must have either result or error
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");

        if !has_result && !has_error {
            return Err(anyhow::anyhow!(
                "Response must contain either result or error"
            ));
        }

        if has_result && has_error {
            return Err(anyhow::anyhow!(
                "Response cannot contain both result and error"
            ));
        }

        Ok(())
    }
}

/// Forwarding statistics for monitoring
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForwardingStats {
    /// Total number of requests forwarded
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Timestamp of last request
    pub last_request_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl ForwardingStats {
    /// Calculate success rate as percentage
    #[allow(dead_code)]
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Calculate failure rate as percentage
    #[allow(dead_code)]
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }

    /// Check if performance is acceptable
    #[allow(dead_code)]
    pub fn is_healthy(&self) -> bool {
        self.success_rate() >= 95.0 && self.average_response_time_ms < 1000.0
    }
}
