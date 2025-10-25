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
    /// Category SSE servers (14 tool categories)
    #[serde(default)]
    pub category_servers: Vec<CategoryServerConfig>,
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

/// Category SSE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryServerConfig {
    pub name: String,
    pub binary: String,
    pub port: u16,
    #[serde(default = "default_true")]
    pub enabled: bool,
}



/// Discover certificate paths from standard installation locations
/// Checks system-wide and user-level install directories
pub fn discover_certificate_paths() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
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

impl ServiceConfig {
    fn default_category_servers() -> Vec<CategoryServerConfig> {
        vec![
            CategoryServerConfig {
                name: "browser".to_string(),
                binary: "kodegen-browser".to_string(),
                port: 30438,
                enabled: true,
            },
            CategoryServerConfig {
                name: "citescrape".to_string(),
                binary: "kodegen-citescrape".to_string(),
                port: 30439,
                enabled: true,
            },
            CategoryServerConfig {
                name: "claude-agent".to_string(),
                binary: "kodegen-claude-agent".to_string(),
                port: 30440,
                enabled: true,
            },
            CategoryServerConfig {
                name: "config".to_string(),
                binary: "kodegen-config".to_string(),
                port: 30441,
                enabled: true,
            },
            CategoryServerConfig {
                name: "database".to_string(),
                binary: "kodegen-database".to_string(),
                port: 30442,
                enabled: true,
            },
            CategoryServerConfig {
                name: "filesystem".to_string(),
                binary: "kodegen-filesystem".to_string(),
                port: 30443,
                enabled: true,
            },
            CategoryServerConfig {
                name: "git".to_string(),
                binary: "kodegen-git".to_string(),
                port: 30444,
                enabled: true,
            },
            CategoryServerConfig {
                name: "github".to_string(),
                binary: "kodegen-github".to_string(),
                port: 30445,
                enabled: true,
            },
            CategoryServerConfig {
                name: "introspection".to_string(),
                binary: "kodegen-introspection".to_string(),
                port: 30446,
                enabled: true,
            },
            CategoryServerConfig {
                name: "process".to_string(),
                binary: "kodegen-process".to_string(),
                port: 30447,
                enabled: true,
            },
            CategoryServerConfig {
                name: "prompt".to_string(),
                binary: "kodegen-prompt".to_string(),
                port: 30448,
                enabled: true,
            },
            CategoryServerConfig {
                name: "reasoner".to_string(),
                binary: "kodegen-reasoner".to_string(),
                port: 30449,
                enabled: true,
            },
            CategoryServerConfig {
                name: "sequential-thinking".to_string(),
                binary: "kodegen-sequential-thinking".to_string(),
                port: 30450,
                enabled: true,
            },
            CategoryServerConfig {
                name: "terminal".to_string(),
                binary: "kodegen-terminal".to_string(),
                port: 30451,
                enabled: true,
            },
            CategoryServerConfig {
                name: "candle-agent".to_string(),
                binary: "kodegen-candle-agent".to_string(),
                port: 30452,
                enabled: true,
            },
        ]
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
            category_servers: ServiceConfig::default_category_servers(),
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
