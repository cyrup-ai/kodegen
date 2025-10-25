mod common;

use anyhow::Context;
use kodegen_mcp_client::responses::{
    GitHubBranch, GitHubCodeResult, GitHubCommentsResponse, GitHubCommit, GitHubIssue,
    GitHubIssuesResponse, GitHubRepository, GitHubSearchResults, GitHubUser,
};
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting GitHub tools example");

    // Connect to kodegen server with github category
    let (conn, mut server) =
        common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/tmp/mcp-client/github.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with guaranteed cleanup
    let result = run_github_example(&client).await;

    // Always close connection, regardless of example result
    conn.close().await?;
    server.shutdown().await?;

    // Propagate any error from the example AFTER cleanup
    result
}

async fn run_github_example(client: &common::LoggingClient) -> anyhow::Result<()> {
    // =================================================================
    // SEARCH TOOLS (3 tools - read-only, safe)
    // =================================================================

    info!("\n=== Testing Search Tools ===");

    // 1. SEARCH_REPOSITORIES
    info!("1. Testing search_repositories");
    match client
        .call_tool_typed::<GitHubSearchResults<GitHubRepository>>(
            tools::SEARCH_REPOSITORIES,
            json!({
                "query": "language:rust stars:>1000",
                "per_page": 5
            }),
        )
        .await
    {
        Ok(repos) => {
            info!("✅ Found {} repositories", repos.total_count);
            if let Some(repo) = repos.items.first() {
                info!(
                    "   Top result: {} ({} stars)",
                    repo.full_name,
                    repo.stargazers_count.unwrap_or(0)
                );
            }
        }
        Err(e) => error!("❌ search_repositories failed: {}", e),
    }

    // 2. SEARCH_USERS
    info!("2. Testing search_users");
    match client
        .call_tool_typed::<GitHubSearchResults<GitHubUser>>(
            tools::SEARCH_USERS,
            json!({
                "query": "location:tokyo language:rust",
                "per_page": 5
            }),
        )
        .await
    {
        Ok(users) => {
            info!("✅ Found {} users", users.total_count);
            if let Some(user) = users.items.first() {
                info!("   Top result: {}", user.login);
            }
        }
        Err(e) => error!("❌ search_users failed: {}", e),
    }

    // 3. SEARCH_CODE (with star enrichment)
    info!("3. Testing search_code with star enrichment");
    match client
        .call_tool_typed::<GitHubSearchResults<GitHubCodeResult>>(
            tools::SEARCH_CODE,
            json!({
                "query": "tokio language:rust",
                "per_page": 5,
                "enrich_stars": true
            }),
        )
        .await
    {
        Ok(code) => {
            info!("✅ Found {} code results", code.total_count);
            info!("   Showing top 5 results with enriched star counts:");
            for (idx, result) in code.items.iter().enumerate() {
                let stars = result
                    .repository
                    .stargazers_count
                    .map_or_else(|| "? ⭐".to_string(), |s| format!("{s} ⭐"));
                info!(
                    "   {}. {} in {} ({})",
                    idx + 1,
                    result.name,
                    result.repository.full_name,
                    stars
                );
            }
        }
        Err(e) => error!("❌ search_code failed: {}", e),
    }

    // =================================================================
    // REPOSITORY INFO TOOLS (6 tools - read-only on public repos)
    // =================================================================

    info!("\n=== Testing Repository Info Tools ===");

    // 4. LIST_BRANCHES
    info!("4. Testing list_branches");
    match client
        .call_tool_typed::<Vec<GitHubBranch>>(
            tools::LIST_BRANCHES,
            json!({
                "owner": "rust-lang",
                "repo": "rust"
            }),
        )
        .await
    {
        Ok(branches) => {
            info!("✅ Found {} branches", branches.len());
            if let Some(branch) = branches.first() {
                info!("   First branch: {}", branch.name);
            }
        }
        Err(e) => error!("❌ list_branches failed: {}", e),
    }

    // 5. LIST_COMMITS
    info!("5. Testing list_commits");
    let commits_result = client
        .call_tool_typed::<Vec<GitHubCommit>>(
            tools::LIST_COMMITS,
            json!({
                "owner": "rust-lang",
                "repo": "rust",
                "per_page": 5
            }),
        )
        .await;

    match commits_result {
        Ok(commits) => {
            info!("✅ Found {} commits", commits.len());

            if let Some(commit) = commits.first() {
                let message_first_line = commit.commit.message.lines().next().unwrap_or("");
                info!("   Latest commit: {}", message_first_line);

                // 6. GET_COMMIT (using the first commit's SHA)
                info!("6. Testing get_commit");
                match client
                    .call_tool_typed::<GitHubCommit>(
                        tools::GET_COMMIT,
                        json!({
                            "owner": "rust-lang",
                            "repo": "rust",
                            "commit_sha": commit.sha
                        }),
                    )
                    .await
                {
                    Ok(commit_detail) => {
                        let msg = commit_detail.commit.message.lines().next().unwrap_or("");
                        info!("✅ Got commit: {}", msg);
                    }
                    Err(e) => error!("❌ get_commit failed: {}", e),
                }
            }
        }
        Err(e) => error!("❌ list_commits failed: {}", e),
    }

    // =================================================================
    // ISSUE TOOLS (7 tools - read-only)
    // =================================================================

    info!("\n=== Testing Issue Tools ===");

    // 7. LIST_ISSUES
    info!("7. Testing list_issues");
    let issues_result = client
        .call_tool_typed::<GitHubIssuesResponse>(
            tools::LIST_ISSUES,
            json!({
                "owner": "rust-lang",
                "repo": "rustlings",
                "state": "open",
                "per_page": 5
            }),
        )
        .await;

    match issues_result {
        Ok(response) => {
            info!("✅ Found {} issues", response.count);

            if let Some(issue) = response.issues.first() {
                info!("   First issue: #{} - {}", issue.number, issue.title);

                // 8. GET_ISSUE
                info!("8. Testing get_issue");
                match client
                    .call_tool_typed::<GitHubIssue>(
                        tools::GET_ISSUE,
                        json!({
                            "owner": "rust-lang",
                            "repo": "rustlings",
                            "issue_number": issue.number
                        }),
                    )
                    .await
                {
                    Ok(issue_detail) => {
                        info!(
                            "✅ Got issue #{}: {}",
                            issue_detail.number, issue_detail.title
                        );
                    }
                    Err(e) => error!("❌ get_issue failed: {}", e),
                }

                // 9. GET_ISSUE_COMMENTS
                info!("9. Testing get_issue_comments");
                match client
                    .call_tool_typed::<GitHubCommentsResponse>(
                        tools::GET_ISSUE_COMMENTS,
                        json!({
                            "owner": "rust-lang",
                            "repo": "rustlings",
                            "issue_number": issue.number
                        }),
                    )
                    .await
                {
                    Ok(response) => {
                        info!(
                            "✅ Found {} comments on issue #{}",
                            response.count, issue.number
                        );
                    }
                    Err(e) => error!("❌ get_issue_comments failed: {}", e),
                }
            }
        }
        Err(e) => error!("❌ list_issues failed: {}", e),
    }

    // 10. SEARCH_ISSUES
    info!("10. Testing search_issues");
    match client
        .call_tool_typed::<GitHubIssuesResponse>(
            tools::SEARCH_ISSUES,
            json!({
                "query": "repo:rust-lang/rustlings is:open is:issue",
                "per_page": 5
            }),
        )
        .await
    {
        Ok(response) => {
            info!("✅ Found {} matching issues", response.count);
            if let Some(issue) = response.issues.first() {
                info!("   First result: #{} - {}", issue.number, issue.title);
            }
        }
        Err(e) => error!("❌ search_issues failed: {}", e),
    }

    // =================================================================
    // Write operation tools (15) - Logging but skipping execution
    // =================================================================

    info!("\n=== Write Operation Tools (Not Executed) ===");
    info!("The following tools are registered but not tested to avoid creating spam:");
    info!("  11. create_issue");
    info!("  12. update_issue");
    info!("  13. add_issue_comment");
    info!("  14. create_pull_request");
    info!("  15. update_pull_request");
    info!("  16. merge_pull_request");
    info!("  17. get_pull_request_status");
    info!("  18. get_pull_request_files");
    info!("  19. get_pull_request_reviews");
    info!("  20. create_pull_request_review");
    info!("  21. add_pull_request_review_comment");
    info!("  22. request_copilot_review");
    info!("  23. create_repository");
    info!("  24. fork_repository");
    info!("  25. create_branch");

    info!("\n✅ GitHub tools example completed successfully");
    info!("Total tools tested: 10 read-only operations");
    info!("Total tools registered: 25 (15 write operations skipped)");

    Ok(())
}
