//! Resume command implementation.
//!
//! Handles resuming an interrupted or failed release from the last successful checkpoint.

use crate::cli::{Args, Command, ResumePhase, RuntimeConfig};
use crate::error::{ReleaseError, Result};
use crate::git::{GitConfig, GitManager};
use crate::publish::{Publisher, PublisherConfig};
use crate::state::{ReleasePhase, create_state_manager_at};
use crate::version::VersionManager;
use crate::workspace::{WorkspaceInfo, WorkspaceValidator};
use std::sync::Arc;
use std::time::Duration;

use super::temp_clone::{clear_active_temp_path, get_active_temp_path};

/// Execute resume command
pub(super) async fn execute_resume(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Resume {
        force,
        reset_to_phase,
        skip_validation: _,
    } = &args.command
    {
        config.verbose_println("Resuming release operation...");

        // Check if there's an active temp clone from a previous release
        let (workspace_path, state_file_path) = if let Some(temp_path) = get_active_temp_path() {
            if temp_path.exists() {
                config.println(&format!(
                    "📂 Resuming from temp clone: {}",
                    temp_path.display()
                ));
                let temp_state = temp_path.join(".cyrup_release_state.json");
                (temp_path, temp_state)
            } else {
                config.warning_println(
                    "Tracked temp clone no longer exists, using current workspace",
                );
                clear_active_temp_path()?;
                (
                    config.workspace_path.clone(),
                    config.state_file_path.clone(),
                )
            }
        } else {
            (
                config.workspace_path.clone(),
                config.state_file_path.clone(),
            )
        };

        // Load release state
        let mut state_manager = create_state_manager_at(&state_file_path)?;
        let load_result = state_manager.load_state().await?;
        let mut release_state = load_result.state;

        // Validate resumability
        if !release_state.is_resumable() && !force {
            return Err(ReleaseError::State(crate::error::StateError::LoadFailed {
                reason: "Release is not in a resumable state. Use --force to resume anyway"
                    .to_string(),
            }));
        }

        if release_state.has_critical_errors() && !force {
            return Err(ReleaseError::State(crate::error::StateError::Corrupted {
                reason: "Release has critical errors. Use --force to resume anyway".to_string(),
            }));
        }

        // Reset to specific phase if requested
        if let Some(reset_phase) = reset_to_phase {
            let new_phase = match reset_phase {
                ResumePhase::Validation => ReleasePhase::Validation,
                ResumePhase::VersionUpdate => ReleasePhase::VersionUpdate,
                ResumePhase::GitOperations => ReleasePhase::GitOperations,
                ResumePhase::Publishing => ReleasePhase::Publishing,
            };

            config.println(&format!("Resetting to phase: {:?}", new_phase));
            release_state.set_phase(new_phase);
            state_manager.save_state(&release_state).await?;
        }

        config.println(&format!(
            "Resuming release {} from phase: {:?}",
            release_state.target_version, release_state.current_phase
        ));

        // Reconstruct workspace (needed for all phases)
        let workspace = Arc::new(WorkspaceInfo::analyze(&workspace_path)?);

        // Execute phases in order until completion
        loop {
            match release_state.current_phase {
                ReleasePhase::Validation => {
                    config.println("Re-validating workspace...");

                    // Perform validation
                    let validator = WorkspaceValidator::new(workspace.clone())?;
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

                    // Move to next phase
                    release_state.set_phase(ReleasePhase::VersionUpdate);
                    state_manager.save_state(&release_state).await?;
                }

                ReleasePhase::VersionUpdate => {
                    config.println("Continuing version update...");

                    // Initialize version manager
                    let mut version_manager = VersionManager::new(workspace.clone());

                    // Perform version update (using stored version_bump from state)
                    let version_result =
                        version_manager.release_version(release_state.version_bump.clone())?;
                    release_state.set_version_state(&version_result.update_result);
                    release_state.add_checkpoint(
                        "version_updated".to_string(),
                        ReleasePhase::VersionUpdate,
                        None,
                        true,
                    );
                    state_manager.save_state(&release_state).await?;

                    config
                        .success_println(&format!("Version updated: {}", version_result.summary()));

                    // Move to next phase
                    release_state.set_phase(ReleasePhase::GitOperations);
                    state_manager.save_state(&release_state).await?;
                }

                ReleasePhase::GitOperations => {
                    config.println("Continuing git operations...");

                    // Reconstruct git manager from original config
                    let git_config = GitConfig {
                        default_remote: "origin".to_string(),
                        annotated_tags: true,
                        auto_push_tags: release_state.config.push_to_remote,
                        ..Default::default()
                    };
                    let mut git_manager =
                        GitManager::with_config(&workspace_path, git_config).await?;

                    // Perform git operations
                    let git_result = git_manager
                        .perform_release(
                            &release_state.target_version,
                            release_state.config.push_to_remote,
                        )
                        .await?;

                    release_state.set_git_state(Some(&git_result.commit), Some(&git_result.tag));

                    if let Some(push_info) = &git_result.push_info {
                        release_state.set_git_push_state(push_info);
                    }

                    release_state.add_checkpoint(
                        "git_operations_complete".to_string(),
                        ReleasePhase::GitOperations,
                        None,
                        true,
                    );
                    state_manager.save_state(&release_state).await?;

                    config.success_println(&format!(
                        "Git operations completed: {}",
                        git_result.format_result()
                    ));

                    // Move to next phase
                    release_state.set_phase(ReleasePhase::Publishing);
                    state_manager.save_state(&release_state).await?;
                }

                ReleasePhase::Publishing => {
                    config.println("Continuing publishing...");

                    // Reconstruct publisher from original config
                    let publisher_config = PublisherConfig {
                        inter_package_delay: Duration::from_millis(
                            release_state.config.inter_package_delay_ms,
                        ),
                        registry: release_state.config.registry.clone(),
                        max_concurrent_per_tier: 1,
                        ..Default::default()
                    };
                    let mut publisher =
                        Publisher::with_config(workspace.clone(), publisher_config)?;

                    // Initialize publish state if not already done
                    if release_state.publish_state.is_none() {
                        let publish_order = crate::workspace::DependencyGraph::build(&workspace)?
                            .publish_order()?;
                        release_state.init_publish_state(publish_order.tier_count());
                        state_manager.save_state(&release_state).await?;
                    }

                    // Perform publishing
                    let publish_result = publisher.publish_all_packages(config).await?;

                    // Update state with results
                    for package_result in publish_result.successful_publishes.values() {
                        release_state.add_published_package(package_result);
                    }

                    for (package_name, error) in &publish_result.failed_packages {
                        release_state.add_failed_package(package_name.clone(), error.clone());
                    }

                    release_state.add_checkpoint(
                        "publishing_complete".to_string(),
                        ReleasePhase::Publishing,
                        None,
                        true,
                    );
                    state_manager.save_state(&release_state).await?;

                    if publish_result.all_successful {
                        config.success_println(&format!(
                            "Publishing completed: {}",
                            publish_result.format_summary()
                        ));
                    } else {
                        config.warning_println(&format!(
                            "Publishing partially failed: {}",
                            publish_result.format_summary()
                        ));
                    }

                    // Move to cleanup and exit loop
                    release_state.set_phase(ReleasePhase::Cleanup);
                    state_manager.save_state(&release_state).await?;
                    break;
                }

                ReleasePhase::Cleanup | ReleasePhase::Completed => {
                    // Already in cleanup or completed phase, exit loop
                    break;
                }

                _ => {
                    return Err(ReleaseError::State(crate::error::StateError::Corrupted {
                        reason: format!(
                            "Cannot resume from phase: {:?}",
                            release_state.current_phase
                        ),
                    }));
                }
            }
        }

        // Cleanup phase (same as execute_release)
        config.println("🧹 Cleaning up...");
        release_state.set_phase(ReleasePhase::Cleanup);
        state_manager.save_state(&release_state).await?;

        // Mark as completed
        release_state.set_phase(ReleasePhase::Completed);
        release_state.add_checkpoint(
            "release_completed".to_string(),
            ReleasePhase::Completed,
            None,
            false,
        );
        state_manager.save_state(&release_state).await?;

        config.success_println(&format!(
            "🎉 Release {} resumed and completed successfully!",
            release_state.target_version
        ));

        // Cleanup state file after successful completion
        state_manager.cleanup_state()?;
    } else {
        unreachable!("execute_resume called with non-Resume command");
    }

    Ok(())
}
