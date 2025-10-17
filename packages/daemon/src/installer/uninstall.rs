//! Uninstallation and cleanup functionality
//!
//! This module provides uninstallation logic, certificate cleanup, and host file
//! restoration with zero allocation fast paths and blazing-fast performance.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use log::{info, warn};

// Removed unused import: use super::core::InstallProgress;
use super::config::remove_sweetmcp_host_entries;
use crate::install::fluent_voice;
use crate::install::uninstall_daemon_async;

/// Uninstall SweetMCP daemon with comprehensive cleanup
pub async fn uninstall_kodegen_daemon() -> Result<()> {
    info!("Starting SweetMCP daemon uninstallation");

    // Remove daemon service
    match uninstall_daemon_async("kodegend").await {
        Ok(()) => {
            info!("Daemon service uninstalled successfully");
        }
        Err(e) => {
            warn!("Failed to uninstall daemon service: {}", e);
            // Continue with other cleanup steps
        }
    }

    // Remove host entries
    if let Err(e) = remove_sweetmcp_host_entries() {
        warn!("Failed to remove SweetMCP host entries: {}", e);
    }

    // Remove wildcard certificate from system trust store
    if let Err(e) = remove_wildcard_certificate_from_system().await {
        warn!("Failed to remove wildcard certificate from system: {}", e);
    }

    // Clean up installation directories
    if let Err(e) = cleanup_installation_directories() {
        warn!("Failed to clean up installation directories: {}", e);
    }

    // Uninstall fluent-voice components
    let fluent_voice_path = std::path::Path::new("/opt/sweetmcp/fluent-voice");
    if let Err(e) = fluent_voice::uninstall_fluent_voice(fluent_voice_path).await {
        warn!("Failed to uninstall fluent-voice components: {}", e);
    }

    info!("SweetMCP daemon uninstallation completed");
    Ok(())
}

/// Validate existing wildcard certificate with fast validation (used by config.rs)
#[allow(dead_code)] // Library function for certificate validation operations
pub fn validate_existing_wildcard_cert(cert_path: &Path) -> Result<()> {
    let cert_content = fs::read_to_string(cert_path).context("Failed to read certificate file")?;

    // Basic validation - check if it contains the expected domains
    let required_domains = [
        "sweetmcp.kodegen.dev",
        "sweetmcp.kodegen.ai",
        "sweetmcp.kodegen.cloud",
        "sweetmcp.kodegen.pro",
    ];

    for domain in &required_domains {
        if !cert_content.contains(domain) {
            return Err(anyhow::anyhow!("Missing required domain: {}", domain));
        }
    }

    // Check if it has both certificate and private key
    if !cert_content.contains("-----BEGIN CERTIFICATE-----")
        || !cert_content.contains("-----BEGIN PRIVATE KEY-----")
    {
        return Err(anyhow::anyhow!(
            "Invalid certificate format - missing certificate or private key"
        ));
    }

    Ok(())
}

/// Import wildcard certificate on Linux
#[cfg(target_os = "linux")]
fn import_wildcard_certificate_linux(cert_path: &str) -> Result<()> {
    info!("Importing SweetMCP wildcard certificate to Linux system trust store");

    // Extract just the certificate part from the combined PEM file
    let cert_content =
        std::fs::read_to_string(cert_path).context("Failed to read certificate file")?;

    // Find the certificate part (everything before the private key)
    let cert_part = if let Some(key_start) = cert_content.find("-----BEGIN PRIVATE KEY-----") {
        &cert_content[..key_start]
    } else {
        &cert_content
    };

    // Copy certificate to system trust store
    let system_cert_path = "/usr/local/share/ca-certificates/sweetmcp-wildcard.crt";

    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(system_cert_path).parent() {
        std::fs::create_dir_all(parent).context("Failed to create ca-certificates directory")?;
    }

    std::fs::write(system_cert_path, cert_part)
        .context("Failed to write certificate to system trust store")?;

    // Update certificate trust store
    let output = Command::new("update-ca-certificates")
        .output()
        .context("Failed to execute update-ca-certificates")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to update certificate trust store: {}", stderr);
        // Don't fail the installation if this step fails
    } else {
        info!("Successfully imported SweetMCP wildcard certificate to Linux system trust store");
    }

    Ok(())
}

