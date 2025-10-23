//! Git push operations
//!
//! Provides functionality for pushing commits and tags to remote repositories.
//! Uses native git CLI since gix doesn't yet support push operations.
//!
//! **Dependency**: Requires git to be installed and available in PATH.
//!
//! # Authentication
//!
//! This module relies on git's configured authentication methods. Authentication
//! configuration is critical for production use to avoid hangs, timeouts, and failures.
//!
//! ## SSH (Recommended for automation)
//!
//! SSH authentication requires keys to be properly configured and loaded:
//! - SSH keys must be loaded in ssh-agent
//! - Or SSH key must not have a passphrase
//! - Respects user's `~/.ssh/config` settings
//! - Environment variable: `SSH_AUTH_SOCK` (set by ssh-agent)
//!
//! **Setup for CI/CD:**
//! ```bash
//! # Start SSH agent and add key
//! eval "$(ssh-agent -s)"
//! ssh-add ~/.ssh/id_rsa
//!
//! # Or disable strict host checking for CI
//! export GIT_SSH_COMMAND="ssh -o StrictHostKeyChecking=no"
//! ```
//!
//! ## HTTPS
//!
//! HTTPS authentication requires credential configuration:
//! - Credential helper: `git config --global credential.helper store`
//! - Or environment variables: `GIT_ASKPASS`, `GIT_USERNAME`, `GIT_PASSWORD`
//! - Or credentials stored in git config (not recommended for security)
//!
//! **WARNING**: HTTPS push will fail with an error if credentials are needed
//! because `GIT_TERMINAL_PROMPT=0` is set by this implementation to prevent
//! hanging on password prompts in automation scenarios.
//!
//! ## Preventing Hangs in CI/CD
//!
//! This implementation sets `GIT_TERMINAL_PROMPT=0` to prevent git from prompting
//! for credentials, which would cause indefinite hangs in automated environments.
//! If authentication is not properly configured, the push will fail immediately
//! rather than hang.
//!
//! **Recommended practices:**
//! ```rust
//! use std::env;
//!
//! // For SSH in CI/CD environments
//! env::set_var("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=no");
//!
//! // For HTTPS with credential helper
//! // Run: git config --global credential.helper store
//! // Or set GIT_ASKPASS to a script that provides credentials
//! ```
//!
//! # Examples
//!
//! ## CI/CD Setup (GitHub Actions)
//!
//! ```yaml
//! # Using SSH with GitHub Actions
//! - name: Setup SSH
//!   uses: webfactory/ssh-agent@v0.5.4
//!   with:
//!     ssh-private-key: ${{ secrets.SSH_PRIVATE_KEY }}
//!
//! # Using HTTPS with personal access token
//! - name: Configure Git Credentials
//!   run: |
//!     git config --global credential.helper store
//!     echo "https://${{ secrets.GITHUB_TOKEN }}@github.com" > ~/.git-credentials
//! ```
//!
//! ## Local Development
//!
//! ```bash
//! # SSH authentication (recommended)
//! eval "$(ssh-agent -s)"
//! ssh-add ~/.ssh/id_rsa
//!
//! # HTTPS with credential helper
//! git config --global credential.helper store
//! # First push will prompt for credentials, then store them
//!
//! # HTTPS with personal access token
//! git config --global credential.helper 'store --file ~/.git-credentials'
//! echo "https://username:token@github.com" > ~/.git-credentials
//! chmod 600 ~/.git-credentials
//! ```
//!
//! # Troubleshooting
//!
//! ## "Failed to authenticate" or "Permission denied"
//! - Verify SSH key is loaded: `ssh-add -l`
//! - Test SSH connection: `ssh -T git@github.com`
//! - Check credential helper: `git config --get credential.helper`
//!
//! ## "Operation timed out"
//! - Check network connectivity to remote
//! - Verify remote URL is correct: `git remote -v`
//! - Increase timeout via `PushOpts.timeout_secs`
//!
//! ## "Authentication required" in CI/CD
//! - Ensure SSH key is added to ssh-agent in CI workflow
//! - Or configure HTTPS credential helper before push
//! - Verify `GIT_SSH_COMMAND` or `GIT_ASKPASS` environment variables
//!
//! For more details, see:
//! - [Git Credential Storage](https://git-scm.com/docs/gitcredentials)
//! - [Git SSH Configuration](https://git-scm.com/docs/git-config#Documentation/git-config.txt-coresshCommand)

