//! `start_crawl` MCP tool implementation
//!
//! Initiates background web crawls with automatic search indexing.

use chrono::Utc;
use kodegen_mcp_schema::citescrape::{StartCrawlArgs, StartCrawlPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

use crate::ChromiumoxideCrawler;
use crate::CrawlRequest;
use crate::Crawler;
use crate::config::CrawlConfig;
use crate::mcp::manager::{
    CrawlSessionManager, ManifestManager, SearchEngineCache, url_to_output_dir,
};
use crate::mcp::types::{ActiveCrawlSession, CrawlManifest, CrawlStatus};

// =============================================================================
// Tool Struct
// =============================================================================

#[derive(Clone)]
pub struct StartCrawlTool {
    session_manager: Arc<CrawlSessionManager>,
    #[allow(dead_code)]
    engine_cache: Arc<SearchEngineCache>,
}

impl StartCrawlTool {
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

    fn validate_content_types(types: &[String]) -> Result<Vec<String>, McpError> {
        const VALID_TYPES: &[&str] = &["markdown", "html", "json", "png"];
        let mut normalized = Vec::new();

        for t in types {
            let lower = t.to_lowercase();
            if !VALID_TYPES.contains(&lower.as_str()) {
                return Err(McpError::InvalidArguments(format!(
                    "Invalid content_type '{t}'. Valid: markdown, html, json, png"
                )));
            }
            normalized.push(lower);
        }

        if normalized.is_empty() {
            return Err(McpError::InvalidArguments(
                "content_types cannot be empty. Valid: markdown, html, json, png".to_string(),
            ));
        }

        Ok(normalized)
    }

    fn validate_url(url: &str) -> Result<(), McpError> {
        Url::parse(url).map_err(|e| McpError::InvalidUrl(format!("Invalid URL '{url}': {e}")))?;
        Ok(())
    }

    fn resolve_output_dir(args: &StartCrawlArgs) -> Result<PathBuf, McpError> {
        if let Some(ref dir) = args.output_dir {
            Ok(PathBuf::from(dir))
        } else {
            url_to_output_dir(&args.url, None)
        }
    }

    fn build_config(
        args: &StartCrawlArgs,
        output_dir: &Path,
        content_types: Option<Vec<String>>,
    ) -> Result<CrawlConfig, McpError> {
        let search_index_dir = if args.enable_search {
            Some(output_dir.join(".search_index"))
        } else {
            None
        };

        // Determine save flags based on content_types if provided (now normalized)
        let save_markdown = if let Some(ref types) = content_types {
            types.iter().any(|t| t == "markdown")
        } else {
            args.save_markdown
        };

        let save_screenshots = if let Some(ref types) = content_types {
            types.iter().any(|t| t == "png")
        } else {
            args.save_screenshots
        };

        let config = CrawlConfig {
            storage_dir: output_dir.to_path_buf(),
            start_url: args.url.clone(),
            target_url: args.url.clone(),
            limit: args.limit,
            allow_subdomains: args.allow_subdomains,
            save_screenshots,
            save_markdown,
            max_depth: args.max_depth,
            search_index_dir,
            crawl_rate_rps: Some(args.crawl_rate_rps),
            ..Default::default()
        };

        Ok(config)
    }
}

// =============================================================================
// Tool Trait Implementation
// =============================================================================

impl Tool for StartCrawlTool {
    type Args = StartCrawlArgs;
    type PromptArgs = StartCrawlPromptArgs;

    fn name() -> &'static str {
        "start_crawl"
    }

    fn description() -> &'static str {
        "Start a background web crawl that saves content to markdown/HTML/JSON \
         and optionally indexes for full-text search. Returns immediately with \
         crawl_id for status tracking via get_crawl_results.\n\n\
         Features:\n\
         - Background processing (non-blocking)\n\
         - Automatic search indexing (Tantivy)\n\
         - Rate limiting (default 2 req/sec)\n\
         - Markdown/HTML/JSON output\n\
         - Screenshot capture support\n\
         - Content type selection via content_types parameter\n\n\
         Content Types:\n\
         - \"markdown\": Save markdown files (.md)\n\
         - \"html\": HTML files (always saved, informational only for filtering)\n\
         - \"json\": JSON metadata (always saved, informational only for filtering)\n\
         - \"png\": Enable screenshot capture (.png)\n\
         Content types are case-insensitive (\"markdown\", \"Markdown\", \"MARKDOWN\" all work).\n\
         Empty arrays are rejected with error.\n\
         Invalid types return clear error messages.\n\n\
         Example: start_crawl({\"url\": \"https://ratatui.rs\", \"content_types\": [\"markdown\", \"html\", \"png\"]})"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // 1. Validate input
        Self::validate_url(&args.url)?;

        // 2. Validate and normalize content_types if provided
        let content_types = if let Some(ref types) = args.content_types {
            Some(Self::validate_content_types(types)?)
        } else {
            None
        };

        // 3. Resolve output directory
        let output_dir = Self::resolve_output_dir(&args)?;

        // 4. Build crawl config
        let mut config = Self::build_config(&args, &output_dir, content_types)?;

        // 5. Get or initialize search engine and indexing sender if search is enabled
        if args.enable_search {
            let entry = self
                .engine_cache
                .get_or_init(output_dir.clone(), &config)
                .await?;
            if let Some(indexing_sender) = entry.indexing_sender {
                config = config.with_indexing_sender(indexing_sender);
            }
        }

        // 6. Generate unique crawl ID
        let crawl_id = Uuid::new_v4().to_string();

        // 6.5. Create unique Chrome user data directory for this crawl session
        // Using crawl_id ensures profile isolation between concurrent/sequential crawls
        let chrome_data_dir = std::env::temp_dir().join(format!("enigo_chrome_{crawl_id}"));
        eprintln!(
            "DEBUG start_crawl: Created chrome_data_dir: {}",
            chrome_data_dir.display()
        );

        // 7. Create event bus for this crawl
        let event_bus = std::sync::Arc::new(crate::crawl_events::CrawlEventBus::new(1000));

        // 8. Attach event bus and chrome data dir to config
        let config = config
            .with_event_bus(event_bus.clone())
            .with_chrome_data_dir(chrome_data_dir.clone());

        eprintln!(
            "DEBUG start_crawl: After with_chrome_data_dir, config.chrome_data_dir = {:?}",
            config.chrome_data_dir
        );

        // 8.5. Subscribe to events for real-time progress tracking
        let mut event_receiver = event_bus.subscribe();
        let progress_session_manager = self.session_manager.clone();
        let progress_crawl_id = crawl_id.clone();

        tokio::spawn(async move {
            let mut page_count = 0;
            while let Ok(event) = event_receiver.recv().await {
                match event {
                    crate::crawl_events::CrawlEvent::PageCrawled { url, .. } => {
                        page_count += 1;
                        progress_session_manager
                            .update_progress(&progress_crawl_id, page_count, Some(url))
                            .await;
                    }
                    crate::crawl_events::CrawlEvent::Shutdown { .. } => {
                        // Crawler called shutdown_gracefully, exit subscriber
                        break;
                    }
                    _ => {}
                }
            }
        });

        // 6. Start background crawl (crawler.crawl() spawns internally)
        let session_manager = self.session_manager.clone();
        let crawl_id_clone = crawl_id.clone();

        // DEBUG: Verify chrome_data_dir is set before passing to crawler
        if let Some(ref dir) = config.chrome_data_dir {
            tracing::info!("✅ Config has chrome_data_dir: {}", dir.display());
        } else {
            tracing::warn!("⚠️  Config chrome_data_dir is None!");
        }

        // Create crawler and start crawl (internally spawns via tokio::spawn)
        let crawler = ChromiumoxideCrawler::new(config.clone());
        let request: CrawlRequest = crawler.crawl();

        // Spawn result handling task and STORE the JoinHandle
        let task_handle = tokio::spawn(async move {
            let result = request.await;

            // Handle result
            match result {
                Ok(()) => {
                    // Crawl completed without errors
                    let total_pages =
                        if let Some(sess) = session_manager.get_session(&crawl_id_clone).await {
                            sess.total_pages
                        } else {
                            0
                        };

                    // Check if any pages were successfully crawled
                    if total_pages == 0 {
                        // Zero pages crawled - treat as failure for better AI agent feedback
                        let error_msg = "No pages could be crawled. This may indicate an invalid URL, network issue, or inaccessible domain.".to_string();
                        session_manager
                            .update_status(
                                &crawl_id_clone,
                                CrawlStatus::Failed {
                                    error: error_msg.clone(),
                                },
                            )
                            .await;

                        // Save failed manifest
                        if let Some(failed_session) =
                            session_manager.get_session(&crawl_id_clone).await
                        {
                            let mut manifest = CrawlManifest::from_session(&failed_session);
                            manifest.fail(error_msg);

                            if let Err(e) = ManifestManager::save(&manifest).await {
                                eprintln!("Failed to save manifest: {e}");
                            }
                        }
                    } else {
                        // Success - at least one page was crawled
                        session_manager
                            .update_status(&crawl_id_clone, CrawlStatus::Completed)
                            .await;

                        // Save manifest
                        if let Some(final_session) =
                            session_manager.get_session(&crawl_id_clone).await
                        {
                            let mut manifest = CrawlManifest::from_session(&final_session);
                            manifest.complete(total_pages);

                            if let Err(e) = ManifestManager::save(&manifest).await {
                                eprintln!("Failed to save manifest: {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    // Crawl failed
                    let error_msg = format!("Crawl failed: {e}");
                    session_manager
                        .update_status(
                            &crawl_id_clone,
                            CrawlStatus::Failed {
                                error: error_msg.clone(),
                            },
                        )
                        .await;

                    // Save failed manifest
                    if let Some(failed_session) = session_manager.get_session(&crawl_id_clone).await
                    {
                        let mut manifest = CrawlManifest::from_session(&failed_session);
                        manifest.fail(error_msg);

                        if let Err(e) = ManifestManager::save(&manifest).await {
                            eprintln!("Failed to save manifest: {e}");
                        }
                    }
                }
            }
        });

        // 9. Create and register session with task handle
        let session = ActiveCrawlSession {
            crawl_id: crawl_id.clone(),
            config: config.clone(),
            start_time: Utc::now(),
            output_dir: output_dir.clone(),
            status: CrawlStatus::Running,
            progress: None,
            total_pages: 0,
            current_url: Some(args.url.clone()),
            task_handle: Some(std::sync::Arc::new(task_handle)),
        };

        self.session_manager
            .register(crawl_id.clone(), session)
            .await;

        // 7. Return immediately with crawl info
        let search_index_dir = if args.enable_search {
            output_dir
                .join(".search_index")
                .to_string_lossy()
                .to_string()
        } else {
            "disabled".to_string()
        };

        Ok(json!({
            "crawl_id": crawl_id,
            "status": "running",
            "output_dir": output_dir.to_string_lossy(),
            "search_index_dir": search_index_dir,
            "message": format!(
                "Crawl started for {}. Use get_crawl_results with crawl_id '{}' to check status.",
                args.url,
                crawl_id
            )
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I crawl a website?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The start_crawl tool initiates background web crawling:\n\n\
                     **Basic usage:**\n\
                     ```json\n\
                     start_crawl({\"url\": \"https://ratatui.rs\"})\n\
                     ```\n\n\
                     **With options:**\n\
                     ```json\n\
                     start_crawl({\n\
                       \"url\": \"https://docs.rs\",\n\
                       \"output_dir\": \"docs/docs.rs\",\n\
                       \"max_depth\": 2,\n\
                       \"limit\": 50,\n\
                       \"save_markdown\": true,\n\
                       \"enable_search\": true\n\
                     })\n\
                     ```\n\n\
                     **Key features:**\n\
                     - Runs in background (returns immediately)\n\
                     - Auto-generates output_dir from URL if not specified\n\
                     - Saves as markdown, HTML, and JSON\n\
                     - Optional search indexing (Tantivy full-text)\n\
                     - Rate limiting (default 2 req/sec)\n\
                     - Progress tracking via get_crawl_results\n\n\
                     **Recommended workflow:**\n\
                     1. start_crawl({\"url\": \"https://example.com\"})\n\
                     2. Returns crawl_id\n\
                     3. Poll with get_crawl_results({\"crawl_id\": \"...\"})\n\
                     4. Search with search_crawl_results({\"output_dir\": \"docs/example.com\", \"query\": \"...\"})",
                ),
            },
        ])
    }
}
