//! Comprehensive error types for cyrup_release operations.
//!
//! This module defines all error types with actionable error messages and recovery suggestions.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for cyrup_release operations
pub type Result<T> = std::result::Result<T, ReleaseError>;

/// Main error type for all cyrup_release operations
#[derive(Error, Debug)]
pub enum ReleaseError {
    /// Workspace analysis errors
    #[error("Workspace error: {0}")]
    Workspace(#[from] WorkspaceError),

    /// Version management errors
    #[error("Version error: {0}")]
    Version(#[from] VersionError),

    /// Git operation errors
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    /// Publishing errors
    #[error("Publish error: {0}")]
    Publish(#[from] PublishError),

    /// State management errors
    #[error("State error: {0}")]
    State(#[from] StateError),

    /// CLI argument errors
    #[error("CLI error: {0}")]
    Cli(#[from] CliError),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML parsing errors
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// TOML editing errors
    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    /// GitHub operation errors
    #[error("GitHub error: {0}")]
    GitHub(String),

    /// Bundler errors
    #[error("Bundler error: {0}")]
    Bundler(#[from] crate::bundler::Error),
}

/// Workspace-specific errors
#[derive(Error, Debug)]
pub enum WorkspaceError {
    /// Workspace root not found
    #[error("Could not find workspace root. Please run from within a Cargo workspace.")]
    RootNotFound,

    /// Invalid workspace structure
    #[error("Invalid workspace structure: {reason}")]
    InvalidStructure {
        /// Reason for the error
        reason: String,
    },

    /// Package not found in workspace
    #[error("Package '{name}' not found in workspace")]
    PackageNotFound {
        /// Package name
        name: String,
    },

    /// Circular dependency detected
    #[error("Circular dependency detected in packages: {packages:?}")]
    CircularDependency {
        /// Package names involved in the cycle
        packages: Vec<String>,
    },

    /// Missing Cargo.toml file
    #[error("Missing Cargo.toml file at {path}")]
    MissingCargoToml {
        /// Path where Cargo.toml was expected
        path: PathBuf,
    },

    /// Invalid package configuration
    #[error("Invalid package configuration for '{package}': {reason}")]
    InvalidPackage {
        /// Package name
        package: String,
        /// Reason for the error
        reason: String,
    },
}

/// Version management errors
#[derive(Error, Debug)]
pub enum VersionError {
    /// Invalid version format
    #[error("Invalid version '{version}': {reason}")]
    InvalidVersion {
        /// Version string
        version: String,
        /// Reason for the error
        reason: String,
    },

    /// Version parsing failed
    #[error("Failed to parse version '{version}': {source}")]
    ParseFailed {
        /// Version string
        version: String,
        /// Parsing error
        #[source]
        source: semver::Error,
    },

    /// Internal dependency version mismatch
    #[error(
        "Internal dependency version mismatch for '{dependency}': expected {expected}, found {found}"
    )]
    DependencyMismatch {
        /// Dependency name
        dependency: String,
        /// Expected version
        expected: String,
        /// Found version
        found: String,
    },

    /// Failed to update Cargo.toml
    #[error("Failed to update Cargo.toml at {path}: {reason}")]
    TomlUpdateFailed {
        /// Path to Cargo.toml
        path: PathBuf,
        /// Reason for the error
        reason: String,
    },

    /// Version bump not supported
    #[error("Version bump '{bump}' not supported for version '{version}'")]
    UnsupportedBump {
        /// Bump type
        bump: String,
        /// Current version
        version: String,
    },
}

/// Git operation errors
#[derive(Error, Debug)]
pub enum GitError {
    /// Not a git repository
    #[error("Not a git repository. Please initialize git first.")]
    NotRepository,

    /// Working directory not clean
    #[error("Working directory not clean. Please commit or stash changes before releasing.")]
    DirtyWorkingDirectory,

    /// Git authentication failed
    #[error("Git authentication failed: {reason}")]
    AuthenticationFailed {
        /// Reason for the error
        reason: String,
    },

    /// Remote operation failed
    #[error("Git remote operation failed: {operation} - {reason}")]
    RemoteOperationFailed {
        /// Operation that failed
        operation: String,
        /// Reason for the error
        reason: String,
    },

    /// Tag already exists
    #[error(
        "Git tag '{tag}' already exists. Use --force to overwrite or choose a different version."
    )]
    TagExists {
        /// Tag name
        tag: String,
    },

    /// Branch operation failed
    #[error("Git branch operation failed: {reason}")]
    BranchOperationFailed {
        /// Reason for the error
        reason: String,
    },

    /// Commit failed
    #[error("Git commit failed: {reason}")]
    CommitFailed {
        /// Reason for the error
        reason: String,
    },

    /// Push failed
    #[error("Git push failed: {reason}")]
    PushFailed {
        /// Reason for the error
        reason: String,
    },
}

/// Publishing errors
#[derive(Error, Debug)]
pub enum PublishError {
    /// Package already published
    #[error("Package '{package}' version '{version}' already published to crates.io")]
    AlreadyPublished {
        /// Package name
        package: String,
        /// Version string
        version: String,
    },

    /// Publish command failed
    #[error("Cargo publish failed for '{package}': {reason}")]
    PublishFailed {
        /// Package name
        package: String,
        /// Reason for the error
        reason: String,
    },

    /// Dry run validation failed
    #[error("Dry run validation failed for '{package}': {reason}")]
    DryRunFailed {
        /// Package name
        package: String,
        /// Reason for the error
        reason: String,
    },

    /// Rate limit exceeded
    #[error(
        "Rate limit exceeded for crates.io. Please wait {retry_after_seconds} seconds before retrying."
    )]
    RateLimitExceeded {
        /// Seconds to wait
        retry_after_seconds: u64,
    },

