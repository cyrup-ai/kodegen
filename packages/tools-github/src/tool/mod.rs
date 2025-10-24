//! MCP Tools for GitHub operations
//!
//! This module provides Model Context Protocol (MCP) tool wrappers around
//! the core GitHub operations for use in AI agent systems.

// Issue Operations
pub mod add_issue_comment;
pub mod create_issue;
pub mod get_issue;
pub mod get_issue_comments;
pub mod list_issues;
pub mod search_issues;
pub mod update_issue;

// Pull Request Operations
pub mod create_pull_request;
pub mod get_pull_request_files;
pub mod get_pull_request_status;
pub mod merge_pull_request;
pub mod update_pull_request;

// Pull Request Review Operations
pub mod add_pull_request_review_comment;
pub mod create_pull_request_review;
pub mod get_pull_request_reviews;
pub mod request_copilot_review;

// Repository Operations
pub mod create_branch;
pub mod create_repository;
pub mod fork_repository;
pub mod get_commit;
pub mod list_branches;
pub mod list_commits;

// Search Operations
pub mod search_code;
pub mod search_repositories;
pub mod search_users;

// Re-export tools only (Args are imported from kodegen_mcp_schema::github)
pub use add_issue_comment::AddIssueCommentTool;
pub use create_issue::CreateIssueTool;
pub use get_issue::GetIssueTool;
pub use get_issue_comments::GetIssueCommentsTool;
pub use list_issues::ListIssuesTool;
pub use search_issues::SearchIssuesTool;
pub use update_issue::UpdateIssueTool;

pub use create_pull_request::CreatePullRequestTool;
pub use get_pull_request_files::GetPullRequestFilesTool;
pub use get_pull_request_status::GetPullRequestStatusTool;
pub use merge_pull_request::MergePullRequestTool;
pub use update_pull_request::UpdatePullRequestTool;

pub use add_pull_request_review_comment::AddPullRequestReviewCommentTool;
pub use create_pull_request_review::CreatePullRequestReviewTool;
pub use get_pull_request_reviews::GetPullRequestReviewsTool;
pub use request_copilot_review::RequestCopilotReviewTool;

pub use create_branch::CreateBranchTool;
pub use create_repository::CreateRepositoryTool;
pub use fork_repository::ForkRepositoryTool;
pub use get_commit::GetCommitTool;
pub use list_branches::ListBranchesTool;
pub use list_commits::ListCommitsTool;

pub use search_code::SearchCodeTool;
pub use search_repositories::SearchRepositoriesTool;
pub use search_users::SearchUsersTool;
