//! GitHub API operations module
//!
//! Provides GitHub API operations using the octocrab library.

pub mod client;
pub mod error;
pub mod util;

// Re-export client types
pub use client::{GitHubClient, GitHubClientBuilder};

// Re-export error types
pub use error::{GitHubError, GitHubResult};
pub use util::spawn_task;

// Re-export options types
pub use add_pull_request_review_comment::AddPullRequestReviewCommentRequest;
pub use create_or_update_file::CreateOrUpdateFileRequest;
pub use create_pull_request::CreatePullRequestRequest;
pub use create_pull_request_review::CreatePullRequestReviewOptions;
pub use create_release::{CreateReleaseOptions, ReleaseResult, create_release, get_release_by_tag, delete_release};
pub use list_commits::ListCommitsOptions;
pub use list_issues::ListIssuesRequest;
pub use merge_pull_request::MergePullRequestOptions;
pub use update_issue::UpdateIssueRequest;
pub use update_pull_request::UpdatePullRequestOptions;

// GitHub API operations - Issues (internal)
pub(crate) mod add_issue_comment;
pub(crate) mod create_issue;
pub(crate) mod get_issue;
pub(crate) mod get_issue_comments;
pub(crate) mod list_issues;
pub(crate) mod search_issues;
pub(crate) mod update_issue;

// GitHub API operations - Pull Requests (internal)
pub(crate) mod add_pull_request_review_comment;
pub(crate) mod create_pull_request;
pub(crate) mod create_pull_request_review;
pub(crate) mod get_pull_request_comments;
pub(crate) mod get_pull_request_files;
pub(crate) mod get_pull_request_reviews;
pub(crate) mod get_pull_request_status;
pub(crate) mod merge_pull_request;
pub(crate) mod request_copilot_review;
pub(crate) mod update_pull_request;

// GitHub API operations - Repositories (internal)
pub(crate) mod create_branch;
pub(crate) mod create_or_update_file;
pub(crate) mod create_release;
pub(crate) mod upload_release_asset;
pub(crate) mod create_repository;
pub(crate) mod fork_repository;
pub(crate) mod get_commit;
pub(crate) mod get_file_contents;
pub(crate) mod list_branches;
pub(crate) mod list_commits;
pub(crate) mod push_files;
pub(crate) mod search_code;
pub mod search_repositories;

// GitHub API operations - Users (internal)
pub(crate) mod get_me;
pub mod search_users;

// GitHub API operations - Security (internal)
pub(crate) mod code_scanning_alerts;
pub(crate) mod secret_scanning_alerts;

// Re-export search_users types for public API
pub use search_users::{SearchOrder, UserSearchSort};

// Re-export search functionality
pub use search_repositories::{
    search_repositories, search_repositories_with_config, ActivityMetrics, CiCdMetrics,
    CodeQualityMetrics, DependencyMetrics, DocumentationMetrics, GithubSearch, LocalMetrics,
    MetadataInfo, Output, QualityMetrics, ReadmeMetrics, RepositoryResult, SearchConfig,
    SearchError, SearchProvider, SearchQuery, SearchSession, SecurityMetrics, StructureMetrics,
    TestMetrics,
};
