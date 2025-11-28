//! Web crawling and search tools: documentation scraping with full-text indexing

use kodegen_mcp_schema::citescrape;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn citescrape_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: citescrape::SCRAPE_URL,
            category: "citescrape",
            description: "Start a background web crawl that saves content to markdown/HTML/JSON and optionally indexes for full-text search. Returns immediately with crawl_i...",
            schema: build_schema::<citescrape::ScrapeUrlArgs>(),
        },
        ToolMetadata {
            name: citescrape::WEB_SEARCH,
            category: "citescrape",
            description: "Perform web search using DuckDuckGo and return structured results with titles, URLs, and snippets.",
            schema: build_schema::<citescrape::WebSearchArgs>(),
        },
    ]
}
