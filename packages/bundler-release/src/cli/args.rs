//! Command line argument parsing and validation.
//!
//! This module provides comprehensive CLI argument parsing using clap,
//! with proper validation and error handling.

use crate::version::VersionBump;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::time::Duration;

/// Cyrup Release - Production-quality release management for Rust workspaces
#[derive(Parser, Debug)]
#[command(
    name = "cyrup_release",
    version,
    about = "Production-quality release management for Rust workspaces",
    long_about = "Cyrup Release provides atomic release operations with proper error handling,
automatic internal dependency version synchronization, and rollback capabilities
including crate yanking for published packages."
)]
pub struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Path to workspace root (defaults to current directory)
    #[arg(short, long, global = true, value_name = "PATH")]
    pub workspace: Option<PathBuf>,

    /// Path to state file (defaults to .cyrup_release_state.json)
    #[arg(long, global = true, value_name = "PATH")]
    pub state_file: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Release packages with version bump
    Release {
        /// Type of version bump to perform
        #[arg(value_enum)]
        bump_type: BumpType,

        /// Perform dry run without making changes
        #[arg(short, long)]
        dry_run: bool,

        /// Skip validation checks
        #[arg(long)]
        skip_validation: bool,

        /// Force release even if working directory is dirty
        #[arg(long)]
        allow_dirty: bool,

        /// Don't push to remote repository
        #[arg(long)]
        no_push: bool,

        /// Registry to publish to (defaults to crates.io)
        #[arg(long, value_name = "REGISTRY")]
        registry: Option<String>,

        /// Delay between package publishes in seconds
        #[arg(long, default_value = "15", value_name = "SECONDS")]
        package_delay: u64,

        /// Maximum number of retry attempts for publishing
        #[arg(long, default_value = "3", value_name = "COUNT")]
        max_retries: usize,

        /// Timeout for individual operations in seconds
        #[arg(long, default_value = "300", value_name = "SECONDS")]
        timeout: u64,

        /// Don't create backups during operation
        #[arg(long)]
        no_backup: bool,

        /// Skip GitHub release creation (releases are created by default)
        #[arg(long)]
        no_github_release: bool,

        /// GitHub repository (format: owner/repo)
        #[arg(long, value_name = "OWNER/REPO")]
        github_repo: Option<String>,

        /// Mark as draft GitHub release
        #[arg(long)]
        github_draft: bool,

        /// Custom release notes file
        #[arg(long, value_name = "FILE")]
        release_notes: Option<PathBuf>,

        /// Skip creating distributable bundles (bundles are created by default)
        #[arg(long)]
        no_bundles: bool,

        /// Skip uploading bundles to GitHub release
        #[arg(long)]
        no_upload_bundles: bool,

        /// Continue release even if GitHub operations fail
        #[arg(long)]
        continue_on_github_error: bool,

        /// Keep temporary clone directory for debugging (don't cleanup)
        #[arg(long)]
        keep_temp: bool,

        /// Don't automatically clear orphaned remote branches/tags from failed releases
        #[arg(long)]
        no_clear_runway: bool,
    },

    /// Bundle binaries into platform-specific installers (automatic by default)
    Bundle {
        /// Skip building binaries (assumes they're already built)
        #[arg(long)]
        no_build: bool,

        /// Use release mode (default: true)
        #[arg(short, long, default_value = "true")]
        release: bool,

        /// Force rebuild of Docker image even if it exists
        #[arg(long)]
        rebuild_image: bool,

        /// Only bundle for current platform (default: all platforms)
        #[arg(long)]
        current_platform_only: bool,

        /// Specific platform to bundle for (deb, rpm, appimage, app, dmg, msi, nsis)
        #[arg(short, long)]
        platform: Option<String>,

        /// Upload bundles to GitHub release
        #[arg(short, long)]
        upload: bool,

        /// Override product name
        #[arg(long)]
        name: Option<String>,

        /// Override version
        #[arg(long)]
        version: Option<String>,

        /// Target triple for cross-compilation
        #[arg(short, long)]
        target: Option<String>,

        /// GitHub repository (format: owner/repo)
        #[arg(long, requires = "upload")]
        github_repo: Option<String>,
        
        // === Docker Resource Limits ===
        
        /// Maximum memory for Docker containers (e.g., "4g", "2048m")
        /// Default: Auto-detected (50% of system RAM, min 2GB, max 8GB)
        #[arg(long, value_name = "SIZE")]
        docker_memory: Option<String>,
        
        /// Maximum memory + swap for Docker containers (e.g., "6g", "3072m")
        /// Default: docker_memory + 2GB
        #[arg(long, value_name = "SIZE")]
        docker_memory_swap: Option<String>,
        
        /// Maximum CPUs for Docker containers (e.g., "2", "1.5")
        /// Default: Auto-detected (50% of system CPUs, minimum 2)
        #[arg(long, value_name = "COUNT")]
        docker_cpus: Option<String>,
        
        /// Maximum processes in Docker containers
        /// Prevents fork bombs and runaway process creation
        #[arg(long, default_value = "1000", value_name = "COUNT")]
        docker_pids_limit: u32,
    },

    /// Rollback a failed or completed release
    Rollback {
        /// Force rollback even if state indicates success
        #[arg(short, long)]
        force: bool,

        /// Only rollback git operations (don't yank packages)
        #[arg(long)]
        git_only: bool,

        /// Only yank published packages (don't touch git)
        #[arg(long, conflicts_with = "git_only")]
        packages_only: bool,

        /// Confirm rollback without prompting
        #[arg(short, long)]
        yes: bool,
    },

    /// Resume an interrupted release
    Resume {
        /// Force resume even if state seems inconsistent
        #[arg(short, long)]
        force: bool,

        /// Reset to specific phase before resuming
        #[arg(long, value_enum)]
        reset_to_phase: Option<ResumePhase>,

        /// Don't validate state before resuming
        #[arg(long)]
        skip_validation: bool,
    },

    /// Show status of current or last release
    Status {
        /// Show detailed status information
        #[arg(short, long)]
        detailed: bool,

        /// Show release history
        #[arg(long)]
        history: bool,

        /// Format output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Clean up old state files and backups
    Cleanup {
        /// Remove all state files including backups
        #[arg(short, long)]
        all: bool,

        /// Remove state files older than N days
        #[arg(long, value_name = "DAYS")]
        older_than: Option<u32>,

        /// Confirm cleanup without prompting
        #[arg(short, long)]
        yes: bool,
    },

    /// Validate workspace for release readiness
    Validate {
        /// Fix validation issues automatically where possible
        #[arg(long)]
        fix: bool,

        /// Show detailed validation report
        #[arg(short, long)]
        detailed: bool,

        /// Format output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Preview version bump without making changes
    Preview {
        /// Type of version bump to preview
        #[arg(value_enum)]
        bump_type: BumpType,

        /// Show detailed preview including file changes
        #[arg(short, long)]
        detailed: bool,

        /// Format output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Type of version bump
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum BumpType {
    /// Bump major version (breaking changes)
    Major,
    /// Bump minor version (new features)
    Minor,
    /// Bump patch version (bug fixes)
    Patch,
    /// Set exact version
    Exact,
}

/// Phase to reset to when resuming
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum ResumePhase {
    /// Reset to validation phase
    Validation,
    /// Reset to version update phase
    VersionUpdate,
    /// Reset to git operations phase
    GitOperations,
    /// Reset to publishing phase
    Publishing,
}

impl TryFrom<BumpType> for VersionBump {
    type Error = String;
    
    fn try_from(bump_type: BumpType) -> Result<Self, Self::Error> {
        match bump_type {
            BumpType::Major => Ok(VersionBump::Major),
            BumpType::Minor => Ok(VersionBump::Minor),
            BumpType::Patch => Ok(VersionBump::Patch),
            BumpType::Exact => Err(
                "Exact version bump requires --version parameter (not yet implemented)".to_string()
            ),
        }
    }
}

impl Args {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get workspace path or default to current directory
    pub fn workspace_path(&self) -> PathBuf {
        self.workspace.clone().unwrap_or_else(|| PathBuf::from("."))
    }

    /// Get state file path or default
    pub fn state_file_path(&self) -> PathBuf {
        self.state_file.clone()
            .unwrap_or_else(|| PathBuf::from(".cyrup_release_state.json"))
    }

    /// Check if running in verbose mode
    pub fn is_verbose(&self) -> bool {
        self.verbose && !self.quiet
    }

    /// Check if running in quiet mode
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Validate arguments for consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check for conflicting global options
        if self.verbose && self.quiet {
            return Err("Cannot specify both --verbose and --quiet".to_string());
        }

        // Validate workspace path if provided
        if let Some(ref workspace) = self.workspace {
            if !workspace.exists() {
                return Err(format!("Workspace path does not exist: {}", workspace.display()));
            }
            if !workspace.is_dir() {
                return Err(format!("Workspace path is not a directory: {}", workspace.display()));
            }
        }

        // Validate state file path if provided
        if let Some(ref state_file) = self.state_file
            && let Some(parent) = state_file.parent()
            && !parent.exists() {
                return Err(format!("State file directory does not exist: {}", parent.display()));
            }

        // Validate command-specific arguments
        match &self.command {
            Command::Release { 
                package_delay, 
                max_retries, 
                timeout,
                .. 
            } => {
                if *package_delay > 3600 {
                    return Err("Package delay cannot exceed 1 hour (3600 seconds)".to_string());
                }
                if *max_retries > 10 {
                    return Err("Max retries cannot exceed 10".to_string());
                }
                if *timeout < 30 {
                    return Err("Timeout cannot be less than 30 seconds".to_string());
                }
                if *timeout > 3600 {
                    return Err("Timeout cannot exceed 1 hour (3600 seconds)".to_string());
                }
            }
            Command::Cleanup { older_than, .. } => {
                if let Some(days) = older_than
                    && *days > 365 {
                        return Err("Cleanup age cannot exceed 365 days".to_string());
                    }
            }
            _ => {}
        }

        Ok(())
    }
}

impl Command {
    /// Get the command name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Command::Release { .. } => "release",
            Command::Bundle { .. } => "bundle",
            Command::Rollback { .. } => "rollback",
            Command::Resume { .. } => "resume",
            Command::Status { .. } => "status",
            Command::Cleanup { .. } => "cleanup",
            Command::Validate { .. } => "validate",
            Command::Preview { .. } => "preview",
        }
    }

    /// Check if this command requires an existing release state
    pub fn requires_state(&self) -> bool {
        matches!(self, Command::Rollback { .. } | Command::Resume { .. })
    }

    /// Check if this command modifies the workspace
    pub fn is_modifying(&self) -> bool {
        matches!(
            self,
            Command::Release { dry_run: false, .. } |
            Command::Bundle { .. } |
            Command::Rollback { .. } | 
            Command::Resume { .. } |
            Command::Validate { fix: true, .. }
        )
    }

    /// Check if this command requires workspace validation
    pub fn requires_validation(&self) -> bool {
        matches!(
            self,
            Command::Release { skip_validation: false, .. } |
            Command::Resume { skip_validation: false, .. }
        )
    }
}

