//! Version management for Cargo workspaces.
//!
//! This module provides comprehensive version management capabilities including
//! semantic version bumping, workspace synchronization, and TOML editing.

use serde::{Deserialize, Serialize};

mod bumper;
mod toml_editor;
mod updater;

pub use bumper::{BumpPreview, VersionBump, VersionBumper};
pub use toml_editor::{DependencyInfo, DependencySection, TomlBackup, TomlEditor};
pub use updater::{
    ConsistencyReport, DependencyUpdate, InconsistencyType, PackageUpdate, UpdateConfig,
    UpdatePreview, UpdateResult, VersionChange, VersionInconsistency, VersionUpdater,
};

use crate::error::{Result, VersionError};
use crate::workspace::{SharedWorkspaceInfo, WorkspaceInfo};
use semver::Version;
use std::sync::Arc;

/// Unified version manager that orchestrates all version operations
#[derive(Debug)]
pub struct VersionManager {
    /// Workspace information
    workspace: SharedWorkspaceInfo,
    /// Version updater for workspace operations
    updater: VersionUpdater,
}

impl VersionManager {
    /// Create a new version manager for the workspace
    pub fn new(workspace: SharedWorkspaceInfo) -> Self {
        let updater = VersionUpdater::new(workspace.clone());

        Self { workspace, updater }
    }

    /// Perform a complete version release cycle
    pub fn release_version(&mut self, bump: VersionBump) -> Result<ReleaseVersionResult> {
        // Get current workspace version
        let current_version_str = self.workspace.workspace_version()?;
        let current_version =
            Version::parse(&current_version_str).map_err(|e| VersionError::ParseFailed {
                version: current_version_str,
                source: e,
            })?;

        // Calculate new version
        let bumper = VersionBumper::from_version(current_version.clone());
        let new_version = bumper.bump(bump.clone())?;

        // Validate consistency before update
        let consistency_report = self.updater.validate_version_consistency()?;
        if !consistency_report.is_consistent() {
            return Err(VersionError::DependencyMismatch {
                dependency: "workspace".to_string(),
                expected: current_version.to_string(),
                found: format!(
                    "{} inconsistencies",
                    consistency_report.inconsistencies.len()
                ),
            }
            .into());
        }

        // Preview the update
        let preview = self.updater.preview_update(&new_version)?;

        // Perform the update
        let update_config = UpdateConfig::default();
        let update_result = self
            .updater
            .update_workspace_version(&new_version, update_config)?;

        // Note: Backups are not cleared here to allow caller to extract them for state storage
        // Caller should call clear_backups() after saving backups to state

        Ok(ReleaseVersionResult {
            bump_type: bump,
            previous_version: current_version,
            new_version,
            update_result,
            preview,
            consistency_report,
        })
    }

    /// Rollback version changes
    pub fn rollback(&self) -> Result<()> {
        self.updater.rollback_all_changes()
    }

    /// Preview version bump without making changes
    pub fn preview_bump(&self, bump: VersionBump) -> Result<BumpPreviewResult> {
        let current_version_str = self.workspace.workspace_version()?;
        let current_version =
            Version::parse(&current_version_str).map_err(|e| VersionError::ParseFailed {
                version: current_version_str,
                source: e,
            })?;

        let bumper = VersionBumper::from_version(current_version);
        let new_version = bumper.bump(bump.clone())?;
        let bump_preview = bumper.preview_bumps()?;
        let update_preview = self.updater.preview_update(&new_version)?;

        Ok(BumpPreviewResult {
            bump_type: bump,
            bump_preview,
            update_preview,
        })
    }

    /// Validate workspace version consistency
    pub fn validate_consistency(&self) -> Result<ConsistencyReport> {
        self.updater.validate_version_consistency()
    }

    /// Get current workspace version
    pub fn current_version(&self) -> Result<Version> {
        let version_str = self.workspace.workspace_version()?;
        Version::parse(&version_str).map_err(|e| {
            VersionError::ParseFailed {
                version: version_str,
                source: e,
            }
            .into()
        })
    }

    /// Check if workspace uses version inheritance
    pub fn uses_workspace_inheritance(&self) -> Result<WorkspaceInheritanceInfo> {
        let mut packages_using_inheritance = Vec::new();
        let mut packages_with_explicit_versions = Vec::new();

        for (package_name, package_info) in &self.workspace.packages {
            let editor = TomlEditor::open(&package_info.cargo_toml_path)?;

            if editor.uses_workspace_version() {
                packages_using_inheritance.push(package_name.clone());
            } else {
                packages_with_explicit_versions.push(package_name.clone());
            }
        }

        Ok(WorkspaceInheritanceInfo {
            packages_using_inheritance,
            packages_with_explicit_versions,
            total_packages: self.workspace.packages.len(),
        })
    }

    /// Synchronize all package versions to workspace version
    pub fn synchronize_versions(&mut self) -> Result<UpdateResult> {
        let current_version = self.current_version()?;
        let update_config = UpdateConfig {
            update_internal_dependencies: true,
            preserve_workspace_inheritance: false, // Force synchronization
        };

        let result = self
            .updater
            .update_workspace_version(&current_version, update_config)?;
        self.updater.clear_backups();
        Ok(result)
    }

