//! Web and external tools: browser, citescrape

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn web_external_tools() -> Vec<ToolMetadata> {
    vec![
        // BROWSER (14 tools)
        ToolMetadata {
            name: "browser_agent",
            category: "browser",
            description: "Autonomous browser agent that executes multi-step tasks using AI reasoning.nn The agent can navigate websites, interact with forms, extract informa...",
            schema: build_schema::<browser::BrowserAgentArgs>(),
        },
        ToolMetadata {
            name: "browser_click",
            category: "browser",
            description: "Click an element on the page using a CSS selector.nn Automatically scrolls element into view before clicking.nn Example: browser_click({'selector':...",
            schema: build_schema::<browser::BrowserClickArgs>(),
        },
        ToolMetadata {
            name: "browser_extract_text",
            category: "browser",
            description: "Extract text content from the page or specific element.nn Returns the text content for AI agent analysis.nn Example: browser_extract_text({}) - Ful...",
            schema: build_schema::<browser::BrowserExtractTextArgs>(),
        },
        ToolMetadata {
            name: "browser_navigate",
            category: "browser",
            description: "Navigate to a URL in the browser. Opens the page and waits for load completion.nn Returns current URL after navigation (may differ from requested U...",
            schema: build_schema::<browser::BrowserNavigateArgs>(),
        },
        ToolMetadata {
            name: "browser_screenshot",
            category: "browser",
            description: "Take a screenshot of the current page or specific element. Returns base64-encoded image.nn Example: browser_screenshot({}) for full pagen Example: ...",
            schema: build_schema::<browser::BrowserScreenshotArgs>(),
        },
        ToolMetadata {
            name: "browser_scroll",
            category: "browser",
            description: "Scroll the page by amount or to a specific element.nn Examples:n - browser_scroll({'y': 500}) - Scroll down 500pxn - browser_scroll({'selector': '#...",
            schema: build_schema::<browser::BrowserScrollArgs>(),
        },
        ToolMetadata {
            name: "browser_type_text",
            category: "browser",
            description: "Type text into an input element using a CSS selector.nn Automatically focuses element and clears existing text by default.nn Example: browser_type_...",
            schema: build_schema::<browser::BrowserTypeTextArgs>(),
        },
        ToolMetadata {
            name: "browser_wait",
            category: "browser",
            description: "Wait for a specified duration (useful for waiting for dynamic content to load).nn Example: browser_wait({'duration_ms': 2000}) - Wait 2 seconds' } ...",
            schema: build_schema::<browser::BrowserWaitArgs>(),
        },
        ToolMetadata {
            name: "browser_get_research_result",
            category: "browser",
            description: "Get final results from a completed browser research session.nn Returns comprehensive summary, sources, key findings, and individual page results.nn ...",
            schema: build_schema::<browser::GetResearchResultArgs>(),
        },
        ToolMetadata {
            name: "browser_get_research_status",
            category: "browser",
            description: "Get current status and progress of a browser research session.nn Returns status (running/completed/failed/cancelled), runtime, pages visited, and ...",
            schema: build_schema::<browser::GetResearchStatusArgs>(),
        },
        ToolMetadata {
            name: "browser_list_research_sessions",
            category: "browser",
            description: "List all active browser research sessions.nn Shows session ID, query, status, runtime, and progress for each session.nn Useful for tracking multiple...",
            schema: build_schema::<browser::ListResearchSessionsArgs>(),
        },
        ToolMetadata {
            name: "browser_start_research",
            category: "browser",
            description: "Start async browser research session that runs in background.nn Searches web, crawls multiple pages, and generates AI summaries without blocking.nn...",
            schema: build_schema::<browser::StartBrowserResearchArgs>(),
        },
        ToolMetadata {
            name: "browser_stop_research",
            category: "browser",
            description: "Cancel a running browser research session.nn Aborts the background research task and marks session as cancelled.nn Does nothing if research is alr...",
            schema: build_schema::<browser::StopBrowserResearchArgs>(),
        },
        ToolMetadata {
            name: "browser_web_search",
            category: "browser",
            description: "Perform a web search using DuckDuckGo and return structured results with titles, URLs, and snippets.nn Returns up to 10 search results with:n - ran...",
            schema: build_schema::<citescrape::WebSearchArgs>(),
        },
        // CITESCRAPE (3 tools)
        ToolMetadata {
            name: "scrape_check_results",
            category: "citescrape",
            description: "Check crawl status and retrieve results for active or completed crawls. Returns progress information for running crawls and summary with file list ...",
            schema: build_schema::<citescrape::ScrapeCheckResultsArgs>(),
        },
        ToolMetadata {
            name: "scrape_search_results",
            category: "citescrape",
            description: "Full-text search across crawled documentation using Tantivy. Supports advanced query syntax including text, phrase, boolean, field-specific, and fu...",
            schema: build_schema::<citescrape::ScrapeSearchResultsArgs>(),
        },
        ToolMetadata {
            name: "scrape_url",
            category: "citescrape",
            description: "Start a background web crawl that saves content to markdown/HTML/JSON and optionally indexes for full-text search. Returns immediately with crawl_i...",
            schema: build_schema::<citescrape::ScrapeUrlArgs>(),
        },
    ]
}
