use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Top‑level daemon configuration (mirrors original defaults).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub services_dir: Option<String>,
    pub log_dir: Option<String>,
    pub default_user: Option<String>,
    pub default_group: Option<String>,
    pub auto_restart: Option<bool>,
    pub services: Vec<ServiceDefinition>,
    pub sse: Option<SseServerConfig>,
    /// MCP Streamable HTTP transport binding (host:port)
    pub mcp_bind: Option<String>,
    /// Kodegen SSE server configuration
    #[serde(default)]
    pub kodegen_sse: KodegenSseConfig,
}

/// SSE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseServerConfig {
    /// Enable SSE server
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Port to bind SSE server to
    #[serde(default = "default_sse_port")]
    pub port: u16,
    /// MCP server URL to bridge requests to
    #[serde(default = "default_mcp_server_url")]
    pub mcp_server_url: String,
    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Ping interval for keep-alive (seconds)
    #[serde(default = "default_ping_interval")]
    pub ping_interval: u64,
    /// Session timeout (seconds)
    #[serde(default = "default_session_timeout")]
    pub session_timeout: u64,
    /// CORS allowed origins
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
}

fn default_true() -> bool {
    true
}
fn default_sse_port() -> u16 {
    30436
}
fn default_kodegen_port() -> u16 {
    30437
}
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}
fn default_mcp_server_url() -> String {
    "http://127.0.0.1:3000".to_string()
}
fn default_max_connections() -> usize {
    100
}
fn default_ping_interval() -> u64 {
    30
}
fn default_session_timeout() -> u64 {
    300
}
fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

impl Default for SseServerConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            port: default_sse_port(),
            mcp_server_url: default_mcp_server_url(),
            max_connections: default_max_connections(),
            ping_interval: default_ping_interval(),
            session_timeout: default_session_timeout(),
            cors_origins: default_cors_origins(),
        }
    }
}

/// Kodegen SSE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KodegenSseConfig {
    /// Enable kodegen SSE server
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Port for kodegen SSE server
    #[serde(default = "default_kodegen_port")]
    pub port: u16,

    /// Bind address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Tool categories to enable
    #[serde(default)]
    pub enabled_tools: Option<Vec<String>>,

    /// TLS certificate path (PEM format)
    /// If not specified, auto-discovered from standard install locations
    #[serde(default)]
    pub tls_cert: Option<std::path::PathBuf>,

    /// TLS private key path (PEM format)
    /// If not specified, auto-discovered from standard install locations
    #[serde(default)]
    pub tls_key: Option<std::path::PathBuf>,
}

impl Default for KodegenSseConfig {
    fn default() -> Self {
        let (tls_cert, tls_key) = discover_certificate_paths();
        Self {
            enabled: true,
            port: 30437,
            bind_address: "127.0.0.1".to_string(),
            enabled_tools: None,
            tls_cert,
            tls_key,
        }
    }
}

/// Discover certificate paths from standard installation locations
/// Checks system-wide and user-level install directories
fn discover_certificate_paths() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    use std::path::PathBuf;

    // Standard certificate file names
    const CERT_FILE: &str = "server.crt";
    const KEY_FILE: &str = "server.key";

    // Build search paths using conditional compilation
    #[cfg(target_os = "macos")]
    let search_paths = vec![
        PathBuf::from("/usr/local/var/kodegen/certs"),
        dirs::data_local_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")))
            .join("kodegen")
            .join("certs"),
    ];

    #[cfg(target_os = "linux")]
    let search_paths = vec![
        PathBuf::from("/var/lib/kodegen/certs"),
        dirs::data_local_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".local")
                    .join("share")
            })
            .join("kodegen")
            .join("certs"),
    ];

    #[cfg(target_os = "windows")]
    let search_paths = vec![
        PathBuf::from("C:\\ProgramData\\Kodegen\\certs"),
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\temp"))
            .join("Kodegen")
            .join("certs"),
    ];

    // Search for certificates in priority order
    for cert_dir in search_paths {
        let cert_path = cert_dir.join(CERT_FILE);
        let key_path = cert_dir.join(KEY_FILE);

        // Check if both certificate and key exist
        if cert_path.exists() && key_path.exists() {
            log::info!(
                "Auto-discovered TLS certificates at: cert={}, key={}",
                cert_path.display(),
                key_path.display()
            );
            return (Some(cert_path), Some(key_path));
        }
    }

    // No certificates found - will run in HTTP mode
    log::info!("No TLS certificates found in standard locations, HTTPS will not be available");
    log::debug!("To enable HTTPS, ensure certificates exist at one of the standard paths");
    (None, None)
}

impl From<SseServerConfig> for crate::service::sse::SseConfig {
    fn from(config: SseServerConfig) -> Self {
        Self {
            port: config.port,
            mcp_server_url: config.mcp_server_url,
            max_connections: config.max_connections,
            ping_interval: config.ping_interval,
            session_timeout: config.session_timeout,
            cors_origins: config.cors_origins,
            // Use defaults for MCP bridge configuration
            mcp_timeout: 30,
            mcp_keepalive_timeout: 90,
            mcp_max_idle_connections: 10,
            mcp_user_agent: "Kodegen-Daemon/1.0".to_string(),
            // Use defaults for retry configuration
            mcp_max_retries: 3,
            mcp_retry_delay_ms: 100,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            services_dir: Some("/etc/kodegend/services".into()),
            log_dir: Some("/var/log/kodegend".into()),
            default_user: Some("kodegend".into()),
            default_group: Some("cyops".into()),
            auto_restart: Some(true),
            services: vec![],
            sse: Some(SseServerConfig::default()),
            mcp_bind: Some("0.0.0.0:33399".into()),
            kodegen_sse: KodegenSseConfig::default(),
        }
    }
}

/// On‑disk TOML description of a single service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub auto_restart: bool,
    pub user: Option<String>,
    pub group: Option<String>,
    pub restart_delay_s: Option<u64>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(default)]
    pub log_rotation: Option<LogRotationConfig>,
    #[serde(default)]
    pub watch_dirs: Vec<String>,
    pub ephemeral_dir: Option<String>,
    /// Service type (e.g., "autoconfig" for special handling)
    pub service_type: Option<String>,
    pub memfs: Option<MemoryFsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFsConfig {
    pub size_mb: u32, // clamped at 2048 elsewhere
    pub mount_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_type: String, // http | tcp | script
    pub target: String,
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub retries: u32,
    pub expected_response: Option<String>,
    #[serde(default)]
    pub on_failure: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    pub max_size_mb: u64,
    pub max_files: u32,
    pub interval_days: u32,
    pub compress: bool,
    pub timestamp: bool,
}
