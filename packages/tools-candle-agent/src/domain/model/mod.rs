//! Model system core module
//!
//! This module provides the core abstractions and types for AI model management,
//! including traits, information, registry, and error handling.

pub mod capabilities;
pub mod download_lock;
pub mod error;
pub mod info;
pub mod traits;
pub mod usage;
pub mod validation;

// Re-export commonly used Candle types
pub use capabilities::*;
pub use error::{CandleModelError, CandleResult};
pub use info::{CandleModelInfo, CandleProvider};
pub use traits::*;
pub use usage::CandleUsage;
pub use validation::*;
