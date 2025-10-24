//! Configuration persistence system for saving and loading

use serde::{Deserialize, Serialize};

/// Candle persistence event for lock-free tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlePersistenceEvent {
    /// Current timestamp in nanoseconds since UNIX epoch
    pub timestamp_nanos: u64,
    /// Previous timestamp in nanoseconds since UNIX epoch
    pub previous_timestamp_nanos: u64,
    /// Type of persistence operation
    pub persistence_type: CandlePersistenceType,
    /// Whether persistence operation was successful
    pub success: bool,
}

/// Candle type of persistence operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandlePersistenceType {
    /// Manual persistence triggered by user
    Manual,
    /// Automatic persistence via timer
    Auto,
    /// Configuration change triggered persistence
    Change,
    /// System shutdown persistence
    Shutdown,
}

/// Candle configuration persistence settings
#[derive(Debug, Clone)]
pub struct CandleConfigurationPersistence {
    /// Enable automatic persistence
    pub auto_save: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Configuration file path
    pub config_file_path: String,
    /// Backup retention count
    pub backup_retention: u32,
    /// Compression enabled
    pub compression: bool,
    /// Encryption enabled
    pub encryption: bool,
    /// File format (json, yaml, toml, binary)
    pub format: String,
}

impl Default for CandleConfigurationPersistence {
    fn default() -> Self {
        Self {
            auto_save: true,
            auto_save_interval: 300, // 5 minutes
            config_file_path: String::from("chat_config.json"),
            backup_retention: 5,
            compression: true,
            encryption: false,
            format: String::from("json"),
        }
    }
}
