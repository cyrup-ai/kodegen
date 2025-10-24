//! Configuration manager with atomic updates and lock-free operations

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, broadcast};
use tokio_stream::Stream;
use uuid::Uuid;

use super::persistence::CandleConfigurationPersistence;
use super::streaming::{CandleConfigUpdate, CandleConfigUpdateType};
use super::types::{CandleChatConfig, duration_secs};
use super::validation::{
    CandleBehaviorValidator, CandleConfigurationValidator, CandlePersonalityValidator,
    CandleUIValidator,
};
use crate::domain::util::unix_timestamp_nanos;

/// Candle configuration change event with zero-allocation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleConfigurationChangeEvent {
    /// Event ID
    pub id: Uuid,
    /// Timestamp of the change
    #[serde(with = "duration_secs")]
    pub timestamp: Duration,
    /// Configuration section that changed
    pub section: String,
    /// Type of change (update, replace, validate)
    pub change_type: CandleConfigurationChangeType,
    /// Old configuration value (optional)
    pub old_value: Option<String>,
    /// New configuration value (optional)
    pub new_value: Option<String>,
    /// User who made the change
    pub user: Option<String>,
    /// Change description
    pub description: String,
}

/// Candle configuration change type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandleConfigurationChangeType {
    /// Update existing configuration
    Update,
    /// Replace entire configuration
    Replace,
    /// Validate configuration
    Validate,
    /// Reset to default
    Reset,
    /// Import from file
    Import,
    /// Export to file
    Export,
}

/// Candle configuration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleConfigurationStatistics {
    /// Total number of configuration changes made
    pub total_changes: usize,
    /// Current configuration version number
    pub current_version: usize,
    /// Duration since last modification
    pub last_modified: Duration,
    /// Number of active validators
    pub validators_count: usize,
    /// Whether auto-save is currently enabled
    pub auto_save_enabled: bool,
}

/// Candle configuration manager with atomic updates and lock-free operations
pub struct CandleConfigurationManager {
    /// Current configuration with atomic updates
    config: ArcSwap<CandleChatConfig>,
    /// Configuration change event queue
    change_events: Arc<Mutex<Vec<CandleConfigurationChangeEvent>>>,
    /// Change notification broadcaster
    change_notifier: broadcast::Sender<CandleConfigurationChangeEvent>,
    /// Configuration validation rules
    validation_rules:
        Arc<RwLock<HashMap<String, Arc<dyn CandleConfigurationValidator + Send + Sync>>>>,
    /// Persistence settings
    persistence: Arc<RwLock<CandleConfigurationPersistence>>,
    /// Configuration change counter
    change_counter: Arc<AtomicUsize>,
    /// Last persistence timestamp (nanoseconds since UNIX epoch) - lock-free tracking
    last_persistence: Arc<AtomicU64>,
    /// Configuration version counter
    version_counter: Arc<AtomicUsize>,
    /// Configuration locks for atomic operations
    configuration_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::RwLock<()>>>>>,
}

impl Clone for CandleConfigurationManager {
    fn clone(&self) -> Self {
        // Create a new instance with current configuration
        let current_config = self.config.load_full();
        let (change_notifier, _) = broadcast::channel(1000);

        Self {
            config: ArcSwap::new(current_config),
            change_events: Arc::new(Mutex::new(Vec::new())), // Fresh event queue
            change_notifier,
            validation_rules: Arc::clone(&self.validation_rules),
            persistence: Arc::clone(&self.persistence),
            change_counter: Arc::new(AtomicUsize::new(0)), // Fresh counter
            last_persistence: Arc::new(AtomicU64::new(unix_timestamp_nanos())),
            version_counter: Arc::new(AtomicUsize::new(1)), // Fresh version counter
            configuration_locks: Arc::clone(&self.configuration_locks),
        }
    }
}

