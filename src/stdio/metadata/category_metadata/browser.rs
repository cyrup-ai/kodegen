//! Browser tools: web page interaction, navigation, research

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn browser_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: BROWSER_AGENT,
            category: "browser",
            description: "Autonomous browser agent that executes multi-step tasks using AI reasoning.nn The agent can navigate websites, interact with forms, extract informa...",
            schema: build_schema::<browser::BrowserAgentArgs>(),
        },
        ToolMetadata {
            name: BROWSER_CLICK,
            category: "browser",
            description: "Click an element on the page using a CSS selector.nn Automatically scrolls element into view before clicking.nn Example: browser_click({'selector':...",
            schema: build_schema::<browser::BrowserClickArgs>(),
        },
        ToolMetadata {
            name: BROWSER_EXTRACT_TEXT,
            category: "browser",
            description: "Extract text content from the page or specific element.nn Returns the text content for AI agent analysis.nn Example: browser_extract_text({}) - Ful...",
            schema: build_schema::<browser::BrowserExtractTextArgs>(),
        },
        ToolMetadata {
            name: BROWSER_NAVIGATE,
            category: "browser",
            description: "Navigate to a URL in the browser. Opens the page and waits for load completion.nn Returns current URL after navigation (may differ from requested U...",
            schema: build_schema::<browser::BrowserNavigateArgs>(),
        },
        ToolMetadata {
            name: BROWSER_SCREENSHOT,
            category: "browser",
            description: "Take a screenshot of the current page or specific element. Returns base64-encoded image.nn Example: browser_screenshot({}) for full pagen Example: ...",
            schema: build_schema::<browser::BrowserScreenshotArgs>(),
        },
        ToolMetadata {
            name: BROWSER_SCROLL,
            category: "browser",
            description: "Scroll the page by amount or to a specific element.nn Examples:n - browser_scroll({'y': 500}) - Scroll down 500pxn - browser_scroll({'selector': '#...",
            schema: build_schema::<browser::BrowserScrollArgs>(),
        },
        ToolMetadata {
            name: BROWSER_TYPE_TEXT,
            category: "browser",
            description: "Type text into an input element using a CSS selector.nn Automatically focuses element and clears existing text by default.nn Example: browser_type_...",
            schema: build_schema::<browser::BrowserTypeTextArgs>(),
        },
        ToolMetadata {
            name: browser::BROWSER_RESEARCH,
            category: "browser",
            description: "Deep web research with real-time progress streaming. Searches web, crawls multiple pages, extracts content, generates AI summaries. Blocks until complete (20-120s depending on pages). Returns comprehensive report with sources.",
            schema: build_schema::<browser::BrowserResearchArgs>(),
        },
        ToolMetadata {
            name: BROWSER_WEB_SEARCH,
            category: "browser",
            description: "Perform a web search using DuckDuckGo and return structured results with titles, URLs, and snippets.nn Returns up to 10 search results with:n - ran...",
            schema: build_schema::<citescrape::WebSearchArgs>(),
        },
    ]
}
