use crate::utils::{DeepResearch, ResearchOptions};
use kodegen_mcp_schema::browser::{BrowserResearchArgs, BrowserResearchPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

/// Browser research tool that performs deep multi-page web research with AI summarization
///
/// Uses loopback MCP client pattern from BrowserAgentTool to call web_search, browser_navigate,
/// and browser_extract_text tools via the hot path.
#[derive(Clone)]
pub struct BrowserResearchTool {
    server_url: String,
}

impl BrowserResearchTool {
    /// Create new browser research tool with loopback server URL
    ///
    /// The server_url enables the tool to create an MCP client that calls back into
    /// the same server's tools (web_search, browser_navigate, browser_extract_text).
    ///
    /// # Pattern
    /// Follows BrowserAgentTool loopback pattern (packages/tools-browser/src/tools/browser_agent.rs:65-73)
    pub fn new(server_url: String) -> Self {
        Self { server_url }
    }
}

impl Tool for BrowserResearchTool {
    type Args = BrowserResearchArgs;
    type PromptArgs = BrowserResearchPromptArgs;

    fn name() -> &'static str {
        "browser_research"
    }

    fn description() -> &'static str {
        "Deep research tool that searches the web, crawls multiple pages, and generates AI-powered summaries.\n\n\
         Automatically extracts key findings, data points, and conclusions from web content.\n\
         Useful for gathering comprehensive information on technical topics, documentation, or current events.\n\n\
         Example: browser_research({\"query\": \"Rust async programming best practices 2024\", \"max_pages\": 5, \"temperature\": 0.3})"
    }

    fn read_only() -> bool {
        true // Research only reads web content, doesn't modify browser state permanently
    }

    fn open_world() -> bool {
        true // Can research arbitrary URLs from search results
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // STEP 1: Create loopback MCP client (pattern from browser_agent.rs:100-107)
        let (mcp_client, _connection) = kodegen_mcp_client::create_sse_client(&self.server_url)
            .await
            .map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Failed to create loopback MCP client to {}: {}.\n\
                 Ensure SSE server is running and accessible.",
                    self.server_url,
                    e
                ))
            })?;

        // STEP 2: Create DeepResearch instance with MCP client
        // Constructor signature from deep_research.rs:89-96
        let research = DeepResearch::new(
            std::sync::Arc::new(mcp_client),
            args.temperature,
            args.max_tokens,
        );

        // STEP 3: Build research options from args
        let options = Some(ResearchOptions {
            max_pages: args.max_pages,
            max_depth: args.max_depth,
            search_engine: args.search_engine.clone(),
            include_links: args.include_links,
            extract_tables: args.extract_tables,
            extract_images: args.extract_images,
            timeout_seconds: args.timeout_seconds,
        });

        // STEP 4: Execute research (calls web_search, browser_navigate, browser_extract_text via MCP)
        // Method signature from deep_research.rs:98-138
        let results = research.research(&args.query, options).await.map_err(|e| {
            McpError::Other(anyhow::anyhow!(
                "Research failed for query '{}': {}",
                args.query,
                e
            ))
        })?;

        // STEP 5: Build comprehensive response
        let pages_visited = results.len();

        // Combine all individual summaries into unified report
        let comprehensive_summary = if results.is_empty() {
            format!("No results found for query: '{}'", args.query)
        } else {
            let mut summary = format!("# Research Report: {}\n\n", args.query);
            summary.push_str(&format!("Analyzed {} pages\n\n", pages_visited));

            for (i, result) in results.iter().enumerate() {
                summary.push_str(&format!("## Source {} - {}\n", i + 1, result.title));
                summary.push_str(&format!("URL: {}\n\n", result.url));
                summary.push_str(&result.summary);
                summary.push_str("\n\n---\n\n");
            }

            summary
        };

        // Extract key findings (first sentence of each summary)
        let key_findings: Vec<String> = results
            .iter()
            .filter_map(|r| {
                let first_line = r.summary.lines().next()?;
                if !first_line.is_empty() {
                    Some(format!("{}: {}", r.title, first_line))
                } else {
                    None
                }
            })
            .collect();

        // Extract all source URLs for verification
        let sources: Vec<String> = results.iter().map(|r| r.url.clone()).collect();

        // Return structured JSON response
        Ok(json!({
            "success": pages_visited > 0,
            "query": args.query,
            "pages_visited": pages_visited,
            "max_pages": args.max_pages,
            "comprehensive_summary": comprehensive_summary,
            "sources": sources,
            "key_findings": key_findings,
            "individual_results": results.iter().map(|r| json!({
                "url": r.url,
                "title": r.title,
                "summary": r.summary,
                "content_length": r.content.len(),
                "timestamp": r.timestamp.to_rfc3339(),
            })).collect::<Vec<_>>(),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I research a technical topic?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_research to perform deep multi-page research with AI summarization.\n\n\
                     Technology research example:\n\
                     {\"query\": \"What are the latest features in Rust 1.80?\", \
                       \"max_pages\": 5, \
                       \"temperature\": 0.3}\n\n\
                     Documentation deep dive example:\n\
                     {\"query\": \"How does tokio's runtime work internally?\", \
                       \"max_pages\": 8, \
                       \"max_depth\": 3, \
                       \"extract_tables\": true, \
                       \"temperature\": 0.4}\n\n\
                     Current events research example:\n\
                     {\"query\": \"Latest developments in WebAssembly 2024\", \
                       \"max_pages\": 6, \
                       \"search_engine\": \"duckduckgo\"}\n\n\
                     The tool will:\n\
                     - Search the web for relevant pages using web_search tool\n\
                     - Visit and extract content from each page via browser_navigate + browser_extract_text\n\
                     - Generate AI summaries of findings using CandleFluentAi\n\
                     - Compile a comprehensive research report with all sources",
                ),
            },
        ])
    }
}
