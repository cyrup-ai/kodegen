//! GitHub API client wrapper
//!
//! Provides clean API for GitHub operations without exposing Octocrab.
//!
//! # Examples
//!
//! ```rust,no_run
//! use gitgix::GitHubClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let gh = GitHubClient::with_token("ghp_...")?;
//!
//!     // Use with any GitHub operation
//!     let issue = gitgix::create_issue(
//!         gh,
//!         "owner",
//!         "repo",
//!         "Issue title",
//!         None, None, None
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::github::error::{GitHubError, GitHubResult};
use jsonwebtoken::EncodingKey;
use octocrab::{Octocrab, models::AppId};
use std::sync::Arc;

/// GitHub API client wrapper that encapsulates Octocrab.
///
/// Provides clean API without exposing Octocrab dependency.
/// Cloning is cheap (Arc clone).
#[derive(Clone, Debug)]
pub struct GitHubClient {
    inner: Arc<Octocrab>,
}

impl GitHubClient {
    /// Create a new client builder
    #[must_use]
    pub fn builder() -> GitHubClientBuilder {
        GitHubClientBuilder::new()
    }

    /// Convenience: create client with personal access token
    pub fn with_token(token: impl Into<String>) -> GitHubResult<Self> {
        Self::builder().personal_token(token).build()
    }

    /// Get inner Octocrab client
    #[must_use]
    pub fn inner(&self) -> &Arc<Octocrab> {
        &self.inner
    }

    // ========================================================================
    // Issues
    // ========================================================================

    /// Get a single issue
    pub fn get_issue(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::get_issue::get_issue(self.inner.clone(), owner, repo, issue_number)
    }

    /// Create a new issue
    pub fn create_issue(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        title: impl Into<String>,
        body: Option<String>,
        assignees: Option<Vec<String>>,
        labels: Option<Vec<String>>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::create_issue::create_issue(
            self.inner.clone(),
            owner,
            repo,
            title,
            body,
            assignees,
            labels,
        )
    }

