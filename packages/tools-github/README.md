# kodegen_github

GitHub API operations via Octocrab with MCP tool wrappers for AI agents.

## Features

- **Async-first**: Built on tokio for efficient async operations
- **Clean API**: Wraps octocrab with ergonomic interfaces
- **MCP Tools**: 12 GitHub tools for AI agent integration (7 issue + 5 pull request tools)
- **Type-safe**: Full Rust type safety throughout
- **Streaming**: Efficient streaming for large result sets

## MCP Tools

### Issue Operations

#### create_issue

Create a new issue in a GitHub repository.

**Arguments:**
- `owner` (string): Repository owner (user or organization)
- `repo` (string): Repository name
- `title` (string): Issue title
- `body` (string, optional): Issue description (Markdown supported)
- `labels` (array<string>, optional): Labels to apply
- `assignees` (array<string>, optional): Users to assign

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "title": "Bug: Login fails",
  "body": "When I try to login, the form doesn't submit...",
  "labels": ["bug", "priority-high"],
  "assignees": ["octocat"]
}
```

**Requirements:**
- GITHUB_TOKEN environment variable must be set
- Token needs 'repo' scope for private repos, 'public_repo' for public
- User must have write access to the repository
- Labels must already exist in the repository
- Assignees must be collaborators on the repository

---

#### get_issue

Fetch a single issue by number.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `issue_number` (number): Issue number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "issue_number": 42
}
```

**Returns:**
- `number`: Issue number
- `title`: Issue title
- `body`: Issue description
- `state`: "open" or "closed"
- `labels`: Array of label objects
- `assignees`: Array of assigned users
- `created_at`: Creation timestamp
- `updated_at`: Last update timestamp
- `comments`: Number of comments
- `html_url`: Link to issue on GitHub

**Note:** `issue_number` is the issue number (e.g., #42), NOT the internal ID. Works for both issues and pull requests.

---

#### list_issues

List and filter repository issues.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `state` (string, optional): "open" (default), "closed", or "all"
- `labels` (array<string>, optional): Filter by labels (AND logic)
- `assignee` (string, optional): Filter by assignee username
- `page` (number, optional): Page number for pagination
- `per_page` (number, optional): Results per page (max 100, default 30)

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "state": "open",
  "labels": ["bug"],
  "per_page": 50
}
```

**Filter behavior:**
- Multiple labels match issues with ALL labels (AND logic, not OR)
- State defaults to "open" if not specified
- Returns issues in most recent order

---

#### update_issue

Update an existing issue.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `issue_number` (number): Issue number
- `title` (string, optional): New title
- `body` (string, optional): New body
- `state` (string, optional): "open" or "closed"
- `labels` (array<string>, optional): Replace labels
- `assignees` (array<string>, optional): Replace assignees

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "issue_number": 42,
  "state": "closed",
  "labels": ["bug", "resolved"]
}
```

**Important notes:**
- All fields are optional - only specified fields are updated
- `labels` and `assignees` REPLACE existing values (not additive)
- To clear labels or assignees, pass empty array: []
- Set `state` to "closed" to close an issue, "open" to reopen

---

#### search_issues

Search issues across GitHub using GitHub's search syntax.

**Arguments:**
- `query` (string): GitHub search query (supports complex syntax)
- `sort` (string, optional): "comments", "reactions", "created", "updated"
- `order` (string, optional): "asc" or "desc"
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page (max 100)

**Search Query Syntax:**
- `repo:owner/repo` - Search in specific repository
- `is:open` / `is:closed` - Filter by state
- `label:bug` - Filter by label
- `assignee:username` - Filter by assignee
- `author:username` - Filter by author
- `involves:username` - User is author, assignee, or mentioned
- `created:>=2024-01-01` - Created after date
- `created:2024-01-01..2024-12-31` - Date range
- Text without prefix searches title and body

**Example:**
```json
{
  "query": "repo:octocat/hello-world is:open label:bug created:>=2024-01-01",
  "sort": "created",
  "order": "desc",
  "per_page": 20
}
```

**Combined filters example:**
```
repo:octocat/hello-world is:open label:bug assignee:alice created:>=2024-01-01
```

**Important notes:**
- Search API has stricter rate limits (30 requests/minute authenticated)
- Results are relevance-ranked by default
- Use quotes for multi-word searches: "bug report"
- Date format: YYYY-MM-DD

---

#### add_issue_comment

