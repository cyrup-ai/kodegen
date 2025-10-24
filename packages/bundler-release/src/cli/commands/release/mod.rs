//! Release command execution module.
//!
//! Handles the complete release workflow by coordinating all modules
//! in an isolated temporary clone to prevent modifications to the user's working directory.

mod r#impl;

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{ReleaseError, Result};
use crate::state::has_active_release_at;
use crate::workspace::{SharedWorkspaceInfo, WorkspaceInfo, WorkspaceValidator};
use std::sync::Arc;

use super::temp_clone::{
    clear_active_temp_path, clone_main_to_temp_for_release, save_active_temp_path,
};
use r#impl::perform_release_impl;
use std::path::Path;

/// Options for configuring the release process
#[derive(Clone)]
pub(super) struct ReleaseOptions {
    pub bump_type: crate::cli::args::BumpType,
    pub dry_run: bool,
    pub no_push: bool,
    pub registry: Option<String>,
    pub package_delay: u64,
    pub github_release: bool,
    pub github_repo: Option<String>,
    pub github_draft: bool,
    pub release_notes: Option<std::path::PathBuf>,
    pub with_bundles: bool,
    pub upload_bundles: bool,
    pub continue_on_github_error: bool,
}

/// Execute release in temp directory with guaranteed cleanup on all code paths.
///
/// This helper ensures that ANY error (from save_active_temp_path, workspace analysis,
/// or the actual release) flows through a single Result that can be handled after
/// guaranteed cleanup runs.
async fn execute_release_in_temp(
    temp_dir: &Path,
    config: &RuntimeConfig,
    options: &ReleaseOptions,
) -> Result<i32> {
    // Save temp path for resume support (can fail)
    save_active_temp_path(temp_dir)?;

    config.println("🚀 Starting release in isolated environment");
    config.println("   Your working directory will not be modified");

    // Re-analyze workspace from temp clone (can fail)
    let workspace: SharedWorkspaceInfo = Arc::new(WorkspaceInfo::analyze(temp_dir)?);

    // Perform release in temp directory (can fail)
    perform_release_impl(temp_dir, workspace, config, options).await
}

/// Execute release command
pub(super) async fn execute_release(args: &Args, config: &RuntimeConfig) -> Result<i32> {
    if let Command::Release {
        bump_type,
        dry_run,
        skip_validation,
        allow_dirty: _,
        no_push,
        registry,
        package_delay,
        max_retries: _,
        timeout: _,
        no_github_release,
        github_repo,
        github_draft,
        release_notes,
        no_bundles,
        no_upload_bundles,
        continue_on_github_error,
        keep_temp,
        no_clear_runway,
    } = &args.command
    {
        config.verbose_println("Starting release operation...");

        // Check for existing release state
        if has_active_release_at(&config.state_file_path) {
            return Err(ReleaseError::State(crate::error::StateError::SaveFailed {
                reason: "Another release is in progress. Use 'resume' or 'cleanup' first"
                    .to_string(),
            }));
        }

        // Analyze workspace
        config.verbose_println("Analyzing workspace...");
        let workspace: SharedWorkspaceInfo =
            Arc::new(WorkspaceInfo::analyze(&config.workspace_path)?);

        // Validate workspace if not skipped
        if !skip_validation {
            config.section("Workspace Validation");
            let validator = WorkspaceValidator::new(workspace.clone())?;

            config.progress("Checking git repository state...");
            let validation = validator.validate().await?;

            if !validation.success {
                config.error_println("Workspace validation failed:");
                for error in &validation.critical_errors {
                    config.error_println(&format!("  • {}", error));
                }
                return Err(ReleaseError::Workspace(
                    crate::error::WorkspaceError::InvalidStructure {
                        reason: "Workspace validation failed".to_string(),
                    },
                ));
            }

            if !validation.warnings.is_empty() && config.is_verbose() {
                config.warning_println("Workspace validation warnings:");
                for warning in &validation.warnings {
                    config.warning_println(&format!("  • {}", warning));
                }
            }
        }

        // Clear runway (remove orphaned branches/tags from failed releases)
        if !no_clear_runway {
            config.println("🛫 Checking runway for release...");
            match super::runway::clear_runway_for_version(
                &config.workspace_path,
                &workspace,
                bump_type,
                config,
            )
            .await
            {
                Ok(result) => {
                    if result.cleared_anything() {
                        config.println(&result.format_result());
                    } else {
                        config.verbose_println(&result.format_result());
                    }
                }
                Err(e) => {
                    config.warning_println(&format!("Failed to clear runway: {}", e));
                    config.warning_println("Continuing with release anyway...");
                }
            }
        }

        // Clone main branch to temp for isolated execution
        config.println("🔄 Cloning main branch to isolated environment...");
        let temp_dir = clone_main_to_temp_for_release(&config.workspace_path).await?;
        config.println(&format!("   Temp location: {}", temp_dir.display()));

        // Create release options
        let options = ReleaseOptions {
            bump_type: bump_type.clone(),
            dry_run: *dry_run,
            no_push: *no_push,
            registry: registry.clone(),
            package_delay: *package_delay,
            github_release: !no_github_release, // Inverted: default is TRUE unless --no-github-release
            github_repo: github_repo.clone(),
            github_draft: *github_draft,
            release_notes: release_notes.clone(),
            with_bundles: !no_bundles, // Inverted: default is TRUE unless --no-bundles
            upload_bundles: !no_upload_bundles, // Inverted: default is TRUE unless --no-upload-bundles
            continue_on_github_error: *continue_on_github_error,
        };

        // Execute release in temp directory - ALL errors flow through release_result
        // This ensures cleanup ALWAYS runs regardless of which operation fails
        let release_result = execute_release_in_temp(&temp_dir, config, &options).await;

        // ALWAYS cleanup temp directory (runs whether success or failure)
        if !keep_temp {
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                config.warning_println(&format!("Failed to cleanup temp directory: {}", e));
                config.warning_println(&format!(
                    "You may need to manually remove: {}",
                    temp_dir.display()
                ));
            } else {
                config.verbose_println("✅ Temp clone cleaned up");
            }
            // Clear temp path tracking (ignore errors)
            let _ = clear_active_temp_path();
        } else {
            config.println(&format!(
                "🔍 Temp clone kept for debugging at: {}",
                temp_dir.display()
            ));
            config.println("   Use 'cleanup' command to remove it later");
        }

        // Return the original result (success or error)
        release_result
    } else {
        unreachable!("execute_release called with non-Release command");
    }
}
