//! Workspace structure analysis and package enumeration.

use crate::error::{Result, WorkspaceError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// External crate for glob pattern expansion
use glob;

/// Shared reference to workspace information using Arc for efficient cloning.
/// WorkspaceInfo is immutable after analysis, so Arc without RwLock is sufficient.
pub type SharedWorkspaceInfo = Arc<WorkspaceInfo>;

/// Complete workspace information
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Root directory of the workspace
    pub root: PathBuf,
    /// Workspace-level configuration
    pub workspace_config: WorkspaceConfig,
    /// All packages in the workspace
    pub packages: HashMap<String, PackageInfo>,
    /// Internal dependencies between workspace packages
    pub internal_dependencies: HashMap<String, Vec<String>>,
}

/// Workspace-level configuration from root Cargo.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace members (supports glob patterns like "packages/*")
    pub members: Vec<String>,
    /// Workspace exclusions (supports glob patterns)
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Workspace package configuration
    pub package: Option<WorkspacePackage>,
    /// Workspace dependencies
    pub dependencies: Option<HashMap<String, toml::Value>>,
}

/// Workspace package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePackage {
    /// Workspace version
    pub version: Option<String>,
    /// Workspace edition
    pub edition: Option<String>,
    /// Other workspace package fields
    #[serde(flatten)]
    pub other: HashMap<String, toml::Value>,
}

/// Information about a single package in the workspace
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version (current)
    pub version: String,
    /// Path to package directory relative to workspace root
    pub path: PathBuf,
    /// Absolute path to package directory
    pub absolute_path: PathBuf,
    /// Path to Cargo.toml file
    pub cargo_toml_path: PathBuf,
    /// Package configuration
    pub config: PackageConfig,
    /// Dependencies on other workspace packages
    pub workspace_dependencies: Vec<String>,
    /// All dependencies (including external)
    pub all_dependencies: HashMap<String, DependencySpec>,
}

/// Package configuration from Cargo.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Package name
    pub name: String,
    /// Package version
    pub version: toml::Value,
    /// Package edition
    pub edition: Option<toml::Value>,
    /// Package description
    pub description: Option<String>,
    /// Package license
    pub license: Option<toml::Value>,
    /// Package authors
    pub authors: Option<toml::Value>,
    /// Package homepage
    pub homepage: Option<toml::Value>,
    /// Package repository
    pub repository: Option<toml::Value>,
    /// Other package fields
    #[serde(flatten)]
    pub other: HashMap<String, toml::Value>,
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    /// Dependency version requirement
    pub version: Option<String>,
    /// Local path for path dependencies
    pub path: Option<String>,
    /// Git repository URL
    pub git: Option<String>,
    /// Git branch/tag/rev
    pub rev: Option<String>,
    /// Dependency features
    pub features: Option<Vec<String>>,
    /// Optional dependency
    pub optional: Option<bool>,
    /// Default features
    pub default_features: Option<bool>,
}

impl WorkspaceInfo {
    /// Analyze a workspace starting from the given directory
    pub fn analyze<P: AsRef<Path>>(start_dir: P) -> Result<Self> {
        let workspace_root = Self::find_workspace_root(start_dir)?;

        // Parse root Cargo.toml ONCE - this eliminates all redundant reads
        let root_cargo_toml_path = workspace_root.join("Cargo.toml");
        let root_cargo_content = std::fs::read_to_string(&root_cargo_toml_path)?;
        let root_cargo_parsed: toml::Value = toml::from_str(&root_cargo_content)?;

        // Pass parsed value to both functions - zero additional I/O
        let workspace_config = Self::parse_workspace_config(&root_cargo_parsed)?;
        let packages =
            Self::enumerate_packages(&workspace_root, &workspace_config, &root_cargo_parsed)?;
        let internal_dependencies = Self::build_internal_dependency_map(&packages)?;

        Ok(Self {
            root: workspace_root,
            workspace_config,
            packages,
            internal_dependencies,
        })
    }

    /// Find the workspace root directory
    fn find_workspace_root<P: AsRef<Path>>(start_dir: P) -> Result<PathBuf> {
        // Try canonicalization, fall back to absolute path for network mounts
        let mut current_dir = start_dir.as_ref().canonicalize().or_else(|_| {
            let path = start_dir.as_ref();
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                std::env::current_dir().map(|cwd| cwd.join(path))
            }
        })?;

        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this Cargo.toml defines a workspace
                let content = std::fs::read_to_string(&cargo_toml)?;
                let parsed: toml::Value = toml::from_str(&content)?;

