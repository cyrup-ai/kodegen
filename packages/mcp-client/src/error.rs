use rmcp::service::ClientInitializeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("MCP protocol error: {0}")]
    Protocol(#[from] rmcp::RmcpError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Client initialization error: {0}")]
    InitError(#[from] ClientInitializeError),

    #[error("Service error: {0}")]
    ServiceError(#[from] rmcp::ServiceError),

    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Operation timed out: {0}")]
    Timeout(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Connection error: {0}")]
    Connection(String),
}
