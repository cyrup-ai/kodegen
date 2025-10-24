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
pub use github::upload_release_asset::{UploadAssetOptions, upload_release_asset};

// Re-export GitHub types for public API
pub use github::{
    ActivityMetrics,
    CiCdMetrics,
    CodeQualityMetrics,
    DependencyMetrics,
    DocumentationMetrics,
    GithubSearch,
    LocalMetrics,
    MetadataInfo,
    Output as SearchOutput,
    QualityMetrics,
    ReadmeMetrics,
    RepositoryResult,
    SearchConfig,
    SearchError,
    // User search types
    SearchOrder,
    SearchProvider,
    SearchQuery,
    SearchSession,
    SecurityMetrics,
    StructureMetrics,
    TestMetrics,
    UserSearchSort,
    // Search functionality - both convenience functions and types
    search_repositories,
    search_repositories_with_config,
};

// MCP Tools (conditional compilation)
#[cfg(feature = "mcp")]
pub mod tool;

#[cfg(feature = "mcp")]
pub use tool::{
    AddIssueCommentArgs, AddIssueCommentTool, AddPullRequestReviewCommentArgs,
    AddPullRequestReviewCommentTool, CreateBranchArgs, CreateBranchTool, CreateIssueArgs,
    CreateIssueTool, CreatePullRequestArgs, CreatePullRequestReviewArgs,
    CreatePullRequestReviewTool, CreatePullRequestTool, CreateRepositoryArgs, CreateRepositoryTool,
    ForkRepositoryArgs, ForkRepositoryTool, GetCommitArgs, GetCommitTool, GetIssueArgs,
    GetIssueCommentsArgs, GetIssueCommentsTool, GetIssueTool, GetPullRequestFilesArgs,
    GetPullRequestFilesTool, GetPullRequestReviewsArgs, GetPullRequestReviewsTool,
    GetPullRequestStatusArgs, GetPullRequestStatusTool, ListBranchesArgs, ListBranchesTool,
    ListCommitsArgs, ListCommitsTool, ListIssuesArgs, ListIssuesTool, MergePullRequestArgs,
    MergePullRequestTool, RequestCopilotReviewArgs, RequestCopilotReviewTool, SearchCodeArgs,
    SearchCodeTool, SearchIssuesArgs, SearchIssuesTool, SearchRepositoriesArgs,
    SearchRepositoriesTool, SearchUsersArgs, SearchUsersTool, UpdateIssueArgs, UpdateIssueTool,
    UpdatePullRequestArgs, UpdatePullRequestTool,
};
