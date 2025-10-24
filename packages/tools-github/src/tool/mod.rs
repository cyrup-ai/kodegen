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

// Re-export tools and their argument types
pub use add_issue_comment::{AddIssueCommentArgs, AddIssueCommentTool};
pub use create_issue::{CreateIssueArgs, CreateIssueTool};
pub use get_issue::{GetIssueArgs, GetIssueTool};
pub use get_issue_comments::{GetIssueCommentsArgs, GetIssueCommentsTool};
pub use list_issues::{ListIssuesArgs, ListIssuesTool};
pub use search_issues::{SearchIssuesArgs, SearchIssuesTool};
pub use update_issue::{UpdateIssueArgs, UpdateIssueTool};

pub use create_pull_request::{CreatePullRequestArgs, CreatePullRequestTool};
pub use get_pull_request_files::{GetPullRequestFilesArgs, GetPullRequestFilesTool};
pub use get_pull_request_status::{GetPullRequestStatusArgs, GetPullRequestStatusTool};
pub use merge_pull_request::{MergePullRequestArgs, MergePullRequestTool};
pub use update_pull_request::{UpdatePullRequestArgs, UpdatePullRequestTool};

pub use add_pull_request_review_comment::{
    AddPullRequestReviewCommentArgs, AddPullRequestReviewCommentTool,
};
pub use create_pull_request_review::{CreatePullRequestReviewArgs, CreatePullRequestReviewTool};
pub use get_pull_request_reviews::{GetPullRequestReviewsArgs, GetPullRequestReviewsTool};
pub use request_copilot_review::{RequestCopilotReviewArgs, RequestCopilotReviewTool};

pub use create_branch::{CreateBranchArgs, CreateBranchTool};
pub use create_repository::{CreateRepositoryArgs, CreateRepositoryTool};
pub use fork_repository::{ForkRepositoryArgs, ForkRepositoryTool};
pub use get_commit::{GetCommitArgs, GetCommitTool};
pub use list_branches::{ListBranchesArgs, ListBranchesTool};
pub use list_commits::{ListCommitsArgs, ListCommitsTool};

pub use search_code::{SearchCodeArgs, SearchCodeTool};
pub use search_repositories::{SearchRepositoriesArgs, SearchRepositoriesTool};
pub use search_users::{SearchUsersArgs, SearchUsersTool};