    /// Update internal dependencies to use explicit versions
    pub fn add_explicit_dependency_versions(&mut self) -> Result<UpdateResult> {
        let current_version = self.current_version()?;
        let update_config = UpdateConfig {
            update_internal_dependencies: true,
            preserve_workspace_inheritance: true,
        };

        let result = self
            .updater
            .update_workspace_version(&current_version, update_config)?;
        self.updater.clear_backups();
        Ok(result)
    }

    /// Get workspace information
    pub fn workspace(&self) -> &WorkspaceInfo {
        &self.workspace
    }

    /// Get version updater (for advanced operations)
    pub fn updater(&mut self) -> &mut VersionUpdater {
        &mut self.updater
    }
}

/// Result of a complete version release operation
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseVersionResult {
    /// Type of version bump performed
    pub bump_type: VersionBump,
    /// Previous version
    pub previous_version: Version,
    /// New version
    pub new_version: Version,
    /// Update operation result
    pub update_result: UpdateResult,
    /// Preview of changes made
    pub preview: UpdatePreview,
    /// Consistency report before update
    pub consistency_report: ConsistencyReport,
}

/// Result of version bump preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BumpPreviewResult {
    /// Type of version bump
    pub bump_type: VersionBump,
    /// Preview of version bumps
    pub bump_preview: BumpPreview,
    /// Preview of workspace updates
    pub update_preview: UpdatePreview,
}

/// Information about workspace version inheritance usage
#[derive(Debug, Clone)]
pub struct WorkspaceInheritanceInfo {
    /// Packages using workspace version inheritance
    pub packages_using_inheritance: Vec<String>,
    /// Packages with explicit versions
    pub packages_with_explicit_versions: Vec<String>,
    /// Total number of packages
    pub total_packages: usize,
}

impl ReleaseVersionResult {
    /// Check if the release was successful
    pub fn is_successful(&self) -> bool {
        self.update_result.packages_updated > 0 || self.update_result.dependencies_updated > 0
    }

    /// Get summary of changes made
    pub fn summary(&self) -> String {
        format!(
            "Version {} → {}: {} packages updated, {} dependencies updated, {} files modified",
            self.previous_version,
            self.new_version,
            self.update_result.packages_updated,
            self.update_result.dependencies_updated,
            self.update_result.modified_files.len()
        )
    }

    /// Format detailed report
    pub fn format_report(&self) -> String {
        let mut report = format!(
            "🚀 Version Release: {} → {}\n",
            self.previous_version, self.new_version
        );
        report.push_str(&format!("Bump Type: {}\n\n", self.bump_type));

        report.push_str("📊 Changes Summary:\n");
        report.push_str(&format!(
            "  - Packages updated: {}\n",
            self.update_result.packages_updated
        ));
        report.push_str(&format!(
            "  - Dependencies updated: {}\n",
            self.update_result.dependencies_updated
        ));
        report.push_str(&format!(
            "  - Files modified: {}\n\n",
            self.update_result.modified_files.len()
        ));

        if !self.update_result.modified_files.is_empty() {
            report.push_str("📝 Modified Files:\n");
            for file in &self.update_result.modified_files {
                report.push_str(&format!("  - {}\n", file.display()));
            }
            report.push('\n');
        }

        report.push_str("✅ Pre-update Validation:\n");
        report.push_str(&format!("  {}\n", self.consistency_report.format_report()));

        report
    }
}

impl BumpPreviewResult {
    /// Format preview for display
    pub fn format_preview(&self) -> String {
        let mut preview = format!("🔍 Version Bump Preview ({})\n\n", self.bump_type);

        preview.push_str("📈 Version Options:\n");
        preview.push_str(&format!("  {}\n\n", self.bump_preview.format_preview()));

        preview.push_str("📋 Workspace Changes:\n");
        preview.push_str(&format!("  {}\n", self.update_preview.format_preview()));

        preview
    }
}

impl WorkspaceInheritanceInfo {
    /// Get percentage of packages using inheritance
    pub fn inheritance_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            0.0
        } else {
            (self.packages_using_inheritance.len() as f64 / self.total_packages as f64) * 100.0
        }
    }

    /// Check if all packages use inheritance
    pub fn all_use_inheritance(&self) -> bool {
        self.packages_with_explicit_versions.is_empty()
    }

    /// Check if no packages use inheritance
    pub fn none_use_inheritance(&self) -> bool {
        self.packages_using_inheritance.is_empty()
    }

    /// Format inheritance info for display
    pub fn format_info(&self) -> String {
        format!(
            "Workspace inheritance: {:.1}% ({}/{} packages use inheritance)",
            self.inheritance_percentage(),
            self.packages_using_inheritance.len(),
            self.total_packages
        )
    }
}

// Convenience functions for common version operations

/// Create a version manager for the current directory
pub fn create_version_manager() -> Result<VersionManager> {
    let workspace = Arc::new(WorkspaceInfo::analyze(".")?);
    Ok(VersionManager::new(workspace))
}

/// Quick version bump for the current workspace
pub fn quick_version_bump(bump: VersionBump) -> Result<ReleaseVersionResult> {
    let mut manager = create_version_manager()?;
    manager.release_version(bump)
}

/// Quick version preview for the current workspace
pub fn quick_version_preview(bump: VersionBump) -> Result<BumpPreviewResult> {
    let manager = create_version_manager()?;
    manager.preview_bump(bump)
}

/// Quick consistency check for the current workspace
pub fn quick_consistency_check() -> Result<ConsistencyReport> {
    let manager = create_version_manager()?;
    manager.validate_consistency()
}
