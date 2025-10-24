//! MCP type definitions for crawl session management

use crate::config::CrawlConfig;
use crate::crawl_engine::CrawlProgress;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of a crawl session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrawlStatus {
    /// Crawl is currently running
    Running,
    /// Crawl completed successfully
    Completed,
    /// Crawl failed with error message
    Failed { error: String },
}

/// Active crawl session tracked in memory
///
/// This struct is NOT serialized - it's for runtime tracking only.
/// For persistence, use `CrawlManifest`.
#[derive(Debug, Clone)]
pub struct ActiveCrawlSession {
    /// Unique identifier (UUID v4)
    pub crawl_id: String,

    /// Full crawl configuration
    pub config: CrawlConfig,

    /// When the crawl started
    pub start_time: DateTime<Utc>,

    /// Output directory path
    pub output_dir: PathBuf,

    /// Current crawl status
    pub status: CrawlStatus,

    /// Latest progress update (if any)
    pub progress: Option<CrawlProgress>,

    /// Number of pages crawled so far
    pub total_pages: usize,

    /// Current URL being processed
    pub current_url: Option<String>,

    /// Background task handle (keeps task alive)
    ///
    /// Similar to `TerminalManager`'s pattern, we store the `JoinHandle` to ensure
    /// the background crawl task remains tracked and alive for the session's lifetime.
    /// This prevents the browser WebSocket from being prematurely dropped.
    #[allow(dead_code)]
    pub task_handle: Option<std::sync::Arc<tokio::task::JoinHandle<()>>>,
}

/// Lightweight configuration summary for manifest storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    pub start_url: String,
    pub max_depth: u8,
    pub limit: Option<usize>,
    pub save_markdown: bool,
    pub save_screenshots: bool,
    pub enable_search: bool,
    pub crawl_rate_rps: f64,
}

impl From<&CrawlConfig> for ConfigSummary {
    fn from(config: &CrawlConfig) -> Self {
        Self {
            start_url: config.start_url().to_string(),
            max_depth: config.max_depth(),
            limit: config.limit(),
            save_markdown: config.save_markdown(),
            save_screenshots: config.save_screenshots(),
            // CRITICAL: Access field directly, not getter
            // search_index_dir() has fallback logic; we want to know if it was explicitly set
            enable_search: config.search_index_dir.is_some(),
            crawl_rate_rps: config.crawl_rate_rps().unwrap_or(2.0),
        }
    }
}

/// Persistent manifest for crawl metadata
///
/// Saved to {`output_dir}/manifest.json` for historical queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlManifest {
    pub crawl_id: String,
    pub start_url: String,
    pub output_dir: PathBuf,
    pub search_index_dir: PathBuf,

    /// Crawl start time (serialized as Unix timestamp seconds)
    #[serde(with = "chrono::serde::ts_seconds")]
    pub start_time: DateTime<Utc>,

    /// Crawl end time (serialized as Unix timestamp seconds)
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub end_time: Option<DateTime<Utc>>,

    pub status: CrawlStatus,
    pub total_pages: usize,
    pub config_summary: ConfigSummary,
}

impl CrawlManifest {
    /// Create manifest from an active session
    #[must_use]
    pub fn from_session(session: &ActiveCrawlSession) -> Self {
        Self {
            crawl_id: session.crawl_id.clone(),
            start_url: session.config.start_url().to_string(),
            output_dir: session.output_dir.clone(),
            search_index_dir: session.output_dir.join(".search_index"),
            start_time: session.start_time,
            end_time: None, // Not ended yet
            status: session.status.clone(),
            total_pages: session.total_pages,
            config_summary: ConfigSummary::from(&session.config),
        }
    }

    /// Mark crawl as successfully completed
    pub fn complete(&mut self, total_pages: usize) {
        self.end_time = Some(Utc::now());
        self.status = CrawlStatus::Completed;
        self.total_pages = total_pages;
    }

    /// Mark crawl as failed with error
    pub fn fail(&mut self, error: String) {
        self.end_time = Some(Utc::now());
        self.status = CrawlStatus::Failed { error };
    }
}
