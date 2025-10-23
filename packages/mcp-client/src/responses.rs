//! Typed response structures for MCP tool calls
//!
//! This module provides strongly-typed response structures for parsing tool call results.
//! Using these types instead of manual JSON parsing provides:
//! - Type safety with compiler checks
//! - Clear error messages with context
//! - Support for multiple field name conventions (`camelCase/snake_case`)
//! - Prevention of silent failures from missing or mistyped fields

use serde::Deserialize;

/// Response from starting a web crawl session
#[derive(Debug, Deserialize)]
pub struct StartCrawlResponse {
    /// The crawl ID for this crawl session
    pub crawl_id: String,
}

/// Response from starting a file/content search
#[derive(Debug, Deserialize)]
pub struct StartSearchResponse {
    /// The session ID for this search
    /// Supports both `sessionId` (camelCase) and `session_id` (`snake_case`)
    #[serde(alias = "sessionId")]
    pub session_id: String,
}

/// Response from spawning a Claude agent sub-session
#[derive(Debug, Deserialize)]
pub struct SpawnClaudeAgentResponse {
    /// The session IDs for spawned Claude agents
    pub session_ids: Vec<String>,
    
    /// Number of workers spawned
    pub worker_count: u32,
    
    /// Agent information for each spawned agent
    #[serde(default)]
    pub agents: Vec<serde_json::Value>,
}

/// Response from starting a terminal command
#[derive(Debug, Deserialize)]
pub struct StartTerminalCommandResponse {
    /// The process ID (PID) of the started command
    pub pid: i64,

    /// Optional status information
    #[serde(default)]
    pub status: Option<String>,
}

/// Response from getting prompt template
#[derive(Debug, Deserialize)]
pub struct GetPromptResponse {
    pub name: String,
    pub metadata: PromptMetadata,
    pub content: String,
    pub rendered: bool,
}

/// Response from rendering prompt with parameters
#[derive(Debug, Deserialize)]
pub struct RenderPromptResponse {
    pub name: String,
    pub content: String,
    pub rendered: bool,
}

/// Prompt metadata structure
#[derive(Debug, Deserialize)]
pub struct PromptMetadata {
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub author: String,
    #[serde(default)]
    pub parameters: Vec<ParameterDefinition>,
}

/// Parameter definition in prompt metadata
#[derive(Debug, Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// Response from `get_config` tool
#[derive(Debug, Deserialize)]
pub struct GetConfigResponse {
    pub blocked_commands: Vec<String>,
    pub default_shell: String,
    pub allowed_directories: Vec<String>,
    pub denied_directories: Vec<String>,
    pub file_read_line_limit: usize,
    pub file_write_line_limit: usize,
    pub fuzzy_search_threshold: f64,
    pub sse_connection_timeout_secs: u64,
    #[serde(default)]
    pub current_client: Option<ClientInfo>,
    #[serde(default)]
    pub client_history: Vec<ClientRecord>,
    pub system_info: SystemInfo,
}

/// Client information from MCP initialization
#[derive(Debug, Deserialize, Clone)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Client connection record with timestamps
#[derive(Debug, Deserialize, Clone)]
pub struct ClientRecord {
    pub client_info: ClientInfo,
    pub connected_at: String,
    pub last_seen: String,
}

/// System diagnostic information
#[derive(Debug, Deserialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub rust_version: String,
    pub cpu_count: usize,
    pub memory: MemoryInfo,
}

/// Memory usage information
#[derive(Debug, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: String,
    pub available_mb: String,
    pub used_mb: String,
}

/// Response from `sequential_thinking` tool
#[derive(Debug, Deserialize)]
pub struct SequentialThinkingResponse {
    pub session_id: String,
    pub thought_number: u32,
    pub total_thoughts: u32,
    pub next_thought_needed: bool,
    pub branches: Vec<String>,
    pub thought_history_length: usize,
}

// ============================================================================
// GitHub Response Types
// ============================================================================

/// GitHub user information
#[derive(Debug, Deserialize, Clone, Default)]
pub struct GitHubUser {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub html_url: String,
}

