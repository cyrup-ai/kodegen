//! Configuration structures for certificate provisioning and setup.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the setup command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupConfig {
    /// Platform-specific configuration
    #[serde(flatten)]
    pub platform: PlatformConfig,

    /// Dry-run mode (validate without making changes)
    #[serde(default)]
    pub dry_run: bool,

    /// Verbose output
    #[serde(default)]
    pub verbose: bool,
}

/// Platform-specific setup configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform")]
pub enum PlatformConfig {
    #[serde(rename = "macos")]
    MacOS(MacOSSetupConfig),

    #[serde(rename = "linux")]
    Linux(LinuxSetupConfig),

    #[serde(rename = "windows")]
    Windows(WindowsSetupConfig),
}

/// macOS-specific setup configuration for App Store Connect API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSSetupConfig {
    pub issuer_id: String,
    pub key_id: String,
    pub private_key_path: PathBuf,
    #[serde(default = "default_cert_type")]
    pub certificate_type: CertificateType,
    #[serde(default = "default_common_name")]
    pub common_name: String,
    #[serde(default = "default_keychain")]
    pub keychain: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CertificateType {
    #[serde(rename = "developer_id")]
    DeveloperIdApplication,
    #[serde(rename = "mac_app_distribution")]
    MacAppDistribution,
}

/// Linux-specific setup configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSetupConfig {
    /// Validate GPG is installed
    #[serde(default = "default_true")]
    pub validate_gpg: bool,
}

/// Windows-specific setup configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSetupConfig {
    /// Validate signtool.exe is available
    #[serde(default = "default_true")]
    pub validate_signtool: bool,
}

fn default_cert_type() -> CertificateType {
    CertificateType::DeveloperIdApplication
}

fn default_common_name() -> String {
    "Kodegen Helper".to_string()
}

fn default_keychain() -> String {
    "login.keychain-db".to_string()
}

fn default_true() -> bool {
    true
}
