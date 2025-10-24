//! Tokenizer Module
//!
//! Provides tokenization utilities for the Candle ML framework.
//! This module contains shared tokenization infrastructure used across
//! text generation and embedding capabilities.

pub mod core;

// Re-export main tokenizer types for convenient access
pub use core::{
    CandlePaddingStrategy, CandleStreamingTokenizer, CandleTokenizer, CandleTokenizerConfig,
    CandleTokenizerError, CandleTokenizerStats,
};