/// Configuration derived from command line arguments
#[derive(Debug)]
pub struct RuntimeConfig {
    /// Workspace root path
    pub workspace_path: PathBuf,
    /// State file path
    pub state_file_path: PathBuf,
    /// Verbosity level
    pub verbosity: VerbosityLevel,
    /// Package delay duration
    pub package_delay: Duration,
    /// Maximum retry attempts
    pub max_retries: usize,
    /// Operation timeout
    pub timeout: Duration,
    /// Registry to use
    pub registry: Option<String>,
    /// Whether to create backups
    pub create_backups: bool,
    /// Output manager for colored terminal output
    output: super::OutputManager,
}

/// Verbosity level for output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Minimal output
    Quiet,
    /// Standard output
    Normal,
    /// Detailed output
    Verbose,
}

impl From<&Args> for RuntimeConfig {
    fn from(args: &Args) -> Self {
        let verbosity = if args.quiet {
            VerbosityLevel::Quiet
        } else if args.verbose {
            VerbosityLevel::Verbose
        } else {
            VerbosityLevel::Normal
        };

        let (package_delay, max_retries, timeout, registry, create_backups) = match &args.command {
            Command::Release {
                package_delay,
                max_retries,
                timeout,
                registry,
                no_backup,
                ..
            } => (
                Duration::from_secs(*package_delay),
                *max_retries,
                Duration::from_secs(*timeout),
                registry.clone(),
                !no_backup,
            ),
            _ => (
                Duration::from_secs(15), // Default 15 seconds
                3,                       // Default 3 retries
                Duration::from_secs(300), // Default 5 minutes
                None,                    // Default registry
                true,                    // Create backups by default
            ),
        };

        let output = super::OutputManager::new(
            verbosity == VerbosityLevel::Verbose,
            verbosity == VerbosityLevel::Quiet,
        );

        Self {
            workspace_path: args.workspace_path(),
            state_file_path: args.state_file_path(),
            verbosity,
            package_delay,
            max_retries,
            timeout,
            registry,
            create_backups,
            output,
        }
    }
}

