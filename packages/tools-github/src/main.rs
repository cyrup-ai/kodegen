// Category SSE Server: GitHub Tools
//
// This binary serves GitHub API tools over SSE/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30451.

use anyhow::Result;
use kodegen_mcp_server_core::{run_sse_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_sse_server("github", |_config, _tracker| {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Register all GitHub tools (zero-state structs, no constructors)
        use kodegen_tools_github::*;

        // Issue tools (7)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateIssueTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetIssueTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListIssuesTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, UpdateIssueTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchIssuesTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, AddIssueCommentTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetIssueCommentsTool);

        // Pull Request tools (9)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreatePullRequestTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, UpdatePullRequestTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, MergePullRequestTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestStatusTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestFilesTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetPullRequestReviewsTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreatePullRequestReviewTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, AddPullRequestReviewCommentTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, RequestCopilotReviewTool);

        // Repository tools (2)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateRepositoryTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ForkRepositoryTool);

        // Branch/Commit tools (4)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListBranchesTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, CreateBranchTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, ListCommitsTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GetCommitTool);

        // Search tools (3)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchCodeTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchRepositoriesTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, SearchUsersTool);

        Ok(RouterSet::new(tool_router, prompt_router, managers))
    })
    .await
}
