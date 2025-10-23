//! Workspace version synchronization and update coordination.
//!
//! This module orchestrates atomic version updates across the entire workspace,
//! ensuring internal dependencies are properly synchronized.

use crate::error::{Result, VersionError};
use crate::version::{TomlEditor, TomlBackup};
use crate::workspace::{WorkspaceInfo, SharedWorkspaceInfo};
use semver::Version;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Coordinates version updates across the entire workspace
#[derive(Debug)]
pub struct VersionUpdater {
    /// Workspace information
    workspace: SharedWorkspaceInfo,
    /// Created backups for rollback
    backups: Vec<TomlBackup>,
}

/// Result of a version update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    /// Previous version before update
    pub previous_version: Version,
    /// New version after update
    pub new_version: Version,
    /// Number of packages updated
    pub packages_updated: usize,
    /// Number of internal dependencies updated
    pub dependencies_updated: usize,
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
}

/// Configuration for version update operations
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Whether to update internal dependency versions
    pub update_internal_dependencies: bool,
    /// Whether to preserve workspace inheritance where possible
    pub preserve_workspace_inheritance: bool,
}

/// Tracking statistics for update operations
struct UpdateStats {
    modified_files: Vec<PathBuf>,
    packages_updated: usize,
    dependencies_updated: usize,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            update_internal_dependencies: true,
            preserve_workspace_inheritance: true,
        }
    }
}

impl VersionUpdater {
    /// Create a new version updater for the workspace
    pub fn new(workspace: SharedWorkspaceInfo) -> Self {
        Self {
            workspace,
            backups: Vec::new(),
        }
    }

    /// Update workspace to new version with atomic operation
    pub fn update_workspace_version(
        &mut self,
        new_version: &Version,
        config: UpdateConfig,
    ) -> Result<UpdateResult> {
        let current_version = self.workspace.workspace_version()
            .and_then(|v| Version::parse(&v).map_err(|e| VersionError::ParseFailed {
                version: v,
                source: e,
            }.into()))?;

        // Validate version progression
        if new_version <= &current_version {
            return Err(VersionError::InvalidVersion {
                version: new_version.to_string(),
                reason: format!(
                    "New version '{}' must be greater than current version '{}'",
                    new_version, current_version
                ),
            }.into());
        }

        let mut stats = UpdateStats {
            modified_files: Vec::new(),
            packages_updated: 0,
            dependencies_updated: 0,
        };

        // Update workspace version in root Cargo.toml
        if let Err(e) = self.update_root_workspace_version(new_version, &config, &mut stats.modified_files) {
            self.rollback_all_changes()?;
            return Err(e);
        }

        // Update all packages in the workspace
        // Collect package info first to avoid borrow checker issues
        let packages: Vec<_> = self.workspace.packages.iter()
            .map(|(name, info)| (name.clone(), info.clone()))
            .collect();
        
        for (package_name, package_info) in packages {
            match self.update_package_version(
                &package_name,
                &package_info,
                new_version,
                &config,
                &mut stats,
            ) {
                Ok(()) => {}
                Err(e) => {
                    self.rollback_all_changes()?;
                    return Err(e);
                }
            }
        }

        Ok(UpdateResult {
            previous_version: current_version,
            new_version: new_version.clone(),
            packages_updated: stats.packages_updated,
            dependencies_updated: stats.dependencies_updated,
            modified_files: stats.modified_files,
        })
    }

    /// Update root workspace version
    fn update_root_workspace_version(
        &mut self,
        new_version: &Version,
        _config: &UpdateConfig,
        modified_files: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let workspace_cargo_toml = self.workspace.root.join("Cargo.toml");
        let mut editor = TomlEditor::open(&workspace_cargo_toml)?;

        // Update workspace version
        editor.update_workspace_version(new_version)?;
        editor.save()?;

        modified_files.push(workspace_cargo_toml);
        Ok(())
    }

