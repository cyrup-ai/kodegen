//! Error types for certificate provisioning and setup.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SetupError>;

#[derive(Debug, Error)]
pub enum SetupError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("App Store Connect API error: {0}")]
    AppStoreConnectApi(String),

    #[error("Certificate generation failed: {0}")]
    CertificateGeneration(String),

    #[error("CSR generation failed: {0}")]
    CsrGeneration(String),

    #[error("Keychain operation failed: {0}")]
    KeychainOperation(String),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    #[error("HTTP request failed: {0}")]
    HttpRequest(String),

    #[error("JWT creation failed: {0}")]
    JwtCreation(String),

    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}
