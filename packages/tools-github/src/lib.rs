//! `kodegen_github` - GitHub API operations via Octocrab
//!
//! This library provides an async-first GitHub service layer with comprehensive
//! GitHub API support using the octocrab crate. Each GitHub operation is
//! implemented in its own module with builder patterns for ergonomic usage.

// Module declarations
pub mod github;
pub mod runtime;

// Re-export runtime types
pub use runtime::{AsyncStream, AsyncTask, EmitterBuilder};

// Re-export GitHub client types
pub use github::{GitHubClient, GitHubClientBuilder};

// Re-export GitHub error types
pub use github::{GitHubError, GitHubResult};

// Re-export GitHub operation options
pub use github::{
    CreatePullRequestReviewOptions, CreateReleaseOptions as GitHubReleaseOptions,
    ListCommitsOptions, MergePullRequestOptions, ReleaseResult as GitHubReleaseResult,
    UpdatePullRequestOptions, create_release, delete_release, get_release_by_tag,
};

// Re-export release asset upload types
pub use github::upload_release_asset::{upload_release_asset, UploadAssetOptions};

// Re-export GitHub types for public API
pub use github::{
    // User search types
    SearchOrder, UserSearchSort,
    // Search functionality - both convenience functions and types
    search_repositories, search_repositories_with_config, ActivityMetrics, CiCdMetrics,
    CodeQualityMetrics, DependencyMetrics, DocumentationMetrics, GithubSearch, LocalMetrics,
    MetadataInfo, Output as SearchOutput, QualityMetrics, ReadmeMetrics, RepositoryResult,
    SearchConfig, SearchError, SearchProvider, SearchQuery, SearchSession, SecurityMetrics,
    StructureMetrics, TestMetrics,
};

// MCP Tools (conditional compilation)
#[cfg(feature = "mcp")]
pub mod tool;

#[cfg(feature = "mcp")]
pub use tool::{
    CreateIssueTool, CreateIssueArgs,
    GetIssueTool, GetIssueArgs,
    ListIssuesTool, ListIssuesArgs,
    UpdateIssueTool, UpdateIssueArgs,
    SearchIssuesTool, SearchIssuesArgs,
    AddIssueCommentTool, AddIssueCommentArgs,
    GetIssueCommentsTool, GetIssueCommentsArgs,
    CreatePullRequestTool, CreatePullRequestArgs,
    UpdatePullRequestTool, UpdatePullRequestArgs,
    MergePullRequestTool, MergePullRequestArgs,
    GetPullRequestStatusTool, GetPullRequestStatusArgs,
    GetPullRequestFilesTool, GetPullRequestFilesArgs,
    GetPullRequestReviewsTool, GetPullRequestReviewsArgs,
    CreatePullRequestReviewTool, CreatePullRequestReviewArgs,
    AddPullRequestReviewCommentTool, AddPullRequestReviewCommentArgs,
    RequestCopilotReviewTool, RequestCopilotReviewArgs,
    CreateRepositoryTool, CreateRepositoryArgs,
    ForkRepositoryTool, ForkRepositoryArgs,
    ListBranchesTool, ListBranchesArgs,
    CreateBranchTool, CreateBranchArgs,
    ListCommitsTool, ListCommitsArgs,
    GetCommitTool, GetCommitArgs,
    SearchCodeTool, SearchCodeArgs,
    SearchRepositoriesTool, SearchRepositoriesArgs,
    SearchUsersTool, SearchUsersArgs,
};
