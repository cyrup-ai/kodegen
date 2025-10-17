use rmcp::{
    model::{CallToolRequestParam, CallToolResult, InitializeResult},
    service::RunningService,
};
use tokio::time::{timeout, Duration};

pub mod error;
pub mod tools;

pub use error::ClientError;

/// Default timeout for MCP operations (30 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Generic MCP Client
pub struct KodegenClient {
    client: RunningService<rmcp::RoleClient, ()>,
    default_timeout: Duration,
}

impl KodegenClient {
    /// Create a client from an existing MCP service connection
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the connection fails.
    pub fn from_service(client: RunningService<rmcp::RoleClient, ()>) -> Self {
        Self { 
            client,
            default_timeout: DEFAULT_TIMEOUT,
        }
    }
    
    /// Configure custom timeout for all operations
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = KodegenClient::from_service(service)
    ///     .with_timeout(Duration::from_secs(60));
    /// ```
    #[must_use]
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.default_timeout = duration;
        self
    }
    
    /// Get server information
    #[must_use]
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.client.peer_info()
    }

    /// List all available tools
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Timeout` if the operation exceeds the configured timeout,
    /// or `ClientError::ServiceError` if the MCP request fails.
    pub async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, ClientError> {
        timeout(self.default_timeout, self.client.list_all_tools())
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
        let call = self.client.call_tool(CallToolRequestParam {
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
    
    /// Graceful shutdown with proper MCP protocol cancellation
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Timeout` if the operation exceeds the configured timeout,
    /// or `ClientError` if the service cancellation fails.
    pub async fn close(self) -> Result<(), ClientError> {
        let cancel_future = self.client.cancel();
        
        timeout(self.default_timeout, cancel_future)
            .await
            .map_err(|_| ClientError::Timeout(
                format!("close operation timed out after {}s", self.default_timeout.as_secs())
            ))?
            .map(|_| ())
            .map_err(ClientError::from)
    }
}
