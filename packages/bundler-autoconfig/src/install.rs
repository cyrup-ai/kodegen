use anyhow::{Context, Result};
use log::{debug, error, info};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::ClientConfigPlugin;

/// Result of installing kodegen for a single client
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub client_name: String,
    pub client_id: String,
    pub success: bool,
    pub message: String,
    pub config_path: Option<PathBuf>,
}

/// Install kodegen for all detected clients
///
/// # Errors
///
/// Returns an error if there are issues scanning for clients or processing configurations.
pub fn install_all_clients() -> Result<Vec<InstallResult>> {
    let clients = crate::clients::all_clients();
    let mut results = Vec::new();

    info!("🔍 Scanning for MCP-compatible editors...");

    for client in clients {
        let result = install_client(client.as_ref());
        results.push(result);
    }

    Ok(results)
}

/// Install kodegen for a single client
fn install_client(client: &dyn ClientConfigPlugin) -> InstallResult {
    debug!("Checking {} installation", client.client_name());

    // Check if client is installed (copied from watcher.rs perform_initial_scan)
    let watch_paths = client.watch_paths();
    let is_installed = watch_paths.iter().any(|p| client.is_installed(p));

    if !is_installed {
        return InstallResult {
            client_name: client.client_name().to_string(),
            client_id: client.client_id().to_string(),
            success: false,
            message: "Not installed".to_string(),
            config_path: None,
        };
    }

    info!("Found {} installation", client.client_name());

    // Try to process each config path
    for config_path in client.config_paths() {
        match process_config_file(client, &config_path.path) {
            Ok(status) => {
                return InstallResult {
                    client_name: client.client_name().to_string(),
                    client_id: client.client_id().to_string(),
                    success: true,
                    message: status,
                    config_path: Some(config_path.path),
                };
            }
            Err(e) => {
                error!("Failed to process {}: {}", config_path.path.display(), e);
                // Continue to try next config path
            }
        }
    }

    // All config paths failed
    InstallResult {
        client_name: client.client_name().to_string(),
        client_id: client.client_id().to_string(),
        success: false,
        message: "Failed to configure".to_string(),
        config_path: None,
    }
}

/// Process a config file - sync version adapted from watcher.rs
fn process_config_file(client: &dyn ClientConfigPlugin, path: &Path) -> Result<String> {
    use std::fs;

    // Read existing config (adapted from watcher.rs line 193-209)
    let config_content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // Config doesn't exist - create it
            let new_config = client.inject_kodegen("{}", client.config_format())?;

            // Ensure directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write new config
            fs::write(path, &new_config)?;
            info!("✅ Created kodegen config for {}", client.client_name());
            return Ok("Created new config".to_string());
        }
        Err(e) => return Err(e.into()),
    };

    // Fast-path check: already configured? (watcher.rs line 220-223)
    if config_content.contains("kodegen") {
        debug!("Already configured, skipping");
        return Ok("Already configured".to_string());
    }

    // Create backup (watcher.rs line 229-237)
    let backup_path = {
        let mut bp = path.to_path_buf();
        if let Some(filename) = bp.file_name() {
            let mut new_name = filename.to_os_string();
            new_name.push(".backup");
            bp.set_file_name(new_name);
        }
        bp
    };

    fs::copy(path, &backup_path).context("Failed to create backup")?;

    // Inject kodegen config (watcher.rs line 242)
    let updated_config = client.inject_kodegen(&config_content, client.config_format())?;

    // Write updated config (watcher.rs line 245)
    fs::write(path, &updated_config)?;

    info!("✅ Injected kodegen config for {}", client.client_name());
    Ok("Configured successfully".to_string())
}
