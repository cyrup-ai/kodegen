//! Tests for git repository open and initialization operations.

use kodegen_tools_git::git::open::{
    RepositoryInfo, discover_repo, init_bare_repo, init_repo, is_repository, open_repo,
    probe_repository,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_is_repository_false() {
    let temp_dir = TempDir::new().unwrap();
    let result = is_repository(temp_dir.path()).await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn test_is_repository_nonexistent() {
    let result = is_repository("/nonexistent/path/that/does/not/exist")
        .await
        .unwrap();
    assert!(!result);
}

#[tokio::test]
async fn test_open_nonexistent_repo() {
    let result = open_repo("/nonexistent/path/that/does/not/exist").await;
    assert!(result.is_ok()); // AsyncTask succeeded
    let inner_result = result.unwrap();
    assert!(inner_result.is_err());

    match inner_result.unwrap_err() {
        kodegen_tools_git::GitError::InvalidInput(msg) => {
            assert!(msg.contains("does not exist"));
        }
        _ => panic!("Expected InvalidInput error"),
    }
}

#[tokio::test]
async fn test_discover_nonexistent_path() {
    let result = discover_repo("/nonexistent/path/that/does/not/exist").await;
    assert!(result.is_ok()); // AsyncTask succeeded
    let inner_result = result.unwrap();
    assert!(inner_result.is_err());
}

#[tokio::test]
async fn test_init_repo() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("new-repo");

    let result = init_repo(&repo_path).await;
    assert!(result.is_ok());

    // Verify the repository was created
    let is_repo = is_repository(&repo_path).await.unwrap();
    assert!(is_repo);

    // Verify it's not bare
    let info = probe_repository(&repo_path).await.unwrap().unwrap();
    assert!(!info.is_bare);
    assert!(info.work_dir.is_some());
}

#[tokio::test]
async fn test_init_bare_repo() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("bare-repo.git");

    let result = init_bare_repo(&repo_path).await;
    assert!(result.is_ok());

    // Verify the repository was created
    let is_repo = is_repository(&repo_path).await.unwrap();
    assert!(is_repo);

    // Verify it's bare
    let info = probe_repository(&repo_path).await.unwrap().unwrap();
    assert!(info.is_bare);
    assert!(info.work_dir.is_none());
}

#[tokio::test]
async fn test_init_existing_repo() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("existing-repo");

    // Initialize once
    let result1 = init_repo(&repo_path).await;
    assert!(result1.is_ok());

    // Try to initialize again - should fail
    let result2 = init_repo(&repo_path).await;
    assert!(result2.is_ok()); // AsyncTask succeeded
    let inner_result = result2.unwrap();
    assert!(inner_result.is_err());

    match inner_result.unwrap_err() {
        kodegen_tools_git::GitError::InvalidInput(msg) => {
            assert!(msg.contains("already a Git repository"));
        }
        _ => panic!("Expected InvalidInput error"),
    }
}

#[tokio::test]
async fn test_open_repo_integration() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("test-repo");

    // First initialize a repository
    let _init_result = init_repo(&repo_path).await.unwrap();

    // Then open it
    let open_result = open_repo(&repo_path).await;
    assert!(open_result.is_ok());

    let repo_handle = open_result.unwrap().unwrap();
    assert!(!repo_handle.raw().is_bare());
}

#[tokio::test]
async fn test_discover_repo_integration() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("test-repo");
    let subdir_path = repo_path.join("src").join("deep").join("nested");

    // Initialize a repository
    let _init_result = init_repo(&repo_path).await.unwrap();

    // Create nested subdirectories
    std::fs::create_dir_all(&subdir_path).unwrap();

    // Discover from nested directory
    let discover_result = discover_repo(&subdir_path).await;
    assert!(discover_result.is_ok());

    let repo_handle = discover_result.unwrap().unwrap();
    assert!(!repo_handle.raw().is_bare());
}

#[test]
fn test_repository_info() {
    let info = RepositoryInfo {
        path: PathBuf::from("/test/repo"),
        is_bare: false,
        git_dir: PathBuf::from("/test/repo/.git"),
        work_dir: Some(PathBuf::from("/test/repo")),
    };

    assert_eq!(info.path, PathBuf::from("/test/repo"));
    assert!(!info.is_bare);
    assert_eq!(info.git_dir, PathBuf::from("/test/repo/.git"));
    assert_eq!(info.work_dir, Some(PathBuf::from("/test/repo")));
}

#[test]
fn test_repository_info_bare() {
    let info = RepositoryInfo {
        path: PathBuf::from("/test/bare.git"),
        is_bare: true,
        git_dir: PathBuf::from("/test/bare.git"),
        work_dir: None,
    };

    assert_eq!(info.path, PathBuf::from("/test/bare.git"));
    assert!(info.is_bare);
    assert_eq!(info.git_dir, PathBuf::from("/test/bare.git"));
    assert!(info.work_dir.is_none());
}
