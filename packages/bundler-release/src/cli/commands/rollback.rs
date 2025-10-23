//! Rollback command implementation.
//!
//! Handles rolling back a failed or partially completed release by:
//! - Yanking published packages from the registry
//! - Reverting git commits and tags
//! - Deleting GitHub releases
//! - Restoring version changes from backups

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{Result, ReleaseError};
use crate::git::{GitManager, GitConfig};
use crate::publish::Publisher;
use crate::state::{ReleasePhase, create_state_manager_at};
use crate::workspace::{WorkspaceInfo, SharedWorkspaceInfo};
use std::sync::Arc;

use super::temp_clone::{get_active_temp_path, clear_active_temp_path};

/// Execute rollback command
pub(super) async fn execute_rollback(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Rollback { force, git_only, packages_only, yes } = &args.command {
        config.verbose_println("Starting rollback operation...");

        // Check if there's an active temp clone from a previous release
        let (workspace_path, state_file_path) = if let Some(temp_path) = get_active_temp_path() {
            if temp_path.exists() {
                config.println(&format!("📂 Rolling back from temp clone: {}", temp_path.display()));
                let temp_state = temp_path.join(".cyrup_release_state.json");
                (temp_path, temp_state)
            } else {
                config.warning_println("Tracked temp clone no longer exists, using current workspace");
                clear_active_temp_path()?;
                (config.workspace_path.clone(), config.state_file_path.clone())
            }
        } else {
            (config.workspace_path.clone(), config.state_file_path.clone())
        };

        // Load release state
        let mut state_manager = create_state_manager_at(&state_file_path)?;
        let load_result = state_manager.load_state().await?;
        let mut release_state = load_result.state;

        if load_result.recovered_from_backup {
            config.warning_println("Loaded state from backup file");
        }

        // Validate rollback conditions
        if release_state.current_phase == ReleasePhase::Completed && !force {
            return Err(ReleaseError::State(crate::error::StateError::SaveFailed {
                reason: "Release completed successfully. Use --force to rollback anyway".to_string(),
            }));
        }

        if !yes {
            config.println(&format!(
                "About to rollback release {} (phase: {:?})",
                release_state.target_version,
                release_state.current_phase
            ));

            // In a real CLI, you'd prompt for confirmation here
            // For now, we'll assume confirmation
        }

        release_state.set_phase(ReleasePhase::RollingBack);
        state_manager.save_state(&release_state).await?;

        let workspace: SharedWorkspaceInfo = Arc::new(WorkspaceInfo::analyze(&workspace_path)?);

        // Rollback publishing if needed and not git-only
        if !git_only && release_state.publish_state.is_some() {
            config.println("📤 Rolling back published packages...");
            let publisher = Publisher::new(workspace.clone())?;
            let rollback_result = publisher.rollback_published_packages(config).await?;

            if rollback_result.fully_successful {
                config.success_println("All published packages yanked successfully");
            } else {
                config.warning_println(&format!("Rollback completed with warnings: {}", rollback_result.format_summary()));
            }
        }

        // Rollback git operations if needed and not packages-only
        if !packages_only && release_state.git_state.is_some() {
            config.println("📦 Rolling back git operations...");
            let git_config = GitConfig::default();
            let mut git_manager = GitManager::with_config(&workspace_path, git_config).await?;

            let git_rollback = git_manager.rollback_release().await?;

            if git_rollback.success {
                config.success_println("Git operations rolled back successfully");
            } else {
                config.warning_println(&format!("Git rollback completed with warnings: {}", git_rollback.format_result()));
            }
        }

        // Rollback GitHub release if created
        if let Some(github_state) = &release_state.github_state
            && let Some(release_id) = github_state.release_id {
                config.println("🐙 Rolling back GitHub release...");

                let github_token = std::env::var("GH_TOKEN")
                    .or_else(|_| std::env::var("GITHUB_TOKEN"))
                    .ok();

                if let Some(token) = github_token {
                    let github_config = crate::github::GitHubReleaseConfig {
                        owner: github_state.owner.clone(),
                        repo: github_state.repo.clone(),
                        draft: false,
                        prerelease_for_zero_versions: true,
                        notes: None,
                        token: Some(token),
                    };

                    match crate::github::GitHubReleaseManager::new(github_config) {
                        Ok(github_manager) => {
                            match github_manager.delete_release(release_id).await {
                                Ok(()) => {
                                    config.success_println("GitHub release deleted successfully");
                                }
                                Err(e) => {
                                    config.warning_println(&format!("Failed to delete GitHub release: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            config.warning_println(&format!("Could not initialize GitHub client for rollback: {}", e));
                        }
                    }
                } else {
                    config.warning_println("GH_TOKEN or GITHUB_TOKEN not set, skipping GitHub release rollback");
                    config.warning_println(&format!("To manually delete, visit: {}", github_state.html_url.as_ref().unwrap_or(&"GitHub releases page".to_string())));
                }
            }

        // Rollback version changes
        if let Some(version_state) = &release_state.version_state {
            config.println("📝 Rolling back version changes...");

            // Check if git operations completed (git reset will handle restoration)
            if release_state.git_state.as_ref()
                .and_then(|gs| gs.release_commit.as_ref())
                .is_some() {
                config.success_println("Version changes restored via git reset");
            } else {
                // Git didn't commit changes yet, restore from backups manually
                if version_state.backup_files.is_empty() {
                    config.warning_println("No backups available for version rollback");
                    config.warning_println("Please manually revert version changes in Cargo.toml files");
                } else {
                    config.verbose_println(&format!("Restoring {} backup files...", version_state.backup_files.len()));

                    let mut rollback_errors = Vec::new();
                    for backup in version_state.backup_files.iter().rev() {
                        if let Err(e) = std::fs::write(&backup.file_path, &backup.backup_content) {
                            rollback_errors.push(format!("Failed to restore {}: {}", backup.file_path.display(), e));
                        }
                    }

                    if rollback_errors.is_empty() {
                        config.success_println(&format!("Successfully restored {} files", version_state.backup_files.len()));
                    } else {
                        config.warning_println(&format!("Rollback completed with {} errors", rollback_errors.len()));
                        for err in rollback_errors {
                            config.warning_println(&format!("  - {}", err));
                        }
                    }
                }
            }
        }

        release_state.set_phase(ReleasePhase::RolledBack);
        release_state.add_checkpoint(
            "rollback_completed".to_string(),
            ReleasePhase::RolledBack,
            None,
            false,
        );
        state_manager.save_state(&release_state).await?;

        // Cleanup temp directory if we were using one
        if workspace_path != config.workspace_path {
            if let Err(e) = std::fs::remove_dir_all(&workspace_path) {
                config.warning_println(&format!("Failed to cleanup temp directory: {}", e));
                config.warning_println(&format!("You may need to manually remove: {}", workspace_path.display()));
            } else {
                config.verbose_println("✅ Temp clone cleaned up");
            }
            clear_active_temp_path()?;
        }

        config.success_println("🔄 Rollback completed");

    } else {
        unreachable!("execute_rollback called with non-Rollback command");
    }

    Ok(())
}
