use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

// ============================================================================
// LOG ENTRY TYPE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBlockLogEntry {
    pub timestamp: DateTime<Utc>,
    pub search_text: String,
    pub found_text: Option<String>,
    pub similarity: Option<f64>,
    pub execution_time_ms: f64,
    pub exact_match_count: usize,
    pub expected_replacements: usize,
    pub fuzzy_threshold: f64,
    pub below_threshold: bool,
    pub diff: Option<String>,
    pub search_length: usize,
    pub found_length: Option<usize>,
    pub file_extension: String,
    pub character_codes: Option<String>,
    pub unique_character_count: Option<usize>,
    pub diff_length: Option<usize>,
    pub result: EditBlockResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditBlockResult {
    ExactMatch,
    FuzzyMatchAccepted,
    FuzzyMatchRejected,
    NoMatchFound,
    Error(String),
}

impl EditBlockLogEntry {
    /// Format as TSV line (tab-separated values)
    #[must_use]
    pub fn to_tsv(&self) -> String {
        vec![
            self.timestamp.to_rfc3339(),
            escape_tsv(&self.search_text),
            escape_tsv(self.found_text.as_deref().unwrap_or("")),
            format_option(self.similarity),
            self.execution_time_ms.to_string(),
            self.exact_match_count.to_string(),
            self.expected_replacements.to_string(),
            self.fuzzy_threshold.to_string(),
            self.below_threshold.to_string(),
            escape_tsv(self.diff.as_deref().unwrap_or("")),
            self.search_length.to_string(),
            format_option(self.found_length),
            self.file_extension.clone(),
            escape_tsv(self.character_codes.as_deref().unwrap_or("")),
            format_option(self.unique_character_count),
            format_option(self.diff_length),
            format!("{:?}", self.result),
        ]
        .join("\t")
    }
}

/// Escape special characters for TSV format
fn escape_tsv(s: &str) -> String {
    s.replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

/// Format Option<T> as string (empty if None)
fn format_option<T: ToString>(opt: Option<T>) -> String {
    opt.map(|v| v.to_string()).unwrap_or_default()
}

// ============================================================================
// ASYNC BACKGROUND LOGGER (FIRE-AND-FORGET)
// ============================================================================

#[derive(Clone)]
pub struct EditBlockLogger {
    /// Fire-and-forget channel sender (NEVER BLOCKS!)
    sender: mpsc::UnboundedSender<EditBlockLogEntry>,
    log_path: Arc<PathBuf>,
}

impl EditBlockLogger {
    /// Create new async logger with background task
    #[must_use]
    pub fn new() -> Self {
        let log_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kodegen-logs");

        let log_path = log_dir.join("edit-block.log");
        let log_path_arc = Arc::new(log_path);

        // Create unbounded channel for fire-and-forget
        let (tx, rx) = mpsc::unbounded_channel();

        // Start background processor
        Self::start_background_processor(rx, Arc::clone(&log_path_arc));

        Self {
            sender: tx,
            log_path: log_path_arc,
        }
    }

    /// Fire-and-forget logging (NEVER BLOCKS!)
    pub fn log(&self, entry: EditBlockLogEntry) {
        // Send to background task - if it fails, channel is closed (server shutdown)
        let _ = self.sender.send(entry);
    }

    #[must_use]
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Background task that processes log entries and batches disk writes
    /// This is copied from `usage_tracker.rs` pattern
    fn start_background_processor(
        mut rx: mpsc::UnboundedReceiver<EditBlockLogEntry>,
        log_path: Arc<PathBuf>,
    ) {
        tokio::spawn(async move {
            // Buffer for batching writes
            let mut pending_entries: Vec<EditBlockLogEntry> = Vec::new();

            // Flush interval (5 seconds like usage_tracker)
            let mut flush_interval = tokio::time::interval(std::time::Duration::from_secs(5));
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Lazy file writer initialization
            let mut writer: Option<tokio::io::BufWriter<tokio::fs::File>> = None;

            loop {
                tokio::select! {
                    // Receive log entry from channel
                    Some(entry) = rx.recv() => {
                        pending_entries.push(entry);
                    }

                    // Periodic flush (every 5 seconds)
                    _ = flush_interval.tick() => {
                        if !pending_entries.is_empty() {
                            // Ensure writer is initialized
                            if writer.is_none() {
                                match Self::init_log_file(&log_path).await {
                                    Ok(w) => writer = Some(w),
                                    Err(e) => {
                                        log::error!("Failed to initialize edit_block log: {e}");
                                        pending_entries.clear();
                                        continue;
                                    }
                                }
                            }

                            // Write all pending entries
                            if let Some(ref mut w) = writer {
                                for entry in pending_entries.drain(..) {
                                    let line = format!("{}\n", entry.to_tsv());
                                    if let Err(e) = w.write_all(line.as_bytes()).await {
                                        log::error!("Failed to write edit_block log entry: {e}");
                                    }
                                }

                                // Flush to disk
                                if let Err(e) = w.flush().await {
                                    log::error!("Failed to flush edit_block log: {e}");
                                }
                            }
                        }
                    }

                    // Channel closed (server shutdown)
                    else => {
                        // Final flush before exit
                        if !pending_entries.is_empty() && writer.is_none() {
                            writer = Self::init_log_file(&log_path).await.ok();
                        }

                        if let Some(ref mut w) = writer {
                            for entry in pending_entries.drain(..) {
                                let line = format!("{}\n", entry.to_tsv());
                                let _ = w.write_all(line.as_bytes()).await;
                            }
                            let _ = w.flush().await;
                        }
                        break;
                    }
                }
            }
        });
    }

    /// Initialize log file with headers (called from background task)
    async fn init_log_file(
        log_path: &PathBuf,
    ) -> std::io::Result<tokio::io::BufWriter<tokio::fs::File>> {
        // Create directory
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Check if file exists
        let file_exists = tokio::fs::try_exists(log_path).await.unwrap_or(false);

        // Open file in append mode
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await?;

        let mut writer = tokio::io::BufWriter::new(file);

        // Write headers if new file
        if !file_exists {
            let header = "timestamp\tsearch_text\tfound_text\tsimilarity\texecution_time_ms\t\
                 exact_match_count\texpected_replacements\tfuzzy_threshold\t\
                 below_threshold\tdiff\tsearch_length\tfound_length\t\
                 file_extension\tcharacter_codes\tunique_character_count\t\
                 diff_length\tresult\n";
            writer.write_all(header.as_bytes()).await?;
            writer.flush().await?;
        }

        Ok(writer)
    }
}

impl Default for EditBlockLogger {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GLOBAL SINGLETON
// ============================================================================

pub static EDIT_BLOCK_LOGGER: OnceLock<EditBlockLogger> = OnceLock::new();

/// Get the global async logger instance (NEVER BLOCKS!)
pub fn get_edit_logger() -> &'static EditBlockLogger {
    EDIT_BLOCK_LOGGER.get_or_init(EditBlockLogger::new)
}