                if parsed.get("workspace").is_some() {
                    return Ok(current_dir);
                }
            }

            match current_dir.parent() {
                Some(parent) => current_dir = parent.to_path_buf(),
                None => return Err(WorkspaceError::RootNotFound.into()),
            }
        }
    }

    /// Parse workspace configuration from root Cargo.toml
    fn parse_workspace_config(root_cargo_parsed: &toml::Value) -> Result<WorkspaceConfig> {
        let workspace_table =
            root_cargo_parsed
                .get("workspace")
                .ok_or_else(|| WorkspaceError::InvalidStructure {
                    reason: "No [workspace] section found in root Cargo.toml".to_string(),
                })?;

        let workspace_config: WorkspaceConfig =
            workspace_table
                .clone()
                .try_into()
                .map_err(|e| WorkspaceError::InvalidStructure {
                    reason: format!("Failed to parse workspace configuration: {}", e),
                })?;

        Ok(workspace_config)
    }

    /// Enumerate all packages in the workspace
    fn enumerate_packages(
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        root_cargo_parsed: &toml::Value,
    ) -> Result<HashMap<String, PackageInfo>> {
        let mut packages = HashMap::new();

        // Step 1: Expand all member patterns to concrete paths
        let mut member_paths = Vec::new();

        for member_pattern in &workspace_config.members {
            let literal_path = workspace_root.join(member_pattern);

            // Check if pattern contains glob metacharacters
            if member_pattern.contains('*')
                || member_pattern.contains('?')
                || member_pattern.contains('[')
            {
                // Glob pattern - expand it
                let pattern_path = workspace_root.join(member_pattern);
                let pattern_str =
                    pattern_path
                        .to_str()
                        .ok_or_else(|| WorkspaceError::InvalidStructure {
                            reason: format!("Invalid UTF-8 in path pattern: {}", member_pattern),
                        })?;

                let entries =
                    glob::glob(pattern_str).map_err(|e| WorkspaceError::InvalidStructure {
                        reason: format!("Invalid glob pattern '{}': {}", member_pattern, e),
                    })?;

                for entry in entries {
                    let path = entry.map_err(|e| WorkspaceError::InvalidStructure {
                        reason: format!("Error reading glob entry: {}", e),
                    })?;

                    if path.is_dir() {
                        member_paths.push(path);
                    }
                }
            } else if literal_path.exists() && literal_path.is_dir() {
                // Literal path exists - use directly
                member_paths.push(literal_path);
            } else {
                // Path doesn't exist and isn't a glob - error
                return Err(WorkspaceError::InvalidStructure {
                    reason: format!(
                        "Workspace member '{}' not found and is not a valid glob pattern",
                        member_pattern
                    ),
                }
                .into());
            }
        }

        // Step 2: Expand exclusion patterns
        let mut excluded_paths = std::collections::HashSet::new();

        for exclude_pattern in &workspace_config.exclude {
            let literal_path = workspace_root.join(exclude_pattern);

            if exclude_pattern.contains('*')
                || exclude_pattern.contains('?')
                || exclude_pattern.contains('[')
            {
                // Glob pattern
                let pattern_path = workspace_root.join(exclude_pattern);
                if let Some(pattern_str) = pattern_path.to_str()
                    && let Ok(entries) = glob::glob(pattern_str)
                {
                    for entry in entries.flatten() {
                        excluded_paths.insert(entry);
                    }
                }
            } else if literal_path.exists() {
                // Literal path
                excluded_paths.insert(literal_path);
            }
        }

        // Step 3: Filter out excluded paths
        member_paths.retain(|path| !excluded_paths.contains(path));

        // Step 4: Process each member path
        for member_path in member_paths {
            let cargo_toml_path = member_path.join("Cargo.toml");

            // Skip directories without Cargo.toml (glob may match non-packages)
            if !cargo_toml_path.exists() {
                continue;
            }

            let package_info = Self::parse_package_info(
                workspace_root,
                &member_path,
                &cargo_toml_path,
                root_cargo_parsed,
            )?;

            packages.insert(package_info.name.clone(), package_info);
        }

        // Step 5: Validate we found at least one package
        if packages.is_empty() {
            return Err(WorkspaceError::InvalidStructure {
                reason: "No packages found in workspace after expanding member patterns"
                    .to_string(),
            }
            .into());
        }

        Ok(packages)
    }

    /// Parse information for a single package
    fn parse_package_info(
        workspace_root: &Path,
        package_path: &Path,
        cargo_toml_path: &Path,
        root_cargo_parsed: &toml::Value,
    ) -> Result<PackageInfo> {
        let content = std::fs::read_to_string(cargo_toml_path)?;
        let parsed: toml::Value = toml::from_str(&content)?;

        let package_table =
            parsed
                .get("package")
                .ok_or_else(|| WorkspaceError::InvalidPackage {
                    package: package_path.display().to_string(),
                    reason: "No [package] section found".to_string(),
                })?;

        let config: PackageConfig =
            package_table
                .clone()
                .try_into()
                .map_err(|e| WorkspaceError::InvalidPackage {
                    package: package_path.display().to_string(),
                    reason: format!("Failed to parse package configuration: {}", e),
                })?;

        // Resolve version (might be workspace inherited)
        let version = match &config.version {
            toml::Value::String(v) => v.clone(),
            toml::Value::Table(table)
                if table.get("workspace") == Some(&toml::Value::Boolean(true)) =>
            {
                // Use cached parsed workspace config - no file I/O needed
                root_cargo_parsed
                    .get("workspace")
                    .and_then(|w| w.get("package"))
                    .and_then(|p| p.get("version"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| WorkspaceError::InvalidPackage {
                        package: config.name.clone(),
                        reason: "Workspace version not found".to_string(),
                    })?
                    .to_string()
            }
            _ => {
                return Err(WorkspaceError::InvalidPackage {
                    package: config.name.clone(),
                    reason: "Invalid version specification".to_string(),
                }
                .into());
            }
        };

        // Parse dependencies
        let all_dependencies = Self::parse_dependencies(&parsed)?;
        let workspace_dependencies = Self::extract_workspace_dependencies(&all_dependencies);

        let relative_path = package_path
            .strip_prefix(workspace_root)
            .map_err(|_| WorkspaceError::InvalidPackage {
                package: config.name.clone(),
                reason: "Package path not within workspace".to_string(),
            })?
            .to_path_buf();

        Ok(PackageInfo {
            name: config.name.clone(),
            version,
            path: relative_path,
            absolute_path: package_path.to_path_buf(),
            cargo_toml_path: cargo_toml_path.to_path_buf(),
            config,
            workspace_dependencies,
            all_dependencies,
        })
    }

    /// Parse dependencies from package TOML
    fn parse_dependencies(parsed: &toml::Value) -> Result<HashMap<String, DependencySpec>> {
        let mut dependencies = HashMap::new();

        // Parse regular dependencies
        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
            for (name, spec) in deps {
                dependencies.insert(name.clone(), Self::parse_dependency_spec(spec)?);
            }
        }

        // Parse dev-dependencies
        if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
            for (name, spec) in dev_deps {
                dependencies.insert(format!("dev:{}", name), Self::parse_dependency_spec(spec)?);
            }
        }

        // Parse build-dependencies
        if let Some(build_deps) = parsed.get("build-dependencies").and_then(|d| d.as_table()) {
            for (name, spec) in build_deps {
                dependencies.insert(
                    format!("build:{}", name),
                    Self::parse_dependency_spec(spec)?,
                );
            }
        }

        Ok(dependencies)
    }

    /// Parse a single dependency specification
    fn parse_dependency_spec(spec: &toml::Value) -> Result<DependencySpec> {
        match spec {
            toml::Value::String(version) => Ok(DependencySpec {
                version: Some(version.clone()),
                path: None,
                git: None,
                rev: None,
                features: None,
                optional: None,
                default_features: None,
            }),
            toml::Value::Table(table) => {
                let spec: DependencySpec =
                    table
                        .clone()
                        .try_into()
                        .map_err(|e| WorkspaceError::InvalidStructure {
                            reason: format!("Failed to parse dependency spec: {}", e),
                        })?;
                Ok(spec)
            }
            _ => Err(WorkspaceError::InvalidStructure {
                reason: "Invalid dependency specification".to_string(),
            }
            .into()),
        }
    }

    /// Extract workspace dependencies from all dependencies
    fn extract_workspace_dependencies(
        all_dependencies: &HashMap<String, DependencySpec>,
    ) -> Vec<String> {
        all_dependencies
            .iter()
            .filter_map(|(name, spec)| {
                // Check if this is a path dependency pointing to another workspace member
                if spec.path.is_some() && !name.contains(':') {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Build internal dependency mapping
    fn build_internal_dependency_map(
        packages: &HashMap<String, PackageInfo>,
    ) -> Result<HashMap<String, Vec<String>>> {
        let package_names: std::collections::HashSet<_> = packages.keys().cloned().collect();
        let mut internal_deps = HashMap::new();

        for (package_name, package_info) in packages {
            let mut deps = Vec::new();
            for dep_name in &package_info.workspace_dependencies {
                if package_names.contains(dep_name) {
                    deps.push(dep_name.clone());
                }
            }
            internal_deps.insert(package_name.clone(), deps);
        }

        Ok(internal_deps)
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Result<&PackageInfo> {
        self.packages.get(name).ok_or_else(|| {
            WorkspaceError::PackageNotFound {
                name: name.to_string(),
            }
            .into()
        })
    }

    /// Get workspace version
    pub fn workspace_version(&self) -> Result<String> {
        self.workspace_config
            .package
            .as_ref()
            .and_then(|p| p.version.as_ref())
            .ok_or_else(|| {
                WorkspaceError::InvalidStructure {
                    reason: "No workspace version found".to_string(),
                }
                .into()
            })
            .cloned()
    }

    /// Get all package names
    pub fn package_names(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }

    /// Check if a package exists in the workspace
    pub fn has_package(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }
}
