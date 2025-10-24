//! `search_crawl_results` MCP tool implementation
//!
//! Full-text search across crawled documentation using Tantivy.

use kodegen_mcp_schema::citescrape::{SearchCrawlResultsArgs, SearchCrawlResultsPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::config::CrawlConfig;
use crate::mcp::manager::{CrawlSessionManager, SearchEngineCache};
use crate::search::query::SearchQueryBuilder;

// =============================================================================
// DEFAULT VALUES AND CONSTANTS
// =============================================================================

/// Maximum search results per page: 1000
///
/// Prevents users from requesting excessively large result sets that could
/// cause memory issues or slow responses. Users should paginate through
/// results using offset/limit parameters.
const MAX_SEARCH_RESULTS_PER_PAGE: usize = 1000;

// =============================================================================
// Tool Struct
// =============================================================================

#[derive(Clone)]
pub struct SearchCrawlResultsTool {
    session_manager: Arc<CrawlSessionManager>,
    engine_cache: Arc<SearchEngineCache>,
}

impl SearchCrawlResultsTool {
    #[must_use]
    pub fn new(
        session_manager: Arc<CrawlSessionManager>,
        engine_cache: Arc<SearchEngineCache>,
    ) -> Self {
        Self {
            session_manager,
            engine_cache,
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn validate_args(args: &SearchCrawlResultsArgs) -> Result<(), McpError> {
        if args.crawl_id.is_none() && args.output_dir.is_none() {
            return Err(McpError::InvalidArguments(
                "Either crawl_id or output_dir must be provided".to_string(),
            ));
        }

        if args.query.trim().is_empty() {
            return Err(McpError::InvalidArguments(
                "Query cannot be empty".to_string(),
            ));
        }

        if args.limit > MAX_SEARCH_RESULTS_PER_PAGE {
            return Err(McpError::InvalidArguments(format!(
                "Limit {} exceeds maximum of {}",
                args.limit, MAX_SEARCH_RESULTS_PER_PAGE
            )));
        }

        Ok(())
    }

    async fn resolve_output_dir(&self, args: &SearchCrawlResultsArgs) -> Result<PathBuf, McpError> {
        // Try crawl_id first
        if let Some(ref crawl_id) = args.crawl_id {
            if let Some(session) = self.session_manager.get_session(crawl_id).await {
                return Ok(session.output_dir);
            }

            return Err(McpError::ResourceNotFound(format!(
                "Crawl with ID '{crawl_id}' not found"
            )));
        }

        // Fall back to output_dir
        if let Some(ref dir) = args.output_dir {
            let path = PathBuf::from(dir);
            if !path.exists() {
                return Err(McpError::ResourceNotFound(format!(
                    "Output directory '{dir}' does not exist"
                )));
            }
            return Ok(path);
        }

        Err(McpError::InvalidArguments(
            "Neither crawl_id nor output_dir provided".to_string(),
        ))
    }

    fn verify_search_index(output_dir: &Path) -> Result<PathBuf, McpError> {
        let search_index_dir = output_dir.join(".search_index");
        let meta_file = search_index_dir.join("meta.json");

        if !meta_file.exists() {
            return Err(McpError::SearchIndex(format!(
                "Search index not found at {search_index_dir:?}. Ensure crawl completed with enable_search=true."
            )));
        }

        Ok(search_index_dir)
    }
}

// =============================================================================
// Tool Trait Implementation
// =============================================================================

impl Tool for SearchCrawlResultsTool {
    type Args = SearchCrawlResultsArgs;
    type PromptArgs = SearchCrawlResultsPromptArgs;

    fn name() -> &'static str {
        "search_crawl_results"
    }

    fn description() -> &'static str {
        "Full-text search across crawled documentation using Tantivy. Supports advanced \
         query syntax including text, phrase, boolean, field-specific, and fuzzy search. \
         Results include highlighted excerpts and relevance scores.\n\n\
         Query Syntax:\n\
         - Text: 'layout components' (searches all fields)\n\
         - Phrase: '\"exact phrase\"' (exact match)\n\
         - Boolean: 'layout AND (components OR widgets)'\n\
         - Field: 'title:layout' (search specific field)\n\
         - Fuzzy: 'layot~2' (allows 2 character differences)\n\n\
         Returns:\n\
         - results: array of {url, title, path, excerpt, score}\n\
         - total_count: total matching documents\n\
         - pagination: offset, limit, has_more, next_offset\n\n\
         Example: search_crawl_results({\"query\": \"layout components\", \"output_dir\": \"docs/ratatui.rs\"})"
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // 1. Validate arguments
        Self::validate_args(&args)?;

        // 2. Resolve output directory
        let output_dir = self.resolve_output_dir(&args).await?;

        // 3. Verify search index exists
        let search_index_dir = Self::verify_search_index(&output_dir)?;

        // 4. Create minimal config for search engine initialization
        let config = CrawlConfig {
            storage_dir: output_dir.clone(),
            start_url: "http://localhost".to_string(),
            target_url: "http://localhost".to_string(),
            search_index_dir: Some(search_index_dir.clone()),
            ..Default::default()
        };

        // 5. Get or initialize search engine from cache
        let entry = self
            .engine_cache
            .get_or_init(output_dir.clone(), &config)
            .await?;

        // 6. Start timer for search performance
        let start_time = Instant::now();

        // 7. Build and execute search query - Direct await of Future
        let search_results = SearchQueryBuilder::new(&args.query)
            .limit(args.limit)
            .offset(args.offset)
            .highlight(args.highlight)
            .execute_with_metadata((*entry.engine).clone())
            .await
            .map_err(|e| McpError::SearchIndex(format!("Search query execution failed: {e}")))?;

        let search_time_ms = start_time.elapsed().as_millis();

        // 9. Format results as JSON
        let results: Vec<Value> = search_results
            .results
            .iter()
            .map(|item| {
                json!({
                    "url": item.url,
                    "title": item.title,
                    "path": item.path,
                    "excerpt": item.excerpt,
                    "score": item.score,
                })
            })
            .collect();

        // 10. Build pagination info
        let has_more = search_results.has_more();
        let next_offset = search_results.next_offset();

        // 11. Return complete response
        Ok(json!({
            "query": args.query,
            "total_count": search_results.total_count,
            "results": results,
            "pagination": {
                "offset": args.offset,
                "limit": args.limit,
                "has_more": has_more,
                "next_offset": next_offset,
            },
            "search_time_ms": search_time_ms,
            "message": format!(
                "Found {} results in {}ms",
                search_results.total_count,
                search_time_ms
            ),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I search crawled documentation?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The search_crawl_results tool performs full-text search with Tantivy:\n\n\
                     **Basic search:**\n\
                     ```json\n\
                     search_crawl_results({\n\
                       \"query\": \"layout components\",\n\
                       \"output_dir\": \"docs/ratatui.rs\"\n\
                     })\n\
                     ```\n\n\
                     **With pagination:**\n\
                     ```json\n\
                     search_crawl_results({\n\
                       \"query\": \"layout\",\n\
                       \"output_dir\": \"docs/ratatui.rs\",\n\
                       \"limit\": 5,\n\
                       \"offset\": 10\n\
                     })\n\
                     ```\n\n\
                     **Advanced Query Syntax:**\n\n\
                     1. **Text search** (default):\n\
                        `\"layout components\"` - searches all fields\n\n\
                     2. **Phrase search**:\n\
                        `'\"exact phrase\"'` - exact match only\n\n\
                     3. **Boolean search**:\n\
                        `\"layout AND components\"` - both terms required\n\
                        `\"layout OR widgets\"` - either term\n\
                        `\"layout NOT deprecated\"` - exclude term\n\
                        `\"layout AND (components OR widgets)\"` - complex logic\n\n\
                     4. **Field-specific search**:\n\
                        `\"title:layout\"` - search only in title\n\
                        `\"content:components\"` - search only in content\n\n\
                     5. **Fuzzy search** (typo tolerance):\n\
                        `\"layot~2\"` - allows up to 2 character differences\n\
                        `\"componets~\"` - default distance of 1\n\n\
                     **Response format:**\n\
                     ```json\n\
                     {\n\
                       \"total_count\": 28,\n\
                       \"results\": [\n\
                         {\n\
                           \"url\": \"https://ratatui.rs/concepts/layout\",\n\
                           \"title\": \"Layout System\",\n\
                           \"path\": \"docs/ratatui.rs/concepts/layout.md\",\n\
                           \"excerpt\": \"The <em>layout</em> system...\",\n\
                           \"score\": 0.89\n\
                         }\n\
                       ],\n\
                       \"pagination\": {\n\
                         \"has_more\": true,\n\
                         \"next_offset\": 10\n\
                       },\n\
                       \"search_time_ms\": 12\n\
                     }\n\
                     ```\n\n\
                     **Pagination workflow:**\n\
                     1. First query: offset=0, limit=10\n\
                     2. Check pagination.has_more\n\
                     3. If true, use pagination.next_offset for next query\n\
                     4. Repeat until has_more is false\n\n\
                     **Tips:**\n\
                     - Results are ranked by relevance (score)\n\
                     - Excerpts include <em> tags for highlighted terms\n\
                     - Use boolean operators for precise queries\n\
                     - Fuzzy search helps with typos\n\
                     - Field search narrows results to specific sections",
                ),
            },
        ])
    }
}
