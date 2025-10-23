mod common;

use anyhow::Context;
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
    let (conn, mut server) = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Git])
    ).await?;

    // Wrap client with logging
    let log_path = std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/git.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with cleanup
    let result = run_git_example(&client).await;

    // Always close connection, regardless of example result
    conn.close().await?;
    server.shutdown().await?;

    // Propagate any error from the example
    result
}

async fn run_git_example(client: &common::LoggingClient) -> anyhow::Result<()> {
    let test_repo = std::env::temp_dir().join("kodegen_git_test");
    let worktree_path = test_repo.parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?
        .join("worktree_test");

    // Run tests
    let test_result = async {
        // 1. GIT_INIT - Initialize a repository
        info!("1. Testing git_init");
    let result = client.call_tool(
        tools::GIT_INIT,
        json!({ "path": test_repo.to_string_lossy() })
    ).await?;
    info!("Initialized repo: {:?}", result);

    // 2. GIT_OPEN - Open the repository
    info!("2. Testing git_open");
    let result = client.call_tool(
        tools::GIT_OPEN,
        json!({ "path": test_repo.to_string_lossy() })
    ).await?;
    info!("Opened repo: {:?}", result);

    // 3. GIT_DISCOVER - Discover git repository from path
    info!("3. Testing git_discover");
    let result = client.call_tool(
        tools::GIT_DISCOVER,
        json!({ "path": test_repo.to_string_lossy() })
    ).await?;
    info!("Discovered repo: {:?}", result);

    // 4. GIT_BRANCH_LIST - List branches
    info!("4. Testing git_branch_list");
    let result = client.call_tool(
        tools::GIT_BRANCH_LIST,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await?;
    info!("Branch list: {:?}", result);

    // 5. GIT_BRANCH_CREATE - Create a new branch
    info!("5. Testing git_branch_create");
    let result = client.call_tool(
        tools::GIT_BRANCH_CREATE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch_name": "feature-test"
        })
    ).await?;
    info!("Created branch: {:?}", result);

    // 6. GIT_BRANCH_RENAME - Rename a branch
    info!("6. Testing git_branch_rename");
    let result = client.call_tool(
        tools::GIT_BRANCH_RENAME,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "old_name": "feature-test",
            "new_name": "feature-renamed"
        })
    ).await?;
    info!("Renamed branch: {:?}", result);

    // 7. GIT_CHECKOUT - Checkout a branch
    info!("7. Testing git_checkout");
    let result = client.call_tool(
        tools::GIT_CHECKOUT,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch": "feature-renamed"
        })
    ).await?;
    info!("Checked out: {:?}", result);

    // 8. GIT_ADD - Add files to staging
    info!("8. Testing git_add");
    let result = client.call_tool(
        tools::GIT_ADD,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "paths": ["."]
        })
    ).await?;
    info!("Added files: {:?}", result);

    // 9. GIT_COMMIT - Commit changes
    info!("9. Testing git_commit");
    let result = client.call_tool(
        tools::GIT_COMMIT,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "message": "Initial commit"
        })
    ).await?;
    info!("Committed: {:?}", result);

    // 10. GIT_LOG - View commit log
    info!("10. Testing git_log");
    let result = client.call_tool(
        tools::GIT_LOG,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "max_count": 10
        })
    ).await?;
    info!("Log: {:?}", result);

    // 11. GIT_FETCH - Fetch from remote
    info!("11. Testing git_fetch");
    match client.call_tool(
        tools::GIT_FETCH,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "remote": "origin"
        })
    ).await {
        Ok(result) => info!("✅ Fetched from remote: {:?}", result),
        Err(e) if e.to_string().contains("remote") || e.to_string().contains("origin") => {
            info!("⏭️  Skipping remote operations (no remote configured)");
            info!("Git tools example completed (remote tests skipped)");
            return Ok(());
        }
        Err(e) => return Err(e).context("Unexpected git_fetch error"),
    }

    // 12. GIT_MERGE - Merge branches
    info!("12. Testing git_merge");
    let result = client.call_tool(
        tools::GIT_MERGE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch": "main"
        })
    ).await?;
    info!("Merged: {:?}", result);

    // 13. GIT_CLONE - Clone repository (demo with public repo)
    info!("13. Testing git_clone (skipped - requires network)");

    // 14. GIT_WORKTREE_ADD - Add worktree
    info!("14. Testing git_worktree_add");
    let result = client.call_tool(
        tools::GIT_WORKTREE_ADD,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy(),
            "branch": "feature-renamed"
        })
    ).await?;
    info!("Added worktree: {:?}", result);

    // 15. GIT_WORKTREE_LIST - List worktrees
    info!("15. Testing git_worktree_list");
    let result = client.call_tool(
        tools::GIT_WORKTREE_LIST,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await?;
    info!("Worktrees: {:?}", result);

    // 16. GIT_WORKTREE_LOCK - Lock a worktree
    info!("16. Testing git_worktree_lock");
    let result = client.call_tool(
        tools::GIT_WORKTREE_LOCK,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await?;
    info!("Locked worktree: {:?}", result);

    // 17. GIT_WORKTREE_UNLOCK - Unlock a worktree
    info!("17. Testing git_worktree_unlock");
    let result = client.call_tool(
        tools::GIT_WORKTREE_UNLOCK,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await?;
    info!("Unlocked worktree: {:?}", result);

    // 18. GIT_WORKTREE_REMOVE - Remove a worktree
    info!("18. Testing git_worktree_remove");
    let result = client.call_tool(
        tools::GIT_WORKTREE_REMOVE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "path": worktree_path.to_string_lossy()
        })
    ).await?;
    info!("Removed worktree: {:?}", result);

    // 19. GIT_WORKTREE_PRUNE - Prune worktrees
    info!("19. Testing git_worktree_prune");
    let result = client.call_tool(
        tools::GIT_WORKTREE_PRUNE,
        json!({ "repo_path": test_repo.to_string_lossy() })
    ).await?;
    info!("Pruned worktrees: {:?}", result);

    // 20. GIT_BRANCH_DELETE - Delete a branch
    info!("20. Testing git_branch_delete");
    let result = client.call_tool(
        tools::GIT_BRANCH_DELETE,
        json!({
            "repo_path": test_repo.to_string_lossy(),
            "branch_name": "feature-renamed"
        })
    ).await?;
    info!("Deleted branch: {:?}", result);

    info!("Git tools example tests completed");
    Ok::<(), anyhow::Error>(())
    }.await;

    // Always cleanup test repositories, regardless of test result
    cleanup_git_repositories(&test_repo, &worktree_path).await;

    // Propagate test result
    test_result
}

async fn cleanup_git_repositories(test_repo: &std::path::Path, worktree_path: &std::path::Path) {
    info!("\nCleaning up test repositories...");
    
    // Clean up test repository
    if let Err(e) = std::fs::remove_dir_all(test_repo) {
        error!("⚠️  Failed to remove test repository {}: {}", test_repo.display(), e);
        error!("   Manual cleanup required: rm -rf {}", test_repo.display());
    } else {
        info!("✅ Cleaned up test repository: {}", test_repo.display());
    }
    
    // Clean up worktree if it exists
    if worktree_path.exists() {
        if let Err(e) = std::fs::remove_dir_all(worktree_path) {
            error!("⚠️  Failed to remove worktree {}: {}", worktree_path.display(), e);
            error!("   Manual cleanup required: rm -rf {}", worktree_path.display());
        } else {
            info!("✅ Cleaned up worktree: {}", worktree_path.display());
        }
    }
}
