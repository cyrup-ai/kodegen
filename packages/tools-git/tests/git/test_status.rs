//! Tests for Git status operations

use kodegen_tools_git::{is_clean, current_branch, list_remotes, is_detached, init_repo};
use tempfile::TempDir;

#[tokio::test]
async fn test_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Repository should be clean initially (no commits yet, so HEAD doesn't exist)
    // Create initial commit
    std::fs::write(temp_dir.path().join("test.txt"), "test")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Now should be clean
    assert!(is_clean(&repo).await?);
    
    // Make a change
    std::fs::write(temp_dir.path().join("test.txt"), "modified")?;
    
    // Should not be clean
    assert!(!is_clean(&repo).await?);
    
    Ok(())
}

#[tokio::test]
async fn test_current_branch() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create initial commit to establish branch
    std::fs::write(temp_dir.path().join("test.txt"), "test")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    let branch = current_branch(&repo).await?;
    
    // Default branch might be 'main' or 'master' depending on Git configuration
    assert!(branch.name == "main" || branch.name == "master");
    assert!(branch.is_current);
    
    Ok(())
}

#[tokio::test]
async fn test_list_remotes() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // New repository should have no remotes
    let remotes = list_remotes(&repo).await?;
    assert_eq!(remotes.len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_is_detached() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create initial commit
    std::fs::write(temp_dir.path().join("test.txt"), "test")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Should not be detached on a branch
    assert!(!is_detached(&repo).await?);
    
    Ok(())
}