Add a comment to an existing issue.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `issue_number` (number): Issue number
- `body` (string): Comment text (Markdown supported)

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "issue_number": 42,
  "body": "This has been fixed in the latest release."
}
```

**Markdown features:**
- Full Markdown support (headings, code blocks, lists, etc.)
- @mention users to notify them
- Reference other issues/PRs with #number
- Link commits with SHA hashes
- Add emojis with :emoji_name:

**Important notes:**
- This tool CREATES a new comment each time (not idempotent)
- Cannot edit existing comments (separate tool needed)
- Works for both issues and pull requests

---

#### get_issue_comments

Fetch all comments for an issue.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `issue_number` (number): Issue number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "issue_number": 42
}
```

**Returns:**
Array of comment objects with:
- `id`: Comment ID
- `body`: Comment text (Markdown)
- `user`: Author information (login, avatar_url, etc.)
- `created_at`: When comment was created
- `updated_at`: When comment was last edited
- `html_url`: Link to comment on GitHub
- `author_association`: Relationship to repo (OWNER, CONTRIBUTOR, etc.)

**Comment ordering:**
- Comments are returned in chronological order (oldest first)
- Use `created_at` timestamp to determine comment age
- Check `author_association` to see if author is repo owner/maintainer
- `updated_at` differs from `created_at` if comment was edited

---

### Pull Request Operations

#### create_pull_request

Create a new pull request in a GitHub repository.

**Arguments:**
- `owner` (string): Repository owner (user or organization)
- `repo` (string): Repository name
- `title` (string): Pull request title
- `body` (string, optional): Pull request description (Markdown supported)
- `head` (string): Name of the branch where changes are implemented (head branch)
- `base` (string): Name of the branch to merge into (base branch)
- `draft` (boolean, optional): Create as draft PR (default: false)
- `maintainer_can_modify` (boolean, optional): Allow maintainer edits (default: true)

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "title": "Add new feature",
  "body": "This PR adds...\n\nCloses #123",
  "head": "feature-branch",
  "base": "main",
  "draft": false
}
```

**Cross-fork PRs:**
For PRs from a fork, use format: `fork-owner:branch-name` for head:
```json
{
  "owner": "upstream-owner",
  "repo": "project",
  "title": "Fix authentication bug",
  "head": "fork-owner:fix-auth-bug",
  "base": "main"
}
```

**Requirements:**
- GITHUB_TOKEN with appropriate permissions
- Head branch must exist
- Base branch must exist
- User must have push access to head repository
- User needs write access for non-fork PRs

---

#### update_pull_request

Update an existing pull request.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pr_number` (number): Pull request number
- `title` (string, optional): New title
- `body` (string, optional): New description
- `state` (string, optional): "open" or "closed"
- `base` (string, optional): New base branch
- `maintainer_can_modify` (boolean, optional): Allow maintainer edits

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "title": "Updated: Add new feature",
  "state": "open"
}
```

**Important notes:**
- All fields are optional except owner, repo, and pr_number
- Only specified fields are updated
- Changing base branch retargets the PR
- Set state to "closed" to close without merging

---

#### merge_pull_request

Merge a pull request in a GitHub repository.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pr_number` (number): Pull request number
- `commit_title` (string, optional): Merge commit title
- `commit_message` (string, optional): Merge commit message
- `merge_method` (string, optional): "merge", "squash", or "rebase"
- `sha` (string, optional): SHA of PR head for safety check

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "merge_method": "squash",
  "commit_title": "Add authentication feature"
}
```

**Merge methods:**
- `merge`: Creates merge commit, preserves all commits
- `squash`: Combines all commits into one
- `rebase`: Rebases commits onto base branch

**Safety:**
Use `sha` parameter to ensure PR hasn't changed:
```json
{
  "pr_number": 42,
  "sha": "6dcb09b5b57875f334f61aebed695e2e4193db5e"
}
```

**Requirements:**
- PR must be in mergeable state
- All required checks must pass
- Sufficient review approvals
- No merge conflicts
- User must have write access

**Warning:** This is a destructive operation that modifies the base branch.

---

#### get_pull_request_status

Get detailed status information about a pull request.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pr_number` (number): Pull request number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

**Returns comprehensive status:**
- Basic info: number, title, state, author
- Merge status: mergeable, merge conflicts
- Base/head branches and SHAs
- CI/CD check status and results
- Review state (approved, changes requested, pending)
- Labels and assignees
- Draft status
- Mergeable state (clean, dirty, blocked, unstable, etc.)

**Use cases:**
- Pre-merge validation
- CI/CD monitoring
- Review requirement checks
- Conflict detection
- Workflow automation
- Status dashboards

