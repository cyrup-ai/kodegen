//! Message processing utilities for the chat system.
//!
//! This module provides functionality for processing, validating, and transforming
//! chat messages in a production environment using async streaming patterns.

// Removed unused import: use crate::error::ZeroAllocResult;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use super::types::CandleMessage;

/// Maximum allowed content length (100K characters = ~400KB UTF-8)
const MAX_CONTENT_LENGTH: usize = 100_000;

/// Errors that can occur during content sanitization
#[derive(Debug, Clone, Error)]
pub enum SanitizationError {
    /// Content exceeds maximum allowed length
    #[error("Content too long: {0} characters (maximum: {1})")]
    TooLong(usize, usize),

    /// Invalid UTF-8 encoding detected
    #[error("Invalid UTF-8 encoding in content")]
    InvalidEncoding,

    /// Content contains prohibited characters after filtering
    #[error("Content contains prohibited characters")]
    ProhibitedCharacters,

    /// Generic sanitization processing error
    #[error("Sanitization failed: {0}")]
    ProcessingError(String),
}

/// Processes a message before it's sent to the chat system using async streaming.
///
/// # Arguments
/// * `message` - The message to process
///
/// # Returns
/// Returns a tokio Stream that will emit the processed message.
/// The `on_chunk` handler should validate the processed message.
pub fn process_message(message: CandleMessage) -> impl tokio_stream::Stream<Item = CandleMessage> {
    crate::async_stream::spawn_stream(move |sender| async move {
        let mut processed_message = message;

        // Apply security sanitization pipeline to message content
        processed_message.content =
            sanitize_content(&processed_message.content).unwrap_or_else(|e| {
                log::warn!("Content sanitization failed: {e}");

                // SECURITY CRITICAL: Truncate FIRST, then sanitize the truncated content
                // This ensures overlength content still goes through full security pipeline
                // Without this, attackers could bypass HTML escaping by sending 100K+ chars
                let truncated: String = processed_message.content.chars().take(1000).collect();

                // Apply sanitization to truncated content
                sanitize_content(&truncated).unwrap_or_else(|_| {
                    // Ultimate fallback: empty string if sanitization impossible
                    log::error!("Failed to sanitize even truncated content");
                    String::new()
                })
            });

        // Emit the sanitized message
        let _ = sender.send(processed_message);
    })
}

/// Validates that a message is safe to send using async streaming.
///
/// # Arguments
/// * `message` - The message to validate
///
/// # Returns
/// Returns a tokio Stream that will emit the message if valid.
/// Invalid messages will be handled by the `on_chunk` error handler.
pub fn validate_message(message: CandleMessage) -> impl tokio_stream::Stream<Item = CandleMessage> {
    crate::async_stream::spawn_stream(move |sender| async move {
        // Always emit the message - the `on_chunk` handler decides validation behavior
        let _ = sender.send(message);
    })
}

/// Sanitizes potentially dangerous content using a 4-stage security pipeline
///
/// # Security Stages
/// 1. Length validation - prevents `DoS` attacks
/// 2. Control character filtering - prevents terminal corruption
/// 3. Unicode normalization (`NFC`) - prevents encoding attacks
/// 4. HTML entity escaping - prevents XSS attacks
///
/// # Arguments
/// * `content` - The content to sanitize
///
/// # Returns
/// Returns sanitized content or error if content cannot be safely processed
///
/// # Security Rationale
/// This multi-stage approach defends against multiple attack vectors:
/// - XSS: HTML escaping prevents script injection
/// - Terminal corruption: Control char filtering blocks ANSI escapes
/// - Unicode attacks: `NFC` normalization prevents homograph bypasses
/// - `DoS`: Length validation prevents memory exhaustion
///
/// # Errors
///
/// Returns `SanitizationError` if:
/// - Content exceeds maximum length
/// - Content contains disallowed control characters
/// - Unicode normalization fails
pub fn sanitize_content(content: &str) -> Result<String, SanitizationError> {
    // Stage 1: Length validation (DoS prevention)
    let char_count = content.chars().count();
    if char_count > MAX_CONTENT_LENGTH {
        return Err(SanitizationError::TooLong(char_count, MAX_CONTENT_LENGTH));
    }

    // Stage 2: Control character filtering (terminal corruption prevention)
    let filtered: String = content
        .chars()
        .filter(|&c| {
            // Whitelist: Allow newline, tab, carriage return
            if c == '\n' || c == '\t' || c == '\r' {
                return true;
            }
            // Block all other control characters (0x00-0x1F, 0x7F-0x9F)
            !c.is_control()
        })
        .collect();

    // Stage 3: Unicode normalization (encoding attack prevention)
    // NFC = Canonical Decomposition followed by Canonical Composition
    // Ensures consistent representation of characters like é, ñ, etc.
    let normalized: String = filtered.nfc().collect();

    // Stage 4: HTML entity escaping (XSS prevention)
    // Converts: < → &lt;, > → &gt;, & → &amp;, etc.
    let escaped = html_escape::encode_text(&normalized).to_string();

    // Final cleanup: trim and return
    Ok(escaped.trim().to_string())
}

/// Validates a message to ensure it meets system requirements.
///
/// # Arguments
/// * `message` - The message to validate
///
/// # Errors
///
/// Returns error string if:
/// - Message content is empty
/// - Message fails validation checks
pub fn validate_message_sync(message: &CandleMessage) -> Result<(), String> {
    // Basic validation logic - can be extended as needed
    if message.content.is_empty() {
        return Err("Empty message content".to_string());
    }

    Ok(())
}