    /// Update a single package and its dependencies
    fn update_package_version(
        &mut self,
        _package_name: &str,
        package_info: &crate::workspace::PackageInfo,
        new_version: &Version,
        config: &UpdateConfig,
        stats: &mut UpdateStats,
    ) -> Result<()> {
        let mut editor = TomlEditor::open(&package_info.cargo_toml_path)?;

        let mut package_modified = false;

        // Update package version if not using workspace inheritance
        if !editor.uses_workspace_version() || !config.preserve_workspace_inheritance {
            editor.update_package_version(new_version)?;
            package_modified = true;
            stats.packages_updated += 1;
        }

        // Update internal dependency versions if requested
        if config.update_internal_dependencies {
            let internal_deps_to_update = self.collect_internal_dependencies_to_update(package_info, new_version);
            
            for (dep_name, dep_version) in internal_deps_to_update {
                editor.update_dependency_version(&dep_name, &dep_version)?;
                package_modified = true;
                stats.dependencies_updated += 1;
            }
        }

        // Save changes if any modifications were made
        if package_modified {
            editor.save()?;
            stats.modified_files.push(package_info.cargo_toml_path.clone());
        }

        Ok(())
    }

    /// Collect internal dependencies that need version updates
    fn collect_internal_dependencies_to_update(
        &self,
        package_info: &crate::workspace::PackageInfo,
        new_version: &Version,
    ) -> HashMap<String, Version> {
        let mut updates = HashMap::new();

        for dep_name in &package_info.workspace_dependencies {
            if self.workspace.packages.contains_key(dep_name) {
                // This is an internal workspace dependency, update its version
                updates.insert(dep_name.clone(), new_version.clone());
            }
        }

        updates
    }

    /// Rollback all changes made during the update
    pub fn rollback_all_changes(&self) -> Result<()> {
        let mut rollback_errors = Vec::new();

        // Restore all backups in reverse order
        for backup in self.backups.iter().rev() {
            if let Err(e) = TomlEditor::restore_from_backup(backup) {
                rollback_errors.push(format!("Failed to restore {}: {}", backup.file_path.display(), e));
            }
        }

        if !rollback_errors.is_empty() {
            return Err(VersionError::TomlUpdateFailed {
                path: PathBuf::from("multiple_files"),
                reason: format!("Rollback failures: {}", rollback_errors.join("; ")),
            }.into());
        }

        Ok(())
    }

    /// Clear all backups (call after successful operation)
    pub fn clear_backups(&mut self) {
        self.backups.clear();
    }

    /// Get current backups (before clearing)
    pub fn get_backups(&self) -> Vec<TomlBackup> {
        self.backups.clone()
    }

    /// Validate workspace version consistency
    pub fn validate_version_consistency(&self) -> Result<ConsistencyReport> {
        let workspace_version = self.workspace.workspace_version()
            .and_then(|v| Version::parse(&v).map_err(|e| VersionError::ParseFailed {
                version: v,
                source: e,
            }.into()))?;

        let mut inconsistencies = Vec::new();
        let mut packages_checked = 0;
        let mut dependencies_checked = 0;

        for (package_name, package_info) in &self.workspace.packages {
            packages_checked += 1;

            // Check package version consistency
            if let Ok(package_version) = Version::parse(&package_info.version)
                && package_version != workspace_version {
                    inconsistencies.push(VersionInconsistency {
                        package: package_name.clone(),
                        dependency: None,
                        expected_version: workspace_version.clone(),
                        actual_version: package_version,
                        inconsistency_type: InconsistencyType::PackageVersion,
                    });
                }

            // Check internal dependency versions
            for dep_name in &package_info.workspace_dependencies {
                dependencies_checked += 1;

                if let Some(dep_spec) = package_info.all_dependencies.get(dep_name) {
                    if let Some(dep_version_str) = &dep_spec.version
                        && let Ok(dep_version) = Version::parse(dep_version_str)
                        && dep_version != workspace_version {
                            inconsistencies.push(VersionInconsistency {
                                package: package_name.clone(),
                                dependency: Some(dep_name.clone()),
                                expected_version: workspace_version.clone(),
                                actual_version: dep_version,
                                inconsistency_type: InconsistencyType::DependencyVersion,
                            });
                        } else if dep_spec.version.is_none() {
                            // Missing version specification for internal dependency
                            inconsistencies.push(VersionInconsistency {
                                package: package_name.clone(),
                                dependency: Some(dep_name.clone()),
                                expected_version: workspace_version.clone(),
                                actual_version: Version::new(0, 0, 0), // Placeholder
                                inconsistency_type: InconsistencyType::MissingVersion,
                            });
                        }
                    }
            }
        }

        Ok(ConsistencyReport {
            workspace_version,
            packages_checked,
            dependencies_checked,
            inconsistencies,
        })
    }

