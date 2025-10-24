use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use termcolor::{BufferWriter, ColorChoice};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{OnceCell, RwLock};

const MAX_HISTORY_ENTRIES: usize = 1000;
const MAX_DISK_ENTRIES: usize = 5000;
const ROTATION_CHECK_INTERVAL: usize = 100;

/// Single tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// ISO 8601 timestamp (UTC)
    pub timestamp: String,

    /// Name of the tool that was called
    pub tool_name: String,

    /// Arguments passed to the tool (as JSON)
    pub arguments: serde_json::Value,

    /// Output returned by the tool (as JSON)
    pub output: serde_json::Value,

    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Tool call history manager with in-memory cache and disk persistence
pub struct ToolHistory {
    /// In-memory entries (last 1000)
    /// NOTE: Only accessed by background task after refactor
    entries: Arc<RwLock<VecDeque<ToolCallRecord>>>,

    /// Path to JSONL history file
    history_file: PathBuf,

    /// Write queue for async batching
    /// NOTE: Only accessed by background task after refactor
    write_queue: Arc<RwLock<Vec<ToolCallRecord>>>,

    /// Fire-and-forget channel for recording calls
    record_sender: tokio::sync::mpsc::UnboundedSender<ToolCallRecord>,

    /// Counter for rotation check
    writes_since_check: Arc<RwLock<usize>>,
}

// Remove Default impl since new() is now async

impl ToolHistory {
    /// Create new history manager and start background writer
    pub async fn new(instance_id: String) -> Self {
        // Determine history file location
        let history_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kodegen-mcp");

        // Create directory if needed (async)
        if let Err(e) = tokio::fs::create_dir_all(&history_dir).await {
            let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
            let mut buffer = bufwtr.buffer();
            let _ = writeln!(&mut buffer, "Failed to create history directory: {e}");
            let _ = bufwtr.print(&buffer);
        }

        let history_file = history_dir.join(format!("tool-history_{instance_id}.jsonl"));

        // Create unbounded channel for fire-and-forget recording
        let (record_sender, record_receiver) = tokio::sync::mpsc::unbounded_channel();

        let history = Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            history_file: history_file.clone(),
            write_queue: Arc::new(RwLock::new(Vec::new())),
            record_sender,
            writes_since_check: Arc::new(RwLock::new(0)),
        };

        // Load existing history from disk
        history.load_from_disk().await;

        // Start background processor
        history.start_background_processor(record_receiver);

