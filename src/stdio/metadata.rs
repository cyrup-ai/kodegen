//! Static tool metadata for stdio server proxy.
//!
//! This module contains hardcoded metadata for all 109 tools across 14 categories.
//! Metadata is extracted from source files to avoid instantiating tool objects.

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
}

/// Helper to build schema from Args type.
fn build_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap_or(Value::Null)
}

/// Port assignments for category HTTP servers (matches daemon config.rs allocation).
pub const CATEGORY_PORTS: &[(&str, u16)] = &[
    ("browser", 30438),
    ("candle_agent", 30452),
    ("citescrape", 30439),
    ("claude_agent", 30440),
    ("config", 30441),
    ("database", 30442),
    ("filesystem", 30443),
    ("git", 30444),
    ("github", 30445),
    ("introspection", 30446),
    ("process", 30447),
    ("prompt", 30448),
    ("reasoner", 30449),
    ("sequential_thinking", 30450),
    ("terminal", 30451),
];

/// All 109 tools with static metadata.
pub fn all_tool_metadata() -> Vec<ToolMetadata> {
    vec![
        // BROWSER (13 tools)
        ToolMetadata {
            name: "browser_agent",
            category: "browser",
            description: "Autonomous browser agent that executes multi-step tasks using AI reasoning.nn The agent can navigate websites, interact with forms, extract informa...",
            schema: build_schema::<browser::BrowserAgentArgs>(),
        },
        ToolMetadata {
            name: "browser_click",
            category: "browser",
            description: "Click an element on the page using a CSS selector.nn Automatically scrolls element into view before clicking.nn Example: browser_click({'selector':...",
            schema: build_schema::<browser::BrowserClickArgs>(),
        },
        ToolMetadata {
            name: "browser_extract_text",
            category: "browser",
            description: "Extract text content from the page or specific element.nn Returns the text content for AI agent analysis.nn Example: browser_extract_text({}) - Ful...",
            schema: build_schema::<browser::BrowserExtractTextArgs>(),
        },
        ToolMetadata {
            name: "browser_navigate",
            category: "browser",
            description: "Navigate to a URL in the browser. Opens the page and waits for load completion.nn Returns current URL after navigation (may differ from requested U...",
            schema: build_schema::<browser::BrowserNavigateArgs>(),
        },

        ToolMetadata {
            name: "browser_screenshot",
            category: "browser",
            description: "Take a screenshot of the current page or specific element. Returns base64-encoded image.nn Example: browser_screenshot({}) for full pagen Example: ...",
            schema: build_schema::<browser::BrowserScreenshotArgs>(),
        },
        ToolMetadata {
            name: "browser_scroll",
            category: "browser",
            description: "Scroll the page by amount or to a specific element.nn Examples:n - browser_scroll({'y': 500}) - Scroll down 500pxn - browser_scroll({'selector': '#...",
            schema: build_schema::<browser::BrowserScrollArgs>(),
        },
        ToolMetadata {
            name: "browser_type_text",
            category: "browser",
            description: "Type text into an input element using a CSS selector.nn Automatically focuses element and clears existing text by default.nn Example: browser_type_...",
            schema: build_schema::<browser::BrowserTypeTextArgs>(),
        },
        ToolMetadata {
            name: "browser_wait",
            category: "browser",
            description: "Wait for a specified duration (useful for waiting for dynamic content to load).nn Example: browser_wait({'duration_ms': 2000}) - Wait 2 seconds' } ...",
            schema: build_schema::<browser::BrowserWaitArgs>(),
        },
        ToolMetadata {
            name: "get_research_result",
            category: "browser",
            description: "Get final results from a completed browser research session.nn Returns comprehensive summary, sources, key findings, and individual page results.nn ...",
            schema: build_schema::<browser::GetResearchResultArgs>(),
        },
        ToolMetadata {
            name: "get_research_status",
            category: "browser",
            description: "Get current status and progress of a browser research session.nn Returns status (running/completed/failed/cancelled), runtime, pages visited, and ...",
            schema: build_schema::<browser::GetResearchStatusArgs>(),
        },
        ToolMetadata {
            name: "list_research_sessions",
            category: "browser",
            description: "List all active browser research sessions.nn Shows session ID, query, status, runtime, and progress for each session.nn Useful for tracking multiple...",
            schema: build_schema::<browser::ListResearchSessionsArgs>(),
        },
        ToolMetadata {
            name: "start_browser_research",
            category: "browser",
            description: "Start async browser research session that runs in background.nn Searches web, crawls multiple pages, and generates AI summaries without blocking.nn...",
            schema: build_schema::<browser::StartBrowserResearchArgs>(),
        },
        ToolMetadata {
            name: "stop_browser_research",
            category: "browser",
            description: "Cancel a running browser research session.nn Aborts the background research task and marks session as cancelled.nn Does nothing if research is alr...",
            schema: build_schema::<browser::StopBrowserResearchArgs>(),
        },
        // CITESCRAPE (4 tools)
        ToolMetadata {
            name: "get_crawl_results",
            category: "citescrape",
            description: "Check crawl status and retrieve results for active or completed crawls. Returns progress information for running crawls and summary with file list ...",
            schema: build_schema::<citescrape::GetCrawlResultsArgs>(),
        },
        ToolMetadata {
            name: "search_crawl_results",
            category: "citescrape",
            description: "Full-text search across crawled documentation using Tantivy. Supports advanced query syntax including text, phrase, boolean, field-specific, and fu...",
            schema: build_schema::<citescrape::SearchCrawlResultsArgs>(),
        },
        ToolMetadata {
            name: "start_crawl",
            category: "citescrape",
            description: "Start a background web crawl that saves content to markdown/HTML/JSON and optionally indexes for full-text search. Returns immediately with crawl_i...",
            schema: build_schema::<citescrape::StartCrawlArgs>(),
        },
        ToolMetadata {
            name: "web_search",
            category: "citescrape",
            description: "Perform a web search using DuckDuckGo and return structured results with titles, URLs, and snippets.nn Returns up to 10 search results with:n - ran...",
            schema: build_schema::<citescrape::WebSearchArgs>(),
        },
        // CLAUDE_AGENT (5 tools)
        ToolMetadata {
            name: "list_claude_agents",
            category: "claude_agent",
            description: "List all active and completed agent sessions with status and output preview. Shows working indicator (true if actively processing), turn count, run...",
            schema: build_schema::<claude_agent::ListClaudeAgentsArgs>(),
        },
        ToolMetadata {
            name: "read_claude_agent_output",
            category: "claude_agent",
            description: "Read paginated output from an agent session. Returns messages with working indicator. Use offset/length for pagination (offset=0 for start, negativ...",
            schema: build_schema::<claude_agent::ReadClaudeAgentOutputArgs>(),
        },
        ToolMetadata {
            name: "send_claude_agent_prompt",
            category: "claude_agent",
            description: "Send a follow-up prompt to an active agent session. Continues the conversation with new instructions or questions. Use read_claude_agent_output to ...",
            schema: build_schema::<claude_agent::SendClaudeAgentPromptArgs>(),
        },
        ToolMetadata {
            name: "spawn_claude_agent",
            category: "claude_agent",
            description: "Spawn one or more Claude agent sessions for parallel task delegation. Each agent gets identical configuration and can work independently. Use worke...",
            schema: build_schema::<claude_agent::SpawnClaudeAgentArgs>(),
        },
        ToolMetadata {
            name: "terminate_claude_agent_session",
            category: "claude_agent",
            description: "Gracefully terminate an agent session. Closes the ClaudeSDKClient connection, returns final statistics (turn count, message count, runtime), and mo...",
            schema: build_schema::<claude_agent::TerminateClaudeAgentSessionArgs>(),
        },
        // CANDLE_AGENT (4 tools)
        ToolMetadata {
            name: "memorize",
            category: "candle_agent",
            description: "Store content in a named memory library with automatic embedding generation. The memory will be tagged with the library name and can be retrieved later using recall(). Each library is a separate namespace for organizing memories.",
            schema: build_schema::<claude_agent::MemorizeArgs>(),
        },
        ToolMetadata {
            name: "check_memorize_status",
            category: "candle_agent",
            description: "Check the status of an async memorize operation started with memorize().\n\nReturns current status, progress information, and memory_id when complete.\n\nStatus values:\n- IN_PROGRESS: Task is still running (loading content, generating embeddings, storing)\n- COMPLETED: Task finished successfully (memory_id available)\n- FAILED: Task failed (error message available)\n\nPoll this repeatedly (with delays) until status is COMPLETED or FAILED.\nProgress includes current stage (Loading content, Generating embeddings, Storing in database)\nand file counts for multi-file operations.",
            schema: build_schema::<claude_agent::CheckMemorizeStatusArgs>(),
        },
        ToolMetadata {
            name: "recall",
            category: "candle_agent",
            description: "Retrieve relevant memories from a library using semantic search. Searches for content similar to the provided context and returns the most relevant results. Uses vector similarity (cosine) to find semantically related memories.",
            schema: build_schema::<claude_agent::RecallArgs>(),
        },
        ToolMetadata {
            name: "list_memory_libraries",
            category: "candle_agent",
            description: "List all unique memory library names that have been created. Returns a list of all libraries that contain at least one memory. Use this to discover what libraries are available for recall.",
            schema: build_schema::<claude_agent::ListMemoryLibrariesArgs>(),
        },
        // CONFIG (2 tools)
        ToolMetadata {
            name: "get_config",
            category: "config",
            description: "Get complete server configuration including security settings (blocked commands, allowed directories), shell preferences, resource limits, and live...",
            schema: build_schema::<config::GetConfigArgs>(),
        },
        ToolMetadata {
            name: "set_config_value",
            category: "config",
            description: "Set a specific configuration value by key.nn WARNING: Should be used in a separate chat from file operations and n command execution to prevent sec...",
            schema: build_schema::<config::SetConfigValueArgs>(),
        },
        // DATABASE (7 tools)
        ToolMetadata {
            name: "execute_sql",
            category: "database",
            description: "Execute SQL query or multiple SQL statements (separated by semicolons). nn MULTI-STATEMENT BEHAVIOR:n - Write operations (INSERT/UPDATE/DELETE/CREA...",
            schema: build_schema::<database::ExecuteSQLArgs>(),
        },
        ToolMetadata {
            name: "get_pool_stats",
            category: "database",
            description: "Get connection pool health metrics including active connections, idle connections, and pool configuration. Use this to diagnose connection pool exh...",
            schema: build_schema::<database::GetPoolStatsArgs>(),
        },
        ToolMetadata {
            name: "get_stored_procedures",
            category: "database",
            description: "List stored procedures in a schema. Returns procedure names and optionally detailed information including parameters and definitions. Not supported...",
            schema: build_schema::<database::GetStoredProceduresArgs>(),
        },
        ToolMetadata {
            name: "get_table_indexes",
            category: "database",
            description: "Get index information for a table including index names, columns, uniqueness, and primary key status. Use this to understand which columns are inde...",
            schema: build_schema::<database::GetTableIndexesArgs>(),
        },
        ToolMetadata {
            name: "get_table_schema",
            category: "database",
            description: "Get column information for a table including column names, data types, nullability, and default values. Use this before writing queries to understa...",
            schema: build_schema::<database::GetTableSchemaArgs>(),
        },
        ToolMetadata {
            name: "list_schemas",
            category: "database",
            description: "List all schemas (databases) in the current database connection. For PostgreSQL, returns all user schemas (excludes pg_catalog, information_schema)...",
            schema: build_schema::<database::ListSchemasArgs>(),
        },
        ToolMetadata {
            name: "list_tables",
            category: "database",
            description: "List all tables in a schema. If schema not provided, uses default schema (public for PostgreSQL, current database for MySQL, main for SQLite, dbo f...",
            schema: build_schema::<database::ListTablesArgs>(),
        },
        // FILESYSTEM (14 tools)
        ToolMetadata {
            name: "create_directory",
            category: "filesystem",
            description: "Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. Automatically validates paths.' } fn ...",
            schema: build_schema::<filesystem::CreateDirectoryArgs>(),
        },
        ToolMetadata {
            name: "delete_directory",
            category: "filesystem",
            description: "Delete a directory and all its contents recursively. This operation is permanent and cannot be undone. Requires recursive=true to confirm deletion....",
            schema: build_schema::<filesystem::DeleteDirectoryArgs>(),
        },
        ToolMetadata {
            name: "delete_file",
            category: "filesystem",
            description: "Delete a file from the filesystem. This operation is permanent and cannot be undone. Only deletes files, not directories. Automatically validates p...",
            schema: build_schema::<filesystem::DeleteFileArgs>(),
        },
        ToolMetadata {
            name: "edit_block",
            category: "filesystem",
            description: "Apply surgical text replacements to files. Takes old_string and new_string, and performs exact string replacement. By default replaces one occurren...",
            schema: build_schema::<filesystem::EditBlockArgs>(),
        },
        ToolMetadata {
            name: "get_file_info",
            category: "filesystem",
            description: "Retrieve detailed metadata about a file or directory including size, creation time, last modified time, permissions, type, and line count (for text...",
            schema: build_schema::<filesystem::GetFileInfoArgs>(),
        },
        ToolMetadata {
            name: "get_more_search_results",
            category: "filesystem",
            description: "Get more results from an active search with offset-based pagination.nn Supports partial result reading with:n - 'offset' (start result index, defau...",
            schema: build_schema::<filesystem::GetMoreSearchResultsArgs>(),
        },
        ToolMetadata {
            name: "list_directory",
            category: "filesystem",
            description: "List all files and directories in a specified path. Returns entries prefixed with [DIR] or [FILE] to distinguish types. Supports filtering hidden f...",
            schema: build_schema::<filesystem::ListDirectoryArgs>(),
        },
        ToolMetadata {
            name: "list_searches",
            category: "filesystem",
            description: "List all active searches.nn Shows search IDs, search types, patterns, status, and runtime.n Similar to list_sessions for terminal processes. Useful...",
            schema: build_schema::<filesystem::ListSearchesArgs>(),
        },
        ToolMetadata {
            name: "move_file",
            category: "filesystem",
            description: "Move or rename files and directories. Can move files between directories and rename them in a single operation. Both source and destination must be...",
            schema: build_schema::<filesystem::MoveFileArgs>(),
        },
        ToolMetadata {
            name: "read_file",
            category: "filesystem",
            description: "Read the contents of a file from the filesystem or a URL. Supports text files (returned as text) and image files (returned as base64). Use offset a...",
            schema: build_schema::<filesystem::ReadFileArgs>(),
        },
        ToolMetadata {
            name: "read_multiple_files",
            category: "filesystem",
            description: "Read multiple files in parallel. Returns results for all files, including errors for individual files that fail. Supports offset and length paramet...",
            schema: build_schema::<filesystem::ReadMultipleFilesArgs>(),
        },
        ToolMetadata {
            name: "start_search",
            category: "filesystem",
            description: "Start a streaming search that can return results progressively.nn SEARCH STRATEGY GUIDE:n Choose the right search type based on what the user is lo...",
            schema: build_schema::<filesystem::StartSearchArgs>(),
        },
        ToolMetadata {
            name: "stop_search",
            category: "filesystem",
            description: "Stop an active search session.nn Stops the background search process gracefully. Use this when you've found what you need or if a search is taking ...",
            schema: build_schema::<filesystem::StopSearchArgs>(),
        },
        ToolMetadata {
            name: "write_file",
            category: "filesystem",
            description: "Write or append to file contents. Supports two modes: 'rewrite' (overwrite entire file) and 'append' (add to end of file). Automatically validates ...",
            schema: build_schema::<filesystem::WriteFileArgs>(),
        },
        // GIT (20 tools)
        ToolMetadata {
            name: "git_add",
            category: "git",
            description: "Stage file changes for commit in a Git repository. Specify paths to stage specific files.' } fn read_only() -> bool { false // Modifies index } fn ...",
            schema: build_schema::<git::GitAddArgs>(),
        },
        ToolMetadata {
            name: "git_branch_create",
            category: "git",
            description: "Create a new branch in a Git repository. Optionally specify a starting point and checkout the branch after creation.' } fn read_only() -> bool { fa...",
            schema: build_schema::<git::GitBranchCreateArgs>(),
        },
        ToolMetadata {
            name: "git_branch_delete",
            category: "git",
            description: "Delete a branch from a Git repository. Cannot delete the currently checked-out branch.' } fn read_only() -> bool { false // Modifies repository } f...",
            schema: build_schema::<git::GitBranchDeleteArgs>(),
        },
        ToolMetadata {
            name: "git_branch_list",
            category: "git",
            description: "List all local branches in a Git repository.' } fn read_only() -> bool { true // Only reads, doesn't modify } fn destructive() -> bool { false } fn...",
            schema: build_schema::<git::GitBranchListArgs>(),
        },
        ToolMetadata {
            name: "git_branch_rename",
            category: "git",
            description: "Rename a branch in a Git repository. Automatically updates HEAD if renaming the current branch.' } fn read_only() -> bool { false // Modifies repos...",
            schema: build_schema::<git::GitBranchRenameArgs>(),
        },
        ToolMetadata {
            name: "git_checkout",
            category: "git",
            description: "Checkout a Git reference (branch, tag, or commit) or restore specific files. Without paths: switches branches/commits. With paths: restores files f...",
            schema: build_schema::<git::GitCheckoutArgs>(),
        },
        ToolMetadata {
            name: "git_clone",
            category: "git",
            description: "Clone a remote Git repository to a local path. Supports shallow cloning (limited history) and branch-specific cloning. The destination path must no...",
            schema: build_schema::<git::GitCloneArgs>(),
        },
        ToolMetadata {
            name: "git_commit",
            category: "git",
            description: "Create a new commit in a Git repository. Optionally specify author information and stage all modified files.' } fn read_only() -> bool { false // C...",
            schema: build_schema::<git::GitCommitArgs>(),
        },
        ToolMetadata {
            name: "git_discover",
            category: "git",
            description: "Discover a Git repository by searching upward from the given path. This will traverse parent directories until it finds a .git directory or reaches...",
            schema: build_schema::<git::GitDiscoverArgs>(),
        },
        ToolMetadata {
            name: "git_fetch",
            category: "git",
            description: "Fetch updates from a remote repository. Downloads objects and refs from another repository.' } fn read_only() -> bool { false // Fetches refs } fn ...",
            schema: build_schema::<git::GitFetchArgs>(),
        },
        ToolMetadata {
            name: "git_init",
            category: "git",
            description: "Initialize a new Git repository at the specified path. Supports both normal repositories (with working directory) and bare repositories (without wo...",
            schema: build_schema::<git::GitInitArgs>(),
        },
        ToolMetadata {
            name: "git_log",
            category: "git",
            description: "List commit history from a Git repository. Optionally filter by file path and limit the number of results.' } fn read_only() -> bool { true // Only...",
            schema: build_schema::<git::GitLogArgs>(),
        },
        ToolMetadata {
            name: "git_merge",
            category: "git",
            description: "Merge a branch or commit into the current branch. Joins two or more development histories together.' } fn read_only() -> bool { false // Modifies H...",
            schema: build_schema::<git::GitMergeArgs>(),
        },
        ToolMetadata {
            name: "git_open",
            category: "git",
            description: "Open an existing Git repository at the specified path. The repository must already exist at the given location.' } fn read_only() -> bool { true //...",
            schema: build_schema::<git::GitOpenArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_add",
            category: "git",
            description: "Create a new worktree linked to the repository. Allows working on multiple branches simultaneously.' } fn read_only() -> bool { false // Creates wo...",
            schema: build_schema::<git::GitWorktreeAddArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_list",
            category: "git",
            description: "List all worktrees in the repository with detailed status. Returns main worktree and all linked worktrees with their paths, branches, lock status, ...",
            schema: build_schema::<git::GitWorktreeListArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_lock",
            category: "git",
            description: "Lock a worktree to prevent deletion. Useful for worktrees on removable media or network drives.' } fn read_only() -> bool { false // Writes lock fi...",
            schema: build_schema::<git::GitWorktreeLockArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_prune",
            category: "git",
            description: "Remove stale worktree administrative files. Cleans up .git/worktrees/ entries for worktrees whose directories have been manually deleted. Returns l...",
            schema: build_schema::<git::GitWorktreePruneArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_remove",
            category: "git",
            description: "Remove a worktree and its associated administrative files. Cannot remove locked worktrees without force flag.' } fn read_only() -> bool { false // ...",
            schema: build_schema::<git::GitWorktreeRemoveArgs>(),
        },
        ToolMetadata {
            name: "git_worktree_unlock",
            category: "git",
            description: "Unlock a locked worktree. Removes the lock that prevents worktree deletion.' } fn read_only() -> bool { false // Removes lock file } fn destructive...",
            schema: build_schema::<git::GitWorktreeUnlockArgs>(),
        },
        // GITHUB (31 tools)
        ToolMetadata {
            name: "add_issue_comment",
            category: "github",
            description: "Add a comment to an existing GitHub issue. Supports Markdown formatting in the comment body. Requires GITHUB_TOKEN environment variable with write ...",
            schema: build_schema::<github::AddIssueCommentArgs>(),
        },
        ToolMetadata {
            name: "add_pull_request_review_comment",
            category: "github",
            description: "Add an inline review comment to a pull request (comment on specific lines of code). Supports single-line, multi-line, and threaded comments. Requir...",
            schema: build_schema::<github::AddPullRequestReviewCommentArgs>(),
        },
        ToolMetadata {
            name: "code_scanning_alerts",
            category: "github",
            description: "List code scanning security alerts for a GitHub repository. Returns alerts with details about vulnerabilities, their severity, location, and status...",
            schema: build_schema::<github::CodeScanningAlertsArgs>(),
        },
        ToolMetadata {
            name: "create_branch",
            category: "github",
            description: "Create a new branch from a commit SHA' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool { false } fn o...",
            schema: build_schema::<github::CreateBranchArgs>(),
        },
        ToolMetadata {
            name: "create_issue",
            category: "github",
            description: "Create a new issue in a GitHub repository. Supports setting title, body, labels, and assignees. Requires GITHUB_TOKEN environment variable with app...",
            schema: build_schema::<github::CreateIssueArgs>(),
        },
        ToolMetadata {
            name: "create_or_update_file",
            category: "github",
            description: "Create a new file or update an existing file in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idemp...",
            schema: build_schema::<github::CreateOrUpdateFileArgs>(),
        },
        ToolMetadata {
            name: "create_pull_request",
            category: "github",
            description: "Create a new pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool { f...",
            schema: build_schema::<github::CreatePullRequestArgs>(),
        },
        ToolMetadata {
            name: "create_pull_request_review",
            category: "github",
            description: "Create a review on a pull request (approve, request changes, or comment). Requires GITHUB_TOKEN environment variable with repo permissions.' } fn r...",
            schema: build_schema::<github::CreatePullRequestReviewArgs>(),
        },
        ToolMetadata {
            name: "create_repository",
            category: "github",
            description: "Create a new repository under the authenticated user's account' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent...",
            schema: build_schema::<github::CreateRepositoryArgs>(),
        },
        ToolMetadata {
            name: "fork_repository",
            category: "github",
            description: "Fork a repository to your account or an organization' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool...",
            schema: build_schema::<github::ForkRepositoryArgs>(),
        },
        ToolMetadata {
            name: "get_commit",
            category: "github",
            description: "Get detailed information about a specific commit' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool { tr...",
            schema: build_schema::<github::GetCommitArgs>(),
        },
        ToolMetadata {
            name: "get_file_contents",
            category: "github",
            description: "Get file or directory contents from a GitHub repository' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bo...",
            schema: build_schema::<github::GetFileContentsArgs>(),
        },
        ToolMetadata {
            name: "get_issue",
            category: "github",
            description: "Fetch a single GitHub issue by number. Returns detailed issue information including title, body, state, labels, assignees, comments count, and time...",
            schema: build_schema::<github::GetIssueArgs>(),
        },
        ToolMetadata {
            name: "get_issue_comments",
            category: "github",
            description: "Fetch all comments for a GitHub issue. Returns an array of comment objects including author, body, timestamps, and metadata. Comments are returned ...",
            schema: build_schema::<github::GetIssueCommentsArgs>(),
        },
        ToolMetadata {
            name: "get_me",
            category: "github",
            description: "Get information about the authenticated GitHub user. Returns user profile details including login, name, email, avatar, bio, company, location, rep...",
            schema: build_schema::<github::GetMeArgs>(),
        },
        ToolMetadata {
            name: "get_pull_request_files",
            category: "github",
            description: "Get all files changed in a pull request with their diff stats' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent()...",
            schema: build_schema::<github::GetPullRequestFilesArgs>(),
        },
        ToolMetadata {
            name: "get_pull_request_reviews",
            category: "github",
            description: "Get all reviews for a pull request. Shows approval status, requested changes, and comments from reviewers. Requires GITHUB_TOKEN environment variab...",
            schema: build_schema::<github::GetPullRequestReviewsArgs>(),
        },
        ToolMetadata {
            name: "get_pull_request_status",
            category: "github",
            description: "Get detailed status information about a pull request including merge status, checks, and review state' } fn read_only() -> bool { true } fn destruc...",
            schema: build_schema::<github::GetPullRequestStatusArgs>(),
        },
        ToolMetadata {
            name: "list_branches",
            category: "github",
            description: "List all branches in a repository' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool { true } fn open_wo...",
            schema: build_schema::<github::ListBranchesArgs>(),
        },
        ToolMetadata {
            name: "list_commits",
            category: "github",
            description: "List commits in a repository with filtering options' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool {...",
            schema: build_schema::<github::ListCommitsArgs>(),
        },
        ToolMetadata {
            name: "list_issues",
            category: "github",
            description: "List and filter issues in a GitHub repository. Supports filtering by state, labels, assignee, and pagination. Returns an array of issue objects. Re...",
            schema: build_schema::<github::ListIssuesArgs>(),
        },
        ToolMetadata {
            name: "merge_pull_request",
            category: "github",
            description: "Merge a pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { true } fn idempotent() -> bool { false }...",
            schema: build_schema::<github::MergePullRequestArgs>(),
        },
        ToolMetadata {
            name: "push_files",
            category: "github",
            description: "Push multiple files to a GitHub repository in a single commit. All files are added atomically (creates tree, commit, and updates ref). File content...",
            schema: build_schema::<github::PushFilesArgs>(),
        },
        ToolMetadata {
            name: "request_copilot_review",
            category: "github",
            description: "Request GitHub Copilot to review a pull request (experimental feature). Triggers automated code review from Copilot. Requires GITHUB_TOKEN and Copi...",
            schema: build_schema::<github::RequestCopilotReviewArgs>(),
        },
        ToolMetadata {
            name: "search_code",
            category: "github",
            description: "Search code across GitHub repositories using GitHub's code search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn i...",
            schema: build_schema::<github::SearchCodeArgs>(),
        },
        ToolMetadata {
            name: "search_issues",
            category: "github",
            description: "Search for issues across GitHub using GitHub's powerful search syntax. Supports filtering by repository, state, labels, assignee, author, dates, an...",
            schema: build_schema::<github::SearchIssuesArgs>(),
        },
        ToolMetadata {
            name: "search_repositories",
            category: "github",
            description: "Search GitHub repositories using GitHub's repository search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempot...",
            schema: build_schema::<github::SearchRepositoriesArgs>(),
        },
        ToolMetadata {
            name: "search_users",
            category: "github",
            description: "Search GitHub users using GitHub's user search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool...",
            schema: build_schema::<github::SearchUsersArgs>(),
        },
        ToolMetadata {
            name: "secret_scanning_alerts",
            category: "github",
            description: "List secret scanning alerts (leaked credentials) for a GitHub repository. Returns alerts about exposed secrets like API keys, tokens, passwords, an...",
            schema: build_schema::<github::SecretScanningAlertsArgs>(),
        },
        ToolMetadata {
            name: "update_issue",
            category: "github",
            description: "Update an existing GitHub issue. Supports partial updates - only specified fields will be modified. Can update title, body, state (open/closed), la...",
            schema: build_schema::<github::UpdateIssueArgs>(),
        },
        ToolMetadata {
            name: "update_pull_request",
            category: "github",
            description: "Update an existing pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bo...",
            schema: build_schema::<github::UpdatePullRequestArgs>(),
        },
        // INTROSPECTION (2 tools)
        ToolMetadata {
            name: "get_recent_tool_calls",
            category: "introspection",
            description: "Get recent tool call history with their arguments and outputs. Returns chronological list of tool calls made during this session. Supports paginati...",
            schema: build_schema::<introspection::GetRecentToolCallsArgs>(),
        },
        ToolMetadata {
            name: "get_usage_stats",
            category: "introspection",
            description: "Get usage statistics for debugging and analysis. Returns summary of tool usage, success/failure rates, and performance metrics.' } async fn execute...",
            schema: build_schema::<introspection::GetUsageStatsArgs>(),
        },
        // PROCESS (2 tools)
        ToolMetadata {
            name: "kill_process",
            category: "process",
            description: "Terminate a running process by its PID. Sends SIGKILL signal to forcefully stop the process. Use with caution as this does not allow graceful shutd...",
            schema: build_schema::<process::KillProcessArgs>(),
        },
        ToolMetadata {
            name: "list_processes",
            category: "process",
            description: "List all running processes with PID, command name, CPU usage, and memory usage. Supports filtering by process name and limiting results. Returns co...",
            schema: build_schema::<process::ListProcessesArgs>(),
        },
        // PROMPT (4 tools)
        ToolMetadata {
            name: "add_prompt",
            category: "prompt",
            description: "Create a new prompt template. The content must include YAML frontmatter with metadata (title, description, categories, author) followed by the temp...",
            schema: build_schema::<prompt::AddPromptArgs>(),
        },
        ToolMetadata {
            name: "delete_prompt",
            category: "prompt",
            description: "Delete a prompt template. Requires confirm=true for safety. This action cannot be undone. Default prompts can be deleted but will be recreated on n...",
            schema: build_schema::<prompt::DeletePromptArgs>(),
        },
        ToolMetadata {
            name: "edit_prompt",
            category: "prompt",
            description: "Edit an existing prompt template. Provide the prompt name and complete new content (including YAML frontmatter). The content is validated before sa...",
            schema: build_schema::<prompt::EditPromptArgs>(),
        },
        ToolMetadata {
            name: "get_prompt",
            category: "prompt",
            description: "Browse and retrieve prompt templates. nn Actions:n - list_categories: Show all prompt categoriesn - list_prompts: List all prompts (optionally filt...",
            schema: build_schema::<prompt::GetPromptArgs>(),
        },
        // REASONER (1 tool)
        ToolMetadata {
            name: "sequential_thinking_reasoner",
            category: "reasoner",
            description: "Advanced reasoning tool with multiple strategies (Beam Search, MCTS). Processes thoughts step-by-step, supports branching and revision, and tracks best reasoning paths. Use for complex problem-solving that requires exploration of multiple solution approaches.\n\nStrategies:\n- beam_search: Breadth-first exploration (default)\n- mcts: Monte Carlo Tree Search with UCB1\n- mcts_002_alpha: High exploration MCTS variant\n- mcts_002alt_alpha: Length-rewarding MCTS variant\n\nOptional VoyageAI Embedding Integration: Set VOYAGE_API_KEY environment variable to enable semantic coherence scoring.",
            schema: build_schema::<reasoning::ReasonerArgs>(),
        },
        // SEQUENTIAL-THINKING (1 tool)
        ToolMetadata {
            name: "sequential_thinking",
            category: "sequential_thinking",
            description: "A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens.",
            schema: build_schema::<reasoning::SequentialThinkingArgs>(),
        },
        // TERMINAL (5 tools)
        ToolMetadata {
            name: "list_terminal_commands",
            category: "terminal",
            description: "List all active command sessions. Returns array of sessions with PID, blocked status, and runtime. Use this to monitor all running commands and get...",
            schema: build_schema::<terminal::ListTerminalCommandsArgs>(),
        },
        ToolMetadata {
            name: "read_terminal_output",
            category: "terminal",
            description: "Get output from a PTY terminal session with offset-based pagination.nn Supports partial output reading from VT100 screen buffer:n - offset: 0, leng...",
            schema: build_schema::<terminal::ReadTerminalOutputArgs>(),
        },
        ToolMetadata {
            name: "send_terminal_input",
            category: "terminal",
            description: "Send input text to a running PTY terminal process. Perfect for interacting with REPLs (Python, Node.js, etc.), interactive programs (vim, top), and...",
            schema: build_schema::<terminal::SendTerminalInputArgs>(),
        },
        ToolMetadata {
            name: "start_terminal_command",
            category: "terminal",
            description: "Execute a shell command with full terminal emulation. Supports long-running commands, output streaming, and session management. Returns PID for tra...",
            schema: build_schema::<terminal::StartTerminalCommandArgs>(),
        },
        ToolMetadata {
            name: "stop_terminal_command",
            category: "terminal",
            description: "Force terminate a running command session by PID. Attempts graceful termination first (SIGTERM), then force kills after 1 second if still running (...",
            schema: build_schema::<terminal::StopTerminalCommandArgs>(),
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
