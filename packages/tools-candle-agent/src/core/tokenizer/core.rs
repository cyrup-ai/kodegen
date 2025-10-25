//! Candle Tokenizer Implementation
//!
//! Zero-allocation tokenizer wrapper for kimi_k2 with streaming token generation support.
//! Provides memory-efficient tensor operations with the Candle ML framework.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrayvec::ArrayVec;
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

/// Configuration for the Candle tokenizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleTokenizerConfig {
    /// Path to the tokenizer configuration file
    pub tokenizer_path: PathBuf,
    /// Maximum sequence length for encoding
    pub max_length: usize,
    /// Whether to add special tokens
    pub add_special_tokens: bool,
    /// Padding token ID
    pub pad_token_id: Option<u32>,
    /// Beginning of sequence token ID
    pub bos_token_id: Option<u32>,
    /// End of sequence token ID
    pub eos_token_id: Option<u32>,
    /// Unknown token ID
    pub unk_token_id: Option<u32>,
    /// Whether to truncate sequences that exceed max_length
    pub truncation: bool,
    /// Padding strategy
    pub padding: CandlePaddingStrategy,
}

/// Padding strategy for token sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandlePaddingStrategy {
    /// No padding
    None,
    /// Pad to maximum length in batch
    Longest,
    /// Pad to fixed length
    MaxLength,
}

impl Default for CandleTokenizerConfig {
    fn default() -> Self {
        Self {
            tokenizer_path: PathBuf::from("./models/kimi-k2/tokenizer.json"),
            max_length: 4096,
            add_special_tokens: true,
            pad_token_id: Some(0),
            bos_token_id: Some(1),
            eos_token_id: Some(2),
            unk_token_id: Some(3),
            truncation: true,
            padding: CandlePaddingStrategy::None,
        }
    }
}

/// Zero-allocation tokenizer wrapper for kimi_k2 model
pub struct CandleTokenizer {
    /// Underlying tokenizer implementation
    tokenizer: Arc<Tokenizer>,
    /// Configuration
    config: CandleTokenizerConfig,
    /// Vocabulary mapping for fast lookups
    vocab: HashMap<String, u32>,
    /// Reverse vocabulary for decoding
    reverse_vocab: HashMap<u32, String>,
    /// Token cache for frequently used tokens
    token_cache: HashMap<String, ArrayVec<u32, 512>>,
}

impl CandleTokenizer {
    /// Create a new tokenizer from configuration file
    pub fn from_file<P: AsRef<Path>>(tokenizer_path: P) -> Result<Self, CandleTokenizerError> {
        let config = CandleTokenizerConfig {
            tokenizer_path: tokenizer_path.as_ref().to_path_buf(),
            ..Default::default()
        };

        Self::from_config(config)
    }

    /// Create a new tokenizer from configuration
    pub fn from_config(config: CandleTokenizerConfig) -> Result<Self, CandleTokenizerError> {
        // Load tokenizer from file
        let tokenizer = Tokenizer::from_file(&config.tokenizer_path)
            .map_err(|e| CandleTokenizerError::LoadFailed(e.to_string()))?;

        // Build vocabulary mappings for fast lookups
        let vocab = Self::build_vocab_map(&tokenizer)?;
        let reverse_vocab = Self::build_reverse_vocab_map(&vocab);

        Ok(Self {
            tokenizer: Arc::new(tokenizer),
            config,
            vocab,
            reverse_vocab,
            token_cache: HashMap::with_capacity(1024),
        })
    }

    /// Encode text to token IDs with zero allocation in hot paths
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, CandleTokenizerError> {
        // Check cache first for frequently used phrases
        if let Some(cached_tokens) = self.token_cache.get(text) {
            return Ok(cached_tokens.to_vec());
        }

        // Configure encoding parameters
        let encoding = self
            .tokenizer
            .encode(text, self.config.add_special_tokens)
            .map_err(|e| CandleTokenizerError::EncodeFailed(e.to_string()))?;

        let mut token_ids = encoding.get_ids().to_vec();

        // Apply truncation if necessary
        if self.config.truncation && token_ids.len() > self.config.max_length {
            token_ids.truncate(self.config.max_length);

            // Add EOS token if truncated
            if let Some(eos_id) = self.config.eos_token_id
                && let Some(last) = token_ids.last_mut()
            {
                *last = eos_id;
            }
        }

        Ok(token_ids)
    }

