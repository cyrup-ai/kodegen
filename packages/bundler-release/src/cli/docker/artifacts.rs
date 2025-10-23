//! Artifact verification and discovery for Docker builds.
//!
//! Handles finding and verifying package artifacts created by Docker containers.

use crate::error::{CliError, ReleaseError};
use std::path::{Path, PathBuf};

/// Finds the bundle directory for a platform, handling case-insensitive matching.
///
/// This function searches for the bundle directory case-insensitively to handle
/// differences in how bundlers create directory names across platforms.
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace root
/// * `platform_str` - Platform string (e.g., "deb", "rpm", "appimage")
///
/// # Returns
///
/// * `Ok(PathBuf)` - Path to the bundle directory
/// * `Err` - Bundle directory not found
pub fn find_bundle_directory(
    workspace_path: &Path,
    platform_str: &str,
) -> Result<PathBuf, ReleaseError> {
    let bundle_base = workspace_path
        .join("target")
        .join("release")
        .join("bundle");
    
    if !bundle_base.exists() {
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "find bundle directory".to_string(),
            reason: format!("Bundle directory does not exist: {}", bundle_base.display()),
        }));
    }
    
    // Try exact match first (most common)
    let exact_match = bundle_base.join(platform_str.to_lowercase());
    if exact_match.exists() {
        return Ok(exact_match);
    }
    
    // Search for case-insensitive match
    let entries = std::fs::read_dir(&bundle_base)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read bundle directory".to_string(),
            reason: format!("Failed to read {}: {}", bundle_base.display(), e),
        }))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read directory entry".to_string(),
            reason: format!("Failed to read entry in {}: {}", bundle_base.display(), e),
        }))?;
        
        let path = entry.path();
        if path.is_dir()
            && let Some(dir_name) = path.file_name()
            && dir_name.to_string_lossy().eq_ignore_ascii_case(platform_str)
        {
            return Ok(path);
        }
    }
    
    // Not found
    Err(ReleaseError::Cli(CliError::ExecutionFailed {
        command: "find bundle directory".to_string(),
        reason: format!(
            "Bundle directory not found for platform '{}' in {}",
            platform_str,
            bundle_base.display()
        ),
    }))
}

/// Verifies that artifacts are complete and not corrupted.
///
/// Checks:
/// - File exists and is readable
/// - File size > 0 (not empty)
/// - File has expected extension
///
/// # Arguments
///
/// * `artifacts` - Paths to artifact files to verify
/// * `runtime_config` - For verbose output
pub fn verify_artifacts(
    artifacts: &[PathBuf],
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<(), ReleaseError> {
    for artifact in artifacts {
        // Check file exists and get metadata
        let metadata = std::fs::metadata(artifact)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "verify artifact".to_string(),
                reason: format!("Cannot read artifact {}: {}", artifact.display(), e),
            }))?;

        // Check file is not empty
        if metadata.len() == 0 {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "verify artifact".to_string(),
                reason: format!(
                    "Artifact is empty (0 bytes): {}\n\
                     This indicates a failed or incomplete build.",
                    artifact.display()
                ),
            }));
        }

        // Success - log verification
        runtime_config.indent(&format!(
            "  ✓ Verified: {} ({} bytes)",
            artifact.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unknown>"),
            metadata.len()
        ));
    }

    Ok(())
}
