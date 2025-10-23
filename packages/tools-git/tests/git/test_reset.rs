//! Tests for Git reset operations

use kodegen_tools_git::{reset, reset_soft, reset_mixed, reset_hard, ResetOpts, ResetMode, init_repo, head_commit};
use tempfile::TempDir;

#[tokio::test]
async fn test_reset_soft() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create two commits
    std::fs::write(temp_dir.path().join("test.txt"), "first")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    let first_commit = kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "First commit".to_string(),
        ..Default::default()
    }).await?;
    
    std::fs::write(temp_dir.path().join("test.txt"), "second")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Second commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Reset soft to first commit
    reset_soft(&repo, &first_commit.to_string()).await?;
    
    // HEAD should be at first commit
    let current = head_commit(&repo).await?;
    assert_eq!(current, first_commit.to_string());
    
    Ok(())
}

#[tokio::test]
async fn test_reset_modes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create initial commit
    std::fs::write(temp_dir.path().join("test.txt"), "content")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    let commit_id = kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Test different reset modes
    reset(&repo, ResetOpts {
        target: commit_id.to_string(),
        mode: ResetMode::Soft,
    }).await?;
    
    reset(&repo, ResetOpts {
        target: commit_id.to_string(),
        mode: ResetMode::Mixed,
    }).await?;
    
    reset(&repo, ResetOpts {
        target: commit_id.to_string(),
        mode: ResetMode::Hard,
    }).await?;
    
    Ok(())
}
