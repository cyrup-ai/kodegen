//! Installer module decomposition
//!
//! This module provides the decomposed installer functionality split into
//! logical modules for better maintainability and adherence to the 300-line limit.

pub mod builder;
pub mod config;
pub mod core;
pub mod error;
pub mod fluent_voice;
pub mod linux;
pub mod macos;
pub mod uninstall;
pub mod windows;

// Re-export key types and functions for backward compatibility
pub use builder::{CommandBuilder, InstallerBuilder};
pub use core::AsyncTask;
pub use error::InstallerError;

// All config and uninstall functions removed as unused

// Compatibility re-exports for main.rs
use anyhow::{Context, Result};
use tokio::sync::mpsc;

/// Install the daemon with full end-to-end handling (compatibility wrapper)
#[inline]
pub fn install(dry: bool, sign: bool, _identity: Option<String>) -> AsyncTask<Result<()>> {
    let (tx, rx) = mpsc::channel(1);

    tokio::spawn(async move {
        let result = (async {
            if dry {
                // Dry run mode
                let config_path = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
                    .join("kodegen")
                    .join("config.toml");
                config::validate_configuration(&config_path)
            } else {
                // Get executable path and config path for installation
                let exe_path =
                    std::env::current_exe().context("Failed to get current executable path")?;
                let config_path = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
                    .join("kodegen")
                    .join("config.toml");
                config::install_kodegen_daemon(exe_path, config_path, sign).await
            }
        })
        .await;
        let _ = tx.send(result).await;
    });

    AsyncTask::from_receiver(rx)
}

/// Async uninstall the daemon (compatibility wrapper)
#[inline]
pub async fn uninstall_async(dry: bool) -> Result<()> {
    if dry {
        // Dry run - just validate current state
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("kodegen")
            .join("config.toml");
        config::validate_configuration(&config_path)
    } else {
        uninstall::uninstall_kodegen_daemon().await
    }
}

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
        Err(InstallerError::System(
            "Unsupported platform".to_string(),
        ))
    }
}