        history
    }

    /// Add a tool call to history (fire-and-forget, never blocks)
    pub fn add_call(
        &self,
        tool_name: String,
        arguments: serde_json::Value,
        output: serde_json::Value,
        duration_ms: Option<u64>,
    ) {
        let record = ToolCallRecord {
            timestamp: Utc::now().to_rfc3339(),
            tool_name,
            arguments,
            output,
            duration_ms,
        };

        // Fire-and-forget: send to background processor
        // If send fails (channel closed), silently ignore - history is best-effort
        let _ = self.record_sender.send(record);
    }

    /// Get recent tool calls with optional filters and offset support
    pub async fn get_recent_calls(
        &self,
        max_results: usize,
        offset: i64,
        tool_name: Option<&str>,
        since: Option<&str>,
    ) -> Vec<ToolCallRecord> {
        // Parse since timestamp if provided
        let since_dt = since.and_then(|s| DateTime::parse_from_rfc3339(s).ok());

        // Filter entries and clone, dropping lock early
        let filtered: Vec<_> = {
            let entries = self.entries.read().await;
            entries
                .iter()
                .filter(|record| {
                    // Filter by tool name
                    if let Some(name) = tool_name
                        && record.tool_name != name
                    {
                        return false;
                    }

                    // Filter by timestamp
                    if let Some(since_dt) = since_dt
                        && let Ok(record_dt) = DateTime::parse_from_rfc3339(&record.timestamp)
                        && record_dt < since_dt
                    {
                        return false;
                    }

                    true
                })
                .cloned()
                .collect()
        };

        // Apply offset-based pagination
        let total = filtered.len();

        let (start, end) = if offset < 0 {
            // Negative offset: tail behavior (max_results ignored)
            let tail_count = usize::try_from(-offset).unwrap_or(0).min(total);
            let start = total.saturating_sub(tail_count);
            (start, total)
        } else {
            // Positive offset: standard forward reading with max_results
            let limit = max_results.min(MAX_HISTORY_ENTRIES);
            let start = usize::try_from(offset).unwrap_or(0).min(total);
            let end = (start + limit).min(total);
            (start, end)
        };

        filtered[start..end].to_vec()
    }

    /// Get history statistics
    pub async fn get_stats(&self) -> HistoryStats {
        let entries = self.entries.read().await;
        HistoryStats {
            total_entries: entries.len(),
            oldest_entry: entries.front().map(|e| e.timestamp.clone()),
            newest_entry: entries.back().map(|e| e.timestamp.clone()),
        }
    }

    /// Load history from disk (JSONL format)
    async fn load_from_disk(&self) {
        if !tokio::fs::try_exists(&self.history_file)
            .await
            .unwrap_or(false)
        {
            return;
        }

        match tokio::fs::read_to_string(&self.history_file).await {
            Ok(content) => {
                let mut entries = VecDeque::new();

                // Parse each line as JSON
                for line in content.lines() {
                    if let Ok(record) = serde_json::from_str::<ToolCallRecord>(line) {
                        entries.push_back(record);
                    }
                }

                // Keep only last 1000 entries
                while entries.len() > MAX_HISTORY_ENTRIES {
                    entries.pop_front();
                }

                *self.entries.write().await = entries;
            }
            Err(e) => {
                let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                let mut buffer = bufwtr.buffer();
                let _ = writeln!(&mut buffer, "Failed to load tool history: {e}");
                let _ = bufwtr.print(&buffer);
            }
        }
    }

    /// Start background processor task (receives records, updates cache, writes to disk)
    fn start_background_processor(
        &self,
        mut record_receiver: tokio::sync::mpsc::UnboundedReceiver<ToolCallRecord>,
    ) {
        let entries = Arc::clone(&self.entries);
        let write_queue = Arc::clone(&self.write_queue);
        let writes_since_check = Arc::clone(&self.writes_since_check);
        let history_file = self.history_file.clone();

        tokio::spawn(async move {
            // Disk flush interval (1 second)
            let mut flush_interval = tokio::time::interval(std::time::Duration::from_secs(1));

            loop {
                tokio::select! {
                    // Receive new records from channel
                    Some(record) = record_receiver.recv() => {
                        // Update in-memory cache
                        {
                            let mut entries = entries.write().await;
                            entries.push_back(record.clone());

                            // Keep only last 1000 in memory
                            if entries.len() > MAX_HISTORY_ENTRIES {
                                entries.pop_front();
                            }
                        }

                        // Queue for disk write
                        {
                            let mut queue = write_queue.write().await;
                            queue.push(record);
                        }
                    }

                    // Periodic disk flush
                    _ = flush_interval.tick() => {
                        // Get queued records
                        let records = {
                            let mut queue = write_queue.write().await;
                            std::mem::take(&mut *queue)
                        };

                        if records.is_empty() {
                            continue;
                        }

                        // Append to file (JSONL format)
                        match OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&history_file)
                            .await
                        {
                            Ok(mut file) => {
                                for record in &records {
                                    if let Ok(json) = serde_json::to_string(record) {
                                        let line = format!("{json}\n");
                                        let _ = file.write_all(line.as_bytes()).await;
                                    }
                                }
                            }
                            Err(e) => {
                                let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                                let mut buffer = bufwtr.buffer();
                                let _ = writeln!(&mut buffer, "Failed to write tool history: {e}");
                                let _ = bufwtr.print(&buffer);
                                continue;
                            }
                        }

                        // Check if rotation is needed
                        let should_rotate = {
                            let mut check_counter = writes_since_check.write().await;
                            *check_counter += records.len();

                            if *check_counter >= ROTATION_CHECK_INTERVAL {
                                *check_counter = 0;
                                true
                            } else {
                                false
                            }
                        };

                        if should_rotate {
                            // Perform rotation check
                            if let Err(e) = Self::rotate_if_needed(&history_file).await {
                                let bufwtr = BufferWriter::stderr(ColorChoice::Auto);
                                let mut buffer = bufwtr.buffer();
                                let _ = writeln!(&mut buffer, "Failed to rotate tool history: {e}");
                                let _ = bufwtr.print(&buffer);
                            }
                        }
                    }

                    // Channel closed (shutdown)
                    else => {
                        // Flush any remaining records before exiting
                        let records = {
                            let queue = write_queue.read().await;
                            queue.clone()
                        };

                        if !records.is_empty()
                            && let Ok(mut file) = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&history_file)
                                .await
                        {
                            for record in &records {
                                if let Ok(json) = serde_json::to_string(record) {
                                    let line = format!("{json}\n");
                                    let _ = file.write_all(line.as_bytes()).await;
                                }
                            }
                        }

                        break;
                    }
                }
            }
        });
    }

    /// Check if file needs rotation and rotate if necessary
    ///
    /// This is called periodically by the background writer when the write counter
    /// reaches `ROTATION_CHECK_INTERVAL`. If the file has more than `MAX_DISK_ENTRIES`
    /// lines, it keeps only the last `MAX_DISK_ENTRIES` lines and atomically replaces
    /// the file using a temp file + rename strategy.
    async fn rotate_if_needed(history_file: &PathBuf) -> Result<(), std::io::Error> {
        // Read current file
        let content = match tokio::fs::read_to_string(history_file).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };

        // Count lines
        let line_count = content.lines().count();

        // Only rotate if exceeds limit
        if line_count <= MAX_DISK_ENTRIES {
            return Ok(());
        }

        // Keep only the last MAX_DISK_ENTRIES lines
        let keep_from = line_count.saturating_sub(MAX_DISK_ENTRIES);
        let kept_lines: Vec<&str> = content.lines().skip(keep_from).collect();

        // Write to temporary file (atomic operation step 1)
        let temp_file = history_file.with_extension("jsonl.tmp");
        {
            let mut file = tokio::fs::File::create(&temp_file).await?;
            for line in kept_lines {
                file.write_all(line.as_bytes()).await?;
                file.write_all(b"\n").await?;
            }
            file.sync_all().await?;
        }

        // Atomic rename (atomic operation step 2)
        // On Unix systems (including macOS), this is an atomic filesystem operation
        tokio::fs::rename(&temp_file, history_file).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryStats {
    pub total_entries: usize,
    pub oldest_entry: Option<String>,
    pub newest_entry: Option<String>,
}

// ============================================================================
// GLOBAL INSTANCE
// ============================================================================

static TOOL_HISTORY: OnceCell<ToolHistory> = OnceCell::const_new();

/// Initialize the global tool history instance (call once in main.rs)
pub async fn init_global_history(instance_id: String) -> &'static ToolHistory {
    TOOL_HISTORY
        .get_or_init(|| async move { ToolHistory::new(instance_id).await })
        .await
}

/// Get the global tool history instance (returns None if not initialized)
pub fn get_global_history() -> Option<&'static ToolHistory> {
    TOOL_HISTORY.get()
}