**Important fields:**
- `mergeable`: true/false/null (null = still calculating)
- `mergeable_state`: "clean", "dirty", "blocked", "unstable"
- `draft`: true if still in draft mode
- `merged`: true if already merged

---

#### get_pull_request_files

Get all files changed in a pull request with diff stats.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pr_number` (number): Pull request number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

**Returns for each file:**
- `filename`: Path to the file
- `status`: "added", "modified", "removed", "renamed", "copied"
- `additions`: Lines added
- `deletions`: Lines deleted
- `changes`: Total changes (additions + deletions)
- `patch`: Actual diff content
- `blob_url`: URL to view file
- `raw_url`: URL to download raw file
- `previous_filename`: Original name (for renamed files)

**Response format:**
```json
{
  "files": [
    {
      "filename": "src/main.rs",
      "status": "modified",
      "additions": 15,
      "deletions": 3,
      "changes": 18,
      "patch": "@@ -10,7 +10,19 @@..."
    }
  ],
  "count": 5
}
```

**Use cases:**
- Code review preparation
- Impact analysis and scope assessment
- Automated PR checks
- Test coverage verification
- Documentation update checks
- File type filtering
- Change size metrics

---

### Pull Request Review Operations

#### get_pull_request_reviews

Get all reviews for a pull request.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pull_number` (number): PR number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42
}
```

**Returns:** Array of reviews with:
- `id`: Review ID
- `user`: Reviewer username and profile
- `body`: Review comment text
- `state`: "APPROVED", "CHANGES_REQUESTED", "COMMENTED", "DISMISSED", "PENDING"
- `submitted_at`: When review was submitted
- `commit_id`: SHA the review is associated with

**Use cases:**
- Check approval status before merging
- See who has reviewed and their feedback
- Understand what changes were requested
- Track review history over time

---

#### create_pull_request_review

Create a review on a pull request (approve, request changes, or comment).

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pull_number` (number): PR number
- `event` (string): "APPROVE", "REQUEST_CHANGES", or "COMMENT"
- `body` (string, optional): Review comment
- `commit_id` (string, optional): Specific commit to review

**Example (approve):**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42,
  "event": "APPROVE",
  "body": "Looks good to me!"
}
```

**Example (request changes):**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42,
  "event": "REQUEST_CHANGES",
  "body": "Please address the comments before merging."
}
```

**Event types:**
- `APPROVE`: Approve the PR (allows merging if required reviews are met)
- `REQUEST_CHANGES`: Block PR until changes are made
- `COMMENT`: Leave review comments without approval/blocking

**Note:** This creates a REVIEW, not individual line comments. Use add_pull_request_review_comment for inline comments.

---

#### add_pull_request_review_comment

Add an inline review comment to a PR (comment on specific lines of code).

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pull_number` (number): PR number
- `body` (string): Comment text
- `commit_id` (string, optional): Commit SHA
- `path` (string, optional): File path
- `line` (number, optional): Line number in diff
- `side` (string, optional): "LEFT" or "RIGHT"
- `start_line` (number, optional): For multi-line comments
- `start_side` (string, optional): Side of start line
- `subject_type` (string, optional): Subject type
- `in_reply_to` (number, optional): Comment ID to reply to

**Example (inline comment):**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42,
  "body": "Consider using const here",
  "commit_id": "abc123",
  "path": "src/main.rs",
  "line": 45,
  "side": "RIGHT"
}
```

**Example (multi-line comment):**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42,
  "body": "This entire function could be simplified",
  "commit_id": "abc123",
  "path": "src/utils.rs",
  "start_line": 20,
  "line": 25,
  "side": "RIGHT"
}
```

**Example (reply to comment):**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42,
  "body": "Good point, will fix",
  "in_reply_to": 123456789
}
```

**Comment types:**
1. **New inline comment**: Requires commit_id, path, line, and optionally side
2. **Multi-line comment**: Requires commit_id, path, start_line, line, and optionally sides
3. **Threaded reply**: Only requires in_reply_to (inherits position from parent)

**Tips:**
- Use RIGHT side for commenting on new/changed code (most common)
- Use LEFT side for commenting on old code being removed
- Multi-line comments span from start_line to line (inclusive)
- Thread replies create conversations on specific code sections

---

#### request_copilot_review