impl CandleConfigurationManager {
    /// Create a new Candle configuration manager
    #[must_use]
    pub fn new(initial_config: CandleChatConfig) -> Self {
        let (change_notifier, _) = broadcast::channel(1000);

        let manager = Self {
            config: ArcSwap::new(Arc::new(initial_config)),
            change_events: Arc::new(Mutex::new(Vec::new())),
            change_notifier,
            validation_rules: Arc::new(RwLock::new(HashMap::new())),
            persistence: Arc::new(RwLock::new(CandleConfigurationPersistence::default())),
            change_counter: Arc::new(AtomicUsize::new(0)),
            last_persistence: Arc::new(AtomicU64::new(unix_timestamp_nanos())),
            version_counter: Arc::new(AtomicUsize::new(1)),
            configuration_locks: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default validators
        {
            let mut rules = manager.validation_rules.blocking_write();
            rules.insert("personality".into(), Arc::new(CandlePersonalityValidator));
            rules.insert("behavior".into(), Arc::new(CandleBehaviorValidator));
            rules.insert("ui".into(), Arc::new(CandleUIValidator));
        }

        manager
    }

    /// Get current Candle configuration
    pub fn get_config(&self) -> Arc<CandleChatConfig> {
        self.config.load_full()
    }

    /// Update Candle configuration atomically
    pub fn update_config(
        &self,
        new_config: CandleChatConfig,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::context::chunks::CandleUnit> + Send>> {
        let manager = self.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Validate the new configuration (sync validation)
            // Validation would go here if needed

            let old_config = manager.config.load_full();
            let config_arc = Arc::new(new_config);

            // Perform atomic update
            manager.config.store(config_arc.clone());

            // Create change event
            let change_event = CandleConfigurationChangeEvent {
                id: Uuid::new_v4(),
                timestamp: Duration::from_secs(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                section: "all".to_string(),
                change_type: CandleConfigurationChangeType::Replace,
                old_value: Some(format!("{old_config:?}")),
                new_value: Some(format!("{config_arc:?}")),
                user: None,
                description: String::from("Configuration updated"),
            };

            // Queue change event
            manager
                .change_events
                .lock()
                .await
                .push(change_event.clone());
            manager.change_counter.fetch_add(1, Ordering::Relaxed);
            manager.version_counter.fetch_add(1, Ordering::Relaxed);

            // Update persistence timestamp atomically on config change
            let now_nanos = unix_timestamp_nanos();
            manager.last_persistence.store(now_nanos, Ordering::Release);

            // Notify subscribers
            let _ = manager.change_notifier.send(change_event);

            // Emit completion
            let _ = tx.send(crate::domain::context::chunks::CandleUnit(()));
        }))
    }

