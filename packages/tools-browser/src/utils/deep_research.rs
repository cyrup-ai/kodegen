//! Deep research module - infrastructure for future use

use std::sync::Arc;
use std::time::Duration;

// Workspace LLM infrastructure
use kodegen_candle_agent::prelude::*;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::utils::errors::UtilsError;

/// Research result containing extracted information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Research options for izing research behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchOptions {
    pub max_pages: usize,
    pub max_depth: usize,
    pub search_engine: String,
    pub include_links: bool,
    pub extract_tables: bool,
    pub extract_images: bool,
    pub timeout_seconds: u64,
}

impl Default for ResearchOptions {
    fn default() -> Self {
        Self {
            max_pages: 5,
            max_depth: 2,
            search_engine: "google".to_string(),
            include_links: true,
            extract_tables: true,
            extract_images: false,
            timeout_seconds: 60,
        }
    }
}

/// Deep research service using direct library integration
///
/// All browser operations call local functions directly:
/// - web_search (local) - DuckDuckGo search via BrowserManager
/// - browser_navigate - URL loading via BrowserManager
/// - browser_extract_text - Content extraction via BrowserManager
///
/// LLM operations use CandleFluentAi streaming (no trait objects).
pub struct DeepResearch {
    /// Browser manager for web_search and navigation
    browser_manager: Arc<crate::web_search::BrowserManager>,

    /// LLM temperature for summarization (0.0 = deterministic, 2.0 = creative)
    temperature: f64,

    /// Maximum tokens for LLM generation
    max_tokens: u64,

    /// Track visited URLs to avoid duplicates
    visited_urls: Arc<Mutex<Vec<String>>>,
}

impl DeepResearch {
    /// Create new DeepResearch instance with BrowserManager
    ///
    /// All browser operations call local web_search functions directly.
    ///
    /// # Arguments
    /// * `browser_manager` - Browser manager for web_search and navigation
    /// * `temperature` - LLM sampling temperature (0.0-2.0)
    /// * `max_tokens` - Maximum tokens for LLM generation
    pub fn new(browser_manager: Arc<crate::web_search::BrowserManager>, temperature: f64, max_tokens: u64) -> Self {
        Self {
            browser_manager,
            temperature,
            max_tokens,
            visited_urls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Perform web research on a query
    pub async fn research(
        &self,
        query: &str,
        options: Option<ResearchOptions>,
    ) -> Result<Vec<ResearchResult>, UtilsError> {
        let options = options.unwrap_or_default();

        // Initialize results
        let mut results = Vec::new();

        // Reset visited URLs
        let mut visited = self.visited_urls.lock().await;
        visited.clear();
        drop(visited);

        // Search for query
        let search_results = self.search_query(query, &options).await?;

        // Process each search result
        for url in search_results.iter().take(options.max_pages) {
            match self.process_url(url, &options).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    warn!("Error processing URL {}: {}", url, e);
                }
            }

            // Add to visited URLs
            let mut visited = self.visited_urls.lock().await;
            visited.push(url.clone());
            drop(visited);
        }

        Ok(results)
    }

    /// Search for query using web_search module directly
    ///
    /// Calls local web_search which provides DuckDuckGo search
    /// with kromekover stealth, retries, and structured result parsing.
    ///
    /// # Arguments
    /// * `query` - Search query string
    /// * `options` - Research options (currently unused, web_search has sensible defaults)
    ///
    /// # Returns
    /// Vector of URLs from search results (up to 10)
    ///
    /// # Direct Integration
    /// This method calls web_search directly (same package) instead of via MCP.
    /// Benefits:
    /// - Faster (no IPC overhead)
    /// - Simpler (no serialization/deserialization)
    /// - More reliable (no network/process dependencies)
    async fn search_query(
        &self,
        query: &str,
        _options: &ResearchOptions,
    ) -> Result<Vec<String>, UtilsError> {
        debug!("Searching DuckDuckGo via web_search (direct): {}", query);

        // Call web_search directly (same package, no MCP needed)
        let search_results = crate::web_search::search_with_manager(&self.browser_manager, query)
            .await
            .map_err(|e| UtilsError::BrowserError(e.to_string()))?;

        // Extract URLs from SearchResults
        let urls: Vec<String> = search_results.results.iter()
            .map(|r| r.url.clone())
            .collect();

        if urls.is_empty() {
            warn!("web_search returned no results for query: {}", query);
        } else {
            info!("web_search found {} URLs for query: {}", urls.len(), query);
        }

        Ok(urls)
    }

