use kodegen_tool::error::McpError;
use crate::system_info::{ClientInfo, ClientRecord, SystemInfo, get_system_info};
use parking_lot::RwLock;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

// ============================================================================
// DEFAULT VALUE FUNCTIONS
// ============================================================================

fn default_fuzzy_search_threshold() -> f64 {
    0.7
}

fn default_sse_connection_timeout_secs() -> u64 {
    5
}

// ============================================================================
// PROFILING INSTRUMENTATION
// ============================================================================

/// Counter for tracking config write frequency
static CONFIG_WRITE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Start time for calculating write rate
static CONFIG_WRITE_START: OnceLock<std::time::Instant> = OnceLock::new();

// ============================================================================
// SERVER CONFIG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Commands that cannot be executed
    pub blocked_commands: Vec<String>,

    /// Default shell for command execution
    pub default_shell: String,

    /// Directories the server can access (empty = full access)
    pub allowed_directories: Vec<String>,

    /// Directories the server cannot access
    pub denied_directories: Vec<String>,

    /// Max lines for file read operations
    pub file_read_line_limit: usize,

    /// Max lines per file write operation
    pub file_write_line_limit: usize,

    /// Minimum similarity ratio (0.0-1.0) for fuzzy search suggestions
    /// Default: 0.7 (70% similarity required)
    #[serde(default = "default_fuzzy_search_threshold")]
    pub fuzzy_search_threshold: f64,

    /// SSE connection timeout in seconds (default: 5)
    #[serde(default = "default_sse_connection_timeout_secs")]
    pub sse_connection_timeout_secs: u64,

    /// Currently connected client (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_client: Option<ClientInfo>,

    /// History of all clients that have connected
    #[serde(default)]
    pub client_history: Vec<ClientRecord>,
    
    /// System diagnostic information (populated on every `get_config` call)
    pub system_info: SystemInfo,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            blocked_commands: vec![
                "rm".to_string(),
                "rmdir".to_string(),
                "del".to_string(),
                "format".to_string(),
                "dd".to_string(),
                "shred".to_string(),
                "sudo".to_string(),
                "su".to_string(),
                "passwd".to_string(),
                "useradd".to_string(),
                "userdel".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
            ],
            default_shell: if cfg!(windows) {
                "powershell.exe".to_string()
            } else {
                "/bin/sh".to_string()
            },
            allowed_directories: Vec::new(),
            denied_directories: Vec::new(),
            file_read_line_limit: 1000,
            file_write_line_limit: 50,
            fuzzy_search_threshold: 0.7,
            current_client: None,
            client_history: Vec::new(),
            system_info: get_system_info(),
        }
    }
}

// ============================================================================
// ENVIRONMENT VARIABLE LOADING
// ============================================================================

/// Load allowed directories from `KODEGEN_ALLOWED_DIRS` environment variable
/// Format: Colon-separated on Unix/macOS, semicolon-separated on Windows
fn load_allowed_dirs_from_env() -> Vec<String> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    
    std::env::var("KODEGEN_ALLOWED_DIRS")
        .ok()
        .map(|dirs| dirs
            .split(separator)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
        .unwrap_or_default()
}

/// Load denied directories from `KODEGEN_DENIED_DIRS` environment variable
/// Format: Colon-separated on Unix/macOS, semicolon-separated on Windows
fn load_denied_dirs_from_env() -> Vec<String> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    
    std::env::var("KODEGEN_DENIED_DIRS")
        .ok()
        .map(|dirs| dirs
            .split(separator)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
        .unwrap_or_default()
}

// ============================================================================
// CONFIG VALUE ENUM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<String>),
}

impl ConfigValue {
    /// Convert to string value
    ///
    /// # Errors
    /// Returns error if the value is not a string
    pub fn into_string(self) -> Result<String, McpError> {
        match self {
            ConfigValue::String(s) => Ok(s),
            _ => Err(McpError::InvalidArguments("Expected string".to_string())),
        }
    }

    /// Convert to number value
    ///
    /// # Errors
    /// Returns error if the value is not a number
    pub fn into_number(self) -> Result<i64, McpError> {
        match self {
            ConfigValue::Number(n) => Ok(n),
            _ => Err(McpError::InvalidArguments("Expected number".to_string())),
        }
    }

    /// Convert to boolean value
    ///
    /// # Errors
    /// Returns error if the value is not a boolean
    pub fn into_boolean(self) -> Result<bool, McpError> {
        match self {
            ConfigValue::Boolean(b) => Ok(b),
            _ => Err(McpError::InvalidArguments("Expected boolean".to_string())),
        }
    }

    /// Convert to array value
    ///
    /// # Errors
    /// Returns error if the value is not an array or valid JSON array string
    pub fn into_array(self) -> Result<Vec<String>, McpError> {
        match self {
            ConfigValue::Array(arr) => Ok(arr),
            ConfigValue::String(s) if s.starts_with('[') => {
                serde_json::from_str(&s)
                    .map_err(|_| McpError::InvalidArguments("Invalid array format".to_string()))
            }
            _ => Err(McpError::InvalidArguments("Expected array".to_string())),
        }
    }
}

