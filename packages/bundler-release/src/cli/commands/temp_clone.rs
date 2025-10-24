//! Temp clone helpers for isolated release execution.
//!
//! This module provides functionality to clone the repository to a temporary
//! directory for isolated release operations, and track active temp paths.

use crate::error::{CliError, ReleaseError, Result};
use std::path::PathBuf;

/// Get origin URL from current workspace for cloning
pub(super) async fn get_origin_url_for_clone(workspace_path: &std::path::Path) -> Result<String> {
    let repo = kodegen_tools_git::discover_repo(workspace_path)
        .await
        .map_err(|_| ReleaseError::Git(crate::error::GitError::NotRepository))?
        .map_err(|_| ReleaseError::Git(crate::error::GitError::NotRepository))?;

    let remotes = kodegen_tools_git::list_remotes(&repo).await.map_err(|e| {
        ReleaseError::Git(crate::error::GitError::RemoteOperationFailed {
            operation: "list_remotes".to_string(),
            reason: e.to_string(),
        })
    })?;
    let origin = remotes.iter().find(|r| r.name == "origin").ok_or_else(|| {
        ReleaseError::Git(crate::error::GitError::RemoteOperationFailed {
            operation: "find_origin".to_string(),
            reason: "No origin remote found. Please configure an 'origin' remote.".to_string(),
        })
    })?;

    Ok(origin.fetch_url.clone())
}

/// Clone main branch to temporary directory for isolated release execution
pub(super) async fn clone_main_to_temp_for_release(
    workspace_path: &std::path::Path,
) -> Result<PathBuf> {
    // Get origin URL from current repository
    let remote_url = get_origin_url_for_clone(workspace_path).await?;

    // Create unique temp directory with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "get_timestamp".to_string(),
                reason: e.to_string(),
            })
        })?
        .as_secs();

    let temp_dir = std::env::temp_dir().join(format!("kodegen-release-{}", timestamp));

    // Clone main branch to temp
    let clone_opts = kodegen_tools_git::CloneOpts::new(remote_url, temp_dir.clone()).branch("main");

    kodegen_tools_git::clone_repo(clone_opts)
        .await
        .map_err(|e| {
            ReleaseError::Git(crate::error::GitError::RemoteOperationFailed {
                operation: "clone_repo".to_string(),
                reason: format!("Failed to clone repository: {}", e),
            })
        })?
        .map_err(|e| {
            ReleaseError::Git(crate::error::GitError::RemoteOperationFailed {
                operation: "clone_repo".to_string(),
                reason: e.to_string(),
            })
        })?;

    Ok(temp_dir)
}

/// Save active temp release path for resume support
pub(super) fn save_active_temp_path(temp_dir: &std::path::Path) -> Result<()> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "get_home_dir".to_string(),
                reason: "Could not determine home directory".to_string(),
            })
        })?
        .join(".kodegen");

    std::fs::create_dir_all(&config_dir).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "create_config_dir".to_string(),
            reason: e.to_string(),
        })
    })?;

    let tracking_file = config_dir.join("last_release_temp");
    std::fs::write(&tracking_file, temp_dir.to_string_lossy().as_bytes()).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "save_temp_path".to_string(),
            reason: e.to_string(),
        })
    })?;

    Ok(())
}

/// Get active temp release path if one exists
#[allow(dead_code)]
pub(super) fn get_active_temp_path() -> Option<PathBuf> {
    let config_dir = dirs::home_dir()?.join(".kodegen");
    let tracking_file = config_dir.join("last_release_temp");

    if tracking_file.exists() {
        let path_str = std::fs::read_to_string(&tracking_file).ok()?;
        Some(PathBuf::from(path_str.trim()))
    } else {
        None
    }
}

/// Clear active temp release path tracking
pub(super) fn clear_active_temp_path() -> Result<()> {
    if let Some(home_dir) = dirs::home_dir() {
        let tracking_file = home_dir.join(".kodegen").join("last_release_temp");
        if tracking_file.exists() {
            std::fs::remove_file(&tracking_file).map_err(|e| {
                ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "clear_temp_path".to_string(),
                    reason: e.to_string(),
                })
            })?;
        }
    }
    Ok(())
}
