//! Format-preserving TOML editing for Cargo.toml files.
//!
//! This module provides surgical editing of TOML files while preserving comments,
//! formatting, and structure using the toml_edit crate.

use crate::error::{Result, VersionError};
use semver::Version;
use std::collections::HashMap;
use std::path::Path;
use toml_edit::{DocumentMut, InlineTable, Item, Value};

/// Format-preserving TOML editor for Cargo.toml files
#[derive(Debug)]
pub struct TomlEditor {
    /// Path to the TOML file being edited
    file_path: std::path::PathBuf,
    /// Parsed TOML document
    document: DocumentMut,
    /// Original file content for rollback
    original_content: String,
}

/// Backup of TOML file state for rollback
#[derive(Debug, Clone)]
pub struct TomlBackup {
    /// File path
    pub file_path: std::path::PathBuf,
    /// Original content
    pub content: String,
}

impl TomlEditor {
    /// Open a TOML file for editing
    pub fn open<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&file_path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Failed to read {}: {}", file_path.display(), e),
            )
        })?;

        let document =
            content
                .parse::<DocumentMut>()
                .map_err(|e| VersionError::TomlUpdateFailed {
                    path: file_path.clone(),
                    reason: format!("Failed to parse TOML: {}", e),
                })?;

        Ok(Self {
            file_path,
            document,
            original_content: content,
        })
    }

    /// Update package version in [package] section
    pub fn update_package_version(&mut self, new_version: &Version) -> Result<()> {
        let version_str = new_version.to_string();

        // Navigate to [package] section
        let package_table = self
            .document
            .get_mut("package")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| VersionError::TomlUpdateFailed {
                path: self.file_path.clone(),
                reason: "No [package] section found".to_string(),
            })?;

        // Update version field
        match package_table.get_mut("version") {
            Some(version_item) => {
                match version_item {
                    Item::Value(Value::String(formatted_string)) => {
                        // Preserve formatting (quotes, etc.)
                        *formatted_string = toml_edit::Formatted::new(version_str);
                    }
                    Item::Value(Value::InlineTable(table)) => {
                        // Handle inline table workspace inheritance: version = { workspace = true }
                        if table.contains_key("workspace") {
                            // Version is inherited from workspace, don't modify
                            return Ok(());
                        }
                        return Err(VersionError::TomlUpdateFailed {
                            path: self.file_path.clone(),
                            reason: "Unexpected version format in inline table".to_string(),
                        }
                        .into());
                    }
                    Item::Table(table) => {
                        // Handle dotted key workspace inheritance: version.workspace = true
                        if table.contains_key("workspace") {
                            // Version is inherited from workspace, don't modify
                            return Ok(());
                        }
                        return Err(VersionError::TomlUpdateFailed {
                            path: self.file_path.clone(),
                            reason: "Unexpected version table format".to_string(),
                        }
                        .into());
                    }
                    _ => {
                        return Err(VersionError::TomlUpdateFailed {
                            path: self.file_path.clone(),
                            reason: format!(
                                "Version field has unexpected format: {:?}",
                                version_item
                            ),
                        }
                        .into());
                    }
                }
            }
            None => {
                // Add version field if it doesn't exist
                package_table.insert("version", toml_edit::value(version_str));
            }
        }

        Ok(())
    }

    /// Update workspace version in [workspace.package] section
    pub fn update_workspace_version(&mut self, new_version: &Version) -> Result<()> {
        let version_str = new_version.to_string();

        // Navigate to [workspace] section
        let workspace_table = self
            .document
            .get_mut("workspace")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| VersionError::TomlUpdateFailed {
                path: self.file_path.clone(),
                reason: "No [workspace] section found".to_string(),
            })?;

        // Navigate to [workspace.package] section
        let package_table = workspace_table
            .get_mut("package")
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| VersionError::TomlUpdateFailed {
                path: self.file_path.clone(),
                reason: "No [workspace.package] section found".to_string(),
            })?;

        // Update version field
        match package_table.get_mut("version") {
            Some(version_item) => {
                if let Item::Value(Value::String(formatted_string)) = version_item {
                    *formatted_string = toml_edit::Formatted::new(version_str);
                } else {
                    return Err(VersionError::TomlUpdateFailed {
                        path: self.file_path.clone(),
                        reason: "Workspace version field has unexpected format".to_string(),
                    }
                    .into());
                }
            }
            None => {
                // Add version field if it doesn't exist
                package_table.insert("version", toml_edit::value(version_str));
            }
        }

        Ok(())
    }

    /// Update internal dependency version
    pub fn update_dependency_version(
        &mut self,
        dependency_name: &str,
        new_version: &Version,
    ) -> Result<()> {
        let version_str = new_version.to_string();
        let mut updated = false;

        // Update in [dependencies] section
        if let Some(deps_table) = self
            .document
            .get_mut("dependencies")
            .and_then(|item| item.as_table_mut())
            && let Some(dep_item) = deps_table.get_mut(dependency_name)
        {
            Self::update_dependency_item(&self.file_path, dep_item, &version_str)?;
            updated = true;
        }

        // Update in [dev-dependencies] section
        if let Some(dev_deps_table) = self
            .document
            .get_mut("dev-dependencies")
            .and_then(|item| item.as_table_mut())
            && let Some(dep_item) = dev_deps_table.get_mut(dependency_name)
        {
            Self::update_dependency_item(&self.file_path, dep_item, &version_str)?;
            updated = true;
        }

        // Update in [build-dependencies] section
        if let Some(build_deps_table) = self
            .document
            .get_mut("build-dependencies")
            .and_then(|item| item.as_table_mut())
            && let Some(dep_item) = build_deps_table.get_mut(dependency_name)
        {
            Self::update_dependency_item(&self.file_path, dep_item, &version_str)?;
            updated = true;
        }

        if !updated {
            return Err(VersionError::DependencyMismatch {
                dependency: dependency_name.to_string(),
                expected: version_str,
                found: "not found".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Update a single dependency item (handles different formats)
    fn update_dependency_item(
        file_path: &Path,
        dep_item: &mut Item,
        version_str: &str,
    ) -> Result<()> {
        match dep_item {
            Item::Value(Value::String(version_ref)) => {
                // Simple string dependency: "1.0.0"
                *version_ref = toml_edit::Formatted::new(version_str.to_string());
            }
            Item::Value(Value::InlineTable(table)) => {
                // Inline table dependency: { version = "1.0.0", path = "../path" }
                if table.contains_key("version") {
                    table.insert("version", toml_edit::Value::from(version_str));
                } else {
                    // Add version if only path was specified
                    table.insert("version", toml_edit::Value::from(version_str));
                }
            }
            Item::Table(table) => {
                // Table dependency
                if table.contains_key("version") {
                    table.insert("version", toml_edit::value(version_str));
                } else {
                    // Add version if only path was specified
                    table.insert("version", toml_edit::value(version_str));
                }
            }
            _ => {
                return Err(VersionError::TomlUpdateFailed {
                    path: file_path.to_path_buf(),
                    reason: format!("Unexpected dependency format for item: {:?}", dep_item),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Update multiple dependency versions in batch
    pub fn update_multiple_dependencies(
        &mut self,
        updates: &HashMap<String, Version>,
    ) -> Result<()> {
        for (dep_name, version) in updates {
            self.update_dependency_version(dep_name, version)?;
        }
        Ok(())
    }

    /// Add a new dependency
    pub fn add_dependency(
        &mut self,
        dependency_name: &str,
        version: &Version,
        section: DependencySection,
        additional_fields: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let section_name = section.section_name();

        // Ensure the section exists
        if !self.document.contains_key(section_name) {
            self.document.insert(
                section_name,
                toml_edit::Item::Table(toml_edit::Table::new()),
            );
        }

        let deps_table = self
            .document
            .get_mut(section_name)
            .and_then(|item| item.as_table_mut())
            .ok_or_else(|| VersionError::TomlUpdateFailed {
                path: self.file_path.clone(),
                reason: format!("Failed to access {} section", section_name),
            })?;

        // Create dependency specification
        if let Some(additional) = additional_fields {
            // Create inline table with version and additional fields
            let mut inline_table = InlineTable::new();
            inline_table.insert("version", toml_edit::Value::from(version.to_string()));

            for (key, value) in additional {
                inline_table.insert(&key, toml_edit::Value::from(value));
            }

            deps_table.insert(
                dependency_name,
                toml_edit::Item::Value(Value::InlineTable(inline_table)),
            );
        } else {
            // Simple string version
            deps_table.insert(dependency_name, toml_edit::value(version.to_string()));
        }

        Ok(())
    }

    /// Remove a dependency
    pub fn remove_dependency(
        &mut self,
        dependency_name: &str,
        section: DependencySection,
    ) -> Result<bool> {
        let section_name = section.section_name();

        if let Some(deps_table) = self
            .document
            .get_mut(section_name)
            .and_then(|item| item.as_table_mut())
        {
            Ok(deps_table.remove(dependency_name).is_some())
        } else {
            Ok(false)
        }
    }

    /// Get current version from the TOML
    pub fn get_current_version(&self) -> Result<Version> {
        // Try package version first
        if let Some(package_table) = self
            .document
            .get("package")
            .and_then(|item| item.as_table())
            && let Some(version_item) = package_table.get("version")
        {
            match version_item.as_value() {
                Some(Value::String(version_str)) => {
                    return Version::parse(version_str.value()).map_err(|e| {
                        VersionError::ParseFailed {
                            version: version_str.value().to_string(),
                            source: e,
                        }
                        .into()
                    });
                }
                Some(Value::InlineTable(table)) => {
                    // Check for workspace inheritance
                    if table.contains_key("workspace") {
                        // Fall through to check workspace version
                    } else {
                        return Err(VersionError::TomlUpdateFailed {
                            path: self.file_path.clone(),
                            reason: "Package version has unexpected inline table format"
                                .to_string(),
                        }
                        .into());
                    }
                }
                _ => {}
            }
        }

        // Try workspace version
        if let Some(workspace_table) = self
            .document
            .get("workspace")
            .and_then(|item| item.as_table())
            && let Some(package_table) = workspace_table
                .get("package")
                .and_then(|item| item.as_table())
            && let Some(version_item) = package_table.get("version")
            && let Some(Value::String(version_str)) = version_item.as_value()
        {
            return Version::parse(version_str.value()).map_err(|e| {
                VersionError::ParseFailed {
                    version: version_str.value().to_string(),
                    source: e,
                }
                .into()
            });
        }

        Err(VersionError::TomlUpdateFailed {
            path: self.file_path.clone(),
            reason: "No version found in package or workspace.package sections".to_string(),
        }
        .into())
    }

    /// Check if file has workspace version inheritance
    pub fn uses_workspace_version(&self) -> bool {
        if let Some(package_table) = self
            .document
            .get("package")
            .and_then(|item| item.as_table())
            && let Some(version_item) = package_table.get("version")
        {
            // Handle dotted key syntax: version.workspace = true
            if let Some(table) = version_item.as_table()
                && table.contains_key("workspace")
            {
                return true;
            }
            // Handle inline table syntax: version = { workspace = true }
            if let Some(Value::InlineTable(table)) = version_item.as_value() {
                return table.contains_key("workspace");
            }
        }
        false
    }

    /// Save changes to file
    pub fn save(&self) -> Result<()> {
        std::fs::write(&self.file_path, self.document.to_string()).map_err(|e| {
            VersionError::TomlUpdateFailed {
                path: self.file_path.clone(),
                reason: format!("Failed to write file: {}", e),
            }
            .into()
        })
    }

    /// Create backup of current state
    pub fn create_backup(&self) -> TomlBackup {
        TomlBackup {
            file_path: self.file_path.clone(),
            content: self.original_content.clone(),
        }
    }

    /// Restore from backup
    pub fn restore_from_backup(backup: &TomlBackup) -> Result<()> {
        std::fs::write(&backup.file_path, &backup.content).map_err(|e| {
            VersionError::TomlUpdateFailed {
                path: backup.file_path.clone(),
                reason: format!("Failed to restore backup: {}", e),
            }
            .into()
        })
    }

    /// Get the document as string (preview changes)
    pub fn preview(&self) -> String {
        self.document.to_string()
    }

    /// Get file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Check if document has been modified
    pub fn is_modified(&self) -> bool {
        self.document.to_string() != self.original_content
    }

    /// Get list of all dependencies in all sections
    pub fn get_all_dependencies(&self) -> HashMap<String, DependencyInfo> {
        let mut dependencies = HashMap::new();

        // Helper to extract dependencies from a table
        let extract_deps = |table: &toml_edit::Table,
                            section: DependencySection|
         -> Vec<(String, DependencyInfo)> {
            let mut deps = Vec::new();
            for (name, item) in table.iter() {
                if let Some(dep_info) = self.parse_dependency_info(item, section) {
                    deps.push((name.to_string(), dep_info));
                }
            }
            deps
        };

        // Extract from [dependencies]
        if let Some(deps_table) = self
            .document
            .get("dependencies")
            .and_then(|item| item.as_table())
        {
            for (name, dep_info) in extract_deps(deps_table, DependencySection::Dependencies) {
                dependencies.insert(name, dep_info);
            }
        }

        // Extract from [dev-dependencies]
        if let Some(dev_deps_table) = self
            .document
            .get("dev-dependencies")
            .and_then(|item| item.as_table())
        {
            for (name, dep_info) in extract_deps(dev_deps_table, DependencySection::DevDependencies)
            {
                dependencies.insert(name, dep_info);
            }
        }

        // Extract from [build-dependencies]
        if let Some(build_deps_table) = self
            .document
            .get("build-dependencies")
            .and_then(|item| item.as_table())
        {
            for (name, dep_info) in
                extract_deps(build_deps_table, DependencySection::BuildDependencies)
            {
                dependencies.insert(name, dep_info);
            }
        }

        dependencies
    }

    /// Parse dependency information from TOML item
    fn parse_dependency_info(
        &self,
        item: &Item,
        section: DependencySection,
    ) -> Option<DependencyInfo> {
        match item {
            Item::Value(Value::String(version_str)) => Some(DependencyInfo {
                version: Some(version_str.value().to_string()),
                path: None,
                git: None,
                section,
            }),
            Item::Value(Value::InlineTable(table)) => {
                let version = table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let path = table
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let git = table
                    .get("git")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(DependencyInfo {
                    version,
                    path,
                    git,
                    section,
                })
            }
            Item::Table(table) => {
                let version = table
                    .get("version")
                    .and_then(|item| item.as_value())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let path = table
                    .get("path")
                    .and_then(|item| item.as_value())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let git = table
                    .get("git")
                    .and_then(|item| item.as_value())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(DependencyInfo {
                    version,
                    path,
                    git,
                    section,
                })
            }
            _ => None,
        }
    }
}

/// Dependency section type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencySection {
    /// Regular dependencies
    Dependencies,
    /// Development dependencies
    DevDependencies,
    /// Build dependencies
    BuildDependencies,
}

impl DependencySection {
    /// Get the TOML section name
    pub fn section_name(&self) -> &'static str {
        match self {
            DependencySection::Dependencies => "dependencies",
            DependencySection::DevDependencies => "dev-dependencies",
            DependencySection::BuildDependencies => "build-dependencies",
        }
    }
}

/// Information about a dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Version requirement
    pub version: Option<String>,
    /// Path to local dependency
    pub path: Option<String>,
    /// Git repository URL
    pub git: Option<String>,
    /// Which dependency section this belongs to
    pub section: DependencySection,
}

impl DependencyInfo {
    /// Check if this is a path dependency
    pub fn is_path_dependency(&self) -> bool {
        self.path.is_some()
    }

    /// Check if this is a git dependency
    pub fn is_git_dependency(&self) -> bool {
        self.git.is_some()
    }

    /// Check if this dependency has a version specification
    pub fn has_version(&self) -> bool {
        self.version.is_some()
    }
}
