use anyhow;
use kodegen_mcp_schema::github::SearchUsersArgs;
use kodegen_mcp_tool::{McpError, Tool};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for searching GitHub users
pub struct SearchUsersTool;

impl Tool for SearchUsersTool {
    type Args = SearchUsersArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "search_users"
    }

    fn description() -> &'static str {
        "Search GitHub users using GitHub's user search syntax"
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

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert sort string to UserSearchSort enum
        let sort_enum = if let Some(s) = args.sort.as_ref() {
            match s.as_str() {
                "followers" => Some(crate::github::UserSearchSort::Followers),
                "repositories" => Some(crate::github::UserSearchSort::Repositories),
                "joined" => Some(crate::github::UserSearchSort::Joined),
                _ => {
                    return Err(McpError::InvalidArguments(
                        "sort must be followers, repositories, or joined".into(),
                    ));
                }
            }
        } else {
            None
        };

        // Convert order string to SearchOrder enum
        let order_enum = if let Some(o) = args.order.as_ref() {
            match o.as_str() {
                "asc" => Some(crate::github::SearchOrder::Asc),
                "desc" => Some(crate::github::SearchOrder::Desc),
                _ => {
                    return Err(McpError::InvalidArguments(
                        "order must be asc or desc".into(),
                    ));
                }
            }
        } else {
            None
        };

        let task_result = client
            .search_users(args.query, sort_enum, order_enum, args.page, args.per_page)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let page =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&page)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub User Search Examples

## Basic User Search
To search for users by username or name:

```json
{
  "query": "tom",
  "per_page": 20
}
```

## Search by Location and Language
To find Rust developers in San Francisco:

```json
{
  "query": "location:\"San Francisco\" language:rust",
  "sort": "followers",
  "order": "desc"
}
```

## GitHub User Search Query Syntax

### Location Filter

**location:place** - Filter by user location
```json
{
  "query": "location:London language:python"
}
```

```json
{
  "query": "location:\"New York\" language:javascript"
}
```

### Language Filter

**language:name** - Filter by programming language in repositories
```json
{
  "query": "language:rust followers:>100"
}
```

### Follower and Repository Filters

**followers:>n** - Users with more than n followers
**followers:<n** - Users with fewer than n followers
**followers:n..m** - Users with followers in range

```json
{
  "query": "followers:1000..5000 language:go"
}
```

**repos:>n** - Users with more than n public repositories
**repos:<n** - Users with fewer than n repositories

```json
{
  "query": "repos:>50 language:rust"
}
```

### Company and Email Filters

**in:email** - Search in email addresses
**in:login** - Search in usernames
**in:name** - Search in names

```json
{
  "query": "john in:name location:Seattle"
}
```

### Type Filter

**type:user** - Search for users only
**type:org** - Search for organizations only

```json
{
  "query": "type:org location:\"San Francisco\""
}
```

### Combining Filters

Find influential Rust developers:
```json
{
  "query": "language:rust followers:>500",
  "sort": "followers",
  "order": "desc"
}
```

Find active developers in a location:
```json
{
  "query": "location:Berlin repos:>20 language:javascript",
  "sort": "repositories",
  "order": "desc"
}
```

## Sort Options

**followers** - Sort by number of followers
- **Use when:** Finding influential users or potential collaborators
- **Best with:** Language and location filters
```json
{
  "query": "language:python location:Tokyo",
  "sort": "followers",
  "order": "desc"
}
```

**repositories** - Sort by number of public repositories
- **Use when:** Finding active contributors or prolific developers
- **Best with:** Language filters
```json
{
  "query": "language:rust repos:>10",
  "sort": "repositories",
  "order": "desc"
}
```

**joined** - Sort by account creation date
- **Use when:** Finding new users or oldest accounts
- **Best with:** Location or language filters
```json
{
  "query": "language:go followers:>100",
  "sort": "joined",
  "order": "asc"
}
```

## Order Options

**asc** - Ascending order (least to most, oldest to newest)
**desc** - Descending order (most to least, newest to oldest)

## Response Information

The response includes:
- **total_count**: Total number of matching users
- **incomplete_results**: Whether the search timed out
- **items**: Array of user objects

Each user object contains:
- **id**: Unique user ID
- **login**: GitHub username
- **avatar_url**: Profile picture URL
- **html_url**: Profile page URL
- **type**: "User" or "Organization"
- **site_admin**: Whether user is a GitHub admin

## Pagination

- Default per_page is 30 users
- Maximum per_page is 100
- Use page parameter to navigate through results
- Check total_count for total matches

## Common Use Cases

1. **Find Contributors**: Search for developers with specific skills
2. **Recruitment**: Find developers in specific locations with desired skills
3. **Community Building**: Discover users interested in specific technologies
4. **Influencer Discovery**: Find users with large followings in your domain
5. **Network Growth**: Connect with developers working in similar areas
6. **Conference Planning**: Find speakers or attendees by location
7. **Open Source**: Find potential collaborators for projects

## Example Workflows

### Find Python Experts in New York
```json
{
  "query": "location:\"New York\" language:python followers:>100",
  "sort": "followers",
  "order": "desc",
  "per_page": 50
}
```

### Find Active Rust Contributors
```json
{
  "query": "language:rust repos:>20",
  "sort": "repositories",
  "order": "desc"
}
```

### Find Organizations in Silicon Valley
```json
{
  "query": "type:org location:\"Silicon Valley\"",
  "per_page": 30
}
```

### Find New Developers Learning Go
```json
{
  "query": "language:go repos:1..10",
  "sort": "joined",
  "order": "desc"
}
```

## Best Practices

- **Combine filters**: Use language + location for targeted results
- **Use quotes**: Wrap multi-word locations in quotes
- **Sort appropriately**: Choose sort based on your goal
  - followers: For influence and expertise
  - repositories: For activity and contributions
  - joined: For new or veteran users
- **Set follower minimums**: Filter out inactive accounts
- **Use repo count ranges**: Find users at specific experience levels
- **Check user profiles**: Verify results match your needs
- **Respect privacy**: Only use publicly available information

## Search Tips

- Location names can be approximate (city, state, country)
- Language reflects primary languages in public repositories
- Follower count indicates community recognition
- Repository count shows activity level
- Combine type:user or type:org to filter result types
- Use in:name for searching by actual names
- Empty results may mean no users match all criteria
- Broaden search if too few results
- Narrow search if too many irrelevant results
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