    /// Network error during publishing
    #[error("Network error during publishing: {reason}")]
    NetworkError {
        /// Reason for the error
        reason: String,
    },

    /// Authentication error for crates.io
    #[error("Authentication error: Please ensure you're logged in with 'cargo login'")]
    AuthenticationError,

    /// Yank operation failed
    #[error("Failed to yank package '{package}' version '{version}': {reason}")]
    YankFailed {
        /// Package name
        package: String,
        /// Version string
        version: String,
        /// Reason for the error
        reason: String,
    },
}

/// State management errors
#[derive(Error, Debug)]
pub enum StateError {
    /// State file corrupted
    #[error("State file corrupted: {reason}")]
    Corrupted {
        /// Reason for the error
        reason: String,
    },

    /// State file not found
    #[error("State file not found. No release in progress.")]
    NotFound,

    /// State version mismatch
    #[error("State file version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// Expected version
        expected: String,
        /// Found version
        found: String,
    },

    /// Failed to save state
    #[error("Failed to save state: {reason}")]
    SaveFailed {
        /// Reason for the error
        reason: String,
    },

    /// Failed to load state
    #[error("Failed to load state: {reason}")]
    LoadFailed {
        /// Reason for the error
        reason: String,
    },
}

/// CLI-specific errors
#[derive(Error, Debug)]
pub enum CliError {
    /// Invalid command line arguments
    #[error("Invalid arguments: {reason}")]
    InvalidArguments {
        /// Reason for the error
        reason: String,
    },

    /// Missing required argument
    #[error("Missing required argument: {argument}")]
    MissingArgument {
        /// Argument name
        argument: String,
    },

    /// Conflicting arguments
    #[error("Conflicting arguments: {arguments:?}")]
    ConflictingArguments {
        /// Arguments that conflict
        arguments: Vec<String>,
    },

    /// Command execution failed
    #[error("Command execution failed: {command} - {reason}")]
    ExecutionFailed {
        /// Command that failed
        command: String,
        /// Reason for the error
        reason: String,
    },
}

