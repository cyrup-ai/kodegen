mod common;

use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting git tools example");

    // Connect to kodegen server with git category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Git])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    let test_repo = std::env::temp_dir().join("kodegen_git_test");

    // 1. GIT_INIT - Initialize a repository
    info!("1. Testing git_init");
    match client.call_tool(
        tools::GIT_INIT,
        json!({ "path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Initialized repo: {:?}", result),
        Err(e) => error!("Failed to init: {}", e),
    }

    // 2. GIT_OPEN - Open the repository
    info!("2. Testing git_open");
    match client.call_tool(
        tools::GIT_OPEN,
        json!({ "path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Opened repo: {:?}", result),
        Err(e) => error!("Failed to open: {}", e),
    }

    // 3. GIT_DISCOVER - Discover git repository from path
    info!("3. Testing git_discover");
    match client.call_tool(
        tools::GIT_DISCOVER,
        json!({ "path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Discovered repo: {:?}", result),
        Err(e) => error!("Failed to discover: {}", e),
    }

    // 4. GIT_BRANCH_LIST - List branches
    info!("4. Testing git_branch_list");
    match client.call_tool(
        tools::GIT_BRANCH_LIST,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Branch list: {:?}", result),
        Err(e) => error!("Failed to list branches: {}", e),
    }

    // 5. GIT_BRANCH_CREATE - Create a new branch
    info!("5. Testing git_branch_create");
    match client.call_tool(
        tools::GIT_BRANCH_CREATE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch_name": "feature-test"
        })
    ).await {
        Ok(result) => info!("Created branch: {:?}", result),
        Err(e) => error!("Failed to create branch: {}", e),
    }

    // 6. GIT_BRANCH_RENAME - Rename a branch
    info!("6. Testing git_branch_rename");
    match client.call_tool(
        tools::GIT_BRANCH_RENAME,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "old_name": "feature-test",
            "new_name": "feature-renamed"
        })
    ).await {
        Ok(result) => info!("Renamed branch: {:?}", result),
        Err(e) => error!("Failed to rename branch: {}", e),
    }

    // 7. GIT_CHECKOUT - Checkout a branch
    info!("7. Testing git_checkout");
    match client.call_tool(
        tools::GIT_CHECKOUT,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch": "feature-renamed"
        })
    ).await {
        Ok(result) => info!("Checked out: {:?}", result),
        Err(e) => error!("Failed to checkout: {}", e),
    }

    // 8. GIT_ADD - Add files to staging
    info!("8. Testing git_add");
    match client.call_tool(
        tools::GIT_ADD,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "paths": ["."]
        })
    ).await {
        Ok(result) => info!("Added files: {:?}", result),
        Err(e) => error!("Failed to add: {}", e),
    }

    // 9. GIT_COMMIT - Commit changes
    info!("9. Testing git_commit");
    match client.call_tool(
        tools::GIT_COMMIT,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "message": "Initial commit"
        })
    ).await {
        Ok(result) => info!("Committed: {:?}", result),
        Err(e) => error!("Failed to commit: {}", e),
    }

    // 10. GIT_LOG - View commit log
    info!("10. Testing git_log");
    match client.call_tool(
        tools::GIT_LOG,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "max_count": 10
        })
    ).await {
        Ok(result) => info!("Log: {:?}", result),
        Err(e) => error!("Failed to get log: {}", e),
    }

    // 11. GIT_FETCH - Fetch from remote
    info!("11. Testing git_fetch");
    match client.call_tool(
        tools::GIT_FETCH,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "remote": "origin"
        })
    ).await {
        Ok(result) => info!("Fetched: {:?}", result),
        Err(e) => info!("Expected error (no remote configured): {}", e),
    }

    // 12. GIT_MERGE - Merge branches
    info!("12. Testing git_merge");
    match client.call_tool(
        tools::GIT_MERGE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch": "main"
        })
    ).await {
        Ok(result) => info!("Merged: {:?}", result),
        Err(e) => error!("Failed to merge: {}", e),
    }

    // 13. GIT_CLONE - Clone repository (demo with public repo)
    info!("13. Testing git_clone (skipped - requires network)");

    // 14. GIT_WORKTREE_ADD - Add worktree
    info!("14. Testing git_worktree_add");
    let worktree_path = test_repo.parent()
        .map(|p| p.join("worktree_test"))
        .unwrap_or_else(|| std::env::temp_dir().join("worktree_test"));
    match client.call_tool(
        tools::GIT_WORKTREE_ADD,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy(),
            "branch": "feature-renamed"
        })
    ).await {
        Ok(result) => info!("Added worktree: {:?}", result),
        Err(e) => error!("Failed to add worktree: {}", e),
    }

    // 15. GIT_WORKTREE_LIST - List worktrees
    info!("15. Testing git_worktree_list");
    match client.call_tool(
        tools::GIT_WORKTREE_LIST,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Worktrees: {:?}", result),
        Err(e) => error!("Failed to list worktrees: {}", e),
    }

    // 16. GIT_WORKTREE_LOCK - Lock a worktree
    info!("16. Testing git_worktree_lock");
    match client.call_tool(
        tools::GIT_WORKTREE_LOCK,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await {
        Ok(result) => info!("Locked worktree: {:?}", result),
        Err(e) => error!("Failed to lock worktree: {}", e),
    }

    // 17. GIT_WORKTREE_UNLOCK - Unlock a worktree
    info!("17. Testing git_worktree_unlock");
    match client.call_tool(
        tools::GIT_WORKTREE_UNLOCK,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await {
        Ok(result) => info!("Unlocked worktree: {:?}", result),
        Err(e) => error!("Failed to unlock worktree: {}", e),
    }

    // 18. GIT_WORKTREE_REMOVE - Remove a worktree
    info!("18. Testing git_worktree_remove");
    match client.call_tool(
        tools::GIT_WORKTREE_REMOVE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await {
        Ok(result) => info!("Removed worktree: {:?}", result),
        Err(e) => error!("Failed to remove worktree: {}", e),
    }

    // 19. GIT_WORKTREE_PRUNE - Prune worktrees
    info!("19. Testing git_worktree_prune");
    match client.call_tool(
        tools::GIT_WORKTREE_PRUNE,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await {
        Ok(result) => info!("Pruned worktrees: {:?}", result),
        Err(e) => error!("Failed to prune worktrees: {}", e),
    }

    // 20. GIT_BRANCH_DELETE - Delete a branch
    info!("20. Testing git_branch_delete");
    match client.call_tool(
        tools::GIT_BRANCH_DELETE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch_name": "feature-renamed"
        })
    ).await {
        Ok(result) => info!("Deleted branch: {:?}", result),
        Err(e) => error!("Failed to delete branch: {}", e),
    }

    // Graceful shutdown
    client.close().await?;
    info!("Git tools example completed successfully");

    Ok(())
}
