use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::SearchRepositoriesArgs;
use octocrab::Octocrab;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

/// Tool for searching GitHub repositories
pub struct SearchRepositoriesTool;

impl Tool for SearchRepositoriesTool {
    type Args = SearchRepositoriesArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "search_repositories"
    }

    fn description() -> &'static str {
        "Search GitHub repositories using GitHub's repository search syntax"
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Create octocrab instance directly
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        let mut request = octocrab.search().repositories(&args.query);

        if let Some(sort_val) = args.sort {
            request = request.sort(&sort_val);
        }

        if let Some(order_val) = args.order {
            request = request.order(&order_val);
        }

        if let Some(p) = args.page {
            request = request.page(p);
        }

        if let Some(pp) = args.per_page {
            request = request.per_page(pp);
        }

        let page = request
            .send()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&page)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Repository Search Examples

## Basic Repository Search
To search for repositories by name or description:

```json
{
  "query": "machine learning",
  "per_page": 20
}
```

## Search by Language
To find repositories in a specific programming language:

```json
{
  "query": "language:rust",
  "sort": "stars",
  "order": "desc",
  "per_page": 30
}
```

## GitHub Repository Search Query Syntax

### Language Filter

**language:name** - Filter by programming language
```json
{
  "query": "language:rust stars:>100"
}
```

Popular languages: rust, javascript, python, go, typescript, java, c++, ruby, php, swift

### Stars Filter

**stars:>n** - Repositories with more than n stars
**stars:<n** - Repositories with fewer than n stars
**stars:n..m** - Repositories with stars in range

```json
{
  "query": "language:rust stars:>1000",
  "sort": "stars"
}
```

```json
{
  "query": "stars:100..500 language:python"
}
```

### Forks Filter

**forks:>n** - Repositories with more than n forks
**forks:<n** - Repositories with fewer than n forks
**forks:n..m** - Repositories with forks in range

```json
{
  "query": "language:javascript forks:>100"
}
```

### Date Filters

**created:>YYYY-MM-DD** - Created after date
**created:<YYYY-MM-DD** - Created before date
**pushed:>YYYY-MM-DD** - Updated after date
**pushed:<YYYY-MM-DD** - Updated before date

```json
{
  "query": "language:rust created:>2024-01-01",
  "sort": "stars"
}
```

```json
{
  "query": "pushed:>2024-06-01 stars:>100"
}
```

### Topic Filter

**topic:name** - Repositories with specific topic
```json
{
  "query": "topic:async language:rust"
}
```

```json
{
  "query": "topic:machine-learning topic:python"
}
```

### User and Organization Filters

**user:username** - Repositories owned by user
**org:orgname** - Repositories owned by organization

```json
{
  "query": "user:octocat language:ruby"
}
```

```json
{
  "query": "org:github topic:ai",
  "sort": "updated"
}
```

### Combining Multiple Filters

Find popular async Rust libraries:
```json
{
  "query": "language:rust stars:>100 topic:async",
  "sort": "stars",
  "order": "desc",
  "per_page": 20
}
```

Find recently updated Python ML projects:
```json
{
  "query": "language:python topic:machine-learning pushed:>2024-01-01",
  "sort": "updated",
  "order": "desc"
}
```

Find active projects in an organization:
```json
{
  "query": "org:github stars:>50 pushed:>2024-06-01",
  "sort": "stars"
}
```

## Sort Options

**stars** - Sort by number of stars (most popular)
**forks** - Sort by number of forks (most forked)
**updated** - Sort by last update date (most recently updated)

```json
{
  "query": "language:rust",
  "sort": "stars",
  "order": "desc"
}
```

## Order Options

**asc** - Ascending order (least to most)
**desc** - Descending order (most to least)

## Response Information

The response includes:
- **total_count**: Total number of matching repositories
- **incomplete_results**: Whether the search timed out
- **items**: Array of repository objects

Each repository object contains:
- **id**: Unique repository ID
- **name**: Repository name
- **full_name**: Owner/repo format
- **description**: Repository description
- **html_url**: Web URL to the repository
- **stargazers_count**: Number of stars
- **forks_count**: Number of forks
- **language**: Primary programming language
- **topics**: Array of repository topics
- **created_at**: Creation date
- **updated_at**: Last update date
- **pushed_at**: Last push date

## Pagination

- Default per_page is 30 repositories
- Maximum per_page is 100
- Use page parameter to navigate through results
- Check total_count for total matches

## Common Use Cases

1. **Discover Libraries**: Find popular libraries in your language
2. **Research Tools**: Find tools for specific tasks
3. **Find Examples**: Discover example projects and implementations
4. **Track Trends**: Monitor emerging projects and technologies
5. **Competitor Analysis**: Research similar projects
6. **Open Source Discovery**: Find projects to contribute to
7. **Technology Assessment**: Evaluate adoption of technologies

## Example Workflows

### Find Popular Rust Web Frameworks
```json
{
  "query": "language:rust topic:web stars:>500",
  "sort": "stars",
  "order": "desc"
}
```

### Find Active Python Projects
```json
{
  "query": "language:python pushed:>2024-01-01 stars:>100",
  "sort": "updated",
  "order": "desc"
}
```

### Find Beginner-Friendly Projects
```json
{
  "query": "good-first-issue language:javascript stars:50..500",
  "sort": "updated"
}
```

### Find Organization Repositories
```json
{
  "query": "org:rust-lang topic:compiler",
  "sort": "stars"
}
```

## Best Practices

- **Use specific filters**: Combine multiple filters to narrow results
- **Sort appropriately**: Use "stars" for popularity, "updated" for activity
- **Filter by language**: Dramatically improves result relevance
- **Use date filters**: Find actively maintained projects
- **Check topics**: Topics provide better categorization than text search
- **Consider forks**: High fork count indicates usefulness
- **Look at update dates**: Avoid abandoned projects
- **Use star ranges**: Find projects at your experience level

## Search Tips

- Search repository names: Just use the text without qualifiers
- Combine with text search: `machine learning language:python`
- Use topic: for precise categorization
- Filter archived repos: Add `archived:false` to exclude archived
- Find templates: Search for `template` or `boilerplate`
- Discover trending: Use `created:>YYYY-MM-DD` for new projects
- Find maintained projects: Use `pushed:>YYYY-MM-DD` for recent activity
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
