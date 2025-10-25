//! Static tool metadata for stdio server proxy.
//!
//! This module contains hardcoded metadata for all 107 tools across 13 categories.
//! The metadata is extracted from source files and hardcoded here to avoid instantiating
//! actual tool objects, reducing binary size from ~15MB to ~1MB.
//!
//! Each tool's schema is generated from kodegen_mcp_schema Args types, not from
//! the actual tool implementations in tools-* packages.

use kodegen_mcp_schema::*;
use rmcp::schemars::{schema_for, JsonSchema};
use serde_json::Value;
use std::collections::HashMap;

/// Metadata for a single tool.
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub schema: Value,
    pub read_only: bool,
}

/// Helper to build schema from Args type.
fn build_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap_or(Value::Null)
}

/// Port assignments for category SSE servers (from category server main.rs files).
pub const CATEGORY_PORTS: &[(&str, u16)] = &[
    ("browser", 30440),
    ("citescrape", 30441),
    ("claude-agent", 30439),
    ("config", 30442),
    ("database", 30443),
    ("filesystem", 30444),
    ("git", 30445),
    ("github", 30446),
    ("introspection", 30447),
    ("process", 30448),
    ("prompt", 30438),
    ("sequential-thinking", 30437),
    ("terminal", 30449),
];

