//! Embedding Usage Tracking Types and Utilities
//!
//! This module provides types for tracking token usage in embedding operations.

use serde::{Deserialize, Serialize};

/// Tracks token usage statistics for embedding operations
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingUsage {
    /// Number of tokens in the input prompt
    pub prompt_tokens: u32,

    /// Total tokens used (including prompt and any overhead)
    pub total_tokens: u32,
}

impl EmbeddingUsage {
    /// Create a new `EmbeddingUsage` with the given token counts
    #[inline]
    #[must_use]
    pub const fn new(prompt_tokens: u32, total_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            total_tokens,
        }
    }

    /// Create a new `EmbeddingUsage` with the same value for both prompt and total tokens
    #[inline]
    #[must_use]
    pub const fn from_prompt_tokens(prompt_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            total_tokens: prompt_tokens,
        }
    }

    /// Add another `EmbeddingUsage` to this one
    #[inline]
    #[must_use = "Returns a new EmbeddingUsage without modifying the original"]
    pub const fn add(&self, other: &Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
        }
    }

    /// Calculate the overhead tokens (`total_tokens` - `prompt_tokens`)
    #[inline]
    #[must_use]
    pub const fn overhead_tokens(&self) -> u32 {
        self.total_tokens.saturating_sub(self.prompt_tokens)
    }
}
impl std::ops::Add for EmbeddingUsage {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            prompt_tokens: self.prompt_tokens + rhs.prompt_tokens,
            total_tokens: self.total_tokens + rhs.total_tokens,
        }
    }
}

impl std::ops::AddAssign for EmbeddingUsage {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(&rhs);
    }
}

/// Trait for types that can report token usage
pub trait TokenUsage {
    /// Get the token usage for this operation
    fn token_usage(&self) -> Option<EmbeddingUsage>;
}