    /// Process a URL and extract content
    async fn process_url(
        &self,
        url: &str,
        options: &ResearchOptions,
    ) -> Result<ResearchResult, UtilsError> {
        // Check if already visited
        let visited = self.visited_urls.lock().await;
        if visited.contains(&url.to_string()) {
            return Err(UtilsError::UnexpectedError("URL already visited".into()));
        }
        drop(visited);

        // 1. NAVIGATE VIA MCP CLIENT (HOT PATH)
        debug!("Navigating to {} via MCP browser_navigate tool", url);
        let nav_result = self
            .mcp_client
            .call_tool(
                "browser_navigate",
                serde_json::json!({
                    "url": url,
                    "timeout_ms": options.timeout_seconds * 1000
                }),
            )
            .await
            .map_err(|e| UtilsError::BrowserError(e.to_string()))?;

        // Parse navigation result to get actual URL (may have redirected)
        let final_url = if let Some(content) = nav_result.content.first() {
            if let Some(text) = content.as_text() {
                let response: serde_json::Value = serde_json::from_str(&text.text)
                    .map_err(|e| UtilsError::JsonParseError(e.to_string()))?;
                response
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(url)
                    .to_string()
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        };

        // 2. WAIT FOR PAGE TO SETTLE
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 3. GET PAGE TITLE
        // TODO: Add browser_get_page_info tool to get title/metadata
        let title = final_url
            .split('/')
            .next_back()
            .unwrap_or("Untitled")
            .to_string();

        // 4. EXTRACT CONTENT VIA MCP CLIENT (HOT PATH)
        debug!("Extracting content via MCP browser_extract_text tool");
        let extract_result = self
            .mcp_client
            .call_tool(
                "browser_extract_text",
                serde_json::json!({}), // No selector = full page
            )
            .await
            .map_err(|e| UtilsError::BrowserError(e.to_string()))?;

        // Parse extraction result
        let content = if let Some(content_item) = extract_result.content.first() {
            if let Some(text) = content_item.as_text() {
                let response: serde_json::Value = serde_json::from_str(&text.text)
                    .map_err(|e| UtilsError::JsonParseError(e.to_string()))?;
                response
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                return Err(UtilsError::JsonParseError("Expected text content".into()));
            }
        } else {
            return Err(UtilsError::JsonParseError(
                "No content in extract result".into(),
            ));
        };

        // 5. GENERATE SUMMARY WITH CANDLEFLUENTAI
        let summary = self.summarize_content(&title, &content).await?;

        // 6. ADD TO VISITED URLS
        let mut visited = self.visited_urls.lock().await;
        visited.push(final_url.clone());
        drop(visited);

        Ok(ResearchResult {
            url: final_url,
            title,
            content,
            summary,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Summarize content using CandleFluentAi streaming
    ///
    /// Creates an LLM agent on-demand with configured temperature and max_tokens.
    /// Streams response in real-time for better perceived performance.
    ///
    /// # Pattern Reference
    /// Based on: packages/tools-candle-agent/examples/fluent_builder.rs:58-90
    async fn summarize_content(&self, title: &str, content: &str) -> Result<String, UtilsError> {
        // Truncate content if too long (avoid context overflow)
        let max_content_length = 8000;
        let truncated_content = if content.len() > max_content_length {
            format!("{}... [content truncated]", &content[0..max_content_length])
        } else {
            content.to_string()
        };

        // Build prompt
        let prompt = format!(
            "Please summarize the following webpage content.\n\nTitle: '{}'\n\nContent:\n{}",
            title, truncated_content
        );

        // Create streaming agent with CandleFluentAi builder
        let mut stream = CandleFluentAi::agent_role("research-summarizer")
            .temperature(self.temperature)
            .max_tokens(self.max_tokens)
            .system_prompt(
                "You are an AI research assistant that summarizes web content accurately \
                and concisely. Extract key information, findings, data points, and conclusions. \
                Organize information logically and provide accurate section headers where appropriate. \
                Focus on factual content, avoid speculation."
            )
            .on_chunk(|chunk| async move {
                // Pass through chunks (could add logging here)
                chunk
            })
            .into_agent()
            .chat(move |_conversation| {
                let prompt_clone = prompt.clone();
                async move { CandleChatLoop::UserPrompt(prompt_clone) }
            })
            .map_err(|e| UtilsError::LlmError(e.to_string()))?;

        // Collect streamed response into String
        use tokio_stream::StreamExt;
        // Pre-allocate for research summary streaming
        // Typical summaries: 1000-2000 tokens (~4-8KB)
        // Use 8KB (8192 bytes) conservative estimate
        let mut summary = String::with_capacity(8192);
        while let Some(chunk) = stream.next().await {
            match chunk {
                CandleMessageChunk::Text(text) => {
                    summary.push_str(&text);
                }
                CandleMessageChunk::Complete { .. } => {
                    // Generation complete, summary is ready
                    break;
                }
                _ => {
                    // Ignore other chunk types (Thinking, etc.)
                }
            }
        }

        if summary.is_empty() {
            return Err(UtilsError::LlmError("Empty summary generated".into()));
        }

        Ok(summary)
    }
}
