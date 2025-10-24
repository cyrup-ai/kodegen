//! Token usage tracking for API operations
//!
//! Provides a zero-allocation, thread-safe way to track token usage across
//! different operations in the system.

use serde::{Deserialize, Serialize};

/// Tracks token usage statistics for Candle API operations
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[must_use = "Candle token usage should be handled or logged"]
#[repr(C)]
pub struct CandleUsage {
    /// Number of tokens in the input/prompt
    pub input_tokens: u32,
    /// Number of tokens in the output/completion
    pub output_tokens: u32,
    /// Total tokens used (input + output)
    pub total_tokens: u32,
}

impl CandleUsage {
    /// Creates a new `Usage` instance with the given token counts
    #[inline]
    pub const fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }

    /// Creates a new `Usage` instance with zero tokens
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0, 0)
    }

    /// Adds another `Usage` to this one, returning a new instance
    #[inline]
    #[must_use = "Returns a new Usage instance"]
    pub const fn add(&self, other: &Self) -> Self {
        let input = self.input_tokens + other.input_tokens;
        let output = self.output_tokens + other.output_tokens;
        Self::new(input, output)
    }

    /// Returns whether no tokens have been used
    #[inline]
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.input_tokens == 0 && self.output_tokens == 0
    }
}

impl std::ops::Add for CandleUsage {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let input = self.input_tokens + rhs.input_tokens;
        let output = self.output_tokens + rhs.output_tokens;
        Self::new(input, output)
    }
}

impl std::ops::AddAssign for CandleUsage {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(&rhs);
    }
}

/// Type alias for backward compatibility
pub type Usage = CandleUsage;