use crate::{GitError, GitResult, RepoHandle};
use tokio::process::Command;
use tokio::time::Duration;

/// Options for push operation
#[derive(Debug, Clone)]
pub struct PushOpts {
    /// Remote name (defaults to "origin")
    pub remote: String,
    /// Refspecs to push (empty means current branch)
    pub refspecs: Vec<String>,
    /// Force push
    pub force: bool,
    /// Push all tags
    pub tags: bool,
    /// Timeout in seconds (default: 300)
    pub timeout_secs: Option<u64>,
}

impl Default for PushOpts {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: false,
            timeout_secs: None,
        }
    }
}

/// Result of push operation
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Number of refs (branches/tags) successfully pushed
    ///
    /// Note: This counts the number of ref updates, not individual commits.
    /// For example, pushing a branch with 5 commits counts as 1 ref update.
    pub commits_pushed: usize,

    /// Number of tags pushed (conservative estimate)
    ///
    /// **Note:** Returns 1 when `--tags` is used and push succeeds, or counts
    /// the number of `refs/tags/*` refspecs provided. Does not parse git output
    /// for exact count due to fragility. Sufficient for most telemetry use cases.
    pub tags_pushed: usize,

    /// Any warnings or messages
    pub warnings: Vec<String>,
}

/// Push to remote repository
///
/// Pushes commits and/or tags to the specified remote using native git CLI.
///
/// Note: This uses the git command-line tool rather than gix library calls
/// because gix does not yet support push operations. Requires git to be
/// installed and available in PATH.
///
/// # Authentication
///
/// **IMPORTANT**: This function requires proper git authentication configuration.
/// See the [module-level documentation](index.html) for detailed authentication setup.
///
/// This implementation sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on credential
/// prompts in automated environments. If authentication is not configured, the push
/// will fail immediately with an error rather than hang waiting for user input.
///
/// **Quick setup:**
/// - **SSH**: Ensure keys are loaded in ssh-agent: `ssh-add ~/.ssh/id_rsa`
/// - **HTTPS**: Configure credential helper: `git config --global credential.helper store`
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `opts` - Push options
///
/// # Returns
///
/// Returns `PushResult` containing the number of refs successfully pushed
/// (not individual commits). If 3 branches are pushed, `commits_pushed` will be 3
/// regardless of how many commits each branch contains.
///
/// # Errors
///
/// Returns `GitError::InvalidInput` if:
/// - Push fails due to authentication issues
/// - git command is not found in PATH
/// - Network connectivity issues
/// - Remote repository rejects the push
/// - Operation times out (default: 300 seconds, configurable via `opts.timeout_secs`)
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push, PushOpts};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let result = push(&repo, PushOpts {
///     remote: "origin".to_string(),
///     refspecs: vec![],
///     force: false,
///     tags: false,
///     timeout_secs: None,
/// }).await?;
/// println!("Pushed {} commits", result.commits_pushed);
/// # Ok(())
/// # }
/// ```
pub async fn push(repo: &RepoHandle, opts: PushOpts) -> GitResult<PushResult> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let PushOpts {
        remote,
        refspecs,
        force,
        tags,
        timeout_secs,
    } = opts;

    // Default 5 minute timeout (configurable via opts)
    let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(300));

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.arg("push");

    // Prevent credential prompts from hanging
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    // Force English output for consistent parsing (locale-independent)
    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    if force {
        cmd.arg("--force");
    }

    if tags {
        cmd.arg("--tags");
    }

    cmd.arg(&remote);

    for refspec in &refspecs {
        cmd.arg(refspec);
    }

    // Spawn child process with handle for proper cancellation
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout and cancellation support using select!
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput(format!("Push operation timed out after {} seconds", timeout_secs.unwrap_or(300))));
        }
    };

    // Read stdout and stderr after process completes
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!("Push failed: {stderr}")));
    }

    // Parse output to estimate what was pushed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    // Count successful ref updates (branches/tags pushed)
    // Note: This counts refs, not individual commits, as accurate commit
    // counting would require additional git commands (git rev-list)
    let commits_pushed = combined
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();

            // Must contain the push arrow
            if !trimmed.contains(" -> ") {
                return false;
            }

            // Exclude errors and rejections
            if trimmed.starts_with('!')
                || trimmed.starts_with("error:")
                || trimmed.contains("[rejected]")
            {
                return false;
            }

            // Match successful update patterns:
            // "   abc123..def456  ref -> ref" (commit range)
            // " * [new branch]    ref -> ref" (new branch)
            // " + abc123...def456 ref -> ref" (forced update)
            trimmed.starts_with(|c: char| c.is_ascii_hexdigit())
                || trimmed.starts_with("* [new")
                || trimmed.starts_with('+')
        })
        .count();
    // Conservative tag counting: indicate whether tags were pushed
    // without attempting fragile output parsing
    let tags_pushed = if tags && output.status.success() {
        // --tags flag used and push succeeded
        1 // At least some tags were pushed (conservative estimate)
    } else if output.status.success() && refspecs.iter().any(|r| r.contains("refs/tags/")) {
        // Specific tag refspecs provided and push succeeded
        refspecs.iter().filter(|r| r.contains("refs/tags/")).count()
    } else {
        0
    };

    let mut warnings = Vec::new();
    // Check the force flag directly instead of parsing output (locale-independent)
    if force {
        warnings.push("Force push executed".to_string());
    }

    Ok(PushResult {
        commits_pushed,
        tags_pushed,
        warnings,
    })
}