/// All 107 tools with static metadata.
pub fn all_tool_metadata() -> Vec<ToolMetadata> {
    vec![
        // BROWSER (10 tools)
        ToolMetadata {
            name: "browser_agent",
            category: "browser",
            description: "Autonomous browser agent that executes multi-step tasks using AI reasoning.\\n\\n\
         The agent can navigate websites, interact with forms, e...",
            schema: build_schema::<browser::BrowserAgentArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_click",
            category: "browser",
            description: "Click an element on the page using a CSS selector.\\n\\n\
         Automatically scrolls element into view before clicking.\\n\\n\
         Example...",
            schema: build_schema::<browser::BrowserClickArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_extract_text",
            category: "browser",
            description: "Extract text content from the page or specific element.\\n\\n\
         Returns the text content for AI agent analysis.\\n\\n\
         Example: br...",
            schema: build_schema::<browser::BrowserExtractTextArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_navigate",
            category: "browser",
            description: "Navigate to a URL in the browser. Opens the page and waits for load completion.\\n\\n\
         Returns current URL after navigation (may differ fr...",
            schema: build_schema::<browser::BrowserNavigateArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_research",
            category: "browser",
            description: "Deep research tool that searches the web, crawls multiple pages, and generates AI-powered summaries.\n\n\
         Automatically extracts key findi...",
            schema: build_schema::<browser::BrowserResearchArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_screenshot",
            category: "browser",
            description: "Take a screenshot of the current page or specific element. Returns base64-encoded image.\\n\\n\
         Example: browser_screenshot({}) for full p...",
            schema: build_schema::<browser::BrowserScreenshotArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_scroll",
            category: "browser",
            description: "Scroll the page by amount or to a specific element.\\n\\n\
         Examples:\\n\
         - browser_scroll({\",
            schema: build_schema::<browser::BrowserScrollArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_type_text",
            category: "browser",
            description: "Type text into an input element using a CSS selector.\\n\\n\
         Automatically focuses element and clears existing text by default.\\n\\n\
   ...",
            schema: build_schema::<browser::BrowserTypeTextArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_wait",
            category: "browser",
            description: "Wait for a specified duration (useful for waiting for dynamic content to load).\\n\\n\
         Example: browser_wait({\",
            schema: build_schema::<browser::BrowserWaitArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "browser_wait_for",
            category: "browser",
            description: "Wait for an element to meet a specific condition before proceeding.\\n\\n\
         Supports multiple wait conditions:\\n\
         - present: Elem...",
            schema: build_schema::<browser::BrowserWaitForArgs>(),
            read_only: false,
        },
        // CITESCRAPE (4 tools)
        ToolMetadata {
            name: "get_crawl_results",
            category: "citescrape",
            description: "Check crawl status and retrieve results for active or completed crawls. \
         Returns progress information for running crawls and summary with...",
            schema: build_schema::<citescrape::GetCrawlResultsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "search_crawl_results",
            category: "citescrape",
            description: "Full-text search across crawled documentation using Tantivy. Supports advanced \
         query syntax including text, phrase, boolean, field-speci...",
            schema: build_schema::<citescrape::SearchCrawlResultsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "start_crawl",
            category: "citescrape",
            description: "Start a background web crawl that saves content to markdown/HTML/JSON \
         and optionally indexes for full-text search. Returns immediately w...",
            schema: build_schema::<citescrape::StartCrawlArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "web_search",
            category: "citescrape",
            description: "Perform a web search using DuckDuckGo and return structured results with titles, URLs, and snippets.\\n\\n\
         Returns up to 10 search result...",
            schema: build_schema::<citescrape::WebSearchArgs>(),
            read_only: true,
        },
        // CLAUDE-AGENT (5 tools)
        ToolMetadata {
            name: "list_claude_agents",
            category: "claude-agent",
            description: "List all active and completed agent sessions with status and output preview. \
         Shows working indicator (true if actively processing), turn...",
            schema: build_schema::<claude_agent::ListClaudeAgentsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "read_claude_agent_output",
            category: "claude-agent",
            description: "Read paginated output from an agent session. Returns messages with working indicator. \
         Use offset/length for pagination (offset=0 for sta...",
            schema: build_schema::<claude_agent::ReadClaudeAgentOutputArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "send_claude_agent_prompt",
            category: "claude-agent",
            description: "Send a follow-up prompt to an active agent session. Continues the conversation \
         with new instructions or questions. Use read_claude_agent...",
            schema: build_schema::<claude_agent::SendClaudeAgentPromptArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "spawn_claude_agent",
            category: "claude-agent",
            description: "Spawn one or more Claude agent sessions for parallel task delegation. \
         Each agent gets identical configuration and can work independently...",
            schema: build_schema::<claude_agent::SpawnClaudeAgentArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "terminate_claude_agent_session",
            category: "claude-agent",
            description: "Gracefully terminate an agent session. Closes the ClaudeSDKClient connection, \
         returns final statistics (turn count, message count, runti...",
            schema: build_schema::<claude_agent::TerminateClaudeAgentSessionArgs>(),
            read_only: false,
        },
        // CONFIG (2 tools)
        ToolMetadata {
            name: "get_config",
            category: "config",
            description: "Get complete server configuration including security settings (blocked commands, \
         allowed directories), shell preferences, resource limit...",
            schema: build_schema::<config::GetConfigArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "set_config_value",
            category: "config",
            description: "Set a specific configuration value by key.\n\n\
         WARNING: Should be used in a separate chat from file operations and \n\
         command e...",
            schema: build_schema::<config::SetConfigValueArgs>(),
            read_only: false,
        },
        // DATABASE (7 tools)
        ToolMetadata {
            name: "execute_sql",
            category: "database",
            description: "Execute SQL query or multiple SQL statements (separated by semicolons). \
         \n\n\
         MULTI-STATEMENT BEHAVIOR:\n\
         - Write ope...",
            schema: build_schema::<database::ExecuteSQLArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_pool_stats",
            category: "database",
            description: "Get connection pool health metrics including active connections, \
         idle connections, and pool configuration. Use this to diagnose \
      ...",
            schema: build_schema::<database::GetPoolStatsArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_stored_procedures",
            category: "database",
            description: "List stored procedures in a schema. Returns procedure names and optionally \
         detailed information including parameters and definitions. \
...",
            schema: build_schema::<database::GetStoredProceduresArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_table_indexes",
            category: "database",
            description: "Get index information for a table including index names, columns, uniqueness, \
         and primary key status. Use this to understand which colum...",
            schema: build_schema::<database::GetTableIndexesArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_table_schema",
            category: "database",
            description: "Get column information for a table including column names, data types, \
         nullability, and default values. Use this before writing queries ...",
            schema: build_schema::<database::GetTableSchemaArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "list_schemas",
            category: "database",
            description: "List all schemas (databases) in the current database connection. \
         For PostgreSQL, returns all user schemas (excludes pg_catalog, informat...",
            schema: build_schema::<database::ListSchemasArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_tables",
            category: "database",
            description: "List all tables in a schema. If schema not provided, uses default schema \
         (public for PostgreSQL, current database for MySQL, main for SQ...",
            schema: build_schema::<database::ListTablesArgs>(),
            read_only: true,
        },
        // FILESYSTEM (14 tools)
        ToolMetadata {
            name: "create_directory",
            category: "filesystem",
            description: "Create a new directory or ensure a directory exists. Can create multiple nested \
         directories in one operation. Automatically validates pa...",
            schema: build_schema::<filesystem::CreateDirectoryArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "delete_directory",
            category: "filesystem",
            description: "Delete a directory and all its contents recursively. This operation is permanent and \
         cannot be undone. Requires recursive=true to confir...",
            schema: build_schema::<filesystem::DeleteDirectoryArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "delete_file",
            category: "filesystem",
            description: "Delete a file from the filesystem. This operation is permanent and cannot be undone. \
         Only deletes files, not directories. Automatically ...",
            schema: build_schema::<filesystem::DeleteFileArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "edit_block",
            category: "filesystem",
            description: "Apply surgical text replacements to files. Takes old_string and new_string, and performs \
         exact string replacement. By default replaces o...",
            schema: build_schema::<filesystem::EditBlockArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_file_info",
            category: "filesystem",
            description: "Retrieve detailed metadata about a file or directory including size, creation time, \
         last modified time, permissions, type, and line coun...",
            schema: build_schema::<filesystem::GetFileInfoArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_more_search_results",
            category: "filesystem",
            description: "Get more results from an active search with offset-based pagination.\n\n\
         Supports partial result reading with:\n\
         - 'offset' (st...",
            schema: build_schema::<filesystem::GetMoreSearchResultsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_directory",
            category: "filesystem",
            description: "List all files and directories in a specified path. Returns entries prefixed with \
         [DIR] or [FILE] to distinguish types. Supports filteri...",
            schema: build_schema::<filesystem::ListDirectoryArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_searches",
            category: "filesystem",
            description: "List all active searches.\n\n\
         Shows search IDs, search types, patterns, status, and runtime.\n\
         Similar to list_sessions for ter...",
            schema: build_schema::<filesystem::ListSearchesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "move_file",
            category: "filesystem",
            description: "Move or rename files and directories. Can move files between directories and rename \
         them in a single operation. Both source and destinat...",
            schema: build_schema::<filesystem::MoveFileArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "read_file",
            category: "filesystem",
            description: "Read the contents of a file from the filesystem or a URL. Supports text files (returned as text) \
         and image files (returned as base64). U...",
            schema: build_schema::<filesystem::ReadFileArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "read_multiple_files",
            category: "filesystem",
            description: "Read multiple files in parallel. Returns results for all files, including errors for \
         individual files that fail. Supports offset and len...",
            schema: build_schema::<filesystem::ReadMultipleFilesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "start_search",
            category: "filesystem",
            description: "Start a streaming search that can return results progressively.\n\n\
         SEARCH STRATEGY GUIDE:\n\
         Choose the right search type based...",
            schema: build_schema::<filesystem::StartSearchArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "stop_search",
            category: "filesystem",
            description: "Stop an active search session.\n\n\
         Stops the background search process gracefully. Use this when you've found \
         what you need or...",
            schema: build_schema::<filesystem::StopSearchArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "write_file",
            category: "filesystem",
            description: "Write or append to file contents. Supports two modes: 'rewrite' (overwrite entire file) \
         and 'append' (add to end of file). Automatically...",
            schema: build_schema::<filesystem::WriteFileArgs>(),
            read_only: false,
        },
        // GIT (20 tools)
        ToolMetadata {
            name: "git_add",
            category: "git",
            description: "Stage file changes for commit in a Git repository. \
         Specify paths to stage specific files.",
            schema: build_schema::<git::GitAddArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_branch_create",
            category: "git",
            description: "Create a new branch in a Git repository. \
         Optionally specify a starting point and checkout the branch after creation.",
            schema: build_schema::<git::GitBranchCreateArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_branch_delete",
            category: "git",
            description: "Delete a branch from a Git repository. \
         Cannot delete the currently checked-out branch.",
            schema: build_schema::<git::GitBranchDeleteArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_branch_list",
            category: "git",
            description: "List all local branches in a Git repository.",
            schema: build_schema::<git::GitBranchListArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_branch_rename",
            category: "git",
            description: "Rename a branch in a Git repository. \
         Automatically updates HEAD if renaming the current branch.",
            schema: build_schema::<git::GitBranchRenameArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_checkout",
            category: "git",
            description: "Checkout a Git reference (branch, tag, or commit) or restore specific files. \
         Without paths: switches branches/commits. With paths: resto...",
            schema: build_schema::<git::GitCheckoutArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_clone",
            category: "git",
            description: "Clone a remote Git repository to a local path. \
         Supports shallow cloning (limited history) and branch-specific cloning. \
         The de...",
            schema: build_schema::<git::GitCloneArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_commit",
            category: "git",
            description: "Create a new commit in a Git repository. \
         Optionally specify author information and stage all modified files.",
            schema: build_schema::<git::GitCommitArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_discover",
            category: "git",
            description: "Discover a Git repository by searching upward from the given path. \
         This will traverse parent directories until it finds a .git directory...",
            schema: build_schema::<git::GitDiscoverArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_fetch",
            category: "git",
            description: "Fetch updates from a remote repository. \
         Downloads objects and refs from another repository.",
            schema: build_schema::<git::GitFetchArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_init",
            category: "git",
            description: "Initialize a new Git repository at the specified path. \
         Supports both normal repositories (with working directory) and \
         bare re...",
            schema: build_schema::<git::GitInitArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_log",
            category: "git",
            description: "List commit history from a Git repository. \
         Optionally filter by file path and limit the number of results.",
            schema: build_schema::<git::GitLogArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_merge",
            category: "git",
            description: "Merge a branch or commit into the current branch. \
         Joins two or more development histories together.",
            schema: build_schema::<git::GitMergeArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_open",
            category: "git",
            description: "Open an existing Git repository at the specified path. \
         The repository must already exist at the given location.",
            schema: build_schema::<git::GitOpenArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_add",
            category: "git",
            description: "Create a new worktree linked to the repository. \
         Allows working on multiple branches simultaneously.",
            schema: build_schema::<git::GitWorktreeAddArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_list",
            category: "git",
            description: "List all worktrees in the repository with detailed status. \
         Returns main worktree and all linked worktrees with their paths, branches, \
...",
            schema: build_schema::<git::GitWorktreeListArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_lock",
            category: "git",
            description: "Lock a worktree to prevent deletion. \
         Useful for worktrees on removable media or network drives.",
            schema: build_schema::<git::GitWorktreeLockArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_prune",
            category: "git",
            description: "Remove stale worktree administrative files. \
         Cleans up .git/worktrees/ entries for worktrees whose directories have been manually deleted...",
            schema: build_schema::<git::GitWorktreePruneArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_remove",
            category: "git",
            description: "Remove a worktree and its associated administrative files. \
         Cannot remove locked worktrees without force flag.",
            schema: build_schema::<git::GitWorktreeRemoveArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "git_worktree_unlock",
            category: "git",
            description: "Unlock a locked worktree. \
         Removes the lock that prevents worktree deletion.",
            schema: build_schema::<git::GitWorktreeUnlockArgs>(),
            read_only: false,
        },
        // GITHUB (31 tools)
        ToolMetadata {
            name: "add_issue_comment",
            category: "github",
            description: "Add a comment to an existing GitHub issue. Supports Markdown formatting in the comment body. \
         Requires GITHUB_TOKEN environment variable ...",
            schema: build_schema::<github::AddIssueCommentArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "add_pull_request_review_comment",
            category: "github",
            description: "Add an inline review comment to a pull request (comment on specific lines of code). \
         Supports single-line, multi-line, and threaded comme...",
            schema: build_schema::<github::AddPullRequestReviewCommentArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "code_scanning_alerts",
            category: "github",
            description: "List code scanning security alerts for a GitHub repository. Returns alerts \
         with details about vulnerabilities, their severity, location,...",
            schema: build_schema::<github::CodeScanningAlertsArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_branch",
            category: "github",
            description: "Create a new branch from a commit SHA",
            schema: build_schema::<github::CreateBranchArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_issue",
            category: "github",
            description: "Create a new issue in a GitHub repository. Supports setting title, body, \
         labels, and assignees. Requires GITHUB_TOKEN environment variab...",
            schema: build_schema::<github::CreateIssueArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_or_update_file",
            category: "github",
            description: "Create a new file or update an existing file in a GitHub repository",
            schema: build_schema::<github::CreateOrUpdateFileArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_pull_request",
            category: "github",
            description: "Create a new pull request in a GitHub repository",
            schema: build_schema::<github::CreatePullRequestArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_pull_request_review",
            category: "github",
            description: "Create a review on a pull request (approve, request changes, or comment). \
         Requires GITHUB_TOKEN environment variable with repo permissions.",
            schema: build_schema::<github::CreatePullRequestReviewArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "create_repository",
            category: "github",
            description: "Create a new repository under the authenticated user's account",
            schema: build_schema::<github::CreateRepositoryArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "fork_repository",
            category: "github",
            description: "Fork a repository to your account or an organization",
            schema: build_schema::<github::ForkRepositoryArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_commit",
            category: "github",
            description: "Get detailed information about a specific commit",
            schema: build_schema::<github::GetCommitArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_file_contents",
            category: "github",
            description: "Get file or directory contents from a GitHub repository",
            schema: build_schema::<github::GetFileContentsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_issue",
            category: "github",
            description: "Fetch a single GitHub issue by number. Returns detailed issue information including \
         title, body, state, labels, assignees, comments coun...",
            schema: build_schema::<github::GetIssueArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_issue_comments",
            category: "github",
            description: "Fetch all comments for a GitHub issue. Returns an array of comment objects \
         including author, body, timestamps, and metadata. Comments ar...",
            schema: build_schema::<github::GetIssueCommentsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_me",
            category: "github",
            description: "Get information about the authenticated GitHub user. Returns user profile \
         details including login, name, email, avatar, bio, company, lo...",
            schema: build_schema::<github::GetMeArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_pull_request_files",
            category: "github",
            description: "Get all files changed in a pull request with their diff stats",
            schema: build_schema::<github::GetPullRequestFilesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_pull_request_reviews",
            category: "github",
            description: "Get all reviews for a pull request. Shows approval status, requested changes, \
         and comments from reviewers. Requires GITHUB_TOKEN environ...",
            schema: build_schema::<github::GetPullRequestReviewsArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_pull_request_status",
            category: "github",
            description: "Get detailed status information about a pull request including merge status, checks, and review state",
            schema: build_schema::<github::GetPullRequestStatusArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_branches",
            category: "github",
            description: "List all branches in a repository",
            schema: build_schema::<github::ListBranchesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_commits",
            category: "github",
            description: "List commits in a repository with filtering options",
            schema: build_schema::<github::ListCommitsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "list_issues",
            category: "github",
            description: "List and filter issues in a GitHub repository. Supports filtering by state, labels, \
         assignee, and pagination. Returns an array of issue ...",
            schema: build_schema::<github::ListIssuesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "merge_pull_request",
            category: "github",
            description: "Merge a pull request in a GitHub repository",
            schema: build_schema::<github::MergePullRequestArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "push_files",
            category: "github",
            description: "Push multiple files to a GitHub repository in a single commit. All files \
         are added atomically (creates tree, commit, and updates ref). F...",
            schema: build_schema::<github::PushFilesArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "request_copilot_review",
            category: "github",
            description: "Request GitHub Copilot to review a pull request (experimental feature). \
         Triggers automated code review from Copilot. Requires GITHUB_TOK...",
            schema: build_schema::<github::RequestCopilotReviewArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "search_code",
            category: "github",
            description: "Search code across GitHub repositories using GitHub's code search syntax",
            schema: build_schema::<github::SearchCodeArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "search_issues",
            category: "github",
            description: "Search for issues across GitHub using GitHub's powerful search syntax. \
         Supports filtering by repository, state, labels, assignee, author...",
            schema: build_schema::<github::SearchIssuesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "search_repositories",
            category: "github",
            description: "Search GitHub repositories using GitHub's repository search syntax",
            schema: build_schema::<github::SearchRepositoriesArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "search_users",
            category: "github",
            description: "Search GitHub users using GitHub's user search syntax",
            schema: build_schema::<github::SearchUsersArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "secret_scanning_alerts",
            category: "github",
            description: "List secret scanning alerts (leaked credentials) for a GitHub repository. \
         Returns alerts about exposed secrets like API keys, tokens, pa...",
            schema: build_schema::<github::SecretScanningAlertsArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "update_issue",
            category: "github",
            description: "Update an existing GitHub issue. Supports partial updates - only specified fields \
         will be modified. Can update title, body, state (open/...",
            schema: build_schema::<github::UpdateIssueArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "update_pull_request",
            category: "github",
            description: "Update an existing pull request in a GitHub repository",
            schema: build_schema::<github::UpdatePullRequestArgs>(),
            read_only: false,
        },
        // INTROSPECTION (2 tools)
        ToolMetadata {
            name: "get_recent_tool_calls",
            category: "introspection",
            description: "Get recent tool call history with their arguments and outputs. \
         Returns chronological list of tool calls made during this session. \
    ...",
            schema: build_schema::<introspection::GetRecentArgsCallsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "get_usage_stats",
            category: "introspection",
            description: "Get usage statistics for debugging and analysis. Returns summary of tool usage, \
         success/failure rates, and performance metrics.",
            schema: build_schema::<introspection::GetUsageStatsArgs>(),
            read_only: true,
        },
        // PROCESS (2 tools)
        ToolMetadata {
            name: "kill_process",
            category: "process",
            description: "Terminate a running process by its PID. Sends SIGKILL signal to forcefully stop the \
         process. Use with caution as this does not allow gra...",
            schema: build_schema::<process::KillProcessArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "list_processes",
            category: "process",
            description: "List all running processes with PID, command name, CPU usage, and memory usage. \
         Supports filtering by process name and limiting results....",
            schema: build_schema::<process::ListProcessesArgs>(),
            read_only: true,
        },
        // PROMPT (4 tools)
        ToolMetadata {
            name: "add_prompt",
            category: "prompt",
            description: "Create a new prompt template. The content must include YAML frontmatter with metadata \
         (title, description, categories, author) followed ...",
            schema: build_schema::<prompt::AddPromptArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "delete_prompt",
            category: "prompt",
            description: "Delete a prompt template. Requires confirm=true for safety. This action cannot be undone. \
         Default prompts can be deleted but will be rec...",
            schema: build_schema::<prompt::DeletePromptArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "edit_prompt",
            category: "prompt",
            description: "Edit an existing prompt template. Provide the prompt name and complete new content \
         (including YAML frontmatter). The content is validate...",
            schema: build_schema::<prompt::EditPromptArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "get_prompt",
            category: "prompt",
            description: "Browse and retrieve prompt templates. \n\n\
         Actions:\n\
         - list_categories: Show all prompt categories\n\
         - list_prompts:...",
            schema: build_schema::<prompt::GetPromptArgs>(),
            read_only: true,
        },
        // SEQUENTIAL-THINKING (1 tools)
        ToolMetadata {
            name: "sequential_thinking",
            category: "sequential-thinking",
            description: "A detailed tool for dynamic and reflective problem-solving through thoughts.\n\
         This tool helps analyze problems through a flexible thinki...",
            schema: build_schema::<sequential_thinking::SequentialThinkingArgs>(),
            read_only: false,
        },
        // TERMINAL (5 tools)
        ToolMetadata {
            name: "list_terminal_commands",
            category: "terminal",
            description: "List all active command sessions. Returns array of sessions with PID, blocked status, \
         and runtime. Use this to monitor all running comma...",
            schema: build_schema::<terminal::ListTerminalCommandsArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "read_terminal_output",
            category: "terminal",
            description: "Get output from a PTY terminal session with offset-based pagination.\n\n\
         Supports partial output reading from VT100 screen buffer:\n\
   ...",
            schema: build_schema::<terminal::ReadTerminalOutputArgs>(),
            read_only: true,
        },
        ToolMetadata {
            name: "send_terminal_input",
            category: "terminal",
            description: "Send input text to a running PTY terminal process. Perfect for interacting with REPLs \
         (Python, Node.js, etc.), interactive programs (vim...",
            schema: build_schema::<terminal::SendTerminalInputArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "start_terminal_command",
            category: "terminal",
            description: "Execute a shell command with full terminal emulation. Supports long-running commands, \
         output streaming, and session management. Returns ...",
            schema: build_schema::<terminal::StartTerminalCommandArgs>(),
            read_only: false,
        },
        ToolMetadata {
            name: "stop_terminal_command",
            category: "terminal",
            description: "Force terminate a running command session by PID. Attempts graceful termination first \
         (SIGTERM), then force kills after 1 second if stil...",
            schema: build_schema::<terminal::StopTerminalCommandArgs>(),
            read_only: false,
        },
    ]
}

/// Build routing table: tool_name -> (category, port)
pub fn build_routing_table() -> HashMap<&'static str, (&'static str, u16)> {
    let mut table = HashMap::new();
    let port_map: HashMap<&str, u16> = CATEGORY_PORTS.iter().copied().collect();
    
    for tool in all_tool_metadata() {
        if let Some(&port) = port_map.get(tool.category) {
            table.insert(tool.name, (tool.category, port));
        }
    }
    
    table
}
