//! `NotNaN` utility for safe floating point comparisons
//!
//! This module provides a wrapper type for floating point numbers that guarantees
//! the value is not NaN, enabling safe comparisons and ordering operations.

use std::cmp::Ordering;
use std::fmt;

/// A wrapper around f32 that guarantees the value is not NaN
#[derive(Debug, Clone, Copy)]
pub struct NotNan<T>(T);

impl NotNan<f32> {
    /// Create a new `NotNan` from an f32, returning an error if the value is NaN
    ///
    /// # Errors
    ///
    /// Returns `NotNanError` if the value is NaN
    #[inline]
    pub fn new(val: f32) -> Result<Self, NotNanError> {
        if val.is_nan() {
            Err(NotNanError)
        } else {
            Ok(NotNan(val))
        }
    }

    /// Get the inner f32 value
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> f32 {
        self.0
    }
}

impl AsRef<f32> for NotNan<f32> {
    /// Get a reference to the inner f32 value
    #[inline]
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl PartialEq for NotNan<f32> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for NotNan<f32> {}

impl PartialOrd for NotNan<f32> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NotNan<f32> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Since we guarantee no NaN values, this is safe
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

impl fmt::Display for NotNan<f32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Error type returned when trying to create a `NotNan` from a NaN value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotNanError;

impl fmt::Display for NotNanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "attempted to create NotNan from NaN value")
    }
}

impl std::error::Error for NotNanError {}
