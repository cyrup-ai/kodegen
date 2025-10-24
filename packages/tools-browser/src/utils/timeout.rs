//! Timeout validation utilities for browser operations

use std::time::Duration;
use kodegen_mcp_tool::error::McpError;

/// Maximum timeout for browser navigation operations (5 minutes)
/// Covers slow-loading sites, heavy SPAs, and network delays
pub const MAX_NAVIGATION_TIMEOUT_MS: u64 = 300_000; // 5 minutes

/// Maximum timeout for element interaction operations (30 seconds)
/// Covers dynamic element loading and animations
pub const MAX_INTERACTION_TIMEOUT_MS: u64 = 30_000; // 30 seconds

/// Maximum timeout for explicit wait operations (30 seconds)
/// Prevents accidental infinite waits in automation scripts
pub const MAX_WAIT_TIMEOUT_MS: u64 = 30_000; // 30 seconds

/// Validate timeout for navigation operations (navigate, wait_for_selector)
///
/// # Arguments
/// * `timeout_ms` - Optional timeout in milliseconds
/// * `default_ms` - Default timeout if None provided
///
/// # Returns
/// * `Ok(Duration)` - Validated Duration object
/// * `Err(McpError)` - If timeout exceeds MAX_NAVIGATION_TIMEOUT_MS
///
/// # Example
/// ```rust
/// let timeout = validate_navigation_timeout(Some(45000), 30000)?;
/// ```
pub fn validate_navigation_timeout(
    timeout_ms: Option<u64>,
    default_ms: u64,
) -> Result<Duration, McpError> {
    let ms = timeout_ms.unwrap_or(default_ms);
    
    if ms > MAX_NAVIGATION_TIMEOUT_MS {
        return Err(McpError::invalid_arguments(
            format!(
                "Timeout cannot exceed {}ms ({} minutes). Received: {}ms ({:.1} minutes)",
                MAX_NAVIGATION_TIMEOUT_MS,
                MAX_NAVIGATION_TIMEOUT_MS / 60_000,
                ms,
                ms as f64 / 60_000.0
            )
        ));
    }
    
    Ok(Duration::from_millis(ms))
}

/// Validate timeout for element interaction operations (click, type_text)
///
/// # Arguments
/// * `timeout_ms` - Optional timeout in milliseconds
/// * `default_ms` - Default timeout if None provided
///
/// # Returns
/// * `Ok(Duration)` - Validated Duration object
/// * `Err(McpError)` - If timeout exceeds MAX_INTERACTION_TIMEOUT_MS
pub fn validate_interaction_timeout(
    timeout_ms: Option<u64>,
    default_ms: u64,
) -> Result<Duration, McpError> {
    let ms = timeout_ms.unwrap_or(default_ms);
    
    if ms > MAX_INTERACTION_TIMEOUT_MS {
        return Err(McpError::invalid_arguments(
            format!(
                "Timeout cannot exceed {}ms ({} seconds). Received: {}ms ({} seconds)",
                MAX_INTERACTION_TIMEOUT_MS,
                MAX_INTERACTION_TIMEOUT_MS / 1000,
                ms,
                ms / 1000
            )
        ));
    }
    
    Ok(Duration::from_millis(ms))
}

/// Validate timeout for explicit wait operations
///
/// # Arguments
/// * `duration_ms` - Wait duration in milliseconds
///
/// # Returns
/// * `Ok(Duration)` - Validated Duration object
/// * `Err(McpError)` - If duration exceeds MAX_WAIT_TIMEOUT_MS
pub fn validate_wait_timeout(duration_ms: u64) -> Result<Duration, McpError> {
    if duration_ms > MAX_WAIT_TIMEOUT_MS {
        return Err(McpError::invalid_arguments(
            format!(
                "Duration cannot exceed {}ms ({} seconds). Received: {}ms ({} seconds)",
                MAX_WAIT_TIMEOUT_MS,
                MAX_WAIT_TIMEOUT_MS / 1000,
                duration_ms,
                duration_ms / 1000
            )
        ));
    }
    
    Ok(Duration::from_millis(duration_ms))
}
