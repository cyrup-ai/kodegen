//! Utility modules for domain types and operations

use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod json_util;
pub mod notnan;

// Re-export commonly used types
pub use notnan::{NotNan, NotNanError};

// Duration and timestamp conversion helpers to avoid u128→u64 cast warnings

/// Safely convert Duration to u64 nanoseconds with saturation
#[inline]
#[must_use]
pub fn duration_to_nanos_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// Safely convert Duration to u64 microseconds with saturation  
#[inline]
#[must_use]
pub fn duration_to_micros_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// Safely convert Duration to u64 milliseconds with saturation
#[inline]
#[must_use]
pub fn duration_to_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Get current UNIX timestamp in nanoseconds as u64
#[inline]
#[must_use]
pub fn unix_timestamp_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_nanos()).ok())
        .unwrap_or(0)
}

/// Get current UNIX timestamp in microseconds as u64
#[inline]
#[must_use]
pub fn unix_timestamp_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_micros()).ok())
        .unwrap_or(0)
}