Request GitHub Copilot to review a pull request (experimental).

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `pull_number` (number): PR number

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pull_number": 42
}
```

**What Copilot reviews:**
- Code quality and best practices
- Potential bugs and issues
- Security vulnerabilities
- Performance improvements
- Code style and conventions

**Important notes:**
- This is an **EXPERIMENTAL** feature that may change
- Requires GitHub Copilot access on the repository
- The request triggers the review but doesn't return the review content
- Check PR comments after a short delay to see Copilot's feedback
- Not all repositories have Copilot review enabled

**Example workflow:**
1. Create or update a pull request
2. Request Copilot review: `request_copilot_review({...})`
3. Wait a few moments for Copilot to analyze
4. Check PR comments: `get_pull_request_reviews({...})`
5. Review Copilot's suggestions and feedback

---

### Repository Operations

#### create_repository

Create a new repository under the authenticated user's account.

**Arguments:**
- `name` (string): Repository name
- `description` (string, optional): Repository description
- `private` (boolean, optional): Make private
- `auto_init` (boolean, optional): Initialize with README

**Example:**
```json
{
  "name": "my-project",
  "description": "My awesome project",
  "private": false,
  "auto_init": true
}
```

---

#### fork_repository

Fork a repository.

**Arguments:**
- `owner` (string): Repository owner to fork from
- `repo` (string): Repository name
- `organization` (string, optional): Fork to organization

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world"
}
```

---

#### list_branches

List branches in a repository.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "per_page": 50
}
```

---

#### create_branch

Create a new branch from a commit SHA.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `branch_name` (string): New branch name
- `sha` (string): Commit SHA to branch from

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch_name": "feature/new-feature",
  "sha": "abc123def456"
}
```

---

#### list_commits

List commits in a repository with filtering options.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `sha` (string, optional): Branch or SHA to start from
- `path` (string, optional): Only commits affecting this path
- `author` (string, optional): Filter by author
- `since` (string, optional): Only after this date (ISO 8601)
- `until` (string, optional): Only before this date (ISO 8601)
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "sha": "main",
  "author": "octocat",
  "since": "2024-01-01T00:00:00Z",
  "per_page": 25
}
```

---

#### get_commit

Get detailed information about a specific commit.

**Arguments:**
- `owner` (string): Repository owner
- `repo` (string): Repository name
- `commit_sha` (string): Commit SHA
- `page` (number, optional): Page for files list
- `per_page` (number, optional): Results per page

**Example:**
```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "commit_sha": "abc123def456"
}
```

---

### Search Operations

#### search_code

Search code across GitHub repositories.

**Arguments:**
- `query` (string): Search query (GitHub code search syntax)
- `sort` (string, optional): Sort by "indexed"
- `order` (string, optional): "asc" or "desc"
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page

**Query Syntax:**
- `repo:owner/repo` - Search in repository
- `language:rust` - Filter by language
- `path:src/` - Search in path
- `extension:rs` - Filter by extension
- Text without prefix searches code content

**Example:**
```json
{
  "query": "repo:octocat/hello-world async fn language:rust",
  "per_page": 20
}
```

---

#### search_repositories

Search GitHub repositories.

**Arguments:**
- `query` (string): Search query (GitHub repository search syntax)
- `sort` (string, optional): "stars", "forks", or "updated"
- `order` (string, optional): "asc" or "desc"
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page

**Query Syntax:**
- `language:rust` - Filter by language
- `stars:>1000` - Star count filter
- `forks:>100` - Fork count filter
- `topic:async` - Filter by topic
- `user:octocat` - By user
- `org:github` - By organization

**Example:**
```json
{
  "query": "language:rust stars:>100 topic:async",
  "sort": "stars",
  "order": "desc",
  "per_page": 10
}
```

---

#### search_users

Search GitHub users.

**Arguments:**
- `query` (string): Search query
- `sort` (string, optional): "followers", "repositories", or "joined"
- `order` (string, optional): "asc" or "desc"
- `page` (number, optional): Page number
- `per_page` (number, optional): Results per page

**Example:**
```json
{
  "query": "location:\"San Francisco\" language:rust",
  "sort": "followers",
  "order": "desc"
}
```

---

## Environment Variables

All tools require:
- `GITHUB_TOKEN`: Personal access token or app token

**Token scopes:**
- Public repos: `public_repo` scope
- Private repos: `repo` scope

## Usage in Rust

```rust
use kodegen_github::GitHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GitHubClient::with_token("ghp_...")?;
    
    let issue = client.get_issue("octocat", "hello-world", 42).await?;
    println!("Issue: {:?}", issue);
    
    Ok(())
}
```

## MCP Integration

These tools are automatically available when the `github` feature is enabled in the kodegen server:

```bash
# Enable GitHub tools
kodegen --tool github

# List available categories
kodegen --list-categories
```

## License

MIT
