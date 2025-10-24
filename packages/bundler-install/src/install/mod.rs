//! Installer module decomposition
//!
//! This module provides the decomposed installer functionality split into
//! logical modules for better maintainability and adherence to the 300-line limit.

pub mod builder;
pub mod config;
pub mod core;
pub mod error;
pub mod fluent_voice;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

pub mod uninstall;

#[cfg(target_os = "windows")]
pub mod windows;

// Re-export key types and functions for backward compatibility
pub use builder::InstallerBuilder;
pub use error::InstallerError;

// All config and uninstall functions removed as unused

// Compatibility re-exports for main.rs
use anyhow::Result;

/// Install daemon asynchronously using platform-specific implementation
pub async fn install_daemon_async(builder: InstallerBuilder) -> Result<(), InstallerError> {
    #[cfg(target_os = "macos")]
    return macos::PlatformExecutor::install(builder);

    #[cfg(target_os = "linux")]
    return linux::PlatformExecutor::install(builder);

    #[cfg(target_os = "windows")]
    return windows::PlatformExecutor::install(builder);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = builder;
        Err(InstallerError::System("Unsupported platform".to_string()))
    }
}