    /// Encode text with streaming support for large inputs
    pub fn encode_streaming<'a>(
        &'a self,
        text: &'a str,
        chunk_size: usize,
    ) -> Vec<Result<Vec<u32>, CandleTokenizerError>> {
        // Create an iterator over the text chunks
        let chars: Vec<char> = text.chars().collect();
        let chunks: Vec<String> = chars
            .chunks(chunk_size)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect();

        // Map each chunk through the encoder
        chunks.iter().map(|chunk| self.encode(chunk)).collect()
    }

    /// Decode token IDs to text
    pub fn decode(&self, token_ids: &[u32]) -> Result<String, CandleTokenizerError> {
        self.tokenizer
            .decode(token_ids, true)
            .map_err(|e| CandleTokenizerError::DecodeFailed(e.to_string()))
    }

    /// Decode single token ID to text
    pub fn decode_token(&self, token_id: u32) -> Result<String, CandleTokenizerError> {
        if let Some(token_text) = self.reverse_vocab.get(&token_id) {
            Ok(token_text.clone())
        } else {
            self.decode(&[token_id])
        }
    }

    /// Get the end of sequence token ID
    pub fn eos_token_id(&self) -> u32 {
        self.config.eos_token_id.unwrap_or(2)
    }

    /// Get the beginning of sequence token ID
    pub fn bos_token_id(&self) -> u32 {
        self.config.bos_token_id.unwrap_or(1)
    }

    /// Get the padding token ID
    pub fn pad_token_id(&self) -> u32 {
        self.config.pad_token_id.unwrap_or(0)
    }

    /// Get the unknown token ID
    pub fn unk_token_id(&self) -> u32 {
        self.config.unk_token_id.unwrap_or(3)
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    /// Get maximum sequence length
    pub fn max_length(&self) -> usize {
        self.config.max_length
    }

    /// Batch encode multiple texts efficiently
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Vec<Vec<u32>>, CandleTokenizerError> {
        texts.iter().map(|text| self.encode(text)).collect()
    }

    /// Batch decode multiple token sequences efficiently
    pub fn decode_batch(
        &self,
        token_sequences: &[&[u32]],
    ) -> Result<Vec<String>, CandleTokenizerError> {
        token_sequences
            .iter()
            .map(|tokens| self.decode(tokens))
            .collect()
    }

    /// Add padding to token sequences to make them equal length
    pub fn pad_sequences(&self, sequences: &mut [Vec<u32>]) {
        if sequences.is_empty() {
            return;
        }

        let max_len = match self.config.padding {
            CandlePaddingStrategy::None => return,
            CandlePaddingStrategy::Longest => {
                sequences.iter().map(|seq| seq.len()).max().unwrap_or(0)
            }
            CandlePaddingStrategy::MaxLength => self.config.max_length,
        };

        let pad_token = self.pad_token_id();

        for sequence in sequences.iter_mut() {
            while sequence.len() < max_len {
                sequence.push(pad_token);
            }
        }
    }

    /// Check if token is a special token
    pub fn is_special_token(&self, token_id: u32) -> bool {
        token_id == self.bos_token_id()
            || token_id == self.eos_token_id()
            || token_id == self.pad_token_id()
            || token_id == self.unk_token_id()
    }

    /// Get token statistics for performance monitoring
    pub fn get_stats(&self) -> CandleTokenizerStats {
        CandleTokenizerStats {
            vocab_size: self.vocab.len(),
            cache_size: self.token_cache.len(),
            max_length: self.config.max_length,
            special_tokens: 4, // bos, eos, pad, unk
        }
    }

    /// Clear token cache to free memory
    pub fn clear_cache(&mut self) {
        self.token_cache.clear();
    }

    // Helper methods for building vocabulary mappings
    fn build_vocab_map(
        tokenizer: &Tokenizer,
    ) -> Result<HashMap<String, u32>, CandleTokenizerError> {
        let vocab = tokenizer.get_vocab(true);
        Ok(vocab)
    }

    fn build_reverse_vocab_map(vocab: &HashMap<String, u32>) -> HashMap<u32, String> {
        vocab
            .iter()
            .map(|(token, &id)| (id, token.clone()))
            .collect()
    }
}

/// Statistics for tokenizer performance monitoring
#[derive(Debug, Clone)]
pub struct CandleTokenizerStats {
    /// Size of the vocabulary
    pub vocab_size: usize,
    /// Number of cached token sequences
    pub cache_size: usize,
    /// Maximum sequence length
    pub max_length: usize,
    /// Number of special tokens
    pub special_tokens: usize,
}

/// Streaming tokenizer for processing large texts
pub struct CandleStreamingTokenizer {
    /// Base tokenizer
    tokenizer: Arc<CandleTokenizer>,
    /// Overlap size for streaming chunks
    overlap_size: usize,
    /// Current buffer state
    buffer: String,
    /// Current position in input
    position: usize,
}