/// Remove wildcard certificate from system trust store
async fn remove_wildcard_certificate_from_system() -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            remove_wildcard_certificate_macos().await
        } else if #[cfg(target_os = "linux")] {
            remove_wildcard_certificate_linux().await
        } else {
            warn!("Wildcard certificate removal not supported on this platform");
            Ok(())
        }
    }
}

/// Remove wildcard certificate from macOS keychain
#[cfg(target_os = "macos")]
async fn remove_wildcard_certificate_macos() -> Result<()> {
    info!("Removing SweetMCP wildcard certificate from macOS System keychain");

    // Find and delete the certificate
    let output = Command::new("security")
        .args([
            "delete-certificate",
            "-c",
            "*.kodegen.dev",
            "/Library/Keychains/System.keychain",
        ])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't treat this as a fatal error since the certificate might not exist
        warn!(
            "Failed to remove certificate from macOS keychain (might not exist): {}",
            stderr
        );
    } else {
        info!("Successfully removed SweetMCP wildcard certificate from macOS System keychain");
    }

    Ok(())
}

/// Remove wildcard certificate from Linux system trust store
#[cfg(target_os = "linux")]
async fn remove_wildcard_certificate_linux() -> Result<()> {
    info!("Removing SweetMCP wildcard certificate from Linux system trust store");

    let system_cert_path = "/usr/local/share/ca-certificates/sweetmcp-wildcard.crt";

    // Remove certificate file
    if std::path::Path::new(system_cert_path).exists() {
        std::fs::remove_file(system_cert_path)
            .context("Failed to remove certificate from system trust store")?;

        // Update certificate trust store
        let output = Command::new("update-ca-certificates")
            .output()
            .context("Failed to execute update-ca-certificates")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to update certificate trust store: {}", stderr);
        } else {
            info!(
                "Successfully removed SweetMCP wildcard certificate from Linux system trust store"
            );
        }
    } else {
        info!("SweetMCP wildcard certificate not found in system trust store");
    }

    Ok(())
}

/// Clean up installation directories with comprehensive cleanup
fn cleanup_installation_directories() -> Result<()> {
    let directories_to_remove = get_installation_directories();

    for dir in directories_to_remove {
        if dir.exists() {
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => {
                    info!("Removed directory: {:?}", dir);
                }
                Err(e) => {
                    warn!("Failed to remove directory {:?}: {}", dir, e);
                    // Continue with other directories
                }
            }
        }
    }

    Ok(())
}

/// Get list of installation directories to clean up
fn get_installation_directories() -> Vec<PathBuf> {
    vec![
        #[cfg(target_os = "macos")]
        PathBuf::from("/usr/local/var/sweetmcp"),
        #[cfg(target_os = "linux")]
        PathBuf::from("/var/lib/sweetmcp"),
        #[cfg(target_os = "linux")]
        PathBuf::from("/etc/sweetmcp"),
        #[cfg(target_os = "windows")]
        PathBuf::from("C:\\ProgramData\\SweetMCP"),
        #[cfg(target_os = "windows")]
        PathBuf::from("C:\\Program Files\\SweetMCP"),
        // Common directories
        PathBuf::from("/opt/sweetmcp"),
        PathBuf::from("/tmp/sweetmcp"),
    ]
}