impl RuntimeConfig {
    /// Check if output should be suppressed
    pub fn is_quiet(&self) -> bool {
        self.verbosity == VerbosityLevel::Quiet
    }

    /// Check if verbose output is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbosity == VerbosityLevel::Verbose
    }

    /// Print message if not in quiet mode
    pub fn println(&self, message: &str) {
        self.output.println(message);
    }

    /// Print verbose message if in verbose mode
    pub fn verbose_println(&self, message: &str) {
        self.output.verbose(message);
    }

    /// Print error message (always shown)
    pub fn error_println(&self, message: &str) {
        self.output.error(message);
    }

    /// Print warning message if not in quiet mode
    pub fn warning_println(&self, message: &str) {
        self.output.warn(message);
    }

    /// Print success message if not in quiet mode
    pub fn success_println(&self, message: &str) {
        self.output.success(message);
    }

    /// Print success message (alias for success_println for convenience)
    pub fn success(&self, message: &str) {
        self.output.success(message);
    }

    /// Print warning message (alias for warning_println for convenience)
    pub fn warn(&self, message: &str) {
        self.output.warn(message);
    }

    /// Print error message (always shown, alias for error_println)
    pub fn error(&self, message: &str) {
        self.output.error(message);
    }

    /// Print info message
    pub fn info(&self, message: &str) {
        self.output.info(message);
    }

    /// Print progress message
    pub fn progress(&self, message: &str) {
        self.output.progress(message);
    }

    /// Print section header
    pub fn section(&self, title: &str) {
        self.output.section(title);
    }

    /// Print indented text
    pub fn indent(&self, message: &str) {
        self.output.indent(message);
    }
}