    /// Add a comment to an issue
    pub fn add_issue_comment(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
        body: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Comment, GitHubError>> {
        crate::github::add_issue_comment::add_issue_comment(
            self.inner.clone(),
            owner,
            repo,
            issue_number,
            body,
        )
    }

    /// Get comments for an issue
    pub fn get_issue_comments(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        issue_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Comment, GitHubError>> {
        crate::github::get_issue_comments::get_issue_comments(
            self.inner.clone(),
            owner,
            repo,
            issue_number,
        )
    }

    /// List issues with filters
    #[must_use]
    pub fn list_issues(
        &self,
        request: crate::github::ListIssuesRequest,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::list_issues::list_issues(self.inner.clone(), request)
    }

    /// Update an issue
    #[must_use]
    pub fn update_issue(
        &self,
        request: crate::github::UpdateIssueRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::update_issue::update_issue(self.inner.clone(), request)
    }

    /// Search issues
    pub fn search_issues(
        &self,
        query: impl Into<String>,
        sort: Option<String>,
        order: Option<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::issues::Issue, GitHubError>> {
        crate::github::search_issues::search_issues(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
        )
    }

    // ========================================================================
    // Pull Requests
    // ========================================================================

    /// Create a pull request
    #[must_use]
    pub fn create_pull_request(
        &self,
        request: crate::github::CreatePullRequestRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::PullRequest, GitHubError>> {
        crate::github::create_pull_request::create_pull_request(self.inner.clone(), request)
    }

    /// Get pull request status
    pub fn get_pull_request_status(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::CombinedStatus, GitHubError>> {
        crate::github::get_pull_request_status::get_pull_request_status(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request comments
    pub fn get_pull_request_comments(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::pulls::Comment, GitHubError>> {
        crate::github::get_pull_request_comments::get_pull_request_comments(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request files
    pub fn get_pull_request_files(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::repos::DiffEntry, GitHubError>> {
        crate::github::get_pull_request_files::get_pull_request_files(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Get pull request reviews
    pub fn get_pull_request_reviews(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncStream<Result<octocrab::models::pulls::Review, GitHubError>> {
        crate::github::get_pull_request_reviews::get_pull_request_reviews(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }

    /// Create a pull request review
    pub fn create_pull_request_review(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::CreatePullRequestReviewOptions,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::Review, GitHubError>> {
        crate::github::create_pull_request_review::create_pull_request_review(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }

    /// Add a review comment to a pull request
    #[must_use]
    pub fn add_pull_request_review_comment(
        &self,
        request: crate::github::AddPullRequestReviewCommentRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::ReviewComment, GitHubError>>
    {
        crate::github::add_pull_request_review_comment::add_pull_request_review_comment(
            self.inner.clone(),
            request,
        )
    }

    /// Update a pull request
    pub fn update_pull_request(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::UpdatePullRequestOptions,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::pulls::PullRequest, GitHubError>> {
        crate::github::update_pull_request::update_pull_request(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }

    /// Merge a pull request
    pub fn merge_pull_request(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
        options: crate::github::MergePullRequestOptions,
    ) -> crate::runtime::AsyncTask<Result<serde_json::Value, GitHubError>> {
        crate::github::merge_pull_request::merge_pull_request(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
            options,
        )
    }

    // ========================================================================
    // Repositories
    // ========================================================================

    /// Get file contents
    pub fn get_file_contents(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        path: impl Into<String>,
        ref_name: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::Content>, GitHubError>> {
        crate::github::get_file_contents::get_file_contents(
            self.inner.clone(),
            owner,
            repo,
            path,
            ref_name,
        )
    }

    /// Create or update a file
    #[must_use]
    pub fn create_or_update_file(
        &self,
        request: crate::github::CreateOrUpdateFileRequest,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::FileUpdate, GitHubError>> {
        crate::github::create_or_update_file::create_or_update_file(self.inner.clone(), request)
    }

    /// List branches
    pub fn list_branches(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::Branch>, GitHubError>> {
        crate::github::list_branches::list_branches(self.inner.clone(), owner, repo, page, per_page)
    }

    /// Create a branch
    pub fn create_branch(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        branch_name: impl Into<String>,
        sha: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::Ref, GitHubError>> {
        crate::github::create_branch::create_branch(
            self.inner.clone(),
            owner,
            repo,
            branch_name,
            sha,
        )
    }

    /// List commits
    pub fn list_commits(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        options: crate::github::ListCommitsOptions,
    ) -> crate::runtime::AsyncTask<Result<Vec<octocrab::models::repos::RepoCommit>, GitHubError>>
    {
        crate::github::list_commits::list_commits(self.inner.clone(), owner, repo, options)
    }

    /// Get a commit
    pub fn get_commit(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        commit_sha: impl Into<String>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::RepoCommit, GitHubError>> {
        crate::github::get_commit::get_commit(
            self.inner.clone(),
            owner,
            repo,
            commit_sha,
            page,
            per_page,
        )
    }

    /// Search code
    pub fn search_code(
        &self,
        query: impl Into<String>,
        sort: Option<String>,
        order: Option<String>,
        page: Option<u32>,
        per_page: Option<u8>,
        enrich_stars: bool,
    ) -> crate::runtime::AsyncTask<Result<octocrab::Page<octocrab::models::Code>, GitHubError>>
    {
        crate::github::search_code::search_code(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
            enrich_stars,
        )
    }

    /// Create a repository
    pub fn create_repository(
        &self,
        name: impl Into<String>,
        description: Option<String>,
        private: Option<bool>,
        auto_init: Option<bool>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Repository, GitHubError>> {
        crate::github::create_repository::create_repository(
            self.inner.clone(),
            name,
            description,
            private,
            auto_init,
        )
    }

    /// Fork a repository
    pub fn fork_repository(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        organization: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Repository, GitHubError>> {
        crate::github::fork_repository::fork_repository(
            self.inner.clone(),
            owner,
            repo,
            organization,
        )
    }

    /// Push files to a repository
    pub fn push_files(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        branch: impl Into<String>,
        files: std::collections::HashMap<String, String>,
        commit_message: impl Into<String>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::repos::Commit, GitHubError>> {
        crate::github::push_files::push_files(
            self.inner.clone(),
            owner,
            repo,
            branch,
            files,
            commit_message,
        )
    }

    // ========================================================================
    // Users
    // ========================================================================

    /// Get the authenticated user
    #[must_use]
    pub fn get_me(
        &self,
    ) -> crate::runtime::AsyncTask<Result<octocrab::models::Author, GitHubError>> {
        crate::github::get_me::get_me(self.inner.clone())
    }

    /// Search users
    pub fn search_users(
        &self,
        query: impl Into<String>,
        sort: Option<crate::github::search_users::UserSearchSort>,
        order: Option<crate::github::search_users::SearchOrder>,
        page: Option<u32>,
        per_page: Option<u8>,
    ) -> crate::runtime::AsyncTask<Result<octocrab::Page<octocrab::models::Author>, GitHubError>>
    {
        crate::github::search_users::search_users(
            self.inner.clone(),
            query,
            sort,
            order,
            page,
            per_page,
        )
    }

    // ========================================================================
    // Security
    // ========================================================================

    /// List code scanning alerts
    pub fn list_code_scanning_alerts(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        state: Option<String>,
        ref_name: Option<String>,
        tool_name: Option<String>,
        severity: Option<String>,
    ) -> crate::runtime::AsyncTask<Result<Vec<serde_json::Value>, GitHubError>> {
        crate::github::code_scanning_alerts::list_code_scanning_alerts(
            self.inner.clone(),
            owner,
            repo,
            state,
            ref_name,
            tool_name,
            severity,
        )
    }

    /// Get a code scanning alert
    pub fn get_code_scanning_alert(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        alert_number: u64,
    ) -> crate::runtime::AsyncTask<Result<serde_json::Value, GitHubError>> {
        crate::github::code_scanning_alerts::get_code_scanning_alert(
            self.inner.clone(),
            owner,
            repo,
            alert_number,
        )
    }

    /// List secret scanning alerts
    pub fn list_secret_scanning_alerts(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        state: Option<String>,
        secret_type: Option<String>,
        resolution: Option<String>,
    ) -> crate::runtime::AsyncTask<
        Result<
            Vec<octocrab::models::repos::secret_scanning_alert::SecretScanningAlert>,
            GitHubError,
        >,
    > {
        crate::github::secret_scanning_alerts::list_secret_scanning_alerts(
            self.inner.clone(),
            owner,
            repo,
            state,
            secret_type,
            resolution,
        )
    }

    /// Get a secret scanning alert
    pub fn get_secret_scanning_alert(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        alert_number: u32,
    ) -> crate::runtime::AsyncTask<
        Result<octocrab::models::repos::secret_scanning_alert::SecretScanningAlert, GitHubError>,
    > {
        crate::github::secret_scanning_alerts::get_secret_scanning_alert(
            self.inner.clone(),
            owner,
            repo,
            alert_number,
        )
    }

    // ========================================================================
    // Release Assets
    // ========================================================================

    /// Upload an asset to a release
    ///
    /// Requires the release ID and binary content of the file.
    /// Returns the uploaded asset information including download URL.
    pub async fn upload_release_asset(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        options: crate::github::upload_release_asset::UploadAssetOptions,
    ) -> Result<octocrab::models::repos::Asset, crate::github::error::GitHubError> {
        crate::github::upload_release_asset::upload_release_asset(
            self.inner.clone(),
            &owner.into(),
            &repo.into(),
            options,
        )
        .await
        .map_err(crate::github::error::GitHubError::from)
    }

    /// Delete a release asset
    pub async fn delete_release_asset(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        asset_id: u64,
    ) -> Result<(), crate::github::error::GitHubError> {
        crate::github::upload_release_asset::delete_release_asset(
            self.inner.clone(),
            &owner.into(),
            &repo.into(),
            asset_id,
        )
        .await
        .map_err(crate::github::error::GitHubError::from)
    }

    // ========================================================================
    // Experimental
    // ========================================================================

    /// Request a Copilot review
    pub fn request_copilot_review(
        &self,
        owner: impl Into<String>,
        repo: impl Into<String>,
        pr_number: u64,
    ) -> crate::runtime::AsyncTask<Result<(), GitHubError>> {
        crate::github::request_copilot_review::request_copilot_review(
            self.inner.clone(),
            owner,
            repo,
            pr_number,
        )
    }
}

/// Builder for creating `GitHubClient` with various authentication methods
pub struct GitHubClientBuilder {
    token: Option<String>,
    app_auth: Option<(AppId, String)>,
    base_uri: Option<String>,
}

impl GitHubClientBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: None,
            app_auth: None,
            base_uri: None,
        }
    }

    /// Set personal access token for authentication
    pub fn personal_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set GitHub App authentication (app ID and private key)
    pub fn app(mut self, app_id: AppId, private_key: impl Into<String>) -> Self {
        self.app_auth = Some((app_id, private_key.into()));
        self
    }

    /// Set base URI (for GitHub Enterprise)
    pub fn base_uri(mut self, uri: impl Into<String>) -> Self {
        self.base_uri = Some(uri.into());
        self
    }

    /// Build the `GitHubClient`
    pub fn build(self) -> GitHubResult<GitHubClient> {
        let mut builder = Octocrab::builder();

        // Set authentication
        if let Some(token) = self.token {
            builder = builder.personal_token(token);
        } else if let Some((app_id, private_key)) = self.app_auth {
            let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
                .map_err(|e| GitHubError::ClientSetup(format!("Invalid RSA key: {e}")))?;
            builder = builder.app(app_id, key);
        }

        // Set base URI if provided
        if let Some(uri) = self.base_uri {
            builder = builder
                .base_uri(&uri)
                .map_err(|e| GitHubError::ClientSetup(e.to_string()))?;
        }

        // Build Octocrab instance
        let octocrab = builder
            .build()
            .map_err(|e| GitHubError::ClientSetup(e.to_string()))?;

        Ok(GitHubClient {
            inner: Arc::new(octocrab),
        })
    }
}

impl Default for GitHubClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