// ============================================================================
// CONFIG MANAGER
// ============================================================================

#[derive(Clone)]
pub struct ConfigManager {
    config: Arc<RwLock<ServerConfig>>,
    config_path: PathBuf,
    
    // Debouncing field for fire-and-forget saves
    save_sender: tokio::sync::mpsc::UnboundedSender<()>,
}

impl ConfigManager {
    #[must_use]
    pub fn new() -> Self {
        let config_dir = match dirs::home_dir() {
            Some(home) => home.join(".kodegen"),
            None => PathBuf::from(".kodegen"),
        };
        let config_path = config_dir.join("config.json");
        
        // Create channel for debounced saves
        let (save_sender, save_receiver) = tokio::sync::mpsc::unbounded_channel();
        
        let manager = Self {
            config: Arc::new(RwLock::new(ServerConfig::default())),
            config_path: config_path.clone(),
            save_sender,
        };
        
        // Start background saver task
        manager.start_background_saver(save_receiver);
        
        manager
    }

    /// Initialize configuration from disk and environment variables
    ///
    /// # Errors
    /// Returns error if config directory cannot be created or config file cannot be read/written
    pub async fn init(&self) -> Result<(), McpError> {
        if let Some(config_dir) = self.config_path.parent() {
            tokio::fs::create_dir_all(config_dir).await?;
        }
        
        // Load from disk or use defaults
        let mut loaded_config = match tokio::fs::read_to_string(&self.config_path).await {
            Ok(content) => serde_json::from_str::<ServerConfig>(&content)?,
            Err(_) => ServerConfig::default(),
        };
        
        // OVERRIDE with environment variables (for security)
        let env_allowed = load_allowed_dirs_from_env();
        let env_denied = load_denied_dirs_from_env();
        
        if !env_allowed.is_empty() {
            loaded_config.allowed_directories = env_allowed;
            log::info!("Loaded {} allowed directories from KODEGEN_ALLOWED_DIRS", 
                loaded_config.allowed_directories.len());
        }
        
        if !env_denied.is_empty() {
            loaded_config.denied_directories = env_denied;
            log::info!("Loaded {} denied directories from KODEGEN_DENIED_DIRS", 
                loaded_config.denied_directories.len());
        }
        
        *self.config.write() = loaded_config;
        self.save_to_disk().await?;
        Ok(())
    }

    #[must_use]
    pub fn get_config(&self) -> ServerConfig {
        self.config.read().clone()
    }

    #[must_use]
    pub fn get_file_read_line_limit(&self) -> usize {
        self.config.read().file_read_line_limit
    }

    #[must_use]
    pub fn get_file_write_line_limit(&self) -> usize {
        self.config.read().file_write_line_limit
    }

    #[must_use]
    pub fn get_blocked_commands(&self) -> Vec<String> {
        self.config.read().blocked_commands.clone()
    }

    #[must_use]
    pub fn get_fuzzy_search_threshold(&self) -> f64 {
        self.config.read().fuzzy_search_threshold
    }

    #[must_use]
    pub fn get_value(&self, key: &str) -> Option<ConfigValue> {
        let config = self.config.read();
        match key {
            "blocked_commands" => Some(ConfigValue::Array(config.blocked_commands.clone())),
            "default_shell" => Some(ConfigValue::String(config.default_shell.clone())),
            "allowed_directories" => Some(ConfigValue::Array(config.allowed_directories.clone())),
            "denied_directories" => Some(ConfigValue::Array(config.denied_directories.clone())),
            "file_read_line_limit" => Some(ConfigValue::Number(i64::try_from(config.file_read_line_limit).unwrap_or(i64::MAX))),
            "file_write_line_limit" => Some(ConfigValue::Number(i64::try_from(config.file_write_line_limit).unwrap_or(i64::MAX))),
            "fuzzy_search_threshold" => Some(ConfigValue::Number(
                (config.fuzzy_search_threshold * 100.0) as i64
            )),
            _ => None,
        }
    }