    /// Preview version update without making changes
    pub fn preview_update(&self, new_version: &Version) -> Result<UpdatePreview> {
        let current_version = self.workspace.workspace_version()
            .and_then(|v| Version::parse(&v).map_err(|e| VersionError::ParseFailed {
                version: v,
                source: e,
            }.into()))?;

        let mut files_to_modify = Vec::new();
        let mut packages_to_update = Vec::new();
        let mut dependencies_to_update = Vec::new();

        // Root workspace file
        files_to_modify.push(self.workspace.root.join("Cargo.toml"));

        // Check each package
        for (package_name, package_info) in &self.workspace.packages {
            let editor = TomlEditor::open(&package_info.cargo_toml_path)?;
            
            let mut package_changes = Vec::new();
            
            // Package version change
            if !editor.uses_workspace_version() {
                package_changes.push(VersionChange {
                    field: "version".to_string(),
                    from: current_version.clone(),
                    to: new_version.clone(),
                });
            }

            // Internal dependency changes
            for dep_name in &package_info.workspace_dependencies {
                if self.workspace.packages.contains_key(dep_name) {
                    package_changes.push(VersionChange {
                        field: format!("dependencies.{}", dep_name),
                        from: current_version.clone(),
                        to: new_version.clone(),
                    });
                    dependencies_to_update.push(DependencyUpdate {
                        package: package_name.clone(),
                        dependency: dep_name.clone(),
                        from: current_version.clone(),
                        to: new_version.clone(),
                    });
                }
            }

            if !package_changes.is_empty() {
                files_to_modify.push(package_info.cargo_toml_path.clone());
                packages_to_update.push(PackageUpdate {
                    name: package_name.clone(),
                    file_path: package_info.cargo_toml_path.clone(),
                    changes: package_changes,
                });
            }
        }

        Ok(UpdatePreview {
            from_version: current_version,
            to_version: new_version.clone(),
            files_to_modify,
            packages_to_update,
            dependencies_to_update,
        })
    }

    /// Get workspace information
    pub fn workspace(&self) -> &WorkspaceInfo {
        &self.workspace
    }

    /// Get number of backups created
    pub fn backup_count(&self) -> usize {
        self.backups.len()
    }
}

/// Report of version consistency across the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    /// Expected workspace version
    pub workspace_version: Version,
    /// Number of packages checked
    pub packages_checked: usize,
    /// Number of dependencies checked
    pub dependencies_checked: usize,
    /// Found inconsistencies
    pub inconsistencies: Vec<VersionInconsistency>,
}

/// A version inconsistency found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInconsistency {
    /// Package where inconsistency was found
    pub package: String,
    /// Dependency name (if inconsistency is in dependency)
    pub dependency: Option<String>,
    /// Expected version
    pub expected_version: Version,
    /// Actual version found
    pub actual_version: Version,
    /// Type of inconsistency
    pub inconsistency_type: InconsistencyType,
}