impl ReleaseError {
    /// Get actionable recovery suggestions for this error
    pub fn recovery_suggestions(&self) -> Vec<String> {
        match self {
            ReleaseError::Workspace(WorkspaceError::RootNotFound) => vec![
                "Navigate to a directory containing a Cargo workspace".to_string(),
                "Ensure you have a Cargo.toml file with [workspace] section".to_string(),
            ],
            ReleaseError::Workspace(WorkspaceError::CircularDependency { packages }) => vec![
                format!(
                    "Review dependencies between packages: {}",
                    packages.join(", ")
                ),
                "Remove circular dependencies by restructuring package relationships".to_string(),
            ],
            ReleaseError::Git(GitError::DirtyWorkingDirectory) => vec![
                "Commit pending changes: git add . && git commit -m 'message'".to_string(),
                "Stash changes temporarily: git stash".to_string(),
                "Reset working directory: git reset --hard HEAD".to_string(),
            ],
            ReleaseError::Git(GitError::AuthenticationFailed { .. }) => vec![
                "Check SSH key configuration: ssh -T git@github.com".to_string(),
                "Verify git remote URL: git remote -v".to_string(),
                "Regenerate SSH keys if needed".to_string(),
            ],
            ReleaseError::Publish(PublishError::AuthenticationError) => vec![
                "Login to crates.io: cargo login".to_string(),
                "Verify API token is valid and has publish permissions".to_string(),
            ],
            ReleaseError::Publish(PublishError::RateLimitExceeded {
                retry_after_seconds,
            }) => vec![
                format!("Wait {} seconds before retrying", retry_after_seconds),
                "Use --publish-interval to add delays between packages".to_string(),
            ],
            _ => vec!["Check the error message above for specific details".to_string()],
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        !matches!(
            self,
            ReleaseError::Workspace(WorkspaceError::RootNotFound)
                | ReleaseError::Workspace(WorkspaceError::CircularDependency { .. })
                | ReleaseError::Git(GitError::NotRepository)
                | ReleaseError::Version(VersionError::InvalidVersion { .. })
                | ReleaseError::Publish(PublishError::AlreadyPublished { .. })
        )
    }
}

/// Convert kodegen_tools_git::GitError to our internal error types
///
/// This implementation preserves error type information by:
/// - Extracting IO errors to ReleaseError::Io (preserves std::io::Error type)
/// - Explicitly mapping all git-specific errors to appropriate GitError variants
impl From<kodegen_tools_git::GitError> for ReleaseError {
    fn from(error: kodegen_tools_git::GitError) -> Self {
        // Special case: preserve IO errors at top level for better error handling
        // This allows callers to match on ReleaseError::Io directly
        if let kodegen_tools_git::GitError::Io(io_err) = error {
            return ReleaseError::Io(io_err);
        }

        // Convert all git-specific errors to internal GitError representation
        let internal_error = match error {
            // Already handled above
            kodegen_tools_git::GitError::Io(_) => unreachable!(),

            // Repository and reference errors
            kodegen_tools_git::GitError::RepoNotFound(_path) => GitError::NotRepository,
            kodegen_tools_git::GitError::RemoteNotFound(name) => GitError::RemoteOperationFailed {
                operation: "find remote".to_string(),
                reason: format!("Remote '{}' not found", name),
            },
            kodegen_tools_git::GitError::BranchNotFound(name) => GitError::BranchOperationFailed {
                reason: format!("Branch '{}' not found", name),
            },
            kodegen_tools_git::GitError::ReferenceNotFound(name) => {
                GitError::RemoteOperationFailed {
                    operation: "find reference".to_string(),
                    reason: format!("Reference '{}' not found", name),
                }
            }

            // Operation errors
            kodegen_tools_git::GitError::MergeConflict(msg) => GitError::CommitFailed {
                reason: format!("Merge conflict: {}", msg),
            },
            kodegen_tools_git::GitError::InvalidInput(msg) => GitError::RemoteOperationFailed {
                operation: "git operation".to_string(),
                reason: msg,
            },
            kodegen_tools_git::GitError::Unsupported(op) => GitError::RemoteOperationFailed {
                operation: op.to_string(),
                reason: "Unsupported operation".to_string(),
            },
            kodegen_tools_git::GitError::Parse(msg) => GitError::RemoteOperationFailed {
                operation: "parse git data".to_string(),
                reason: msg,
            },
            kodegen_tools_git::GitError::ChannelClosed => GitError::RemoteOperationFailed {
                operation: "git operation".to_string(),
                reason: "Internal communication channel closed".to_string(),
            },
            kodegen_tools_git::GitError::Aborted => GitError::RemoteOperationFailed {
                operation: "git operation".to_string(),
                reason: "Operation aborted".to_string(),
            },

            // Gix library errors
            kodegen_tools_git::GitError::Gix(err) => GitError::RemoteOperationFailed {
                operation: "gix operation".to_string(),
                reason: err.to_string(),
            },

            // Worktree-related errors
            kodegen_tools_git::GitError::WorktreeAlreadyExists(path) => {
                GitError::BranchOperationFailed {
                    reason: format!("Worktree already exists at path: {}", path.display()),
                }
            }
            kodegen_tools_git::GitError::WorktreeNotFound(name) => {
                GitError::BranchOperationFailed {
                    reason: format!("Worktree not found: {}", name),
                }
            }
            kodegen_tools_git::GitError::WorktreeLocked(name) => GitError::BranchOperationFailed {
                reason: format!("Worktree is locked: {}", name),
            },
            kodegen_tools_git::GitError::BranchInUse(branch) => GitError::BranchOperationFailed {
                reason: format!(
                    "Branch '{}' is already checked out in another worktree",
                    branch
                ),
            },
            kodegen_tools_git::GitError::CannotModifyMainWorktree => {
                GitError::BranchOperationFailed {
                    reason: "Cannot modify main worktree".to_string(),
                }
            }
            kodegen_tools_git::GitError::InvalidWorktreeName(name) => {
                GitError::BranchOperationFailed {
                    reason: format!("Invalid worktree name: {}", name),
                }
            }
        };
        ReleaseError::Git(internal_error)
    }
}
