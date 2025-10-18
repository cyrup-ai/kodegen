//! Code signing functionality for installer
//!
//! Stub module - actual signing implemented in daemon package

use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SigningConfig {
    pub binary_path: PathBuf,
    pub output_path: PathBuf,
    pub platform: PlatformConfig,
}

#[derive(Debug, Clone)]
pub enum PlatformConfig {
    #[cfg(target_os = "macos")]
    MacOS {
        identity: String,
        entitlements: Option<PathBuf>,
    },
    #[cfg(target_os = "windows")]
    Windows {
        certificate: String,
        password: Option<String>,
    },
    #[cfg(target_os = "linux")]
    Linux {
        key_id: Option<String>,
    },
}

impl SigningConfig {
    pub fn new(binary_path: PathBuf) -> Self {
        Self {
            output_path: binary_path.clone(),
            binary_path,
            platform: PlatformConfig::default_for_platform(),
        }
    }
}

impl PlatformConfig {
    fn default_for_platform() -> Self {
        #[cfg(target_os = "macos")]
        return Self::MacOS {
            identity: "-".to_string(),
            entitlements: None,
        };

        #[cfg(target_os = "windows")]
        return Self::Windows {
            certificate: String::new(),
            password: None,
        };

        #[cfg(target_os = "linux")]
        return Self::Linux { key_id: None };
    }
}

pub fn verify_signature(_path: &Path) -> Result<bool> {
    // Stub - return true for now
    Ok(true)
}

pub fn sign_binary(_config: &SigningConfig) -> Result<()> {
    // Stub - do nothing for now
    Ok(())
}
