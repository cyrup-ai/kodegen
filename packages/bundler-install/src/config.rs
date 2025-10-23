//! Configuration types re-exported from `kodegen_daemon`
//!
//! This module provides access to daemon configuration types needed by the installer.

// Re-export types from kodegen_daemon that are used by the installer
pub use kodegen_daemon::config::{HealthCheckConfig, ServiceDefinition};
