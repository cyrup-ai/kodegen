//! Workspace validation for pre-release checks.
//!
//! This module performs comprehensive validation to ensure the workspace is ready
//! for release operations, preventing failures during the release process.

use crate::error::{Result, GitError, PublishError};
use crate::workspace::SharedWorkspaceInfo;
use gix::bstr::ByteSlice;
use kodegen_tools_git;
use serde::{Serialize, Deserialize};
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;

/// Comprehensive workspace validator
#[derive(Debug)]
pub struct WorkspaceValidator {
    workspace: SharedWorkspaceInfo,
    repository: gix::Repository,
}

/// Validation result with detailed pass/fail information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation success
    pub success: bool,
    /// Individual validation checks and their results
    pub checks: Vec<ValidationCheck>,
    /// Critical errors that must be fixed before release
    pub critical_errors: Vec<String>,
    /// Warnings that should be addressed but don't block release
    pub warnings: Vec<String>,
}

/// Individual validation check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Name of the validation check
    pub name: String,
    /// Whether this check passed
    pub passed: bool,
    /// Detailed message about the check result
    pub message: String,
    /// Whether this is a critical check (failure blocks release)
    pub critical: bool,
    /// Duration of the check in milliseconds
    pub duration_ms: u64,
}

impl WorkspaceValidator {
    /// Create a new workspace validator
    pub fn new(workspace: SharedWorkspaceInfo) -> Result<Self> {
        let repository = gix::discover(&workspace.root)
            .map_err(|_| GitError::NotRepository)?;

        Ok(Self {
            workspace,
            repository,
        })
    }

    /// Perform comprehensive workspace validation
    pub async fn validate(&self) -> Result<ValidationResult> {
        let mut checks = Vec::new();
        let mut critical_errors = Vec::new();
        let mut warnings = Vec::new();

        // Git repository validation
        self.validate_git_state(&mut checks, &mut critical_errors).await?;

        // Version consistency validation
        self.validate_version_consistency(&mut checks, &mut critical_errors, &mut warnings).await?;

        // Build validation
        self.validate_builds(&mut checks, &mut critical_errors, &mut warnings).await?;

        // Credentials validation
        self.validate_credentials(&mut checks, &mut warnings).await?;

        // Dependency validation
        self.validate_dependencies(&mut checks, &mut critical_errors, &mut warnings).await?;

        // Crates.io validation
        self.validate_crates_io_readiness(&mut checks, &mut warnings).await?;

        let success = critical_errors.is_empty();

        Ok(ValidationResult {
            success,
            checks,
            critical_errors,
            warnings,
        })
    }

    /// Validate git repository state
    async fn validate_git_state(
        &self,
        checks: &mut Vec<ValidationCheck>,
        critical_errors: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Check working directory status
        let status_result = self.check_working_directory_clean().await;
        let duration = start_time.elapsed().as_millis() as u64;

        match status_result {
            Ok(true) => {
                checks.push(ValidationCheck {
                    name: "Git Working Directory".to_string(),
                    passed: true,
                    message: "Working directory is clean".to_string(),
                    critical: true,
                    duration_ms: duration,
                });
            }
            Ok(false) => {
                let warning_msg = "Working directory has uncommitted changes (will be included in release commit)";
                checks.push(ValidationCheck {
                    name: "Git Working Directory".to_string(),
                    passed: true,
                    message: warning_msg.to_string(),
                    critical: false,
                    duration_ms: duration,
                });
            }
            Err(e) => {
                let error_msg = format!("Failed to check git status: {}", e);
                checks.push(ValidationCheck {
                    name: "Git Working Directory".to_string(),
                    passed: false,
                    message: error_msg.clone(),
                    critical: true,
                    duration_ms: duration,
                });
                critical_errors.push(error_msg);
            }
        }

        // Check if we're on a valid branch
        let branch_check_start = std::time::Instant::now();
        let branch_result = self.check_valid_branch().await;
        let branch_duration = branch_check_start.elapsed().as_millis() as u64;

        match branch_result {
            Ok(branch_name) => {
                checks.push(ValidationCheck {
                    name: "Git Branch".to_string(),
                    passed: true,
                    message: format!("On branch: {}", branch_name),
                    critical: false,
                    duration_ms: branch_duration,
                });
            }
            Err(e) => {
                let error_msg = format!("Invalid git branch state: {}", e);
                checks.push(ValidationCheck {
                    name: "Git Branch".to_string(),
                    passed: false,
                    message: error_msg.clone(),
                    critical: true,
                    duration_ms: branch_duration,
                });
                critical_errors.push(error_msg);
            }
        }

        Ok(())
    }