/// Push current branch to remote
///
/// Convenience function that pushes the current branch to the specified remote.
/// Requires proper authentication configuration - see [module-level docs](index.html).
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name (defaults to "origin")
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push_current_branch};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// push_current_branch(&repo, "origin").await?;
/// # Ok(())
/// # }
/// ```
pub async fn push_current_branch(repo: &RepoHandle, remote: &str) -> GitResult<PushResult> {
    push(
        repo,
        PushOpts {
            remote: remote.to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: false,
            timeout_secs: None,
        },
    )
    .await
}

/// Push all tags to remote
///
/// Convenience function that pushes all tags to the specified remote.
/// Requires proper authentication configuration - see [module-level docs](index.html).
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name (defaults to "origin")
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push_tags};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// push_tags(&repo, "origin").await?;
/// # Ok(())
/// # }
/// ```
pub async fn push_tags(repo: &RepoHandle, remote: &str) -> GitResult<PushResult> {
    push(
        repo,
        PushOpts {
            remote: remote.to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: true,
            timeout_secs: None,
        },
    )
    .await
}

/// Delete a tag from remote repository
///
/// Requires proper authentication configuration - see [module-level docs](index.html).
/// Sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on authentication prompts.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `tag_name` - Name of the tag to delete
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, delete_remote_tag};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// delete_remote_tag(&repo, "origin", "v1.0.0").await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_remote_tag(repo: &RepoHandle, remote: &str, tag_name: &str) -> GitResult<()> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    // Normalize tag name: strip "refs/tags/" prefix if present
    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);

    // Validate tag name format
    if tag_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Tag name cannot be empty".to_string(),
        ));
    }
    if tag_name.contains("..") {
        return Err(GitError::InvalidInput(format!(
            "Invalid tag name: {tag_name}"
        )));
    }
    if tag_name.starts_with('/') {
        return Err(GitError::InvalidInput(format!(
            "Invalid tag name: {tag_name}"
        )));
    }

    let remote = remote.to_string();
    let tag_name_owned = tag_name.to_string();

    // Default 5 minute timeout
    let timeout_duration = Duration::from_secs(300);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts from hanging
    cmd.arg("push");
    cmd.arg(&remote);
    cmd.arg("--delete");
    cmd.arg(format!("refs/tags/{tag_name_owned}"));

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process with handle for proper cancellation
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout and cancellation support using select!
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("Delete remote tag operation timed out after 300 seconds".to_string()));
        }
    };

    // Read stdout and stderr after process completes
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote tag '{tag_name_owned}': {stderr}"
        )));
    }

    Ok(())
}

