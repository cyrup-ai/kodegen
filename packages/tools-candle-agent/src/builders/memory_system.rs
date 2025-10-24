//! Memory system builder implementations
//!
//! All memory system configuration and builder patterns.

use crate::domain::memory::{
    CompatibilityMode, DatabaseConfig, MemorySystemConfig, VectorStoreConfig,
};

/// Memory system builder for ergonomic configuration
#[derive(Debug, Default)]
pub struct MemorySystemBuilder {
    database_config: Option<DatabaseConfig>,
    vector_config: Option<VectorStoreConfig>,
    enable_cognitive: bool,
    compatibility_mode: CompatibilityMode,
}

impl MemorySystemBuilder {
    /// Create new memory system builder
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set database configuration
    #[inline]
    pub fn with_database_config(mut self, config: DatabaseConfig) -> Self {
        self.database_config = Some(config);
        self
    }

    /// Set vector store configuration
    #[inline]
    pub fn with_vector_config(mut self, config: VectorStoreConfig) -> Self {
        self.vector_config = Some(config);
        self
    }

    /// Enable cognitive features
    #[inline]
    pub fn with_cognitive(mut self, enabled: bool) -> Self {
        self.enable_cognitive = enabled;
        self
    }

    /// Set compatibility mode
    #[inline]
    pub fn with_compatibility_mode(mut self, mode: CompatibilityMode) -> Self {
        self.compatibility_mode = mode;
        self
    }

    /// Build memory system configuration
    pub fn build(self) -> crate::domain::memory::MemoryResult<MemorySystemConfig> {
        let config = MemorySystemConfig {
            database: self.database_config.unwrap_or_default(),
            vector_store: self.vector_config.unwrap_or_default(),
            enable_cognitive: self.enable_cognitive,
            compatibility_mode: self.compatibility_mode,
        };

        config.validate()?;
        Ok(config)
    }
}
