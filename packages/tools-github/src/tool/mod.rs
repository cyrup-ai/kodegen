//! MCP Tools for GitHub operations
//!
//! This module provides Model Context Protocol (MCP) tool wrappers around
//! the core GitHub operations for use in AI agent systems.

// Issue Operations
pub mod create_issue;
pub mod get_issue;
pub mod list_issues;
pub mod update_issue;
pub mod search_issues;
pub mod add_issue_comment;
pub mod get_issue_comments;

// Pull Request Operations
pub mod create_pull_request;
pub mod update_pull_request;
pub mod merge_pull_request;
pub mod get_pull_request_status;
pub mod get_pull_request_files;

// Pull Request Review Operations
pub mod get_pull_request_reviews;
pub mod create_pull_request_review;
pub mod add_pull_request_review_comment;
pub mod request_copilot_review;

// Repository Operations
pub mod create_repository;
pub mod fork_repository;
pub mod list_branches;
pub mod create_branch;
pub mod list_commits;
pub mod get_commit;

// Search Operations
pub mod search_code;
pub mod search_repositories;
pub mod search_users;

// Re-export tools and their argument types
pub use create_issue::{CreateIssueTool, CreateIssueArgs};
pub use get_issue::{GetIssueTool, GetIssueArgs};
pub use list_issues::{ListIssuesTool, ListIssuesArgs};
pub use update_issue::{UpdateIssueTool, UpdateIssueArgs};
pub use search_issues::{SearchIssuesTool, SearchIssuesArgs};
pub use add_issue_comment::{AddIssueCommentTool, AddIssueCommentArgs};
pub use get_issue_comments::{GetIssueCommentsTool, GetIssueCommentsArgs};

pub use create_pull_request::{CreatePullRequestTool, CreatePullRequestArgs};
pub use update_pull_request::{UpdatePullRequestTool, UpdatePullRequestArgs};
pub use merge_pull_request::{MergePullRequestTool, MergePullRequestArgs};
pub use get_pull_request_status::{GetPullRequestStatusTool, GetPullRequestStatusArgs};
pub use get_pull_request_files::{GetPullRequestFilesTool, GetPullRequestFilesArgs};

pub use get_pull_request_reviews::{GetPullRequestReviewsTool, GetPullRequestReviewsArgs};
pub use create_pull_request_review::{CreatePullRequestReviewTool, CreatePullRequestReviewArgs};
pub use add_pull_request_review_comment::{AddPullRequestReviewCommentTool, AddPullRequestReviewCommentArgs};
pub use request_copilot_review::{RequestCopilotReviewTool, RequestCopilotReviewArgs};

pub use create_repository::{CreateRepositoryTool, CreateRepositoryArgs};
pub use fork_repository::{ForkRepositoryTool, ForkRepositoryArgs};
pub use list_branches::{ListBranchesTool, ListBranchesArgs};
pub use create_branch::{CreateBranchTool, CreateBranchArgs};
pub use list_commits::{ListCommitsTool, ListCommitsArgs};
pub use get_commit::{GetCommitTool, GetCommitArgs};

pub use search_code::{SearchCodeTool, SearchCodeArgs};
pub use search_repositories::{SearchRepositoriesTool, SearchRepositoriesArgs};
pub use search_users::{SearchUsersTool, SearchUsersArgs};