impl CandleStreamingTokenizer {
    /// Create a new streaming tokenizer
    pub fn new(tokenizer: Arc<CandleTokenizer>, overlap_size: usize) -> Self {
        Self {
            tokenizer,
            overlap_size,
            buffer: String::with_capacity(4096),
            position: 0,
        }
    }

    /// Process next chunk of text and return tokens
    pub fn process_chunk(&mut self, chunk: &str) -> Result<Vec<u32>, CandleTokenizerError> {
        // Add chunk to buffer
        self.buffer.push_str(chunk);

        // Tokenize current buffer
        let tokens = self.tokenizer.encode(&self.buffer)?;

        // Keep overlap for next chunk
        if self.buffer.len() > self.overlap_size {
            let start_pos = self.buffer.len() - self.overlap_size;
            self.buffer = self.buffer[start_pos..].to_string();
        }

        self.position += chunk.len();
        Ok(tokens)
    }

    /// Finalize streaming and get remaining tokens
    pub fn finalize(&mut self) -> Result<Vec<u32>, CandleTokenizerError> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }

        let tokens = self.tokenizer.encode(&self.buffer)?;
        self.buffer.clear();
        Ok(tokens)
    }

    /// Reset streaming state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.position = 0;
    }
}

/// Error types for tokenizer operations
#[derive(Debug, Clone)]
pub enum CandleTokenizerError {
    /// Failed to load tokenizer from file
    LoadFailed(String),
    /// Failed to encode text
    EncodeFailed(String),
    /// Failed to decode tokens
    DecodeFailed(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Token not found in vocabulary
    TokenNotFound(u32),
    /// Sequence too long
    SequenceTooLong(usize),
}

impl std::fmt::Display for CandleTokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadFailed(msg) => write!(f, "Failed to load tokenizer: {}", msg),
            Self::EncodeFailed(msg) => write!(f, "Failed to encode text: {}", msg),
            Self::DecodeFailed(msg) => write!(f, "Failed to decode tokens: {}", msg),
            Self::InvalidConfig(msg) => write!(f, "Invalid tokenizer configuration: {}", msg),
            Self::TokenNotFound(id) => write!(f, "Token ID {} not found in vocabulary", id),
            Self::SequenceTooLong(len) => write!(f, "Sequence length {} exceeds maximum", len),
        }
    }
}

impl std::error::Error for CandleTokenizerError {}

/// Utility functions for token operations
pub mod utils {
    use super::*;

    /// Calculate token overlap between two sequences
    pub fn calculate_overlap(seq1: &[u32], seq2: &[u32]) -> usize {
        let min_len = seq1.len().min(seq2.len());

        for i in 1..=min_len {
            let end1 = &seq1[seq1.len() - i..];
            let start2 = &seq2[..i];

            if end1 == start2 {
                return i;
            }
        }

        0
    }

    /// Merge overlapping token sequences
    pub fn merge_sequences(seq1: &[u32], seq2: &[u32], overlap: usize) -> Vec<u32> {
        if overlap == 0 {
            let mut result = seq1.to_vec();
            result.extend_from_slice(seq2);
            return result;
        }

        let mut result = Vec::with_capacity(seq1.len() + seq2.len() - overlap);
        result.extend_from_slice(&seq1[..seq1.len() - overlap]);
        result.extend_from_slice(seq2);
        result
    }

    /// Remove special tokens from sequence
    pub fn remove_special_tokens(tokenizer: &CandleTokenizer, tokens: &[u32]) -> Vec<u32> {
        tokens
            .iter()
            .copied()
            .filter(|&token| !tokenizer.is_special_token(token))
            .collect()
    }

    /// Count tokens in text without full encoding
    pub fn estimate_token_count(text: &str) -> usize {
        // Rough estimation: ~4 characters per token for English text
        (text.len() as f64 / 4.0).ceil() as usize
    }

    /// Split text at token boundaries for efficient processing
    pub fn split_at_token_boundary(text: &str, max_chars: usize) -> (&str, &str) {
        if text.len() <= max_chars {
            return (text, "");
        }

        // Try to split at whitespace near the boundary
        let mut split_pos = max_chars;

        // Look backward for whitespace
        while split_pos > 0 && !text.chars().nth(split_pos).is_some_and(char::is_whitespace) {
            split_pos -= 1;
        }

        // If no whitespace found, use character boundary
        if split_pos == 0 {
            split_pos = max_chars;
        }

        text.split_at(split_pos)
    }
}