/// GitHub repository information
#[derive(Debug, Deserialize, Clone, Default)]
pub struct GitHubRepository {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub owner: GitHubUser,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub clone_url: Option<String>,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub stargazers_count: Option<u64>,
    #[serde(default)]
    pub forks_count: Option<u64>,
    #[serde(default)]
    pub open_issues_count: Option<u64>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// GitHub issue label
#[derive(Debug, Deserialize, Clone)]
pub struct GitHubLabel {
    pub id: u64,
    pub name: String,
    pub color: String,
}

/// GitHub issue
#[derive(Debug, Deserialize)]
pub struct GitHubIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    pub state: String,
    pub user: GitHubUser,
    #[serde(default)]
    pub assignees: Vec<GitHubUser>,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub comment (on issues or PRs)
#[derive(Debug, Deserialize)]
pub struct GitHubComment {
    pub id: u64,
    pub body: String,
    pub user: GitHubUser,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub branch reference
#[derive(Debug, Deserialize, Clone)]
pub struct GitHubBranchRef {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub sha: String,
    pub repo: GitHubRepository,
}

/// GitHub pull request
#[derive(Debug, Deserialize)]
pub struct GitHubPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    pub state: String,
    pub user: GitHubUser,
    pub head: GitHubBranchRef,
    pub base: GitHubBranchRef,
    pub html_url: String,
    #[serde(default)]
    pub mergeable: Option<bool>,
    pub merged: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub pull request review
#[derive(Debug, Deserialize)]
pub struct GitHubReview {
    pub id: u64,
    pub user: GitHubUser,
    #[serde(default)]
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    #[serde(default)]
    pub submitted_at: Option<String>,
}

/// GitHub pull request file change
#[derive(Debug, Deserialize)]
pub struct GitHubPullRequestFile {
    pub filename: String,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    pub changes: u64,
    #[serde(default)]
    pub patch: Option<String>,
}

/// GitHub branch
#[derive(Debug, Deserialize)]
pub struct GitHubBranch {
    pub name: String,
    pub commit: GitHubCommitRef,
    pub protected: bool,
}

/// GitHub commit reference (short version)
#[derive(Debug, Deserialize, Clone)]
pub struct GitHubCommitRef {
    pub sha: String,
    pub url: String,
}

/// GitHub commit (full version)
#[derive(Debug, Deserialize)]
pub struct GitHubCommit {
    pub sha: String,
    pub commit: GitHubCommitDetail,
    #[serde(default)]
    pub author: Option<GitHubUser>,
    #[serde(default)]
    pub committer: Option<GitHubUser>,
    pub html_url: String,
}

/// GitHub commit details
#[derive(Debug, Deserialize)]
pub struct GitHubCommitDetail {
    pub message: String,
    pub author: GitHubCommitAuthor,
    pub committer: GitHubCommitAuthor,
}

/// GitHub commit author/committer information
#[derive(Debug, Deserialize)]
pub struct GitHubCommitAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

/// GitHub merge result
#[derive(Debug, Deserialize)]
pub struct GitHubMergeResult {
    pub sha: String,
    pub merged: bool,
    pub message: String,
}

/// GitHub search results wrapper
#[derive(Debug, Deserialize)]
pub struct GitHubSearchResults<T> {
    pub total_count: u64,
    pub incomplete_results: bool,
    pub items: Vec<T>,
}

/// Response wrapper for `list_issues` and `search_issues` tools
///
/// Our GitHub tools return this custom format instead of standard GitHub API format.
/// The Issue objects are complete—no follow-up API calls needed.
///
/// Format: {"count": N, "issues": [...]}
///
/// Used by:
/// - `packages/github/src/tool/list_issues.rs:131`
/// - `packages/github/src/tool/search_issues.rs:106`
#[derive(Debug, Deserialize)]
pub struct GitHubIssuesResponse {
    /// Total number of issues returned
    pub count: u64,
    
    /// Complete GitHub Issue objects with all fields populated
    pub issues: Vec<GitHubIssue>,
}

/// Response wrapper for `get_issue_comments` tool
///
/// Our GitHub tools return this custom format instead of standard GitHub API format.
/// The Comment objects are complete—no follow-up API calls needed.
///
/// Format: {"count": N, "comments": [...]}
///
/// Used by:
/// - `packages/github/src/tool/get_issue_comments.rs`
#[derive(Debug, Deserialize)]
pub struct GitHubCommentsResponse {
    /// Total number of comments returned
    pub count: u64,
    
    /// Complete GitHub Comment objects with all fields populated
    pub comments: Vec<GitHubComment>,
}

/// GitHub code search result
#[derive(Debug, Deserialize)]
pub struct GitHubCodeResult {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub html_url: String,
    pub repository: GitHubRepository,
}
