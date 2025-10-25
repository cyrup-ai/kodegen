//! Preview command implementation.
//!
//! Previews version changes before performing a release.

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{CliError, ReleaseError, Result};
use crate::version::{VersionBump, VersionManager};
use crate::workspace::{SharedWorkspaceInfo, WorkspaceInfo};
use std::sync::Arc;

/// Execute preview command
pub(super) async fn execute_preview(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Preview {
        bump_type,
        detailed,
        json,
    } = &args.command
    {
        config.verbose_println("Previewing version bump...");

        let workspace: SharedWorkspaceInfo =
            Arc::new(WorkspaceInfo::analyze(&config.workspace_path)?);
        let version_manager = VersionManager::new(workspace.clone());

        let version_bump = VersionBump::try_from(bump_type.clone())
            .map_err(|e| ReleaseError::Cli(CliError::InvalidArguments { reason: e }))?;

        let preview = version_manager.preview_bump(version_bump.clone())?;

        if *json {
            let json_output = serde_json::to_string_pretty(&preview).map_err(ReleaseError::Json)?;
            println!("{}", json_output);
        } else {
            config.println(&format!("🔍 {}", preview.format_preview()));

            if *detailed {
                config.println("\nDetailed changes:");
                let new_version = preview.bump_preview.get_version(&version_bump)
                    .ok_or_else(|| ReleaseError::Version(crate::error::VersionError::InvalidVersion {
                        version: preview.bump_preview.current.to_string(),
                        reason: format!("Failed to calculate new version for bump type: {:?}", version_bump),
                    }))?;
                config.println(&format!(
                    "  Version: {} → {}",
                    preview.bump_preview.current,
                    new_version
                ));

                config.println(&format!(
                    "  Files to modify: {}",
                    preview.update_preview.files_to_modify.len()
                ));
                for file in &preview.update_preview.files_to_modify {
                    config.println(&format!("    • {}", file.display()));
                }

                // Show publish order
                config.println("\nPublish Order:");
                match crate::workspace::DependencyGraph::build(&workspace) {
                    Ok(dep_graph) => match dep_graph.publish_order() {
                        Ok(publish_order) => {
                            config.println(&format!(
                                "  Total packages: {} (in {} tiers)\n",
                                publish_order.total_packages,
                                publish_order.tier_count()
                            ));
                            for tier in &publish_order.tiers {
                                config.println(&format!(
                                    "  Tier {} ({} package{}):",
                                    tier.tier_number,
                                    tier.packages.len(),
                                    if tier.packages.len() == 1 { "" } else { "s" }
                                ));
                                for pkg in &tier.packages {
                                    let dependents = dep_graph.dependents(pkg);
                                    config.println(&format!(
                                        "    • {} ({} dependent{})",
                                        pkg,
                                        dependents.len(),
                                        if dependents.len() == 1 { "" } else { "s" }
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            config
                                .error_println(&format!("Failed to compute publish order: {}", e));
                        }
                    },
                    Err(e) => {
                        config.error_println(&format!("Failed to build dependency graph: {}", e));
                    }
                }
            }
        }
    } else {
        unreachable!("execute_preview called with non-Preview command");
    }

    Ok(())
}
