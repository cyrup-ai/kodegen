use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ClientInfo, InitializeResult},
    service::{Peer, RunningService},
    RoleClient,
};
use tokio::time::{timeout, Duration};

pub mod error;
pub mod responses;
pub mod tools;
pub mod transports;

pub use error::ClientError;
pub use transports::{create_sse_client, create_streamable_client};

/// Default timeout for MCP operations (30 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Cheap-to-clone client handle for MCP operations
///
/// This handle can be cloned freely and shared across tasks/threads.
/// All MCP operations (call_tool, list_tools, etc.) are available through this handle.
/// Cloning only copies Arc pointers internally, making it very cheap.
#[derive(Clone)]
pub struct KodegenClient {
    peer: Peer<RoleClient>,
    default_timeout: Duration,
}

impl KodegenClient {
    /// Create a client from a peer (internal use)
    pub(crate) fn from_peer(peer: Peer<RoleClient>) -> Self {
        Self {
            peer,
            default_timeout: DEFAULT_TIMEOUT,
        }
    }
    
    /// Configure custom timeout for all operations
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (client, _conn) = create_sse_client(url).await?;
    /// let client = client.with_timeout(Duration::from_secs(60));
    /// ```
    #[must_use]
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.default_timeout = duration;
        self
    }
    
    /// Get server information
    #[must_use]
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.peer.peer_info()
    }

    /// List all available tools
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Timeout` if the operation exceeds the configured timeout,
    /// or `ClientError::ServiceError` if the MCP request fails.
    pub async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, ClientError> {
        timeout(self.default_timeout, self.peer.list_all_tools())
            .await
            .map_err(|_| ClientError::Timeout(
                format!("list_tools timed out after {}s", self.default_timeout.as_secs())
            ))?
            .map_err(ClientError::from)
    }

    /// Call a tool by name with JSON arguments
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Timeout` if the operation exceeds the configured timeout,
    /// or `ClientError::ServiceError` if the tool call fails or the tool does not exist.
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<CallToolResult, ClientError> {
        let call = self.peer.call_tool(CallToolRequestParam {
            // name.to_string() allocation is required because CallToolRequestParam
            // expects Cow<'static, str>. Cannot use borrowed reference from &str parameter
            // as it doesn't satisfy the 'static lifetime requirement.
            name: name.to_string().into(),
            arguments: match arguments {
                serde_json::Value::Object(map) => Some(map),
                _ => None,
            },
        });
        
        timeout(self.default_timeout, call)
            .await
            .map_err(|_| ClientError::Timeout(
                format!("Tool '{}' timed out after {}s", name, self.default_timeout.as_secs())
            ))?
            .map_err(ClientError::from)
    }
    
    /// Call a tool and deserialize the response to a typed structure
    /// 
    /// This provides type-safe parsing with clear error messages instead of fragile
    /// manual JSON extraction with nested Options. Use this with response types from
    /// the `responses` module for better error handling.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use kodegen_mcp_client::responses::StartCrawlResponse;
    /// 
    /// let response: StartCrawlResponse = client
    ///     .call_tool_typed("start_crawl", json!({...}))
    ///     .await?;
    /// let session_id = response.session_id;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `ClientError::ParseError` if the response cannot be deserialized,
    /// or any error from the underlying `call_tool` method.
    pub async fn call_tool_typed<T>(&self, name: &str, arguments: serde_json::Value) -> Result<T, ClientError>
    where
        T: serde::de::DeserializeOwned,
    {
        let result = self.call_tool(name, arguments).await?;
        
        // Extract text content from response
        let text_content = result.content.first()
            .and_then(|c| c.as_text())
            .ok_or_else(|| ClientError::ParseError(
                format!("No text content in response from tool '{}'", name)
            ))?;
        
        // Deserialize to target type with context
        serde_json::from_str(&text_content.text)
            .map_err(|e| ClientError::ParseError(
                format!("Failed to parse response from tool '{}': {}", name, e)
            ))
    }
}

/// Connection lifecycle manager for MCP client
///
/// Manages the underlying connection and provides graceful shutdown.
/// NOT Clone - only one owner should manage the connection lifecycle.
///
/// The connection should be held as long as you want the MCP connection to remain active.
/// When dropped, the connection will be cancelled automatically.
pub struct KodegenConnection {
    service: RunningService<RoleClient, ClientInfo>,
}

impl KodegenConnection {
    /// Create connection from running service
    ///
    /// This is a low-level constructor for creating a connection from an already-initialized
    /// MCP service. Most users should use the transport functions like `create_sse_client()`
    /// which handle both service creation and connection setup.
    pub fn from_service(service: RunningService<RoleClient, ClientInfo>) -> Self {
        Self { service }
    }
    
    /// Get a clone-able client handle for MCP operations
    ///
    /// This creates a lightweight client handle that can be cloned and shared.
    /// Multiple client handles can coexist and all operate on the same underlying connection.
    pub fn client(&self) -> KodegenClient {
        KodegenClient::from_peer(self.service.peer().clone())
    }
    
    /// Graceful shutdown with proper MCP protocol cancellation
    ///
    /// Consumes the connection and performs a clean shutdown of the MCP protocol.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the service cancellation fails.
    pub async fn close(self) -> Result<(), ClientError> {
        self.service.cancel()
            .await
            .map(|_| ())
            .map_err(ClientError::from)
    }
    
    /// Wait for the connection to close naturally
    ///
    /// This will block until the remote side closes the connection or an error occurs.
    /// Useful for long-lived connections where you want to keep the connection alive
    /// until the remote end terminates it.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the connection fails or closes with an error.
    pub async fn wait(self) -> Result<(), ClientError> {
        self.service.waiting()
            .await
            .map(|_| ())
            .map_err(ClientError::from)
    }
}