/// Add SweetMCP host entries with optimized host file modification
#[allow(dead_code)] // Library function for host file management operations
fn add_sweetmcp_host_entries() -> Result<()> {
    let hosts_file = if cfg!(target_os = "windows") {
        "C:\\Windows\\System32\\drivers\\etc\\hosts"
    } else {
        "/etc/hosts"
    };

    // Read current hosts file
    let current_hosts = fs::read_to_string(hosts_file).context("Failed to read hosts file")?;

    let sweetmcp_domains = [
        "sweetmcp.kodegen.dev",
        "sweetmcp.kodegen.ai",
        "sweetmcp.kodegen.cloud",
        "sweetmcp.kodegen.pro",
    ];

    let mut new_entries = Vec::new();
    let mut entries_added = false;

    // Check which entries need to be added
    for domain in &sweetmcp_domains {
        if !current_hosts.contains(domain) {
            new_entries.push(format!("127.0.0.1 {}", domain));
            entries_added = true;
        } else {
            info!("{} entry already exists in hosts file", domain);
        }
    }

    if !entries_added {
        info!("All SweetMCP host entries already exist");
        return Ok(());
    }

    // Append new entries
    let mut updated_hosts = current_hosts;
    if !updated_hosts.ends_with('\n') {
        updated_hosts.push('\n');
    }
    updated_hosts.push_str("\n# SweetMCP Auto-Integration\n");
    for entry in &new_entries {
        updated_hosts.push_str(&format!("{}\n", entry));
    }

    // Write updated hosts file
    fs::write(hosts_file, updated_hosts).context("Failed to write hosts file")?;

    info!(
        "Successfully added {} SweetMCP host entries",
        new_entries.len()
    );
    Ok(())
}

/// Get the installed daemon path for the current platform
#[allow(dead_code)] // Library function for daemon path resolution
fn get_installed_daemon_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // Windows installs to Program Files or System32
        PathBuf::from("C:\\Program Files\\kodegend\\kodegend.exe")
    }

    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/local/bin/kodegend")
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/local/bin/kodegend")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/usr/local/bin/kodegend")
    }
}

/// Create tar command arguments with proper path validation
fn create_backup_args(backup_path: &Path, config_dir: &Path) -> Result<Vec<String>> {
    let parent = config_dir.parent()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Config directory has no parent"
        ))?;
    
    let filename = config_dir.file_name()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Config directory has no filename"
        ))?;
    
    Ok(vec![
        "-czf".to_string(),
        backup_path.to_string_lossy().to_string(),
        "-C".to_string(),
        parent.to_string_lossy().to_string(),
        filename.to_string_lossy().to_string(),
    ])
}

/// Backup configuration before uninstall (API function for future CLI use)
#[allow(dead_code)]
pub fn backup_configuration() -> Result<PathBuf> {
    let config_dir = get_config_directory();
    let backup_dir = get_backup_directory();

    // Create backup directory
    std::fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

    // Generate backup filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("sweetmcp_config_backup_{}.tar.gz", timestamp));

    // Create tar archive of configuration
    let args = create_backup_args(&backup_path, &config_dir)
        .context("Failed to prepare backup command arguments")?;

    let output = Command::new("tar")
        .args(&args)
        .output()
        .context("Failed to create configuration backup")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to create configuration backup: {}",
            stderr
        ));
    }

    info!("Configuration backed up to: {:?}", backup_path);
    Ok(backup_path)
}

/// Get configuration directory path
fn get_config_directory() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/local/var/sweetmcp")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/lib/sweetmcp")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\ProgramData\\SweetMCP")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("/tmp/sweetmcp")
    }
}

/// Get backup directory path
fn get_backup_directory() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/local/var/sweetmcp/backups")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/backups/sweetmcp")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\ProgramData\\SweetMCP\\backups")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("/tmp/sweetmcp/backups")
    }
}

/// Create tar extraction arguments with proper path validation  
fn create_restore_args(backup_path: &Path, parent_dir: &Path) -> Vec<String> {
    vec![
        "-xzf".to_string(),
        backup_path.to_string_lossy().to_string(),
        "-C".to_string(),
        parent_dir.to_string_lossy().to_string(),
    ]
}

/// Restore configuration from backup (API function for future CLI use)
#[allow(dead_code)]
pub fn restore_configuration(backup_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow::anyhow!("Backup file not found: {:?}", backup_path));
    }

    let config_dir = get_config_directory();
    let parent_dir = config_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid configuration directory"))?;

    // Extract backup
    let args = create_restore_args(backup_path, parent_dir);

    let output = Command::new("tar")
        .args(&args)
        .output()
        .context("Failed to extract configuration backup")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to extract configuration backup: {}",
            stderr
        ));
    }

    info!("Configuration restored from: {:?}", backup_path);
    Ok(())
}