/// Type of version inconsistency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InconsistencyType {
    /// Package version doesn't match workspace version
    PackageVersion,
    /// Internal dependency version doesn't match expected version
    DependencyVersion,
    /// Missing version specification for internal dependency
    MissingVersion,
}

/// Preview of version update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreview {
    /// Current version
    pub from_version: Version,
    /// Target version
    pub to_version: Version,
    /// Files that will be modified
    pub files_to_modify: Vec<PathBuf>,
    /// Packages that will be updated
    pub packages_to_update: Vec<PackageUpdate>,
    /// Dependencies that will be updated
    pub dependencies_to_update: Vec<DependencyUpdate>,
}

/// Preview of package update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUpdate {
    /// Package name
    pub name: String,
    /// Path to Cargo.toml file
    pub file_path: PathBuf,
    /// List of changes to be made
    pub changes: Vec<VersionChange>,
}

/// Preview of dependency update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyUpdate {
    /// Package containing the dependency
    pub package: String,
    /// Dependency name
    pub dependency: String,
    /// Current version
    pub from: Version,
    /// Target version
    pub to: Version,
}

/// A single version change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionChange {
    /// Field being changed (e.g., "version", "dependencies.foo")
    pub field: String,
    /// Current version
    pub from: Version,
    /// Target version
    pub to: Version,
}

impl ConsistencyReport {
    /// Check if the workspace has any version inconsistencies
    ///
    /// Note: MissingVersion inconsistencies are not considered blocking, as they will be
    /// fixed during the update process (update_dependency_version adds missing versions).
    /// Only PackageVersion and DependencyVersion mismatches are blocking.
    pub fn is_consistent(&self) -> bool {
        !self.inconsistencies.iter().any(|inc| {
            matches!(inc.inconsistency_type,
                InconsistencyType::PackageVersion | InconsistencyType::DependencyVersion)
        })
    }

    /// Get inconsistencies by type
    pub fn inconsistencies_by_type(&self, inconsistency_type: InconsistencyType) -> Vec<&VersionInconsistency> {
        self.inconsistencies
            .iter()
            .filter(|inc| inc.inconsistency_type == inconsistency_type)
            .collect()
    }

    /// Format report for display
    pub fn format_report(&self) -> String {
        if self.is_consistent() {
            format!(
                "✅ Workspace version consistency: {} packages and {} dependencies are all at version {}",
                self.packages_checked, self.dependencies_checked, self.workspace_version
            )
        } else {
            let mut report = format!(
                "❌ Found {} version inconsistencies (workspace version: {})\n",
                self.inconsistencies.len(), self.workspace_version
            );

            for inconsistency in &self.inconsistencies {
                let location = match &inconsistency.dependency {
                    Some(dep) => format!("{}::{}", inconsistency.package, dep),
                    None => inconsistency.package.clone(),
                };

                report.push_str(&format!(
                    "  - {}: expected {}, found {}\n",
                    location, inconsistency.expected_version, inconsistency.actual_version
                ));
            }

            report
        }
    }
}

impl UpdatePreview {
    /// Get total number of changes
    pub fn total_changes(&self) -> usize {
        self.packages_to_update.iter().map(|p| p.changes.len()).sum::<usize>()
            + self.dependencies_to_update.len()
    }

    /// Format preview for display
    pub fn format_preview(&self) -> String {
        let mut preview = format!(
            "Version update preview: {} → {}\n",
            self.from_version, self.to_version
        );

        preview.push_str(&format!("Files to modify: {}\n", self.files_to_modify.len()));
        preview.push_str(&format!("Packages to update: {}\n", self.packages_to_update.len()));
        preview.push_str(&format!("Dependencies to update: {}\n", self.dependencies_to_update.len()));
        preview.push_str(&format!("Total changes: {}\n", self.total_changes()));

        if !self.packages_to_update.is_empty() {
            preview.push_str("\nPackages:\n");
            for package in &self.packages_to_update {
                preview.push_str(&format!("  - {}\n", package.name));
            }
        }

        preview
    }
}