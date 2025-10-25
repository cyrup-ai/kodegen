//! Comprehensive Git Operations Integration Example
//!
//! This example creates a real git repository and exercises all operations
//! that were fixed for index corruption bugs:
//! - add (staging files)
//! - commit (creating commits)
//! - branch (creating/switching branches)
//! - reset (soft/mixed/hard)
//! - checkout (branches/commits)
//! - worktree (linked worktrees)
//!
//! After each operation, we verify that the git index maintains a valid
//! SHA-1 checksum and correct structure, proving our fixes work.

use anyhow::{Context, Result};
use kodegen_tools_git::{
    self as git, AddOpts, BranchOpts, CheckoutOpts, CommitOpts, RepoHandle, ResetMode, ResetOpts,
    WorktreeAddOpts,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Type alias for scenario function signature
type ScenarioFn =
    fn(
        &TestRepository,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ScenarioStats>> + '_>>;

/// Statistics from running a test scenario
#[derive(Debug, Clone)]
struct ScenarioStats {
    operations_count: usize,
    files_created: usize,
    index_verifications: usize,
    duration: Duration,
}

impl ScenarioStats {
    fn new() -> Self {
        Self {
            operations_count: 0,
            files_created: 0,
            index_verifications: 0,
            duration: Duration::ZERO,
        }
    }
}

/// Index integrity verification results
#[derive(Debug)]
#[allow(dead_code)]
struct IndexStats {
    checksum_valid: bool,
    entry_count: usize,
    version: u32,
}

/// Test repository with RAII cleanup
struct TestRepository {
    repo: RepoHandle,
    path: PathBuf,
    start_time: Instant,
}

impl TestRepository {
    /// Create a new test repository in workspace `tmp/` directory
    async fn new() -> Result<Self> {
        let workspace_root = std::env::current_dir()
            .context("Failed to get current directory")?
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists() && p.join("packages").exists())
            .context("Failed to find workspace root")?
            .to_path_buf();
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = workspace_root.join(format!("tmp/git_ops_test_{timestamp}"));

        // Create directory
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create test directory: {}", path.display()))?;

        // Initialize git repository
        let repo = git::init_repo(&path)
            .await
            .context("Failed to initialize git repository")?
            .map_err(|e| anyhow::anyhow!("Git error: {e}"))?;

        println!("[TEST] Created test repository at: {}", path.display());

        // Configure git identity (required for commits and reflog)
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .context("Failed to configure git user.name")?;

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .context("Failed to configure git user.email")?;

        let test_repo = Self {
            repo: repo.clone(),
            path: path.clone(),
            start_time: Instant::now(),
        };

        // Create initial commit to initialize index
        // This is necessary because gix::init() doesn't create the index file
        test_repo.create_file(".gitignore", b"# Git ignore file\n")?;

        git::add(
            repo.clone(),
            AddOpts {
                paths: vec![path.join(".gitignore")],
                update_only: false,
                force: false,
            },
        )
        .await
        .context("Failed to add .gitignore")?;

        git::commit(
            repo.clone(),
            CommitOpts {
                message: "Initial commit".to_string(),
                amend: false,
                all: false,
                author: None,
                committer: None,
            },
        )
        .await
        .context("Failed to create initial commit")?;

        // Verify index was created correctly
        test_repo.verify_index().await?;

        Ok(test_repo)
    }

    /// Create a file in the repository
    fn create_file(&self, name: &str, content: &[u8]) -> Result<PathBuf> {
        let file_path = self.path.join(name);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for {name}"))?;
        }

        std::fs::write(&file_path, content)
            .with_context(|| format!("Failed to write file: {name}"))?;

        Ok(file_path)
    }

    /// Verify git index integrity
    async fn verify_index(&self) -> Result<IndexStats> {
        verify_index_integrity(&self.repo).await
    }

    /// Clean up test repository
    fn cleanup(&self) -> Result<()> {
        println!("[TEST] Cleaning up repository at: {}", self.path.display());
        std::fs::remove_dir_all(&self.path)
            .with_context(|| format!("Failed to remove test directory: {}", self.path.display()))?;
        Ok(())
    }

    /// Get elapsed time since repo creation
    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Drop for TestRepository {
    fn drop(&mut self) {
        // Attempt cleanup, but don't panic if it fails during drop
        if let Err(e) = self.cleanup() {
            eprintln!("[WARN] Failed to cleanup test repository: {e}");
        }
    }
}

/// Verify index file integrity using gix
async fn verify_index_integrity(repo: &RepoHandle) -> Result<IndexStats> {
    let repo_clone = repo.clone();

    tokio::task::spawn_blocking(move || {
        let index = repo_clone.raw().index().context("Failed to open index")?;

        // Check if index has a valid checksum
        let checksum_valid = index.checksum().is_some();

        if !checksum_valid {
            anyhow::bail!("Index checksum is missing - index file is corrupted!");
        }

        let entry_count = index.entries().len();
        let version = match index.version() {
            gix::index::Version::V2 => 2,
            gix::index::Version::V3 => 3,
            gix::index::Version::V4 => 4,
        };

        Ok(IndexStats {
            checksum_valid,
            entry_count,
            version,
        })
    })
    .await
    .context("Task join error")?
}

/// Generate small text file content (100 bytes)
fn generate_small_text() -> Vec<u8> {
    b"Hello from kodegen_git test!\nThis is a small file.\nLine 3\nLine 4\nLine 5\n".to_vec()
}

/// Generate medium text file content (50KB)
fn generate_medium_text() -> Vec<u8> {
    let line = "The quick brown fox jumps over the lazy dog. 1234567890\n";
    let repeat_count = (50 * 1024) / line.len();
    line.repeat(repeat_count).into_bytes()
}

/// Generate large text file content (2MB)
fn generate_large_text() -> Vec<u8> {
    let line = "[2025-01-20 12:34:56] INFO: Processing request from client 192.168.1.100\n";
    let repeat_count = (2 * 1024 * 1024) / line.len();
    line.repeat(repeat_count).into_bytes()
}

/// Generate binary file content (1MB of patterned data)
fn generate_binary() -> Vec<u8> {
    let pattern = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    let repeat_count = (1024 * 1024) / pattern.len();
    pattern.repeat(repeat_count)
}

/// Performance tracker for operations
struct PerformanceTracker {
    operations: Vec<(String, Duration)>,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    fn track(&mut self, name: String, duration: Duration) {
        self.operations.push((name, duration));
    }

    fn report(&self) {
        if self.operations.is_empty() {
            println!("\n[PERF] No operations tracked");
            return;
        }

        let total: Duration = self.operations.iter().map(|(_, d)| *d).sum();
        let avg = total / self.operations.len() as u32;

        println!("\n[PERF] Performance Report:");
        println!("[PERF] =====================================");

        for (name, duration) in &self.operations {
            let pct = (duration.as_secs_f64() / total.as_secs_f64()) * 100.0;
            println!(
                "[PERF] {:40} {:>10.3}ms ({:>5.1}%)",
                name,
                duration.as_secs_f64() * 1000.0,
                pct
            );
        }

        println!("[PERF] =====================================");
        println!("[PERF] Total time: {:.3}s", total.as_secs_f64());
        println!("[PERF] Average:    {:.3}ms", avg.as_secs_f64() * 1000.0);

        if let Some((slowest_name, slowest_duration)) = self.slowest() {
            println!(
                "[PERF] Slowest:    {} ({:.3}ms)",
                slowest_name,
                slowest_duration.as_secs_f64() * 1000.0
            );
        }
    }

    fn slowest(&self) -> Option<(&str, Duration)> {
        self.operations
            .iter()
            .max_by_key(|(_, d)| *d)
            .map(|(n, d)| (n.as_str(), *d))
    }
}

/// Scenario: Test add operations
async fn scenario_add(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing ADD operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create files with different content types
    repo.create_file("file1.txt", &generate_small_text())?;
    repo.create_file("file2.txt", &generate_medium_text())?;
    repo.create_file("file3.log", &generate_large_text())?;
    repo.create_file("data/binary.bin", &generate_binary())?;
    repo.create_file("src/main.rs", b"fn main() {\n    println!(\"Hello\");\n}\n")?;
    stats.files_created = 5;

    // Add files one by one
    println!("[ADD] Adding file1.txt...");
    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.join("file1.txt")],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to add file1.txt")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    println!("[ADD] Adding file2.txt...");
    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.join("file2.txt")],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to add file2.txt")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Add multiple files in batch
    println!("[ADD] Adding remaining files in batch...");
    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.clone()],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to add remaining files")?;
    stats.operations_count += 1;

    let index_stats = repo.verify_index().await?;
    stats.index_verifications += 1;

    println!(
        "[ADD] ✓ Index valid: {} entries (v{})",
        index_stats.entry_count, index_stats.version
    );

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test commit operations
async fn scenario_commit(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing COMMIT operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create and stage files
    repo.create_file("README.md", b"# Test Project\n\nThis is a test.\n")?;
    repo.create_file("LICENSE", b"MIT License\n\nCopyright 2025\n")?;
    repo.create_file(
        "src/lib.rs",
        b"pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )?;
    stats.files_created = 3;

    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.clone()],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to stage files")?;
    stats.operations_count += 1;

    // Create initial commit
    println!("[COMMIT] Creating initial commit...");
    let commit_id = git::commit(
        repo.repo.clone(),
        CommitOpts {
            message: "Initial commit\n\nAdded README, LICENSE, and lib.rs".to_string(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        },
    )
    .await
    .context("Failed to create initial commit")?;
    stats.operations_count += 1;

    println!("[COMMIT] Created commit: {commit_id}");

    let index_stats = repo.verify_index().await?;
    stats.index_verifications += 1;
    println!(
        "[COMMIT] ✓ Index valid after commit: {} entries",
        index_stats.entry_count
    );

    // Modify files and commit again
    repo.create_file(
        "README.md",
        b"# Test Project\n\nUpdated readme.\n\n## Features\n",
    )?;
    repo.create_file(
        "CHANGELOG.md",
        b"# Changelog\n\n## v0.1.0\n- Initial release\n",
    )?;
    stats.files_created += 1;

    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.clone()],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to stage modified files")?;
    stats.operations_count += 1;

    println!("[COMMIT] Creating second commit...");
    let commit_id2 = git::commit(
        repo.repo.clone(),
        CommitOpts {
            message: "Update README and add CHANGELOG".to_string(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        },
    )
    .await
    .context("Failed to create second commit")?;
    stats.operations_count += 1;

    println!("[COMMIT] Created commit: {commit_id2}");

    repo.verify_index().await?;
    stats.index_verifications += 1;

    println!("[COMMIT] ✓ Commit chain verified");

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test branch operations
async fn scenario_branch(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing BRANCH operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create feature branch
    println!("[BRANCH] Creating feature/test branch...");
    git::branch(
        repo.repo.clone(),
        BranchOpts {
            name: "feature/test".to_string(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
    .context("Failed to create feature branch")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Create develop branch
    println!("[BRANCH] Creating develop branch...");
    git::branch(
        repo.repo.clone(),
        BranchOpts {
            name: "develop".to_string(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
    .context("Failed to create develop branch")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Get current branch
    let current = git::current_branch(&repo.repo)
        .await
        .context("Failed to get current branch")?;
    println!("[BRANCH] Current branch: {}", current.name);

    // List all branches
    let branches = git::list_branches(repo.repo.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
        .context("Failed to list branches")?;
    println!("[BRANCH] ✓ Found {} branches", branches.len());
    for branch_name in &branches {
        println!("[BRANCH]   - {branch_name}");
    }

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test reset operations
async fn scenario_reset(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing RESET operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create some commits to reset
    for i in 1..=3 {
        repo.create_file(
            &format!("reset_test_{i}.txt"),
            format!("Content {i}\n").as_bytes(),
        )?;
        stats.files_created += 1;

        git::add(
            repo.repo.clone(),
            AddOpts {
                paths: vec![repo.path.clone()],
                update_only: false,
                force: false,
            },
        )
        .await
        .with_context(|| format!("Failed to stage reset_test_{i}.txt"))?;

        git::commit(
            repo.repo.clone(),
            CommitOpts {
                message: format!("Add reset_test_{i}.txt"),
                amend: false,
                all: false,
                author: None,
                committer: None,
            },
        )
        .await
        .with_context(|| format!("Failed to commit reset_test_{i}.txt"))?;
        stats.operations_count += 2;
    }

    // Get current commit
    let before_reset = git::current_branch(&repo.repo)
        .await
        .context("Failed to get current branch before reset")?;
    println!("[RESET] Current commit: {}", before_reset.commit_hash);

    // Soft reset - keeps index and working directory
    println!("[RESET] Performing soft reset to HEAD~1...");
    git::reset(
        &repo.repo,
        ResetOpts {
            target: "HEAD~1".to_string(),
            mode: ResetMode::Soft,
            cancel_token: None,
        },
    )
    .await
    .context("Failed to perform soft reset")?;
    stats.operations_count += 1;

    let index_stats = repo.verify_index().await?;
    stats.index_verifications += 1;
    println!(
        "[RESET] ✓ Soft reset complete, index has {} entries",
        index_stats.entry_count
    );

    // Mixed reset - updates index but keeps working directory
    println!("[RESET] Performing mixed reset to HEAD~1...");
    git::reset(
        &repo.repo,
        ResetOpts {
            target: "HEAD~1".to_string(),
            mode: ResetMode::Mixed,
            cancel_token: None,
        },
    )
    .await
    .context("Failed to perform mixed reset")?;
    stats.operations_count += 1;

    let index_stats = repo.verify_index().await?;
    stats.index_verifications += 1;
    println!(
        "[RESET] ✓ Mixed reset complete, index has {} entries",
        index_stats.entry_count
    );

    // Hard reset - resets index and working directory
    println!("[RESET] Performing hard reset to HEAD...");
    git::reset(
        &repo.repo,
        ResetOpts {
            target: "HEAD".to_string(),
            mode: ResetMode::Hard,
            cancel_token: None,
        },
    )
    .await
    .context("Failed to perform hard reset")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;
    println!("[RESET] ✓ Hard reset complete");

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test checkout operations
async fn scenario_checkout(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing CHECKOUT operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create a feature branch with some files
    git::branch(
        repo.repo.clone(),
        BranchOpts {
            name: "feature/checkout-test".to_string(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
    .context("Failed to create checkout test branch")?;
    stats.operations_count += 1;

    // Checkout to the feature branch
    println!("[CHECKOUT] Switching to feature/checkout-test...");
    git::checkout(
        repo.repo.clone(),
        CheckoutOpts {
            reference: "feature/checkout-test".to_string(),
            force: false,
            paths: None,
        },
    )
    .await
    .context("Failed to checkout feature branch")?;
    stats.operations_count += 1;

    let index_stats = repo.verify_index().await?;
    stats.index_verifications += 1;
    println!(
        "[CHECKOUT] ✓ Switched to feature branch, index has {} entries",
        index_stats.entry_count
    );

    // Add some files on this branch
    repo.create_file("feature.txt", b"Feature work\n")?;
    stats.files_created += 1;

    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.join("feature.txt")],
            update_only: false,
            force: false,
        },
    )
    .await
    .context("Failed to add feature.txt")?;

    git::commit(
        repo.repo.clone(),
        CommitOpts {
            message: "Add feature work".to_string(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        },
    )
    .await
    .context("Failed to commit on feature branch")?;
    stats.operations_count += 2;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Checkout back to main
    println!("[CHECKOUT] Switching back to main...");
    git::checkout(
        repo.repo.clone(),
        CheckoutOpts {
            reference: "main".to_string(),
            force: false,
            paths: None,
        },
    )
    .await
    .context("Failed to checkout main")?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Verify feature.txt doesn't exist on main
    let feature_file = repo.path.join("feature.txt");
    if feature_file.exists() {
        anyhow::bail!("feature.txt should not exist on main branch!");
    }
    println!("[CHECKOUT] ✓ feature.txt correctly not present on main");

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test worktree operations
async fn scenario_worktree(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing WORKTREE operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create a new branch for worktree
    git::branch(
        repo.repo.clone(),
        BranchOpts {
            name: "worktree-test".to_string(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
    .context("Failed to create worktree test branch")?;
    stats.operations_count += 1;

    // Add a worktree
    let worktree_path = repo
        .path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?
        .join("worktree_branch");

    // Clean up worktree path if it exists from previous run
    if worktree_path.exists() {
        std::fs::remove_dir_all(&worktree_path).with_context(|| {
            format!(
                "Failed to cleanup old worktree: {}",
                worktree_path.display()
            )
        })?;
    }

    println!(
        "[WORKTREE] Creating worktree at {}...",
        worktree_path.display()
    );

    git::worktree_add(
        repo.repo.clone(),
        WorktreeAddOpts {
            path: worktree_path.clone(),
            committish: Some("worktree-test".to_string()),
            detach: false,
            force: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))?
    .context("Failed to add worktree")?;
    stats.operations_count += 1;

    // Verify worktree was created
    if !worktree_path.exists() {
        anyhow::bail!("Worktree directory was not created!");
    }

    // Worktree index is at <main_repo>/.git/worktrees/<name>/index
    let worktree_name = worktree_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid worktree path"))?;
    let worktree_admin_dir = repo.path.join(".git/worktrees").join(worktree_name);
    let worktree_index = worktree_admin_dir.join("index");

    if !worktree_index.exists() {
        anyhow::bail!(
            "Worktree index was not created at {}",
            worktree_index.display()
        );
    }

    println!("[WORKTREE] ✓ Worktree created with index");

    // Cleanup worktree
    std::fs::remove_dir_all(&worktree_path)
        .with_context(|| format!("Failed to cleanup worktree: {}", worktree_path.display()))?;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Test `open_repo` operations
async fn scenario_open_repo(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing OPEN_REPO operations...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Get the repository path
    let repo_path = repo.path.clone();

    println!(
        "[OPEN_REPO] Testing opening existing repository at: {}",
        repo_path.display()
    );

    // Open the existing repository using git::open_repo
    // This tests the correct error handling pattern for AsyncTask<GitResult<RepoHandle>>
    let opened_repo = git::open_repo(&repo_path)
        .await // Result<Result<RepoHandle, GitError>, RecvError>
        .map_err(|e| anyhow::anyhow!("Channel error during open_repo: {e}"))? // Handle RecvError (outer Result)
        .context("Failed to open repository")?; // Handle GitError (inner Result)
    stats.operations_count += 1;

    println!("[OPEN_REPO] ✓ Successfully opened repository");

    // Verify the opened repository works by reading current branch
    let current = git::current_branch(&opened_repo)
        .await
        .context("Failed to get current branch from opened repo")?;
    println!("[OPEN_REPO] ✓ Current branch: {}", current.name);

    // Test opening non-existent repository (should fail gracefully)
    let bad_path = PathBuf::from("/tmp/nonexistent_git_repo_12345");
    println!("[OPEN_REPO] Testing error handling with non-existent path...");

    let result = git::open_repo(&bad_path)
        .await
        .map_err(|e| anyhow::anyhow!("Channel error: {e}"));

    match result {
        Ok(Err(git_error)) => {
            println!("[OPEN_REPO] ✓ Correctly returned GitError for invalid path: {git_error}");
        }
        Ok(Ok(_)) => {
            anyhow::bail!("Expected error when opening non-existent repo, but succeeded!");
        }
        Err(channel_error) => {
            anyhow::bail!("Unexpected channel error: {channel_error}");
        }
    }
    stats.operations_count += 1;

    // Verify index integrity still works with opened repo
    let index_stats = verify_index_integrity(&opened_repo).await?;
    stats.index_verifications += 1;
    println!(
        "[OPEN_REPO] ✓ Opened repository index valid: {} entries (v{})",
        index_stats.entry_count, index_stats.version
    );

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Scenario: Complex workflow combining multiple operations
async fn scenario_complex_workflow(repo: &TestRepository) -> Result<ScenarioStats> {
    println!("\n[SCENARIO] Testing COMPLEX WORKFLOW...");
    let mut stats = ScenarioStats::new();
    let start = Instant::now();

    // Create main branch with baseline
    repo.create_file("app/main.rs", b"fn main() {\n    println!(\"v1.0\");\n}\n")?;
    repo.create_file(
        "app/lib.rs",
        b"pub fn version() -> &'static str { \"1.0\" }\n",
    )?;
    stats.files_created += 2;

    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.clone()],
            update_only: false,
            force: false,
        },
    )
    .await?;

    git::commit(
        repo.repo.clone(),
        CommitOpts {
            message: "Baseline v1.0".to_string(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        },
    )
    .await?;
    stats.operations_count += 2;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Create feature branch and make changes
    git::branch(
        repo.repo.clone(),
        BranchOpts {
            name: "feature/v2".to_string(),
            start_point: None,
            force: false,
            checkout: false,
            track: false,
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Channel error: {e}"))??;

    git::checkout(
        repo.repo.clone(),
        CheckoutOpts {
            reference: "feature/v2".to_string(),
            force: false,
            paths: None,
        },
    )
    .await?;
    stats.operations_count += 2;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    // Make 3 commits on feature branch
    for i in 1..=3 {
        repo.create_file(
            "app/lib.rs",
            format!("pub fn version() -> &'static str {{ \"2.{i}\" }}\n").as_bytes(),
        )?;

        git::add(
            repo.repo.clone(),
            AddOpts {
                paths: vec![repo.path.join("app/lib.rs")],
                update_only: false,
                force: false,
            },
        )
        .await?;

        git::commit(
            repo.repo.clone(),
            CommitOpts {
                message: format!("Update to v2.{i}"),
                amend: false,
                all: false,
                author: None,
                committer: None,
            },
        )
        .await?;
        stats.operations_count += 2;

        repo.verify_index().await?;
        stats.index_verifications += 1;
    }

    println!("[COMPLEX] Created 3 commits on feature/v2");

    // Switch back to main and make conflicting changes
    git::checkout(
        repo.repo.clone(),
        CheckoutOpts {
            reference: "main".to_string(),
            force: false,
            paths: None,
        },
    )
    .await?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    repo.create_file(
        "app/main.rs",
        b"fn main() {\n    println!(\"v1.5 - stable\");\n}\n",
    )?;
    stats.files_created += 1;

    git::add(
        repo.repo.clone(),
        AddOpts {
            paths: vec![repo.path.join("app/main.rs")],
            update_only: false,
            force: false,
        },
    )
    .await?;

    git::commit(
        repo.repo.clone(),
        CommitOpts {
            message: "Update main to v1.5".to_string(),
            amend: false,
            all: false,
            author: None,
            committer: None,
        },
    )
    .await?;
    stats.operations_count += 2;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    println!("[COMPLEX] ✓ Created divergent history");

    // Demonstrate reset in complex scenario
    println!("[COMPLEX] Resetting to demonstrate state management...");
    git::reset(
        &repo.repo,
        ResetOpts {
            target: "HEAD".to_string(),
            mode: ResetMode::Mixed,
            cancel_token: None,
        },
    )
    .await?;
    stats.operations_count += 1;

    repo.verify_index().await?;
    stats.index_verifications += 1;

    println!("[COMPLEX] ✓ Complex workflow completed successfully");

    stats.duration = start.elapsed();
    Ok(stats)
}

/// Main entry point - runs all test scenarios
#[tokio::main]
async fn main() -> Result<()> {
    println!("==============================================");
    println!("Git Operations Integration Test");
    println!("==============================================");
    println!("\nThis example exercises all git operations that were");
    println!("fixed for index corruption bugs, verifying that the");
    println!("git index maintains valid checksums throughout.\n");

    let mut perf_tracker = PerformanceTracker::new();

    // Create test repository
    println!("[INIT] Creating test repository...");
    let start = Instant::now();
    let repo = TestRepository::new().await?;
    perf_tracker.track("Repository initialization".to_string(), start.elapsed());

    println!("[INIT] ✓ Repository created at: {}", repo.path.display());
    println!(
        "[INIT] ✓ Total elapsed: {:.3}s\n",
        repo.elapsed().as_secs_f64()
    );

    // Run all scenarios
    let scenarios: Vec<(&str, ScenarioFn)> = vec![
        ("OPEN_REPO", |r| Box::pin(scenario_open_repo(r))),
        ("ADD", |r| Box::pin(scenario_add(r))),
        ("COMMIT", |r| Box::pin(scenario_commit(r))),
        ("BRANCH", |r| Box::pin(scenario_branch(r))),
        ("RESET", |r| Box::pin(scenario_reset(r))),
        ("CHECKOUT", |r| Box::pin(scenario_checkout(r))),
        ("WORKTREE", |r| Box::pin(scenario_worktree(r))),
        ("COMPLEX_WORKFLOW", |r| {
            Box::pin(scenario_complex_workflow(r))
        }),
    ];

    let mut total_stats = ScenarioStats::new();
    let mut scenario_count = 0;

    for (name, scenario_fn) in scenarios {
        let start = Instant::now();
        match scenario_fn(&repo).await {
            Ok(stats) => {
                let duration = start.elapsed();
                perf_tracker.track(format!("Scenario: {name}"), duration);

                println!(
                    "[{}] ✓ Completed in {:.3}s ({} ops, {} files, {} verifications)",
                    name,
                    stats.duration.as_secs_f64(),
                    stats.operations_count,
                    stats.files_created,
                    stats.index_verifications
                );

                total_stats.operations_count += stats.operations_count;
                total_stats.files_created += stats.files_created;
                total_stats.index_verifications += stats.index_verifications;
                total_stats.duration += stats.duration;
                scenario_count += 1;
            }
            Err(e) => {
                eprintln!("[{name}] ✗ FAILED: {e:?}");
                eprintln!("\nTest failed. Check the error above for details.");
                return Err(e);
            }
        }
    }

    // Final verification
    println!("\n[FINAL] Performing final index verification...");
    let final_index = repo.verify_index().await?;
    println!(
        "[FINAL] ✓ Index is valid: {} entries (version {})",
        final_index.entry_count, final_index.version
    );

    // Print summary
    println!("\n==============================================");
    println!("Test Summary");
    println!("==============================================");
    println!("Scenarios completed:     {scenario_count}");
    println!("Total operations:        {}", total_stats.operations_count);
    println!("Files created:           {}", total_stats.files_created);
    println!(
        "Index verifications:     {}",
        total_stats.index_verifications
    );
    println!(
        "Total scenario time:     {:.3}s",
        total_stats.duration.as_secs_f64()
    );
    println!(
        "Total elapsed time:      {:.3}s",
        repo.elapsed().as_secs_f64()
    );

    // Performance report
    perf_tracker.report();

    println!("\n==============================================");
    println!("✓ ALL TESTS PASSED");
    println!("==============================================");
    println!("\nThe git index maintained valid checksums throughout");
    println!("all operations, proving our index corruption fixes work!");
    println!("\nRepository will be cleaned up on exit.\n");

    Ok(())
}
