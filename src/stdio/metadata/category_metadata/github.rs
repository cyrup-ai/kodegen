//! GitHub API tools: issues, pull requests, repositories, code search

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn github_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: GITHUB_ADD_ISSUE_COMMENT,
            category: "github",
            description: "Add a comment to an existing GitHub issue. Supports Markdown formatting in the comment body. Requires GITHUB_TOKEN environment variable with write ...",
            schema: build_schema::<github::AddIssueCommentArgs>(),
        },
        ToolMetadata {
            name: GITHUB_ADD_PULL_REQUEST_REVIEW_COMMENT,
            category: "github",
            description: "Add an inline review comment to a pull request (comment on specific lines of code). Supports single-line, multi-line, and threaded comments. Requir...",
            schema: build_schema::<github::AddPullRequestReviewCommentArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CODE_SCANNING_ALERTS,
            category: "github",
            description: "List code scanning security alerts for a GitHub repository. Returns alerts with details about vulnerabilities, their severity, location, and status...",
            schema: build_schema::<github::CodeScanningAlertsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_BRANCH,
            category: "github",
            description: "Create a new branch from a commit SHA' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool { false } fn o...",
            schema: build_schema::<github::CreateBranchArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_ISSUE,
            category: "github",
            description: "Create a new issue in a GitHub repository. Supports setting title, body, labels, and assignees. Requires GITHUB_TOKEN environment variable with app...",
            schema: build_schema::<github::CreateIssueArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_OR_UPDATE_FILE,
            category: "github",
            description: "Create a new file or update an existing file in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idemp...",
            schema: build_schema::<github::CreateOrUpdateFileArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_PULL_REQUEST,
            category: "github",
            description: "Create a new pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool { f...",
            schema: build_schema::<github::CreatePullRequestArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_PULL_REQUEST_REVIEW,
            category: "github",
            description: "Create a review on a pull request (approve, request changes, or comment). Requires GITHUB_TOKEN environment variable with repo permissions.' } fn r...",
            schema: build_schema::<github::CreatePullRequestReviewArgs>(),
        },
        ToolMetadata {
            name: GITHUB_CREATE_REPOSITORY,
            category: "github",
            description: "Create a new repository under the authenticated user's account' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent...",
            schema: build_schema::<github::CreateRepositoryArgs>(),
        },
        ToolMetadata {
            name: GITHUB_FORK_REPOSITORY,
            category: "github",
            description: "Fork a repository to your account or an organization' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bool...",
            schema: build_schema::<github::ForkRepositoryArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_COMMIT,
            category: "github",
            description: "Get detailed information about a specific commit' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool { tr...",
            schema: build_schema::<github::GetCommitArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_FILE_CONTENTS,
            category: "github",
            description: "Get file or directory contents from a GitHub repository' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bo...",
            schema: build_schema::<github::GetFileContentsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_ISSUE,
            category: "github",
            description: "Fetch a single GitHub issue by number. Returns detailed issue information including title, body, state, labels, assignees, comments count, and time...",
            schema: build_schema::<github::GetIssueArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_ISSUE_COMMENTS,
            category: "github",
            description: "Fetch all comments for a GitHub issue. Returns an array of comment objects including author, body, timestamps, and metadata. Comments are returned ...",
            schema: build_schema::<github::GetIssueCommentsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_ME,
            category: "github",
            description: "Get information about the authenticated GitHub user. Returns user profile details including login, name, email, avatar, bio, company, location, rep...",
            schema: build_schema::<github::GetMeArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_PULL_REQUEST_FILES,
            category: "github",
            description: "Get all files changed in a pull request with their diff stats' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent()...",
            schema: build_schema::<github::GetPullRequestFilesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_PULL_REQUEST_REVIEWS,
            category: "github",
            description: "Get all reviews for a pull request. Shows approval status, requested changes, and comments from reviewers. Requires GITHUB_TOKEN environment variab...",
            schema: build_schema::<github::GetPullRequestReviewsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_GET_PULL_REQUEST_STATUS,
            category: "github",
            description: "Get detailed status information about a pull request including merge status, checks, and review state' } fn read_only() -> bool { true } fn destruc...",
            schema: build_schema::<github::GetPullRequestStatusArgs>(),
        },
        ToolMetadata {
            name: GITHUB_LIST_BRANCHES,
            category: "github",
            description: "List all branches in a repository' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool { true } fn open_wo...",
            schema: build_schema::<github::ListBranchesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_LIST_COMMITS,
            category: "github",
            description: "List commits in a repository with filtering options' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool {...",
            schema: build_schema::<github::ListCommitsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_LIST_ISSUES,
            category: "github",
            description: "List and filter issues in a GitHub repository. Supports filtering by state, labels, assignee, and pagination. Returns an array of issue objects. Re...",
            schema: build_schema::<github::ListIssuesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_MERGE_PULL_REQUEST,
            category: "github",
            description: "Merge a pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { true } fn idempotent() -> bool { false }...",
            schema: build_schema::<github::MergePullRequestArgs>(),
        },
        ToolMetadata {
            name: GITHUB_PUSH_FILES,
            category: "github",
            description: "Push multiple files to a GitHub repository in a single commit. All files are added atomically (creates tree, commit, and updates ref). File content...",
            schema: build_schema::<github::PushFilesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_REQUEST_COPILOT_REVIEW,
            category: "github",
            description: "Request GitHub Copilot to review a pull request (experimental feature). Triggers automated code review from Copilot. Requires GITHUB_TOKEN and Copi...",
            schema: build_schema::<github::RequestCopilotReviewArgs>(),
        },
        ToolMetadata {
            name: GITHUB_SEARCH_CODE,
            category: "github",
            description: "Search code across GitHub repositories using GitHub's code search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn i...",
            schema: build_schema::<github::SearchCodeArgs>(),
        },
        ToolMetadata {
            name: GITHUB_SEARCH_ISSUES,
            category: "github",
            description: "Search for issues across GitHub using GitHub's powerful search syntax. Supports filtering by repository, state, labels, assignee, author, dates, an...",
            schema: build_schema::<github::SearchIssuesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_SEARCH_REPOSITORIES,
            category: "github",
            description: "Search GitHub repositories using GitHub's repository search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempot...",
            schema: build_schema::<github::SearchRepositoriesArgs>(),
        },
        ToolMetadata {
            name: GITHUB_SEARCH_USERS,
            category: "github",
            description: "Search GitHub users using GitHub's user search syntax' } fn read_only() -> bool { true } fn destructive() -> bool { false } fn idempotent() -> bool...",
            schema: build_schema::<github::SearchUsersArgs>(),
        },
        ToolMetadata {
            name: GITHUB_SECRET_SCANNING_ALERTS,
            category: "github",
            description: "List secret scanning alerts (leaked credentials) for a GitHub repository. Returns alerts about exposed secrets like API keys, tokens, passwords, an...",
            schema: build_schema::<github::SecretScanningAlertsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_UPDATE_ISSUE,
            category: "github",
            description: "Update an existing GitHub issue. Supports partial updates - only specified fields will be modified. Can update title, body, state (open/closed), la...",
            schema: build_schema::<github::UpdateIssueArgs>(),
        },
        ToolMetadata {
            name: GITHUB_UPDATE_PULL_REQUEST,
            category: "github",
            description: "Update an existing pull request in a GitHub repository' } fn read_only() -> bool { false } fn destructive() -> bool { false } fn idempotent() -> bo...",
            schema: build_schema::<github::UpdatePullRequestArgs>(),
        },
        ToolMetadata {
            name: GITHUB_LIST_PULL_REQUESTS,
            category: "github",
            description: "List pull requests in a GitHub repository. Supports filtering by state (open, closed, all) and sorting options. Returns PR details including number, title, state, author, and timestamps.",
            schema: build_schema::<github::ListPullRequestsArgs>(),
        },
        ToolMetadata {
            name: GITHUB_DELETE_BRANCH,
            category: "github",
            description: "Delete a branch from a GitHub repository. Removes the branch reference from the remote repository. Cannot delete the default branch.",
            schema: build_schema::<github::DeleteBranchArgs>(),
        },
    ]
}
