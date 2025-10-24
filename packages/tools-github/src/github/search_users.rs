//! GitHub user search operation with type-safe parameters.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::Author};
use std::sync::Arc;

/// Sort field for user search results.
///
/// Determines the primary field used to order search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserSearchSort {
    /// Sort by follower count (most to least)
    Followers,
    /// Sort by public repository count
    Repositories,
    /// Sort by account creation date
    Joined,
}

impl UserSearchSort {
    /// Returns the GitHub API string representation of this sort field.
    ///
    /// This is a zero-cost conversion that returns a static string slice.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Followers => "followers",
            Self::Repositories => "repositories",
            Self::Joined => "joined",
        }
    }
}

/// Sort order for search results.
///
/// Controls whether results are returned in ascending or descending order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOrder {
    /// Ascending order (low to high, old to new)
    Asc,
    /// Descending order (high to low, new to old)
    Desc,
}

impl SearchOrder {
    /// Returns the GitHub API string representation of this sort order.
    ///
    /// This is a zero-cost conversion that returns a static string slice.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Search for GitHub users matching a query.
///
/// This function provides type-safe access to the GitHub user search API with
/// comprehensive input validation and zero-allocation parameter handling.
///
/// # Arguments
///
/// * `client` - GitHub client instance
/// * `query` - Search query using GitHub's search syntax (cannot be empty)
/// * `sort` - Optional field to sort results by
/// * `order` - Optional sort order (ascending or descending)
/// * `page` - Optional page number for pagination (must be >= 1)
/// * `per_page` - Optional results per page (must be between 1 and 100)
///
/// # Query Syntax
///
/// The query supports GitHub's advanced search syntax:
/// - `in:login` - Search in username
/// - `in:name` - Search in display name  
/// - `in:email` - Search in email address
/// - `type:user` - Only match user accounts
/// - `type:org` - Only match organization accounts
/// - `repos:>N` - Users with more than N repositories
/// - `followers:>N` - Users with more than N followers
/// - `language:rust` - Users with repositories in a language
/// - `location:seattle` - Users in a location
///
/// # Errors
///
/// Returns `GitHubError::InvalidInput` if:
/// - Query string is empty
/// - Page number is less than 1
/// - Per-page value is not between 1 and 100
///
/// Returns `GitHubError::Octocrab` if the GitHub API request fails.
///
/// # Performance
///
/// This function is marked `#[inline]` for optimal performance when called
/// from hot paths. The enum parameters use zero-allocation conversions to
/// static string slices.
///
/// # Example
///
/// ```rust,no_run
/// use gitgix::{GitHubClient, UserSearchSort, SearchOrder};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = GitHubClient::with_token("your-token")?;
///
/// // Search for Rust developers with many repositories
/// let results = client.search_users(
///     "language:rust repos:>50",
///     Some(UserSearchSort::Repositories),
///     Some(SearchOrder::Desc),
///     Some(1),
///     Some(50),
/// ).await??;
///
/// for user in results.items {
///     println!("{}: {} repos", user.login, user.public_repos.unwrap_or(0));
/// }
/// # Ok(())
/// # }
/// ```
#[inline]
pub(crate) fn search_users(
    inner: Arc<Octocrab>,
    query: impl Into<String>,
    sort: Option<UserSearchSort>,
    order: Option<SearchOrder>,
    page: Option<u32>,
    per_page: Option<u8>,
) -> AsyncTask<Result<octocrab::Page<Author>, GitHubError>> {
    let query = query.into();

    spawn_task(async move {
        // Validate query is not empty
        if query.is_empty() {
            return Err(GitHubError::InvalidInput(
                "search query cannot be empty".into(),
            ));
        }

        // Validate page number if provided
        if let Some(p) = page
            && p < 1
        {
            return Err(GitHubError::InvalidInput("page must be >= 1".into()));
        }

        // Validate per_page is within GitHub API limits
        if let Some(pp) = per_page
            && !(1..=100).contains(&pp)
        {
            return Err(GitHubError::InvalidInput(
                "per_page must be between 1 and 100".into(),
            ));
        }

        let mut request = inner.search().users(&query);

        if let Some(s) = sort {
            request = request.sort(s.as_str());
        }

        if let Some(o) = order {
            request = request.order(o.as_str());
        }

        if let Some(p) = page {
            request = request.page(p);
        }

        if let Some(pp) = per_page {
            request = request.per_page(pp);
        }

        let results = request.send().await.map_err(GitHubError::from)?;

        Ok(results)
    })
}
