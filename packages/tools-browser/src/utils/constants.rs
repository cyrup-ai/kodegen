//! Shared configuration constants for browser tools
//!
//! This module contains default values and configuration constants used
//! throughout the codebase to ensure consistency and avoid magic numbers.

/// Chrome user agent string for stealth mode
///
/// Updated: 2025-01-29 to Chrome 132 (current stable)
/// Next update: 2025-04-29 (quarterly schedule)
///
/// Chrome releases new stable versions ~every 4 weeks.
/// Update quarterly to stay within reasonable version window.
///
/// Reference: https://chromiumdash.appspot.com/schedule
pub const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.6834.160 Safari/537.36";
