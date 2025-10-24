//! Validate command implementation.
//!
//! Validates the workspace structure and configuration for release readiness.

use crate::cli::{Args, Command, RuntimeConfig};
use crate::error::{ReleaseError, Result};
use crate::workspace::{SharedWorkspaceInfo, WorkspaceInfo, WorkspaceValidator};
use std::sync::Arc;

/// Execute validate command
pub(super) async fn execute_validate(args: &Args, config: &RuntimeConfig) -> Result<()> {
    if let Command::Validate {
        fix: _,
        detailed,
        json,
    } = &args.command
    {
        config.verbose_println("Validating workspace...");

        let workspace: SharedWorkspaceInfo =
            Arc::new(WorkspaceInfo::analyze(&config.workspace_path)?);
        let validator = WorkspaceValidator::new(workspace.clone())?;
        let validation = validator.validate().await?;

        if *json {
            let json_output =
                serde_json::to_string_pretty(&validation).map_err(ReleaseError::Json)?;
            println!("{}", json_output);
        } else {
            config.println(&format!("📋 {}", validation.summary()));

            if *detailed {
                for check in &validation.checks {
                    config.println(&format!("  {}", check.format_result()));
                }
            }

            if !validation.warnings.is_empty() && !config.is_quiet() {
                config.println("\n⚠️ Warnings:");
                for warning in &validation.warnings {
                    config.warning_println(&format!("  • {}", warning));
                }
            }

            if !validation.critical_errors.is_empty() {
                config.println("\n❌ Critical Errors:");
                for error in &validation.critical_errors {
                    config.error_println(&format!("  • {}", error));
                }
            }
        }

        if !validation.success {
            return Err(ReleaseError::Workspace(
                crate::error::WorkspaceError::InvalidStructure {
                    reason: "Workspace validation failed".to_string(),
                },
            ));
        }
    } else {
        unreachable!("execute_validate called with non-Validate command");
    }

    Ok(())
}
