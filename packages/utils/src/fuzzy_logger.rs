//! Fuzzy search logging for `edit_block` failures
//!
//! Logs fuzzy match attempts to ~/.kodegen-logs/fuzzy-search.log
//! for debugging and analysis. Format: tab-separated values (TSV)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzySearchLogEntry {
    pub timestamp: DateTime<Utc>,
    pub search_text: String,
    pub found_text: String,
    pub similarity: f64,
    pub execution_time_ms: f64,
    pub exact_match_count: usize,
    pub expected_replacements: usize,
    pub fuzzy_threshold: f64,
    pub below_threshold: bool,
    pub diff: String,
    pub search_length: usize,
    pub found_length: usize,
    pub file_extension: String,
}

pub struct FuzzyLogger {
    log_path: PathBuf,
}

impl Default for FuzzyLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzyLogger {
    /// Create a new fuzzy logger with default path
    #[must_use]
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let log_dir = home.join(".kodegen-logs");
        let log_path = log_dir.join("fuzzy-search.log");

        Self { log_path }
    }

    /// Get the log file path
    #[must_use]
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Ensure log directory and file exist
    async fn ensure_log_file(&self) -> Result<(), std::io::Error> {
        let log_dir = self.log_path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid log path: no parent directory",
            )
        })?;
        fs::create_dir_all(log_dir).await?;

        // Check if file exists (async)
        if !fs::try_exists(&self.log_path).await.unwrap_or(false) {
            // Create with headers
            let headers = [
                "timestamp",
                "search_text",
                "found_text",
                "similarity",
                "execution_time_ms",
                "exact_match_count",
                "expected_replacements",
                "fuzzy_threshold",
                "below_threshold",
                "diff",
                "search_length",
                "found_length",
                "file_extension",
            ]
            .join("\t");

            fs::write(&self.log_path, format!("{headers}\n")).await?;
        }

        Ok(())
    }

    /// Log a fuzzy search attempt
    pub async fn log(&self, entry: &FuzzySearchLogEntry) -> Result<(), std::io::Error> {
        self.ensure_log_file().await?;

        // Escape tabs and newlines
        let escape = |s: &str| s.replace('\n', "\\n").replace('\t', "\\t");

        let line = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            entry.timestamp.to_rfc3339(),
            escape(&entry.search_text),
            escape(&entry.found_text),
            entry.similarity,
            entry.execution_time_ms,
            entry.exact_match_count,
            entry.expected_replacements,
            entry.fuzzy_threshold,
            entry.below_threshold,
            escape(&entry.diff),
            entry.search_length,
            entry.found_length,
            entry.file_extension,
        );

        // Append to log file
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        file.write_all(line.as_bytes()).await?;

        Ok(())
    }
}

// Global singleton
use tokio::sync::Mutex;

static FUZZY_LOGGER: std::sync::LazyLock<Mutex<FuzzyLogger>> =
    std::sync::LazyLock::new(|| Mutex::new(FuzzyLogger::new()));

/// Get the global fuzzy logger instance
pub async fn get_logger() -> tokio::sync::MutexGuard<'static, FuzzyLogger> {
    FUZZY_LOGGER.lock().await
}