    /// Check if working directory is clean
    async fn check_working_directory_clean(&self) -> Result<bool> {
        // Use kodegen_tools_git::is_clean which properly handles the gix status API
        let repo_handle = kodegen_tools_git::RepoHandle::new(self.repository.clone());
        kodegen_tools_git::is_clean(&repo_handle)
            .await
            .map_err(|_| GitError::RemoteOperationFailed {
                operation: "status check".to_string(),
                reason: "Channel error".to_string(),
            }.into())
    }

    /// Check if we're on a valid branch
    async fn check_valid_branch(&self) -> Result<String> {
        let head = self.repository.head()
            .map_err(|e| GitError::BranchOperationFailed {
                reason: e.to_string(),
            })?;

        let branch_name = head.referent_name()
            .and_then(|name| name.shorten().to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "detached HEAD".to_string());

        Ok(branch_name)
    }

    /// Validate version consistency across packages
    async fn validate_version_consistency(
        &self,
        checks: &mut Vec<ValidationCheck>,
        critical_errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        let workspace_version = match self.workspace.workspace_version() {
            Ok(version) => version,
            Err(e) => {
                let error_msg = format!("Failed to get workspace version: {}", e);
                checks.push(ValidationCheck {
                    name: "Version Consistency".to_string(),
                    passed: false,
                    message: error_msg.clone(),
                    critical: true,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
                critical_errors.push(error_msg);
                return Ok(());
            }
        };

        let mut version_mismatches = Vec::new();
        let mut dependency_version_issues = Vec::new();

        // Check each package's version
        for (package_name, package_info) in &self.workspace.packages {
            // Check if package version matches workspace version (if using workspace inheritance)
            if package_info.version != workspace_version {
                let mismatch = format!(
                    "Package '{}' version '{}' doesn't match workspace version '{}'",
                    package_name, package_info.version, workspace_version
                );
                version_mismatches.push(mismatch);
            }

            // Check internal dependency versions
            for dep_name in &package_info.workspace_dependencies {
                if let Some(dep_package) = self.workspace.packages.get(dep_name)
                    && let Some(dep_spec) = package_info.all_dependencies.get(dep_name)
                    && let Some(dep_version) = &dep_spec.version
                    && dep_version != &dep_package.version {
                        let issue = format!(
                            "Package '{}' depends on '{}' version '{}' but '{}' is at version '{}'",
                            package_name, dep_name, dep_version, dep_name, dep_package.version
                        );
                        dependency_version_issues.push(issue);
                    }
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        if version_mismatches.is_empty() && dependency_version_issues.is_empty() {
            checks.push(ValidationCheck {
                name: "Version Consistency".to_string(),
                passed: true,
                message: format!("All packages consistent with workspace version {}", workspace_version),
                critical: true,
                duration_ms: duration,
            });
        } else {
            let mut message_parts = Vec::new();
            let has_version_mismatches = !version_mismatches.is_empty();
            
            if has_version_mismatches {
                message_parts.extend(version_mismatches.iter().cloned());
                critical_errors.extend(version_mismatches);
            }
            
            if !dependency_version_issues.is_empty() {
                message_parts.extend(dependency_version_issues.iter().cloned());
                warnings.extend(dependency_version_issues);
            }

            checks.push(ValidationCheck {
                name: "Version Consistency".to_string(),
                passed: false,
                message: message_parts.join("; "),
                critical: has_version_mismatches,
                duration_ms: duration,
            });
        }

        Ok(())
    }

    /// Validate that all packages can be built successfully
    async fn validate_builds(
        &self,
        checks: &mut Vec<ValidationCheck>,
        critical_errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        let mut build_failures = Vec::new();
        let mut build_warnings = Vec::new();

        // Run cargo check once on the entire workspace (much faster than per-package)
        match self.test_workspace_build().await {
            Ok(BuildResult::Success) => {
                // Workspace builds successfully
            }
            Ok(BuildResult::Warning(warning)) => {
                build_warnings.push(warning);
            }
            Err(e) => {
                build_failures.push(format!("Workspace build failed: {}", e));
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        if build_failures.is_empty() {
            let message = if build_warnings.is_empty() {
                "All packages build successfully".to_string()
            } else {
                format!("All packages build successfully ({} warnings)", build_warnings.len())
            };

            checks.push(ValidationCheck {
                name: "Package Builds".to_string(),
                passed: true,
                message,
                critical: true,
                duration_ms: duration,
            });

            warnings.extend(build_warnings);
        } else {
            checks.push(ValidationCheck {
                name: "Package Builds".to_string(),
                passed: false,
                message: format!("{} packages failed to build", build_failures.len()),
                critical: true,
                duration_ms: duration,
            });

            critical_errors.extend(build_failures);
            warnings.extend(build_warnings);
        }

        Ok(())
    }

    /// Test building the entire workspace (faster than per-package)
    async fn test_workspace_build(&self) -> Result<BuildResult> {
        eprintln!("🔍 Validating workspace build:");
        eprintln!("   Working directory: {}", self.workspace.root.display());
        eprintln!("   Command: cargo check --workspace --all-targets");
        
        let mut cmd = AsyncCommand::new("cargo");
        cmd.arg("check")
            .arg("--workspace")
            .arg("--all-targets")
            .current_dir(&self.workspace.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| PublishError::PublishFailed {
                package: "workspace".to_string(),
                reason: format!("Failed to execute cargo check: {}", e),
            })?;

        if output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("warning:") {
                // Count warnings
                let warning_count = stderr.matches("warning:").count();
                Ok(BuildResult::Warning(format!("{} warning(s) found", warning_count)))
            } else {
                Ok(BuildResult::Success)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(PublishError::DryRunFailed {
                package: "workspace".to_string(),
                reason: stderr.to_string(),
            }.into())
        }
    }

    /// Validate crates.io credentials and authentication
    async fn validate_credentials(
        &self,
        checks: &mut Vec<ValidationCheck>,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        match self.check_cargo_login().await {
            Ok(true) => {
                checks.push(ValidationCheck {
                    name: "Crates.io Authentication".to_string(),
                    passed: true,
                    message: "Successfully authenticated with crates.io".to_string(),
                    critical: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
            }
            Ok(false) => {
                let warning_msg = "Not authenticated with crates.io - run 'cargo login' first";
                checks.push(ValidationCheck {
                    name: "Crates.io Authentication".to_string(),
                    passed: false,
                    message: warning_msg.to_string(),
                    critical: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
                warnings.push(warning_msg.to_string());
            }
            Err(e) => {
                let warning_msg = format!("Could not verify crates.io authentication: {}", e);
                checks.push(ValidationCheck {
                    name: "Crates.io Authentication".to_string(),
                    passed: false,
                    message: warning_msg.clone(),
                    critical: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
                warnings.push(warning_msg);
            }
        }

        Ok(())
    }

    /// Check if user is logged into cargo/crates.io
    async fn check_cargo_login(&self) -> Result<bool> {
        let mut cmd = AsyncCommand::new("cargo");
        cmd.arg("login")
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // Try to get current login status by attempting a dry run operation
        let mut whoami_cmd = AsyncCommand::new("cargo");
        whoami_cmd.arg("owner")
            .arg("--list")
            .arg("nonexistent-crate-name-12345")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = whoami_cmd.output().await
            .map_err(|_| PublishError::AuthenticationError)?;

        // If we get an authentication error, we're not logged in
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(!stderr.contains("authentication") && !stderr.contains("token"))
    }

    /// Validate workspace dependencies
    async fn validate_dependencies(
        &self,
        checks: &mut Vec<ValidationCheck>,
        critical_errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        let mut missing_dependencies = Vec::new();
        let mut version_conflicts = Vec::new();

        // Check that all workspace dependencies exist and are properly versioned
        for (package_name, package_info) in &self.workspace.packages {
            for dep_name in &package_info.workspace_dependencies {
                if !self.workspace.packages.contains_key(dep_name) {
                    missing_dependencies.push(format!(
                        "Package '{}' depends on missing workspace package '{}'",
                        package_name, dep_name
                    ));
                }

                // Check for version specification in published dependencies
                if let Some(dep_spec) = package_info.all_dependencies.get(dep_name)
                    && dep_spec.version.is_none() && dep_spec.path.is_some() {
                        version_conflicts.push(format!(
                            "Package '{}' dependency on '{}' lacks version (needed for crates.io)",
                            package_name, dep_name
                        ));
                    }
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        if missing_dependencies.is_empty() && version_conflicts.is_empty() {
            checks.push(ValidationCheck {
                name: "Workspace Dependencies".to_string(),
                passed: true,
                message: "All workspace dependencies are properly configured".to_string(),
                critical: true,
                duration_ms: duration,
            });
        } else {
            let mut all_issues = Vec::new();
            all_issues.extend(missing_dependencies.iter().cloned());
            all_issues.extend(version_conflicts.iter().cloned());

            checks.push(ValidationCheck {
                name: "Workspace Dependencies".to_string(),
                passed: false,
                message: format!("{} dependency issues found", all_issues.len()),
                critical: !missing_dependencies.is_empty(),
                duration_ms: duration,
            });

            if !missing_dependencies.is_empty() {
                critical_errors.extend(missing_dependencies);
            }
            if !version_conflicts.is_empty() {
                warnings.extend(version_conflicts);
            }
        }

        Ok(())
    }

    /// Validate crates.io readiness
    async fn validate_crates_io_readiness(
        &self,
        checks: &mut Vec<ValidationCheck>,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        let mut readiness_issues = Vec::new();

        // Check that packages have required metadata for crates.io
        for (package_name, package_info) in &self.workspace.packages {
            if package_info.config.description.is_none() {
                readiness_issues.push(format!(
                    "Package '{}' missing description (recommended for crates.io)",
                    package_name
                ));
            }

            if package_info.config.license.is_none() {
                readiness_issues.push(format!(
                    "Package '{}' missing license (required for crates.io)",
                    package_name
                ));
            }

            if package_info.config.homepage.is_none() && package_info.config.repository.is_none() {
                readiness_issues.push(format!(
                    "Package '{}' missing homepage or repository (recommended for crates.io)",
                    package_name
                ));
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        if readiness_issues.is_empty() {
            checks.push(ValidationCheck {
                name: "Crates.io Readiness".to_string(),
                passed: true,
                message: "All packages ready for crates.io publishing".to_string(),
                critical: false,
                duration_ms: duration,
            });
        } else {
            checks.push(ValidationCheck {
                name: "Crates.io Readiness".to_string(),
                passed: false,
                message: format!("{} metadata issues found", readiness_issues.len()),
                critical: false,
                duration_ms: duration,
            });

            warnings.extend(readiness_issues);
        }

        Ok(())
    }
}

/// Result of a package build test
#[derive(Debug)]
enum BuildResult {
    Success,
    Warning(String),
}

impl ValidationResult {
    /// Get all failed checks
    pub fn failed_checks(&self) -> Vec<&ValidationCheck> {
        self.checks.iter().filter(|check| !check.passed).collect()
    }

    /// Get all critical failed checks
    pub fn critical_failed_checks(&self) -> Vec<&ValidationCheck> {
        self.checks
            .iter()
            .filter(|check| !check.passed && check.critical)
            .collect()
    }

    /// Get total validation duration
    pub fn total_duration_ms(&self) -> u64 {
        self.checks.iter().map(|check| check.duration_ms).sum()
    }

    /// Check if validation passed with only warnings
    pub fn passed_with_warnings(&self) -> bool {
        self.success && !self.warnings.is_empty()
    }

    /// Get summary of validation results
    pub fn summary(&self) -> String {
        let total_checks = self.checks.len();
        let passed_checks = self.checks.iter().filter(|c| c.passed).count();
        let critical_failures = self.critical_failed_checks().len();

        if self.success {
            if self.warnings.is_empty() {
                format!("✅ All {} checks passed", total_checks)
            } else {
                format!(
                    "✅ {}/{} checks passed ({} warnings)",
                    passed_checks, total_checks, self.warnings.len()
                )
            }
        } else {
            format!(
                "❌ {}/{} checks passed ({} critical failures)",
                passed_checks, total_checks, critical_failures
            )
        }
    }
}

impl ValidationCheck {
    /// Check if this validation check is considered a failure
    pub fn is_failure(&self) -> bool {
        !self.passed
    }

    /// Check if this is a critical failure
    pub fn is_critical_failure(&self) -> bool {
        !self.passed && self.critical
    }

    /// Format the check result for display
    pub fn format_result(&self) -> String {
        let status = if self.passed { "✅" } else { "❌" };
        let criticality = if self.critical { " [CRITICAL]" } else { "" };
        
        format!(
            "{} {} ({}ms){}: {}",
            status, self.name, self.duration_ms, criticality, self.message
        )
    }
}