    /// Set a configuration value by key
    ///
    /// # Errors
    /// Returns error if the key is unknown, value type is invalid, or config cannot be saved
    pub async fn set_value(&self, key: &str, value: ConfigValue) -> Result<(), McpError> {
        {
            let mut config = self.config.write();
            match key {
                "blocked_commands" => {
                    config.blocked_commands = value.into_array()?;
                }
                "default_shell" => {
                    config.default_shell = value.into_string()?;
                }
                "allowed_directories" => {
                    config.allowed_directories = value.into_array()?;
                }
                "denied_directories" => {
                    config.denied_directories = value.into_array()?;
                }
                "file_read_line_limit" => {
                    let num = value.into_number()?;
                    if num <= 0 {
                        return Err(McpError::InvalidArguments(
                            "file_read_line_limit must be positive".to_string()
                        ));
                    }
                    config.file_read_line_limit = usize::try_from(num)
                        .map_err(|_| McpError::InvalidArguments(
                            "file_read_line_limit value out of range".to_string()
                        ))?;
                }
                "file_write_line_limit" => {
                    let num = value.into_number()?;
                    if num <= 0 {
                        return Err(McpError::InvalidArguments(
                            "file_write_line_limit must be positive".to_string()
                        ));
                    }
                    config.file_write_line_limit = usize::try_from(num)
                        .map_err(|_| McpError::InvalidArguments(
                            "file_write_line_limit value out of range".to_string()
                        ))?;
                }
                "fuzzy_search_threshold" => {
                    let num = value.into_number()?;
                    if !(0..=100).contains(&num) {
                        return Err(McpError::InvalidArguments(
                            "fuzzy_search_threshold must be between 0 and 100".to_string()
                        ));
                    }
                    config.fuzzy_search_threshold = (num as f64) / 100.0;
                }
                _ => {
                    return Err(McpError::InvalidArguments(
                        format!("Unknown config key: {key}")
                    ));
                }
            }
        }
        
        // Fire-and-forget debounced save
        let _ = self.save_sender.send(());
        Ok(())
    }

    async fn save_to_disk(&self) -> Result<(), McpError> {
        // Profiling instrumentation
        let start_time = CONFIG_WRITE_START.get_or_init(std::time::Instant::now);
        let count = CONFIG_WRITE_COUNT.fetch_add(1, Ordering::Relaxed);
        
        if count.is_multiple_of(10) {
            let elapsed = start_time.elapsed().as_secs();
            let rate = if elapsed > 0 { 
                f64::from(u32::try_from(count).unwrap_or(u32::MAX)) / elapsed as f64 * 60.0 
            } else { 
                0.0 
            };
            log::info!("Config writes: {} total ({:.2}/min)", count, rate);
        }
        
        // Existing save logic
        let json = {
            let config = self.config.read();
            serde_json::to_string_pretty(&*config)?
        };
        tokio::fs::write(&self.config_path, json).await?;
        Ok(())
    }

    /// Background task that debounces config saves
    /// 
    /// Pattern copied from packages/utils/src/usage_tracker.rs:154-234
    fn start_background_saver(
        &self,
        mut save_receiver: tokio::sync::mpsc::UnboundedReceiver<()>
    ) {
        let config = Arc::clone(&self.config);
        let config_path = self.config_path.clone();
        
        tokio::spawn(async move {
            // Debounce: wait 300ms after last change
            const DEBOUNCE_MS: u64 = 300;
            
            let mut has_pending_save = false;
            let mut last_save_request = std::time::Instant::now();
            
            loop {
                tokio::select! {
                    // Receive save request from set_value() or set_client_info()
                    Some(()) = save_receiver.recv() => {
                        has_pending_save = true;
                        last_save_request = std::time::Instant::now();
                    }
                    
                    // Check every 100ms if debounce period has passed
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        if has_pending_save && last_save_request.elapsed().as_millis() >= DEBOUNCE_MS as u128 {
                            // Perform batched save
                            let json = {
                                let cfg = config.read();
                                match serde_json::to_string_pretty(&*cfg) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        log::error!("Failed to serialize config: {}", e);
                                        continue;
                                    }
                                }
                            };
                            
                            if let Err(e) = tokio::fs::write(&config_path, json).await {
                                log::error!("Failed to save config: {}", e);
                            }
                            
                            has_pending_save = false;
                        }
                    }
                    
                    // Channel closed (server shutdown)
                    else => {
                        // Final flush before exit
                        if has_pending_save {
                            let json = {
                                let cfg = config.read();
                                serde_json::to_string_pretty(&*cfg).unwrap_or_default()
                            };
                            let _ = tokio::fs::write(&config_path, json).await;
                        }
                        break;
                    }
                }
            }
        });
    }

    /// Store client information from MCP initialization
    ///
    /// # Errors
    /// Returns error if config cannot be saved to disk
    pub async fn set_client_info(&self, client_info: ClientInfo) -> Result<(), McpError> {
        {
            let mut config = self.config.write();
            let now = chrono::Utc::now();
            
            // Update or create client history record
            let existing = config.client_history
                .iter_mut()
                .find(|r| r.client_info.name == client_info.name 
                       && r.client_info.version == client_info.version);
            
            if let Some(record) = existing {
                // Update existing record's last_seen timestamp
                record.last_seen = now;
            } else {
                // Add new client record
                config.client_history.push(ClientRecord {
                    client_info: client_info.clone(),
                    connected_at: now,
                    last_seen: now,
                });
            }
            
            // Set as current client
            config.current_client = Some(client_info);
        }
        
        // Fire-and-forget debounced save
        let _ = self.save_sender.send(());
        Ok(())
    }
    
    /// Get current client information
    #[must_use]
    pub fn get_client_info(&self) -> Option<ClientInfo> {
        self.config.read().current_client.clone()
    }
    
    /// Get client connection history
    #[must_use]
    pub fn get_client_history(&self) -> Vec<ClientRecord> {
        self.config.read().client_history.clone()
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
