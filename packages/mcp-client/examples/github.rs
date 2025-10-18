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

    info!("Starting github tools example");

    // Check for required environment variables
    if std::env::var("GITHUB_TOKEN").is_err() {
        tracing::warn!("⚠️  GITHUB_TOKEN not set. GitHub operations require authentication.");
        tracing::warn!("Get a token at: https://github.com/settings/tokens");
        tracing::warn!("Set it with: export GITHUB_TOKEN=your_token_here");
        tracing::warn!("Required scopes: 'repo' or 'public_repo'");
        info!("⏭️  Skipping example - set token to run this test");
        return Ok(());
    }

    // Connect to kodegen server with github category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Github])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // Use environment-configured repo or default to GitHub's official example repo
    let test_repo = std::env::var("GITHUB_TEST_REPO")
        .unwrap_or_else(|_| {
            tracing::info!("Using default test repository: octocat/Hello-World");
            tracing::info!("Set GITHUB_TEST_REPO to use your own repository");
            "octocat/Hello-World".to_string()
        });

    let test_issue_number = 1;
    let test_pr_number = 1;

    tracing::info!("Testing with repository: {}", test_repo);

    // ISSUE TOOLS (7 tools)

    // 1. CREATE_ISSUE - Create a new issue
    info!("1. Testing create_issue");
    match client.call_tool(
        tools::CREATE_ISSUE,
        json!({
            "repo": test_repo,
            "title": "Test issue",
            "body": "This is a test issue"
        })
    ).await {
        Ok(result) => info!("Created issue: {:?}", result),
        Err(e) => error!("Failed to create issue: {}", e),
    }

    // 2. GET_ISSUE - Get issue details
    info!("2. Testing get_issue");
    match client.call_tool(
        tools::GET_ISSUE,
        json!({
            "repo": test_repo,
            "issue_number": test_issue_number
        })
    ).await {
        Ok(result) => info!("Got issue: {:?}", result),
        Err(e) => error!("Failed to get issue: {}", e),
    }

    // 3. LIST_ISSUES - List repository issues
    info!("3. Testing list_issues");
    match client.call_tool(
        tools::LIST_ISSUES,
        json!({
            "repo": test_repo,
            "state": "open",
            "per_page": 10
        })
    ).await {
        Ok(result) => info!("Listed issues: {:?}", result),
        Err(e) => error!("Failed to list issues: {}", e),
    }

    // 4. UPDATE_ISSUE - Update an issue
    info!("4. Testing update_issue");
    match client.call_tool(
        tools::UPDATE_ISSUE,
        json!({
            "repo": test_repo,
            "issue_number": test_issue_number,
            "title": "Updated test issue"
        })
    ).await {
        Ok(result) => info!("Updated issue: {:?}", result),
        Err(e) => error!("Failed to update issue: {}", e),
    }

    // 5. SEARCH_ISSUES - Search for issues
    info!("5. Testing search_issues");
    match client.call_tool(
        tools::SEARCH_ISSUES,
        json!({
            "query": "is:issue is:open label:bug",
            "per_page": 10
        })
    ).await {
        Ok(result) => info!("Searched issues: {:?}", result),
        Err(e) => error!("Failed to search issues: {}", e),
    }

    // 6. ADD_ISSUE_COMMENT - Add comment to issue
    info!("6. Testing add_issue_comment");
    match client.call_tool(
        tools::ADD_ISSUE_COMMENT,
        json!({
            "repo": test_repo,
            "issue_number": test_issue_number,
            "body": "This is a test comment"
        })
    ).await {
        Ok(result) => info!("Added comment: {:?}", result),
        Err(e) => error!("Failed to add comment: {}", e),
    }

    // 7. GET_ISSUE_COMMENTS - Get issue comments
    info!("7. Testing get_issue_comments");
    match client.call_tool(
        tools::GET_ISSUE_COMMENTS,
        json!({
            "repo": test_repo,
            "issue_number": test_issue_number
        })
    ).await {
        Ok(result) => info!("Got comments: {:?}", result),
        Err(e) => error!("Failed to get comments: {}", e),
    }

    // PULL REQUEST TOOLS (5 tools)

    // 8. CREATE_PULL_REQUEST - Create a PR
    info!("8. Testing create_pull_request");
    match client.call_tool(
        tools::CREATE_PULL_REQUEST,
        json!({
            "repo": test_repo,
            "title": "Test PR",
            "head": "feature-branch",
            "base": "main",
            "body": "This is a test pull request"
        })
    ).await {
        Ok(result) => info!("Created PR: {:?}", result),
        Err(e) => error!("Failed to create PR: {}", e),
    }

    // 9. UPDATE_PULL_REQUEST - Update a PR
    info!("9. Testing update_pull_request");
    match client.call_tool(
        tools::UPDATE_PULL_REQUEST,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number,
            "title": "Updated test PR"
        })
    ).await {
        Ok(result) => info!("Updated PR: {:?}", result),
        Err(e) => error!("Failed to update PR: {}", e),
    }

    // 10. MERGE_PULL_REQUEST - Merge a PR
    info!("10. Testing merge_pull_request");
    match client.call_tool(
        tools::MERGE_PULL_REQUEST,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number,
            "merge_method": "merge"
        })
    ).await {
        Ok(result) => info!("Merged PR: {:?}", result),
        Err(e) => error!("Failed to merge PR: {}", e),
    }

    // 11. GET_PULL_REQUEST_STATUS - Get PR status
    info!("11. Testing get_pull_request_status");
    match client.call_tool(
        tools::GET_PULL_REQUEST_STATUS,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number
        })
    ).await {
        Ok(result) => info!("PR status: {:?}", result),
        Err(e) => error!("Failed to get PR status: {}", e),
    }

    // 12. GET_PULL_REQUEST_FILES - Get PR files
    info!("12. Testing get_pull_request_files");
    match client.call_tool(
        tools::GET_PULL_REQUEST_FILES,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number
        })
    ).await {
        Ok(result) => info!("PR files: {:?}", result),
        Err(e) => error!("Failed to get PR files: {}", e),
    }

    // PR REVIEW TOOLS (4 tools)

    // 13. GET_PULL_REQUEST_REVIEWS - Get PR reviews
    info!("13. Testing get_pull_request_reviews");
    match client.call_tool(
        tools::GET_PULL_REQUEST_REVIEWS,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number
        })
    ).await {
        Ok(result) => info!("PR reviews: {:?}", result),
        Err(e) => error!("Failed to get PR reviews: {}", e),
    }

    // 14. CREATE_PULL_REQUEST_REVIEW - Create a review
    info!("14. Testing create_pull_request_review");
    match client.call_tool(
        tools::CREATE_PULL_REQUEST_REVIEW,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number,
            "event": "COMMENT",
            "body": "This is a review comment"
        })
    ).await {
        Ok(result) => info!("Created review: {:?}", result),
        Err(e) => error!("Failed to create review: {}", e),
    }

    // 15. ADD_PULL_REQUEST_REVIEW_COMMENT - Add review comment
    info!("15. Testing add_pull_request_review_comment");
    match client.call_tool(
        tools::ADD_PULL_REQUEST_REVIEW_COMMENT,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number,
            "body": "Inline review comment",
            "commit_id": "abc123",
            "path": "file.rs",
            "position": 10
        })
    ).await {
        Ok(result) => info!("Added review comment: {:?}", result),
        Err(e) => error!("Failed to add review comment: {}", e),
    }

    // 16. REQUEST_COPILOT_REVIEW - Request Copilot review
    info!("16. Testing request_copilot_review");
    match client.call_tool(
        tools::REQUEST_COPILOT_REVIEW,
        json!({
            "repo": test_repo,
            "pr_number": test_pr_number
        })
    ).await {
        Ok(result) => info!("Requested copilot review: {:?}", result),
        Err(e) => error!("Failed to request copilot review: {}", e),
    }

    // Graceful shutdown
    client.close().await?;
    info!("GitHub tools example completed successfully");

    Ok(())
}