/// Delete a branch from remote repository
///
/// Requires proper authentication configuration - see [module-level docs](index.html).
/// Sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on authentication prompts.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `branch_name` - Name of the branch to delete
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, delete_remote_branch};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// delete_remote_branch(&repo, "origin", "v1.2.3").await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_remote_branch(
    repo: &RepoHandle,
    remote: &str,
    branch_name: &str,
) -> GitResult<()> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    // Normalize branch name: strip "refs/heads/" prefix if present
    let branch_name = branch_name
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_name);

    // Validate branch name format
    if branch_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Branch name cannot be empty".to_string(),
        ));
    }
    if branch_name.contains("..") {
        return Err(GitError::InvalidInput(format!(
            "Invalid branch name: {branch_name}"
        )));
    }
    if branch_name.starts_with('/') {
        return Err(GitError::InvalidInput(format!(
            "Invalid branch name: {branch_name}"
        )));
    }

    let remote = remote.to_string();
    let branch_name_owned = branch_name.to_string();

    // Default 5 minute timeout
    let timeout_duration = Duration::from_secs(300);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts from hanging
    cmd.arg("push");
    cmd.arg(&remote);
    cmd.arg("--delete");
    cmd.arg(format!("refs/heads/{branch_name_owned}")); // Use full ref to avoid ambiguity with tags

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process with handle for proper cancellation
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout and cancellation support using select!
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("Delete remote branch operation timed out after 300 seconds".to_string()));
        }
    };

    // Read stdout and stderr after process completes
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote branch '{branch_name_owned}': {stderr}"
        )));
    }

    Ok(())
}

/// Check if a branch exists on remote repository
///
/// Uses `git ls-remote` to check if a branch exists on the remote without
/// fetching all refs. This is faster and lighter than a full fetch.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `branch_name` - Name of the branch to check
///
/// # Returns
///
/// * `Ok(true)` - Branch exists on remote
/// * `Ok(false)` - Branch does not exist on remote
/// * `Err(_)` - Network or authentication error
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, check_remote_branch_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if check_remote_branch_exists(&repo, "origin", "v1.2.3").await? {
///     println!("Branch exists on remote");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn check_remote_branch_exists(
    repo: &RepoHandle,
    remote: &str,
    branch_name: &str,
) -> GitResult<bool> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    // Normalize branch name: strip "refs/heads/" prefix if present
    let branch_name = branch_name
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_name);

    // Validate branch name format
    if branch_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Branch name cannot be empty".to_string(),
        ));
    }

    let remote = remote.to_string();
    let branch_name_owned = branch_name.to_string();
    let refspec = format!("refs/heads/{branch_name_owned}");

    // Default 30 second timeout for ls-remote (should be quick)
    let timeout_duration = Duration::from_secs(30);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts
    cmd.arg("ls-remote");
    cmd.arg("--heads"); // Only list branches
    cmd.arg(&remote);
    cmd.arg(&refspec);

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("ls-remote operation timed out after 30 seconds".to_string()));
        }
    };

    // Read stdout and stderr
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "ls-remote failed: {stderr}"
        )));
    }

    // If output is non-empty, the branch exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Check if a tag exists on remote repository
///
/// Uses `git ls-remote` to check if a tag exists on the remote without
/// fetching all refs. This is faster and lighter than a full fetch.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `tag_name` - Name of the tag to check
///
/// # Returns
///
/// * `Ok(true)` - Tag exists on remote
/// * `Ok(false)` - Tag does not exist on remote
/// * `Err(_)` - Network or authentication error
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, check_remote_tag_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if check_remote_tag_exists(&repo, "origin", "v1.2.3").await? {
///     println!("Tag exists on remote");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn check_remote_tag_exists(
    repo: &RepoHandle,
    remote: &str,
    tag_name: &str,
) -> GitResult<bool> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    // Normalize tag name: strip "refs/tags/" prefix if present
    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);

    // Validate tag name format
    if tag_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Tag name cannot be empty".to_string(),
        ));
    }

    let remote = remote.to_string();
    let tag_name_owned = tag_name.to_string();
    let refspec = format!("refs/tags/{tag_name_owned}");

    // Default 30 second timeout for ls-remote (should be quick)
    let timeout_duration = Duration::from_secs(30);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts
    cmd.arg("ls-remote");
    cmd.arg("--tags"); // Only list tags
    cmd.arg(&remote);
    cmd.arg(&refspec);

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("ls-remote operation timed out after 30 seconds".to_string()));
        }
    };

    // Read stdout and stderr
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "ls-remote failed: {stderr}"
        )));
    }

    // If output is non-empty, the tag exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}
