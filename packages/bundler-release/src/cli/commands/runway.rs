//! Runway clearing for failed releases.
//!
//! This module provides functionality to automatically detect and clean up
//! orphaned release branches and tags from remote repositories when a previous
//! release attempt failed.

use crate::cli::RuntimeConfig;
use crate::error::{GitError, ReleaseError, Result};
use crate::version::{VersionBump, VersionBumper, VersionManager};
use crate::workspace::SharedWorkspaceInfo;
use semver::Version;

/// Result of runway clearing operation
#[derive(Debug, Clone)]
pub struct RunwayClearResult {
    /// Version that was being prepared
    pub version: Version,
    /// Whether the remote branch was deleted
    pub branch_deleted: bool,
    /// Whether the remote tag was deleted
    pub tag_deleted: bool,
    /// Any warnings during clearing
    pub warnings: Vec<String>,
}

impl RunwayClearResult {
    /// Check if anything was cleared
    pub fn cleared_anything(&self) -> bool {
        self.branch_deleted || self.tag_deleted
    }

    /// Format result for display
    pub fn format_result(&self) -> String {
        if !self.cleared_anything() {
            return format!("✓ Runway is clear for v{}", self.version);
        }

        let mut result = format!("🧹 Cleared runway for v{}:\n", self.version);

        if self.branch_deleted {
            result.push_str(&format!("  ✓ Deleted remote branch v{}\n", self.version));
        }

        if self.tag_deleted {
            result.push_str(&format!("  ✓ Deleted remote tag v{}\n", self.version));
        }

        if !self.warnings.is_empty() {
            result.push_str("  ⚠️ Warnings:\n");
            for warning in &self.warnings {
                result.push_str(&format!("    • {}\n", warning));
            }
        }

        result
    }
}

/// Clear the runway for a new release by removing orphaned branches and tags
///
/// This function checks if a release branch/tag exists on the remote for the target
/// version. If they exist (indicating a failed previous release attempt), they are
/// automatically deleted to allow the release to proceed.
///
/// # Safety
///
/// This function only deletes branches/tags that match the version pattern (v{version}).
/// It does not check release state history, assuming that if you're starting a new
/// release with the same version bump, the previous attempt failed and should be cleared.
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace repository
/// * `workspace` - Shared workspace information
/// * `bump_type` - Type of version bump being performed
/// * `config` - Runtime configuration for output
///
/// # Returns
///
/// Returns `RunwayClearResult` indicating what was cleared, or an error if the
/// clearing operation failed critically.
pub async fn clear_runway_for_version(
    workspace_path: &std::path::Path,
    workspace: &SharedWorkspaceInfo,
    bump_type: &crate::cli::args::BumpType,
    config: &RuntimeConfig,
) -> Result<RunwayClearResult> {
    // Calculate target version
    let version_manager = VersionManager::new(workspace.clone());
    let current_version = version_manager.current_version()?;

    let version_bump = VersionBump::try_from(bump_type.clone())
        .map_err(|e| ReleaseError::Cli(crate::error::CliError::InvalidArguments { reason: e }))?;

    let bumper = VersionBumper::from_version(current_version.clone());
    let target_version = bumper.bump(version_bump)?;

    config.verbose_println(&format!(
        "Checking runway for v{} (current: v{})",
        target_version, current_version
    ));

    // Open repository
    let repo = kodegen_tools_git::discover_repo(workspace_path)
        .await
        .map_err(|_| ReleaseError::Git(GitError::NotRepository))?
        .map_err(|_| ReleaseError::Git(GitError::NotRepository))?;

    let branch_name = format!("v{}", target_version);
    let tag_name = format!("v{}", target_version);
    let remote = "origin";

    let mut warnings = Vec::new();
    let mut branch_deleted = false;
    let mut tag_deleted = false;

    // Check if remote branch exists
    match kodegen_tools_git::check_remote_branch_exists(&repo, remote, &branch_name).await {
        Ok(exists) => {
            if exists {
                config.warning_println(&format!(
                    "Found orphaned remote branch '{}' from failed release",
                    branch_name
                ));
                config.println(&format!("  Deleting remote branch '{}'...", branch_name));

                // Delete remote branch
                match kodegen_tools_git::delete_remote_branch(&repo, remote, &branch_name).await {
                    Ok(()) => {
                        branch_deleted = true;
                        config.verbose_println(&format!(
                            "  ✓ Successfully deleted remote branch '{}'",
                            branch_name
                        ));
                    }
                    Err(e) => {
                        warnings.push(format!(
                            "Failed to delete remote branch '{}': {}",
                            branch_name, e
                        ));
                        config.warning_println(&format!(
                            "  Failed to delete remote branch '{}': {}",
                            branch_name, e
                        ));
                    }
                }
            } else {
                config.verbose_println(&format!("  No remote branch '{}' found", branch_name));
            }
        }
        Err(e) => {
            warnings.push(format!(
                "Failed to check remote branch '{}': {}",
                branch_name, e
            ));
            config.verbose_println(&format!(
                "  Could not check remote branch '{}': {}",
                branch_name, e
            ));
        }
    }

    // Check if remote tag exists
    match kodegen_tools_git::check_remote_tag_exists(&repo, remote, &tag_name).await {
        Ok(exists) => {
            if exists {
                config.warning_println(&format!(
                    "Found orphaned remote tag '{}' from failed release",
                    tag_name
                ));
                config.println(&format!("  Deleting remote tag '{}'...", tag_name));

                // Delete remote tag
                match kodegen_tools_git::delete_remote_tag(&repo, remote, &tag_name).await {
                    Ok(()) => {
                        tag_deleted = true;
                        config.verbose_println(&format!(
                            "  ✓ Successfully deleted remote tag '{}'",
                            tag_name
                        ));
                    }
                    Err(e) => {
                        warnings.push(format!("Failed to delete remote tag '{}': {}", tag_name, e));
                        config.warning_println(&format!(
                            "  Failed to delete remote tag '{}': {}",
                            tag_name, e
                        ));
                    }
                }
            } else {
                config.verbose_println(&format!("  No remote tag '{}' found", tag_name));
            }
        }
        Err(e) => {
            warnings.push(format!("Failed to check remote tag '{}': {}", tag_name, e));
            config.verbose_println(&format!(
                "  Could not check remote tag '{}': {}",
                tag_name, e
            ));
        }
    }

    let result = RunwayClearResult {
        version: target_version,
        branch_deleted,
        tag_deleted,
        warnings,
    };

    // Log summary
    if result.cleared_anything() {
        config.success_println(&format!(
            "Runway cleared for v{} - ready for release",
            result.version
        ));
    } else {
        config.verbose_println(&format!("Runway already clear for v{}", result.version));
    }

    Ok(result)
}