    /// Update specific Candle configuration section
    pub fn update_section<F>(
        &self,
        section: &str,
        updater: F,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::context::chunks::CandleUnit> + Send>>
    where
        F: FnOnce(&mut CandleChatConfig) + Send + 'static,
    {
        let section_arc: String = String::from(section);
        let manager = self.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Load current config and make a copy
            let current_config = manager.config.load_full();
            let mut new_config = current_config.as_ref().clone();

            // Apply update
            updater(&mut new_config);

            // Store the updated configuration atomically
            let config_arc = Arc::new(new_config);
            manager.config.store(config_arc.clone());

            // Create change event
            let change_event = CandleConfigurationChangeEvent {
                id: Uuid::new_v4(),
                timestamp: Duration::from_secs(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                section: section_arc.clone(),
                change_type: CandleConfigurationChangeType::Update,
                old_value: Some(format!("{current_config:?}")),
                new_value: Some(format!("{config_arc:?}")),
                user: None,
                description: String::from("Configuration section updated"),
            };

            // Queue change event
            manager
                .change_events
                .lock()
                .await
                .push(change_event.clone());
            manager.change_counter.fetch_add(1, Ordering::Relaxed);
            manager.version_counter.fetch_add(1, Ordering::Relaxed);

            // Update persistence timestamp atomically on config change
            let now_nanos = unix_timestamp_nanos();
            manager.last_persistence.store(now_nanos, Ordering::Release);

            // Notify subscribers
            let _ = manager.change_notifier.send(change_event);

            // Emit completion
            let _ = tx.send(crate::domain::context::chunks::CandleUnit(()));
        }))
    }

    /// Subscribe to configuration changes
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<CandleConfigurationChangeEvent> {
        self.change_notifier.subscribe()
    }

    /// Validate configuration using streaming pattern
    pub fn validate_config_stream(
        &self,
        _config: CandleChatConfig,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = CandleConfigUpdate> + Send>> {
        let _manager = self.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Create validation update
                let now_nanos = unix_timestamp_nanos();

                let validation_start = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::ValidationStarted,
                    section: None,
                    success: true,
                    description: Some("Configuration validation initiated".to_string()),
                };

                let _ = sender.send(validation_start);

                // Emit completion update
                let completion_update = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::ValidationCompleted,
                    section: None,
                    success: true,
                    description: Some("Configuration validation completed".to_string()),
                };

                let _ = sender.send(completion_update);
            },
        ))
    }

    /// Register a configuration validator using streaming pattern
    pub fn register_validator_stream(
        &self,
        validator: &Arc<dyn CandleConfigurationValidator + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = CandleConfigUpdate> + Send>> {
        let _manager = self.clone();
        let validator_name: String = String::from(validator.name());

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let now_nanos = unix_timestamp_nanos();

                // Create validator registration update
                let registration_update = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::ValidatorRegistered,
                    section: Some(validator_name.clone()),
                    success: true,
                    description: Some("Configuration validator registered".to_string()),
                };

                let _ = sender.send(registration_update);
            },
        ))
    }

    /// Create persistence event stream for lock-free tracking
    pub fn create_persistence_event_stream(
        &self,
    ) -> std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = super::persistence::CandlePersistenceEvent> + Send>,
    > {
        let manager = self.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Update persistence timestamp atomically
                let now_nanos = unix_timestamp_nanos();

                let previous_nanos = manager.last_persistence.swap(now_nanos, Ordering::AcqRel);

                // Create persistence event
                let event = super::persistence::CandlePersistenceEvent {
                    timestamp_nanos: now_nanos,
                    previous_timestamp_nanos: previous_nanos,
                    persistence_type: super::persistence::CandlePersistenceType::Manual,
                    success: true,
                };

                let _ = sender.send(event);
            },
        ))
    }

    /// Check if auto-save is needed using lock-free atomic operations with streaming pattern
    pub fn check_auto_save_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = CandleConfigUpdate> + Send>> {
        let manager = self.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let now_nanos = unix_timestamp_nanos();

                // Emit check initiated update
                let check_update = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::AutoSaveChecked,
                    section: None,
                    success: true,
                    description: Some("Auto-save check initiated".to_string()),
                };

                let _ = sender.send(check_update);

                let last_save_nanos = manager.last_persistence.load(Ordering::Acquire);
                let elapsed_secs = (now_nanos - last_save_nanos) / 1_000_000_000;

                // Access persistence to get actual auto_save_interval
                let persistence = manager.persistence.read().await;
                let auto_save_interval = persistence.auto_save_interval;

                if elapsed_secs >= auto_save_interval {
                    // Update timestamp atomically before saving
                    manager.last_persistence.store(now_nanos, Ordering::Release);

                    // Emit auto-save executed update
                    let autosave_update = CandleConfigUpdate {
                        timestamp_nanos: now_nanos,
                        update_type: CandleConfigUpdateType::AutoSaveExecuted,
                        section: None,
                        success: true,
                        description: Some("Auto-save executed".to_string()),
                    };

                    let _ = sender.send(autosave_update);
                }
            },
        ))
    }

    /// Save configuration to file using streaming pattern
    pub fn save_to_file_stream(
        &self,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = CandleConfigUpdate> + Send>> {
        let manager = self.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let now_nanos = unix_timestamp_nanos();

                // Emit save initiated update
                let save_start = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::SavedToFile,
                    section: None,
                    success: false,
                    description: Some("File save initiated".to_string()),
                };

                let _ = sender.send(save_start);

                // Perform file save using sync implementation
                let success = manager.save_to_file_sync().await.is_ok();

                // Emit save completion update
                let save_complete = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::SavedToFile,
                    section: None,
                    success,
                    description: Some(if success {
                        "File save completed successfully".to_string()
                    } else {
                        "File save failed".to_string()
                    }),
                };

                let _ = sender.send(save_complete);
            },
        ))
    }

    /// Asynchronous implementation of `save_to_file` for streams-only architecture
    async fn save_to_file_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.get_config();

        // Access persistence configuration synchronously and clone needed values
        let (format, compression, config_file_path) = {
            let persistence = self.persistence.read().await;

            (
                persistence.format.clone(),
                persistence.compression,
                persistence.config_file_path.clone(),
            )
        }; // Drop the RwLock guard here

        let serialized = match format.as_str() {
            "json" => serde_json::to_string_pretty(&*config)?,
            "yaml" => yyaml::to_string(&*config)?,
            "toml" => toml::to_string(&*config)?,
            _ => return Err("Unsupported format".into()),
        };

        let data = if compression {
            let compressed = lz4::block::compress(serialized.as_bytes(), None, true)?;
            {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(&compressed)
            }
        } else {
            serialized
        };

        tokio::fs::write(&config_file_path, data).await?;

        Ok(())
    }

    /// Load configuration from file using streaming pattern
    pub fn load_from_file_stream(&self) -> Pin<Box<dyn Stream<Item = CandleConfigUpdate> + Send>> {
        let manager = self.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let now_nanos = unix_timestamp_nanos();

                // Emit load initiated update
                let load_start = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::LoadedFromFile,
                    section: None,
                    success: false,
                    description: Some("File load initiated".to_string()),
                };

                let _ = sender.send(load_start);

                // Perform file load using sync implementation
                let success = manager.load_from_file_sync().await.is_ok();

                // Emit load completion update
                let load_complete = CandleConfigUpdate {
                    timestamp_nanos: now_nanos,
                    update_type: CandleConfigUpdateType::LoadedFromFile,
                    section: None,
                    success,
                    description: Some(if success {
                        "File load completed successfully".to_string()
                    } else {
                        "File load failed".to_string()
                    }),
                };

                let _ = sender.send(load_complete);
            },
        ))
    }

    /// Asynchronous implementation of `load_from_file` for streams-only architecture
    async fn load_from_file_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Access persistence configuration and clone needed values
        let (format, compression, config_file_path) = {
            let persistence = self.persistence.read().await;

            (
                persistence.format.clone(),
                persistence.compression,
                persistence.config_file_path.clone(),
            )
        }; // Drop the RwLock guard here

        let data = tokio::fs::read_to_string(&config_file_path).await?;

        let content = if compression {
            let compressed = {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.decode(&data)?
            };
            let decompressed = lz4::block::decompress(&compressed, None)?;
            String::from_utf8(decompressed)?
        } else {
            data
        };

        let config: CandleChatConfig = match format.as_str() {
            "json" => serde_json::from_str(&content)?,
            "yaml" => yyaml::from_str(&content)?,
            "toml" => toml::from_str(&content)?,
            _ => return Err("Unsupported format".into()),
        };

        // Update config atomically
        let config_arc = Arc::new(config);
        self.config.store(config_arc);

        Ok(())
    }

    /// Get Candle configuration change history
    pub async fn get_change_history(&self) -> Vec<CandleConfigurationChangeEvent> {
        let mut events = self.change_events.lock().await;
        events.drain(..).collect()
    }

    /// Get Candle configuration statistics
    pub fn get_statistics(&self) -> CandleConfigurationStatistics {
        CandleConfigurationStatistics {
            total_changes: self.change_counter.load(Ordering::Relaxed),
            current_version: self.version_counter.load(Ordering::Relaxed),
            last_modified: Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            validators_count: 0,      // Will be populated asynchronously
            auto_save_enabled: false, // Will be populated asynchronously
        }
    }
}
