// packages/mcp-client/src/transports/sse.rs
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::{SseClientTransport, StreamableHttpClientTransport},
};
use crate::{KodegenClient, ClientError};

/// Create an SSE client from a URL
/// 
/// # Example
/// ```ignore
/// let client = create_sse_client("http://localhost:8080/sse").await?;
/// ```
pub async fn create_sse_client(url: &str) -> Result<KodegenClient, ClientError> {
    // SseClientTransport requires async start
    let transport = SseClientTransport::start(url.to_owned())
        .await
        .map_err(|e| ClientError::Connection(format!("Failed to connect to SSE endpoint: {}", e)))?;
    
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "kodegen-sse-client".to_string(),
            title: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            website_url: None,
            icons: None,
        },
    };
    
    // Use () as the client type for SSE (no custom client needed)
    let service = client_info.serve(transport)
        .await
        .map_err(ClientError::InitError)?;
    
    Ok(KodegenClient::from_service(service))
}

/// Create a Streamable HTTP client from a URL
/// 
/// # Example
/// ```ignore
/// let client = create_streamable_client("http://localhost:8000/mcp").await?;
/// ```
pub async fn create_streamable_client(url: &str) -> Result<KodegenClient, ClientError> {
    // StreamableHttpClientTransport has simpler constructor
    let transport = StreamableHttpClientTransport::from_uri(url);
    
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "kodegen-streamable-client".to_string(),
            title: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            website_url: None,
            icons: None,
        },
    };
    
    let service = client_info.serve(transport)
        .await
        .map_err(ClientError::InitError)?;
    
    Ok(KodegenClient::from_service(service))
}
