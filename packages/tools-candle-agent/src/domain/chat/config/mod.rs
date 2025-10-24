//! Configuration management system for chat features
//!
//! This module provides a comprehensive configuration management system with atomic updates,
//! validation, persistence, and change notifications using zero-allocation patterns and
//! lock-free operations for blazing-fast performance.

mod manager;
mod model;
mod persistence;
mod streaming;
mod types;
mod validation;

// Re-export public API - maintain exact same public interface
pub use model::{CandleModelConfig, CandleModelPerformanceConfig, CandleModelRetryConfig};

pub use types::{CandleBehaviorConfig, CandleChatConfig, CandlePersonalityConfig, CandleUIConfig};

pub use validation::{
    CandleBehaviorValidator, CandleConfigurationValidationError,
    CandleConfigurationValidationResult, CandleConfigurationValidator, CandlePersonalityValidator,
    CandleUIValidator,
};

pub use manager::{
    CandleConfigurationChangeEvent, CandleConfigurationChangeType, CandleConfigurationManager,
    CandleConfigurationStatistics,
};

pub use persistence::{
    CandleConfigurationPersistence, CandlePersistenceEvent, CandlePersistenceType,
};

pub use streaming::{
    CandleConfigUpdate, CandleConfigUpdateType, CandleModelConfigChunk, CandleModelConfigData,
};
