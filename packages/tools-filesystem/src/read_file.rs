use crate::validate_path;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use kodegen_mcp_schema::filesystem::{ReadFileArgs, ReadFilePromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use mime_guess::from_path;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::collections::VecDeque;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{Duration, timeout};

// ============================================================================
// HELPERS
// ============================================================================

fn is_image_mime(mt: &str) -> bool {
    mt.to_lowercase().starts_with("image/")
}

/// Read lines starting from offset, returning both lines and total count in ONE pass
///
/// This is more efficient than separate counting because we iterate once:
/// - Count all lines while processing
/// - Skip lines until we reach start position
/// - Collect lines until we reach start + count
///
/// Memory: O(count) instead of `O(total_lines)`
async fn read_lines_forward_with_total(
    path: &std::path::Path,
    start: usize,
    count: usize,
) -> Result<(Vec<String>, Option<usize>), McpError> {
    let file = tokio::fs::File::open(path).await?;

    let reader = BufReader::new(file);
    let mut lines_stream = reader.lines();

    let mut result = Vec::with_capacity(count);
    let mut line_number = 0;

    while let Some(line) = lines_stream.next_line().await? {
        // Collect lines in target range
        if line_number >= start && result.len() < count {
            result.push(line);
        }

        line_number += 1;

        // Early exit optimization: we have enough lines, skip total count
        if result.len() == count {
            return Ok((result, None));
        }
    }

    Ok((result, Some(line_number)))
}

/// Read last N lines using ring buffer, returning both lines and total count in ONE pass
///
/// Uses `VecDeque` as circular buffer:
/// - Capacity = `tail_count` (memory efficient)
/// - Pop oldest when full, push newest
/// - Result: last N lines + total line count
///
/// Memory: `O(tail_count)` instead of `O(total_lines)`
async fn read_lines_tail_with_total(
    path: &std::path::Path,
    tail_count: usize,
) -> Result<(Vec<String>, Option<usize>), McpError> {
    let file = tokio::fs::File::open(path).await?;

    let reader = BufReader::new(file);
    let mut lines_stream = reader.lines();

    // Handle tail_count = 0 edge case
    if tail_count == 0 {
        // Count total lines but don't collect any
        let mut total_lines = 0;
        while lines_stream.next_line().await?.is_some() {
            total_lines += 1;
        }
        return Ok((Vec::new(), Some(total_lines)));
    }

    // Ring buffer for last N lines
    let mut ring_buffer = VecDeque::with_capacity(tail_count);
    let mut total_lines = 0;

    while let Some(line) = lines_stream.next_line().await? {
        if ring_buffer.len() == tail_count {
            ring_buffer.pop_front(); // Remove oldest
        }
        ring_buffer.push_back(line);
        total_lines += 1;
    }

    Ok((ring_buffer.into_iter().collect(), Some(total_lines)))
}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ReadFileTool {
    default_line_limit: usize,
    config_manager: kodegen_tools_config::ConfigManager,
}

impl ReadFileTool {
    #[must_use]
    pub fn new(
        default_line_limit: usize,
        config_manager: kodegen_tools_config::ConfigManager,
    ) -> Self {
        Self {
            default_line_limit,
            config_manager,
        }
    }

    /// Read file from disk with full validation
    async fn read_file_from_disk(
        &self,
        path: &str,
        offset: i64,
        length: Option<usize>,
    ) -> Result<Value, McpError> {
        let valid_path = validate_path(path, &self.config_manager).await?;

        let guessed = from_path(&valid_path)
            .first_or_octet_stream()
            .essence_str()
            .to_owned();

        let is_img = is_image_mime(&guessed);

        // If image, read as base64
        if is_img {
            let bytes = fs::read(&valid_path).await?;
            let base64_content = BASE64.encode(&bytes);

            return Ok(json!({
                "content": base64_content,
                "mime_type": guessed,
                "is_image": true,
                "size_bytes": bytes.len()
            }));
        }

        // Handle text files - use streaming to avoid loading entire file
        let (lines_vec, total) = if offset < 0 {
            // Tail behavior: read last N lines
            let tail_count = usize::try_from(-offset).unwrap_or(0);
            read_lines_tail_with_total(&valid_path, tail_count).await?
        } else {
            // Forward read: skip to offset, read length lines
            let start = usize::try_from(offset).unwrap_or(0);
            let count = length.unwrap_or(self.default_line_limit);
            read_lines_forward_with_total(&valid_path, start, count).await?
        };

        let lines_read = lines_vec.len();
        let truncated = lines_vec.join("\n");

        // Calculate start/end for notice formatting
        let (start, end) = if offset < 0 {
            // Tail reads always have total
            if let Some(t) = total {
                let start = t.saturating_sub(lines_read);
                (start, t)
            } else {
                // Fallback (should not happen for tail reads)
                (0, lines_read)
            }
        } else {
            let start = usize::try_from(offset).unwrap_or(0);
            let end = start + lines_read;
            (start, end)
        };

        let mut content = truncated;

        // Determine if this is a partial read
        let is_partial = offset != 0 || total.is_none_or(|t| end < t);

        // If partial read, add a notice
        if is_partial {
            let notice = if offset < 0 {
                // Tail reads always have total
                if let Some(t) = total {
                    format!(
                        "[Reading last {} lines (lines {}-{}) of {} total lines]\n\n",
                        end - start,
                        start,
                        end - 1,
                        t
                    )
                } else {
                    // Fallback (should not happen for tail reads)
                    format!("[Reading last {} lines]\n\n", end - start)
                }
            } else {
                // Forward read: may or may not have total
                match total {
                    Some(t) => format!(
                        "[Reading {} lines from line {} of {} total lines]\n\n",
                        end - start,
                        start,
                        t
                    ),
                    None => format!("[Reading {} lines from line {}]\n\n", end - start, start),
                }
            };
            content = format!("{notice}{content}");
        }

        Ok(json!({
            "content": content,
            "mime_type": guessed,
            "is_image": false,
            "total_lines": total,
            "lines_read": end - start,
            "is_partial": is_partial
        }))
    }

    /// Read file from URL with timeout
    async fn read_file_from_url(&self, url: &str) -> Result<Value, McpError> {
        const FETCH_TIMEOUT_MS: u64 = 30000;

        let fetch_operation = async {
            let resp = reqwest::get(url)
                .await
                .map_err(|e| McpError::Network(e.to_string()))?;

            if !resp.status().is_success() {
                return Err(McpError::Network(format!(
                    "HTTP error, status: {}",
                    resp.status()
                )));
            }

            let content_type = resp
                .headers()
                .get("content-type")
                .map_or("text/plain", |v| v.to_str().unwrap_or("text/plain"))
                .to_owned();

            let bytes = resp
                .bytes()
                .await
                .map_err(|e| McpError::Network(e.to_string()))?;

            let is_image = is_image_mime(&content_type);

            if is_image {
                let base64_content = BASE64.encode(&bytes);
                Ok(json!({
                    "content": base64_content,
                    "mime_type": content_type,
                    "is_image": true,
                    "size_bytes": bytes.len()
                }))
            } else {
                let content = String::from_utf8_lossy(&bytes).to_string();
                Ok(json!({
                    "content": content,
                    "mime_type": content_type,
                    "is_image": false,
                    "size_bytes": bytes.len()
                }))
            }
        };

        match timeout(Duration::from_millis(FETCH_TIMEOUT_MS), fetch_operation).await {
            Ok(result) => result,
            Err(_) => Err(McpError::Other(anyhow::anyhow!(
                "Fetch timed out after {FETCH_TIMEOUT_MS}ms: {url}"
            ))),
        }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ReadFileTool {
    type Args = ReadFileArgs;
    type PromptArgs = ReadFilePromptArgs;

    fn name() -> &'static str {
        "read_file"
    }

    fn description() -> &'static str {
        "Read the contents of a file from the filesystem or a URL. Supports text files (returned as text) \
         and image files (returned as base64). Use offset and length parameters to read specific \
         portions of large files. Supports negative offsets for tail behavior (offset: -N reads last N lines). \
         When offset is negative, length is ignored. Automatically validates paths and handles symlinks."
    }

    fn read_only() -> bool {
        true
    }

    fn open_world() -> bool {
        true // Can read from URLs
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Auto-detect URL if not specified
        let is_url =
            args.is_url || args.path.starts_with("http://") || args.path.starts_with("https://");

        if is_url {
            self.read_file_from_url(&args.path).await
        } else {
            self.read_file_from_disk(&args.path, args.offset, args.length)
                .await
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "file_type".to_string(),
            title: None,
            description: Some(
                "Optional file type to focus examples on (e.g., 'json', 'rust', 'markdown')"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the read_file tool to read a large file in chunks?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The read_file tool supports reading large files in chunks using offset and length parameters:\n\n\
                     1. Basic usage: read_file({\"path\": \"file.txt\"})\n\
                     2. Read first 100 lines: read_file({\"path\": \"file.txt\", \"length\": 100})\n\
                     3. Read lines 100-200: read_file({\"path\": \"file.txt\", \"offset\": 100, \"length\": 100})\n\
                     4. Read last 30 lines: read_file({\"path\": \"file.txt\", \"offset\": -30})\n\
                     5. Read last 5 lines: read_file({\"path\": \"file.txt\", \"offset\": -5})\n\
                     6. Read from URL: read_file({\"path\": \"https://example.com/data.json\", \"is_url\": true})\n\n\
                     The tool automatically:\n\
                     - Detects and validates file paths (expands ~, resolves symlinks)\n\
                     - Detects image files and returns them as base64\n\
                     - Adds partial read notices for text files\n\
                     - Handles URL fetching with 30-second timeout\n\
                     - Validates paths are within allowed directories\n\
                     - Ignores length parameter when offset is negative (tail behavior)",
                ),
            },
        ])
    }
}
